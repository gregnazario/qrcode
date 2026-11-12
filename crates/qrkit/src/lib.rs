//! # qrkit
//!
//! Fully customizable QR code generation with true vector output — plus
//! raster (PNG) and terminal formats.
//!
//! - **9 content types** ([`content`]): text, URL, email, SMS, phone, WiFi,
//!   vCard, geo, calendar — each validated eagerly and formatted into its
//!   standards-compliant payload (`WIFI:`, vCard 3.0, VEVENT, `mailto:`…)
//! - **Full styling** ([`style`]): ECC/version/margin, solid & gradient
//!   fills, module shapes (square/rounded/dots/classy), eye shapes with
//!   separate colors, PNG/JPEG logo embedding, and a decorative-eyes mode
//! - **One geometry source** ([`scene`]): the vector scene-graph feeds the
//!   [`writers`] — SVG, PDF 1.7, PostScript-3 EPS, PNG, the kitty/iTerm2/
//!   sixel terminal image protocols, and ANSI half-blocks — so every format
//!   renders identical geometry
//! - **Scanner-safe by default**: eye geometry is decode-capped, a logo
//!   forces ECC High, and the test suite decodes every style combination
//!   back with rqrr to prove it
//!
//! # Example
//!
//! ```
//! use qrkit::builder::QrBuilder;
//! use qrkit::content::Content;
//! use qrkit::scene::Rgba;
//! use qrkit::style::{EyeShape, ModuleShape, StyleConfig};
//!
//! let svg = QrBuilder::new(Content::Text("hello".into()))?
//!     .style(StyleConfig::new()
//!         .shape(ModuleShape::Dots { radius: 0.5 })
//!         .eye_shape(EyeShape::Circle)
//!         .fill(qrkit::scene::Fill::Solid(Rgba::BLACK)))
//!     .to_svg()?;
//! assert!(svg.starts_with(b"<?xml"));
//! # Ok::<(), qrkit::Error>(())
//! ```
//!
//! Build first when you need warnings (a logo forcing ECC High, decorative
//! eyes, small dot radii):
//!
//! ```
//! use qrkit::builder::QrBuilder;
//! use qrkit::content::Content;
//!
//! let img = QrBuilder::new(Content::Url("https://example.com".into()))?
//!     .build()?;
//! let out = std::env::temp_dir().join("qrkit-example.pdf");
//! let _ = std::fs::write(&out, qrkit::writers::pdf::write(&img.scene));
//! for w in &img.warnings {
//!     eprintln!("warning: {w}");
//! }
//! # Ok::<(), qrkit::Error>(())
//! ```
//!
//! The CLI (`qrkit-cli` crate) and the browser app (`qrkit-wasm` crate)
//! are thin layers over this API. User-facing docs live in the repo's
//! `docs/` directory.

pub mod builder;
pub mod content;
pub mod logo;
pub mod render;
pub mod scene;
pub mod style;
pub mod writers;

/// Errors produced while building or writing a QR code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("content validation failed: {0}")]
    Content(String),
    #[error("data too large for QR code: {0}")]
    TooLarge(String),
    #[error("unsupported logo image format (expected PNG or JPEG)")]
    LogoUnsupportedFormat,
    #[error("logo decode failed: {0}")]
    LogoDecode(String),
    #[error("invalid style: {0}")]
    InvalidStyle(String),
}
