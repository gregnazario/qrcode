use qrkit::render::{render, Matrix};
use qrkit::scene::{Fill, ImageFormat, Shape};
use qrkit::style::{Ecc, ModuleShape, StyleConfig, Version};

fn matrix() -> Matrix {
    Matrix::encode("https://example.com", Ecc::High, Version::Auto).unwrap()
}

#[test]
fn matrix_encodes_and_reports_size() {
    let m = matrix();
    assert!(m.size() >= 21 && m.size() <= 177);
    assert!(m.get(0, 0)); // finder corner is dark
}

#[test]
fn matrix_rejects_bad_version_and_huge_payload() {
    assert!(Matrix::encode("x", Ecc::Low, Version::Fixed(41)).is_err());
    let big = "x".repeat(3000);
    assert!(Matrix::encode(&big, Ecc::Low, Version::Auto).is_err());
}

#[test]
fn fixed_version_honors_requested_ecc_and_auto_keeps_high() {
    // A pinned version must not silently boost ECC (qrcodegen's boostecl
    // would raise Low to High since "x" fits version 10 even at High).
    let m = Matrix::encode("x", Ecc::Low, Version::Fixed(10)).unwrap();
    assert_eq!(m.ecc(), qrkit::style::Ecc::to_qrcodegen(Ecc::Low));
    // Auto keeps boosting, so an explicit High stays High.
    let m = Matrix::encode("x", Ecc::High, Version::Auto).unwrap();
    assert_eq!(m.ecc(), qrkit::style::Ecc::to_qrcodegen(Ecc::High));
}

#[test]
fn square_scene_has_margin_and_background() {
    let m = matrix();
    let scene = render(&m, &StyleConfig::new());
    let n = m.size();
    assert_eq!(scene.size, (n + 8) as f32); // margin 4 each side
    assert_eq!(
        scene.background,
        Some(Fill::Solid(qrkit::scene::Rgba::WHITE))
    );
    // Every dark module outside the finder regions produces exactly one Rect.
    // The eyes themselves are drawn as ring+dot paths (same dark area, fewer
    // items), so their modules are excluded from the expected count.
    let in_finder =
        |x: i32, y: i32| (x < 7 && y < 7) || (x >= n - 7 && y < 7) || (x < 7 && y >= n - 7);
    let expected = (0..n)
        .flat_map(|y| (0..n).map(move |x| (x, y)))
        .filter(|&(x, y)| m.get(x, y) && !in_finder(x, y))
        .count();
    let rects = scene
        .items
        .iter()
        .filter(|i| matches!(i.shape, Shape::Rect { .. }))
        .count();
    assert_eq!(rects, expected);
}

#[test]
fn dots_shape_produces_circles() {
    let m = matrix();
    let scene = render(
        &m,
        &StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }),
    );
    assert!(scene
        .items
        .iter()
        .any(|i| matches!(i.shape, Shape::Circle { .. })));
    assert!(!scene
        .items
        .iter()
        .any(|i| matches!(i.shape, Shape::Rect { .. })));
}

#[test]
fn logo_clears_center_modules_and_embeds() {
    let m = matrix();
    let logo =
        qrkit::logo::LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let scene = render(&m, &StyleConfig::new().logo(Some(logo)));
    let emb = scene.logo.as_ref().expect("logo embedded");
    assert_eq!(emb.format, ImageFormat::Png);
    // contain-fit: box side = 0.25 * n; 4:3 image -> w=box, h=box*3/4
    let n = m.size() as f32;
    let box_side = 0.25 * n;
    assert!((emb.w - box_side).abs() < 1e-4);
    assert!((emb.h - box_side * 0.75).abs() < 1e-4);
    // no module rect intersects the cleared region; module rects live at scene
    // (x + margin), so converting back yields matrix coordinates, and the
    // cleared box is centered on the matrix center n/2
    let margin = 4.0f32;
    let half = box_side / 2.0 + 1.0; // + padding 1.0
    let c = n / 2.0;
    for item in &scene.items {
        if let Shape::Rect { x, y, .. } = item.shape {
            let (cx, cy) = (x - margin + 0.5, y - margin + 0.5);
            assert!(
                !(cx > c - half && cx < c + half && cy > c - half && cy < c + half),
                "module inside logo area was not cleared"
            );
        }
    }
}

