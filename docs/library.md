# Library guide

`qrkit` is a library first — the CLI and web app are thin layers over it.
Everything is available from the crate root.

The crate is not yet published to crates.io; use a path or git dependency:

```toml
[dependencies]
qrkit = { path = "crates/qrkit" }
# or: qrkit = { git = "https://github.com/gregnazario/qrcode" }
```

## Quick start

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::style::{ModuleShape, StyleConfig};

let svg = QrBuilder::new(Content::Text("hello".into()))?
    .style(StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }))
    .to_svg()?;
std::fs::write("qr.svg", svg)?;
```

## The pipeline

```
Content ──payload()──▶ str ──qrcodegen──▶ Matrix ──render()──▶ VectorScene
                                                                   │
                                    writers::svg / pdf / eps ◀──────┤
                                    writers::raster ─▶ png / kitty / iterm / sixel
                                    writers::term  ◀── Matrix + Style
```

- `Content` ([content types](content-types.md)) validates and formats the
  payload string
- `Matrix` encodes it (ECC, version pinning)
- `render` composes the `VectorScene` — one geometry source shared by every
  format, which is why SVG/PDF/EPS stay identical
- `QrBuilder` glues it together and returns a `QrImage` with the scene, the
  matrix, and any style warnings

## Builder API

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::{Content, Encryption};
use qrkit::logo::LogoConfig;
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, EyeShape, ModuleShape, StyleConfig};

fn make_outputs() -> Result<(), Box<dyn std::error::Error>> {
    let logo = LogoConfig::from_bytes(std::fs::read("logo.png")?)?;

    let img = QrBuilder::new(Content::Wifi {
        ssid: "Cafe Guest".into(),
        password: Some("latte-2026".into()),
        encryption: Encryption::Wpa,
        hidden: false,
    })?
    .style(StyleConfig::new()
        .ecc(Ecc::High)                                // logo forces this anyway
        .margin_modules(4)
        .shape(ModuleShape::Classy)
        .eye_shape(EyeShape::Leaf)
        .decorative_eyes(false)                        // scanner-safe default
        .fill(Fill::LinearGradient {
            from: Rgba::from_hex("#0d47a1")?,
            to: Rgba::from_hex("#42a5f5")?,
            angle_deg: 45.0,
        })
        .eye_fill(Fill::Solid(Rgba::from_hex("#0d47a1")?))
        .background(Some(Fill::Solid(Rgba::WHITE)))    // None = transparent
        .logo(Some(logo.size_ratio(0.25))))
    .build()?;

    for warning in &img.warnings {
        eprintln!("warning: {warning}");
    }

    std::fs::write("qr.svg", qrkit::writers::svg::write(&img.scene))?;
    std::fs::write("qr.pdf", qrkit::writers::pdf::write(&img.scene))?;
    std::fs::write("qr.eps", qrkit::writers::eps::write(&img.scene))?;
    std::fs::write("qr.png", qrkit::writers::png::write(&img.scene, 24))?;

    // term takes Matrix + Style (module shapes don't apply in a character grid)
    let term_style = StyleConfig::new(); // reuse your style for colors
    print!("{}", String::from_utf8_lossy(&qrkit::writers::term::render(
        &img.matrix, &term_style,
    )));
    Ok(())
}
```

Notes: `Rgba::from_hex` and the builder's content validation return
`qrkit::Error` directly, so `?` works inside any function returning that
(or `Box<dyn Error>` for mixed IO, as above).

Convenience methods `to_svg` / `to_pdf` / `to_eps` / `to_term` build and
serialize in one step (warnings are dropped — call `build()` first if you
need them).

## Style surface

`StyleConfig` fields (all settable via consuming builder methods):
`ecc`, `version` (`Auto`/`Fixed(1..=40)`), `margin_modules`, `shape`
(`Square`/`Rounded`/`Dots{radius}`/`Classy`), `eye_shape`
(`Square`/`Rounded`/`Circle`/`Leaf`), `fill` and `eye_fill`
(`Solid`/`LinearGradient`/`RadialGradient`), `eye_inner_fill`, `background`,
`logo`, `decorative_eyes`. `validate()` returns the same warnings the CLI
prints; `effective_ecc()` reports what a logo forces. See
[scanner safety](scanner-safety.md) before deviating from the defaults.

## Raster and terminal writers

`writers::raster::rasterize(&scene, px_per_module) -> tiny_skia::Pixmap` is
the shared rasterizer (also used by the decode gate). `png::write`,
`kitty::write`, `iterm::write`, and `sixel::write` layer the terminal image
protocols on top; `term_auto::detect()` picks one from the environment.
`term::render(&matrix, &style)` emits ANSI half-blocks.

## WASM API

The `qrkit-wasm` crate exposes one entry point for the browser:

```js
import init, { generate } from "./pkg/qrkit_wasm.js";
await init();

const out = generate(JSON.stringify({
  content: { type: "wifi", ssid: "Cafe Guest", password: "latte-2026" },
  style:   { fg: "#0d47a1", gradient: "linear", gradientTo: "#42a5f5",
             shape: "dots", eye: "circle", decorativeEyes: false,
             logo: { dataBase64: "...", sizeRatio: 0.25 } },
}), "svg");

out.svg;       // string (svg)
out.pdf;       // Uint8Array (pdf/eps)
out.warnings;  // string[]
```

Content uses a `type` tag (`text`, `url`, `email`, `sms`, `phone`, `wifi`,
`vcard`, `geo`, `calendar`) with snake_case fields; style keys are
camelCase and mirror the CLI flags (`px` sets PNG pixels-per-module,
default 24). Formats: `svg`, `pdf`, `eps`, `png` (returned as
`out.png`, a `Uint8Array`).

## Error handling

`qrkit::Error` covers content validation, capacity limits (`TooLarge`
carries a hint to lower ECC or shorten), logo decode failures, and invalid
style values. All writers are infallible `(&Scene) -> Vec<u8>` — problems
that can't happen (serialization) don't pollute signatures.
