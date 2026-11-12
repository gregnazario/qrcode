use qrkit_wasm::render_to_bytes;

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