#[test]
fn decorative_eyes_omit_tabs_and_nubs_and_use_bigger_radii() {
    use qrkit::scene::{CornerRadii, Shape};
    use qrkit::style::EyeShape;

    let build = |decorative: bool| render_eye_scene(decorative, EyeShape::Circle);

    // Safe mode: circle eyes carry 12 tab modules per eye (3 eyes = 36),
    // each a 1x1 RoundedRect with uniform radius 0.2.
    let safe = build(false);
    let tabs = safe
        .items
        .iter()
        .filter(|i| {
            matches!(
                i.shape,
                Shape::RoundedRect {
                    w: 1.0,
                    h: 1.0,
                    radii: CornerRadii {
                        tl: 0.2,
                        tr: 0.2,
                        br: 0.2,
                        bl: 0.2
                    },
                    ..
                }
            )
        })
        .count();
    assert_eq!(tabs, 36, "safe circle eyes must paint 3x12 corner tabs");

    // Decorative mode: no tabs at all.
    let deco = build(true);
    let tabs = deco
        .items
        .iter()
        .filter(|i| {
            matches!(
                i.shape,
                Shape::RoundedRect {
                    w: 1.0,
                    h: 1.0,
                    radii: CornerRadii {
                        tl: 0.2,
                        tr: 0.2,
                        br: 0.2,
                        bl: 0.2
                    },
                    ..
                }
            )
        })
        .count();
    assert_eq!(tabs, 0, "decorative circle eyes must not paint tabs");

    // Decorative rounded eyes: outer radius 2.0, no 0.6 nubs. The ring is a
    // Path whose first Move lands at x + outer_radius (margin 4), so that
    // coordinate is the geometry probe.
    let deco_rounded = render_eye_scene(true, EyeShape::Rounded);
    let ring_starts_at = |scene: &qrkit::scene::VectorScene, x: f32| {
        scene.items.iter().any(|i| {
            matches!(
                i.shape,
                Shape::Path(ref cmds) if matches!(
                    cmds.first(),
                    Some(qrkit::scene::PathCmd::Move(mvx, 4.0)) if (*mvx - x).abs() < 1e-4
                )
            )
        })
    };
    assert!(
        ring_starts_at(&deco_rounded, 6.0),
        "decorative rounded ring must start at x+2.0"
    );
    let nubs = deco_rounded
        .items
        .iter()
        .filter(|i| matches!(i.shape, Shape::Rect { w: 0.6, h: 0.6, .. }))
        .count();
    assert_eq!(nubs, 0, "decorative rounded eyes must not paint nubs");

    // Safe rounded eyes do have nubs (4 per eye = 12).
    let safe_rounded = render_eye_scene(false, EyeShape::Rounded);
    let nubs = safe_rounded
        .items
        .iter()
        .filter(|i| matches!(i.shape, Shape::Rect { w: 0.6, h: 0.6, .. }))
        .count();
    assert_eq!(nubs, 12);

    // Decorative circle hole radius: 2.5 (classic thick ring) vs safe 2.9.
    // Circle subpaths start at (cx + r, cy) with cx = cy = margin + 3.5 = 7.5,
    // so the hole subpath (second inside the ring path) contributes a Move at
    // x = 7.5 + r.
    let circle_hole_has_move_at = |scene: &qrkit::scene::VectorScene, x: f32| {
        scene.items.iter().any(|i| {
            matches!(
                i.shape,
                Shape::Path(ref cmds) if cmds.iter().any(|c| matches!(
                    c,
                    qrkit::scene::PathCmd::Move(mvx, mvy)
                        if (*mvx - x).abs() < 1e-4 && (*mvy - 7.5).abs() < 1e-4
                ))
            )
        })
    };
    assert!(
        circle_hole_has_move_at(&render_eye_scene(true, EyeShape::Circle), 10.0),
        "decorative circle hole must use r=2.5 (thick ring)"
    );
    assert!(
        circle_hole_has_move_at(&render_eye_scene(false, EyeShape::Circle), 10.4),
        "safe circle hole must stay r=2.9"
    );

    // Decorative leaf: tl/br radius 3.0 — ring's first Move at x + r_tl
    // (margin 4 → 7.0); safe mode stays at x+1.5.
    assert!(
        ring_starts_at(&render_eye_scene(true, EyeShape::Leaf), 7.0),
        "decorative leaf ring must start at x+3.0"
    );
    assert!(
        ring_starts_at(&render_eye_scene(false, EyeShape::Leaf), 5.5),
        "safe leaf ring must stay at x+1.5"
    );
}

fn render_eye_scene(decorative: bool, eye: qrkit::style::EyeShape) -> qrkit::scene::VectorScene {
    let m = Matrix::encode("https://example.com", Ecc::High, Version::Auto).unwrap();
    render(
        &m,
        &StyleConfig::new()
            .eye_shape(eye)
            .decorative_eyes(decorative),
    )
}
