//! Geometry helpers shared by all writers.

use crate::scene::{Fill, Rgba};

/// Source-over composite of `c` on the opaque backdrop `bg`; the result is
/// always opaque. Used by the alpha-less output formats (EPS, terminal).
///
/// 8-bit alpha maps to `k/256`, so the halfway alpha 128 composites to an
/// exact 0.5 mix: a 50% black tint over white rounds to 128 = "0.502".
/// Fully opaque colors pass through unchanged.
pub(crate) fn blend_over(c: Rgba, bg: Rgba) -> Rgba {
    if c.a == 255 {
        return c;
    }
    let a = c.a as f32 / 256.0;
    let mix = |fg: u8, b: u8| (fg as f32 * a + b as f32 * (1.0 - a)).round() as u8;
    Rgba {
        r: mix(c.r, bg.r),
        g: mix(c.g, bg.g),
        b: mix(c.b, bg.b),
        a: 255,
    }
}

/// Endpoints of a linear gradient axis through the scene center, spanning
/// the bounding box of a `size x size` square, for `angle_deg`
/// (0 = left-to-right, 90 = top-to-bottom, y grows downward).
#[doc(hidden)]
pub fn linear_endpoints(size: f32, angle_deg: f32) -> ((f32, f32), (f32, f32)) {
    let c = size / 2.0;
    let rad = angle_deg.to_radians();
    let (dx, dy) = (rad.cos(), rad.sin());
    let mut tmin = f32::MAX;
    let mut tmax = f32::MIN;
    for (x, y) in [(0.0, 0.0), (size, 0.0), (0.0, size), (size, size)] {
        let t = (x - c) * dx + (y - c) * dy;
        tmin = tmin.min(t);
        tmax = tmax.max(t);
    }
    (
        (c + dx * tmin, c + dy * tmin),
        (c + dx * tmax, c + dy * tmax),
    )
}

/// Deduplicate gradient fills; returns a stable id list.
pub(crate) fn collect_gradients<'a>(fills: impl Iterator<Item = &'a Fill>) -> Vec<&'a Fill> {
    let mut out: Vec<&Fill> = Vec::new();
    for f in fills {
        if !matches!(f, Fill::Solid(_)) && !out.iter().any(|g| **g == *f) {
            out.push(f);
        }
    }
    out
}
