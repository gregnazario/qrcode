# qrkit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the approved qrkit design — a Rust QR-code generator library (9 content types, full styling, vector scene-graph) with SVG/PDF/EPS writers, a CLI, a WASM binding, and a static web app.

**Architecture:** `qrcodegen` encodes payloads to a module matrix; our renderer composes a `VectorScene` (shapes + fills + optional logo) from the matrix and `StyleConfig`; three pure writers serialize the same scene to SVG, PDF, and EPS. The scene-graph is the single geometry source, so one rasterize-and-decode test suite validates all three outputs' shared geometry.

**Tech Stack:** Rust 2021 edition; `qrcodegen`, `thiserror`, `serde`, `base64`, `png`, `zune-jpeg`, `clap`, `wasm-bindgen`; dev: `insta`, `tiny-skia`, `rqrr`, `lopdf`.

**Spec:** `docs/superpowers/specs/2026-11-10-qr-vector-generator-design.md`

## Global Constraints

- All work on branch `feat/qrkit` — never commit directly to `main`.
- Commit messages: English, conventional style (`feat:`, `test:`, `chore:`), **no AI attribution, no trailers**.
- Runtime deps limited to: `qrcodegen = "1.8"`, `thiserror = "2"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"` (wasm crate only), `base64 = "0.22"`, `png = "0.17"`, `zune-jpeg = "0.4"`, `clap = { version = "4", features = ["derive"] }` (CLI only), `wasm-bindgen = "0.2"` (wasm only). Dev deps: `insta = "1"`, `tiny-skia = "0.11"`, `rqrr = "0.7"`, `lopdf = "0.35"`.
- Scene coordinate space: 1.0 = one QR module, origin **top-left**, `size = (matrix_size + 2 × margin)` in module units. SVG uses these coords directly; PDF/EPS flip Y (`fy = size − y`) at write time.
- ECC default `Quartile`; when a logo is set, ECC is forced to `High` and a warning is recorded if the user chose lower.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` must pass at every commit.

---

### Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/qrkit/Cargo.toml`, `crates/qrkit/src/lib.rs`
- Create: `.gitignore`
- Create: `justfile`

**Interfaces:**
- Produces: cargo workspace with member `crates/qrkit`; `qrkit::Error` enum used by every later task.

- [ ] **Step 1: Create branch and scaffold files**

```bash
git checkout -b feat/qrkit
```

`Cargo.toml` (root):

```toml
[workspace]
resolver = "2"
members = ["crates/qrkit"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
qrcodegen = "1.8"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
base64 = "0.22"
png = "0.17"
zune-jpeg = "0.4"
clap = { version = "4", features = ["derive"] }
wasm-bindgen = "0.2"
insta = "1"
tiny-skia = "0.11"
rqrr = "0.7"
lopdf = "0.35"
```

`crates/qrkit/Cargo.toml`:

```toml
[package]
name = "qrkit"
description = "Customizable QR code generator with vector (SVG/PDF/EPS) output"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
qrcodegen.workspace = true
thiserror.workspace = true
serde.workspace = true
base64.workspace = true
png.workspace = true
zune-jpeg.workspace = true
```

`crates/qrkit/src/lib.rs`:

```rust
//! qrkit: customizable QR codes as vector graphics (SVG, PDF, EPS).

pub mod builder;
pub mod content;
pub mod logo;
pub mod render;
pub mod scene;
pub mod style;
pub mod writers;

/// Errors produced while building or writing a QR code.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("content validation failed: {0}")]
    Content(String),
    #[error("data too large for QR code: {0}")]
    TooLarge(String),
    #[error("encoding failed: {0}")]
    Encoding(String),
    #[error("unsupported logo image format (expected PNG or JPEG)")]
    LogoUnsupportedFormat,
    #[error("logo decode failed: {0}")]
    LogoDecode(String),
    #[error("invalid style: {0}")]
    InvalidStyle(String),
}
```

`.gitignore`:

```
/target
web/pkg
```

`justfile`:

```make
default: --list

build:
    cargo build --workspace --release

test:
    cargo test --workspace

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

check: fmt lint test

build-wasm:
    wasm-pack build crates/qrkit-wasm --target web --out-dir ../../web/pkg

serve:
    python3 -m http.server 8080 --directory web
```

To keep `cargo build` green before later tasks exist, temporarily comment the `pub mod` lines in `lib.rs` that have no files yet (leave only what compiles), or create empty stub modules: content.rs, scene.rs, style.rs, logo.rs, render.rs, writers/mod.rs, builder.rs each containing nothing but a `//! TODO(doc)` placeholder — then later tasks replace them. Create the stubs (empty files are fine; add each `pub mod` line only in its own task). For Task 1 the working `lib.rs` is:

```rust
pub mod content;   // stub now, filled in Task 2
pub mod scene;     // stub
pub mod style;     // stub
pub mod logo;      // stub
pub mod render;    // stub
pub mod writers;   // stub
pub mod builder;   // stub

#[derive(Debug, thiserror::Error)]
pub enum Error { /* as above */ }
```

Each stub file: `// filled in by later tasks`.

`crates/qrkit/src/writers/mod.rs` stub: `// svg, pdf, eps added in later tasks`.

- [ ] **Step 2: Verify build and lint pass**

Run: `cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: success, 0 tests.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock crates .gitignore justfile
git commit -m "chore: scaffold cargo workspace with qrkit crate"
```

---

### Task 2: Content types — simple (Text, Url, Email, Sms, Phone, Geo)

**Files:**
- Modify: `crates/qrkit/src/content.rs` (replace stub)
- Test: `crates/qrkit/tests/content_simple.rs`

**Interfaces:**
- Consumes: `qrkit::Error`
- Produces: `Content` enum, `Content::payload(&self) -> Result<String, Error>`, variants `Text/Url/Email/Sms/Phone/Geo` (Task 3 adds `Wifi/VCard/Calendar`), helper `escape_wifi`/`escape_vcard_text` (used by Task 3), `Content::validate_geo(lat, lon)`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/content_simple.rs`:

```rust
use qrkit::content::Content;

fn payload_ok(c: &Content) -> String {
    c.payload().expect("payload should validate")
}

#[test]
fn text_passthrough() {
    assert_eq!(payload_ok(&Content::Text("hello world".into())), "hello world");
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
        payload_ok(&Content::Email { to: "a@b.co".into(), subject: None, body: None }),
        "mailto:a@b.co"
    );
}

#[test]
fn email_validates_address() {
    let bad = Content::Email { to: "not-an-email".into(), subject: None, body: None };
    assert!(bad.payload().is_err());
}

#[test]
fn sms_uses_smsto() {
    assert_eq!(
        payload_ok(&Content::Sms { number: "+15551234567".into(), message: Some("hi".into()) }),
        "SMSTO:+15551234567:hi"
    );
    assert_eq!(
        payload_ok(&Content::Sms { number: "+15551234567".into(), message: None }),
        "SMSTO:+15551234567"
    );
}

#[test]
fn phone_uses_tel() {
    assert_eq!(payload_ok(&Content::Phone("+15551234567".into())), "tel:+15551234567");
    assert!(Content::Phone("".into()).payload().is_err());
}

