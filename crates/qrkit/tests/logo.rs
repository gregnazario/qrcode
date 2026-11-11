use qrkit::logo::{decode_rgba, LogoConfig};
use qrkit::scene::ImageFormat;

const LOGO: &[u8] = include_bytes!("fixtures/logo.png");

#[test]
fn from_bytes_sniffs_png_and_defaults() {
    let cfg = LogoConfig::from_bytes(LOGO.to_vec()).expect("png should be accepted");
    assert_eq!(cfg.format, ImageFormat::Png);
    assert_eq!((cfg.size_ratio, cfg.padding), (0.25, 1.0));
    assert_eq!(cfg.pixel_w, 4); // set by from_bytes
    assert_eq!(cfg.pixel_h, 3);
}

#[test]
fn from_bytes_rejects_unknown_format() {
    assert!(LogoConfig::from_bytes(vec![0u8; 16]).is_err());
    assert!(LogoConfig::from_bytes(b"GIF89a......".to_vec()).is_err());
}

#[test]
fn builder_methods_override_defaults() {
    let cfg = LogoConfig::from_bytes(LOGO.to_vec())
        .unwrap()
        .size_ratio(0.3)
        .padding(0.5);
    assert_eq!((cfg.size_ratio, cfg.padding), (0.3, 0.5));
}

#[test]
fn decode_rgba_returns_raw_pixels() {
    let cfg = LogoConfig::from_bytes(LOGO.to_vec()).unwrap();
    let (w, h, rgba) = decode_rgba(cfg.format, &cfg.data).unwrap();
    assert_eq!((w, h), (4, 3));
    assert_eq!(rgba.len(), 4 * 3 * 4);
    assert_eq!(&rgba[0..4], &[255, 0, 0, 255]); // first pixel red
    assert_eq!(&rgba[4..8], &[0, 255, 0, 128]); // second pixel semi-green
}
