//! Minimal PDF 1.7 writer: single vector page, uncompressed streams.
//!
//! Object layout (ids strictly in write order, never renumbered):
//! 1 catalog, 2 pages, 3 page, 4 content stream, then one shading + pattern
//! object pair per unique gradient, one ExtGState object per unique fill
//! alpha below 1, then the logo image XObject (plus a gray /SMask object
//! when a PNG logo has translucency). The whole document is assembled as a
//! `Vec<u8>` because image streams are raw binary.

use super::common::{collect_gradients, linear_endpoints};
use crate::logo::decode_rgba;
use crate::render::to_path_into;
use crate::scene::{EmbeddedImage, Fill, FillRule, ImageFormat, PathCmd, Rgba, Shape, VectorScene};
use std::fmt::Write as _;

/// Serialize a scene to a single-page PDF 1.7 document.
pub fn write(scene: &VectorScene) -> Vec<u8> {
    let s = scene.size;
    let fy = |y: f32| s - y; // scene origin is top-left; PDF is bottom-left

    // ---- collect resources ----
    let mut all_fills: Vec<&Fill> = scene.background.iter().collect();
    all_fills.extend(scene.items.iter().map(|i| &i.fill));
    let grads = collect_gradients(all_fills.into_iter());
    // Unique solid-fill opacities < 1, in first-use order; content stream
    // ops `/A{j} gs` and resource entries `/A{j} <id> 0 R` share this index.
    let mut alphas: Vec<f32> = Vec::new();

    // Decode the logo up front: content, resources, and object numbering all
    // depend on whether an image (and its SMask) is embedded. A logo whose
    // pixels cannot be decoded is skipped rather than fatal.
    let logo = scene.logo.as_ref().and_then(logo_streams);

    // ---- content stream ----
    // Fills carrying extended graphics state — translucency (`/A{j} gs`) or
    // a gradient pattern (`/Pattern cs … scn`) — would leak into later
    // painting, so those runs are wrapped in `q … Q`. Opaque solid runs need
    // no isolation: `rg` fully replaces the fill color, so consecutive items
    // with the same fill are batched — color set once, then all their paths.
    // Groups only ever span consecutive items, so paint order is unchanged.
    let mut content = String::new();
    let _ = writeln!(content, "0.000 0.000 0.000 rg"); // deterministic initial fill
    let mut ext_open = false;
    let mut run_fill: Option<&Fill> = None;
    let mut cmds = Vec::new(); // scratch buffer reused across items
    if let Some(bg) = &scene.background {
        begin_run(
            bg,
            &grads,
            &mut alphas,
            &mut run_fill,
            &mut ext_open,
            &mut content,
        );
        let _ = writeln!(content, "0 0 {s:.3} {s:.3} re f");
    }
    for item in &scene.items {
        begin_run(
            &item.fill,
            &grads,
            &mut alphas,
            &mut run_fill,
            &mut ext_open,
            &mut content,
        );
        emit_path(&item.shape, &fy, item.fill_rule, &mut cmds, &mut content);
    }
    if ext_open {
        content.push_str("Q\n");
    }
    if let Some(img) = &scene.logo {
        if logo.is_some() {
            let _ = writeln!(
                content,
                "q {:.3} 0 0 {:.3} {:.3} {:.3} cm /Im0 Do Q",
                img.w,
                img.h,
                img.x,
                fy(img.y) - img.h
            );
        }
    }

    // ---- object ids (fixed order: gradients, alphas, image, smask) ----
    let pattern_id = |i: usize| 5 + 2 * i + 1; // shading object is 5 + 2*i
    let first_gs_id = 5 + 2 * grads.len();
    let gs_id = |j: usize| first_gs_id + j;
    let image_id = first_gs_id + alphas.len();
    let smask_id = image_id + 1; // referenced only when the PNG carries alpha

    // ---- page resources ----
    let mut resources = String::from("<<");
    if !grads.is_empty() {
        let entries: Vec<String> = (0..grads.len())
            .map(|i| format!("/P{i} {} 0 R", pattern_id(i)))
            .collect();
        let _ = write!(resources, " /Pattern << {} >>", entries.join(" "));
    }
    if !alphas.is_empty() {
        let entries: Vec<String> = (0..alphas.len())
            .map(|j| format!("/A{j} {} 0 R", gs_id(j)))
            .collect();
        let _ = write!(resources, " /ExtGState << {} >>", entries.join(" "));
    }
    if logo.is_some() {
        let _ = write!(resources, " /XObject << /Im0 {image_id} 0 R >>");
    }
    resources.push_str(" >>");

    // ---- auxiliary objects (bodies without the `N 0 obj` framing) ----
    let mut aux: Vec<Vec<u8>> = Vec::new();
    for (i, g) in grads.iter().enumerate() {
        let shading_id = pattern_id(i) - 1;
        let (from, to) = gradient_stops(g);
        let shading = match **g {
            Fill::LinearGradient { angle_deg, .. } => {
                let ((x0, y0), (x1, y1)) = linear_endpoints(s, angle_deg);
                format!(
                    "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >>",
                    x0,
                    fy(y0),
                    x1,
                    fy(y1),
                    stitch_function(from, to)
                )
            }
            Fill::RadialGradient { .. } => {
                let c = s / 2.0;
                format!(
                    "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} 0 {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >>",
                    c,
                    fy(c),
                    c,
                    fy(c),
                    c,
                    stitch_function(from, to)
                )
            }
            Fill::Solid(_) => unreachable!("collect_gradients skips solid fills"),
        };
        aux.push(shading.into_bytes());
        aux.push(format!("<< /PatternType 2 /Shading {shading_id} 0 R >>").into_bytes());
    }
    for a in &alphas {
        aux.push(format!("<< /Type /ExtGState /ca {a:.3} >>").into_bytes());
    }
    if let Some(stream) = &logo {
        let img = scene.logo.as_ref().expect("logo streams imply a logo");
        match stream {
            LogoStreams::Jpeg(bytes) => {
                let mut body = format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                    img.pixel_w,
                    img.pixel_h,
                    bytes.len()
                )
                .into_bytes();
                body.extend_from_slice(bytes); // raw binary, never through String
                body.extend_from_slice(b"\nendstream");
                aux.push(body);
            }
            LogoStreams::Png { rgb, smask } => {
                let smask_ref = match smask {
                    Some(_) => format!(" /SMask {smask_id} 0 R"),
                    None => String::new(),
                };
                let mut body = format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {}{smask_ref} >>\nstream\n",
                    img.pixel_w,
                    img.pixel_h,
                    rgb.len()
                )
                .into_bytes();
                body.extend_from_slice(rgb); // raw binary, never through String
                body.extend_from_slice(b"\nendstream");
                aux.push(body);
                if let Some(alpha) = smask {
                    let mut body = format!(
                        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {} >>\nstream\n",
                        img.pixel_w,
                        img.pixel_h,
                        alpha.len()
                    )
                    .into_bytes();
                    body.extend_from_slice(alpha);
                    body.extend_from_slice(b"\nendstream");
                    aux.push(body);
                }
            }
        }
    }

    // ---- assemble (ids 1.. in push order; offsets = byte positions) ----
    let mut out: Vec<u8> = Vec::new();
    // Header plus the conventional binary-marker comment line.
    out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets: Vec<usize> = Vec::new();

    let catalog = "<< /Type /Catalog /Pages 2 0 R >>";
    let pages = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>";
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {s:.3} {s:.3}] /Resources {resources} /Contents 4 0 R >>"
    );
    let contents = format!(
        "<< /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    );

    push_obj(&mut out, &mut offsets, catalog.as_bytes());
    push_obj(&mut out, &mut offsets, pages.as_bytes());
    push_obj(&mut out, &mut offsets, page.as_bytes());
    push_obj(&mut out, &mut offsets, contents.as_bytes());
    for body in &aux {
        push_obj(&mut out, &mut offsets, body);
    }

    // ---- xref & trailer ----
    let xref_pos = out.len();
    let count = offsets.len() + 1; // objects plus the free head entry
    let mut xref = format!("xref\n0 {count}\n0000000000 65535 f \n");
    for off in &offsets {
        let _ = writeln!(xref, "{off:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());
    out
}

/// Append one indirect object; its id is its 1-based position in `offsets`.
fn push_obj(out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]) {
    offsets.push(out.len());
    out.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}

/// Fully decoded logo sample data ready for stream embedding.
enum LogoStreams {
    /// Verbatim JPEG bytes (embedded with /DCTDecode).
    Jpeg(Vec<u8>),
    /// Interleaved RGB8 samples plus a gray alpha mask when any pixel is
    /// translucent (`None` when the PNG is fully opaque).
    Png {
        rgb: Vec<u8>,
        smask: Option<Vec<u8>>,
    },
}

/// Decode a logo for embedding; `None` when a PNG cannot be decoded.
fn logo_streams(img: &EmbeddedImage) -> Option<LogoStreams> {
    match img.format {
        ImageFormat::Jpeg => Some(LogoStreams::Jpeg(img.bytes.clone())),
        ImageFormat::Png => {
            let (w, h, rgba) = decode_rgba(img.format, &img.bytes).ok()?;
            let px = (w as usize) * (h as usize);
            let mut rgb = Vec::with_capacity(px * 3);
            let mut alpha = Vec::with_capacity(px);
            for p in rgba.chunks_exact(4) {
                rgb.extend_from_slice(&p[0..3]);
                alpha.push(p[3]);
            }
            let smask = if alpha.iter().any(|&a| a != 255) {
                Some(alpha)
            } else {
                None
            };
            Some(LogoStreams::Png { rgb, smask })
        }
    }
}

fn gradient_stops(fill: &Fill) -> (Rgba, Rgba) {
    match *fill {
        Fill::LinearGradient { from, to, .. } | Fill::RadialGradient { from, to } => (from, to),
        Fill::Solid(_) => unreachable!("collect_gradients skips solid fills"),
    }
}

/// Type-2 interpolation function between two RGB stops.
fn stitch_function(from: Rgba, to: Rgba) -> String {
    let c = |x: u8| x as f32 / 255.0;
    format!(
        "<< /FunctionType 2 /Domain [0 1] /C0 [{:.3} {:.3} {:.3}] /C1 [{:.3} {:.3} {:.3}] /N 1 >>",
        c(from.r),
        c(from.g),
        c(from.b),
        c(to.r),
        c(to.g),
        c(to.b)
    )
}

/// True when a fill sets graphics state that outlives its own painting
/// (fill alpha via ExtGState, or the pattern colorspace) and therefore
/// must run inside an isolated `q … Q` block.
fn needs_isolation(fill: &Fill) -> bool {
    match fill {
        Fill::Solid(c) => c.a != 255,
        _ => true, // gradient pattern: /Pattern cs … scn
    }
}

/// Start a fill run: close any open isolated block, open a new one when the
/// fill carries extended state, and set the color (once per run).
fn begin_run<'a>(
    fill: &'a Fill,
    grads: &[&Fill],
    alphas: &mut Vec<f32>,
    run_fill: &mut Option<&'a Fill>,
    ext_open: &mut bool,
    content: &mut String,
) {
    if *run_fill == Some(fill) {
        return;
    }
    if *ext_open {
        content.push_str("Q\n");
        *ext_open = false;
    }
    if needs_isolation(fill) {
        content.push_str("q\n");
        *ext_open = true;
    }
    set_fill_color(fill, grads, alphas, content);
    *run_fill = Some(fill);
}

