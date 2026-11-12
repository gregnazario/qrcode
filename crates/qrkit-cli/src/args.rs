//! CLI argument definitions and mapping to library types.

use clap::{Parser, Subcommand};
use qrkit::content::{CalendarEvent, Content, Encryption, VCard};
use qrkit::logo::LogoConfig;
use qrkit::scene::{Fill, Rgba};
use qrkit::style::{Ecc, EyeShape, ModuleShape, StyleConfig, Version};
use qrkit::Error;
use std::path::PathBuf;

#[derive(Parser, Debug)]
// `--version` is the QR symbol version, so clap's auto `--version` flag is
// disabled to avoid the name collision.
#[command(
    name = "qrkit",
    about = "Customizable QR codes as SVG, PDF, and EPS",
    disable_version_flag = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output file; format inferred from extension (svg|pdf|eps|term).
    /// Omit to write SVG to stdout.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Force output format: svg|pdf|eps|term|png|kitty|iterm|sixel|auto
    /// (overrides extension). term = ANSI half-blocks; kitty/iterm/sixel are
    /// terminal image protocols; auto detects the terminal (image protocol
    /// where available, else term half-blocks).
    #[arg(long, global = true)]
    pub format: Option<String>,

    /// Pixels per module for raster formats (png/kitty/iterm/sixel; default 24).
    #[arg(long, global = true, default_value_t = 24,
          value_parser = clap::value_parser!(u32).range(1..=256))]
    pub px: u32,

    /// Error correction: low|medium|quartile|high (default quartile).
    #[arg(long, global = true)]
    pub ecc: Option<String>,

    /// Fixed QR version 1..=40 (default: auto).
    #[arg(long, global = true)]
    pub version: Option<u8>,

    /// Quiet-zone margin in modules (default 4).
    #[arg(long, default_value_t = 4, global = true)]
    pub margin: u32,

    /// Module shape: square|rounded|dots|classy (default square).
    #[arg(long, global = true)]
    pub shape: Option<String>,

    /// Dot radius for --shape dots (default 0.5).
    #[arg(long, default_value_t = 0.5, global = true)]
    pub dot_radius: f32,

    /// Eye shape: square|rounded|circle|leaf (default square).
    #[arg(long, global = true)]
    pub eye: Option<String>,

    /// Foreground / module color, hex (default #000000).
    #[arg(long, global = true)]
    pub fg: Option<String>,

    /// Background color, hex or "transparent" (default #ffffff).
    #[arg(long, global = true)]
    pub bg: Option<String>,

    /// Gradient kind: linear|radial (default linear when --gradient-to is given).
    /// Enables gradient fill from --fg to --gradient-to.
    #[arg(long, global = true)]
    pub gradient: Option<String>,

    /// Gradient end color, hex.
    #[arg(long, global = true)]
    pub gradient_to: Option<String>,

    /// Linear gradient angle in degrees (default 0).
    #[arg(long, default_value_t = 0.0, global = true)]
    pub gradient_angle: f32,

    /// Eye ring color, hex (default: same as --fg).
    #[arg(long, global = true)]
    pub eye_color: Option<String>,

    /// Eye inner dot color, hex (default: module fill).
    #[arg(long, global = true)]
    pub eye_inner_color: Option<String>,

    /// Pure-shape eyes (bigger radii, no scanner-safety tabs/nubs).
    /// Cleaner look, but strict readers may fail to decode the result.
    #[arg(long, global = true)]
    pub decorative_eyes: bool,

    /// PNG/JPEG logo file path.
    #[arg(long, global = true)]
    pub logo: Option<PathBuf>,

    /// Logo size as fraction of code size, 0.1..=0.33 (default 0.25).
    #[arg(long, global = true)]
    pub logo_size: Option<f32>,

    /// Logo padding in modules (default 1.0).
    #[arg(long, global = true)]
    pub logo_padding: Option<f32>,

    /// Render a Unicode preview on stderr.
    #[arg(long, global = true)]
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
        #[arg(long)]
        to: String,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long)]
        body: Option<String>,
    },
    /// SMS message
    Sms {
        #[arg(long)]
        number: String,
        #[arg(long)]
        message: Option<String>,
    },
    /// Phone number
    Phone { number: String },
    /// WiFi network login
    Wifi {
        #[arg(long)]
        ssid: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        encryption: Option<String>,
        #[arg(long)]
        hidden: bool,
    },
    /// vCard contact
    Vcard {
        #[arg(long)]
        name: String,
        #[arg(long)]
        org: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        phone: Vec<String>,
        #[arg(long)]
        email: Vec<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        note: Option<String>,
    },
    /// Geo location
    Geo {
        #[arg(long, allow_hyphen_values = true)]
        lat: f64,
        #[arg(long, allow_hyphen_values = true)]
        lon: f64,
    },
    /// Calendar event (UTC timestamps YYYYMMDDTHHMMSSZ)
    Calendar {
        #[arg(long)]
        title: String,
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long)]
        location: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
}

