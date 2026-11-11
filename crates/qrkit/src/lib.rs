pub mod builder;
pub mod content; // stub now, filled in Task 2
pub mod logo; // stub
pub mod render; // stub
pub mod scene; // stub
pub mod style; // stub
pub mod writers; // stub

/// Errors produced while building or writing a QR code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("content validation failed: {0}")]
    Content(String),
    #[error("data too large for QR code: {0}")]
    TooLarge(String),
    #[error("encoding failed: {0}")]
    Encoding(String),
    #[error("unsupported logo image format (expected PNG or JPEG)")]
    LogoUnsupportedFormat,
    #[error("logo decode failed: {0}")]
    LogoDecode(String),
    #[error("invalid style: {0}")]
    InvalidStyle(String),
}
