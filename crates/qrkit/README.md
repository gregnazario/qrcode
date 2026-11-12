# qrkit

Fully customizable QR code generator with true vector output (SVG, PDF,
EPS), raster PNG, and four terminal formats. Rust core library; the CLI
lives in the sibling `qrkit-cli` crate.

- **9 content types** with standards-compliant payloads: text, URL, email,
  SMS, phone, WiFi, vCard, geo, calendar
- **Full styling**: ECC/version/margin, solid & gradient fills, module
  shapes, eye shapes with separate colors, logo embedding
- **One geometry source** for every format — SVG/PDF/EPS render identical
  output by construction
- **Scanner-safe by default**: decode-capped eye geometry; the test suite
  decodes every style combination back with a QR reader to prove it

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::style::{ModuleShape, StyleConfig};

let svg = QrBuilder::new(Content::Text("hello".into()))?
    .style(StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }))
    .to_svg()?;
```

Full documentation: <https://github.com/gregnazario/qrcode/tree/main/docs>
· Web app: <https://gregnazario.github.io/qrcode/>
