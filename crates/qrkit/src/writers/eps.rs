//! PostScript Level 3 EPS writer.
//!
//! EPS has no alpha: every color is pre-blended against the effective
//! background (scene background if solid, else white) before writing.

use super::common::linear_endpoints;
use crate::logo::decode_rgba;
use crate::render::to_path;
use crate::scene::{EmbeddedImage, Fill, FillRule, PathCmd, Rgba, SceneItem, Shape, VectorScene};
use std::fmt::Write as _;

/// Serialize a scene to a standalone EPS document.
pub fn write(scene: &VectorScene) -> Vec<u8> {
    let s = scene.size;
    let fy = |y: f32| s - y; // scene origin is top-left; PostScript is bottom-left
    let bg = match &scene.background {
        Some(Fill::Solid(c)) => *c,
        _ => Rgba::WHITE, // transparent -> white in EPS (spec)
    };
    let blend = |c: &Rgba| blend_over(*c, bg);

    let mut out = String::new();
    let _ = writeln!(out, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(
        out,
        "%%BoundingBox: 0 0 {} {}",
        s.ceil() as i32,
        s.ceil() as i32
    );
    let _ = writeln!(out, "%%Creator: qrkit");
    let _ = writeln!(out, "%%LanguageLevel: 3");
    let _ = writeln!(out, "%%EndComments");

    if let Some(Fill::Solid(c)) = &scene.background {
        // The backdrop for the background itself is white paper, not the
        // background color (blending a color against itself is a no-op).
        set_color(&blend_over(*c, Rgba::WHITE), &mut out);
        let _ = writeln!(out, "0 0 {s:.3} {s:.3} rectfill");
    } else {
        // transparent background: leave unpainted (white paper)
    }

    // Gradient items paint through a clip of the just-built path. PostScript
    // `clip` uses the nonzero winding rule, which would close the even-odd
    // holes of eye rings (their hole subpath has winding 2 and falls inside
    // the clip); `eoclip` honors the item's fill rule instead.
    let clip_op = |item: &SceneItem| {
        if item.fill_rule == FillRule::EvenOdd {
            "eoclip"
        } else {
            "clip"
        }
    };

    for item in &scene.items {
        emit_path(&item.shape, &fy, &mut out);
        match &item.fill {
            Fill::Solid(c) => {
                set_color(&blend(c), &mut out);
                // The op line carries a leading space so painted operators
                // read as " fill"/" eofill" tokens.
                let op = if item.fill_rule == FillRule::EvenOdd {
                    "eofill"
                } else {
                    "fill"
                };
                let _ = writeln!(out, " {op}");
            }
            Fill::LinearGradient {
                from,
                to,
                angle_deg,
            } => {
                let ((x0, y0), (x1, y1)) = linear_endpoints(s, *angle_deg);
                let _ = writeln!(out, " gsave {}", clip_op(item));
                let _ = writeln!(
                    out,
                    "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >> shfill",
                    x0,
                    fy(y0),
                    x1,
                    fy(y1),
                    ps_function(&blend(from), &blend(to))
                );
                let _ = writeln!(out, "grestore");
            }
            Fill::RadialGradient { from, to } => {
                let c = s / 2.0;
                let _ = writeln!(out, " gsave {}", clip_op(item));
                let _ = writeln!(
                    out,
                    "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} 0 {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >> shfill",
                    c,
                    fy(c),
                    c,
                    fy(c),
                    c, // end radius matches PDF/SVG (full half-size)
                    ps_function(&blend(from), &blend(to))
                );
                let _ = writeln!(out, "grestore");
            }
        }
    }

    if let Some(img) = &scene.logo {
        embed_image(img, &fy, bg, &mut out);
    }

    let _ = writeln!(out, "%%EOF");
    out.into_bytes()
}

/// Source-over composite of `c` on the opaque backdrop `bg`; the result is
/// always opaque (EPS has no alpha).
///
/// 8-bit alpha maps to `k/256`, so the halfway alpha 128 composites to an
/// exact 0.5 mix: a 50% black tint over white rounds to 128 = "0.502".
/// Fully opaque colors pass through unchanged.
fn blend_over(c: Rgba, bg: Rgba) -> Rgba {
    if c.a == 255 {
        return c;
    }
    let a = c.a as f32 / 256.0;
    let mix = |fg: u8, b: u8| (fg as f32 * a + b as f32 * (1.0 - a)).round() as u8;
    Rgba {
        r: mix(c.r, bg.r),
        g: mix(c.g, bg.g),
        b: mix(c.b, bg.b),
        a: 255,
    }
}

fn set_color(c: &Rgba, out: &mut String) {
    let _ = writeln!(
        out,
        "{:.3} {:.3} {:.3} setrgbcolor",
        c.r as f32 / 255.0,
        c.g as f32 / 255.0,
        c.b as f32 / 255.0
    );
}

/// Type-2 interpolation function between two pre-blended RGB stops.
fn ps_function(from: &Rgba, to: &Rgba) -> String {
    format!(
        "<< /FunctionType 2 /Domain [0 1] /C0 [{:.3} {:.3} {:.3}] /C1 [{:.3} {:.3} {:.3}] /N 1 >>",
        from.r as f32 / 255.0,
        from.g as f32 / 255.0,
        from.b as f32 / 255.0,
        to.r as f32 / 255.0,
        to.g as f32 / 255.0,
        to.b as f32 / 255.0
    )
}

fn emit_path(shape: &Shape, fy: &impl Fn(f32) -> f32, out: &mut String) {
    out.push_str("newpath\n");
    for cmd in to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => {
                let _ = writeln!(out, "{:.3} {:.3} moveto", x, fy(y));
            }
            PathCmd::Line(x, y) => {
                let _ = writeln!(out, "{:.3} {:.3} lineto", x, fy(y));
            }
            PathCmd::Cubic(a, b, c, d, e, g) => {
                let _ = writeln!(
                    out,
                    "{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} curveto",
                    a,
                    fy(b),
                    c,
                    fy(d),
                    e,
                    fy(g)
                );
            }
            PathCmd::Close => out.push_str("closepath\n"),
        }
    }
}

