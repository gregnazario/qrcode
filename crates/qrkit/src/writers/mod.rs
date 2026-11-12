//! Format writers: pure serializers of a VectorScene, plus the terminal
//! renderers (which work from the module matrix / raster instead — see
//! `term`, `kitty`, `iterm`, `sixel`).

#[doc(hidden)]
pub mod common;
pub mod eps;
pub mod iterm;
pub mod kitty;
pub mod pdf;
pub mod png;
pub mod raster;
pub mod sixel;
pub mod svg;
pub mod term;
pub mod term_auto;
