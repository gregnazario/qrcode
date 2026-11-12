mod common;

use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::render::Matrix;
use qrkit::scene::Rgba;
use qrkit::style::{Ecc, EyeShape, StyleConfig, Version};
use qrkit::writers::term;

fn plain_bytes(payload: &str) -> Vec<u8> {
    let img = QrBuilder::new(Content::Text(payload.into()))
        .unwrap()
        .build()
        .unwrap();
    term::render(&img.matrix, &StyleConfig::new())
}

/// Minimal parser for OUR term output: each cell is `ESC[38;2;…m
/// ESC[48;2;…m` followed by ▀ (upper half = fg), ▄ (lower half = fg), or a
/// space (both halves = bg). Lines end with reset. Returns one row of
/// (r,g,b) per module row.
fn parse_ansi(bytes: &[u8]) -> Vec<Vec<(u8, u8, u8)>> {
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    let mut grid: Vec<Vec<(u8, u8, u8)>> = Vec::new();
    for line in s.lines() {
        let mut top_row = Vec::new();
        let mut bottom_row = Vec::new();
        let mut fg: Option<(u8, u8, u8)> = None;
        let mut bg: Option<(u8, u8, u8)> = None;
        let rest = line.strip_suffix("\x1b[0m").unwrap_or(line);
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\x1b' {
                let b = bg.expect("bg color set before glyph");
                match c {
                    '▀' => {
                        let f = fg.expect("fg color set before ▀");
                        top_row.push(f);
                        bottom_row.push(b);
                    }
                    '▄' => {
                        let f = fg.expect("fg color set before ▄");
                        top_row.push(b);
                        bottom_row.push(f);
                    }
                    ' ' => {
                        assert!(fg.is_none(), "space cell carries no fg");
                        top_row.push(b);
                        bottom_row.push(b);
                    }
                    other => panic!("unexpected glyph {other:?}"),
                }
                fg = None;
                bg = None;
                continue;
            }
            // consume '[' ... 'm' (drop the leading '[')
            let mut seq = String::new();
            for s2 in chars.by_ref() {
                if s2 == 'm' {
                    break;
                }
                seq.push(s2);
            }
            let seq = seq.trim_start_matches('[');
            let nums: Vec<u32> = seq
                .split(';')
                .map(|p| p.parse::<u32>().unwrap_or(999))
                .collect();
            match nums.first() {
                Some(&38) => {
                    fg = Some((nums[2] as u8, nums[3] as u8, nums[4] as u8));
                }
                Some(&48) => {
                    bg = Some((nums[2] as u8, nums[3] as u8, nums[4] as u8));
                }
                _ => {} // reset or other
            }
        }
        grid.push(top_row);
        grid.push(bottom_row);
    }
    grid
}

#[test]
fn term_output_has_ansi_structure() {
    let bytes = plain_bytes("hello term");
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(s.contains("▀"));
    assert!(
        s.contains("\x1b[38;2;0;0;0m"),
        "truecolor fg for dark modules"
    );
    assert!(s.contains("\x1b[48;2;255;255;255m"), "white quiet zone bg");
    assert!(s.ends_with("\x1b[0m\n"));
    assert!(s.lines().all(|l| l.ends_with("\x1b[0m")));
}

#[test]
fn term_grid_matches_matrix_with_margin() {
    let payload = "grid match";
    let m = Matrix::encode(payload, Ecc::Quartile, Version::Auto).unwrap();
    let bytes = plain_bytes(payload);
    let grid = parse_ansi(&bytes);
    let n = m.size();
    let margin = 2i32;
    let full = (n + 2 * margin) as usize;
    // Half-block cells pair module rows, so an odd grid height rounds up to
    // an even row count (the padded row is background-only).
    let full_even = full + (full % 2);
    assert_eq!(grid.len(), full_even, "grid height (rows doubled per char)");
    assert!(grid.iter().all(|r| r.len() == full), "grid width");
    // luminance: dark module <= 128, light >= 128
    for (y, row) in grid.iter().take(full).enumerate() {
        for (x, &(r, g, b)) in row.iter().take(full).enumerate() {
            let (mx, my) = (x as i32 - margin, y as i32 - margin);
            let in_matrix = (0..n).contains(&mx) && (0..n).contains(&my);
            let dark = in_matrix && m.get(mx, my);
            let lum = (r / 3 + g / 3 + b / 3) as i32;
            assert_eq!(
                lum <= 128,
                dark,
                "mismatch at grid ({x},{y}) matrix ({mx},{my}) color {r},{g},{b}"
            );
        }
    }
}

