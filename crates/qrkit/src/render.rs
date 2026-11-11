//! QR matrix encoding and matrix -> VectorScene composition.

use crate::scene::{CornerRadii, EmbeddedImage, FillRule, PathCmd, SceneItem, Shape, VectorScene};
use crate::style::{Ecc, EyeShape, ModuleShape, StyleConfig, Version};
use crate::Error;

/// Thin wrapper over the qrcodegen matrix.
#[derive(Clone)]
pub struct Matrix {
    code: qrcodegen::QrCode,
}

// qrcodegen::QrCode does not implement Debug; expose the useful summary.
impl std::fmt::Debug for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Matrix")
            .field("size", &self.size())
            .finish()
    }
}

impl Matrix {
    pub fn encode(payload: &str, ecc: Ecc, version: Version) -> Result<Matrix, Error> {
        if let Version::Fixed(v) = version {
            if !(1..=40).contains(&v) {
                return Err(Error::InvalidStyle(format!(
                    "version {v} out of range 1..=40"
                )));
            }
        }
        let ecl = ecc.to_qrcodegen();
        // qrcodegen 1.8 has no `with_version`; pin the version by passing the
        // same value as both the minimum and maximum version.
        let code = match version {
            Version::Auto => qrcodegen::QrCode::encode_text(payload, ecl),
            Version::Fixed(v) => {
                let ver = qrcodegen::Version::new(v);
                let segs = qrcodegen::QrSegment::make_segments(payload);
                // boostecl=false: the version is pinned, so boosting ECC here
                // would silently override the level the caller explicitly chose.
                qrcodegen::QrCode::encode_segments_advanced(&segs, ecl, ver, ver, None, false)
            }
        }
        .map_err(|e| match e {
            qrcodegen::DataTooLong::SegmentTooLong
            | qrcodegen::DataTooLong::DataOverCapacity(..) => Error::TooLarge(
                "payload exceeds capacity; lower the ECC level, use auto version, or shorten the content"
                    .into(),
            ),
        })?;
        Ok(Matrix { code })
    }

    pub fn size(&self) -> i32 {
        self.code.size()
    }

    /// The error correction level actually used by the encoded matrix.
    pub fn ecc(&self) -> qrcodegen::QrCodeEcc {
        self.code.error_correction_level()
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        self.code.get_module(x, y)
    }
}

/// Compose the full scene for a matrix under a style.
///
/// Scene coordinates: module (0, 0) sits at (margin, margin), so the matrix
/// is centered inside the quiet zone and the logo box (centered on the scene)
/// lines up with the cleared module region.
pub fn render(matrix: &Matrix, style: &StyleConfig) -> VectorScene {
    let n = matrix.size();
    let margin = style.margin_modules as f32;
    let size = n as f32 + 2.0 * margin;
    let mut scene = VectorScene {
        size,
        background: style.background.clone(),
        items: Vec::new(),
        logo: None,
    };

    // Logo clearing box in module coordinates (centered).
    let logo_clear = style.logo.as_ref().map(|l| {
        let half = l.size_ratio * n as f32 / 2.0 + l.padding;
        (
            n as f32 / 2.0 - half,
            n as f32 / 2.0 - half,
            n as f32 / 2.0 + half,
            n as f32 / 2.0 + half,
        )
    });

    // Data modules (finder eyes handled separately).
    for y in 0..n {
        for x in 0..n {
            if !matrix.get(x, y) || in_finder(x, y, n) {
                continue;
            }
            if let Some((x0, y0, x1, y1)) = logo_clear {
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                if cx > x0 && cx < x1 && cy > y0 && cy < y1 {
                    continue;
                }
            }
            scene.items.push(module_item(x, y, style));
        }
    }

    // Finder eyes.
    for (ex, ey) in [(0, 0), (n - 7, 0), (0, n - 7)] {
        push_eye(&mut scene.items, ex, ey, style);
    }

    // Logo placement (contain-fit inside a centered square box).
    if let Some(l) = &style.logo {
        let box_side = l.size_ratio * n as f32;
        let scale = box_side / l.pixel_w.max(l.pixel_h) as f32;
        let (dw, dh) = (l.pixel_w as f32 * scale, l.pixel_h as f32 * scale);
        let c = size / 2.0;
        scene.logo = Some(EmbeddedImage {
            bytes: l.data.clone(),
            format: l.format,
            pixel_w: l.pixel_w,
            pixel_h: l.pixel_h,
            x: c - dw / 2.0,
            y: c - dh / 2.0,
            w: dw,
            h: dh,
        });
    }

    scene
}

