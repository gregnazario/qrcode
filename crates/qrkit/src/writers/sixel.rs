//! Sixel output (DEC sixel graphics, ISO 8613-6 image compression).
//!
//! Pixels are quantized to ≤256 colors (uniform 8×8×4 RGB buckets — QR
//! scenes are few-colored, gradients included) and emitted as six-row
//! bands of sixel characters with RLE compression. Terminals: xterm
//! (`-ti vt340`), foot, WezTerm, mintty, Alacritty (recent), others.

use super::raster::rasterize;
use crate::scene::VectorScene;
use std::fmt::Write as _;

/// Encode a scene as sixel graphics at `px_per_module`.
pub fn write(scene: &VectorScene, px_per_module: u32) -> Vec<u8> {
    let pm = rasterize(scene, px_per_module);
    encode(pm.data(), pm.width() as usize, pm.height() as usize)
}

/// Quantize `rgba` (w×h, row-major) and encode as a sixel byte stream.
pub fn encode(rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    // ---- quantize: 256 uniform RGB buckets (8 red x 8 green x 4 blue) ----
    let bucket = |p: &[u8]| -> usize {
        ((p[0] as usize >> 5) << 5) | ((p[1] as usize >> 5) << 2) | (p[2] as usize >> 6)
    };
    // Buckets map to dense color ids; representatives are bucket centers.
    let mut bucket_id = [u16::MAX; 256];
    let mut palette: Vec<(u8, u8, u8)> = Vec::new();
    let mut idx = vec![0u16; w * h];
    for (i, p) in rgba.chunks_exact(4).enumerate() {
        let b = bucket(p);
        if bucket_id[b] == u16::MAX {
            bucket_id[b] = palette.len() as u16;
            let r = ((b >> 5) as f32 + 0.5) * 255.0 / 8.0;
            let g = (((b >> 2) & 7) as f32 + 0.5) * 255.0 / 8.0;
            let bl = ((b & 3) as f32 + 0.5) * 255.0 / 4.0;
            palette.push((r as u8, g as u8, bl as u8));
        }
        idx[i] = bucket_id[b];
    }

    let mut out = String::new();
    let _ = write!(out, "\x1bPq\"1;1;{w};{h}");
    // Color definitions: #id;2;R;G;B on the 0..100 sixel scale.
    for (i, &(r, g, b)) in palette.iter().enumerate() {
        let s = |v: u8| v as u32 * 100 / 255;
        let _ = write!(out, "#{i};2;{};{};{}", s(r), s(g), s(b));
    }
    // ---- six-row bands ----
    let mut y0 = 0;
    while y0 < h {
        let rows = (h - y0).min(6);
        let mut first_color = true;
        for id in 0..palette.len() as u16 {
            // Does this color appear anywhere in the band?
            let present = (0..rows).any(|r| {
                let y = y0 + r;
                (0..w).any(|x| idx[y * w + x] == id)
            });
            if !present {
                continue;
            }
            if first_color {
                first_color = false;
            } else {
                out.push('$'); // carriage return within the band
            }
            let _ = write!(out, "#{id}");
            let mut x = 0;
            while x < w {
                // Bits: which rows of this column carry the color.
                let mut bits = 0u8;
                for r in 0..rows {
                    if idx[(y0 + r) * w + x] == id {
                        bits |= 1 << r;
                    }
                }
                // RLE: count the run of identical columns (all rows equal).
                let mut run = 1;
                while x + run < w {
                    let mut same = true;
                    for r in 0..rows {
                        if (idx[(y0 + r) * w + x + run] == id) != (bits & (1 << r) != 0) {
                            same = false;
                            break;
                        }
                    }
                    if !same {
                        break;
                    }
                    run += 1;
                }
                if run > 3 {
                    let _ = write!(out, "!{run}");
                }
                out.push(char::from_u32(0x3f + bits as u32).unwrap());
                x += run;
            }
        }
        out.push('-'); // next band
        y0 += 6;
    }
    out.push_str("\x1b\\");
    out.into_bytes()
}
