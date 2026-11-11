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

#[test]
fn pdf_isolates_graphics_state_per_item() {
    // Translucent background over default opaque black modules. Before
    // per-item q/Q isolation the background's `/A0 gs` (ca 0.502) leaked
    // into every later fill, dimming the modules in PDF output.
    let mut scene = simple_scene();
    scene.background = Some(qrkit::scene::Fill::Solid(qrkit::scene::Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 128,
    }));
    let item_count = scene.items.len();
    assert!(item_count > 0, "scene should contain module items");

    let bytes = pdf::write(&scene);
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/ca 0.502")); // ExtGState for the translucent bg
    assert!(raw.matches("/A0 gs").count() == 1); // set for the bg fill only

    // Extract and tokenize the page content stream.
    let doc = lopdf::Document::load_from(std::io::Cursor::new(&bytes)).expect("valid pdf");
    let pages = doc.get_pages();
    let page = doc.get_object(pages[&1]).unwrap().as_dict().unwrap();
    let content_id = page.get(b"Contents").unwrap().as_reference().unwrap();
    let stream = doc.get_object(content_id).unwrap().as_stream().unwrap();
    let content = String::from_utf8(stream.content.clone()).expect("uncompressed stream");
    let tokens: Vec<&str> = content.split_whitespace().collect();

    // Balanced q/Q nesting, and one pair per background + per scene item
    // (the logo `Do`, when present, brings its own pair).
    let q_count = tokens.iter().filter(|t| **t == "q").count();
    let big_q = tokens.iter().filter(|t| **t == "Q").count();
    assert_eq!(q_count, big_q, "unbalanced q/Q in:\n{content}");
    assert!(
        q_count > item_count,
        "expected >= {} q blocks (background + {item_count} items), got {q_count}",
        item_count + 1
    );

    // Every fill operator must execute inside a q…Q block (depth >= 1), so
    // no fill can inherit a leaked `/ca` or pattern colorspace.
    let mut depth: i64 = 0;
    let mut fills_outside = 0usize;
    for tok in tokens {
        match tok {
            "q" => depth += 1,
            "Q" => depth -= 1,
            "f" | "f*" if depth < 1 => fills_outside += 1,
            _ => {}
        }
    }
    assert_eq!(fills_outside, 0, "fill operator outside any q…Q block");
}
