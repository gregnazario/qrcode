use qrkit_wasm::{render_to_bytes, verify_scans};

const CFG: &str = r##"{
  "content": { "type": "wifi", "ssid": "Cafe Guest", "password": "latte-2026", "encryption": "wpa", "hidden": false },
  "style": { "ecc": "high", "margin": 4, "shape": "dots", "dotRadius": 0.5, "eye": "circle",
             "fg": "#0d47a1", "bg": "#ffffff" }
}"##;

#[test]
fn svg_generation_from_json() {
    let (bytes, warnings) = render_to_bytes(CFG, "svg").expect("generates");
    let svg = String::from_utf8(bytes.expect("svg bytes")).unwrap();
    assert!(svg.starts_with("<?xml"));
    assert!(warnings.is_empty());
}

#[test]
fn pdf_and_eps_from_json() {
    for fmt in ["pdf", "eps"] {
        let (bytes, _) = render_to_bytes(CFG, fmt).unwrap();
        let b = bytes.unwrap();
        assert!(b.len() > 100, "{fmt} too small");
    }
}

#[test]
fn logo_base64_round_trip() {
    use base64::Engine as _;
    let png = std::fs::read("../qrkit/tests/fixtures/logo.png").unwrap();
    let cfg = format!(
        r#"{{"content": {{"type": "text", "text": "logo wasm"}},
            "style": {{"ecc": "high", "logo": {{"dataBase64": "{}", "sizeRatio": 0.25}}}}}}"#,
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    let (bytes, _) = render_to_bytes(&cfg, "svg").unwrap();
    assert!(String::from_utf8(bytes.unwrap())
        .unwrap()
        .contains("data:image/png;base64,"));
}

#[test]
fn invalid_config_reports_error() {
    assert!(render_to_bytes(r#"{"content": {"type": "url", "url": "nope"}}"#, "svg").is_err());
    assert!(render_to_bytes(CFG, "gif").is_err());
}

#[test]
fn logo_warning_surfaces() {
    use base64::Engine as _;
    let png = std::fs::read("../qrkit/tests/fixtures/logo.png").unwrap();
    let cfg = format!(
        r#"{{"content": {{"type": "text", "text": "w"}},
            "style": {{"ecc": "medium", "logo": {{"dataBase64": "{}"}}}}}}"#,
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    let (_, warnings) = render_to_bytes(&cfg, "svg").unwrap();
    assert!(warnings.iter().any(|w| w.contains("High")));
}

#[test]
fn decorative_eyes_maps_and_warns() {
    let cfg = r#"{"content": {"type": "text", "text": "deco"},
                  "style": {"eye": "circle", "decorativeEyes": true}}"#;
    let (bytes, warnings) = render_to_bytes(cfg, "svg").expect("generates");
    assert!(bytes.unwrap().len() > 100);
    assert!(warnings.iter().any(|w| w.contains("decorative eyes")));
    // absent flag stays off
    let (_, warnings) = render_to_bytes(CFG, "svg").unwrap();
    assert!(!warnings.iter().any(|w| w.contains("decorative eyes")));
}

#[test]
fn png_generation_from_json() {
    let cfg = r##"{"content": {"type": "text", "text": "png wasm"},
                  "style": {"fg": "#0d47a1", "px": 8}}"##;
    let (bytes, warnings) = render_to_bytes(cfg, "png").expect("generates");
    let bytes = bytes.expect("png bytes");
    assert!(warnings.is_empty());
    assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n", "PNG magic");
    // Dimensions honor px: v1 (21 modules) + margin 4 -> 29 * 8 = 232 px.
    let dec = png::Decoder::new(std::io::Cursor::new(&bytes));
    let mut reader = dec.read_info().unwrap();
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (232, 232));
    // Default px = 24 when omitted.
    let (bytes, _) = render_to_bytes(CFG, "png").unwrap();
    let default_bytes = bytes.unwrap();
    let dec = png::Decoder::new(std::io::Cursor::new(&default_bytes));
    let reader = dec.read_info().unwrap();
    assert_eq!(reader.info().width % 24, 0, "default px divides width");
}

#[test]
fn info_line_reports_version_size_ecc() {
    let info = qrkit_wasm::info_for(r#"{"content": {"type": "text", "text": "hi"}}"#).unwrap();
    assert!(info.contains("×"), "info carries dimensions: {info}");
    // ECC H is forced when a logo is present.
    use base64::Engine as _;
    let png = std::fs::read("../qrkit/tests/fixtures/logo.png").unwrap();
    let cfg = format!(
        r#"{{"content": {{"type": "text", "text": "x"}},
            "style": {{"ecc": "medium", "logo": {{"dataBase64": "{}"}}}}}}"#,
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    let info = qrkit_wasm::info_for(&cfg).unwrap();
    assert!(info.ends_with("ECC H"), "logo forces ECC H: {info}");
}

#[test]
fn verify_scans_true_for_default_and_false_for_destructive_style() {
    assert!(verify_scans(r#"{"content": {"type": "text", "text": "verify me"}}"#).unwrap());
    // Aggressive: tiny dots + decorative circle eyes — the geometry that
    // motivated the scanner-safety work.
    let hostile = r##"{"content": {"type": "text", "text": "verify me"},
        "style": {"shape": "dots", "dotRadius": 0.1, "eye": "circle", "decorativeEyes": true}}"##;
    assert!(
        !verify_scans(hostile).unwrap(),
        "hostile style should fail the live decode"
    );
}
