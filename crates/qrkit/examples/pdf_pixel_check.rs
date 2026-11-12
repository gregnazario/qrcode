//! Writes the translucent-background PDF used for the alpha-isolation
//! regression check (run: cargo run -p qrkit --example pdf_pixel_check;
//! writes /tmp/pdfcheck.pdf). Then verify the rendered pixels manually:
//!
//! ```console
//! qlmanage -t -s 512 -o /tmp /tmp/pdfcheck.pdf
//! # with PIL: composite over white — quiet-zone pixels must be exactly
//! # (127,127,127) (50% black bg over white paper) and module pixels < 40
//! # (fully black, NOT dimmed by the leaked /ca).
//! ```

use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::scene::{Fill, Rgba};

fn main() {
    let img = QrBuilder::new(Content::Text("pixel check".into()))
        .unwrap()
        .build()
        .unwrap();
    let mut scene = img.scene;
    scene.background = Some(Fill::Solid(Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 128,
    }));
    std::fs::write("/tmp/pdfcheck.pdf", qrkit::writers::pdf::write(&scene)).unwrap();
    println!("pdf written");
}
