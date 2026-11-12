//! Timing harness for the writers (run: cargo run -p qrkit --release --example bench).
//!
//! Times each writer on three representative scenes (scenes are built once, outside the timed loops):
//! small plain, large dense (v10 payload), and gradient+logo.

use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::scene::Fill::{LinearGradient, Solid};
use qrkit::scene::Rgba;
use qrkit::style::{Ecc, StyleConfig};
use std::time::Instant;

const ITERS: usize = 200;

fn bench(name: &str, f: impl Fn() -> usize) {
    // warmup
    let _ = f();
    let t = Instant::now();
    let mut last = 0;
    for _ in 0..ITERS {
        last = f();
    }
    let per = t.elapsed().as_micros() as f64 / ITERS as f64;
    println!("{name:<28} {per:>9.1} us/iter   output {last} bytes");
}

fn main() {
    let logo = LogoConfig::from_bytes(include_bytes!("../tests/fixtures/logo.png").to_vec()).ok();

    let scenes: Vec<(&str, qrkit::builder::QrImage)> = vec![
        (
            "plain v2 text",
            QrBuilder::new(Content::Text("hello world".into())).unwrap().build().unwrap(),
        ),
        (
            "dense v10 url",
            QrBuilder::new(Content::Url("https://example.com/very/long/path?query=abcdefghijklmnopqrstuvwxyz0123456789&more=data&even=more&padding=xyz".into())).unwrap().build().unwrap(),
        ),
        (
            "gradient+logo",
            QrBuilder::new(Content::Text("gradient logo scene".into())).unwrap()
                .style(StyleConfig::new()
                    .ecc(Ecc::High)
                    .fill(LinearGradient { from: Rgba { r: 13, g: 71, b: 161, a: 255 }, to: Rgba { r: 66, g: 165, b: 245, a: 255 }, angle_deg: 45.0 })
                    .background(Some(Solid(Rgba::WHITE)))
                    .logo(logo))
                .build().unwrap(),
        ),
    ];

    for (name, img) in &scenes {
        println!("--- {name} ({} items) ---", img.scene.items.len());
        bench("svg::write", || {
            qrkit::writers::svg::write(&img.scene).len()
        });
        bench("pdf::write", || {
            qrkit::writers::pdf::write(&img.scene).len()
        });
        bench("eps::write", || {
            qrkit::writers::eps::write(&img.scene).len()
        });
    }
}
