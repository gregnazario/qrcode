//! The vector scene-graph: the single geometry source shared by all writers.

use crate::Error;

/// 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Rgba = Rgba {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const TRANSPARENT: Rgba = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    /// Parse `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` (leading `#` optional).
    pub fn from_hex(s: &str) -> Result<Rgba, Error> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let err = || Error::InvalidStyle(format!("invalid hex color '{s}'"));
        // Reject non-ASCII early: byte-length dispatch would otherwise slice
        // multibyte UTF-8 characters mid-character and panic.
        if !s.is_ascii() {
            return Err(err());
        }
        let (r, g, b, a) = match s.len() {
            3 => {
                let h = |i: usize| -> Result<u8, Error> {
                    u8::from_str_radix(&s[i..i + 1], 16).map_err(|_| err())
                };
                let h2 = |v: u8| v * 17; // 0xa -> 0xaa
                (h2(h(0)?), h2(h(1)?), h2(h(2)?), 255)
            }
            4 => {
                let h = |i: usize| -> Result<u8, Error> {
                    u8::from_str_radix(&s[i..i + 1], 16).map_err(|_| err())
                };
                let h2 = |v: u8| v * 17;
                (h2(h(0)?), h2(h(1)?), h2(h(2)?), h2(h(3)?))
            }
            6 | 8 => {
                let h = |i: usize| -> Result<u8, Error> {
                    u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| err())
                };
                (h(0)?, h(2)?, h(4)?, if s.len() == 8 { h(6)? } else { 255 })
            }
            _ => return Err(err()),
        };
        Ok(Rgba { r, g, b, a })
    }

    /// Six-digit hex form (alpha omitted), used by writers.
    pub fn to_hex6(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Per-corner rounding radii (module units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii {
    pub tl: f32,
    pub tr: f32,
    pub br: f32,
    pub bl: f32,
}

impl CornerRadii {
    pub const fn all(r: f32) -> Self {
        CornerRadii {
            tl: r,
            tr: r,
            br: r,
            bl: r,
        }
    }
    pub const fn zero() -> Self {
        CornerRadii {
            tl: 0.0,
            tr: 0.0,
            br: 0.0,
            bl: 0.0,
        }
    }
}

/// One subpath command. Cubic Béziers are the universal curve form
/// (circles and rounded corners are approximated by cubics in every writer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCmd {
    Move(f32, f32),
    Line(f32, f32),
    /// (c1x, c1y, c2x, c2y, x, y)
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// A drawable primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    RoundedRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        radii: CornerRadii,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    },
    Path(Vec<PathCmd>),
}

/// How a shape is filled.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Rgba),
    LinearGradient {
        from: Rgba,
        to: Rgba,
        angle_deg: f32,
    },
    RadialGradient {
        from: Rgba,
        to: Rgba,
    },
}

/// Winding rule for paths (rings are drawn as two subpaths + EvenOdd).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// One filled shape in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    pub shape: Shape,
    pub fill: Fill,
    pub fill_rule: FillRule,
}

/// Raster logo format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

/// A raster logo embedded in the scene. `bytes` are the original encoded
/// image; `pixel_w/pixel_h` its intrinsic size; `x/y/w/h` the placement
/// rect in scene units.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedImage {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
    pub pixel_w: u32,
    pub pixel_h: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The complete drawable scene for one QR code.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VectorScene {
    /// Total side length in module units (matrix + 2 × margin).
    pub size: f32,
    pub background: Option<Fill>,
    pub items: Vec<SceneItem>,
    pub logo: Option<EmbeddedImage>,
}
