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
