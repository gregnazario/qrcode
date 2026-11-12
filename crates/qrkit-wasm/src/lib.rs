//! WASM binding: one `generate(config_json, format)` entry point.
//!
//! The config mirrors the library types with camelCase field names; see
//! tests/config.rs for the exact JSON shape.

use base64::Engine as _;
use wasm_bindgen::prelude::*;

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WContent {
    Text {
        text: String,
    },
    Url {
        url: String,
    },
    Email {
        to: String,
        subject: Option<String>,
        body: Option<String>,
    },
    Sms {
        number: String,
        message: Option<String>,
    },
    Phone {
        number: String,
    },
    Wifi {
        ssid: String,
        password: Option<String>,
        encryption: Option<String>,
        hidden: Option<bool>,
    },
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
    Geo {
        lat: f64,
        lon: f64,
    },
    Calendar {
        title: String,
        start_utc: String,
        end_utc: String,
        location: Option<String>,
        description: Option<String>,
    },
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
    decorative_eyes: Option<bool>,
    logo: Option<WLogo>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WConfig {
    content: WContent,
    style: Option<WStyle>,
}

/// Pure conversion + generation, testable without a browser.
pub fn render_to_bytes(
    config_json: &str,
    format: &str,
) -> Result<(Option<Vec<u8>>, Vec<String>), String> {
    let cfg: WConfig = serde_json::from_str(config_json).map_err(|e| format!("config: {e}"))?;
    let content = to_content(cfg.content)?;
    let style = cfg.style.map(to_style).transpose()?.unwrap_or_default();
    let builder = qrkit::builder::QrBuilder::new(content)
        .map_err(|e| e.to_string())?
        .style(style);
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
        self.svg
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
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
        self.warnings.iter().map(|w| JsValue::from_str(w)).collect()
    }
}

#[wasm_bindgen]
pub fn generate(config_json: &str, format: &str) -> Result<Generated, JsError> {
    let (bytes, warnings) = render_to_bytes(config_json, format).map_err(|e| JsError::new(&e))?;
    let mut out = Generated {
        svg: None,
        pdf: None,
        eps: None,
        warnings,
    };
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
            // CLI parity: prepend https:// when the scheme is missing. A
            // scheme-less string whose host has no dot (e.g. "nope") is not
            // domain-like, so it passes through raw and the library's URL
            // validation reports it (tests/config.rs requires the error).
            let normalized = if url.contains("://") {
                url
            } else {
                let host = url.split(['/', '?', '#']).next().unwrap_or("");
                if host.contains('.') {
                    format!("https://{url}")
                } else {
                    url
                }
            };
            Content::Url(normalized)
        }
        WContent::Email { to, subject, body } => Content::Email { to, subject, body },
        WContent::Sms { number, message } => Content::Sms { number, message },
        WContent::Phone { number } => Content::Phone(number),
        WContent::Wifi {
            ssid,
            password,
            encryption,
            hidden,
        } => Content::Wifi {
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
        WContent::Vcard {
            name,
            org,
            title,
            phones,
            emails,
            url,
            address,
            note,
        } => Content::VCard(VCard {
            name,
            org,
            title,
            phones: phones.unwrap_or_default(),
            emails: emails.unwrap_or_default(),
            url,
            address,
            note,
        }),
        WContent::Geo { lat, lon } => Content::Geo { lat, lon },
        WContent::Calendar {
            title,
            start_utc,
            end_utc,
            location,
            description,
        } => {
            let ev = CalendarEvent {
                title,
                start_utc,
                end_utc,
                location,
                description,
            };
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
                "linear" => Fill::LinearGradient {
                    from: fg,
                    to,
                    angle_deg: s.gradient_angle.unwrap_or(0.0),
                },
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
        Some("dots") => ModuleShape::Dots {
            radius: s.dot_radius.unwrap_or(0.5),
        },
        Some("classy") => ModuleShape::Classy,
        Some(o) => {
            return Err(format!(
                "shape: expected square|rounded|dots|classy, got '{o}'"
            ))
        }
    };
    let eye = match s.eye.as_deref() {
        None | Some("square") => EyeShape::Square,
        Some("rounded") => EyeShape::Rounded,
        Some("circle") => EyeShape::Circle,
        Some("leaf") => EyeShape::Leaf,
        Some(o) => {
            return Err(format!(
                "eye: expected square|rounded|circle|leaf, got '{o}'"
            ))
        }
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
    let eye_inner = match s.eye_inner_color.as_deref() {
        None => None,
        Some(h) => Some(Fill::Solid(hex(h)?)),
    };
    let logo = match s.logo {
        Some(w) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(w.data_base64)
                .map_err(|e| format!("logo base64: {e}"))?;
            let mut l = LogoConfig::from_bytes(bytes).map_err(|e| e.to_string())?;
            if let Some(r) = w.size_ratio {
                l.size_ratio = r;
            }
            if let Some(p) = w.padding {
                l.padding = p;
            }
            Some(l)
        }
        None => None,
    };

    Ok(StyleConfig::new()
        .ecc(ecc)
        .version(if let Some(v) = s.version {
            Version::Fixed(v)
        } else {
            Version::Auto
        })
        .margin_modules(s.margin.unwrap_or(4))
        .shape(shape)
        .eye_shape(eye)
        .fill(fill)
        .eye_fill(eye_fill)
        .eye_inner_fill(eye_inner)
        .background(bg)
        .decorative_eyes(s.decorative_eyes.unwrap_or(false))
        .logo(logo))
}
