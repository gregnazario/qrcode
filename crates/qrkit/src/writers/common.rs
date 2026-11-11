//! Geometry helpers shared by all writers.

use crate::scene::Fill;

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
