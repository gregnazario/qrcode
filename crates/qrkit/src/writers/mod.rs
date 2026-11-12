//! Format writers: pure serializers of a VectorScene, plus the terminal
//! renderer (which works from the module matrix instead — see `term`).

#[doc(hidden)]
pub mod common;
pub mod eps;
pub mod pdf;
pub mod svg;
pub mod term;
