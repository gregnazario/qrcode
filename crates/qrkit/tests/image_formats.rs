mod common;

use base64::Engine as _;
use qrkit::builder::QrBuilder;
use qrkit::content::Content;
use qrkit::writers::png as png_writer;
use qrkit::writers::{iterm, kitty, raster, sixel, term_auto::TermImage};

const PAYLOAD: &str = "https://example.com/images";

fn scene() -> qrkit::builder::QrImage {
    QrBuilder::new(Content::Url(PAYLOAD.into()))
        .unwrap()
        .build()
        .unwrap()
}

fn png_from(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let dec = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = dec.read_info().unwrap();
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    (info.width, info.height, buf[..info.buffer_size()].to_vec())
}

#[test]
fn png_decodes_via_rqrr() {
    let bytes = png_writer::write(&scene().scene, 24);
    assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n");
    let (w, h, rgba) = png_from(&bytes);
    assert_eq!(w, h);
    // decode by luminance
    let gray: Vec<u8> = rgba
        .chunks(4)
        .map(|p| (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8)
        .collect();
    let mut img = rqrr::PreparedImage::prepare_from_greyscale(w as usize, h as usize, |x, y| {
        gray[y * w as usize + x]
    });
    let mut grids = img.detect_grids();
    let (_m, content) = grids.pop().expect("detect").decode().expect("decode");
    assert_eq!(content, PAYLOAD);
}

#[test]
fn kitty_payload_is_chunked_png() {
    let bytes = kitty::write(&scene().scene, 8); // >1 chunk at this size
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        s.starts_with("\x1b_Ga=d,d=i,i=31,q=2\x1b\\"),
        "delete-first framing (single, response-suppressed)"
    );
    // concatenate all payload segments after ';'
    let mut b64 = String::new();
    for chunk in s.split("\x1b\\").filter(|c| c.starts_with("\x1b_G")) {
        if let Some((_, data)) = chunk.split_once(';') {
            b64.push_str(data);
        }
    }
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .expect("payload is base64");
    assert_eq!(&png_bytes[0..8], b"\x89PNG\r\n\x1a\n", "payload is a PNG");
    // every full chunk is ≤ 4096 bytes of base64, and the transmission
    // terminates: exactly one chunk carries m=0 and it is the last.
    let chunks: Vec<&str> = s.split("\x1b\\").filter(|c| c.contains(';')).collect();
    assert!(chunks.len() > 1, "this test wants the multi-chunk path");
    for chunk in &chunks {
        if let Some((_, data)) = chunk.split_once(';') {
            assert!(data.len() <= 4096, "chunk {} > 4096", data.len());
        }
    }
    let terms = chunks.iter().filter(|c| c.contains("m=0")).count();
    assert_eq!(terms, 1, "exactly one terminating m=0 chunk");
    assert!(chunks.last().unwrap().contains("m=0"), "terminator is last");
}

#[test]
fn iterm_payload_is_inline_osc1337() {
    let bytes = iterm::write(&scene().scene, 8);
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(s.starts_with("\x1b]1337;File=name=qr.png;size="));
    assert!(s.contains(";inline=1:"));
    assert!(s.ends_with("\x1b\\\n"));
    let payload = s
        .split_once("inline=1:")
        .unwrap()
        .1
        .split("\x1b\\")
        .next()
        .unwrap();
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .expect("payload is base64");
    assert_eq!(&png_bytes[0..8], b"\x89PNG\r\n\x1a\n");
}

