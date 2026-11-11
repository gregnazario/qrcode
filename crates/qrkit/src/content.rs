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
            Content::Wifi { .. } => Err(Error::Content(
                "wifi formatting arrives in a later task".into(),
            )),
            Content::VCard(_) => Err(Error::Content(
                "vcard formatting arrives in a later task".into(),
            )),
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
            Content::Calendar(_) => Err(Error::Content(
                "calendar formatting arrives in a later task".into(),
            )),
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

/// Escape a value inside a `WIFI:` payload (backslash, comma, semicolon, quote).
// Wired up in Task 3 (wifi payload formatting).
#[allow(dead_code)]
pub(crate) fn escape_wifi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | ',' | ';' | '"' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape a TEXT value in vCard/VEVENT (backslash, semicolon, comma, newline).
// Wired up in Task 3 (vcard/calendar payload formatting).
#[allow(dead_code)]
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
