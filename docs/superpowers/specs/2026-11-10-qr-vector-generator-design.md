# QR Vector Generator (`qrkit`) — Design

**Date:** 2026-11-10
**Status:** Approved

## Goal

A fully customizable QR code generator supporting multiple QR content types,
outputting true vector graphics (SVG, PDF, EPS) that can be resized and edited
losslessly. Delivered as a Rust core library with a CLI and a client-side
WASM web app.

## Non-goals (v1)

- Symbol variants (Micro QR, rMQR) — content types only.
- Animated QR / structured append.
- Batch generation.
- Server-side rendering.

## Architecture decision

**Approach: vector scene-graph with pluggable writers.**

- Encoding: `qrcodegen` crate (pure Rust, MIT, spec-conformant) produces the
  module matrix. We do not implement Reed–Solomon/masking ourselves.
- Rendering: our crate composes a `VectorScene` (ordered shapes + fills +
  optional embedded logo) from the matrix and style config.
- Serialization: three thin writers (SVG, PDF, EPS) consume the same
  `VectorScene`, keeping all formats geometrically identical.

Rejected alternatives: SVG-first + conversion (heavy `resvg` dep, EPS needs
external tools, lossy round-trips); full from-scratch encoder (weeks of work,
spec-bug risk, no user benefit).

## Workspace layout

```
qrcode/                      # cargo workspace
├── crates/
│   ├── qrkit/               # core library
│   ├── qrkit-cli/           # CLI binary
│   └── qrkit-wasm/          # wasm-bindgen wrapper
├── web/                     # static web app (vanilla JS/CSS + built wasm)
├── docs/superpowers/specs/
└── justfile                 # build / serve / test tasks
```

Crate name `qrkit` (`qrcode` is taken on crates.io).

## Core library (`crates/qrkit`)

### Public API

Fluent builder over typed content + style:

```rust
let svg = QrBuilder::new(Content::Wifi(Wifi {
    ssid: "Cafe Guest".into(),
    password: Some("latte-2026".into()),
    encryption: Encryption::Wpa,
    hidden: false,
}))
.style(StyleConfig::new()
    .ecc(Ecc::High)
    .margin_modules(4)
    .shape(ModuleShape::Dots { radius: 0.5 })
    .eye_shape(EyeShape::Rounded)
    .fill(Fill::gradient_linear("#0d47a1", "#42a5f5", 45.0))
    .eye_fill(Fill::solid("#0d47a1"))
    .logo(LogoConfig::from_bytes(png_bytes).size_ratio(0.25)))
.to_svg()?;   // also .to_pdf()? / .to_eps()?
```

### Content types

Each type validates fields and formats a standards-compliant payload string:

| Type | Fields | Payload |
|---|---|---|
| `Text` | free text | raw text |
| `Url` | URL (validated) | URL string |
| `Email` | to, subject?, body? | `mailto:` with query |
| `Sms` | number, message? | `SMSTO:` |
| `Phone` | number | `tel:` |
| `Wifi` | ssid, password?, WPA/WEP/open, hidden? | `WIFI:T:WPA;S:…;;` |
| `VCard` | name, org?, title?, phones[], emails[], url?, address?, note? | vCard 3.0 |
| `Geo` | lat, lon (range-validated) | `geo:lat,lon` |
| `Calendar` | title, start/end (UTC), location?, description? | VEVENT |

### Style configuration

- **Core:** `ecc` (L/M/Q/H; default H when logo set, else Q), `version`
  (auto or 1–40), `margin_modules` (default 4).
- **Colors:** module fill and eye fill each accept `Solid(Rgba)` or a
  gradient (linear with angle, or radial). Background: solid or transparent
  (transparent supported in SVG/PDF). In EPS, a transparent background
  renders as white, and any alpha in fills is blended against the effective
  background color (white when transparent).
- **Shapes:** `ModuleShape::Square | Rounded | Dots | Classy` (Classy =
  rounded with two diagonally opposite corners left square);
  `EyeShape::Square | Rounded | Circle | Leaf`.
- **Logo:** `LogoConfig { bytes, size_ratio (0.1–0.33, default 0.25),
  padding }`. Modules covered by the logo box are skipped (not overdrawn).
  When a logo is present, ECC is forced/bumped to High; setting lower ECC
  with a logo produces a warning.

