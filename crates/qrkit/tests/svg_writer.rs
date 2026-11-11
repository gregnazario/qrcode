use qrkit::render::{render, Matrix};
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, ModuleShape, StyleConfig, Version};
use qrkit::writers::common::linear_endpoints;
use qrkit::writers::svg;

fn scene_default() -> qrkit::scene::VectorScene {
    let m = Matrix::encode("https://example.com", Ecc::High, Version::Auto).unwrap();
    render(&m, &StyleConfig::new())
}

#[test]
fn linear_endpoints_axis_aligned() {
    let ((x0, y0), (x1, y1)) = linear_endpoints(10.0, 0.0);
    assert!((x0 - 0.0).abs() < 1e-4 && (x1 - 10.0).abs() < 1e-4);
    assert!((y0 - 5.0).abs() < 1e-4 && (y1 - 5.0).abs() < 1e-4);
    let ((x0, _), (x1, _)) = linear_endpoints(10.0, 90.0);
    assert!((x0 - 5.0).abs() < 1e-4 && (x1 - 5.0).abs() < 1e-4); // vertical
}

#[test]
fn svg_header_and_background() {
    let out = svg::write(&scene_default());
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(s.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(s.contains("shape-rendering=\"geometricPrecision\""));
    assert!(s.contains("fill=\"#ffffff\""));
}

#[test]
fn svg_modules_and_evenodd_eyes() {
    let s = String::from_utf8(svg::write(&scene_default())).unwrap();
    assert!(s.contains("<path d=\"M 4.000 4.000")); // first module with margin 4
    assert!(s.contains("fill-rule=\"evenodd\"")); // eye rings
    assert!(s.contains("#000000"));
}

#[test]
fn svg_gradient_defs() {
    let m = Matrix::encode("hi", Ecc::High, Version::Auto).unwrap();
    let scene = render(
        &m,
        &StyleConfig::new().fill(Fill::LinearGradient {
            from: Rgba {
                r: 13,
                g: 71,
                b: 161,
                a: 255,
            },
            to: Rgba::WHITE,
            angle_deg: 45.0,
        }),
    );
    let s = String::from_utf8(svg::write(&scene)).unwrap();
    assert!(s.contains("<linearGradient"), "missing defs: {s}");
    assert!(s.contains("gradientUnits=\"userSpaceOnUse\""));
    assert!(s.contains("fill=\"url(#g0)\""));
}

#[test]
fn svg_logo_data_uri() {
    let m = Matrix::encode("hi", Ecc::High, Version::Auto).unwrap();
    let logo =
        qrkit::logo::LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let scene = render(
        &m,
        &StyleConfig::new()
            .shape(ModuleShape::Square)
            .logo(Some(logo)),
    );
    let s = String::from_utf8(svg::write(&scene)).unwrap();
    assert!(s.contains("<image "), "{s}");
    assert!(s.contains("data:image/png;base64,"));
}
