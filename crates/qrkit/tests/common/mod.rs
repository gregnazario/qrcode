//! Test helpers for end-to-end validation.
//!
//! `rasterize` delegates to the PRODUCTION rasterizer (writers::raster) so
//! the e2e suites validate the code users actually run; `decode` reads the
//! first QR code back out with rqrr.
//
// Each test binary compiles this module independently; some use only one
// helper, so allow unused items here.
#![allow(dead_code)]

use qrkit::scene::VectorScene;
use tiny_skia::Pixmap;

/// Rasterize a scene to a Pixmap of (approximately) `px` pixels square.
///
/// This helper's contract is TOTAL size (the e2e suites pass 256/512/1024);
/// the production rasterizer takes pixels-per-module, so convert.
pub fn rasterize(scene: &VectorScene, px: u32) -> Pixmap {
    let per_module = ((px as f32 / scene.size).ceil() as u32).max(1);
    qrkit::writers::raster::rasterize(scene, per_module)
}

/// Decode the first QR code in a rasterized pixmap; panics on failure.
pub fn decode(pm: &Pixmap) -> String {
    let w = pm.width() as usize;
    let gray: Vec<u8> = pm
        .pixels()
        .iter()
        .map(|p| {
            let c = p.demultiply();
            // Rec.601 luminance.
            (0.299 * c.red() as f32 + 0.587 * c.green() as f32 + 0.114 * c.blue() as f32) as u8
        })
        .collect();
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(
        pm.width() as usize,
        pm.height() as usize,
        |x, y| gray[y * w + x],
    );
    let mut grids = img.detect_grids();
    let grid = grids
        .pop()
        .expect("no QR code detected in rasterized scene");
    let (_meta, content) = grid.decode().expect("QR decode failed");
    content
}
