# qrkit

Fully customizable QR code generator producing true vector graphics (SVG,
PDF, EPS) — resize and restyle losslessly. Rust core library, CLI, and a
client-side WASM web app.

## Quick start (CLI)

```console
$ cargo install --path crates/qrkit-cli

$ qrkit url "https://example.com/promo" \
    --shape dots --fg "#0d47a1" --gradient linear --gradient-to "#42a5f5" --gradient-angle 45 \
    --eye circle --logo /path/to/logo.png --logo-size 0.25 \
    -o poster.svg

$ qrkit wifi --ssid "Cafe Guest" --password "latte-2026" --encryption wpa -o menu.pdf
$ qrkit vcard --name "Ada Lovelace" --email ada@example.com -o card.svg
$ qrkit geo --lat 51.5007 --lon -0.1246 -o spot.eps
$ qrkit text "hello" --preview        # Unicode preview on stderr
```

Output format is inferred from the extension, or forced with `--format`
(`svg|pdf|eps|term|png|kitty|iterm|sixel|auto`). Without `-o`, SVG goes to
stdout. Terminal formats: `term` = ANSI truecolor half-blocks; `kitty`
(kitty/Ghostty/WezTerm graphics protocol), `iterm` (iTerm2/Konsole inline
images), and `sixel` (xterm/foot/WezTerm/mintty) embed a rasterized image
via `--px` pixels per module; `auto` picks the best protocol for your
terminal and falls back to half-blocks. Module shapes do not apply to the
character grid (`term`).

## Content types

`text`, `url`, `email`, `sms`, `phone`, `wifi`, `vcard`, `geo`, `calendar` —
each subcommand validates and formats the standards-compliant payload
(`WIFI:`, vCard 3.0, VEVENT, `mailto:`, `tel:`, `SMSTO:`, `geo:`).

## Customization

- ECC L/M/Q/H (a logo forces H), fixed version 1–40 or auto, quiet-zone margin
- Module shapes: square, rounded, dots (adjustable radius), classy
- Eye shapes: square, rounded, circle, leaf — with separate ring and dot colors
- Solid colors, linear gradients (any angle), radial gradients; transparent background
- `--decorative-eyes`: pure-shape eyes (bigger radii, no scanner-safety tabs/nubs) — cleaner look, but strict readers may fail to decode
- Logo embedding (PNG/JPEG) with module clearing under the logo area

## Web app

```console
$ cargo install wasm-pack
$ just build-wasm
$ just serve          # http://127.0.0.1:8080
```

Everything runs client-side; nothing leaves the browser.

## Library

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::style::{ModuleShape, StyleConfig};

let svg = QrBuilder::new(Content::Text("hello".into()))?
    .style(StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }))
    .to_svg()?;
```

## Notes

- EPS has no alpha: colors are pre-blended against the background (white when
  transparent), and gradients use PostScript Level 3 `shfill`.
- PDFs embed PNG logos as RGB + alpha SMask; JPEGs pass through as DCTDecode.
- The scene-graph is shared by all three writers, so geometry stays identical
  across formats.