/// Emit the fill-color operator for one fill (and intern its alpha).
fn set_fill_color(fill: &Fill, grads: &[&Fill], alphas: &mut Vec<f32>, out: &mut String) {
    match fill {
        Fill::Solid(c) => {
            let _ = writeln!(
                out,
                "{:.3} {:.3} {:.3} rg",
                c.r as f32 / 255.0,
                c.g as f32 / 255.0,
                c.b as f32 / 255.0
            );
            if c.a != 255 {
                let j = intern_alpha(alphas, c.a as f32 / 255.0);
                let _ = writeln!(out, "/A{j} gs");
            }
        }
        g @ (Fill::LinearGradient { .. } | Fill::RadialGradient { .. }) => {
            let i = grads
                .iter()
                .position(|x| **x == *g)
                .expect("gradient collected");
            let _ = writeln!(out, "/Pattern cs /P{i} scn");
        }
    }
}

/// Deduplicated alpha registration; returns the shared `/A{j}` index.
fn intern_alpha(alphas: &mut Vec<f32>, a: f32) -> usize {
    alphas
        .iter()
        .position(|x| (*x - a).abs() < 1e-6)
        .unwrap_or_else(|| {
            alphas.push(a);
            alphas.len() - 1
        })
}

/// Emit path construction operators plus the fill operator for `rule`.
fn emit_path(
    shape: &Shape,
    fy: &impl Fn(f32) -> f32,
    rule: FillRule,
    cmds: &mut Vec<PathCmd>,
    out: &mut String,
) {
    to_path_into(shape, cmds);
    for cmd in cmds.iter().copied() {
        match cmd {
            PathCmd::Move(x, y) => {
                let _ = writeln!(out, "{x:.3} {:.3} m", fy(y));
            }
            PathCmd::Line(x, y) => {
                let _ = writeln!(out, "{x:.3} {:.3} l", fy(y));
            }
            PathCmd::Cubic(x1, y1, x2, y2, x, y) => {
                let _ = writeln!(
                    out,
                    "{x1:.3} {:.3} {x2:.3} {:.3} {x:.3} {:.3} c",
                    fy(y1),
                    fy(y2),
                    fy(y)
                );
            }
            PathCmd::Close => out.push_str("h\n"),
        }
    }
    out.push_str(match rule {
        FillRule::NonZero => "f\n",
        FillRule::EvenOdd => "f*\n",
    });
}