fn in_finder(x: i32, y: i32, n: i32) -> bool {
    (x < 7 && y < 7) || (x >= n - 7 && y < 7) || (x < 7 && y >= n - 7)
}

fn module_item(x: i32, y: i32, style: &StyleConfig) -> SceneItem {
    let margin = style.margin_modules as f32;
    let (fx, fy) = (x as f32 + margin, y as f32 + margin);
    let shape = match style.shape {
        ModuleShape::Square => Shape::Rect {
            x: fx,
            y: fy,
            w: 1.0,
            h: 1.0,
        },
        ModuleShape::Rounded => Shape::RoundedRect {
            x: fx,
            y: fy,
            w: 1.0,
            h: 1.0,
            radii: CornerRadii::all(0.25),
        },
        ModuleShape::Dots { radius } => Shape::Circle {
            cx: fx + 0.5,
            cy: fy + 0.5,
            r: radius.clamp(0.1, 0.5),
        },
        ModuleShape::Classy => Shape::RoundedRect {
            x: fx,
            y: fy,
            w: 1.0,
            h: 1.0,
            radii: CornerRadii {
                tl: 0.0,
                tr: 0.5,
                br: 0.0,
                bl: 0.5,
            },
        },
    };
    SceneItem {
        shape,
        fill: style.fill.clone(),
        fill_rule: FillRule::NonZero,
    }
}

/// One finder eye at module (ex, ey), 7x7 modules: ring + inner 3x3 dot.
/// Both parts are emitted as Paths: writers serialize every shape through
/// [`to_path`], and eye geometry must not depend on the module shape style.
fn push_eye(items: &mut Vec<SceneItem>, ex: i32, ey: i32, style: &StyleConfig) {
    let margin = style.margin_modules as f32;
    let (x, y) = (ex as f32 + margin, ey as f32 + margin);
    let ring_fill = style.eye_fill.clone();
    let dot_fill = style
        .eye_inner_fill
        .clone()
        .unwrap_or_else(|| style.fill.clone());

    let (outer, inner_hole, dot) = match style.eye_shape {
        EyeShape::Square => (
            Shape::Rect {
                x,
                y,
                w: 7.0,
                h: 7.0,
            },
            Shape::Rect {
                x: x + 1.0,
                y: y + 1.0,
                w: 5.0,
                h: 5.0,
            },
            Shape::Rect {
                x: x + 2.0,
                y: y + 2.0,
                w: 3.0,
                h: 3.0,
            },
        ),
        EyeShape::Rounded => (
            Shape::RoundedRect {
                x,
                y,
                w: 7.0,
                h: 7.0,
                radii: CornerRadii::all(2.0),
            },
            Shape::RoundedRect {
                x: x + 1.0,
                y: y + 1.0,
                w: 5.0,
                h: 5.0,
                radii: CornerRadii::all(1.25),
            },
            Shape::RoundedRect {
                x: x + 2.0,
                y: y + 2.0,
                w: 3.0,
                h: 3.0,
                radii: CornerRadii::all(0.75),
            },
        ),
        EyeShape::Circle => (
            Shape::Circle {
                cx: x + 3.5,
                cy: y + 3.5,
                r: 3.5,
            },
            Shape::Circle {
                cx: x + 3.5,
                cy: y + 3.5,
                r: 2.5,
            },
            Shape::Circle {
                cx: x + 3.5,
                cy: y + 3.5,
                r: 1.5,
            },
        ),
        EyeShape::Leaf => (
            Shape::RoundedRect {
                x,
                y,
                w: 7.0,
                h: 7.0,
                radii: CornerRadii {
                    tl: 3.0,
                    tr: 0.0,
                    br: 3.0,
                    bl: 0.0,
                },
            },
            Shape::RoundedRect {
                x: x + 1.0,
                y: y + 1.0,
                w: 5.0,
                h: 5.0,
                radii: CornerRadii {
                    tl: 2.0,
                    tr: 0.0,
                    br: 2.0,
                    bl: 0.0,
                },
            },
            Shape::RoundedRect {
                x: x + 2.0,
                y: y + 2.0,
                w: 3.0,
                h: 3.0,
                radii: CornerRadii {
                    tl: 1.5,
                    tr: 0.0,
                    br: 1.5,
                    bl: 0.0,
                },
            },
        ),
    };

    // Ring = outer path + hole subpath, even-odd.
    items.push(SceneItem {
        shape: Shape::Path([shape_subpath(&outer), shape_subpath(&inner_hole)].concat()),
        fill: ring_fill,
        fill_rule: FillRule::EvenOdd,
    });
    items.push(SceneItem {
        shape: Shape::Path(shape_subpath(&dot)),
        fill: dot_fill,
        fill_rule: FillRule::NonZero,
    });
}

