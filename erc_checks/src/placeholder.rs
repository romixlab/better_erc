//! Placeholders: text that stands for a decision not taken yet, in a value (`BOM-5`) or in a note (`DRAW-49`)

/// The words that mark an open decision, matched whole and case ignored
const WORDS: [&str; 5] = ["TBD", "TBC", "TODO", "FIXME", "CALC"];

/// Letters that stand for a number not filled in (`x V .. y V`)
const UNKNOWNS: [&str; 6] = ["x", "y", "z", "X", "Y", "Z"];

/// Units an unknown stands before, as in `x V` or `xmA`
const UNITS: [&str; 26] = [
    "V", "mV", "uV", "µV", "kV", "A", "mA", "uA", "µA", "W", "mW", "Hz", "kHz", "MHz", "Ω", "ohm",
    "%", "°C", "s", "ms", "us", "µs", "uF", "nF", "pF", "uH",
];

/// Whether a word is one of [`WORDS`]
fn open_word(w: &str) -> bool {
    WORDS.iter().any(|x| w.eq_ignore_ascii_case(x))
}

/// A value that is a decision not taken: starts or ends with `?` (`?`, `??k`, `4k7?`), or has one of [`WORDS`] as a
/// word (`TBD`, `calc`)
pub(crate) fn placeholder(value: &str) -> bool {
    let v = value.trim();
    v.starts_with('?')
        || v.ends_with('?')
        || v.split(|c: char| !c.is_ascii_alphanumeric()).any(open_word)
}

/// Whether a line is an empty label (`Max current:`) or a rule (`--`) that a note template leaves between labels
fn blank_line(line: &str) -> bool {
    let l = line.trim();
    l.is_empty() || l.chars().all(|c| c == '-') || l.ends_with(':')
}

/// The placeholders in one word of a line, given the word after it: an open word (`TBD`), a word of question marks
/// (`???`), an unknown before a unit (`x V`) or joined to it (`ymA`)
fn in_word(raw: &str, next: &str, kind_word: bool, out: &mut Vec<String>) {
    let w = raw.trim_matches(|c: char| matches!(c, ',' | ';' | '.' | '(' | ')' | '=' | ':'));
    let open = !kind_word && w.split(|c: char| !c.is_ascii_alphanumeric()).any(open_word);
    let asked = !raw.is_empty() && raw.chars().all(|c| c == '?');
    let joined = UNKNOWNS
        .iter()
        .any(|x| w.strip_prefix(x).is_some_and(|u| UNITS.contains(&u)));
    let next = next.trim_end_matches([',', '.', ';']);
    if asked {
        out.push(raw.to_string());
    } else if open || joined {
        out.push(w.to_string());
    } else if UNKNOWNS.contains(&w) && UNITS.contains(&next) {
        out.push(format!("{w} {next}"));
    }
}

/// The placeholders in a note or a text, as they stand: an open word (`TBD`), a word of question marks (`???`), an
/// unknown before a unit (`x V`, `ymA`), a label with nothing after it (`Max current:` with nothing but blank lines,
/// rules or other empty labels after it). The open word that starts a note with a colon names the note's kind
/// (`DRAW-32`), not a placeholder.
pub(crate) fn placeholders_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (n, line) in lines.iter().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        for (i, raw) in words.iter().enumerate() {
            let next = words.get(i + 1).copied().unwrap_or("");
            let kind_word = n == 0 && i == 0 && raw.ends_with(':');
            in_word(raw, next, kind_word, &mut out);
        }
        let l = line.trim();
        if l.ends_with(':') && lines[n + 1..].iter().all(|l| blank_line(l)) {
            out.push(l.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{placeholder, placeholders_in};

    #[test]
    fn placeholders() {
        for v in [
            "?",
            "??k",
            "4k7?",
            "TBD",
            "tbd",
            "10k TODO",
            "calc",
            "CALC later",
            "FIXME",
        ] {
            assert!(placeholder(v), "{v}");
        }
        for v in ["", "4k7", "DNP", "TBDX", "STUDIO", "CALCULATOR"] {
            assert!(!placeholder(v), "{v}");
        }
    }

    #[test]
    fn placeholders_in_notes() {
        let cases: [(&str, &[&str]); 9] = [
            (
                "Unregulated, x V .. y V\nMax current:",
                &["x V", "y V", "Max current:"],
            ),
            ("Vout = ? V", &["?"]),
            ("Gain ???", &["???"]),
            ("Ripple TBD, ymA peak", &["TBD", "ymA"]),
            ("Vin:\nIout:\n", &["Vin:", "Iout:"]),
            ("Information:\n--\n", &["Information:"]),
            ("TODO:\nfit R5", &[]),
            ("Information:\n--\nGain = 0.1394", &[]),
            (
                "Reset-time pins:\nS strap, why? a 2x V divider, 3 x 10 V, X7R",
                &[],
            ),
        ];
        for (text, want) in cases {
            assert_eq!(placeholders_in(text), want, "{text:?}");
        }
    }
}
