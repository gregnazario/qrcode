//! Regenerates `tests/fixtures/logo.png` (4x3 RGBA test image).
//!
//! Run from the repo root: `cargo run -p qrkit --example make_fixture`

fn main() {
    let px: Vec<u8> = vec![
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 255,
        0, 128, 0, 0, 255, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255,
        255, 255, 255, 255,
    ];
    let file = std::fs::File::create("crates/qrkit/tests/fixtures/logo.png").unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 4, 3);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&px).unwrap();
}
