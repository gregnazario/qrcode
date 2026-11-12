//! SVG serialization of a VectorScene.

use super::common::{collect_gradients, linear_endpoints};
use crate::render::to_path;
use crate::scene::{EmbeddedImage, Fill, FillRule, PathCmd, Shape, VectorScene};
use base64::Engine as _;
use std::fmt::Write as _;

pub fn write(scene: &VectorScene) -> Vec<u8> {
    let s = scene.size;
    let mut defs = String::new();
    let mut body = String::new();

    let mut all_fills: Vec<&Fill> = scene.background.iter().collect();
    all_fills.extend(scene.items.iter().map(|i| &i.fill));
    let grads = collect_gradients(all_fills.into_iter());
    // Each collected gradient is emitted exactly once (stable ids g0, g1, ...);
    // body items reference them via url(#gN).
    for (id, g) in grads.iter().enumerate() {
        write_gradient_def(id, g, s, &mut defs);
    }

    if let Some(bg) = &scene.background {
        let _ = writeln!(
            body,
            r#"<path d="M 0 0 H {s} V {s} H 0 Z" {}/>"#,
            fill_attrs(bg, &grads)
        );
    }
    for item in &scene.items {
        let d = path_d(&item.shape);
        let _ = writeln!(
            body,
            r#"<path d="{d}"{} {}/>"#,
            if item.fill_rule == FillRule::EvenOdd {
                r#" fill-rule="evenodd""#
            } else {
                ""
            },
            fill_attrs(&item.fill, &grads)
        );
    }
    if let Some(img) = &scene.logo {
        let _ = writeln!(
            body,
            r#"<image x="{}" y="{}" width="{}" height="{}" href="{}"/>"#,
            f(img.x),
            f(img.y),
            f(img.w),
            f(img.h),
            data_uri(img)
        );
    }

    let defs_block = if defs.is_empty() {
        String::new()
    } else {
        format!("<defs>\n{defs}</defs>\n")
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{s}\" height=\"{s}\" viewBox=\"0 0 {s} {s}\" shape-rendering=\"geometricPrecision\">\n{defs_block}{body}</svg>\n",
    )
    .into_bytes()
}

fn fill_attrs(fill: &Fill, grads: &[&Fill]) -> String {
    match fill {
        Fill::Solid(c) => {
            if c.a == 255 {
                format!(r#"fill="{}""#, c.to_hex6())
            } else {
                format!(
                    r#"fill="{}" fill-opacity="{}""#,
                    c.to_hex6(),
                    f(c.a as f32 / 255.0)
                )
            }
        }
        g @ (Fill::LinearGradient { .. } | Fill::RadialGradient { .. }) => {
            let id = grads
                .iter()
                .position(|x| **x == *g)
                .expect("gradient collected");
            format!(r#"fill="url(#g{id})""#)
        }
    }
}

fn write_gradient_def(id: usize, fill: &Fill, size: f32, defs: &mut String) {
    let stop = |tag: &str, c: &crate::scene::Rgba| {
        if c.a == 255 {
            format!(r#"<stop offset="{tag}" stop-color="{}"/>"#, c.to_hex6())
        } else {
            format!(
                r#"<stop offset="{tag}" stop-color="{}" stop-opacity="{}"/>"#,
                c.to_hex6(),
                f(c.a as f32 / 255.0)
            )
        }
    };
    match fill {
        Fill::LinearGradient {
            from,
            to,
            angle_deg,
        } => {
            let ((x0, y0), (x1, y1)) = linear_endpoints(size, *angle_deg);
            let _ = writeln!(
                defs,
                r#"<linearGradient id="g{id}" gradientUnits="userSpaceOnUse" x1="{}" y1="{}" x2="{}" y2="{}">{}{}</linearGradient>"#,
                f(x0),
                f(y0),
                f(x1),
                f(y1),
                stop("0", from),
                stop("1", to)
            );
        }
        Fill::RadialGradient { from, to } => {
            let c = f(size / 2.0);
            let _ = writeln!(
                defs,
                r#"<radialGradient id="g{id}" gradientUnits="userSpaceOnUse" cx="{c}" cy="{c}" r="{c}">{}{}</radialGradient>"#,
                stop("0", from),
                stop("1", to)
            );
        }
        Fill::Solid(_) => unreachable!(),
    }
}

fn path_d(shape: &Shape) -> String {
    let mut d = String::new();
    for cmd in to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => {
                let _ = write!(d, "M {} {} ", f(x), f(y));
            }
            PathCmd::Line(x, y) => {
                let _ = write!(d, "L {} {} ", f(x), f(y));
            }
            PathCmd::Cubic(a, b, c, e, g, h) => {
                let _ = write!(
                    d,
                    "C {} {} {} {} {} {} ",
                    f(a),
                    f(b),
                    f(c),
                    f(e),
                    f(g),
                    f(h)
                );
            }
            PathCmd::Close => d.push_str("Z "),
        }
    }
    d.trim_end().to_string()
}

fn data_uri(img: &EmbeddedImage) -> String {
    let mime = match img.format {
        crate::scene::ImageFormat::Png => "image/png",
        crate::scene::ImageFormat::Jpeg => "image/jpeg",
    };
    format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&img.bytes)
    )
}

fn f(v: f32) -> String {
    format!("{v:.3}")
}
