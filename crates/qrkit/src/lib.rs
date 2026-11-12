//! qrkit: customizable QR codes as vector graphics (SVG, PDF, EPS).

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
