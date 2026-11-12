//! Kitty graphics protocol output (https://sw.kovidgoyal.net/kitty/graphics-protocol/).
//!
//! Transmits a PNG payload via APC escape sequences and displays it inline.
//! The payload is base64 (no escape handling needed) and chunked to 4096
//! bytes per APC — the protocol's recommended chunk size. A delete action
//! for the image id runs first so repeated invocations replace, not stack.
//! Terminals: Kitty, Ghostty, WezTerm.

use super::png;
use crate::scene::VectorScene;
use base64::Engine as _;
use std::fmt::Write as _;

/// Image id used for placements; deleting it first keeps re-runs clean.
const IMAGE_ID: u32 = 31;
/// Maximum base64 payload bytes per APC chunk (protocol recommendation).
const CHUNK: usize = 4096;

/// Render a scene as a kitty-graphics inline image at `px_per_module`.
pub fn write(scene: &VectorScene, px_per_module: u32) -> Vec<u8> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(png::write(scene, px_per_module));

    let mut out = String::new();
    // Delete any prior placement of this id (quietly ignored if absent).
    let _ = write!(out, "\x1b_Ga=d,d=i,i={IMAGE_ID}\x1b\\");
    let _ = write!(out, "\x1b_Ga=d,d=i,i={IMAGE_ID},q=2\x1b\\");

    let mut chunks = b64.as_bytes().chunks(CHUNK).peekable();
    if chunks.peek().is_none() {
        return out.into_bytes();
    }
    // First chunk carries the control keys; m=1 announces more data.
    let first = chunks.next().unwrap();
    let _ = write!(
        out,
        "\x1b_Ga=T,f=100,i={IMAGE_ID},q=1,m=1;{}\x1b\\",
        String::from_utf8_lossy(first)
    );
    while let Some(chunk) = chunks.next() {
        let m = if chunks.peek().is_some() { 1 } else { 0 };
        let _ = write!(out, "\x1b_Gm={m};{}\x1b\\", String::from_utf8_lossy(chunk));
    }
    // newline after the image so the shell prompt lands below it
    out.push('\n');
    out.into_bytes()
}
