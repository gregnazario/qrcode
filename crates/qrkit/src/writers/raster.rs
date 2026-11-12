//! Production rasterizer: a [`VectorScene`] to RGBA pixels via tiny-skia.
//!
//! Shared by the PNG writer, the terminal image protocols (kitty/iTerm2
//! payload both embed a PNG; sixel encodes the pixels directly), and the
//! end-to-end decode tests. Paints background, then items in scene order,
//! then the logo — the same order as every other writer.

use crate::scene::{Fill, FillRule, PathCmd, Rgba, Shape, VectorScene};
use crate::writers::common::linear_endpoints;
use tiny_skia::{
    Color, FillRule as SkFillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap,
    RadialGradient, Rect, Shader, SpreadMode, Transform,
};

/// Rasterize `scene` to a square pixmap with `px_per_module` pixels per
/// module unit (total = `px_per_module × scene.size`, rounded up).
pub fn rasterize(scene: &VectorScene, px_per_module: u32) -> Pixmap {
    let px = (scene.size * px_per_module as f32).ceil() as u32;
    let scale = px as f32 / scene.size;
    let mut pm = Pixmap::new(px, px).expect("pixmap dimensions in range");
    let ts = Transform::from_scale(scale, scale);

    if let Some(bg) = &scene.background {
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(0.0, 0.0, scene.size, scene.size).unwrap());
        let path = pb.finish().unwrap();
        pm.fill_path(
            &path,
            &paint(&Fill::Solid(Rgba::WHITE), scene),
            SkFillRule::Winding,
            ts,
            None,
        );
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
        if let Ok((w, h, rgba)) = crate::logo::decode_rgba(img.format, &img.bytes) {
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
    }
    pm
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

fn sk_color(c: &Rgba) -> Color {
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
    for cmd in crate::render::to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => pb.move_to(x, y),
            PathCmd::Line(x, y) => pb.line_to(x, y),
            PathCmd::Cubic(c1x, c1y, c2x, c2y, x, y) => pb.cubic_to(c1x, c1y, c2x, c2y, x, y),
            PathCmd::Close => pb.close(),
        };
    }
    pb.finish().unwrap()
}
