//! Typed QR content payloads, formatted to standards-compliant strings.

use crate::Error;

/// WiFi network encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    Wpa,
    Wep,
    Nopass,
}

/// A vCard 3.0 contact.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VCard {
    pub name: String,
    pub org: Option<String>,
    pub title: Option<String>,
    pub phones: Vec<String>,
    pub emails: Vec<String>,
    pub url: Option<String>,
    pub address: Option<String>,
    pub note: Option<String>,
}

/// A VEVENT calendar entry. Timestamps are UTC `YYYYMMDDTHHMMSSZ`.
#[derive(Debug, Clone, PartialEq)]
pub struct CalendarEvent {
    pub title: String,
    pub start_utc: String,
    pub end_utc: String,
    pub location: Option<String>,
    pub description: Option<String>,
}

/// Typed QR content.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Text(String),
    Url(String),
    Email {
        to: String,
        subject: Option<String>,
        body: Option<String>,
    },
    Sms {
        number: String,
        message: Option<String>,
    },
    Phone(String),
    Wifi {
        ssid: String,
        password: Option<String>,
        encryption: Encryption,
        hidden: bool,
    },
    VCard(VCard),
    Geo {
        lat: f64,
        lon: f64,
    },
    Calendar(CalendarEvent),
}

impl Content {
    /// Format this content into its QR payload string, validating fields.
    pub fn payload(&self) -> Result<String, Error> {
        match self {
            Content::Text(t) => Ok(t.clone()),
            Content::Url(u) => {
                let (scheme, rest) = u.split_once("://").ok_or_else(|| {
                    Error::Content("url must start with http:// or https://".into())
                })?;
                if scheme != "http" && scheme != "https" {
                    return Err(Error::Content("url scheme must be http or https".into()));
                }
                if rest.is_empty() || rest.starts_with('/') {
                    return Err(Error::Content("url host is empty".into()));
                }
                Ok(u.clone())
            }
            Content::Email { to, subject, body } => {
                validate_email(to)?;
                let mut out = format!("mailto:{to}");
                let mut query: Vec<String> = Vec::new();
                if let Some(s) = subject {
                    query.push(format!("subject={}", percent_encode(s)));
                }
                if let Some(b) = body {
                    query.push(format!("body={}", percent_encode(b)));
                }
                if !query.is_empty() {
                    out.push('?');
                    out.push_str(&query.join("&"));
                }
                Ok(out)
            }
            Content::Sms { number, message } => {
                validate_phone(number)?;
                Ok(match message {
                    Some(m) if !m.is_empty() => format!("SMSTO:{number}:{m}"),
                    _ => format!("SMSTO:{number}"),
                })
            }
            Content::Phone(n) => {
                validate_phone(n)?;
                Ok(format!("tel:{n}"))
            }
            Content::Wifi {
                ssid,
                password,
                encryption,
                hidden,
            } => {
                if ssid.is_empty() {
                    return Err(Error::Content("wifi ssid must not be empty".into()));
                }
                let t = match encryption {
                    Encryption::Wpa => "WPA",
                    Encryption::Wep => "WEP",
                    Encryption::Nopass => "nopass",
                };
                let mut out = format!("WIFI:T:{t};S:{};", escape_wifi(ssid));
                match (encryption, password) {
                    (Encryption::Nopass, None) => {}
                    (Encryption::Nopass, Some(_)) => {
                        return Err(Error::Content(
                            "nopass network cannot have a password".into(),
                        ));
                    }
                    (_, None) => {
                        return Err(Error::Content("WPA/WEP network requires a password".into()))
                    }
                    (_, Some(p)) => out.push_str(&format!("P:{};", escape_wifi(p))),
                }
                if *hidden {
                    out.push_str("H:true;");
                }
                out.push(';');
                Ok(out)
            }
            Content::VCard(v) => {
                if v.name.is_empty() {
                    return Err(Error::Content("vcard name (FN) must not be empty".into()));
                }
                // Derive N from FN: with two or more tokens, the last is the
                // family name and the rest given names; a single token becomes
                // the given name. Pad to the five N components
                // (Family;Given;Middle;Prefix;Suffix).
                let mut tokens: Vec<&str> = v.name.split_whitespace().collect();
                let family = if tokens.len() >= 2 {
                    tokens.pop()
                } else {
                    None
                };
                let mut n = String::new();
                if let Some(family) = family {
                    n.push_str(family);
                }
                for given in &tokens {
                    n.push(';');
                    n.push_str(given);
                }
                while n.matches(';').count() < 4 {
                    n.push(';');
                }
                let mut lines = vec![
                    "BEGIN:VCARD".to_string(),
                    "VERSION:3.0".to_string(),
                    format!("FN:{}", escape_vcard_text(&v.name)),
                    format!("N:{n}"),
                ];
                if let Some(org) = &v.org {
                    lines.push(format!("ORG:{}", escape_vcard_text(org)));
                }
                if let Some(title) = &v.title {
                    lines.push(format!("TITLE:{}", escape_vcard_text(title)));
                }
                for tel in &v.phones {
                    lines.push(format!("TEL:{}", escape_vcard_text(tel)));
                }
                for mail in &v.emails {
                    lines.push(format!("EMAIL:{}", escape_vcard_text(mail)));
                }
                if let Some(url) = &v.url {
                    lines.push(format!("URL:{}", escape_vcard_text(url)));
                }
                if let Some(addr) = &v.address {
                    // ADR is a structured value: the address string is the
                    // pre-joined tail (street;locality;region;postal;country)
                    // and is passed through unescaped.
                    lines.push(format!("ADR:;;{addr}"));
                }
                if let Some(note) = &v.note {
                    lines.push(format!("NOTE:{}", escape_vcard_text(note)));
                }
                lines.push("END:VCARD".to_string());
                Ok(lines.join("\n"))
            }
            Content::Geo { lat, lon } => {
                if !(-90.0..=90.0).contains(lat) || !(-180.0..=180.0).contains(lon) {
                    return Err(Error::Content(
                        "lat must be in [-90,90], lon in [-180,180]".into(),
                    ));
                }
                if !lat.is_finite() || !lon.is_finite() {
                    return Err(Error::Content("coordinates must be finite".into()));
                }
                Ok(format!("geo:{lat},{lon}"))
            }
            Content::Calendar(ev) => {
                if ev.title.is_empty() {
                    return Err(Error::Content("calendar title must not be empty".into()));
                }
                validate_utc_timestamp(&ev.start_utc)?;
                validate_utc_timestamp(&ev.end_utc)?;
                if ev.end_utc < ev.start_utc {
                    return Err(Error::Content(
                        "calendar end must not be before start".into(),
                    ));
                }
                let mut lines = vec![
                    "BEGIN:VEVENT".to_string(),
                    format!("SUMMARY:{}", escape_vcard_text(&ev.title)),
                    format!("DTSTART:{}", ev.start_utc),
                    format!("DTEND:{}", ev.end_utc),
                ];
                if let Some(loc) = &ev.location {
                    lines.push(format!("LOCATION:{}", escape_vcard_text(loc)));
                }
                if let Some(desc) = &ev.description {
                    lines.push(format!("DESCRIPTION:{}", escape_vcard_text(desc)));
                }
                lines.push("END:VEVENT".to_string());
                Ok(lines.join("\n"))
            }
        }
    }
}

