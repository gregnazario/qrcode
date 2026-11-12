# qrkit

Fully customizable QR code generator producing **true vector graphics**
(SVG, PDF, EPS) plus raster (PNG) and four terminal formats — resize and
restyle losslessly. Rust core library, CLI, and a client-side WASM web app.

**Live web app:** https://gregnazario.github.io/qrcode/ (runs entirely in
your browser — nothing leaves it) ·
**Documentation:** [docs/](docs/) · [CLI reference](docs/cli-reference.md)

## Documentation

| Page | Contents |
|---|---|
| [CLI reference](docs/cli-reference.md) | Every subcommand and flag, defaults, examples, exit codes |
| [Content types](docs/content-types.md) | The 9 payload types, their fields, validation, and the exact strings they produce |
| [Output formats](docs/formats.md) | SVG/PDF/EPS/PNG + terminal image protocols (kitty, iTerm2, sixel, ANSI), capability matrix, terminal support |
| [Scanner safety](docs/scanner-safety.md) | Why the geometry looks like it does, `--decorative-eyes`, dot radius, logo ECC — and how decodability is test-enforced |
| [Library guide](docs/library.md) | Rust API (builder, scene, writers), the WASM/JSON API, embedding |

## Quick start (CLI)

```console
$ cargo install --path crates/qrkit-cli

$ qrkit url "https://example.com/promo" \
    --shape dots --fg "#0d47a1" --gradient linear --gradient-to "#42a5f5" --gradient-angle 45 \
    --eye circle --logo /path/to/logo.png --logo-size 0.25 \
    -o poster.pdf

$ qrkit wifi --ssid "Cafe Guest" --password "latte-2026" --encryption wpa -o menu.eps
$ qrkit vcard --name "Ada Lovelace" --email ada@example.com -o card.svg
$ qrkit geo --lat 51.5007 --lon -0.1246 -o spot.png
$ qrkit url "https://example.com" --format auto    # best terminal image protocol
$ qrkit text "hello" --preview                      # Unicode preview on stderr
```

Nine content types: `text`, `url`, `email`, `sms`, `phone`, `wifi`,
`vcard`, `geo`, `calendar` — each validates eagerly and formats the
standards-compliant payload (`WIFI:`, vCard 3.0, VEVENT, `mailto:`, `tel:`,
`SMSTO:`, `geo:`). Full details: [content types](docs/content-types.md).

## Customization

- ECC L/M/Q/H (a logo forces H), fixed version 1–40 or auto, quiet-zone margin
- Module shapes: square, rounded, dots (adjustable radius), classy
- Eye shapes: square, rounded, circle, leaf — with separate ring and dot colors
- Solid colors, linear gradients (any angle), radial gradients; transparent background
- `--decorative-eyes`: pure-shape eyes (bigger radii, no scanner-safety
  tabs/nubs) — cleaner look, but strict readers may fail to decode
- Logo embedding (PNG/JPEG) with module clearing under the logo area

## Output formats

Vector: `svg`, `pdf` (1.7, shading patterns, image XObjects), `eps`
(Level 3, `shfill`; colors pre-blended — EPS has no alpha).
Raster: `png` (straight alpha, `--px` per module).
Terminal: `kitty` / `iterm` / `sixel` image protocols and `term` ANSI
half-blocks; `auto` detects your terminal and falls back to half-blocks.
All formats share one geometry source — the SVG, PDF, and EPS of the same
code are identical by construction. Details: [formats](docs/formats.md).

## Library

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::style::{ModuleShape, StyleConfig};

let svg = QrBuilder::new(Content::Text("hello".into()))?
    .style(StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }))
    .to_svg()?;
```

Rustdoc: `cargo doc -p qrkit --open`. Full guide incl. the WASM/JSON API:
[library](docs/library.md).

## Web app

```console
$ cargo install wasm-pack
$ just build-wasm
$ just serve          # http://127.0.0.1:8080
```

Pick a content type, fill the form, style it live (colors, gradients,
shapes, eyes, logo), download SVG/PDF/EPS. In-app documentation is on the
[Docs](https://gregnazario.github.io/qrcode/docs.html) page. Everything
runs client-side.

## Development

```console
$ just check          # fmt + clippy -D warnings + full test suite (88 tests)
$ just build-wasm     # wasm-pack build for the web app
$ just serve          # local static server for web/
```

The test suite's centerpiece is the decode gate: every style combination is
rasterized and decoded back with rqrr, asserting exact payload round-trips
(see [scanner safety](docs/scanner-safety.md)).
