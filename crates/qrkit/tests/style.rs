use qrkit::logo::LogoConfig;
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, ModuleShape, StyleConfig, Version};

const LOGO: &[u8] = include_bytes!("fixtures/logo.png");

#[test]
fn defaults_match_spec() {
    let s = StyleConfig::new();
    assert_eq!(s.ecc, Ecc::Quartile);
    assert_eq!(s.version, Version::Auto);
    assert_eq!(s.margin_modules, 4);
    assert_eq!(s.shape, ModuleShape::Square);
    assert_eq!(s.fill, Fill::Solid(Rgba::BLACK));
    assert_eq!(s.background, Some(Fill::Solid(Rgba::WHITE)));
    assert!(s.logo.is_none());
}

#[test]
fn builder_methods_chain() {
    let s = StyleConfig::new()
        .ecc(Ecc::High)
        .margin_modules(2)
        .shape(ModuleShape::Dots { radius: 0.4 })
        .fill(Fill::LinearGradient {
            from: Rgba::BLACK,
            to: Rgba::WHITE,
            angle_deg: 45.0,
        })
        .background(None);
    assert_eq!(s.ecc, Ecc::High);
    assert_eq!(s.margin_modules, 2);
    assert_eq!(s.background, None);
    assert!(matches!(s.shape, ModuleShape::Dots { radius: 0.4 }));
}

#[test]
fn logo_forces_high_ecc_with_warning() {
    let mut s = StyleConfig::new().ecc(Ecc::Medium);
    s.logo = Some(LogoConfig::from_bytes(LOGO.to_vec()).unwrap());
    assert_eq!(s.effective_ecc(), Ecc::High);
    let warnings = s.validate();
    assert!(warnings.iter().any(|w| w.contains("High")));
    // user already chose High: no warning
    let ok = StyleConfig::new().ecc(Ecc::High);
    assert!(ok.validate().is_empty());
}

#[test]
fn validate_flags_bad_ranges() {
    let s = StyleConfig::new().version(Version::Fixed(41));
    assert!(!s.validate().is_empty());
    let s = StyleConfig::new().shape(ModuleShape::Dots { radius: 0.9 });
    assert!(!s.validate().is_empty());
    let mut s = StyleConfig::new();
    let mut logo = LogoConfig::from_bytes(LOGO.to_vec()).unwrap();
    logo.size_ratio = 0.5;
    s.logo = Some(logo);
    assert!(!s.validate().is_empty());
}

#[test]
fn decorative_eyes_defaults_off_and_warns_when_on() {
    let s = StyleConfig::new();
    assert!(!s.decorative_eyes);
    assert!(s.validate().is_empty());
    let s = StyleConfig::new().decorative_eyes(true);
    assert!(s.decorative_eyes);
    assert!(s.validate().iter().any(|w| w.contains("decorative eyes")));
}

#[test]
fn decorative_eyes_surfaces_via_builder() {
    let img = qrkit::builder::QrBuilder::new(qrkit::content::Content::Text("decorative".into()))
        .unwrap()
        .style(StyleConfig::new().decorative_eyes(true))
        .build()
        .unwrap();
    assert!(img.warnings.iter().any(|w| w.contains("decorative eyes")));
}