pub fn content_from_args(cli: &Cli) -> Result<Content, Error> {
    Ok(match &cli.command {
        Commands::Text { text } => Content::Text(text.clone()),
        Commands::Url { url } => {
            let normalized = if url.contains("://") {
                url.clone()
            } else {
                format!("https://{url}")
            };
            Content::Url(normalized)
        }
        Commands::Email { to, subject, body } => Content::Email {
            to: to.clone(),
            subject: subject.clone(),
            body: body.clone(),
        },
        Commands::Sms { number, message } => Content::Sms {
            number: number.clone(),
            message: message.clone(),
        },
        Commands::Phone { number } => Content::Phone(number.clone()),
        Commands::Wifi {
            ssid,
            password,
            encryption,
            hidden,
        } => Content::Wifi {
            ssid: ssid.clone(),
            password: password.clone(),
            encryption: match encryption.as_deref() {
                None | Some("wpa") => Encryption::Wpa,
                Some("wep") => Encryption::Wep,
                Some("nopass") => Encryption::Nopass,
                Some(other) => {
                    return Err(Error::Content(format!(
                        "unknown encryption '{other}', expected wpa|wep|nopass"
                    )))
                }
            },
            hidden: *hidden,
        },
        Commands::Vcard {
            name,
            org,
            title,
            phone,
            email,
            url,
            address,
            note,
        } => Content::VCard(VCard {
            name: name.clone(),
            org: org.clone(),
            title: title.clone(),
            phones: phone.clone(),
            emails: email.clone(),
            url: url.clone(),
            address: address.clone(),
            note: note.clone(),
        }),
        Commands::Geo { lat, lon } => Content::Geo {
            lat: *lat,
            lon: *lon,
        },
        Commands::Calendar {
            title,
            start,
            end,
            location,
            description,
        } => {
            let ev = CalendarEvent {
                title: title.clone(),
                start_utc: start.clone(),
                end_utc: end.clone(),
                location: location.clone(),
                description: description.clone(),
            };
            qrkit::content::validate_utc_timestamp(&ev.start_utc)?;
            qrkit::content::validate_utc_timestamp(&ev.end_utc)?;
            Content::Calendar(ev)
        }
    })
}