### Scene-graph

```rust
enum Shape {
    Rect { x, y, w, h },
    RoundedRect { x, y, w, h, radius },
    Circle { cx, cy, r },
    Path(PathBuilder),
}
enum Fill {
    Solid(Rgba),
    LinearGradient { from: Rgba, to: Rgba, angle_deg: f32 },
    RadialGradient { from: Rgba, to: Rgba },
}
struct VectorScene {
    size: Size,
    background: Option<Fill>,
    items: Vec<SceneItem>,          // Shape + Fill
    logo: Option<EmbeddedImage>,    // PNG/JPEG bytes + placement rect
}
```

Composition rules:
- Dark data/timing modules → one item per module in `module_shape`; modules
  under the logo box are skipped.
- Finder eyes (3 corners) → separate items in `eye_shape`/`eye_fill`, split
  into outer ring + inner dot (ring uses eye fill; dot uses module fill by
  default).
- Alignment patterns follow the module shape.

### Writers

| Format | Implementation | Notes |
|---|---|---|
| SVG | hand-rolled serializer | gradient `<defs>`, `<image>` base64 data URI, `shape-rendering="geometricPrecision"` |
| PDF | hand-rolled minimal PDF 1.7 writer | fill operators, axial/radial shadings, logo as image XObject (PNG with SMask) |
| EPS | PostScript Level 3 | gradients via `shfill`; no alpha — approximated by blending against background (documented) |

Writers are pure functions `(&VectorScene) -> Vec<u8>`.

## CLI (`crates/qrkit-cli`, binary `qrkit`)

Subcommand per content type; shared style/output flags (clap derive):

```console
$ qrkit url "https://example.com/promo" \
    --shape dots --fg "#0d47a1" --gradient-to "#42a5f5" --gradient-angle 45 \
    --eye circle --logo assets/logo.png --logo-size 0.25 \
    -o poster.svg                  # format from extension (svg|pdf|eps)

$ qrkit wifi --ssid "Cafe Guest" --password "latte-2026" --wpa --hidden -o menu.pdf
$ qrkit vcard --name "Ada Lovelace" --email ada@example.com -o card.svg
$ qrkit geo --lat 51.5007 --lon -0.1246 -o spot.eps
```

- Defaults: black/white, square modules, ECC Q (H with logo), margin 4.
- `--preview`: renders the QR in the terminal via Unicode half-blocks.
- Errors map library errors to readable messages with actionable hints.

## Web app (client-side WASM)

- `qrkit-wasm` exposes `generate_svg(config_json) -> String`,
  `generate_pdf(config_json) / generate_eps(config_json) -> ArrayBuffer`,
  plus logo passthrough via config (bytes as base64).
- `web/`: static HTML/CSS/vanilla JS, no framework, no build step beyond
  `wasm-pack build`. Left panel: content-type selector → dynamic form +
  logo upload. Right panel: live SVG preview re-rendered on input, download
  buttons (SVG/PDF/EPS). `just serve` for local development.

## Error handling

`thiserror` enum in `qrkit`:

- Content validation (bad URL, out-of-range coords, empty required field).
- Data over capacity (hint: lower ECC or shorten content).
- Logo decode failures.
- Writer/Io failures.

Validation is eager: `QrBuilder::new` validates content; `style` validates
style params; encode/write errors surface with context.

## Testing

1. **Payload golden tests** (`insta`): formatted payload strings per content
   type against spec examples.
2. **Scene round-trip:** reconstruct module matrix from `VectorScene`, diff
   against `qrcodegen` matrix.
3. **End-to-end decode:** rasterize scene with `tiny-skia` (dev-dep) at
   multiple sizes, decode with `rqrr` (dev-dep), assert payload matches —
   across every module-shape × eye-shape × logo combination.
4. **Writer structural tests:** PDF parses with `lopdf`; SVG/EPS snapshot
   tests.

## Dependencies (runtime)

- `qrcodegen` (encoder)
- `thiserror` (errors, qrkit)
- `clap` (CLI only)
- `wasm-bindgen` / `serde` / `serde_json` (WASM only)

Dev-only: `insta`, `tiny-skia`, `rqrr`, `lopdf`.
