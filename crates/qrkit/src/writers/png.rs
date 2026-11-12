//! PNG raster output: the bridge from the vector scene to image formats.

use super::raster::rasterize;
use crate::scene::VectorScene;

/// Serialize a scene to a PNG at `px_per_module` pixels per module unit.
/// Backgrounds follow the style: a solid background renders opaque, a
/// transparent background (`--bg transparent`) yields a transparent PNG
/// with straight (demultiplied) alpha.
pub fn write(scene: &VectorScene, px_per_module: u32) -> Vec<u8> {
    let pm = rasterize(scene, px_per_module);
    // tiny-skia stores premultiplied RGBA; PNG wants straight alpha.
    let straight: Vec<u8> = pm
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply(); // ColorU8: straight u8 components
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, pm.width(), pm.height());
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().expect("png header");
        writer.write_image_data(&straight).expect("png pixel write");
    }
    out
}