pub fn style_from_args(cli: &Cli) -> Result<StyleConfig, Error> {
    let hex = |v: &Option<String>, what: &str| -> Result<Rgba, Error> {
        Rgba::from_hex(v.as_deref().unwrap_or("#000000"))
            .map_err(|_| Error::InvalidStyle(format!("invalid {what} color")))
    };
    let fg = hex(&cli.fg, "--fg")?;

    let fill = if let Some(to) = &cli.gradient_to {
        let to = Rgba::from_hex(to)
            .map_err(|_| Error::InvalidStyle("invalid --gradient-to color".into()))?;
        match cli.gradient.as_deref().unwrap_or("linear") {
            "linear" => Fill::LinearGradient {
                from: fg,
                to,
                angle_deg: cli.gradient_angle,
            },
            "radial" => Fill::RadialGradient { from: fg, to },
            other => {
                return Err(Error::InvalidStyle(format!(
                    "unknown gradient kind '{other}', expected linear|radial"
                )))
            }
        }
    } else if cli.gradient.is_some() {
        return Err(Error::InvalidStyle(
            "--gradient requires --gradient-to".into(),
        ));
    } else {
        Fill::Solid(fg)
    };

    let shape = match cli.shape.as_deref() {
        None | Some("square") => ModuleShape::Square,
        Some("rounded") => ModuleShape::Rounded,
        Some("dots") => ModuleShape::Dots {
            radius: cli.dot_radius,
        },
        Some("classy") => ModuleShape::Classy,
        Some(other) => {
            return Err(Error::InvalidStyle(format!(
                "unknown shape '{other}', expected square|rounded|dots|classy"
            )))
        }
    };
    let eye_shape = match cli.eye.as_deref() {
        None | Some("square") => EyeShape::Square,
        Some("rounded") => EyeShape::Rounded,
        Some("circle") => EyeShape::Circle,
        Some("leaf") => EyeShape::Leaf,
        Some(other) => {
            return Err(Error::InvalidStyle(format!(
                "unknown eye shape '{other}', expected square|rounded|circle|leaf"
            )))
        }
    };
    let ecc = match cli.ecc.as_deref() {
        None => Ecc::Quartile,
        Some("low") => Ecc::Low,
        Some("medium") => Ecc::Medium,
        Some("quartile") => Ecc::Quartile,
        Some("high") => Ecc::High,
        Some(other) => {
            return Err(Error::InvalidStyle(format!(
                "unknown ecc '{other}', expected low|medium|quartile|high"
            )))
        }
    };
    let background = match cli.bg.as_deref() {
        None => Some(Fill::Solid(Rgba::WHITE)),
        Some("transparent") | Some("none") => None,
        Some(hexstr) => {
            Some(Fill::Solid(Rgba::from_hex(hexstr).map_err(|_| {
                Error::InvalidStyle("invalid --bg color".into())
            })?))
        }
    };
    let eye_fill = match &cli.eye_color {
        Some(c) => Fill::Solid(
            Rgba::from_hex(c)
                .map_err(|_| Error::InvalidStyle("invalid --eye-color color".into()))?,
        ),
        None => fill.clone(),
    };
    let eye_inner_fill = match &cli.eye_inner_color {
        Some(c) => Some(Fill::Solid(Rgba::from_hex(c).map_err(|_| {
            Error::InvalidStyle("invalid --eye-inner-color color".into())
        })?)),
        None => None,
    };
    let logo = match &cli.logo {
        Some(path) => Some(
            LogoConfig::from_bytes(
                std::fs::read(path)
                    .map_err(|e| Error::Content(format!("cannot read logo {path:?}: {e}")))?,
            )
            .map_err(|e| Error::Content(format!("logo {path:?}: {e}")))?,
        ),
        None => None,
    };
    let mut style = StyleConfig::new()
        .ecc(ecc)
        .version(if let Some(v) = cli.version {
            Version::Fixed(v)
        } else {
            Version::Auto
        })
        .margin_modules(cli.margin)
        .shape(shape)
        .eye_shape(eye_shape)
        .fill(fill)
        .eye_fill(eye_fill)
        .eye_inner_fill(eye_inner_fill)
        .background(background)
        .decorative_eyes(cli.decorative_eyes)
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
/// `auto` resolves to a concrete terminal format via env detection.
pub fn output_format(cli: &Cli) -> Result<&'static str, Error> {
    if let Some(f) = &cli.format {
        let resolved = match f.as_str() {
            "auto" => match qrkit::writers::term_auto::detect() {
                qrkit::writers::term_auto::TermImage::Kitty => "kitty",
                qrkit::writers::term_auto::TermImage::Iterm => "iterm",
                qrkit::writers::term_auto::TermImage::AnsiBlocks => "term",
            },
            other => other,
        };
        return match resolved {
            "svg" => Ok("svg"),
            "pdf" => Ok("pdf"),
            "eps" => Ok("eps"),
            "term" | "ansi" | "txt" => Ok("term"),
            "png" => Ok("png"),
            "kitty" => Ok("kitty"),
            "iterm" => Ok("iterm"),
            "sixel" => Ok("sixel"),
            other => Err(Error::InvalidStyle(format!(
                "unknown format '{other}', expected svg|pdf|eps|term|png|kitty|iterm|sixel|auto"
            ))),
        };
    }
    if let Some(out) = &cli.output {
        return match out.extension().and_then(|e| e.to_str()) {
            Some("svg") => Ok("svg"),
            Some("pdf") => Ok("pdf"),
            Some("eps") => Ok("eps"),
            Some("term") | Some("ansi") | Some("txt") => Ok("term"),
            Some("png") => Ok("png"),
            Some("six") | Some("sixel") => Ok("sixel"),
            _ => Err(Error::InvalidStyle(
                "cannot infer format from extension; use --format svg|pdf|eps|term|png|kitty|iterm|sixel|auto"
                    .into(),
            )),
        };
    }
    Ok("svg")
}