fn embed_image(img: &EmbeddedImage, fy: &impl Fn(f32) -> f32, bg: Rgba, out: &mut String) {
    // An undecodable logo is skipped, not fatal — same behavior as pdf.rs.
    let Some((w, h, rgba)) = decode_rgba(img.format, &img.bytes).ok() else {
        return;
    };
    // PostScript bottom-left origin: translate to the logo's bottom edge in
    // flipped scene coords; the ImageMatrix row flip keeps pixel row 0 on top.
    let _ = writeln!(
        out,
        "gsave {:.3} {:.3} translate {:.3} {:.3} scale",
        img.x,
        fy(img.y) - img.h,
        img.w,
        img.h
    );
    let _ = writeln!(out, "/DeviceRGB setcolorspace");
    let _ = writeln!(
        out,
        "<< /ImageType 1 /Width {w} /Height {h} /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /ImageMatrix [{w} 0 0 {neg_h} 0 {h}] /DataSource currentfile /ASCIIHexDecode filter >> image",
        w = w,
        h = h,
        neg_h = -(h as i32)
    );
    // interleaved RGB hex, alpha pre-blended against bg, lines <= 120 chars
    let mut line = String::new();
    for px in rgba.chunks(4) {
        let c = blend_over(
            Rgba {
                r: px[0],
                g: px[1],
                b: px[2],
                a: px[3],
            },
            bg,
        );
        let _ = write!(line, "{:02x}{:02x}{:02x}", c.r, c.g, c.b);
        if line.len() >= 120 {
            let _ = writeln!(out, "{line}");
            line.clear();
        }
    }
    if !line.is_empty() {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, ">"); // ASCIIHexDecode EOD
    let _ = writeln!(out, "grestore");
}
