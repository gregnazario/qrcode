//! ANSI truecolor terminal rendering.
//!
//! A character grid cannot express module *shapes* (dots/rounded/classy),
//! so terminal output always paints square modules — colors, gradients, and
//! eye colors still apply. Each character packs two module rows as a
//! half-block cell: `▀` (dark upper module) or `▄` (dark lower module),
//! with the dark module's color as the foreground and the light background
//! as the cell background; two dark modules share one `▀` (upper in fg,
//! lower in bg). The quiet zone paints the background color so the code
//! scans off dark terminal themes.
//!
//! Terminal divergences from the vector writers, by necessity or choice:
//! translucent fills are pre-blended against the background (terminals
//! have no alpha, same rule as EPS); non-solid backgrounds fall back to
//! white; the gradient axis spans the code area only (not the margin —
//! the tight 2-module term frame would otherwise compress most of the
//! ramp into the quiet zone); and `style.logo` is ignored (a logo hole
//! would waste modules for no visual gain on a character grid).

use crate::render::{in_finder, Matrix};
use crate::scene::{Fill, Rgba};
use crate::style::StyleConfig;
use crate::writers::common::{blend_over, linear_endpoints};
use std::fmt::Write as _;

/// Terminal margin in modules (tighter than the 4-module vector default —
/// character cells are large and a 4-module frame wastes most of a line).
const TERM_MARGIN: i32 = 2;

/// Render a QR matrix as ANSI truecolor text (half-block, 2 rows/char).
pub fn render(matrix: &Matrix, style: &StyleConfig) -> Vec<u8> {
    let n = matrix.size();
    // Terminals have no alpha: translucent backgrounds blend onto white
    // (the terminal's implicit paper), non-solid backgrounds fall back to it.
    let bg = match &style.background {
        Some(Fill::Solid(c)) => blend_over(*c, Rgba::WHITE),
        _ => Rgba::WHITE,
    };
    // Cell color at grid coords (includes the quiet-zone frame): None = the
    // background color (light module or margin). Dark module colors are
    // pre-blended against the background so translucent fills look right.
    let cell = |x: i32, y: i32| -> Option<Rgba> {
        let (mx, my) = (x - TERM_MARGIN, y - TERM_MARGIN);
        if (0..n).contains(&mx) && (0..n).contains(&my) {
            module_color(matrix, style, mx, my, n, bg)
        } else {
            None
        }
    };

    let mut out = String::new();
    let full = n + 2 * TERM_MARGIN;
    let mut y = 0;
    while y < full {
        for x in 0..full {
            let top = cell(x, y);
            let bottom = cell(x, y + 1); // past the grid -> None -> bg
                                         // Half-block cells: ▀ paints the top half in the fg color and
                                         // the bottom half in the bg color; ▄ is the mirror image. A
                                         // space fills both halves with bg. Pick the glyph that puts the
                                         // LIGHT module's color as the cell background:
                                         //   dark/light -> ▀ fg=dark   light/dark -> ▄ fg=dark
                                         //   dark/dark  -> ▀ fg=top    light/light -> space
            match (top, bottom) {
                (Some(t), Some(b)) => {
                    // both dark: top in fg, bottom in bg
                    let _ = write!(
                        out,
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                        t.r, t.g, t.b, b.r, b.g, b.b
                    );
                }
                (Some(t), None) => {
                    // top dark, bottom light
                    let _ = write!(
                        out,
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                        t.r, t.g, t.b, bg.r, bg.g, bg.b
                    );
                }
                (None, Some(b)) => {
                    // top light, bottom dark: lower half block, dark in fg
                    let _ = write!(
                        out,
                        "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▄",
                        b.r, b.g, b.b, bg.r, bg.g, bg.b
                    );
                }
                (None, None) => {
                    let _ = write!(out, "\x1b[48;2;{};{};{}m ", bg.r, bg.g, bg.b);
                }
            }
        }
        out.push_str("\x1b[0m\n");
        y += 2;
    }
    out.into_bytes()
}

/// Color of a dark module at matrix coords: finder regions take the eye
/// fill (their inner 3x3 the eye-inner/module fill), everything else the
/// module fill. Light modules return None. Gradients are sampled at the
/// module's center so a gradient fill keeps its direction in the grid.
fn module_color(
    matrix: &Matrix,
    style: &StyleConfig,
    x: i32,
    y: i32,
    n: i32,
    bg: Rgba,
) -> Option<Rgba> {
    if !matrix.get(x, y) {
        return None;
    }
    let fill = if in_finder(x, y, n) {
        let (ex, ey) = if x < 7 && y < 7 {
            (0, 0)
        } else if x >= n - 7 && y < 7 {
            (n - 7, 0)
        } else {
            (0, n - 7)
        };
        let inner = (2..=4).contains(&(x - ex)) && (2..=4).contains(&(y - ey));
        if inner {
            style.eye_inner_fill.as_ref().unwrap_or(&style.fill)
        } else {
            &style.eye_fill
        }
    } else {
        &style.fill
    };
    Some(blend_over(sample_fill(fill, x, y, n), bg))
}

/// Sample a fill at a module center (matrix coords): solids pass through,
/// gradients interpolate along their axis over the code area.
#[doc(hidden)]
pub fn sample_fill(fill: &Fill, x: i32, y: i32, n: i32) -> Rgba {
    match fill {
        Fill::Solid(c) => *c,
        Fill::LinearGradient {
            from,
            to,
            angle_deg,
        } => {
            let ((x0, y0), (x1, y1)) = linear_endpoints(n as f32, *angle_deg);
            let (dx, dy) = (x1 - x0, y1 - y0);
            let len2 = dx * dx + dy * dy;
            let t = if len2 <= f32::EPSILON {
                0.0
            } else {
                ((x as f32 + 0.5 - x0) * dx + (y as f32 + 0.5 - y0) * dy) / len2
            };
            lerp(*from, *to, t.clamp(0.0, 1.0))
        }
        Fill::RadialGradient { from, to } => {
            let c = n as f32 / 2.0;
            let d = ((x as f32 + 0.5 - c).powi(2) + (y as f32 + 0.5 - c).powi(2)).sqrt();
            lerp(*from, *to, (d / c).clamp(0.0, 1.0))
        }
    }
}

fn lerp(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Rgba {
        r: m(a.r, b.r),
        g: m(a.g, b.g),
        b: m(a.b, b.b),
        a: m(a.a, b.a),
    }
}