/// Minimal sixel decoder for OUR encoder output: parses the `"1;1;W;H`
/// header, `#id;2;R;G;B` color defs, and band data (`#id` selects, `$` CR,
/// `-` newline, `!n` RLE, 0x3f-based chars) back into an RGBA grid.
fn decode_sixel(bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    let body = s
        .strip_prefix("\x1bPq")
        .and_then(|b| b.strip_suffix("\x1b\\"))
        .expect("DCS framing");
    let hash = body.find('#').expect("first color def");
    let (header, rest) = (&body[..hash], &body[hash..]); // rest KEEPS the '#'
    let dims: Vec<usize> = header
        .trim_matches('"')
        .split(';')
        .map(|v| v.parse().unwrap())
        .collect();
    let (w, h) = (dims[2], dims[3]);

    // First pass: color definitions (`#id;2;R;G;B`, 0..100 scale). The
    // separator between the id and the color space `2` is a ';'.
    let mut colors: Vec<(u8, u8, u8)> = Vec::new();
    let mut scan = rest.chars().peekable();
    while let Some(c) = scan.next() {
        if c != '#' {
            continue;
        }
        let mut id = String::new();
        while let Some(&d) = scan.peek() {
            if d.is_ascii_digit() {
                id.push(d);
                scan.next();
            } else {
                break;
            }
        }
        let is_def = scan.peek() == Some(&';');
        if is_def {
            scan.next(); // ';'
            scan.next(); // '2'
            scan.next(); // ';'
            let mut vals = String::new();
            // stop BEFORE the structural char so the next pass still sees it
            while let Some(&c3) = scan.peek() {
                if c3 == '#' || c3 == '$' || c3 == '-' {
                    break;
                }
                vals.push(c3);
                scan.next();
            }
            let nums: Vec<u32> = vals.split(';').map(|v| v.trim().parse().unwrap()).collect();
            let inv = |v: u32| (v * 255 / 100) as u8;
            let id: usize = id.parse().unwrap();
            while colors.len() <= id {
                colors.push((0, 0, 0));
            }
            colors[id] = (inv(nums[0]), inv(nums[1]), inv(nums[2]));
        }
    }

    // Second pass: raster data.
    let mut grid = vec![255u8; w * h * 4]; // unmentioned pixels stay white
    let mut cur_color = 0usize;
    let mut x = 0usize;
    let mut y0 = 0usize;
    let mut pending_repeat: usize = 1;
    let mut it = rest.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '#' => {
                let mut id = String::new();
                while let Some(&d) = it.peek() {
                    if d.is_ascii_digit() {
                        id.push(d);
                        it.next();
                    } else {
                        break;
                    }
                }
                if it.peek() == Some(&';') {
                    // definition (`#id;2;R;G;B`) — skip its value, stopping
                    // BEFORE the next structural char (peek; consuming it
                    // would eat the following token's '#')
                    while let Some(&c2) = it.peek() {
                        if c2 == '#' || c2 == '$' || c2 == '-' {
                            break;
                        }
                        it.next();
                    }
                } else {
                    cur_color = id.parse().unwrap();
                    x = 0;
                }
            }
            '!' => {
                let mut n = String::new();
                while let Some(&d) = it.peek() {
                    if d.is_ascii_digit() {
                        n.push(d);
                        it.next();
                    } else {
                        break;
                    }
                }
                pending_repeat = n.parse().unwrap();
            }
            '$' => x = 0,
            '-' => {
                y0 += 6;
                x = 0;
            }
            ch if ('\u{3f}'..='\u{7e}').contains(&ch) => {
                let bits = (ch as u32 - 0x3f) as u8;
                let (r, g, b) = colors[cur_color];
                for _ in 0..pending_repeat {
                    for row in 0..6 {
                        if bits & (1 << row) != 0 {
                            let y = y0 + row;
                            if y < h && x < w {
                                let i = (y * w + x) * 4;
                                grid[i] = r;
                                grid[i + 1] = g;
                                grid[i + 2] = b;
                                grid[i + 3] = 255;
                            }
                        }
                    }
                    x += 1;
                }
                pending_repeat = 1;
            }
            _ => {}
        }
    }
    (w, h, grid)
}

#[test]
fn sixel_round_trips_through_decoder() {
    let img = scene();
    let bytes = sixel::write(&img.scene, 12);
    let (w, h, rgba) = decode_sixel(&bytes);
    let (pw, ph) = (
        (img.scene.size * 12.0).ceil() as usize,
        (img.scene.size * 12.0).ceil() as usize,
    );
    assert_eq!((w, h), (pw, ph), "sixel header dims match raster");

    let gray: Vec<u8> = rgba
        .chunks(4)
        .map(|p| (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8)
        .collect();
    let mut prep = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| gray[y * w + x]);
    let mut grids = prep.detect_grids();
    let (_m, content) = grids.pop().expect("detect").decode().expect("decode");
    assert_eq!(content, PAYLOAD);
}

#[test]
fn rasterizer_matches_reference_dimensions() {
    let img = scene();
    let pm = raster::rasterize(&img.scene, 8);
    let expect = (img.scene.size * 8.0).ceil() as u32;
    assert_eq!((pm.width(), pm.height()), (expect, expect));
}

#[test]
fn term_auto_detection_prefers_protocols() {
    use qrkit::writers::term_auto::detect_with;
    let kitty_id = detect_with(&|k| (k == "KITTY_WINDOW_ID").then(|| "1".into()));
    assert_eq!(kitty_id, TermImage::Kitty);
    let wez = detect_with(&|k| (k == "WEZTERM_EXECUTABLE").then(|| "/wezterm".into()));
    assert_eq!(wez, TermImage::Kitty);
    let ghostty = detect_with(&|k| (k == "TERM").then(|| "xterm-ghostty".into()));
    assert_eq!(ghostty, TermImage::Kitty);
    let it2 = detect_with(&|k| (k == "TERM_PROGRAM").then(|| "iTerm.app".into()));
    assert_eq!(it2, TermImage::Iterm);
    let it_lc = detect_with(&|k| (k == "LC_TERMINAL").then(|| "iTerm2".into()));
    assert_eq!(it_lc, TermImage::Iterm);
    let plain = detect_with(&|_| None);
    assert_eq!(plain, TermImage::AnsiBlocks);
}

#[test]
fn kitty_single_chunk_transmission_terminates() {
    // px=2 keeps the base64 PNG under one 4096-byte chunk (measured ~3.8k);
    // the transmission MUST still end with a final m=0 chunk or kitty
    // buffers it forever and never displays the image. Assert the chunk
    // count up front so payload/encoder changes can't silently grow it past
    // the single-chunk case this test exists to guard.
    let bytes = kitty::write(&scene().scene, 2);
    let s = String::from_utf8(bytes.to_vec()).unwrap();
    let chunks: Vec<&str> = s
        .split("\x1b\\")
        .filter(|c| c.starts_with("\x1b_G") && c.contains(';'))
        .collect();
    assert_eq!(chunks.len(), 1, "scene at px=2 must fit one chunk");
    let only = chunks[0];
    assert!(
        only.contains("m=0"),
        "single-chunk transmission must carry m=0 (terminates): {only:?}"
    );
}
