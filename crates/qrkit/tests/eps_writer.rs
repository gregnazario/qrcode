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

#[test]
fn eps_gradient_eye_rings_use_eoclip() {
    // Eye rings are even-odd items (outer + hole subpath, same winding).
    // PostScript `clip` uses the nonzero rule, which would close the ring
    // hole and fill the whole 7x7 finder block, so gradient rings must
    // clip with `eoclip` to preserve the 1:1:3:1:1 scan ratio.
    let grad = qrkit::scene::Fill::LinearGradient {
        from: qrkit::scene::Rgba::BLACK,
        to: qrkit::scene::Rgba::WHITE,
        angle_deg: 45.0,
    };
    let img = QrBuilder::new(Content::Text("eoclip test".into()))
        .unwrap()
        .style(StyleConfig::new().fill(grad.clone()).eye_fill(grad))
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    // three finder rings, all even-odd gradient items
    assert_eq!(s.matches(" gsave eoclip").count(), 3);
    // module items are nonzero gradient items and keep plain `clip`
    assert!(s.contains(" gsave clip"));
    // every eoclip pairs with a shading fill inside the saved state
    let lines: Vec<&str> = s.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if *line == " gsave eoclip" {
            assert!(
                lines[i + 1].contains("shfill"),
                "eoclip not paired with shfill"
            );
            assert_eq!(lines.get(i + 2), Some(&"grestore"));
        }
    }
}

#[test]
fn eps_solid_even_odd_items_have_no_clip() {
    // Solid even-odd items (default solid eye rings) paint with path +
    // ` eofill` directly; clip operators only appear for gradient fills.
    let img = QrBuilder::new(Content::Text("eoclip test".into()))
        .unwrap()
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(s.contains(" eofill")); // solid eye rings
    assert!(!s.contains("clip")); // no clip/eoclip without gradients
    assert!(!s.contains("gsave"));
}

#[test]
fn eps_radial_gradient_end_radius_matches_other_formats() {
    // PDF and SVG use r1 = size/2; EPS must match (it used to emit size/4).
    let img = QrBuilder::new(Content::Text("radial eps".into()))
        .unwrap()
        .style(StyleConfig::new().fill(qrkit::scene::Fill::RadialGradient {
            from: qrkit::scene::Rgba::BLACK,
            to: qrkit::scene::Rgba::WHITE,
        }))
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    let line = s
        .lines()
        .find(|l| l.contains("/ShadingType 3"))
        .expect("radial shading present");
    let coords: Vec<f32> = line
        .split("/Coords [")
        .nth(1)
        .and_then(|r| r.split(']').next())
        .expect("coords parsed")
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(coords.len(), 6);
    assert_eq!(coords[2], 0.0); // inner radius
    assert_eq!(coords[5], coords[0]); // r1 = center = size/2
}

#[test]
fn eps_translucent_background_blends_over_white() {
    // A translucent background rect blends against white paper, not against
    // itself (self-blend would return the raw color and over-darken nothing
    // while still emitting alpha-dependent RGB).
    let img = QrBuilder::new(Content::Text("bg eps".into()))
        .unwrap()
        .style(
            StyleConfig::new().background(Some(qrkit::scene::Fill::Solid(qrkit::scene::Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 128,
            }))),
        )
        .build()
        .unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    // background is the first setrgbcolor: 50% black over white -> 0.502
    let first = s
        .lines()
        .find(|l| l.contains("setrgbcolor"))
        .expect("color present");
    assert!(first.starts_with("0.502 0.502 0.502"), "got: {first}");
}
