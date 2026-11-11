use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::style::{Ecc, StyleConfig};
use qrkit::writers::pdf;

fn simple_scene() -> qrkit::scene::VectorScene {
    QrBuilder::new(Content::Text("pdf test".into()))
        .unwrap()
        .build()
        .unwrap()
        .scene
}

#[test]
fn pdf_parses_with_lopdf() {
    let bytes = pdf::write(&simple_scene());
    // lopdf 0.35: `load` is path-only; readers use `load_from`.
    let doc = lopdf::Document::load_from(std::io::Cursor::new(&bytes)).expect("valid pdf");
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 1);
    let page = doc.get_object(pages[&1]).unwrap().as_dict().unwrap();
    // lopdf 0.35: numeric objects are read with `as_float` (f32).
    let mbox: Vec<f32> = page
        .get(b"MediaBox")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_float().unwrap())
        .collect();
    assert_eq!(mbox.len(), 4);
}

#[test]
fn pdf_has_fill_and_evenodd_ops() {
    let bytes = pdf::write(&simple_scene());
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains(" re\n") || raw.contains(" re\r"));
    assert!(raw.contains("f*\n")); // even-odd eye rings
                                   // Brief said `contains(" 0 0 m")` "(format may vary; assert 'm' op
                                   // exists)". No scene path point maps to PDF (0,0) — the 4-module margin
                                   // keeps all geometry away from the origin — and coordinates print as
                                   // "{:.3}" ("0.000"), so the literal substring is unreachable. Assert the
                                   // documented intent instead: 'm' path operators are present.
    assert!(raw.contains(" m\n") || raw.contains(" m\r"));
}

#[test]
fn pdf_with_logo_embeds_xobject() {
    let logo = LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let img = QrBuilder::new(Content::Text("logo pdf".into()))
        .unwrap()
        .style(StyleConfig::new().ecc(Ecc::High).logo(Some(logo)))
        .build()
        .unwrap();
    let bytes = pdf::write(&img.scene);
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/Subtype /Image"));
    assert!(raw.contains("/Width 4"));
    assert!(raw.contains("/Height 3"));
    assert!(raw.contains("/Im0 Do"));
}

#[test]
fn builder_to_pdf_matches_writer() {
    let b = QrBuilder::new(Content::Text("same".into())).unwrap();
    let via_builder = b.to_pdf().unwrap();
    let direct = pdf::write(&b.build().unwrap().scene);
    assert_eq!(via_builder, direct);
    // deterministic output
    assert_eq!(pdf::write(&b.build().unwrap().scene), direct);
}
