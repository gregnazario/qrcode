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
    pub fn to_qrcodegen(self) -> qrcodegen::QrCodeEcc {
        match self {
            Ecc::Low => qrcodegen::QrCodeEcc::Low,
            Ecc::Medium => qrcodegen::QrCodeEcc::Medium,
            Ecc::Quartile => qrcodegen::QrCodeEcc::Quartile,
            Ecc::High => qrcodegen::QrCodeEcc::High,
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
    Dots {
        radius: f32,
    },
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

    pub fn ecc(mut self, ecc: Ecc) -> Self {
        self.ecc = ecc;
        self
    }
    pub fn version(mut self, v: Version) -> Self {
        self.version = v;
        self
    }
    pub fn margin_modules(mut self, m: u32) -> Self {
        self.margin_modules = m;
        self
    }
    pub fn shape(mut self, s: ModuleShape) -> Self {
        self.shape = s;
        self
    }
    pub fn eye_shape(mut self, s: EyeShape) -> Self {
        self.eye_shape = s;
        self
    }
    pub fn fill(mut self, f: Fill) -> Self {
        self.fill = f;
        self
    }
    pub fn eye_fill(mut self, f: Fill) -> Self {
        self.eye_fill = f;
        self
    }
    pub fn eye_inner_fill(mut self, f: Option<Fill>) -> Self {
        self.eye_inner_fill = f;
        self
    }
    pub fn background(mut self, f: Option<Fill>) -> Self {
        self.background = f;
        self
    }
    pub fn logo(mut self, l: Option<LogoConfig>) -> Self {
        self.logo = l;
        self
    }

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
                warnings.push(format!(
                    "logo size ratio {} out of range 0.1..=0.33",
                    logo.size_ratio
                ));
            }
            if self.ecc != Ecc::High {
                warnings.push(
                    "logo present: ECC forced to High for scannability (use .ecc(Ecc::High) to silence this)"
                        .to_string(),
                );
            }
        }
        warnings
    }
}
