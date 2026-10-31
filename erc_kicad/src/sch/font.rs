//! Text extents in KiCad's stroke font (newstroke): a glyph advances `units / 21` of the font's width, a capital is
//! as tall as the font's height. Checked against `kicad-cli sch export svg` (`tests/geometry.rs`).

/// Glyph advances in 1/21 of the font width, U+0020 to U+007E
const ADVANCE: [u8; 95] = [
    16, 10, 16, 21, 20, 24, 26, 10, 14, 14, 16, 26, 10, 26, 10, 22, 20, 20, 20, 20, 20, 20, 20, 20,
    20, 20, 10, 10, 26, 26, 26, 18, 27, 18, 21, 21, 21, 19, 18, 21, 22, 10, 16, 21, 17, 24, 22, 22,
    21, 22, 21, 20, 16, 22, 18, 24, 20, 18, 20, 14, 14, 14, 12, 16, 8, 19, 19, 18, 19, 18, 12, 19,
    19, 10, 10, 17, 11, 28, 19, 19, 19, 19, 13, 17, 12, 19, 16, 22, 17, 16, 17, 14, 20, 14, 15,
];

/// The advance of a glyph beyond ASCII that schematics use; anything else as wide as a digit
fn advance(c: char) -> u32 {
    match c {
        ' '..='~' => ADVANCE[c as usize - 0x20] as u32,
        '±' | '×' | '≈' | '≤' | '≥' | '→' => 26,
        'µ' | 'μ' => 22,
        '°' | '²' | '³' => 16,
        'Ω' => 24,
        '–' => 12,
        '—' => 24,
        '…' => 30,
        _ => 20,
    }
}

/// How much smaller KiCad draws `_{sub}` and `^{super}` text
const SCRIPT_SCALE: f64 = 0.7;

/// Line pitch over the font height
pub const LINE_PITCH: f64 = 1.62;

/// A text as shown: `~{OVER}`, `_{sub}` and `^{super}` without their markers
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '~' | '_' | '^' if chars.peek() == Some(&'{') => {
                chars.next();
                let mut depth = 1;
                for c in chars.by_ref() {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    out.push(c);
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// The width of one line with its markup, in the units of `size`
pub fn line_width(line: &str, size: f64) -> f64 {
    let mut units = 0.0;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '~' | '_' | '^' if chars.peek() == Some(&'{') => {
                chars.next();
                let scale = if c == '~' { 1.0 } else { SCRIPT_SCALE };
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    units += advance(c) as f64 * scale;
                }
            }
            c => units += advance(c) as f64,
        }
    }
    units * size / 21.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_as_kicad_draws_them() {
        // textLength of kicad-cli's SVG, less its constant 0.2032 mm
        assert!((line_width("+3V3_TGT", 1.27) - 9.2528).abs() < 1e-3);
        assert!((line_width("connect", 1.27) - 7.4384).abs() < 1e-3);
    }

    #[test]
    fn markup_is_not_drawn() {
        assert_eq!(plain("~{RST}_{1}x"), "RST1x");
        assert_eq!(line_width("~{R}", 21.0), line_width("R", 21.0));
        assert!(line_width("a_{1}", 21.0) < line_width("a1", 21.0));
    }
}
