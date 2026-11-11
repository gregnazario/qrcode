//! End-to-end scannability gate: rasterize rendered scenes and decode them
//! with rqrr, asserting the payload round-trips.

mod common;

use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::style::{Ecc, StyleConfig};

#[test]
fn default_style_round_trips() {
    let builder = QrBuilder::new(Content::Url("https://example.com/promo".into())).unwrap();
    let img = builder.build().unwrap();
    for px in [256u32, 512, 1024] {
        let pm = common::rasterize(&img.scene, px);
        assert_eq!(
            common::decode(&pm),
            "https://example.com/promo",
            "decode failed at {px}px"
        );
    }
}

#[test]
fn every_content_type_decodes() {
    let cases: Vec<(&str, Content)> = vec![
        ("plain text here", Content::Text("plain text here".into())),
        (
            "https://example.com",
            Content::Url("https://example.com".into()),
        ),
        (
            "mailto:a@b.co?subject=Hi",
            Content::Email {
                to: "a@b.co".into(),
                subject: Some("Hi".into()),
                body: None,
            },
        ),
        (
            "SMSTO:+15551234567:yo",
            Content::Sms {
                number: "+15551234567".into(),
                message: Some("yo".into()),
            },
        ),
        ("tel:+15551234567", Content::Phone("+15551234567".into())),
        (
            "WIFI:T:WPA;S:Cafe Guest;P:latte-2026;;",
            Content::Wifi {
                ssid: "Cafe Guest".into(),
                password: Some("latte-2026".into()),
                encryption: qrkit::content::Encryption::Wpa,
                hidden: false,
            },
        ),
        (
            "geo:51.5007,-0.1246",
            Content::Geo {
                lat: 51.5007,
                lon: -0.1246,
            },
        ),
    ];
    for (expected, content) in cases {
        let img = QrBuilder::new(content).unwrap().build().unwrap();
        let pm = common::rasterize(&img.scene, 512);
        assert_eq!(common::decode(&pm), expected);
    }
}

#[test]
fn logo_build_warns_and_decodes() {
    let logo = LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let img = QrBuilder::new(Content::Url("https://example.com".into()))
        .unwrap()
        .style(StyleConfig::new().ecc(Ecc::Medium).logo(Some(logo)))
        .build()
        .unwrap();
    assert!(img.warnings.iter().any(|w| w.contains("High")));
    let pm = common::rasterize(&img.scene, 1024);
    assert_eq!(common::decode(&pm), "https://example.com");
}

#[test]
fn builder_rejects_invalid_content_eagerly() {
    assert!(QrBuilder::new(Content::Url("no-scheme.example".into())).is_err());
}

#[test]
fn to_svg_is_valid_utf8_with_header() {
    let bytes = QrBuilder::new(Content::Text("hello".into()))
        .unwrap()
        .to_svg()
        .unwrap();
    let s = std::str::from_utf8(&bytes).unwrap();
    assert!(s.starts_with("<?xml"));
}
