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
    assert!(raw.contains(" re f\n")); // background rect fill (batched run op)
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
fn pdf_isolates_extended_graphics_state_runs() {
    // Translucent background over default opaque black modules. Before run
    // isolation the background's `/A0 gs` (ca 0.502) leaked into every later
    // fill, dimming the modules in PDF output. Runs that carry extended
    // state (gs alpha, pattern colorspace) are wrapped in q…Q; opaque solid
    // runs need no wrapper because `rg` fully replaces the fill color.
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

    // Balanced q/Q nesting.
    let q_count = tokens.iter().filter(|t| **t == "q").count();
    let big_q = tokens.iter().filter(|t| **t == "Q").count();
    assert_eq!(q_count, big_q, "unbalanced q/Q in:\n{content}");

    // Every extended-state operator (`gs` ExtGState apply, `scn` pattern
    // fill) must execute inside a q…Q block (depth >= 1), so its state can
    // never leak into later painting.
    let mut depth: i64 = 0;
    let mut ext_outside = 0usize;
    for tok in &tokens {
        match *tok {
            "q" => depth += 1,
            "Q" => depth -= 1,
            "gs" | "scn" if depth < 1 => ext_outside += 1,
            _ => {}
        }
    }
    assert_eq!(
        ext_outside, 0,
        "extended-state operator outside any q…Q block"
    );

    // The translucent background run must be closed (Q) before the opaque
    // module fill is set — the exact sequence the original leak broke. (The
    // stream's initial deterministic rg line precedes the bg run, so search
    // for the module fill only after the closing Q.)
    let bg_gs = content.find("/A0 gs").expect("bg gs op");
    let closing_q = content[bg_gs..]
        .find('Q')
        .map(|i| bg_gs + i)
        .expect("Q after bg gs");
    let module_rg = content[closing_q..]
        .find("0.000 0.000 0.000 rg")
        .map(|i| closing_q + i)
        .expect("opaque module fill after bg run");
    assert!(
        closing_q < module_rg,
        "bg run must be closed before opaque fill:\n{content}"
    );

    // Batching sanity: one color-set per run, not per item — the module run
    // sets its rg exactly once for the whole opaque run.
    let rg_count = content.matches(" rg").count();
    assert!(
        rg_count <= 3,
        "expected run-batched color sets, got {rg_count}"
    );
}

#[test]
fn pdf_isolates_gradient_pattern_runs() {
    // Gradient fills paint through `/Pattern cs … scn`, which — like the
    // alpha ExtGState — is state that must never leak into later runs. This
    // is the only place a gradient scene flows through the PDF writer's
    // content stream, so the `scn` half of the isolation invariant is
    // exercised here, not in the alpha test above.
    let img = QrBuilder::new(Content::Text("gradient pdf".into()))
        .unwrap()
        .style(StyleConfig::new().fill(qrkit::scene::Fill::LinearGradient {
            from: qrkit::scene::Rgba::BLACK,
            to: qrkit::scene::Rgba {
                r: 66,
                g: 165,
                b: 245,
                a: 255,
            },
            angle_deg: 45.0,
        }))
        .build()
        .unwrap();
    let bytes = pdf::write(&img.scene);
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/PatternType 2"), "gradient pattern embedded");

    let doc = lopdf::Document::load_from(std::io::Cursor::new(&bytes)).expect("valid pdf");
    let pages = doc.get_pages();
    let page = doc.get_object(pages[&1]).unwrap().as_dict().unwrap();
    let content_id = page.get(b"Contents").unwrap().as_reference().unwrap();
    let stream = doc.get_object(content_id).unwrap().as_stream().unwrap();
    let content = String::from_utf8(stream.content.clone()).expect("uncompressed stream");
    let tokens: Vec<&str> = content.split_whitespace().collect();

    let q_count = tokens.iter().filter(|t| **t == "q").count();
    let big_q = tokens.iter().filter(|t| **t == "Q").count();
    assert_eq!(q_count, big_q, "unbalanced q/Q in:\n{content}");

    let scn_count = tokens.iter().filter(|t| **t == "scn").count();
    assert!(scn_count >= 1, "gradient scene must set a pattern fill");
    let mut depth: i64 = 0;
    let mut scn_outside = 0usize;
    for tok in &tokens {
        match *tok {
            "q" => depth += 1,
            "Q" => depth -= 1,
            "scn" if depth < 1 => scn_outside += 1,
            _ => {}
        }
    }
    assert_eq!(
        scn_outside, 0,
        "pattern scn outside any q…Q block in:\n{content}"
    );

    // One pattern set per gradient RUN, not per item. The default style has
    // gradient module fill with solid black eye rings (and gradient eye
    // dots), so the expected run count comes from the scene itself.
    let expected_runs = {
        let mut runs = 0usize;
        let mut prev: Option<&qrkit::scene::Fill> = None;
        for item in &img.scene.items {
            let is_grad = !matches!(item.fill, qrkit::scene::Fill::Solid(_));
            let prev_grad = prev.is_some_and(|f| !matches!(f, qrkit::scene::Fill::Solid(_)));
            if is_grad && !prev_grad {
                runs += 1;
            }
            prev = Some(&item.fill);
        }
        runs
    };
    assert!(
        expected_runs >= 1,
        "test scene should contain gradient items"
    );
    assert_eq!(
        scn_count, expected_runs,
        "expected one scn per gradient run ({expected_runs}), got {scn_count}"
    );
}
