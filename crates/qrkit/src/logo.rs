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
                u32::from_be_bytes([
                    data[r.start],
                    data[r.start + 1],
                    data[r.start + 2],
                    data[r.start + 3],
                ])
            };
            Ok((be(16..20), be(20..24)))
        }
        ImageFormat::Jpeg => {
            let mut dec = zune_jpeg::JpegDecoder::new(data);
            dec.decode_headers()
                .map_err(|e| Error::LogoDecode(e.to_string()))?;
            let (w, h) = dec
                .dimensions()
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
            let mut reader = decoder
                .read_info()
                .map_err(|e| Error::LogoDecode(e.to_string()))?;
            let mut buf = vec![0u8; reader.output_buffer_size()];
            let info = reader
                .next_frame(&mut buf)
                .map_err(|e| Error::LogoDecode(e.to_string()))?;
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
            dec.decode_headers()
                .map_err(|e| Error::LogoDecode(e.to_string()))?;
            let (w, h) = dec
                .dimensions()
                .ok_or_else(|| Error::LogoDecode("jpeg dimensions unavailable".into()))?;
            let raw = dec.decode().map_err(|e| Error::LogoDecode(e.to_string()))?;
            let rgba = match raw.len() / (w * h).max(1) {
                4 => raw,
                3 => to_rgba(&raw, 3),
                1 => to_rgba(&raw, 1),
                n => {
                    return Err(Error::LogoDecode(format!(
                        "unexpected jpeg channel count {n}"
                    )))
                }
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