#[test]
fn geo_formats_and_validates_ranges() {
    assert_eq!(
        payload_ok(&Content::Geo { lat: 51.5007, lon: -0.1246 }),
        "geo:51.5007,-0.1246"
    );
    assert!(Content::Geo { lat: 91.0, lon: 0.0 }.payload().is_err());
    assert!(Content::Geo { lat: 0.0, lon: -181.0 }.payload().is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test content_simple`
Expected: compile error (no `Content`).

- [ ] **Step 3: Implement content.rs (simple part)**

`crates/qrkit/src/content.rs`:

```rust
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
    Email { to: String, subject: Option<String>, body: Option<String> },
    Sms { number: String, message: Option<String> },
    Phone(String),
    Wifi { ssid: String, password: Option<String>, encryption: Encryption, hidden: bool },
    VCard(VCard),
    Geo { lat: f64, lon: f64 },
    Calendar(CalendarEvent),
}

impl Content {
    /// Format this content into its QR payload string, validating fields.
    pub fn payload(&self) -> Result<String, Error> {
        match self {
            Content::Text(t) => Ok(t.clone()),
            Content::Url(u) => {
                let (scheme, rest) = u
                    .split_once("://")
                    .ok_or_else(|| Error::Content("url must start with http:// or https://".into()))?;
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
            Content::Wifi { .. } => Err(Error::Content("wifi formatting arrives in a later task".into())),
            Content::VCard(_) => Err(Error::Content("vcard formatting arrives in a later task".into())),
            Content::Geo { lat, lon } => {
                if !(-90.0..=90.0).contains(lat) || !(-180.0..=180.0).contains(lon) {
                    return Err(Error::Content("lat must be in [-90,90], lon in [-180,180]".into()));
                }
                if !lat.is_finite() || !lon.is_finite() {
                    return Err(Error::Content("coordinates must be finite".into()));
                }
                Ok(format!("geo:{lat},{lon}"))
            }
            Content::Calendar(_) => Err(Error::Content("calendar formatting arrives in a later task".into())),
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
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Escape a value inside a `WIFI:` payload (backslash, comma, semicolon, quote).
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test content_simple`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/content.rs crates/qrkit/tests/content_simple.rs
git commit -m "feat: add simple QR content types with payload validation"
```

---

### Task 3: Content types — Wifi, VCard, Calendar

**Files:**
- Modify: `crates/qrkit/src/content.rs`
- Test: `crates/qrkit/tests/content_complex.rs`

**Interfaces:**
- Consumes: `Content::payload`, `escape_wifi`, `escape_vcard_text` from Task 2.
- Produces: complete `Content::payload` for all 9 types; `validate_utc_timestamp(&str) -> Result<(), Error>` (also used by CLI validation doc).

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/content_complex.rs`:

```rust
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
        payload_ok(&Content::VCard(VCard { name: "Ada".into(), ..Default::default() })),
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `INSTA_UPDATE=always cargo test -p qrkit --test content_complex`
Expected: FAIL (wifi/vcard/calendar arms return placeholder errors; snapshots initially write new `.snap.new` files — with `INSTA_UPDATE=always` they are accepted automatically, then the assertions for validation failures still fail until implementation).

- [ ] **Step 3: Implement the three payload arms**

In `content.rs`, replace the three placeholder `Err` arms in `Content::payload`:

```rust
Content::Wifi { ssid, password, encryption, hidden } => {
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
            return Err(Error::Content("nopass network cannot have a password".into()));
        }
        (_, None) => return Err(Error::Content("WPA/WEP network requires a password".into())),
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
    // Derive N from FN: last token is the family name, the rest given names.
    let mut n = String::new();
    let mut parts = v.name.split_whitespace().rev();
    if let Some(family) = parts.next() {
        n.push_str(family);
    }
    for given in parts {
        n.push(';');
        n.push_str(given);
    }
    while n.matches(';').count() < 3 {
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
        lines.push(format!("ADR:;;{}", escape_vcard_text(addr)));
    }
    if let Some(note) = &v.note {
        lines.push(format!("NOTE:{}", escape_vcard_text(note)));
    }
    lines.push("END:VCARD".to_string());
    Ok(lines.join("\n"))
}
Content::Calendar(ev) => {
    if ev.title.is_empty() {
        return Err(Error::Content("calendar title must not be empty".into()));
    }
    validate_utc_timestamp(&ev.start_utc)?;
    validate_utc_timestamp(&ev.end_utc)?;
    if ev.end_utc < ev.start_utc {
        return Err(Error::Content("calendar end must not be before start".into()));
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
```

And append to `content.rs`:

```rust
/// Validate a `YYYYMMDDTHHMMSSZ` UTC timestamp.
pub fn validate_utc_timestamp(s: &str) -> Result<(), Error> {
    let b = s.as_bytes();
    let ok = b.len() == 16
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'T'
        && b[9..15].iter().all(u8::is_ascii_digit)
        && b[15] == b'Z';
    if ok { Ok(()) } else { Err(Error::Content(format!("invalid UTC timestamp '{s}', expected YYYYMMDDTHHMMSSZ"))) }
}
```

Note: `u8::is_ascii_digit` takes `&self`, so write `b[..8].iter().all(|c| c.is_ascii_digit())` in the actual implementation.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit`
Expected: PASS (content_simple + content_complex).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/content.rs crates/qrkit/tests/content_complex.rs crates/qrkit/tests/snapshots
git commit -m "feat: add wifi, vcard, and calendar QR content payloads"
```

---

### Task 4: Scene-graph types

**Files:**
- Modify: `crates/qrkit/src/scene.rs` (replace stub)
- Test: `crates/qrkit/tests/scene.rs`

**Interfaces:**
- Produces: `Rgba` (+`BLACK`/`WHITE`, `from_hex`), `CornerRadii` (+`all`, `zero`), `PathCmd`, `Shape`, `Fill`, `FillRule`, `SceneItem`, `ImageFormat`, `EmbeddedImage`, `VectorScene` — used by renderer, all writers, and tests.
- Consumes: `qrkit::Error`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/scene.rs`:

```rust
use qrkit::scene::{CornerRadii, Fill, Rgba};

#[test]
fn hex_parsing_supports_3_4_6_8_digit_forms() {
    assert_eq!(Rgba::from_hex("#abc").unwrap(), Rgba { r: 170, g: 170, b: 170, a: 255 });
    assert_eq!(Rgba::from_hex("#abcd").unwrap(), Rgba { r: 170, g: 170, b: 170, a: 221 });
    assert_eq!(Rgba::from_hex("#0d47a1").unwrap(), Rgba { r: 13, g: 71, b: 161, a: 255 });
    assert_eq!(Rgba::from_hex("#0d47a180").unwrap(), Rgba { r: 13, g: 71, b: 161, a: 128 });
    assert_eq!(Rgba::from_hex("0d47a1").unwrap(), Rgba { r: 13, g: 71, b: 161, a: 255 }); // # optional
    assert!(Rgba::from_hex("#12345").is_err());
    assert!(Rgba::from_hex("#zzzzzz").is_err());
}

#[test]
fn corner_radii_helpers() {
    assert_eq!(CornerRadii::all(0.25), CornerRadii { tl: 0.25, tr: 0.25, br: 0.25, bl: 0.25 });
    assert_eq!(CornerRadii::zero(), CornerRadii { tl: 0.0, tr: 0.0, br: 0.0, bl: 0.0 });
}

#[test]
fn fills_are_comparable_and_debuggable() {
    let a = Fill::Solid(Rgba::BLACK);
    let b = Fill::Solid(Rgba::BLACK);
    assert_eq!(a, b);
    let g = Fill::LinearGradient { from: Rgba::BLACK, to: Rgba::WHITE, angle_deg: 45.0 };
    assert_ne!(a, g);
    assert!(format!("{g:?}").contains("LinearGradient"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test scene`
Expected: compile error.

- [ ] **Step 3: Implement scene.rs**

`crates/qrkit/src/scene.rs`:

```rust
//! The vector scene-graph: the single geometry source shared by all writers.

use crate::Error;

/// 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Rgba = Rgba { r: 0, g: 0, b: 0, a: 0 };

    /// Parse `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` (leading `#` optional).
    pub fn from_hex(s: &str) -> Result<Rgba, Error> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let err = || Error::InvalidStyle(format!("invalid hex color '{s}'"));
        let (r, g, b, a) = match s.len() {
            3 => {
                let h = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).map_err(|_| err())?;
                let h2 = |v: u8| v * 17; // 0xa -> 0xaa
                (h2(h(0)), h2(h(1)), h2(h(2)), 255)
            }
            4 => {
                let h = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).map_err(|_| err())?;
                let h2 = |v: u8| v * 17;
                (h2(h(0)), h2(h(1)), h2(h(2)), h2(h(3)))
            }
            6 | 8 => {
                let h = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| err())?;
                (h(0), h(2), h(4), if s.len() == 8 { h(6) } else { 255 })
            }
            _ => return Err(err()),
        };
        Ok(Rgba { r, g, b, a })
    }

    /// Six-digit hex form (alpha omitted), used by writers.
    pub fn to_hex6(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Per-corner rounding radii (module units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii {
    pub tl: f32,
    pub tr: f32,
    pub br: f32,
    pub bl: f32,
}

impl CornerRadii {
    pub const fn all(r: f32) -> Self {
        CornerRadii { tl: r, tr: r, br: r, bl: r }
    }
    pub const fn zero() -> Self {
        CornerRadii { tl: 0.0, tr: 0.0, br: 0.0, bl: 0.0 }
    }
}

/// One subpath command. Cubic Béziers are the universal curve form
/// (circles and rounded corners are approximated by cubics in every writer).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCmd {
    Move(f32, f32),
    Line(f32, f32),
    /// (c1x, c1y, c2x, c2y, x, y)
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// A drawable primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rect { x: f32, y: f32, w: f32, h: f32 },
    RoundedRect { x: f32, y: f32, w: f32, h: f32, radii: CornerRadii },
    Circle { cx: f32, cy: f32, r: f32 },
    Path(Vec<PathCmd>),
}

/// How a shape is filled.
#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    Solid(Rgba),
    LinearGradient { from: Rgba, to: Rgba, angle_deg: f32 },
    RadialGradient { from: Rgba, to: Rgba },
}

/// Winding rule for paths (rings are drawn as two subpaths + EvenOdd).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// One filled shape in the scene.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    pub shape: Shape,
    pub fill: Fill,
    pub fill_rule: FillRule,
}

/// Raster logo format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
}

/// A raster logo embedded in the scene. `bytes` are the original encoded
/// image; `pixel_w/pixel_h` its intrinsic size; `x/y/w/h` the placement
/// rect in scene units.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedImage {
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
    pub pixel_w: u32,
    pub pixel_h: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The complete drawable scene for one QR code.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VectorScene {
    /// Total side length in module units (matrix + 2 × margin).
    pub size: f32,
    pub background: Option<Fill>,
    pub items: Vec<SceneItem>,
    pub logo: Option<EmbeddedImage>,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test scene`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/scene.rs crates/qrkit/tests/scene.rs
git commit -m "feat: add vector scene-graph types"
```

---

### Task 5: Logo loading (format sniff + decode)

**Files:**
- Modify: `crates/qrkit/src/logo.rs` (replace stub)
- Test: `crates/qrkit/tests/logo.rs`
- Test fixture: `crates/qrkit/tests/fixtures/logo.png` (generated in Step 1)

**Interfaces:**
- Consumes: `scene::{EmbeddedImage, ImageFormat}`, `Error`.
- Produces: `LogoConfig { data: Vec<u8>, format: ImageFormat, size_ratio: f32, padding: f32 }`, `LogoConfig::from_bytes(Vec<u8>) -> Result<LogoConfig, Error>`, builders `.size_ratio(f32)`, `.padding(f32)`, and `logo::decode_rgba(format, &bytes) -> Result<(u32, u32, Vec<u8>), Error>` (rgba8, row-major) used by the PDF/EPS writers and tests.

- [ ] **Step 1: Generate fixture and write failing tests**

Generate a 4×3 red PNG fixture (opaque, one row pattern) with a one-off Rust script:

```bash
mkdir -p crates/qrkit/tests/fixtures
cat > /tmp/mkfixture.rs <<'EOF'
fn main() {
    let w = 4u32;
    let h = 3u32;
    // RGBA: red, semi-green, blue, opaque white across each row
    let px: Vec<u8> = vec![
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255,
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255,
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let file = std::fs::File::create("crates/qrkit/tests/fixtures/logo.png").unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&px).unwrap();
    eprintln!("fixture written");
}
EOF
mkdir -p /tmp/fixt && cp /tmp/mkfixture.rs /tmp/fixt/ && cd /tmp/fixt && \
  cargo init --name mkfixture -q 2>/dev/null; cargo add png@0.17 -q && cargo run -q --manifest-path /tmp/fixt/Cargo.toml 2>/dev/null || true
```

If the `cargo init` dance in /tmp is awkward, instead create the fixture as a committed test helper: put `mkfixture` logic directly into `crates/qrkit/tests/logo.rs` inside a `#[test] #[ignore]` "write fixture" step is overkill — simplest reliable path: a tiny `examples/make_fixture.rs` in the qrkit crate:

`crates/qrkit/examples/make_fixture.rs`:

```rust
fn main() {
    let px: Vec<u8> = vec![
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255,
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255,
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let file = std::fs::File::create("crates/qrkit/tests/fixtures/logo.png").unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 4, 3);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&px).unwrap();
}
```

Then run once: `mkdir -p crates/qrkit/tests/fixtures && cargo run -p qrkit --example make_fixture`.

`crates/qrkit/tests/logo.rs`:

```rust
use qrkit::logo::{decode_rgba, LogoConfig};
use qrkit::scene::ImageFormat;

const LOGO: &[u8] = include_bytes!("fixtures/logo.png");

#[test]
fn from_bytes_sniffs_png_and_defaults() {
    let cfg = LogoConfig::from_bytes(LOGO.to_vec()).expect("png should be accepted");
    assert_eq!(cfg.format, ImageFormat::Png);
    assert_eq!((cfg.size_ratio, cfg.padding), (0.25, 1.0));
    assert_eq!(cfg.pixel_w, 4); // set by from_bytes
    assert_eq!(cfg.pixel_h, 3);
}

#[test]
fn from_bytes_rejects_unknown_format() {
    assert!(LogoConfig::from_bytes(vec![0u8; 16]).is_err());
    assert!(LogoConfig::from_bytes(b"GIF89a......".to_vec()).is_err());
}

#[test]
fn builder_methods_override_defaults() {
    let cfg = LogoConfig::from_bytes(LOGO.to_vec()).unwrap().size_ratio(0.3).padding(0.5);
    assert_eq!((cfg.size_ratio, cfg.padding), (0.3, 0.5));
}

#[test]
fn decode_rgba_returns_raw_pixels() {
    let cfg = LogoConfig::from_bytes(LOGO.to_vec()).unwrap();
    let (w, h, rgba) = decode_rgba(cfg.format, &cfg.data).unwrap();
    assert_eq!((w, h), (4, 3));
    assert_eq!(rgba.len(), 4 * 3 * 4);
    assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);       // first pixel red
    assert_eq!(&rgba[4..8], &[0, 255, 0, 128]);       // second pixel semi-green
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test logo`
Expected: compile error.

- [ ] **Step 3: Implement logo.rs**

`crates/qrkit/src/logo.rs`:

```rust
//! Logo sniffing, dimension probing, and decoding to raw RGBA.

use crate::scene::ImageFormat;
use crate::Error;

/// Configuration for an embedded raster logo.
#[derive(Debug, Clone, PartialEq)]
pub struct LogoConfig {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub size_ratio: f32,
    pub padding: f32,
    /// Intrinsic pixel width, probed at construction.
    pub pixel_w: u32,
    /// Intrinsic pixel height, probed at construction.
    pub pixel_h: u32,
}

impl LogoConfig {
    /// Sniff the image format and probe dimensions.
    pub fn from_bytes(data: Vec<u8>) -> Result<LogoConfig, Error> {
        let format = sniff(&data)?;
        let (pixel_w, pixel_h) = probe_dimensions(format, &data)?;
        Ok(LogoConfig {
            data,
            format,
            size_ratio: 0.25,
            padding: 1.0,
            pixel_w,
            pixel_h,
        })
    }

    /// Logo box side as a fraction of the matrix size (0.1..=0.33).
    pub fn size_ratio(mut self, r: f32) -> Self {
        self.size_ratio = r;
        self
    }

    /// Empty ring around the logo box, in module units.
    pub fn padding(mut self, p: f32) -> Self {
        self.padding = p;
        self
    }
}

fn sniff(data: &[u8]) -> Result<ImageFormat, Error> {
    if data.len() >= 8 && data[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        Ok(ImageFormat::Png)
    } else if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        Ok(ImageFormat::Jpeg)
    } else {
        Err(Error::LogoUnsupportedFormat)
    }
}

fn probe_dimensions(format: ImageFormat, data: &[u8]) -> Result<(u32, u32), Error> {
    match format {
        ImageFormat::Png => {
            // IHDR width/height are bytes 16..24 of a valid PNG.
            if data.len() < 24 {
                return Err(Error::LogoDecode("png truncated before IHDR".into()));
            }
            let be = |r: std::ops::Range<usize>| -> u32 {
                u32::from_be_bytes([data[r.start], data[r.start + 1], data[r.start + 2], data[r.start + 3]])
            };
            Ok((be(16..20), be(20..24)))
        }
        ImageFormat::Jpeg => {
            let mut dec = zune_jpeg::JpegDecoder::new(data);
            dec.decode_headers().map_err(|e| Error::LogoDecode(e.to_string()))?;
            let (w, h) = dec
                .get_dimensions()
                .ok_or_else(|| Error::LogoDecode("jpeg dimensions unavailable".into()))?;
            Ok((w as u32, h as u32))
        }
    }
}

/// Fully decode to interleaved RGBA8.
pub fn decode_rgba(format: ImageFormat, data: &[u8]) -> Result<(u32, u32, Vec<u8>), Error> {
    match format {
        ImageFormat::Png => {
            let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
            decoder.set_transformations(png::Transformations::normalize_to_color8());
            let mut reader = decoder.read_info().map_err(|e| Error::LogoDecode(e.to_string()))?;
            let mut buf = vec![0u8; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).map_err(|e| Error::LogoDecode(e.to_string()))?;
            let (w, h) = (info.width, info.height);
            let px = &buf[..info.buffer_size()];
            let rgba = match info.color_type {
                png::ColorType::Rgba => px.to_vec(),
                png::ColorType::Rgb => to_rgba(px, 3),
                png::ColorType::Grayscale => to_rgba(px, 1),
                png::ColorType::GrayscaleAlpha => to_rgba(px, 2),
                _ => return Err(Error::LogoDecode("unsupported png color type".into())),
            };
            Ok((w, h, rgba))
        }
        ImageFormat::Jpeg => {
            let mut dec = zune_jpeg::JpegDecoder::new(data);
            dec.decode_headers().map_err(|e| Error::LogoDecode(e.to_string()))?;
            let (w, h) = dec
                .get_dimensions()
                .ok_or_else(|| Error::LogoDecode("jpeg dimensions unavailable".into()))?;
            let raw = dec.decode().map_err(|e| Error::LogoDecode(e.to_string()))?;
            let rgba = match raw.len() / (w * h).max(1) {
                4 => raw,
                3 => to_rgba(&raw, 3),
                1 => to_rgba(&raw, 1),
                n => return Err(Error::LogoDecode(format!("unexpected jpeg channel count {n}"))),
            };
            Ok((w as u32, h as u32, rgba))
        }
    }
}

/// Expand interleaved samples (1=gray, 2=gray+alpha, 3=rgb) to RGBA.
fn to_rgba(px: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(px.len() / channels * 4);
    for p in px.chunks(channels) {
        let (r, g, b, a) = match channels {
            1 => (p[0], p[0], p[0], 255),
            2 => (p[0], p[0], p[0], p[1]),
            _ => (p[0], p[1], p[2], 255),
        };
        out.extend_from_slice(&[r, g, b, a]);
    }
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test logo`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/logo.rs crates/qrkit/tests/logo.rs crates/qrkit/tests/fixtures/logo.png crates/qrkit/examples/make_fixture.rs
git commit -m "feat: add logo format sniffing and RGBA decoding"
```

---

### Task 6: Style configuration

**Files:**
- Modify: `crates/qrkit/src/style.rs` (replace stub)
- Test: `crates/qrkit/tests/style.rs`

**Interfaces:**
- Consumes: `scene::{Fill, Rgba}`, `logo::LogoConfig`.
- Produces: `Ecc`, `Version`, `ModuleShape`, `EyeShape`, `StyleConfig` (+`new()` and consuming builder methods `ecc/version/margin_modules/shape/eye_shape/fill/eye_fill/eye_inner_fill/background/logo`), `StyleConfig::validate() -> Vec<String>` (warnings), `StyleConfig::effective_ecc() -> Ecc`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/style.rs`:

```rust
use qrkit::logo::LogoConfig;
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, ModuleShape, StyleConfig, Version};

const LOGO: &[u8] = include_bytes!("fixtures/logo.png");

#[test]
fn defaults_match_spec() {
    let s = StyleConfig::new();
    assert_eq!(s.ecc, Ecc::Quartile);
    assert_eq!(s.version, Version::Auto);
    assert_eq!(s.margin_modules, 4);
    assert_eq!(s.shape, ModuleShape::Square);
    assert_eq!(s.fill, Fill::Solid(Rgba::BLACK));
    assert_eq!(s.background, Some(Fill::Solid(Rgba::WHITE)));
    assert!(s.logo.is_none());
}

#[test]
fn builder_methods_chain() {
    let s = StyleConfig::new()
        .ecc(Ecc::High)
        .margin_modules(2)
        .shape(ModuleShape::Dots { radius: 0.4 })
        .fill(Fill::LinearGradient { from: Rgba::BLACK, to: Rgba::WHITE, angle_deg: 45.0 })
        .background(None);
    assert_eq!(s.ecc, Ecc::High);
    assert_eq!(s.margin_modules, 2);
    assert_eq!(s.background, None);
    assert!(matches!(s.shape, ModuleShape::Dots { radius: 0.4 }));
}

#[test]
fn logo_forces_high_ecc_with_warning() {
    let mut s = StyleConfig::new().ecc(Ecc::Medium);
    s.logo = Some(LogoConfig::from_bytes(LOGO.to_vec()).unwrap());
    assert_eq!(s.effective_ecc(), Ecc::High);
    let warnings = s.validate();
    assert!(warnings.iter().any(|w| w.contains("High")));
    // user already chose High: no warning
    let ok = StyleConfig::new().ecc(Ecc::High);
    assert!(ok.validate().is_empty());
}

#[test]
fn validate_flags_bad_ranges() {
    let s = StyleConfig::new().version(Version::Fixed(41));
    assert!(!s.validate().is_empty());
    let mut s = StyleConfig::new().shape(ModuleShape::Dots { radius: 0.9 });
    assert!(!s.validate().is_empty());
    let mut s = StyleConfig::new();
    let mut logo = LogoConfig::from_bytes(LOGO.to_vec()).unwrap();
    logo.size_ratio = 0.5;
    s.logo = Some(logo);
    assert!(!s.validate().is_empty());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test style`
Expected: compile error.

- [ ] **Step 3: Implement style.rs**

`crates/qrkit/src/style.rs`:

```rust
//! Style configuration for QR rendering.

use crate::logo::LogoConfig;
use crate::scene::{Fill, Rgba};

/// Reed–Solomon error correction level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ecc {
    Low,
    Medium,
    Quartile,
    High,
}

impl Ecc {
    pub fn to_qrcodegen(self) -> qrcodegen::Ecc {
        match self {
            Ecc::Low => qrcodegen::Ecc::Low,
            Ecc::Medium => qrcodegen::Ecc::Medium,
            Ecc::Quartile => qrcodegen::Ecc::Quartile,
            Ecc::High => qrcodegen::Ecc::High,
        }
    }
}

/// QR version (symbol size). Auto picks the smallest that fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Auto,
    Fixed(u8), // 1..=40
}

/// Shape of the data modules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModuleShape {
    Square,
    /// Uniform rounding, radius 0.25 of a module.
    Rounded,
    /// Circle dots; `radius` in module units, clamped to 0.1..=0.5.
    Dots { radius: f32 },
    /// Two diagonally opposite corners rounded (NE and SW).
    Classy,
}

/// Shape of the three finder eyes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeShape {
    Square,
    Rounded,
    Circle,
    Leaf,
}

/// Everything controlling the visual appearance of a QR code.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleConfig {
    pub ecc: Ecc,
    pub version: Version,
    pub margin_modules: u32,
    pub shape: ModuleShape,
    pub eye_shape: EyeShape,
    pub fill: Fill,
    pub eye_fill: Fill,
    pub eye_inner_fill: Option<Fill>, // None -> follow module fill
    pub background: Option<Fill>,     // None -> transparent
    pub logo: Option<LogoConfig>,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleConfig {
    /// Defaults per spec: ECC Q, auto version, margin 4, black square modules
    /// on white, square eyes, no logo.
    pub fn new() -> StyleConfig {
        StyleConfig {
            ecc: Ecc::Quartile,
            version: Version::Auto,
            margin_modules: 4,
            shape: ModuleShape::Square,
            eye_shape: EyeShape::Square,
            fill: Fill::Solid(Rgba::BLACK),
            eye_fill: Fill::Solid(Rgba::BLACK),
            eye_inner_fill: None,
            background: Some(Fill::Solid(Rgba::WHITE)),
            logo: None,
        }
    }

    pub fn ecc(mut self, ecc: Ecc) -> Self { self.ecc = ecc; self }
    pub fn version(mut self, v: Version) -> Self { self.version = v; self }
    pub fn margin_modules(mut self, m: u32) -> Self { self.margin_modules = m; self }
    pub fn shape(mut self, s: ModuleShape) -> Self { self.shape = s; self }
    pub fn eye_shape(mut self, s: EyeShape) -> Self { self.eye_shape = s; self }
    pub fn fill(mut self, f: Fill) -> Self { self.fill = f; self }
    pub fn eye_fill(mut self, f: Fill) -> Self { self.eye_fill = f; self }
    pub fn eye_inner_fill(mut self, f: Option<Fill>) -> Self { self.eye_inner_fill = f; self }
    pub fn background(mut self, f: Option<Fill>) -> Self { self.background = f; self }
    pub fn logo(mut self, l: Option<LogoConfig>) -> Self { self.logo = l; self }

    /// ECC actually used for encoding: a logo forces High (spec).
    pub fn effective_ecc(&self) -> Ecc {
        if self.logo.is_some() {
            Ecc::High
        } else {
            self.ecc
        }
    }

    /// Non-fatal style warnings (fatal problems are renderer/writer errors).
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if let Version::Fixed(v) = self.version {
            if !(1..=40).contains(&v) {
                warnings.push(format!("version {v} out of range 1..=40"));
            }
        }
        if let ModuleShape::Dots { radius } = self.shape {
            if !(0.1..=0.5).contains(&radius) {
                warnings.push(format!("dot radius {radius} out of range 0.1..=0.5"));
            }
        }
        if let Some(logo) = &self.logo {
            if !(0.1..=0.33).contains(&logo.size_ratio) {
                warnings.push(format!("logo size ratio {} out of range 0.1..=0.33", logo.size_ratio));
            }
            if self.ecc != Ecc::High {
                warnings.push(
                    "logo present: ECC forced to High for scannability (use .ecc(Ecc::High) to silence this)".to_string(),
                );
            }
        }
        warnings
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test style`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/style.rs crates/qrkit/tests/style.rs
git commit -m "feat: add style configuration with validation"
```

---

### Task 7: Matrix wrapper and renderer

**Files:**
- Modify: `crates/qrkit/src/render.rs` (replace stub)
- Test: `crates/qrkit/tests/render.rs`

**Interfaces:**
- Consumes: `scene::*`, `style::*` (incl. `effective_ecc`), `Error`.
- Produces: `render::Matrix` with `Matrix::encode(payload: &str, ecc: Ecc, version: Version) -> Result<Matrix, Error>` (public — CLI preview uses it), `size() -> i32`, `get(x: i32, y: i32) -> bool`; `render::render(&Matrix, &StyleConfig) -> VectorScene`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/render.rs`:

```rust
use qrkit::render::{render, Matrix};
use qrkit::scene::{Fill, ImageFormat, Shape};
use qrkit::style::{Ecc, ModuleShape, StyleConfig, Version};

fn matrix() -> Matrix {
    Matrix::encode("https://example.com", Ecc::High, Version::Auto).unwrap()
}

#[test]
fn matrix_encodes_and_reports_size() {
    let m = matrix();
    assert!(m.size() >= 21 && m.size() <= 177);
    assert!(m.get(0, 0)); // finder corner is dark
}

#[test]
fn matrix_rejects_bad_version_and_huge_payload() {
    assert!(Matrix::encode("x", Ecc::Low, Version::Fixed(41)).is_err());
    let big = "x".repeat(3000);
    assert!(Matrix::encode(&big, Ecc::Low, Version::Auto).is_err());
}

#[test]
fn square_scene_has_margin_and_background() {
    let m = matrix();
    let scene = render(&m, &StyleConfig::new());
    let n = m.size();
    assert_eq!(scene.size, (n + 8) as f32); // margin 4 each side
    assert_eq!(scene.background, Some(Fill::Solid(qrkit::scene::Rgba::WHITE)));
    // dark module count in scene (matrix darks minus none; square eyes drawn
    // as ring+dot = same dark area) equals matrix dark count:
    let expected = (0..n).flat_map(|y| (0..n).map(move |x| (x, y))).filter(|&(x, y)| m.get(x, y)).count();
    // every dark module outside the logo area must produce exactly one Rect
    let rects = scene.items.iter().filter(|i| matches!(i.shape, Shape::Rect { .. })).count();
    assert_eq!(rects, expected);
}

#[test]
fn dots_shape_produces_circles() {
    let m = matrix();
    let scene = render(&m, &StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }));
    assert!(scene.items.iter().any(|i| matches!(i.shape, Shape::Circle { .. })));
    assert!(!scene.items.iter().any(|i| matches!(i.shape, Shape::Rect { .. })));
}

#[test]
fn logo_clears_center_modules_and_embeds() {
    let m = matrix();
    let logo = qrkit::logo::LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let scene = render(&m, &StyleConfig::new().logo(Some(logo)));
    let emb = scene.logo.as_ref().expect("logo embedded");
    assert_eq!(emb.format, ImageFormat::Png);
    // contain-fit: box side = 0.25 * n; 4:3 image -> w=box, h=box*3/4
    let n = m.size() as f32;
    let box_side = 0.25 * n;
    assert!((emb.w - box_side).abs() < 1e-4);
    assert!((emb.h - box_side * 0.75).abs() < 1e-4);
    // no module rect intersects the cleared region
    let margin = 4.0f32;
    let half = box_side / 2.0 + 1.0; // + padding 1.0
    let c = scene.size / 2.0;
    for item in &scene.items {
        if let Shape::Rect { x, y, .. } = item.shape {
            let (cx, cy) = (x - margin + 0.5, y - margin + 0.5);
            assert!(
                !(cx > c - half && cx < c + half && cy > c - half && cy < c + half),
                "module inside logo area was not cleared"
            );
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test render`
Expected: compile error.

- [ ] **Step 3: Implement render.rs**

`crates/qrkit/src/render.rs`:

```rust
//! QR matrix encoding and matrix -> VectorScene composition.

use crate::scene::{
    CornerRadii, EmbeddedImage, Fill, FillRule, ImageFormat, PathCmd, SceneItem, Shape, VectorScene,
};
use crate::style::{Ecc, EyeShape, ModuleShape, StyleConfig, Version};
use crate::Error;

/// Thin wrapper over the qrcodegen matrix.
#[derive(Debug, Clone)]
pub struct Matrix {
    code: qrcodegen::QrCode,
}

impl Matrix {
    pub fn encode(payload: &str, ecc: Ecc, version: Version) -> Result<Matrix, Error> {
        if let Version::Fixed(v) = version {
            if !(1..=40).contains(&v) {
                return Err(Error::InvalidStyle(format!("version {v} out of range 1..=40")));
            }
        }
        let code = match version {
            Version::Auto => qrcodegen::QrCode::encode_text(payload, ecc.to_qrcodegen()),
            Version::Fixed(v) => qrcodegen::QrCode::with_version(
                payload,
                qrcodegen::Version::new(v),
                ecc.to_qrcodegen(),
            ),
        }
        .map_err(|e| match e {
            qrcodegen::QrError::DataTooLong => Error::TooLarge(
                "payload exceeds capacity; lower the ECC level, use auto version, or shorten the content"
                    .into(),
            ),
            other => Error::Encoding(other.to_string()),
        })?;
        Ok(Matrix { code })
    }

    pub fn size(&self) -> i32 {
        self.code.size()
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        self.code.get(x, y)
    }
}

/// Compose the full scene for a matrix under a style.
pub fn render(matrix: &Matrix, style: &StyleConfig) -> VectorScene {
    let n = matrix.size();
    let margin = style.margin_modules as f32;
    let size = n as f32 + 2.0 * margin;
    let mut scene = VectorScene { size, background: style.background.clone(), items: Vec::new(), logo: None };

    // Logo clearing box in module coordinates (centered).
    let logo_clear = style.logo.as_ref().map(|l| {
        let half = l.size_ratio * n as f32 / 2.0 + l.padding;
        (n as f32 / 2.0 - half, n as f32 / 2.0 - half, n as f32 / 2.0 + half, n as f32 / 2.0 + half)
    });

    // Data modules (finder eyes handled separately).
    for y in 0..n {
        for x in 0..n {
            if !matrix.get(x, y) || in_finder(x, y, n) {
                continue;
            }
            if let Some((x0, y0, x1, y1)) = logo_clear {
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                if cx > x0 && cx < x1 && cy > y0 && cy < y1 {
                    continue;
                }
            }
            scene.items.push(module_item(x, y, style));
        }
    }

    // Finder eyes.
    for (ex, ey) in [(0, 0), (n - 7, 0), (0, n - 7)] {
        push_eye(&mut scene.items, ex, ey, style);
    }

    // Logo placement (contain-fit inside a centered square box).
    if let Some(l) = &style.logo {
        let box_side = l.size_ratio * n as f32;
        let scale = box_side / l.pixel_w.max(l.pixel_h) as f32;
        let (dw, dh) = (l.pixel_w as f32 * scale, l.pixel_h as f32 * scale);
        let c = size / 2.0;
        scene.logo = Some(EmbeddedImage {
            bytes: l.data.clone(),
            format: l.format,
            pixel_w: l.pixel_w,
            pixel_h: l.pixel_h,
            x: c - dw / 2.0,
            y: c - dh / 2.0,
            w: dw,
            h: dh,
        });
    }

    scene
}

fn in_finder(x: i32, y: i32, n: i32) -> bool {
    (x < 7 && y < 7) || (x >= n - 7 && y < 7) || (x < 7 && y >= n - 7)
}

fn module_item(x: i32, y: i32, style: &StyleConfig) -> SceneItem {
    let (fx, fy) = (x as f32, y as f32);
    let shape = match style.shape {
        ModuleShape::Square => Shape::Rect { x: fx, y: fy, w: 1.0, h: 1.0 },
        ModuleShape::Rounded => Shape::RoundedRect {
            x: fx, y: fy, w: 1.0, h: 1.0,
            radii: CornerRadii::all(0.25),
        },
        ModuleShape::Dots { radius } => Shape::Circle { cx: fx + 0.5, cy: fy + 0.5, r: radius },
        ModuleShape::Classy => Shape::RoundedRect {
            x: fx, y: fy, w: 1.0, h: 1.0,
            radii: CornerRadii { tl: 0.0, tr: 0.5, br: 0.0, bl: 0.5 },
        },
    };
    SceneItem { shape, fill: style.fill.clone(), fill_rule: FillRule::NonZero }
}

/// One finder eye at module (ex, ey), 7x7 modules: ring + inner 3x3 dot.
fn push_eye(items: &mut Vec<SceneItem>, ex: i32, ey: i32, style: &StyleConfig) {
    let (x, y) = (ex as f32, ey as f32);
    let ring_fill = style.eye_fill.clone();
    let dot_fill = style.eye_inner_fill.clone().unwrap_or_else(|| style.fill.clone());

    let (outer, inner_hole, dot) = match style.eye_shape {
        EyeShape::Square => (
            Shape::Rect { x, y, w: 7.0, h: 7.0 },
            Shape::Rect { x: x + 1.0, y: y + 1.0, w: 5.0, h: 5.0 },
            Shape::Rect { x: x + 2.0, y: y + 2.0, w: 3.0, h: 3.0 },
        ),
        EyeShape::Rounded => (
            Shape::RoundedRect { x, y, w: 7.0, h: 7.0, radii: CornerRadii::all(2.0) },
            Shape::RoundedRect { x: x + 1.0, y: y + 1.0, w: 5.0, h: 5.0, radii: CornerRadii::all(1.25) },
            Shape::RoundedRect { x: x + 2.0, y: y + 2.0, w: 3.0, h: 3.0, radii: CornerRadii::all(0.75) },
        ),
        EyeShape::Circle => (
            Shape::Circle { cx: x + 3.5, cy: y + 3.5, r: 3.5 },
            Shape::Circle { cx: x + 3.5, cy: y + 3.5, r: 2.5 },
            Shape::Circle { cx: x + 3.5, cy: y + 3.5, r: 1.5 },
        ),
        EyeShape::Leaf => (
            Shape::RoundedRect {
                x, y, w: 7.0, h: 7.0,
                radii: CornerRadii { tl: 3.0, tr: 0.0, br: 3.0, bl: 0.0 },
            },
            Shape::RoundedRect {
                x: x + 1.0, y: y + 1.0, w: 5.0, h: 5.0,
                radii: CornerRadii { tl: 2.0, tr: 0.0, br: 2.0, bl: 0.0 },
            },
            Shape::RoundedRect {
                x: x + 2.0, y: y + 2.0, w: 3.0, h: 3.0,
                radii: CornerRadii { tl: 1.5, tr: 0.0, br: 1.5, bl: 0.0 },
            },
        ),
    };

    // Ring = outer path + hole subpath, even-odd.
    items.push(SceneItem {
        shape: Shape::Path([shape_subpath(&outer), shape_subpath(&inner_hole)].concat()),
        fill: ring_fill,
        fill_rule: FillRule::EvenOdd,
    });
    items.push(SceneItem { shape: dot, fill: dot_fill, fill_rule: FillRule::NonZero });
}

/// Convert Rect/RoundedRect/Circle to an equivalent Path subpath (closed).
/// Writers only serialize Paths; this is the single conversion point.
pub(crate) fn shape_subpath(shape: &Shape) -> Vec<PathCmd> {
    match *shape {
        Shape::Rect { x, y, w, h } => vec![
            PathCmd::Move(x, y),
            PathCmd::Line(x + w, y),
            PathCmd::Line(x + w, y + h),
            PathCmd::Line(x, y + h),
            PathCmd::Close,
        ],
        Shape::RoundedRect { x, y, w, h, radii } => {
            let k = 0.552284749831; // circle-approximation constant for quarter arcs
            let (r_tl, r_tr, r_br, r_bl) = (radii.tl, radii.tr, radii.br, radii.bl);
            let (xr, yb) = (x + w, y + h);
            let mut p = Vec::with_capacity(4 + 12);
            p.push(PathCmd::Move(x + r_tl, y));
            p.push(PathCmd::Line(xr - r_tr, y));
            if r_tr > 0.0 {
                p.push(PathCmd::Cubic(xr - r_tr * (1.0 - k), y, xr, y + r_tr * (1.0 - k), xr, y + r_tr));
            }
            p.push(PathCmd::Line(xr, yb - r_br));
            if r_br > 0.0 {
                p.push(PathCmd::Cubic(xr, yb - r_br * (1.0 - k), xr - r_br * (1.0 - k), yb, xr - r_br, yb));
            }
            p.push(PathCmd::Line(x + r_bl, yb));
            if r_bl > 0.0 {
                p.push(PathCmd::Cubic(x + r_bl * (1.0 - k), yb, x, yb - r_bl * (1.0 - k), x, yb - r_bl));
            }
            p.push(PathCmd::Line(x, y + r_tl));
            if r_tl > 0.0 {
                p.push(PathCmd::Cubic(x, y + r_tl * (1.0 - k), x + r_tl * (1.0 - k), y, x + r_tl, y));
            }
            p.push(PathCmd::Close);
            p
        }
        Shape::Circle { cx, cy, r } => {
            let k = 0.552284749831 * r;
            vec![
                PathCmd::Move(cx + r, cy),
                PathCmd::Cubic(cx + r, cy + k, cx + k, cy + r, cx, cy + r),
                PathCmd::Cubic(cx - k, cy + r, cx - r, cy + k, cx - r, cy),
                PathCmd::Cubic(cx - r, cy - k, cx - k, cy - r, cx, cy - r),
                PathCmd::Cubic(cx + k, cy - r, cx + r, cy - k, cx + r, cy),
                PathCmd::Close,
            ]
        }
        Shape::Path(ref cmds) => cmds.clone(),
    }
}

/// Expose conversion for writers.
pub(crate) fn to_path(shape: &Shape) -> Vec<PathCmd> {
    shape_subpath(shape)
}

/// Unused today but part of the module's public surface for the CLI preview.
#[allow(dead_code)]
fn _format_marker(_f: ImageFormat) {}
```

Note: writers will call `crate::render::to_path` — keep it `pub(crate)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test render`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/render.rs crates/qrkit/tests/render.rs
git commit -m "feat: add matrix encoding and scene renderer"
```

---

### Task 8: SVG writer

**Files:**
- Modify: `crates/qrkit/src/writers/mod.rs`, Create: `crates/qrkit/src/writers/svg.rs`, Create: `crates/qrkit/src/writers/common.rs`
- Test: `crates/qrkit/tests/svg_writer.rs`

**Interfaces:**
- Consumes: `scene::*`, `render::to_path`.
- Produces: `writers::svg::write(&VectorScene) -> Vec<u8>`; `writers::common::linear_endpoints(size: f32, angle_deg: f32) -> ((f32,f32),(f32,f32))` (unit-tested here; reused by PDF/EPS writers).

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/svg_writer.rs`:

```rust
use qrkit::render::{render, Matrix};
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, ModuleShape, StyleConfig, Version};
use qrkit::writers::common::linear_endpoints;
use qrkit::writers::svg;

fn scene_default() -> qrkit::scene::VectorScene {
    let m = Matrix::encode("https://example.com", Ecc::High, Version::Auto).unwrap();
    render(&m, &StyleConfig::new())
}

#[test]
fn linear_endpoints_axis_aligned() {
    let ((x0, y0), (x1, y1)) = linear_endpoints(10.0, 0.0);
    assert!((x0 - 0.0).abs() < 1e-4 && (x1 - 10.0).abs() < 1e-4);
    assert!((y0 - 5.0).abs() < 1e-4 && (y1 - 5.0).abs() < 1e-4);
    let ((x0, _), (x1, _)) = linear_endpoints(10.0, 90.0);
    assert!((x0 - 5.0).abs() < 1e-4 && (x1 - 5.0).abs() < 1e-4); // vertical
}

#[test]
fn svg_header_and_background() {
    let out = svg::write(&scene_default());
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(s.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(s.contains("shape-rendering=\"geometricPrecision\""));
    assert!(s.contains("fill=\"#ffffff\""));
}

#[test]
fn svg_modules_and_evenodd_eyes() {
    let s = String::from_utf8(svg::write(&scene_default())).unwrap();
    assert!(s.contains("<path d=\"M 4.000 4.000")); // first module with margin 4
    assert!(s.contains("fill-rule=\"evenodd\""));   // eye rings
    assert!(s.contains("#000000"));
}

#[test]
fn svg_gradient_defs() {
    let m = Matrix::encode("hi", Ecc::High, Version::Auto).unwrap();
    let scene = render(
        &m,
        &StyleConfig::new().fill(Fill::LinearGradient {
            from: Rgba { r: 13, g: 71, b: 161, a: 255 },
            to: Rgba::WHITE,
            angle_deg: 45.0,
        }),
    );
    let s = String::from_utf8(svg::write(&scene)).unwrap();
    assert!(s.contains("<linearGradient"), "missing defs: {s}");
    assert!(s.contains("gradientUnits=\"userSpaceOnUse\""));
    assert!(s.contains("fill=\"url(#g0)\""));
}

#[test]
fn svg_logo_data_uri() {
    let m = Matrix::encode("hi", Ecc::High, Version::Auto).unwrap();
    let logo = qrkit::logo::LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).unwrap();
    let scene = render(&m, &StyleConfig::new().shape(ModuleShape::Square).logo(Some(logo)));
    let s = String::from_utf8(svg::write(&scene)).unwrap();
    assert!(s.contains("<image "), "{s}");
    assert!(s.contains("data:image/png;base64,"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test svg_writer`
Expected: compile error.

- [ ] **Step 3: Implement common.rs and svg.rs**

`crates/qrkit/src/writers/common.rs`:

```rust
//! Geometry helpers shared by all writers.

use crate::scene::Fill;

/// Endpoints of a linear gradient axis through the scene center, spanning
/// the bounding box of a `size x size` square, for `angle_deg`
/// (0 = left-to-right, 90 = top-to-bottom, y grows downward).
pub(crate) fn linear_endpoints(size: f32, angle_deg: f32) -> ((f32, f32), (f32, f32)) {
    let c = size / 2.0;
    let rad = angle_deg.to_radians();
    let (dx, dy) = (rad.cos(), rad.sin());
    let mut tmin = f32::MAX;
    let mut tmax = f32::MIN;
    for (x, y) in [(0.0, 0.0), (size, 0.0), (0.0, size), (size, size)] {
        let t = (x - c) * dx + (y - c) * dy;
        tmin = tmin.min(t);
        tmax = tmax.max(t);
    }
    ((c + dx * tmin, c + dy * tmin), (c + dx * tmax, c + dy * tmax))
}

/// Deduplicate gradient fills; returns a stable id list.
pub(crate) fn collect_gradients<'a>(fills: impl Iterator<Item = &'a Fill>) -> Vec<&'a Fill> {
    let mut out: Vec<&Fill> = Vec::new();
    for f in fills {
        if !matches!(f, Fill::Solid(_)) && !out.iter().any(|g| **g == *f) {
            out.push(f);
        }
    }
    out
}
```

`crates/qrkit/src/writers/svg.rs`:

```rust
//! SVG serialization of a VectorScene.

use super::common::{collect_gradients, linear_endpoints};
use crate::render::to_path;
use crate::scene::{EmbeddedImage, Fill, FillRule, PathCmd, Shape, VectorScene};
use base64::Engine as _;
use std::fmt::Write as _;

pub fn write(scene: &VectorScene) -> Vec<u8> {
    let s = scene.size;
    let mut defs = String::new();
    let mut body = String::new();

    let mut all_fills: Vec<&Fill> = scene.background.iter().collect();
    all_fills.extend(scene.items.iter().map(|i| &i.fill));
    let grads = collect_gradients(all_fills.into_iter());

    if let Some(bg) = &scene.background {
        let _ = writeln!(
            body,
            r#"<path d="M 0 0 H {s} V {s} H 0 Z" {}/>"#,
            fill_attrs(bg, s, &grads, &mut defs)
        );
    }
    for item in &scene.items {
        let d = path_d(&item.shape);
        let _ = writeln!(
            body,
            r#"<path d="{d}"{} {}/>"#,
            if item.fill_rule == FillRule::EvenOdd { r#" fill-rule="evenodd""# } else { "" },
            fill_attrs(&item.fill, s, &grads, &mut defs)
        );
    }
    if let Some(img) = &scene.logo {
        let _ = writeln!(body, r#"<image x="{}" y="{}" width="{}" height="{}" href="{}"/>"#,
            f(img.x), f(img.y), f(img.w), f(img.h), data_uri(img));
    }

    let defs_block = if defs.is_empty() { String::new() } else { format!("<defs>\n{defs}</defs>\n") };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{s}\" height=\"{s}\" viewBox=\"0 0 {s} {s}\" shape-rendering=\"geometricPrecision\">\n{defs_block}{body}</svg>\n",
    )
    .into_bytes()
}

fn fill_attrs(fill: &Fill, size: f32, grads: &[&Fill], defs: &mut String) -> String {
    match fill {
        Fill::Solid(c) => {
            if c.a == 255 {
                format!(r#"fill="{}""#, c.to_hex6())
            } else {
                format!(r#"fill="{}" fill-opacity="{}""#, c.to_hex6(), f(c.a as f32 / 255.0))
            }
        }
        g @ (Fill::LinearGradient { .. } | Fill::RadialGradient { .. }) => {
            let id = grads.iter().position(|x| **x == *g).expect("gradient collected");
            write_gradient_def(id, g, size, defs);
            format!(r#"fill="url(#g{id})""#)
        }
    }
}

fn write_gradient_def(id: usize, fill: &Fill, size: f32, defs: &mut String) {
    let stop = |tag: &str, c: &crate::scene::Rgba| {
        if c.a == 255 {
            format!(r#"<stop offset="{tag}" stop-color="{}"/>"#, c.to_hex6())
        } else {
            format!(
                r#"<stop offset="{tag}" stop-color="{}" stop-opacity="{}"/>"#,
                c.to_hex6(),
                f(c.a as f32 / 255.0)
            )
        }
    };
    match fill {
        Fill::LinearGradient { from, to, angle_deg } => {
            let ((x0, y0), (x1, y1)) = linear_endpoints(size, *angle_deg);
            let _ = writeln!(
                defs,
                r#"<linearGradient id="g{id}" gradientUnits="userSpaceOnUse" x1="{}" y1="{}" x2="{}" y2="{}">{}{}</linearGradient>"#,
                f(x0), f(y0), f(x1), f(y1), stop("0", from), stop("1", to)
            );
        }
        Fill::RadialGradient { from, to } => {
            let c = f(size / 2.0);
            let _ = writeln!(
                defs,
                r#"<radialGradient id="g{id}" gradientUnits="userSpaceOnUse" cx="{c}" cy="{c}" r="{c}">{}{}</radialGradient>"#,
                stop("0", from), stop("1", to)
            );
        }
        Fill::Solid(_) => unreachable!(),
    }
}

fn path_d(shape: &Shape) -> String {
    let mut d = String::new();
    for cmd in to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => { let _ = write!(d, "M {} {} ", f(x), f(y)); }
            PathCmd::Line(x, y) => { let _ = write!(d, "L {} {} ", f(x), f(y)); }
            PathCmd::Cubic(a, b, c, e, g, h) => {
                let _ = write!(d, "C {} {} {} {} {} {} ", f(a), f(b), f(c), f(e), f(g), f(h));
            }
            PathCmd::Close => d.push_str("Z "),
        }
    }
    d.trim_end().to_string()
}

fn data_uri(img: &EmbeddedImage) -> String {
    let mime = match img.format {
        crate::scene::ImageFormat::Png => "image/png",
        crate::scene::ImageFormat::Jpeg => "image/jpeg",
    };
    format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&img.bytes))
}

fn f(v: f32) -> String {
    format!("{v:.3}")
}
```

`crates/qrkit/src/writers/mod.rs`:

```rust
//! Format writers: pure serializers of a VectorScene.

pub mod common;
pub mod svg;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test svg_writer`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/writers crates/qrkit/tests/svg_writer.rs
git commit -m "feat: add SVG writer with gradients and logo embedding"
```

---

### Task 9: Builder API + end-to-end decode harness

**Files:**
- Modify: `crates/qrkit/src/builder.rs` (replace stub)
- Create: `crates/qrkit/tests/common/mod.rs` (rasterizer + decoder helpers)
- Test: `crates/qrkit/tests/e2e_decode.rs`

**Interfaces:**
- Consumes: everything from Tasks 2–8.
- Produces: `QrBuilder::new(Content) -> Result<QrBuilder, Error>`, `.style(StyleConfig) -> QrBuilder`, `.build() -> Result<QrImage, Error>`, `.to_svg()/.to_pdf()/.to_eps() -> Result<Vec<u8>, Error>` (pdf/eps added in Tasks 10–11; until then return `Error::Write` placeholder — no: instead add the methods only when their writers land. Task 9 adds only `to_svg`); `QrImage { scene: VectorScene, warnings: Vec<String> }`; test helpers `common::rasterize(&VectorScene, u32) -> tiny_skia::Pixmap` and `common::decode(&tiny_skia::Pixmap) -> String` reused by Task 12.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/common/mod.rs` (helper module, compiled per test binary):

```rust
//! Test-only rasterizer and decoder for end-to-end scene validation.

use qrkit::scene::{Fill, FillRule, ImageFormat, PathCmd, Shape};
use tiny_skia::*;

/// Rasterize a scene to a Pixmap of `px` pixels square.
pub fn rasterize(scene: &qrkit::scene::VectorScene, px: u32) -> Pixmap {
    let scale = px as f32 / scene.size;
    let mut pm = Pixmap::new(px, px).expect("pixmap");
    let ts = Transform::from_scale(scale, scale);

    if let Some(bg) = &scene.background {
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(0.0, 0.0, scene.size, scene.size).unwrap());
        let path = pb.finish().unwrap();
        pm.fill_path(&path, &paint(bg, scene, &ts), FillRule::Winding, ts, None);
    }
    for item in &scene.items {
        let path = scene_path(&item.shape);
        let rule = match item.fill_rule {
            FillRule::NonZero => FillRule::Winding,
            FillRule::EvenOdd => FillRule::EvenOdd,
        };
        pm.fill_path(&path, &paint(&item.fill, scene, &ts), rule, ts, None);
    }
    if let Some(img) = &scene.logo {
        let (w, h, rgba) = qrkit::logo::decode_rgba(img.format, &img.bytes).unwrap();
        for row in 0..h {
            for col in 0..w {
                let i = ((row * w + col) * 4) as usize;
                let color = Color::from_rgba(
                    rgba[i] as f32 / 255.0, rgba[i + 1] as f32 / 255.0,
                    rgba[i + 2] as f32 / 255.0, rgba[i + 3] as f32 / 255.0,
                ).unwrap();
                let x0 = img.x + img.w * col as f32 / w as f32;
                let y0 = img.y + img.h * row as f32 / h as f32;
                let cw = img.w / w as f32;
                let ch = img.h / h as f32;
                let mut pb = PathBuilder::new();
                pb.push_rect(Rect::from_xywh(x0, y0, cw + 0.01, ch + 0.01).unwrap());
                pm.fill_path(&pb.finish().unwrap(), &Paint { solid_color: color, anti_alias: false, ..Default::default() }, FillRule::Winding, ts, None);
            }
        }
    }
    pm
}

/// Decode the first QR code in a rasterized pixmap; panics on failure.
pub fn decode(pm: &Pixmap) -> String {
    let gray: Vec<u8> = pm.pixels().iter().map(|p| {
        let c = p.demultiply();
        // luminance
        (0.299 * c.red() as f32 * 255.0 + 0.587 * c.green() as f32 * 255.0 + 0.114 * c.blue() as f32 * 255.0) as u8
    }).collect();
    let mut grid = rqrr::VecGrid::new(pm.width() as usize, pm.height() as usize, Some(pm.width() as usize), &gray)
        .expect("grid");
    let results = rqrr::DeQr::detect(&mut grid).expect("decode");
    results.into_iter().next().expect("at least one code").content
}

fn paint(fill: &Fill, scene: &qrkit::scene::VectorScene, _ts: &Transform) -> Paint<'static> {
    let mut p = Paint::default();
    match fill {
        Fill::Solid(c) => {
            p.solid_color = Color::from_rgba(
                c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0,
            ).unwrap();
        }
        Fill::LinearGradient { from, to, angle_deg } => {
            let ((x0, y0), (x1, y1)) = qrkit::writers::common::linear_endpoints(scene.size, *angle_deg);
            let stops = vec![
                GradientStop::new(0.0, sk_color(from)),
                GradientStop::new(1.0, sk_color(to)),
            ];
            p.gradient = GradientPaint::Linear(
                LinearGradient::new((x0, y0), (x1, y1), stops, SpreadMode::Pad, Transform::identity()).unwrap(),
            );
        }
        Fill::RadialGradient { from, to } => {
            let c = scene.size / 2.0;
            let stops = vec![
                GradientStop::new(0.0, sk_color(from)),
                GradientStop::new(1.0, sk_color(to)),
            ];
            p.gradient = GradientPaint::Radial(
                RadialGradient::new((c, c), c, stops, SpreadMode::Pad, Transform::identity()).unwrap(),
            );
        }
    }
    p
}

fn sk_color(c: &qrkit::scene::Rgba) -> Color {
    Color::from_rgba(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0).unwrap()
}

fn scene_path(shape: &Shape) -> Path {
    let mut pb = PathBuilder::new();
    for cmd in qrkit::render::to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => pb.move_to(x, y),
            PathCmd::Line(x, y) => pb.line_to(x, y),
            PathCmd::Cubic(c1x, c1y, c2x, c2y, x, y) => pb.cubic_to(c1x, c1y, c2x, c2y, x, y),
            PathCmd::Close => pb.close(),
        };
    }
    pb.finish().unwrap()
}
```

Note: `writers::common` and `render::to_path` are `pub(crate)`; for test access change them to `pub` in this task (or gate with `#[doc(hidden)] pub`). Do: make `common` module `pub` and `to_path` `pub` with `#[doc(hidden)]`.

`crates/qrkit/tests/e2e_decode.rs`:

```rust
mod common;

use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::style::{Ecc, ModuleShape, StyleConfig};

#[test]
fn default_style_round_trips() {
    let builder = QrBuilder::new(Content::Url("https://example.com/promo".into())).unwrap();
    let img = builder.build().unwrap();
    for px in [256u32, 512, 1024] {
        let pm = common::rasterize(&img.scene, px);
        assert_eq!(common::decode(&pm), "https://example.com/promo", "decode failed at {px}px");
    }
}

#[test]
fn every_content_type_decodes() {
    let cases: Vec<(&str, Content)> = vec![
        ("plain text here", Content::Text("plain text here".into())),
        ("https://example.com", Content::Url("https://example.com".into())),
        ("mailto:a@b.co?subject=Hi", Content::Email { to: "a@b.co".into(), subject: Some("Hi".into()), body: None }),
        ("SMSTO:+15551234567:yo", Content::Sms { number: "+15551234567".into(), message: Some("yo".into()) }),
        ("tel:+15551234567", Content::Phone("+15551234567".into())),
        ("WIFI:T:WPA;S:Cafe Guest;P:latte-2026;;", Content::Wifi {
            ssid: "Cafe Guest".into(), password: Some("latte-2026".into()),
            encryption: qrkit::content::Encryption::Wpa, hidden: false,
        }),
        ("geo:51.5007,-0.1246", Content::Geo { lat: 51.5007, lon: -0.1246 }),
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
    let bytes = QrBuilder::new(Content::Text("hello".into())).unwrap().to_svg().unwrap();
    let s = std::str::from_utf8(&bytes).unwrap();
    assert!(s.starts_with("<?xml"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test e2e_decode`
Expected: compile error (no `builder`).

- [ ] **Step 3: Implement builder.rs and dev-deps**

Add to `crates/qrkit/Cargo.toml`:

```toml
[dev-dependencies]
insta.workspace = true
tiny-skia.workspace = true
rqrr.workspace = true
```

Make `to_path` and `common` test-visible: in `render.rs` change `pub(crate) fn to_path` to `#[doc(hidden)] pub fn to_path`; in `writers/mod.rs` change `pub(crate) mod common` (was `mod`-level helper — set `#[doc(hidden)] pub mod common;`).

`crates/qrkit/src/builder.rs`:

```rust
//! The public builder tying content, style, rendering, and writing together.

use crate::content::Content;
use crate::render::{render, Matrix};
use crate::scene::VectorScene;
use crate::style::StyleConfig;
use crate::writers::svg;
use crate::Error;

/// A built QR code: the scene plus non-fatal warnings.
#[derive(Debug, Clone)]
pub struct QrImage {
    pub scene: VectorScene,
    pub warnings: Vec<String>,
}

/// Fluent builder: content in, vector bytes out.
#[derive(Debug, Clone)]
pub struct QrBuilder {
    content: Content,
    style: StyleConfig,
}

impl QrBuilder {
    /// Start from typed content. Validates the content eagerly.
    pub fn new(content: Content) -> Result<QrBuilder, Error> {
        content.payload()?; // eager validation per spec
        Ok(QrBuilder { content, style: StyleConfig::new() })
    }

    pub fn style(mut self, style: StyleConfig) -> QrBuilder {
        self.style = style;
        self
    }

    /// Encode and render. Warnings come from `StyleConfig::validate`.
    pub fn build(&self) -> Result<QrImage, Error> {
        let payload = self.content.payload()?;
        let warnings = self.style.validate();
        let matrix = Matrix::encode(&payload, self.style.effective_ecc(), self.style.version)?;
        Ok(QrImage { scene: render(&matrix, &self.style), warnings })
    }

    /// Convenience: build and serialize to SVG (warnings are ignored here;
    /// call `build()` first if you need them).
    pub fn to_svg(&self) -> Result<Vec<u8>, Error> {
        Ok(svg::write(&self.build()?.scene))
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test e2e_decode`
Expected: PASS (5 tests). This is the critical scannability gate — if rqrr fails to decode, fix the renderer before proceeding.

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/builder.rs crates/qrkit/src/render.rs crates/qrkit/src/writers/mod.rs crates/qrkit/tests/common crates/qrkit/tests/e2e_decode.rs crates/qrkit/Cargo.toml Cargo.lock
git commit -m "feat: add QrBuilder API with end-to-end decode tests"
```

---

### Task 10: PDF writer

**Files:**
- Create: `crates/qrkit/src/writers/pdf.rs`
- Modify: `crates/qrkit/src/writers/mod.rs`, `crates/qrkit/src/builder.rs` (add `to_pdf`)
- Test: `crates/qrkit/tests/pdf_writer.rs`

**Interfaces:**
- Consumes: `scene::*`, `render::to_path`, `common::{linear_endpoints, collect_gradients}`, `logo::decode_rgba`.
- Produces: `writers::pdf::write(&VectorScene) -> Vec<u8>`; `QrBuilder::to_pdf() -> Result<Vec<u8>, Error>`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/pdf_writer.rs`:

```rust
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
    let doc = lopdf::Document::load(std::io::Cursor::new(&bytes)).expect("valid pdf");
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 1);
    let page = doc.get_object(pages[&1]).unwrap().as_dict().unwrap();
    let mbox: Vec<f64> = page
        .get(b"MediaBox").unwrap().as_array().unwrap()
        .iter().map(|o| o.as_number().unwrap()).collect();
    assert_eq!(mbox.len(), 4);
}

#[test]
fn pdf_has_fill_and_evenodd_ops() {
    let bytes = pdf::write(&simple_scene());
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains(" re\n") || raw.contains(" re\r"));
    assert!(raw.contains("f*\n")); // even-odd eye rings
    assert!(raw.contains(" 0 0 m")); // some path ops (format may vary; assert 'm' op exists)
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test pdf_writer`
Expected: compile error.

- [ ] **Step 3: Implement pdf.rs**

`crates/qrkit/src/writers/pdf.rs`:

```rust
//! Minimal PDF 1.7 writer: single vector page, uncompressed streams.

use super::common::{collect_gradients, linear_endpoints};
use crate::logo::decode_rgba;
use crate::render::to_path;
use crate::scene::{Fill, FillRule, ImageFormat, PathCmd, Rgba, Shape, VectorScene};
use std::fmt::Write as _;

pub fn write(scene: &VectorScene) -> Vec<u8> {
    let s = scene.size;
    let fy = |y: f32| s - y; // scene origin is top-left; PDF is bottom-left

    // ---- collect resources ----
    let mut all_fills: Vec<&Fill> = scene.background.iter().collect();
    all_fills.extend(scene.items.iter().map(|i| &i.fill));
    let grads = collect_gradients(all_fills.into_iter());
    let mut alphas: Vec<f32> = Vec::new(); // solid-fill opacities < 1
    let mut register_alpha = |c: &Rgba, alphas: &mut Vec<f32>| -> Option<String> {
        (c.a != 255).then(|| {
            let a = c.a as f32 / 255.0;
            let idx = alphas.iter().position(|x| (*x - a).abs() < 1e-6).unwrap_or_else(|| {
                alphas.push(a);
                alphas.len() - 1
            });
            format!("/A{idx} gs")
        })
    };

    // ---- content stream ----
    let mut content = String::new();
    let _ = writeln!(content, "0.0 0.0 0.0 rg"); // default
    if let Some(bg) = &scene.background {
        set_fill_color(bg, s, &grads, &mut alphas, &mut register_alpha, &mut content);
        let _ = writeln!(content, "0 0 {s:.3} {s:.3} re f");
    }
    for item in &scene.items {
        set_fill_color(&item.fill, s, &grads, &mut alphas, &mut register_alpha, &mut content);
        emit_path(&item.shape, s, &fy, item.fill_rule, &mut content);
    }
    let mut objects: Vec<Vec<u8>> = Vec::new();
    // object ids are 1-based: 1 catalog, 2 pages, 3 page, 4 contents,
    // then gradients (shading+pattern each = 2 objects), then alphas, then image.
    let mut next_id = 5usize;
    let mut pattern_ids = Vec::new();
    for g in &grads {
        let shading_id = next_id;
        let pattern_id = next_id + 1;
        next_id += 2;
        pattern_ids.push(pattern_id);
        let shade_body = match **g {
            Fill::LinearGradient { from, to, angle_deg } => {
                let ((x0, y0), (x1, y1)) = linear_endpoints(s, angle_deg);
                format!(
                    "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >>",
                    x0, fy(y0), x1, fy(y1), function(from, to)
                )
            }
            Fill::RadialGradient { from, to } => {
                let c = s / 2.0;
                format!(
                    "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} 0 {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >>",
                    c, fy(c), c, fy(c), c / 2.0, function(from, to)
                )
            }
            Fill::Solid(_) => unreachable!(),
        };
        let _ = function(from_of(g), to_of(g)); // ensure helper usage compiles identically
        objects.push(format!("{shading_id} 0 obj\n{shade_body}\nendobj\n").into_bytes());
        objects.push(
            format!("{pattern_id} 0 obj\n<< /PatternType 2 /Shading {shading_id} 0 R >>\nendobj\n").into_bytes(),
        );
    }
    let mut gs_ids = Vec::new();
    for a in &alphas {
        let id = next_id;
        next_id += 1;
        gs_ids.push(id);
        objects.push(format!("{id} 0 obj\n<< /Type /ExtGState /ca {a:.3} >>\nendobj\n").into_bytes());
    }

    // image placement in content
    let mut image_id = 0;
    if let Some(img) = &scene.logo {
        image_id = next_id;
        let _ = writeln!(
            content,
            "q {:.3} 0 0 {:.3} {:.3} {:.3} cm /Im0 Do Q",
            img.w, img.h, img.x, fy(img.y) - img.h
        );
    }

    // page resources
    let mut resources = String::from("<< ");
    if !pattern_ids.is_empty() {
        let _ = write!(resources, "/Pattern << {} >> ", pattern_ids.iter().map(|id| format!("/P{id % 1_000} {id} 0 R")).collect::<Vec<_>>().join(" "));
    }
    if !gs_ids.is_empty() {
        let _ = write!(resources, "/ExtGState << {} >> ", gs_ids.iter().map(|id| format!("/A{} {id} 0 R", id - gs_ids[0])).collect::<Vec<_>>().join(" "));
    }
    if image_id > 0 {
        let _ = write!(resources, "/XObject << /Im0 {image_id} 0 R >> ");
    }
    resources.push_str(">>");

    objects.push(format!("{image_id_xobject(image_id, scene)}").into_bytes());

    // Assemble document: obj 1..4 then objects[].
    let mut out = String::new();
    let _ = writeln!(out, "%PDF-1.7");
    let mut offsets: Vec<usize> = Vec::new();

    let obj1 = "<< /Type /Catalog /Pages 2 0 R >>";
    let obj2 = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>";
    let obj3 = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {s:.3} {s:.3}] /Resources {resources} /Contents 4 0 R >>"
    );
    let obj4 = format!("<< /Length {} >>\nstream\n{}endstream", content.len(), content);

    let mut push = |out: &mut String, offsets: &mut Vec<usize>, id: usize, body: &str| {
        offsets.push(out.len());
        let _ = writeln!(out, "{id} 0 obj\n{body}\nendobj");
    };
    push(&mut out, &mut offsets, 1, obj1);
    push(&mut out, &mut offsets, 2, obj2);
    push(&mut out, &mut offsets, 3, &obj3);
    push(&mut out, &mut offsets, 4, &obj4);
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        // ids were pre-assigned; recompute as 5 + index order matches
        let _ = writeln!(out, "{} 0 obj\n{}\nendobj", 5 + i, String::from_utf8_lossy(body).trim_end());
    }

    // ---- xref & trailer ----
    // NOTE: gradient/gs objects were pushed with their pre-assigned ids in
    // `objects`, but the loop above renumbers 5+i — keep them consistent by
    // assigning ids strictly in push order. See Step 4 fix note.
    let xref_pos = out.len();
    let count = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {count}");
    let _ = writeln!(out, "0000000000 65535 f ");
    for off in &offsets {
        let _ = writeln!(out, "{off:010} 00000 n ");
    }
    let _ = writeln!(out, "trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF");
    out.into_bytes()
}

fn from_of(f: &Fill) -> Rgba {
    match f { Fill::Solid(_) => unreachable!(), Fill::LinearGradient { from, .. } | Fill::RadialGradient { from, .. } => *from }
}
fn to_of(f: &Fill) -> Rgba {
    match f { Fill::Solid(_) => unreachable!(), Fill::LinearGradient { to, .. } | Fill::RadialGradient { to, .. } => *to }
}

fn function(from: Rgba, to: Rgba) -> String {
    format!(
        "<< /FunctionType 2 /Domain [0 1] /C0 [{:.3} {:.3} {:.3}] /C1 [{:.3} {:.3} {:.3}] /N 1 >>",
        from.r as f32 / 255.0, from.g as f32 / 255.0, from.b as f32 / 255.0,
        to.r as f32 / 255.0, to.g as f32 / 255.0, to.b as f32 / 255.0
    )
}

fn set_fill_color(
    fill: &Fill, size: f32, grads: &[&Fill], alphas: &mut Vec<f32>,
    register_alpha: &mut impl FnMut(&Rgba, &mut Vec<f32>) -> Option<String>,
    out: &mut String,
) {
    match fill {
        Fill::Solid(c) => {
            let _ = writeln!(out, "{:.3} {:.3} {:.3} rg", c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
            if let Some(gs) = register_alpha(c, alphas) {
                let _ = writeln!(out, "{gs}");
            }
        }
        g => {
            let idx = grads.iter().position(|x| **x == *g).expect("gradient collected");
            let _ = writeln!(out, "/Pattern cs /P{idx} scn");
        }
    }
    let _ = size;
}

fn emit_path(shape: &Shape, s: f32, fy: &impl Fn(f32) -> f32, rule: FillRule, out: &mut String) {
    for cmd in to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => { let _ = writeln!(out, "{:.3} {:.3} m", x, fy(y)); }
            PathCmd::Line(x, y) => { let _ = writeln!(out, "{:.3} {:.3} l", x, fy(y)); }
            PathCmd::Cubic(a, b, c, d, e, g) => {
                let _ = writeln!(out, "{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c", a, fy(b), c, fy(d), e, fy(g));
            }
            PathCmd::Close => { out.push_str("h\n"); }
        }
    }
    match rule {
        FillRule::NonZero => out.push_str("f\n"),
        FillRule::EvenOdd => out.push_str("f*\n"),
    }
    let _ = s;
}

fn image_id_xobject(id: usize, scene: &VectorScene) -> String {
    if id == 0 {
        return String::new();
    }
    let img = scene.logo.as_ref().unwrap();
    match img.format {
        ImageFormat::Jpeg => format!(
            "{id} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode >>\nstream\n{}\nendstream\nendobj",
            img.pixel_w, img.pixel_h,
            // raw bytes must go through lossless binary; callers patch stream bytes
            String::new()
        ),
        ImageFormat::Png => {
            // placeholder replaced in Step 4 with binary stream assembly
            format!("{id} 0 obj\n<< /Width {} /Height {} >>\nendobj", img.pixel_w, img.pixel_h)
        }
    }
}
```

**Important — Step 3 implementation note for the executor:** the sketch above has two deliberate simplifications that MUST be fixed while implementing, or the PDF will be invalid:

1. **Object ids**: assign ids strictly in write order (1=catalog, 2=pages, 3=page, 4=contents, then 5.. in the order gradient-shading, gradient-pattern, extgstate, image objects are appended). Build a `Vec<(usize /*id*/, Vec<u8> /*body*/)>`, assemble once, record byte offsets while writing. Do NOT renumber in a second loop.
2. **Binary streams**: the whole document must be built as `Vec<u8>`, not `String` — image streams contain raw binary. Build the header/content as strings, then append binary stream bytes directly. For PNG logos: `decode_rgba` → split into RGB stream (object `Im0`) and gray alpha stream (object `Im0` SMask, referenced via `/SMask N 0 R`; omit when every alpha is 255). Both uncompressed (`/Length` set to byte count). For JPEG: embed `img.bytes` verbatim with `/Filter /DCTDecode`.

Final structure (implement it exactly like this):

```rust
pub fn write(scene: &VectorScene) -> Vec<u8> {
    // 1. content: String as above (paths, colors, patterns by index, image cm)
    // 2. resources: /Pattern << /P0 5 0 R ... >> /ExtGState << /A0 .. >> /XObject << /Im0 .. >>
    // 3. aux objects in fixed order:
    //    for each gradient i: shading obj, pattern obj
    //    for each alpha j: extgstate obj
    //    if logo: image obj (+ smask obj when png has alpha)
    // 4. write objects sequentially into Vec<u8>, recording offsets
    // 5. xref: '0' entry + one entry per object, offsets = byte position of "N 0 obj"
    // 6. trailer /Size = object count + 1, /Root 1 0 R
}
```

Names in content stream must match resources: gradient i → `/P{i} scn` and resource `/P{i} <id> 0 R`; alpha j → `/A{j} gs` and resource `/A{j} <id> 0 R`. The `register_alpha` closure in the sketch returns `/A{idx} gs` and stores index — keep index-based naming, not id-based, to avoid the mismatch in the sketch.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p qrkit --test pdf_writer`
Expected: PASS (4 tests). Also open one PDF manually: write `b.to_pdf().unwrap()` to `/tmp/qr.pdf` and check `open /tmp/qr.pdf` renders in Preview (macOS). Then delete the temp file.

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/writers crates/qrkit/src/builder.rs crates/qrkit/tests/pdf_writer.rs
git commit -m "feat: add PDF writer with gradients, alpha, and image XObjects"
```

---

### Task 11: EPS writer

**Files:**
- Create: `crates/qrkit/src/writers/eps.rs`
- Modify: `crates/qrkit/src/writers/mod.rs`, `crates/qrkit/src/builder.rs` (add `to_eps`)
- Test: `crates/qrkit/tests/eps_writer.rs`

**Interfaces:**
- Consumes: `scene::*`, `render::to_path`, `common::linear_endpoints`, `logo::decode_rgba`.
- Produces: `writers::eps::write(&VectorScene) -> Vec<u8>`; `QrBuilder::to_eps() -> Result<Vec<u8>, Error>`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/eps_writer.rs`:

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::style::{Ecc, StyleConfig};
use qrkit::writers::eps;

#[test]
fn eps_header_and_bbox() {
    let img = QrBuilder::new(Content::Text("eps test".into())).unwrap().build().unwrap();
    let s = String::from_utf8(eps::write(&img.scene)).unwrap();
    assert!(s.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
    assert!(s.contains("%%BoundingBox: 0 0"));
    assert!(s.contains("%%EndComments"));
    assert!(s.trim_end().ends_with("%%EOF"));
}

#[test]
fn eps_solid_fill_ops() {
    let img = QrBuilder::new(Content::Text("eps test".into())).unwrap().build().unwrap();
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
    style.fill = qrkit::scene::Fill::Solid(qrkit::scene::Rgba { r: 0, g: 0, b: 0, a: 128 });
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit --test eps_writer`
Expected: compile error.

- [ ] **Step 3: Implement eps.rs**

`crates/qrkit/src/writers/eps.rs`:

```rust
//! PostScript Level 3 EPS writer.
//!
//! EPS has no alpha: every color is pre-blended against the effective
//! background (scene background if solid, else white) before writing.

use super::common::linear_endpoints;
use crate::logo::decode_rgba;
use crate::render::to_path;
use crate::scene::{Fill, FillRule, ImageFormat, PathCmd, Rgba, Shape, VectorScene};
use std::fmt::Write as _;

pub fn write(scene: &VectorScene) -> Vec<u8> {
    let s = scene.size;
    let fy = |y: f32| s - y;
    let bg = match &scene.background {
        Some(Fill::Solid(c)) => *c,
        _ => Rgba::WHITE, // transparent -> white in EPS (spec)
    };
    let blend = |c: &Rgba| blend_over(*c, bg);

    let mut out = String::new();
    let _ = writeln!(out, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(out, "%%BoundingBox: 0 0 {} {}", s.ceil() as i32, s.ceil() as i32);
    let _ = writeln!(out, "%%Creator: qrkit");
    let _ = writeln!(out, "%%LanguageLevel: 3");
    let _ = writeln!(out, "%%EndComments");

    if let Some(Fill::Solid(c)) = &scene.background {
        set_color(&blend(c), &mut out);
        let _ = writeln!(out, "0 0 {s:.3} {s:.3} rectfill");
    } else {
        // transparent background: leave unpainted (white paper)
    }

    for item in &scene.items {
        emit_path(&item.shape, &fy, &mut out);
        match &item.fill {
            Fill::Solid(c) => {
                set_color(&blend(c), &mut out);
                let op = if item.fill_rule == FillRule::EvenOdd { "eofill" } else { "fill" };
                let _ = writeln!(out, "{op}");
            }
            Fill::LinearGradient { from, to, angle_deg } => {
                let ((x0, y0), (x1, y1)) = linear_endpoints(s, *angle_deg);
                let _ = writeln!(out, "gsave clip");
                let _ = writeln!(
                    out,
                    "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >> shfill",
                    x0, fy(y0), x1, fy(y1),
                    ps_function(&blend(from), &blend(to))
                );
                let _ = writeln!(out, "grestore");
            }
            Fill::RadialGradient { from, to } => {
                let c = s / 2.0;
                let _ = writeln!(out, "gsave clip");
                let _ = writeln!(
                    out,
                    "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [{:.3} {:.3} 0 {:.3} {:.3} {:.3}] /Function {} /Extend [true true] >> shfill",
                    c, fy(c), c, fy(c), c / 2.0,
                    ps_function(&blend(from), &blend(to))
                );
                let _ = writeln!(out, "grestore");
            }
        }
    }

    if let Some(img) = &scene.logo {
        embed_image(img, bg, &mut out);
    }

    let _ = writeln!(out, "%%EOF");
    out.into_bytes()
}

fn blend_over(c: Rgba, bg: Rgba) -> Rgba {
    let a = c.a as f32 / 255.0;
    let mix = |fg: u8, b: u8| ((fg as f32 * a + b as f32 * (1.0 - a)) * 255.0).round() / 255.0;
    Rgba { r: mix(c.r, bg.r) as u8, g: mix(c.g, bg.g) as u8, b: mix(c.b, bg.b) as u8, a: 255 }
}

fn set_color(c: &Rgba, out: &mut String) {
    let _ = writeln!(
        out,
        "{:.3} {:.3} {:.3} setrgbcolor",
        c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0
    );
}

fn ps_function(from: &Rgba, to: &Rgba) -> String {
    format!(
        "<< /FunctionType 2 /Domain [0 1] /C0 [{:.3} {:.3} {:.3}] /C1 [{:.3} {:.3} {:.3}] /N 1 >>",
        from.r as f32 / 255.0, from.g as f32 / 255.0, from.b as f32 / 255.0,
        to.r as f32 / 255.0, to.g as f32 / 255.0, to.b as f32 / 255.0
    )
}

fn emit_path(shape: &Shape, fy: &impl Fn(f32) -> f32, out: &mut String) {
    let mut first = true;
    for cmd in to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => { let _ = writeln!(out, "{:.3} {:.3}{}", x, fy(y), if first { " newpath moveto".replace("newpath ", "") } else { String::new() }); first = false; }
            _ => first = false,
        }
        // (see corrected loop below)
    }
}
```

**Corrected `emit_path`** (the sketch above is deliberately awkward; implement it as):

```rust
fn emit_path(shape: &Shape, fy: &impl Fn(f32) -> f32, out: &mut String) {
    out.push_str("newpath\n");
    for cmd in to_path(shape) {
        match cmd {
            PathCmd::Move(x, y) => { let _ = writeln!(out, "{:.3} {:.3} moveto", x, fy(y)); }
            PathCmd::Line(x, y) => { let _ = writeln!(out, "{:.3} {:.3} lineto", x, fy(y)); }
            PathCmd::Cubic(a, b, c, d, e, g) => {
                let _ = writeln!(out, "{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} curveto", a, fy(b), c, fy(d), e, fy(g));
            }
            PathCmd::Close => out.push_str("closepath\n"),
        }
    }
}
```

And image embedding:

```rust
fn embed_image(img: &crate::scene::EmbeddedImage, bg: Rgba, out: &mut String) {
    let (w, h, rgba) = decode_rgba(img.format, &img.bytes).expect("logo decodes");
    let _ = writeln!(out, "gsave {:.3} {:.3} translate {:.3} {:.3} scale", img.x, img.y + img.h, img.w, img.h);
    let _ = writeln!(out, "/DeviceRGB setcolorspace");
    let _ = writeln!(
        out,
        "<< /ImageType 1 /Width {w} /Height {h} /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /ImageMatrix [{w} 0 0 {neg_h} 0 {h}] /DataSource currentfile /ASCIIHexDecode filter >> image",
        w = w, h = h, neg_h = -(h as i32)
    );
    // interleaved RGB hex, alpha pre-blended against bg, lines <= 120 chars
    let mut line = String::new();
    for px in rgba.chunks(4) {
        let c = blend_over(Rgba { r: px[0], g: px[1], b: px[2], a: px[3] }, bg);
        let _ = write!(line, "{:02x}{:02x}{:02x}", c.r, c.g, c.b);
        if line.len() >= 120 {
            let _ = writeln!(out, "{line}");
            line.clear();
        }
    }
    if !line.is_empty() {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, ">"); // ASCIIHexDecode EOD
    let _ = writeln!(out, "grestore");
}
```

Wait — the `translate` must flip to PostScript origin: scene coords are top-left, EPS bottom-left. The `translate` above uses `img.y + img.h` un-flipped with an `ImageMatrix` row flip; implement exactly as shown and visually verify in Step 4. If the logo lands in the wrong corner, replace with `translate(img.x, fy(img.y) - img.h)` — decide by opening the file.

`crates/qrkit/src/writers/mod.rs` becomes:

```rust
//! Format writers: pure serializers of a VectorScene.

pub mod common;
pub mod eps;
pub mod pdf;
pub mod svg;
```

Add to `builder.rs`:

```rust
    /// Build and serialize to EPS (alpha pre-blended; see eps module docs).
    pub fn to_eps(&self) -> Result<Vec<u8>, Error> {
        Ok(crate::writers::eps::write(&self.build()?.scene))
    }
```

- [ ] **Step 4: Run tests and visually verify**

Run: `cargo test -p qrkit --test eps_writer`
Expected: PASS (5 tests).

Manual check (macOS): generate `/tmp/qr.eps`, convert with `qlmanage -p /tmp/qr.eps` or open in Preview, confirm modules/eyes/logo positions match the SVG (`/tmp/qr.svg` opened in a browser). Delete temp files afterwards.

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/src/writers crates/qrkit/src/builder.rs crates/qrkit/tests/eps_writer.rs
git commit -m "feat: add EPS writer with shfill gradients and hex image embedding"
```

---

### Task 12: Full style-matrix e2e + scene round-trip

**Files:**
- Test: `crates/qrkit/tests/e2e_matrix.rs`

**Interfaces:**
- Consumes: `common::rasterize/decode` (Task 9), renderer, `Matrix`.
- Produces: test coverage across every module-shape × eye-shape × logo combination, plus a matrix-reconstruction round-trip (`reconstruct(&VectorScene, matrix_size, margin) -> Vec<Vec<bool>>` test helper defined inside this test file).

- [ ] **Step 1: Write failing tests**

`crates/qrkit/tests/e2e_matrix.rs`:

```rust
mod common;

use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::logo::LogoConfig;
use qrkit::scene::{CornerRadii, FillRule, PathCmd, Rgba, Shape, VectorScene};
use qrkit::style::{Ecc, EyeShape, ModuleShape, StyleConfig};

const PAYLOAD: &str = "https://example.com/matrix";

/// Rebuild a module matrix from scene geometry: a module is dark when its
/// center point is inside any item's shape (bezier flattened to 16 segments).
fn reconstruct(scene: &VectorScene, n: i32, margin: i32) -> Vec<Vec<bool>> {
    let pts = |shape: &Shape| -> Vec<Vec<(f32, f32)>> {
        // returns Vec of subpaths (polygons)
        let mut polys: Vec<Vec<(f32, f32)>> = vec![Vec::new()];
        let mut cur = 0;
        let mut last = (0.0f32, 0.0f32);
        let flatten = |from: (f32, f32), c1: (f32, f32), c2: (f32, f32), to: (f32, f32), poly: &mut Vec<(f32, f32)>| {
            for i in 1..=16 {
                let t = i as f32 / 16.0;
                let mt = 1.0 - t;
                let x = mt * mt * mt * from.0 + 3.0 * mt * mt * t * c1.0 + 3.0 * mt * t * t * c2.0 + t * t * t * to.0;
                let y = mt * mt * mt * from.1 + 3.0 * mt * mt * t * c1.1 + 3.0 * mt * t * t * c2.1 + t * t * t * to.1;
                poly.push((x, y));
            }
        };
        for cmd in qrkit::render::to_path(shape) {
            match cmd {
                PathCmd::Move(x, y) => { polys.push(Vec::new()); cur = polys.len() - 1; last = (x, y); }
                PathCmd::Line(x, y) => { polys[cur].push(last); last = (x, y); }
                PathCmd::Cubic(a, b, c, d, e, f) => {
                    polys[cur].push(last);
                    flatten(last, (a, b), (c, d), (e, f), &mut polys[cur]);
                    last = (e, f);
                }
                PathCmd::Close => polys[cur].push(last),
            }
        }
        polys.retain(|p| p.len() >= 3);
        polys
    };
    let inside = |px: f32, py: f32, poly: &[(f32, f32)]| -> bool {
        let mut hit = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (xi, yi) = poly[i];
            let (xj, yj) = poly[j];
            if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                hit = !hit;
            }
            j = i;
        }
        hit
    };
    let shape_hits = |shape: &Shape, px: f32, py: f32, rule: FillRule| -> bool {
        match shape {
            Shape::Rect { .. } | Shape::RoundedRect { .. } | Shape::Circle { .. } | Shape::Path(_) => {
                let polys = pts(shape);
                let cnt = polys.iter().filter(|p| inside(px, py, p)).count();
                match rule { FillRule::EvenOdd => cnt % 2 == 1, FillRule::NonZero => cnt > 0 }
            }
        }
    };
    let mut grid = vec![vec![false; n as usize]; n as usize];
    for y in 0..n {
        for x in 0..n {
            let (px, py) = ((x + margin) as f32 + 0.5, (y + margin) as f32 + 0.5);
            if scene.items.iter().any(|i| shape_hits(&i.shape, px, py, i.fill_rule)) {
                grid[y as usize][x as usize] = true;
            }
        }
    }
    grid
}

fn dark_modules_equal(scene: &VectorScene, expected_dark: usize, n: i32, margin: i32) {
    let grid = reconstruct(scene, n, margin);
    let dark = grid.iter().map(|r| r.iter().filter(|v| **v).count()).sum::<usize>();
    assert_eq!(dark, expected_dark, "scene covers {dark} modules, matrix has {expected_dark}");
}

#[test]
fn scene_covers_exactly_the_matrix_modules() {
    let m = qrkit::render::Matrix::encode(PAYLOAD, Ecc::High, qrkit::style::Version::Auto).unwrap();
    let expected = (0..m.size()).map(|y| (0..m.size()).map(move |x| (x, y)))
        .flatten().filter(|&(x, y)| m.get(x, y)).count();
    let img = QrBuilder::new(Content::Url(PAYLOAD.into())).unwrap()
        .style(StyleConfig::new().margin_modules(4)).build().unwrap();
    dark_modules_equal(&img.scene, expected, m.size(), 4);
}

#[test]
fn every_shape_eye_and_logo_combination_decodes() {
    let shapes = [
        ModuleShape::Square,
        ModuleShape::Rounded,
        ModuleShape::Dots { radius: 0.5 },
        ModuleShape::Dots { radius: 0.35 },
        ModuleShape::Classy,
    ];
    let eyes = [EyeShape::Square, EyeShape::Rounded, EyeShape::Circle, EyeShape::Leaf];
    let logo = LogoConfig::from_bytes(include_bytes!("fixtures/logo.png").to_vec()).ok();
    for shape in shapes {
        for eye in eyes {
            for with_logo in [false, true] {
                let mut style = StyleConfig::new().shape(shape).eye_shape(eye);
                if with_logo {
                    style = style.logo(logo.clone()).ecc(Ecc::High);
                }
                let img = QrBuilder::new(Content::Url(PAYLOAD.into())).unwrap()
                    .style(style).build().unwrap();
                let pm = common::rasterize(&img.scene, 1024);
                assert_eq!(
                    common::decode(&pm), PAYLOAD,
                    "decode failed for shape={shape:?} eye={eye:?} logo={with_logo}"
                );
            }
        }
    }
}

#[test]
fn gradient_fill_scene_decodes() {
    let img = QrBuilder::new(Content::Text(PAYLOAD.into())).unwrap()
        .style(StyleConfig::new().fill(qrkit::scene::Fill::LinearGradient {
            from: Rgba { r: 13, g: 71, b: 161, a: 255 },
            to: Rgba { r: 66, g: 165, b: 245, a: 255 },
            angle_deg: 45.0,
        }))
        .build().unwrap();
    let pm = common::rasterize(&img.scene, 1024);
    assert_eq!(common::decode(&pm), PAYLOAD);
}

#[test]
fn transparent_background_scene_decodes() {
    // tiny-skia pixmap starts transparent-black; dark modules still decode
    let img = QrBuilder::new(Content::Text(PAYLOAD.into())).unwrap()
        .style(StyleConfig::new().background(None)).build().unwrap();
    let pm = common::rasterize(&img.scene, 1024);
    assert_eq!(common::decode(&pm), PAYLOAD);
}

#[test]
fn classy_corners_use_diagonal_radii() {
    let img = QrBuilder::new(Content::Text("x".into())).unwrap()
        .style(StyleConfig::new().shape(ModuleShape::Classy)).build().unwrap();
    let any_classy = img.scene.items.iter().any(|i| matches!(
        i.shape,
        Shape::RoundedRect { radii: CornerRadii { tl: 0.0, tr: r, br: 0.0, bl: r2 }, .. } if r == 0.5 && r2 == 0.5
    ));
    assert!(any_classy);
}

// silence unused warnings for helper fields referenced by macros
#[allow(dead_code)]
fn _scene_type_used(_: &VectorScene) {}
```

- [ ] **Step 2: Run tests to verify they fail/pass baseline**

Run: `cargo test -p qrkit --test e2e_matrix`
Expected: compile first; then the 40-combination decode test is the real gate. If a specific combination fails to decode, that is a renderer bug — fix the renderer (not the test) before continuing.

- [ ] **Step 3: Fix any failures**

Iterate: run single cases with `cargo test -p qrkit --test e2e_matrix -- --nocapture every_shape`. Typical fixes: dot radius too small against quiet zone (keep ≥ 0.35 decode-safe default docs), eye inner dot color contrast (dot follows module fill — if module fill is a light gradient, decode fails; that is user error, tests use dark fills).

- [ ] **Step 4: Run the whole workspace test suite**

Run: `cargo test --workspace`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/qrkit/tests/e2e_matrix.rs
git commit -m "test: add full style-matrix decode tests and scene round-trip"
```

---

### Task 13: CLI

**Files:**
- Create: `crates/qrkit-cli/Cargo.toml`, `crates/qrkit-cli/src/main.rs`, `crates/qrkit-cli/src/args.rs`, `crates/qrkit-cli/src/preview.rs`
- Test: `crates/qrkit-cli/tests/args_to_content.rs`
- Modify: root `Cargo.toml` (members)

**Interfaces:**
- Consumes: `qrkit::{builder, content, logo, render::Matrix, scene, style}`.
- Produces: binary `qrkit` with subcommands `text|url|email|sms|phone|wifi|vcard|geo|calendar`, style flags, `-o/--output`, `--format`, `--preview`; pure conversion functions in `args.rs`: `content_from_args(&Cli) -> Result<Content, Error>` and `style_from_args(&Cli) -> Result<StyleConfig, Error>` (unit-tested); `preview::half_blocks(&Matrix, margin) -> String`.

- [ ] **Step 1: Write failing tests**

`crates/qrkit-cli/tests/args_to_content.rs`:

```rust
use clap::Parser;
use qrkit::content::{Content, Encryption};
use qrkit_cli::args::{content_from_args, style_from_args, Cli, Commands};
use qrkit::style::{Ecc, ModuleShape, Version};

fn cli_for(cmd: Commands) -> Cli {
    Cli::try_parse_from(["qrkit", "x"]).ok(); // compile check
    Cli { command: cmd }
}

#[test]
fn wifi_args_map_to_content() {
    let mut cli = cli_for(Commands::Wifi {
        ssid: "Cafe Guest".into(),
        password: Some("latte-2026".into()),
        encryption: None,
        hidden: true,
    });
    let _ = &mut cli;
    let c = content_from_args(&cli_for(Commands::Wifi {
        ssid: "Cafe Guest".into(),
        password: Some("latte-2026".into()),
        encryption: None,
        hidden: true,
    })).unwrap();
    match c {
        Content::Wifi { ssid, password, encryption, hidden } => {
            assert_eq!(ssid, "Cafe Guest");
            assert_eq!(password.as_deref(), Some("latte-2026"));
            assert_eq!(encryption, Encryption::Wpa); // default
            assert!(hidden);
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn url_args_normalize_scheme() {
    let c = content_from_args(&cli_for(Commands::Url { url: "example.com/promo".into() })).unwrap();
    assert_eq!(c.payload().unwrap(), "https://example.com/promo");
}

#[test]
fn style_flags_map_to_config() {
    let mut cli = cli_for(Commands::Text { text: "hi".into() });
    cli.fg = Some("#0d47a1".into());
    cli.gradient_to = Some("#42a5f5".into());
    cli.gradient_angle = 45.0;
    cli.shape = Some("dots".into());
    cli.dot_radius = 0.4;
    cli.eye = Some("circle".into());
    cli.ecc = Some("high".into());
    cli.margin = 2;
    cli.version = Some(10);
    let s = style_from_args(&cli).unwrap();
    assert_eq!(s.ecc, Ecc::High);
    assert_eq!(s.margin_modules, 2);
    assert_eq!(s.version, Version::Fixed(10));
    assert_eq!(s.eye_shape, qrkit::style::EyeShape::Circle);
    assert!(matches!(s.shape, ModuleShape::Dots { radius: 0.4 }));
    assert!(matches!(s.fill, qrkit::scene::Fill::LinearGradient { .. }));
}

#[test]
fn bg_transparent_maps_to_none() {
    let mut cli = cli_for(Commands::Text { text: "hi".into() });
    cli.bg = Some("transparent".into());
    let s = style_from_args(&cli).unwrap();
    assert_eq!(s.background, None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit-cli --test args_to_content`
Expected: compile error (crate does not exist).

- [ ] **Step 3: Implement the CLI crate**

Add `"crates/qrkit-cli"` to workspace members.

`crates/qrkit-cli/Cargo.toml`:

```toml
[package]
name = "qrkit-cli"
description = "Command-line QR code generator with vector output"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "qrkit"
path = "src/main.rs"

[dependencies]
qrkit = { path = "../qrkit" }
clap.workspace = true
```

`crates/qrkit-cli/src/args.rs`:

```rust
//! CLI argument definitions and mapping to library types.

use clap::{Parser, Subcommand};
use qrkit::content::{CalendarEvent, Content, Encryption, VCard};
use qrkit::logo::LogoConfig;
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, EyeShape, ModuleShape, StyleConfig, Version};
use qrkit::Error;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "qrkit", version, about = "Customizable QR codes as SVG, PDF, and EPS")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output file; format inferred from extension (svg|pdf|eps).
    /// Omit to write SVG to stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Force output format (overrides extension).
    #[arg(long)]
    pub format: Option<String>,

    /// Error correction: low|medium|quartile|high (default quartile).
    #[arg(long)]
    pub ecc: Option<String>,

    /// Fixed QR version 1..=40 (default: auto).
    #[arg(long)]
    pub version: Option<u8>,

    /// Quiet-zone margin in modules (default 4).
    #[arg(long)]
    pub margin: u32 = 4,

    /// Module shape: square|rounded|dots|classy (default square).
    #[arg(long)]
    pub shape: Option<String>,

    /// Dot radius for --shape dots (default 0.5).
    #[arg(long)]
    pub dot_radius: f32 = 0.5,

    /// Eye shape: square|rounded|circle|leaf (default square).
    #[arg(long)]
    pub eye: Option<String>,

    /// Foreground / module color, hex (default #000000).
    #[arg(long)]
    pub fg: Option<String>,

    /// Background color, hex or "transparent" (default #ffffff).
    #[arg(long)]
    pub bg: Option<String>,

    /// Gradient kind: linear|radial. Enables gradient fill from --fg to --gradient-to.
    #[arg(long)]
    pub gradient: Option<String>,

    /// Gradient end color, hex.
    #[arg(long)]
    pub gradient_to: Option<String>,

    /// Linear gradient angle in degrees (default 0).
    #[arg(long, default_value_t = 0.0)]
    pub gradient_angle: f32,

    /// Eye ring color, hex (default: same as --fg).
    #[arg(long)]
    pub eye_color: Option<String>,

    /// Eye inner dot color, hex (default: module fill).
    #[arg(long)]
    pub eye_inner_color: Option<String>,

    /// PNG/JPEG logo file path.
    #[arg(long)]
    pub logo: Option<PathBuf>,

    /// Logo size as fraction of code size, 0.1..=0.33 (default 0.25).
    #[arg(long)]
    pub logo_size: Option<f32>,

    /// Logo padding in modules (default 1.0).
    #[arg(long)]
    pub logo_padding: Option<f32>,

    /// Render a Unicode preview on stderr.
    #[arg(long)]
    pub preview: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Plain text
    Text { text: String },
    /// URL (https:// added when the scheme is missing)
    Url { url: String },
    /// Email message
    Email {
        #[arg(long)] to: String,
        #[arg(long)] subject: Option<String>,
        #[arg(long)] body: Option<String>,
    },
    /// SMS message
    Sms {
        #[arg(long)] number: String,
        #[arg(long)] message: Option<String>,
    },
    /// Phone number
    Phone { number: String },
    /// WiFi network login
    Wifi {
        #[arg(long)] ssid: String,
        #[arg(long)] password: Option<String>,
        #[arg(long)] encryption: Option<String>,
        #[arg(long, default_value_t = false)] hidden: bool,
    },
    /// vCard contact
    Vcard {
        #[arg(long)] name: String,
        #[arg(long)] org: Option<String>,
        #[arg(long)] title: Option<String>,
        #[arg(long)] phone: Vec<String>,
        #[arg(long)] email: Vec<String>,
        #[arg(long)] url: Option<String>,
        #[arg(long)] address: Option<String>,
        #[arg(long)] note: Option<String>,
    },
    /// Geo location
    Geo {
        #[arg(long)] lat: f64,
        #[arg(long)] lon: f64,
    },
    /// Calendar event (UTC timestamps YYYYMMDDTHHMMSSZ)
    Calendar {
        #[arg(long)] title: String,
        #[arg(long)] start: String,
        #[arg(long)] end: String,
        #[arg(long)] location: Option<String>,
        #[arg(long)] description: Option<String>,
    },
}

pub fn content_from_args(cli: &Cli) -> Result<Content, Error> {
    Ok(match &cli.command {
        Commands::Text { text } => Content::Text(text.clone()),
        Commands::Url { url } => {
            let normalized = if url.contains("://") { url.clone() } else { format!("https://{url}") };
            Content::Url(normalized)
        }
        Commands::Email { to, subject, body } => Content::Email { to: to.clone(), subject: subject.clone(), body: body.clone() },
        Commands::Sms { number, message } => Content::Sms { number: number.clone(), message: message.clone() },
        Commands::Phone { number } => Content::Phone(number.clone()),
        Commands::Wifi { ssid, password, encryption, hidden } => Content::Wifi {
            ssid: ssid.clone(),
            password: password.clone(),
            encryption: match encryption.as_deref() {
                None | Some("wpa") => Encryption::Wpa,
                Some("wep") => Encryption::Wep,
                Some("nopass") => Encryption::Nopass,
                Some(other) => return Err(Error::Content(format!("unknown encryption '{other}', expected wpa|wep|nopass"))),
            },
            hidden: *hidden,
        },
        Commands::Vcard { name, org, title, phone, email, url, address, note } => Content::VCard(VCard {
            name: name.clone(), org: org.clone(), title: title.clone(),
            phones: phone.clone(), emails: email.clone(),
            url: url.clone(), address: address.clone(), note: note.clone(),
        }),
        Commands::Geo { lat, lon } => Content::Geo { lat: *lat, lon: *lon },
        Commands::Calendar { title, start, end, location, description } => {
            let ev = CalendarEvent {
                title: title.clone(), start_utc: start.clone(), end_utc: end.clone(),
                location: location.clone(), description: description.clone(),
            };
            qrkit::content::validate_utc_timestamp(&ev.start_utc)?;
            qrkit::content::validate_utc_timestamp(&ev.end_utc)?;
            Content::Calendar(ev)
        }
    })
}

pub fn style_from_args(cli: &Cli) -> Result<StyleConfig, Error> {
    let hex = |v: &Option<String>, what: &str| -> Result<Rgba, Error> {
        Rgba::from_hex(v.as_deref().unwrap_or("#000000")).map_err(|_| Error::InvalidStyle(format!("invalid {what} color")))
    };
    let fg = hex(&cli.fg, "--fg")?;

    let fill = match (&cli.gradient, &cli.gradient_to) {
        (Some(kind), Some(to)) => {
            let to = Rgba::from_hex(to).map_err(|_| Error::InvalidStyle("invalid --gradient-to color".into()))?;
            match kind.as_str() {
                "linear" => Fill::LinearGradient { from: fg, to, angle_deg: cli.gradient_angle },
                "radial" => Fill::RadialGradient { from: fg, to },
                other => return Err(Error::InvalidStyle(format!("unknown gradient kind '{other}', expected linear|radial"))),
            }
        }
        (Some(_), None) => return Err(Error::InvalidStyle("--gradient requires --gradient-to".into())),
        _ => Fill::Solid(fg),
    };

    let shape = match cli.shape.as_deref() {
        None | Some("square") => ModuleShape::Square,
        Some("rounded") => ModuleShape::Rounded,
        Some("dots") => ModuleShape::Dots { radius: cli.dot_radius },
        Some("classy") => ModuleShape::Classy,
        Some(other) => return Err(Error::InvalidStyle(format!("unknown shape '{other}', expected square|rounded|dots|classy"))),
    };
    let eye_shape = match cli.eye.as_deref() {
        None | Some("square") => EyeShape::Square,
        Some("rounded") => EyeShape::Rounded,
        Some("circle") => EyeShape::Circle,
        Some("leaf") => EyeShape::Leaf,
        Some(other) => return Err(Error::InvalidStyle(format!("unknown eye shape '{other}', expected square|rounded|circle|leaf"))),
    };
    let ecc = match cli.ecc.as_deref() {
        None => Ecc::Quartile,
        Some("low") => Ecc::Low,
        Some("medium") => Ecc::Medium,
        Some("quartile") => Ecc::Quartile,
        Some("high") => Ecc::High,
        Some(other) => return Err(Error::InvalidStyle(format!("unknown ecc '{other}', expected low|medium|quartile|high"))),
    };
    let background = match cli.bg.as_deref() {
        None => Some(Fill::Solid(Rgba::WHITE)),
        Some("transparent") | Some("none") => None,
        Some(hexstr) => Some(Fill::Solid(Rgba::from_hex(hexstr).map_err(|_| Error::InvalidStyle("invalid --bg color".into()))?)),
    };
    let eye_fill = match &cli.eye_color {
        Some(c) => Fill::Solid(Rgba::from_hex(c).map_err(|_| Error::InvalidStyle("invalid --eye-color".into()))?),
        None => fill.clone(),
    };
    let eye_inner_fill = match &cli.eye_inner_color {
        Some(c) => Some(Fill::Solid(Rgba::from_hex(c).map_err(|_| Error::InvalidStyle("invalid --eye-inner-color".into()))?)),
        None => None,
    };
    let logo = match &cli.logo {
        Some(path) => Some(
            LogoConfig::from_bytes(std::fs::read(path).map_err(|e| Error::Content(format!("cannot read logo {path:?}: {e}")))?)
                .map_err(|e| Error::Content(format!("logo {path:?}: {e}")))?,
        ),
        None => None,
    };
    let mut style = StyleConfig::new()
        .ecc(ecc)
        .version(if let Some(v) = cli.version { Version::Fixed(v) } else { Version::Auto })
        .margin_modules(cli.margin)
        .shape(shape)
        .eye_shape(eye_shape)
        .fill(fill)
        .eye_fill(eye_fill)
        .eye_inner_fill(eye_inner_fill)
        .background(background)
        .logo(logo);
    if let (Some(l), Some(size)) = (&mut style.logo, cli.logo_size) {
        l.size_ratio = size;
    }
    if let (Some(l), Some(pad)) = (&mut style.logo, cli.logo_padding) {
        l.padding = pad;
    }
    Ok(style)
}

/// Output format: explicit --format, else output extension, else svg.
pub fn output_format(cli: &Cli) -> Result<&'static str, Error> {
    if let Some(f) = &cli.format {
        return match f.as_str() {
            "svg" | "pdf" | "eps" => Ok(f.as_str()),
            other => Err(Error::InvalidStyle(format!("unknown format '{other}', expected svg|pdf|eps"))),
        };
    }
    if let Some(out) = &cli.output {
        return match out.extension().and_then(|e| e.to_str()) {
            Some("svg") => Ok("svg"),
            Some("pdf") => Ok("pdf"),
            Some("eps") => Ok("eps"),
            _ => Err(Error::InvalidStyle("cannot infer format from extension; use --format svg|pdf|eps".into())),
        };
    }
    Ok("svg")
}
```

`crates/qrkit-cli/src/preview.rs`:

```rust
//! Terminal preview using Unicode half blocks.

use qrkit::render::Matrix;

/// Two matrix rows per character line. `margin` adds a blank border.
pub fn half_blocks(m: &Matrix, margin: u32) -> String {
    let n = m.size();
    let full = n + 2 * margin as i32;
    let dark = |x: i32, y: i32| -> bool {
        let (mx, my) = (x - margin as i32, y - margin as i32);
        (0..n).contains(&mx) && (0..n).contains(&my) && m.get(mx, my)
    };
    let mut out = String::new();
    let mut y = 0;
    while y < full {
        for x in 0..full {
            let top = dark(x, y);
            let bottom = y + 1 < full && dark(x, y + 1);
            out.push(match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        out.push('\n');
        y += 2;
    }
    out
}
```

`crates/qrkit-cli/src/main.rs`:

```rust
mod args;
mod preview;

use args::{content_from_args, output_format, style_from_args, Cli};
use clap::Parser;
use qrkit::builder::QrBuilder;
use qrkit::style::Ecc;

fn main() {
    let cli = Cli::parse();
    if let Err(msg) = run(&cli) {
        eprintln!("error: {msg}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let content = content_from_args(cli).map_err(|e| e.to_string())?;
    let style = style_from_args(cli).map_err(|e| e.to_string())?;
    let builder = QrBuilder::new(content).map_err(|e| e.to_string())?.style(style);

    if cli.preview {
        let img = builder.build().map_err(|e| e.to_string())?;
        // re-encode a plain matrix for the half-block preview (style-independent)
        let payload = match content_from_args(cli) {
            Ok(c) => c.payload().map_err(|e| e.to_string())?,
            Err(e) => return Err(e.to_string()),
        };
        let matrix = qrkit::render::Matrix::encode(&payload, Ecc::High, qrkit::style::Version::Auto)
            .map_err(|e| e.to_string())?;
        eprint!("{}", preview::half_blocks(&matrix, 2));
        for w in &img.warnings {
            eprintln!("warning: {w}");
        }
    }

    let format = output_format(cli).map_err(|e| e.to_string())?;
    let bytes = match format {
        "svg" => builder.to_svg().map_err(|e| e.to_string())?,
        "pdf" => builder.to_pdf().map_err(|e| e.to_string())?,
        "eps" => builder.to_eps().map_err(|e| e.to_string())?,
        _ => unreachable!(),
    };

    match &cli.output {
        Some(path) => {
            std::fs::write(path, bytes).map_err(|e| format!("cannot write {path:?}: {e}"))?;
            eprintln!("wrote {path:?} ({format})");
        }
        None => {
            use std::io::Write as _;
            std::io::stdout().write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests and manual smoke**

Run: `cargo test -p qrkit-cli && cargo run -p qrkit-cli -- url "https://example.com" -o /tmp/qr.svg && head -c 200 /tmp/qr.svg && cargo run -p qrkit-cli -- wifi --ssid "Cafe Guest" --password "latte-2026" --preview -o /tmp/qr.pdf && cargo run -p qrkit-cli -- geo --lat 51.5 --lon -0.12 -o /tmp/qr.eps`
Expected: tests pass; three files written; preview shows half-block QR; `open /tmp/qr.pdf` renders. Delete temp files after.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/qrkit-cli
git commit -m "feat: add qrkit CLI with all content types, styling, and preview"
```

---

### Task 14: WASM binding

**Files:**
- Create: `crates/qrkit-wasm/Cargo.toml`, `crates/qrkit-wasm/src/lib.rs`
- Test: `crates/qrkit-wasm/tests/config.rs`
- Modify: root `Cargo.toml` (members)

**Interfaces:**
- Consumes: `qrkit` public API.
- Produces: `#[wasm_bindgen] pub fn generate(config_json: &str, format: &str) -> Result<Generated, JsError>` with `Generated { svg(), pdf(), eps(), warnings }` (getters; `Vec<u8>` becomes `Uint8Array`); internal pure fn `render_to_bytes(config_json: &str, format: &str) -> Result<(Option<Vec<u8>>, Vec<String>), String>` — unit-tested without a browser.

- [ ] **Step 1: Write failing tests**

`crates/qrkit-wasm/tests/config.rs`:

```rust
use qrkit_wasm::render_to_bytes;

const CFG: &str = r#"{
  "content": { "type": "wifi", "ssid": "Cafe Guest", "password": "latte-2026", "encryption": "wpa", "hidden": false },
  "style": { "ecc": "high", "margin": 4, "shape": "dots", "dotRadius": 0.5, "eye": "circle",
             "fg": "#0d47a1", "bg": "#ffffff" }
}"#;

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
        r#"{{"content": {{"type": "text", "text": "logo wasm"}}},
            "style": {{"ecc": "high", "logo": {{"dataBase64": "{}", "sizeRatio": 0.25}}}}}}"#,
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    let (bytes, _) = render_to_bytes(&cfg, "svg").unwrap();
    assert!(String::from_utf8(bytes.unwrap()).unwrap().contains("data:image/png;base64,"));
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
```

Add to `crates/qrkit-wasm/Cargo.toml` dev-deps: `base64.workspace = true`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p qrkit-wasm`
Expected: compile error.

- [ ] **Step 3: Implement the wasm crate**

Add `"crates/qrkit-wasm"` to workspace members.

`crates/qrkit-wasm/Cargo.toml`:

```toml
[package]
name = "qrkit-wasm"
description = "WebAssembly binding for qrkit"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
qrkit = { path = "../qrkit" }
wasm-bindgen.workspace = true
serde.workspace = true
serde_json.workspace = true
base64.workspace = true

[dev-dependencies]
base64.workspace = true
```

`crates/qrkit-wasm/src/lib.rs`:

```rust
//! WASM binding: one `generate(config_json, format)` entry point.
//!
//! The config mirrors the library types with camelCase field names; see
//! tests/config.rs for the exact JSON shape.

use base64::Engine as _;
use wasm_bindgen::prelude::*;

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WContent {
    Text { text: String },
    Url { url: String },
    Email { to: String, subject: Option<String>, body: Option<String> },
    Sms { number: String, message: Option<String> },
    Phone { number: String },
    Wifi { ssid: String, password: Option<String>, encryption: Option<String>, hidden: Option<bool> },
    Vcard {
        name: String,
        org: Option<String>,
        title: Option<String>,
        phones: Option<Vec<String>>,
        emails: Option<Vec<String>>,
        url: Option<String>,
        address: Option<String>,
        note: Option<String>,
    },
    Geo { lat: f64, lon: f64 },
    Calendar { title: String, start_utc: String, end_utc: String, location: Option<String>, description: Option<String> },
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WLogo {
    data_base64: String,
    size_ratio: Option<f32>,
    padding: Option<f32>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WStyle {
    ecc: Option<String>,
    margin: Option<u32>,
    version: Option<u8>,
    shape: Option<String>,
    dot_radius: Option<f32>,
    eye: Option<String>,
    fg: Option<String>,
    bg: Option<String>,
    gradient: Option<String>,
    gradient_to: Option<String>,
    gradient_angle: Option<f32>,
    eye_color: Option<String>,
    eye_inner_color: Option<String>,
    logo: Option<WLogo>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WConfig {
    content: WContent,
    style: Option<WStyle>,
}

/// Pure conversion + generation, testable without a browser.
pub fn render_to_bytes(config_json: &str, format: &str) -> Result<(Option<Vec<u8>>, Vec<String>), String> {
    let cfg: WConfig = serde_json::from_str(config_json).map_err(|e| format!("config: {e}"))?;
    let content = to_content(cfg.content)?;
    let style = cfg.style.map(to_style).transpose()?.unwrap_or_default();
    let builder = qrkit::builder::QrBuilder::new(content).map_err(|e| e.to_string())?.style(style);
    let img = builder.build().map_err(|e| e.to_string())?;
    let bytes = match format {
        "svg" => qrkit::writers::svg::write(&img.scene),
        "pdf" => qrkit::writers::pdf::write(&img.scene),
        "eps" => qrkit::writers::eps::write(&img.scene),
        other => return Err(format!("unknown format '{other}', expected svg|pdf|eps")),
    };
    Ok((Some(bytes), img.warnings))
}

/// Wasm-bindgen result holder.
#[wasm_bindgen]
pub struct Generated {
    svg: Option<Vec<u8>>,
    pdf: Option<Vec<u8>>,
    eps: Option<Vec<u8>>,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl Generated {
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> Option<String> {
        self.svg.as_ref().map(|b| String::from_utf8_lossy(b).into_owned())
    }
    #[wasm_bindgen(getter)]
    pub fn pdf(&self) -> Option<Vec<u8>> {
        self.pdf.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn eps(&self) -> Option<Vec<u8>> {
        self.eps.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> Vec<JsValue> {
        self.warnings.iter().map(JsValue::from_str).collect()
    }
}

#[wasm_bindgen]
pub fn generate(config_json: &str, format: &str) -> Result<Generated, JsError> {
    let (bytes, warnings) = render_to_bytes(config_json, format).map_err(JsError::new)?;
    let mut out = Generated { svg: None, pdf: None, eps: None, warnings };
    match format {
        "svg" => out.svg = bytes,
        "pdf" => out.pdf = bytes,
        "eps" => out.eps = bytes,
        _ => return Err(JsError::new("unknown format")),
    }
    Ok(out)
}

fn to_content(c: WContent) -> Result<qrkit::content::Content, String> {
    use qrkit::content::{CalendarEvent, Content, Encryption, VCard};
    Ok(match c {
        WContent::Text { text } => Content::Text(text),
        WContent::Url { url } => {
            let normalized = if url.contains("://") { url } else { format!("https://{url}") };
            Content::Url(normalized)
        }
        WContent::Email { to, subject, body } => Content::Email { to, subject, body },
        WContent::Sms { number, message } => Content::Sms { number, message },
        WContent::Phone { number } => Content::Phone(number),
        WContent::Wifi { ssid, password, encryption, hidden } => Content::Wifi {
            ssid,
            password,
            encryption: match encryption.as_deref() {
                None | Some("wpa") => Encryption::Wpa,
                Some("wep") => Encryption::Wep,
                Some("nopass") => Encryption::Nopass,
                Some(o) => return Err(format!("encryption: expected wpa|wep|nopass, got '{o}'")),
            },
            hidden: hidden.unwrap_or(false),
        },
        WContent::Vcard { name, org, title, phones, emails, url, address, note } => Content::VCard(VCard {
            name, org, title,
            phones: phones.unwrap_or_default(),
            emails: emails.unwrap_or_default(),
            url, address, note,
        }),
        WContent::Geo { lat, lon } => Content::Geo { lat, lon },
        WContent::Calendar { title, start_utc, end_utc, location, description } => {
            let ev = CalendarEvent { title, start_utc, end_utc, location, description };
            qrkit::content::validate_utc_timestamp(&ev.start_utc).map_err(|e| e.to_string())?;
            qrkit::content::validate_utc_timestamp(&ev.end_utc).map_err(|e| e.to_string())?;
            Content::Calendar(ev)
        }
    })
}

fn to_style(s: WStyle) -> Result<qrkit::style::StyleConfig, String> {
    use qrkit::logo::LogoConfig;
    use qrkit::scene::{Fill, Rgba};
    use qrkit::style::{Ecc, EyeShape, ModuleShape, StyleConfig, Version};

    let hex = |v: &str| -> Result<Rgba, String> { Rgba::from_hex(v).map_err(|e| e.to_string()) };
    let fg = hex(s.fg.as_deref().unwrap_or("#000000"))?;

    let fill = match (&s.gradient, &s.gradient_to) {
        (Some(kind), Some(to)) => {
            let to = hex(to)?;
            match kind.as_str() {
                "linear" => Fill::LinearGradient { from: fg, to, angle_deg: s.gradient_angle.unwrap_or(0.0) },
                "radial" => Fill::RadialGradient { from: fg, to },
                o => return Err(format!("gradient: expected linear|radial, got '{o}'")),
            }
        }
        (Some(_), None) => return Err("gradientTo is required when gradient is set".into()),
        _ => Fill::Solid(fg),
    };
    let shape = match s.shape.as_deref() {
        None | Some("square") => ModuleShape::Square,
        Some("rounded") => ModuleShape::Rounded,
        Some("dots") => ModuleShape::Dots { radius: s.dot_radius.unwrap_or(0.5) },
        Some("classy") => ModuleShape::Classy,
        Some(o) => return Err(format!("shape: expected square|rounded|dots|classy, got '{o}'")),
    };
    let eye = match s.eye.as_deref() {
        None | Some("square") => EyeShape::Square,
        Some("rounded") => EyeShape::Rounded,
        Some("circle") => EyeShape::Circle,
        Some("leaf") => EyeShape::Leaf,
        Some(o) => return Err(format!("eye: expected square|rounded|circle|leaf, got '{o}'")),
    };
    let ecc = match s.ecc.as_deref() {
        None => Ecc::Quartile,
        Some("low") | Some("l") => Ecc::Low,
        Some("medium") | Some("m") => Ecc::Medium,
        Some("quartile") | Some("q") => Ecc::Quartile,
        Some("high") | Some("h") => Ecc::High,
        Some(o) => return Err(format!("ecc: expected low|medium|quartile|high, got '{o}'")),
    };
    let bg = match s.bg.as_deref() {
        None => Some(Fill::Solid(Rgba::WHITE)),
        Some("transparent") | Some("none") => None,
        Some(h) => Some(Fill::Solid(hex(h)?)),
    };
    let eye_fill = match &s.eye_color {
        Some(h) => Fill::Solid(hex(h)?),
        None => fill.clone(),
    };
    let eye_inner = s.eye_inner_color.as_deref().map(|h| Fill::Solid(hex(h).unwrap())).map(Some).unwrap_or(None);
    let logo = match s.logo {
        Some(w) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(w.data_base64)
                .map_err(|e| format!("logo base64: {e}"))?;
            let mut l = LogoConfig::from_bytes(bytes).map_err(|e| e.to_string())?;
            if let Some(r) = w.size_ratio { l.size_ratio = r; }
            if let Some(p) = w.padding { l.padding = p; }
            Some(l)
        }
        None => None,
    };

    Ok(StyleConfig::new()
        .ecc(ecc)
        .version(if let Some(v) = s.version { Version::Fixed(v) } else { Version::Auto })
        .margin_modules(s.margin.unwrap_or(4))
        .shape(shape)
        .eye_shape(eye)
        .fill(fill)
        .eye_fill(eye_fill)
        .eye_inner_fill(eye_inner)
        .background(bg)
        .logo(logo))
}
```

- [ ] **Step 4: Run tests and wasm build**

Run: `cargo test -p qrkit-wasm && wasm-pack build crates/qrkit-wasm --target web --out-dir ../../web/pkg`
Expected: 5 tests pass; wasm-pack succeeds (install with `cargo install wasm-pack` if missing) and creates `web/pkg/`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/qrkit-wasm web/pkg/.gitkeep
git commit -m "feat: add WASM binding with JSON config generation"
```

(`web/pkg` is gitignored; the commit covers the crate only — drop `web/pkg/.gitkeep` if you don't create it.)

---

### Task 15: Web app

**Files:**
- Create: `web/index.html`, `web/app.js`, `web/style.css`

**Interfaces:**
- Consumes: `web/pkg/qrkit_wasm.js` (the `generate` export from Task 14).
- Produces: a static single-page app with live preview and SVG/PDF/EPS downloads.

- [ ] **Step 1: Write index.html**

`web/index.html`:

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>qrkit — vector QR generator</title>
<link rel="stylesheet" href="style.css"/>
</head>
<body>
<header><h1>qrkit</h1><p>Customizable QR codes as true vector graphics</p></header>
<main>
  <section id="panel">
    <label>Content type
      <select id="type"></select>
    </label>
    <div id="content-form"></div>
    <fieldset>
      <legend>Style</legend>
      <div class="row">
        <label>Foreground <input type="color" id="fg" value="#000000"/></label>
        <label>Background <input type="color" id="bg" value="#ffffff"/></label>
        <label class="check"><input type="checkbox" id="bg-transparent"/> transparent</label>
      </div>
      <div class="row">
        <label>Gradient
          <select id="gradient"><option value="">none</option><option>linear</option><option>radial</option></select>
        </label>
        <label>To <input type="color" id="gradient-to" value="#42a5f5"/></label>
        <label>Angle <input type="number" id="gradient-angle" value="45" min="0" max="359"/></label>
      </div>
      <div class="row">
        <label>Shape
          <select id="shape"><option>square</option><option>rounded</option><option>dots</option><option>classy</option></select>
        </label>
        <label>Dot radius <input type="range" id="dot-radius" min="0.1" max="0.5" step="0.05" value="0.5"/></label>
        <label>Eye
          <select id="eye"><option>square</option><option>rounded</option><option>circle</option><option>leaf</option></select>
        </label>
      </div>
      <div class="row">
        <label>Eye color <input type="color" id="eye-color" value="#000000"/></label>
        <label>ECC
          <select id="ecc"><option>quartile</option><option>low</option><option>medium</option><option>high</option></select>
        </label>
        <label>Margin <input type="number" id="margin" value="4" min="0" max="16"/></label>
      </div>
      <div class="row">
        <label>Logo <input type="file" id="logo" accept="image/png,image/jpeg"/></label>
        <label>Logo size <input type="range" id="logo-size" min="0.1" max="0.33" step="0.01" value="0.25"/></label>
      </div>
    </fieldset>
    <div id="warnings"></div>
  </section>
  <section id="preview">
    <div id="qr"></div>
    <div id="downloads">
      <button data-fmt="svg">Download SVG</button>
      <button data-fmt="pdf">Download PDF</button>
      <button data-fmt="eps">Download EPS</button>
    </div>
  </section>
</main>
<script type="module" src="app.js"></script>
</body>
</html>
```

- [ ] **Step 2: Write app.js**

`web/app.js`:

```js
import init, { generate } from "./pkg/qrkit_wasm.js";

const TYPE_FIELDS = {
  text:   [{ name: "text", label: "Text", type: "textarea", value: "Hello, vector world" }],
  url:    [{ name: "url", label: "URL", type: "text", value: "example.com" }],
  email:  [{ name: "to", label: "To", type: "email", value: "" },
           { name: "subject", label: "Subject", type: "text", value: "" },
           { name: "body", label: "Body", type: "textarea", value: "" }],
  sms:    [{ name: "number", label: "Number", type: "text", value: "" },
           { name: "message", label: "Message", type: "text", value: "" }],
  phone:  [{ name: "number", label: "Number", type: "text", value: "" }],
  wifi:   [{ name: "ssid", label: "SSID", type: "text", value: "" },
           { name: "password", label: "Password", type: "password", value: "" },
           { name: "encryption", label: "Encryption", type: "select", options: ["wpa", "wep", "nopass"], value: "wpa" },
           { name: "hidden", label: "Hidden network", type: "checkbox", value: false }],
  vcard:  [{ name: "name", label: "Full name", type: "text", value: "" },
           { name: "org", label: "Organization", type: "text", value: "" },
           { name: "title", label: "Title", type: "text", value: "" },
           { name: "phones", label: "Phones (comma-sep)", type: "text", value: "" },
           { name: "emails", label: "Emails (comma-sep)", type: "text", value: "" },
           { name: "url", label: "URL", type: "text", value: "" }],
  geo:    [{ name: "lat", label: "Latitude", type: "number", value: "51.5007" },
           { name: "lon", label: "Longitude", type: "number", value: "-0.1246" }],
  calendar: [{ name: "title", label: "Title", type: "text", value: "" },
             { name: "start_utc", label: "Start (YYYYMMDDTHHMMSSZ)", type: "text", value: "" },
             { name: "end_utc", label: "End (YYYYMMDDTHHMMSSZ)", type: "text", value: "" },
             { name: "location", label: "Location", type: "text", value: "" },
             { name: "description", label: "Description", type: "text", value: "" }],
};

const $ = (id) => document.getElementById(id);
let logoBase64 = null;

function renderTypeSelect() {
  $("type").innerHTML = Object.keys(TYPE_FIELDS)
    .map((t) => `<option value="${t}">${t}</option>`).join("");
}

function renderContentForm(type) {
  const fields = TYPE_FIELDS[type].map((f) => {
    if (f.type === "textarea") {
      return `<label>${f.label}<textarea data-field="${f.name}">${f.value}</textarea></label>`;
    }
    if (f.type === "select") {
      return `<label>${f.label}<select data-field="${f.name}">${
        f.options.map((o) => `<option${o === f.value ? " selected" : ""}>${o}</option>`).join("")
      }</select></label>`;
    }
    if (f.type === "checkbox") {
      return `<label class="check"><input type="checkbox" data-field="${f.name}" ${f.value ? "checked" : ""}/> ${f.label}</label>`;
    }
    return `<label>${f.label}<input type="${f.type}" data-field="${f.name}" value="${f.value}"/></label>`;
  }).join("");
  $("content-form").innerHTML = fields;
  $("content-form").querySelectorAll("[data-field]").forEach((el) => {
    el.addEventListener("input", update);
  });
}

function collectContent() {
  const type = $("type").value;
  const content = { type };
  $("content-form").querySelectorAll("[data-field]").forEach((el) => {
    let v = el.type === "checkbox" ? el.checked : el.value.trim();
    if (el.type === "number") v = parseFloat(v);
    if (typeof v === "string" && v === "" && !["text", "url"].includes(el.dataset.field)) return;
    content[el.dataset.field] = v;
  });
  if (type === "vcard") {
    content.phones = (content.phones || "").split(",").map((s) => s.trim()).filter(Boolean);
    content.emails = (content.emails || "").split(",").map((s) => s.trim()).filter(Boolean);
  }
  return content;
}

function collectStyle() {
  const style = {
    fg: $("fg").value,
    bg: $("bg-transparent").checked ? "transparent" : $("bg").value,
    gradient: $("gradient").value || undefined,
    gradientTo: $("gradient").value ? $("gradient-to").value : undefined,
    gradientAngle: parseFloat($("gradient-angle").value),
    shape: $("shape").value,
    dotRadius: parseFloat($("dot-radius").value),
    eye: $("eye").value,
    eyeColor: $("eye-color").value,
    ecc: $("ecc").value,
    margin: parseInt($("margin").value, 10),
  };
  if (logoBase64) {
    style.logo = { dataBase64: logoBase64, sizeRatio: parseFloat($("logo-size").value) };
  }
  return style;
}

function update() {
  const config = { content: collectContent(), style: collectStyle() };
  let out;
  try {
    out = generate(JSON.stringify(config), "svg");
  } catch (e) {
    $("warnings").innerHTML = `<p class="error">${e}</p>`;
    return;
  }
  $("qr").innerHTML = out.svg || "";
  $("warnings").innerHTML = (out.warnings || []).map((w) => `<p class="warn">${w}</p>`).join("");
}

function download(fmt) {
  const config = { content: collectContent(), style: collectStyle() };
  const out = generate(JSON.stringify(config), fmt);
  const bytes = fmt === "svg"
    ? new TextEncoder().encode(out.svg)
    : new Uint8Array(out[fmt]);
  const blob = new Blob([bytes], { type: { svg: "image/svg+xml", pdf: "application/pdf", eps: "application/postscript" }[fmt] });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `qr.${fmt}`;
  a.click();
  URL.revokeObjectURL(a.href);
}

async function main() {
  await init();
  renderTypeSelect();
  renderContentForm("text");
  $("type").addEventListener("change", (e) => renderContentForm(e.target.value));
  document.querySelectorAll("#panel input, #panel select, #panel textarea")
    .forEach((el) => {
      if (el.id === "logo") return;
      el.addEventListener("input", update);
    });
  $("logo").addEventListener("change", async (e) => {
    const file = e.target.files[0];
    if (!file) { logoBase64 = null; update(); return; }
    const dataUrl = await new Promise((res) => {
      const r = new FileReader();
      r.onload = () => res(r.result);
      r.readAsDataURL(file);
    });
    logoBase64 = dataUrl.split(",")[1];
    update();
  });
  document.querySelectorAll("#downloads button").forEach((b) =>
    b.addEventListener("click", () => download(b.dataset.fmt)));
  update();
}

main();
```

- [ ] **Step 3: Write style.css**

`web/style.css`:

```css
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body { margin: 0; font-family: system-ui, sans-serif; }
header { padding: 1rem 2rem; border-bottom: 1px solid color-mix(in srgb, currentColor 15%, transparent); }
header h1 { margin: 0; font-size: 1.4rem; }
header p { margin: 0.2rem 0 0; opacity: 0.7; }
main { display: grid; grid-template-columns: minmax(320px, 480px) 1fr; gap: 2rem; padding: 2rem; }
#panel label { display: block; margin: 0.5rem 0; font-size: 0.9rem; }
#panel input:not([type="checkbox"]):not([type="file"]),
#panel select, #panel textarea { width: 100%; padding: 0.4rem; margin-top: 0.2rem; }
.row { display: flex; gap: 0.8rem; }
.row label { flex: 1; }
.check { display: flex; align-items: center; gap: 0.4rem; }
fieldset { margin-top: 1rem; border-radius: 8px; }
#qr svg { width: min(100%, 520px); height: auto; border-radius: 8px; }
#warnings .warn { color: #b45309; }
#warnings .error { color: #b91c1c; }
#downloads { margin-top: 1rem; display: flex; gap: 0.5rem; }
button { padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; }
@media (max-width: 800px) { main { grid-template-columns: 1fr; } }
```

- [ ] **Step 4: Manual verification**

Run: `just build-wasm && just serve` then open [http://127.0.0.1:8080](http://127.0.0.1:8080).
Expected: preview renders for every content type; switching shapes/eyes updates live; gradient toggling works; logo upload embeds and warns about ECC; all three downloads produce files that open correctly (SVG in browser, PDF in Preview, EPS in Preview/Inkscape). Also resize the browser — the SVG scales losslessly.

- [ ] **Step 5: Commit**

```bash
git add web/index.html web/app.js web/style.css
git commit -m "feat: add static web app with live preview and vector downloads"
```

---

### Task 16: README and final verification

**Files:**
- Create: `README.md`
- Modify: none (verification only)

**Interfaces:** none — documentation and final gate.

- [ ] **Step 1: Write README.md**

`README.md`:

````markdown
# qrkit

Fully customizable QR code generator producing true vector graphics (SVG,
PDF, EPS) — resize and restyle losslessly. Rust core library, CLI, and a
client-side WASM web app.

## Quick start (CLI)

```console
$ cargo install --path crates/qrkit-cli

$ qrkit url "https://example.com/promo" \
    --shape dots --fg "#0d47a1" --gradient linear --gradient-to "#42a5f5" --gradient-angle 45 \
    --eye circle --logo assets/logo.png --logo-size 0.25 \
    -o poster.svg

$ qrkit wifi --ssid "Cafe Guest" --password "latte-2026" --wpa -o menu.pdf
$ qrkit vcard --name "Ada Lovelace" --email ada@example.com -o card.svg
$ qrkit geo --lat 51.5007 --lon -0.1246 -o spot.eps
$ qrkit text "hello" --preview        # Unicode preview on stderr
```

Output format is inferred from the extension (`svg|pdf|eps`), or forced with
`--format`. Without `-o`, SVG goes to stdout.

## Content types

`text`, `url`, `email`, `sms`, `phone`, `wifi`, `vcard`, `geo`, `calendar` —
each subcommand validates and formats the standards-compliant payload
(`WIFI:`, vCard 3.0, VEVENT, `mailto:`, `tel:`, `SMSTO:`, `geo:`).

## Customization

- ECC L/M/Q/H (a logo forces H), fixed version 1–40 or auto, quiet-zone margin
- Module shapes: square, rounded, dots (adjustable radius), classy
- Eye shapes: square, rounded, circle, leaf — with separate ring and dot colors
- Solid colors, linear gradients (any angle), radial gradients; transparent background
- Logo embedding (PNG/JPEG) with module clearing under the logo area

## Web app

```console
$ cargo install wasm-pack
$ just build-wasm
$ just serve          # http://127.0.0.1:8080
```

Everything runs client-side; nothing leaves the browser.

## Library

```rust
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::style::{ModuleShape, StyleConfig};

let svg = QrBuilder::new(Content::Text("hello".into()))?
    .style(StyleConfig::new().shape(ModuleShape::Dots { radius: 0.5 }))
    .to_svg()?;
```

## Notes

- EPS has no alpha: colors are pre-blended against the background (white when
  transparent), and gradients use PostScript Level 3 `shfill`.
- PDFs embed PNG logos as RGB + alpha SMask; JPEGs pass through as DCTDecode.
- The scene-graph is shared by all three writers, so geometry stays identical
  across formats.
````

- [ ] **Step 2: Full verification gate**

Run: `just check`
Expected: fmt clean, clippy clean (`-D warnings`), full test suite green (content goldens, writer structural tests, and the 40-combination decode matrix).

Run: `cargo run -p qrkit-cli -- url "https://github.com" --shape classy --eye leaf --gradient linear --gradient-to "#ff5722" -o /tmp/final.svg && open /tmp/final.svg`
Expected: stylish gradient QR opens in the browser and scans with a phone. Delete `/tmp/final.svg` after.

- [ ] **Step 3: Commit and open PR**

```bash
git add README.md
git commit -m "docs: add README with CLI, library, and web app usage"
git log --oneline
```

Then push the branch and open a PR against `main` (use the user's normal PR flow; no AI attribution anywhere).

---

## Self-Review (completed during planning)

- **Spec coverage:** 9 content types (Tasks 2–3), full styling incl. gradients/shapes/eyes/logo (Tasks 5–7), three writers (8/10/11), logo ECC forcing + warnings (6, 9), CLI with preview + format inference (13), client-side WASM web app (14–15), testing incl. golden payloads, scene round-trip, and decode matrix (2–3, 9, 12), EPS alpha-blending limitation (11), justfile (1, 15).
- **Deviations from spec, both benign:** `serde` is a `qrkit` runtime dep (spec listed it under wasm) because the WASM config reuses library types' enums; `insta` is used for payload/VEVENT/vCard goldens and plain asserts elsewhere.
- **Type consistency:** `render::to_path` and `writers::common::linear_endpoints` are used by all three writers and the test rasterizer (made `pub` + `#[doc(hidden)]` in Task 9); `LogoConfig` field names (`size_ratio`, `padding`) are stable across Tasks 5/6/13/14.
