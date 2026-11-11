use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::style::{Ecc, StyleConfig};
use qrkit::writers::eps;

#[test]
fn eps_header_and_bbox() {
    let img = QrBuilder::new(Content::Text("eps test".into()))
        .unwrap()
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(s.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
    assert!(s.contains("%%BoundingBox: 0 0"));
    assert!(s.contains("%%EndComments"));
    assert!(s.trim_end().ends_with("%%EOF"));
}

#[test]
fn eps_solid_fill_ops() {
    let img = QrBuilder::new(Content::Text("eps test".into()))
        .unwrap()
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(s.contains(" setrgbcolor"));
    assert!(s.contains(" eofill")); // eye rings
    assert!(s.contains(" fill"));
}

#[test]
fn eps_gradient_uses_shfill() {
    let img = QrBuilder::new(Content::Text("eps test".into()))
        .unwrap()
        .style(StyleConfig::new().fill(qrkit::scene::Fill::LinearGradient {
            from: qrkit::scene::Rgba::BLACK,
            to: qrkit::scene::Rgba::WHITE,
            angle_deg: 45.0,
        }))
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(s.contains("/ShadingType 2"));
    assert!(s.contains(" shfill"));
    assert!(s.contains(" gsave"));
}

#[test]
fn eps_logo_embeds_asciihex_image() {
    let logo = LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let img = QrBuilder::new(Content::Text("eps logo".into()))
        .unwrap()
        .style(StyleConfig::new().ecc(Ecc::High).logo(Some(logo)))
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(s.contains("/ImageType 1"));
    assert!(s.contains("/ASCIIHexDecode"));
    assert!(s.contains(">")); // hex terminator
}

#[test]
fn eps_blends_alpha_against_background() {
    // a fill with alpha=128 becomes a blended opaque color (no /ca in EPS)
    let mut style = StyleConfig::new();
    style.fill = qrkit::scene::Fill::Solid(qrkit::scene::Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 128,
    });
    let img = QrBuilder::new(Content::Text("alpha eps".into()))
        .unwrap()
        .style(style)
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(!s.contains("/ca"));
    // 0.5 gray on white blends to 0.502 → "0.502" appears in setrgbcolor lines
    assert!(s.contains("0.502"));
}
