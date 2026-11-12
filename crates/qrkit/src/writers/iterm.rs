//! iTerm2 inline-image output (OSC 1337; https://iterm2.com/documentation-images.html).
//!
//! One OSC 1337 `File` command with a base64 PNG payload and `inline=1`,
//! terminated by ST. Terminals: iTerm2, WezTerm, Konsole, mintty, others.

use super::png;
use crate::scene::VectorScene;
use base64::Engine as _;
use std::fmt::Write as _;

/// Render a scene as an iTerm2 inline image at `px_per_module`.
pub fn write(scene: &VectorScene, px_per_module: u32) -> Vec<u8> {
    let png_bytes = png::write(scene, px_per_module);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    let mut out = String::new();
    let _ = write!(
        out,
        "\x1b]1337;File=name=qr.png;size={};inline=1:{b64}\x1b\\",
        png_bytes.len()
    );
    out.push('\n');
    out.into_bytes()
}
