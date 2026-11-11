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
