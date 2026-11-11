use qrkit::content::{CalendarEvent, Content, Encryption, VCard};

fn payload_ok(c: &Content) -> String {
    c.payload().expect("payload should validate")
}

fn sample_vcard() -> VCard {
    VCard {
        name: "Ada Lovelace".into(),
        org: Some("Analytical Engines".into()),
        title: Some("Mathematician".into()),
        phones: vec!["+15551234567".into()],
        emails: vec!["ada@example.com".into()],
        url: Some("https://example.com/ada".into()),
        address: Some("12 Dover St;London;;W1;United Kingdom".into()),
        note: Some("First programmer; note, with comma".into()),
    }
}

#[test]
fn wifi_wpa_payload() {
    assert_eq!(
        payload_ok(&Content::Wifi {
            ssid: "Cafe Guest".into(),
            password: Some("latte-2026".into()),
            encryption: Encryption::Wpa,
            hidden: false,
        }),
        "WIFI:T:WPA;S:Cafe Guest;P:latte-2026;;"
    );
}

#[test]
fn wifi_hidden_and_escapes() {
    assert_eq!(
        payload_ok(&Content::Wifi {
            ssid: "My;Net".into(),
            password: Some("pa\\ss\"word,1".into()),
            encryption: Encryption::Wep,
            hidden: true,
        }),
        "WIFI:T:WEP;S:My\\;Net;P:pa\\\\ss\\\"word\\,1;H:true;;"
    );
}

#[test]
fn wifi_escapes_colon() {
    assert_eq!(
        payload_ok(&Content::Wifi {
            ssid: "ho:tel".into(),
            password: Some("pa:ss:word".into()),
            encryption: Encryption::Wpa,
            hidden: false,
        }),
        "WIFI:T:WPA;S:ho\\:tel;P:pa\\:ss\\:word;;"
    );
}

#[test]
fn wifi_nopass_and_validation() {
    assert_eq!(
        payload_ok(&Content::Wifi {
            ssid: "OpenNet".into(),
            password: None,
            encryption: Encryption::Nopass,
            hidden: false,
        }),
        "WIFI:T:nopass;S:OpenNet;;"
    );
    // WPA without a password is invalid
    assert!(Content::Wifi {
        ssid: "x".into(),
        password: None,
        encryption: Encryption::Wpa,
        hidden: false,
    }
    .payload()
    .is_err());
    // empty SSID is invalid
    assert!(Content::Wifi {
        ssid: "".into(),
        password: Some("p".into()),
        encryption: Encryption::Wpa,
        hidden: false,
    }
    .payload()
    .is_err());
}

#[test]
fn vcard_full_payload() {
    insta::assert_snapshot!(payload_ok(&Content::VCard(sample_vcard())), @r###"
    BEGIN:VCARD
    VERSION:3.0
    FN:Ada Lovelace
    N:Lovelace;Ada;;;
    ORG:Analytical Engines
    TITLE:Mathematician
    TEL:+15551234567
    EMAIL:ada@example.com
    URL:https://example.com/ada
    ADR:;;12 Dover St;London;;W1;United Kingdom
    NOTE:First programmer\; note\, with comma
    END:VCARD
    "###);
}

#[test]
fn vcard_minimal_requires_name() {
    assert!(Content::VCard(VCard::default()).payload().is_err());
    assert_eq!(
        payload_ok(&Content::VCard(VCard {
            name: "Ada".into(),
            ..Default::default()
        })),
        "BEGIN:VCARD\nVERSION:3.0\nFN:Ada\nN:;Ada;;;\nEND:VCARD"
    );
}

#[test]
fn calendar_payload_and_validation() {
    let ev = CalendarEvent {
        title: "Launch party".into(),
        start_utc: "20261201T180000Z".into(),
        end_utc: "20261201T210000Z".into(),
        location: Some("Cafe; upstairs".into()),
        description: Some("Bring stickers".into()),
    };
    insta::assert_snapshot!(payload_ok(&Content::Calendar(ev)), @r###"
    BEGIN:VEVENT
    SUMMARY:Launch party
    DTSTART:20261201T180000Z
    DTEND:20261201T210000Z
    LOCATION:Cafe\; upstairs
    DESCRIPTION:Bring stickers
    END:VEVENT
    "###);

    // bad timestamp format
    let bad = CalendarEvent {
        title: "t".into(),
        start_utc: "2026-12-01 18:00".into(),
        end_utc: "20261201T210000Z".into(),
        location: None,
        description: None,
    };
    assert!(Content::Calendar(bad).payload().is_err());
    // end before start
    let inverted = CalendarEvent {
        title: "t".into(),
        start_utc: "20261202T000000Z".into(),
        end_utc: "20261201T000000Z".into(),
        location: None,
        description: None,
    };
    assert!(Content::Calendar(inverted).payload().is_err());
}