#[test]
fn term_output_decodes_via_rqrr() {
    // The real gate: parse our own ANSI back into pixels, rasterize to a
    // tiny-skia pixmap, decode with rqrr, and require exact payload match —
    // same standard as the vector writers' e2e suite.
    let payload = "https://example.com/term";
    let bytes = plain_bytes(payload);
    let grid = parse_ansi(&bytes);
    let h = grid.len();
    let w = grid[0].len();
    let scale: u32 = 16;
    let mut pm = tiny_skia::Pixmap::new((w as u32) * scale, (h as u32) * scale).unwrap();
    for (y, row) in grid.iter().enumerate() {
        for (x, &(r, g, b)) in row.iter().enumerate() {
            let color = tiny_skia::Color::from_rgba(
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                1.0,
            )
            .unwrap();
            let mut rect = tiny_skia::PathBuilder::new();
            rect.push_rect(
                tiny_skia::Rect::from_xywh(
                    (x as u32 * scale) as f32,
                    (y as u32 * scale) as f32,
                    scale as f32,
                    scale as f32,
                )
                .unwrap(),
            );
            let path = rect.finish().unwrap();
            let paint = tiny_skia::Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Default::default()
            };
            pm.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }
    let decoded = common::decode(&pm);
    assert_eq!(decoded, payload);
}

#[test]
fn term_gradient_samples_module_positions() {
    use qrkit::scene::Fill;
    // Horizontal gradient (0deg) across a 25-module matrix: leftmost column
    // is pure `from`, rightmost pure `to`, middle blends.
    let grad = Fill::LinearGradient {
        from: Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        to: Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
        angle_deg: 0.0,
    };
    let n = 25;
    // The gradient axis spans the full matrix bbox, so module centers sit
    // slightly inside the stops: leftmost t ≈ 0.02, rightmost t ≈ 0.98.
    let left = term::sample_fill(&grad, 0, 12, n);
    assert!(
        left.r >= 245 && left.b <= 10,
        "left edge ≈ from color: {left:?}"
    );
    let right = term::sample_fill(&grad, n - 1, 12, n);
    assert!(
        right.b >= 245 && right.r <= 10,
        "right edge ≈ to color: {right:?}"
    );
    let mid = term::sample_fill(&grad, 12, 12, n);
    assert!(mid.r > 100 && mid.b > 100, "middle blends: {mid:?}");
    // Radial: center = from, corner ≈ to (distance/radius clamped).
    let radial = Fill::RadialGradient {
        from: Rgba {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        },
        to: Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
    };
    let center = term::sample_fill(&radial, 12, 12, n);
    assert_eq!((center.r, center.g, center.b), (0, 255, 0));
}

#[test]
fn term_eye_color_applies_to_finders() {
    let style = StyleConfig::new()
        .eye_shape(EyeShape::Square)
        .eye_fill(qrkit::scene::Fill::Solid(Rgba {
            r: 200,
            g: 10,
            b: 10,
            a: 255,
        }));
    let img = QrBuilder::new(Content::Text("eye term".into()))
        .unwrap()
        .build()
        .unwrap();
    let bytes = term::render(&img.matrix, &style);
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        s.contains("\x1b[38;2;200;10;10m"),
        "eye ring truecolor present"
    );
}

#[test]
fn builder_to_term_matches_writer() {
    let b = QrBuilder::new(Content::Text("same term".into())).unwrap();
    let style = StyleConfig::new();
    let via_builder = b.to_term().unwrap();
    let img = b.build().unwrap();
    let direct = term::render(&img.matrix, &style);
    assert_eq!(via_builder, direct);
}