fn validate_email(to: &str) -> Result<(), Error> {
    let (local, domain) = to
        .split_once('@')
        .ok_or_else(|| Error::Content("email must contain '@'".into()))?;
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(Error::Content(format!("invalid email address: {to}")));
    }
    Ok(())
}

fn validate_phone(n: &str) -> Result<(), Error> {
    if n.is_empty() {
        return Err(Error::Content("phone number must not be empty".into()));
    }
    Ok(())
}

/// Percent-encode for the query part of a `mailto:` URI.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Escape a value inside a `WIFI:` payload (backslash, comma, semicolon, quote, colon).
pub(crate) fn escape_wifi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ',' | ';' | '"' | ':' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape a TEXT value in vCard/VEVENT (backslash, semicolon, comma, newline).
pub(crate) fn escape_vcard_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ';' | ',' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Validate a `YYYYMMDDTHHMMSSZ` UTC timestamp.
pub fn validate_utc_timestamp(s: &str) -> Result<(), Error> {
    let b = s.as_bytes();
    let ok = b.len() == 16
        && b[..8].iter().all(|c| c.is_ascii_digit())
        && b[8] == b'T'
        && b[9..15].iter().all(|c| c.is_ascii_digit())
        && b[15] == b'Z';
    if ok {
        Ok(())
    } else {
        Err(Error::Content(format!(
            "invalid UTC timestamp '{s}', expected YYYYMMDDTHHMMSSZ"
        )))
    }
}
