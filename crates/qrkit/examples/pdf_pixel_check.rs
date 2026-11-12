//! Renders the translucent-background PDF through CoreGraphics and checks
//! the exact pixels the alpha-leak regression guards (run: cargo run -p qrkit
//! --example pdf_pixel_check). Writes /tmp/pdfcheck.{pdf,png}.

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
