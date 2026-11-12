//! PNG raster output: the bridge from the vector scene to image formats.

use super::raster::rasterize;
use crate::scene::VectorScene;

/// Serialize a scene to a PNG at `px_per_module` pixels per module unit.
/// Backgrounds are painted over white, so transparent-background scenes
/// still produce an opaque, scanner-friendly image.
pub fn write(scene: &VectorScene, px_per_module: u32) -> Vec<u8> {
    let pm = rasterize(scene, px_per_module);
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, pm.width(), pm.height());
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().expect("png header");
        writer.write_image_data(pm.data()).expect("png pixel write");
    }
    out
}
