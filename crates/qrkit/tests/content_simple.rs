use qrkit::content::Content;

fn payload_ok(c: &Content) -> String {
    c.payload().expect("payload should validate")
}

#[test]
fn text_passthrough() {
    assert_eq!(
        payload_ok(&Content::Text("hello world".into())),
        "hello world"
    );
}

#[test]
fn url_requires_http_scheme() {
    assert!(Content::Url("example.com".into()).payload().is_err());
    assert!(Content::Url("ftp://example.com".into()).payload().is_err());
    assert_eq!(
        payload_ok(&Content::Url("https://example.com/promo?x=1".into())),
        "https://example.com/promo?x=1"
    );
}

#[test]
fn email_builds_mailto_with_query() {
    assert_eq!(
        payload_ok(&Content::Email {
            to: "ada@example.com".into(),
            subject: Some("Analytical engines".into()),
            body: Some("First line & second line".into()),
        }),
        "mailto:ada@example.com?subject=Analytical%20engines&body=First%20line%20%26%20second%20line"
    );
    assert_eq!(
        payload_ok(&Content::Email {
            to: "a@b.co".into(),
            subject: None,
            body: None
        }),
        "mailto:a@b.co"
    );
}

#[test]
fn email_validates_address() {
    let bad = Content::Email {
        to: "not-an-email".into(),
        subject: None,
        body: None,
    };
    assert!(bad.payload().is_err());
}

#[test]
fn sms_uses_smsto() {
    assert_eq!(
        payload_ok(&Content::Sms {
            number: "+15551234567".into(),
            message: Some("hi".into())
        }),
        "SMSTO:+15551234567:hi"
    );
    assert_eq!(
        payload_ok(&Content::Sms {
            number: "+15551234567".into(),
            message: None
        }),
        "SMSTO:+15551234567"
    );
}

#[test]
fn phone_uses_tel() {
    assert_eq!(
        payload_ok(&Content::Phone("+15551234567".into())),
        "tel:+15551234567"
    );
    assert!(Content::Phone("".into()).payload().is_err());
}

#[test]
fn geo_formats_and_validates_ranges() {
    assert_eq!(
        payload_ok(&Content::Geo {
            lat: 51.5007,
            lon: -0.1246
        }),
        "geo:51.5007,-0.1246"
    );
    assert!(Content::Geo {
        lat: 91.0,
        lon: 0.0
    }
    .payload()
    .is_err());
    assert!(Content::Geo {
        lat: 0.0,
        lon: -181.0
    }
    .payload()
    .is_err());
}