/// Convert Rect/RoundedRect/Circle to an equivalent Path subpath (closed).
/// Writers only serialize Paths; this is the single conversion point.
fn shape_subpath(shape: &Shape) -> Vec<PathCmd> {
    match *shape {
        Shape::Rect { x, y, w, h } => vec![
            PathCmd::Move(x, y),
            PathCmd::Line(x + w, y),
            PathCmd::Line(x + w, y + h),
            PathCmd::Line(x, y + h),
            PathCmd::Close,
        ],
        Shape::RoundedRect { x, y, w, h, radii } => {
            let k = 0.552_284_8; // circle-approximation constant for quarter arcs
            let (r_tl, r_tr, r_br, r_bl) = (radii.tl, radii.tr, radii.br, radii.bl);
            let (xr, yb) = (x + w, y + h);
            let mut p = Vec::with_capacity(4 + 12);
            p.push(PathCmd::Move(x + r_tl, y));
            p.push(PathCmd::Line(xr - r_tr, y));
            if r_tr > 0.0 {
                p.push(PathCmd::Cubic(
                    xr - r_tr * (1.0 - k),
                    y,
                    xr,
                    y + r_tr * (1.0 - k),
                    xr,
                    y + r_tr,
                ));
            }
            p.push(PathCmd::Line(xr, yb - r_br));
            if r_br > 0.0 {
                p.push(PathCmd::Cubic(
                    xr,
                    yb - r_br * (1.0 - k),
                    xr - r_br * (1.0 - k),
                    yb,
                    xr - r_br,
                    yb,
                ));
            }
            p.push(PathCmd::Line(x + r_bl, yb));
            if r_bl > 0.0 {
                p.push(PathCmd::Cubic(
                    x + r_bl * (1.0 - k),
                    yb,
                    x,
                    yb - r_bl * (1.0 - k),
                    x,
                    yb - r_bl,
                ));
            }
            p.push(PathCmd::Line(x, y + r_tl));
            if r_tl > 0.0 {
                p.push(PathCmd::Cubic(
                    x,
                    y + r_tl * (1.0 - k),
                    x + r_tl * (1.0 - k),
                    y,
                    x + r_tl,
                    y,
                ));
            }
            p.push(PathCmd::Close);
            p
        }
        Shape::Circle { cx, cy, r } => {
            let k = 0.552_284_8 * r;
            vec![
                PathCmd::Move(cx + r, cy),
                PathCmd::Cubic(cx + r, cy + k, cx + k, cy + r, cx, cy + r),
                PathCmd::Cubic(cx - k, cy + r, cx - r, cy + k, cx - r, cy),
                PathCmd::Cubic(cx - r, cy - k, cx - k, cy - r, cx, cy - r),
                PathCmd::Cubic(cx + k, cy - r, cx + r, cy - k, cx + r, cy),
                PathCmd::Close,
            ]
        }
        Shape::Path(ref cmds) => cmds.clone(),
    }
}

/// Expose conversion for writers (and the end-to-end test harness).
#[doc(hidden)]
pub fn to_path(shape: &Shape) -> Vec<PathCmd> {
    shape_subpath(shape)
}
