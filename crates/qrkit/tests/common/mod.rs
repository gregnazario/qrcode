//! Test-only rasterizer and decoder for end-to-end scene validation.
//!
//! Shared by `tests/e2e_decode.rs` (Task 9) and the CLI golden tests
//! (Task 12): rasterize a [`VectorScene`] with tiny-skia, then decode the
//! resulting QR code with rqrr to assert the payload round-trips.
//
// Each test binary compiles this module independently; some use only the
// decoder (e.g. term_writer), so allow unused helpers here.
#![allow(dead_code)]

use qrkit::scene::{Fill, FillRule, PathCmd, Shape, VectorScene};
use qrkit::writers::common::linear_endpoints;
use tiny_skia::{
    Color, FillRule as SkFillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap,
    RadialGradient, Rect, Shader, SpreadMode, Transform,
};

/// Rasterize a scene to a Pixmap of `px` pixels square.
pub fn rasterize(scene: &VectorScene, px: u32) -> Pixmap {
    let scale = px as f32 / scene.size;
    let mut pm = Pixmap::new(px, px).expect("pixmap");
    let ts = Transform::from_scale(scale, scale);

    // Background first, then items in order, then the logo on top.
    if let Some(bg) = &scene.background {
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(0.0, 0.0, scene.size, scene.size).unwrap());
        let path = pb.finish().unwrap();
        pm.fill_path(&path, &paint(bg, scene), SkFillRule::Winding, ts, None);
    }
    for item in &scene.items {
        let path = scene_path(&item.shape);
        let rule = match item.fill_rule {
            FillRule::NonZero => SkFillRule::Winding,
            FillRule::EvenOdd => SkFillRule::EvenOdd,
        };
        pm.fill_path(&path, &paint(&item.fill, scene), rule, ts, None);
    }
    if let Some(img) = &scene.logo {
        let (w, h, rgba) = qrkit::logo::decode_rgba(img.format, &img.bytes).unwrap();
        for row in 0..h {
            for col in 0..w {
                let i = ((row * w + col) * 4) as usize;
                let color = Color::from_rgba(
                    rgba[i] as f32 / 255.0,
                    rgba[i + 1] as f32 / 255.0,
                    rgba[i + 2] as f32 / 255.0,
                    rgba[i + 3] as f32 / 255.0,
                )
                .unwrap();
                let x0 = img.x + img.w * col as f32 / w as f32;
                let y0 = img.y + img.h * row as f32 / h as f32;
                let cw = img.w / w as f32;
                let ch = img.h / h as f32;
                let mut pb = PathBuilder::new();
                pb.push_rect(Rect::from_xywh(x0, y0, cw + 0.01, ch + 0.01).unwrap());
                pm.fill_path(
                    &pb.finish().unwrap(),
                    &Paint {
                        shader: Shader::SolidColor(color),
                        anti_alias: false,
                        ..Default::default()
                    },
                    SkFillRule::Winding,
                    ts,
                    None,
                );
            }
        }
    }
    pm
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

fn paint(fill: &Fill, scene: &VectorScene) -> Paint<'static> {
    let shader = match fill {
        Fill::Solid(c) => Shader::SolidColor(sk_color(c)),
        Fill::LinearGradient {
            from,
            to,
            angle_deg,
        } => {
            let ((x0, y0), (x1, y1)) = linear_endpoints(scene.size, *angle_deg);
            let stops = vec![
                GradientStop::new(0.0, sk_color(from)),
                GradientStop::new(1.0, sk_color(to)),
            ];
            LinearGradient::new(
                (x0, y0).into(),
                (x1, y1).into(),
                stops,
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap()
        }
        Fill::RadialGradient { from, to } => {
            let c = scene.size / 2.0;
            let stops = vec![
                GradientStop::new(0.0, sk_color(from)),
                GradientStop::new(1.0, sk_color(to)),
            ];
            RadialGradient::new(
                (c, c).into(),
                (c, c).into(),
                c,
                stops,
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap()
        }
    };
    Paint {
        shader,
        ..Default::default()
    }
}

fn sk_color(c: &qrkit::scene::Rgba) -> Color {
    Color::from_rgba(
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0,
        c.a as f32 / 255.0,
    )
    .unwrap()
}

fn scene_path(shape: &Shape) -> Path {
    let mut pb = PathBuilder::new();
    for cmd in qrkit::render::to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => pb.move_to(x, y),
            PathCmd::Line(x, y) => pb.line_to(x, y),
            PathCmd::Cubic(c1x, c1y, c2x, c2y, x, y) => pb.cubic_to(c1x, c1y, c2x, c2y, x, y),
            PathCmd::Close => pb.close(),
        };
    }
    pb.finish().unwrap()
}
