//! The public builder tying content, style, rendering, and writing together.

use crate::content::Content;
use crate::render::{render, Matrix};
use crate::scene::VectorScene;
use crate::style::StyleConfig;
use crate::writers::{eps, pdf, svg};
use crate::Error;

/// A built QR code: the scene plus non-fatal warnings.
#[derive(Debug, Clone)]
pub struct QrImage {
    pub scene: VectorScene,
    pub warnings: Vec<String>,
}

/// Fluent builder: content in, vector bytes out.
#[derive(Debug, Clone)]
pub struct QrBuilder {
    content: Content,
    style: StyleConfig,
}

impl QrBuilder {
    /// Start from typed content. Validates the content eagerly.
    pub fn new(content: Content) -> Result<QrBuilder, Error> {
        content.payload()?; // eager validation per spec
        Ok(QrBuilder {
            content,
            style: StyleConfig::new(),
        })
    }

    pub fn style(mut self, style: StyleConfig) -> QrBuilder {
        self.style = style;
        self
    }

    /// Encode and render. Warnings come from `StyleConfig::validate`.
    pub fn build(&self) -> Result<QrImage, Error> {
        let payload = self.content.payload()?;
        let warnings = self.style.validate();
        let matrix = Matrix::encode(&payload, self.style.effective_ecc(), self.style.version)?;
        Ok(QrImage {
            scene: render(&matrix, &self.style),
            warnings,
        })
    }

    /// Convenience: build and serialize to SVG (warnings are ignored here;
    /// call `build()` first if you need them).
    pub fn to_svg(&self) -> Result<Vec<u8>, Error> {
        Ok(svg::write(&self.build()?.scene))
    }

    /// Convenience: build and serialize to PDF (warnings are ignored here;
    /// call `build()` first if you need them).
    pub fn to_pdf(&self) -> Result<Vec<u8>, Error> {
        Ok(pdf::write(&self.build()?.scene))
    }

    /// Build and serialize to EPS (alpha pre-blended; see eps module docs).
    pub fn to_eps(&self) -> Result<Vec<u8>, Error> {
        Ok(eps::write(&self.build()?.scene))
    }
}
