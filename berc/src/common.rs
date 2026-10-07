//! Output helpers shared by the schematic and board renderers: quoting, natural sort, generic one-line printing,
//! counted line sets and a stable hash.

use crate::sexpr::Sexp;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Add positions, rotations and per-item geometry (wires, tracks)
    pub with_geometry: bool,
}

/// Children never printed by the generic printer: identity and styling, not content
const DROP_ALWAYS: &[&str] = &["uuid", "tstamp", "effects", "stroke"];
/// Children that are positions, printed only with `--with-geometry`
pub const GEOMETRY: &[&str] = &["at", "xy", "pts", "start", "end", "mid", "center", "size"];

/// Text as a single-line token: bare when it is simple, quoted and escaped otherwise
pub fn q(s: &str) -> String {
    let simple = !s.is_empty()
        && s.chars().all(|c| {
            !c.is_whitespace() && !c.is_control() && !matches!(c, '"' | '\\' | '=' | '(' | ')')
        });
    if simple { s.to_string() } else { quote(s) }
}

/// Always quoted, with `\n`, `\t`, `\"` and `\\` escaped, so any text stays on one line
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Natural order: `R2` before `R10`, `C1.2` before `C1.10`
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.as_bytes(), b.as_bytes());
    loop {
        match (a.first(), b.first()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na = a.iter().take_while(|c| c.is_ascii_digit()).count();
                let nb = b.iter().take_while(|c| c.is_ascii_digit()).count();
                let (da, db) = (&a[..na], &b[..nb]);
                let ta = trim_zeros(da);
                let tb = trim_zeros(db);
                let ord = ta
                    .len()
                    .cmp(&tb.len())
                    .then_with(|| ta.cmp(tb))
                    .then_with(|| na.cmp(&nb));
                if ord != Ordering::Equal {
                    return ord;
                }
                a = &a[na..];
                b = &b[nb..];
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(y);
                }
                a = &a[1..];
                b = &b[1..];
            }
        }
    }
}

fn trim_zeros(d: &[u8]) -> &[u8] {
    let n = d
        .iter()
        .take_while(|&&c| c == b'0')
        .count()
        .min(d.len().saturating_sub(1));
    &d[n..]
}

/// One-line form of any expression, without uuids and styling, and without positions unless asked for
pub fn compact(e: &Sexp, opts: Options) -> String {
    let mut out = String::new();
    compact_into(e, opts, &mut out);
    out
}

fn compact_into(e: &Sexp, opts: Options, out: &mut String) {
    match e {
        Sexp::Atom(a) if a.starts_with('|') => {
            let _ = write!(out, "<{} bytes, hash {}>", a.len(), hash(a));
        }
        Sexp::Atom(a) => out.push_str(a),
        Sexp::Str(s) => out.push_str(&quote(s)),
        Sexp::List(items) => {
            out.push('(');
            let mut first = true;
            for item in items {
                if let Some(h) = item.head() {
                    if DROP_ALWAYS.contains(&h) || (!opts.with_geometry && GEOMETRY.contains(&h)) {
                        continue;
                    }
                }
                if !first {
                    out.push(' ');
                }
                first = false;
                compact_into(item, opts, out);
            }
            out.push(')');
        }
    }
}

/// FNV-1a, 32 bits as 8 hex digits: short, stable across builds and platforms
pub fn hash(s: &str) -> String {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    format!("{h:08x}")
}

/// Lines that may repeat: printed sorted, once each, with ` x3` when repeated
#[derive(Default)]
pub struct Counted(BTreeMap<SortKey, usize>);

impl Counted {
    pub fn add(&mut self, line: String) {
        *self.0.entry(SortKey(line)).or_default() += 1;
    }

    pub fn lines(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|(k, &n)| {
                if n > 1 {
                    format!("{} x{n}", k.0)
                } else {
                    k.0.clone()
                }
            })
            .collect()
    }

    pub fn total(&self) -> usize {
        self.0.values().sum()
    }
}

/// A string ordered naturally (ties broken bytewise so equal keys are really equal)
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SortKey(pub String);

impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        natural_cmp(&self.0, &other.0).then_with(|| self.0.cmp(&other.0))
    }
}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Sorts naturally in place
pub fn sort_natural(v: &mut [String]) {
    v.sort_by(|a, b| natural_cmp(a, b).then_with(|| a.cmp(b)));
}

/// Output: named sections of lines, written with a `# name (count)` header, empty sections left out
#[derive(Default)]
pub struct Out {
    text: String,
}

impl Out {
    pub fn line(&mut self, line: impl AsRef<str>) {
        self.text.push_str(line.as_ref());
        self.text.push('\n');
    }

    /// `count` is what the header shows; blocks of several lines (a symbol and its fields) count as one
    pub fn section(&mut self, name: &str, count: usize, lines: &[String]) {
        if lines.is_empty() {
            return;
        }
        let _ = writeln!(self.text, "\n# {name} ({count})");
        for l in lines {
            self.line(l);
        }
    }

    pub fn append(&mut self, other: Out) {
        self.text.push_str(&other.text);
    }

    pub fn finish(self) -> String {
        self.text
    }
}

/// Number as KiCad writes it, rounded to `decimals`: `90`, `-0.5`, `12.7`
pub fn num(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    if s == "-0" { "0".to_string() } else { s }
}

/// `@(x,y)` or `@(x,y,rot)` from an `(at x y [rot])` child, as written in the file
pub fn at_of(e: &Sexp) -> String {
    match e.child("at") {
        Some(at) => {
            let parts: Vec<&str> = at.args().iter().filter_map(Sexp::text).collect();
            format!("@({})", parts.join(","))
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::parse;

    #[test]
    fn natural() {
        let mut v: Vec<String> = ["R10", "R2", "C1", "R1", "R02", "U1.10", "U1.9"]
            .map(String::from)
            .to_vec();
        sort_natural(&mut v);
        assert_eq!(v, ["C1", "R1", "R2", "R02", "R10", "U1.9", "U1.10"]);
    }

    #[test]
    fn quoting() {
        assert_eq!(q("GND"), "GND");
        assert_eq!(q("/USB Plug/CC1"), "\"/USB Plug/CC1\"");
        assert_eq!(q(""), "\"\"");
        assert_eq!(quote("a\nb\"c\\"), r#""a\nb\"c\\""#);
    }

    #[test]
    fn compact_drops_uuid_and_geometry() {
        let v = parse(r#"(thing "x" (at 1 2) (uuid "u") (layer "F.Cu") (data |abc|))"#).unwrap();
        assert_eq!(
            compact(&v[0], Options::default()),
            r#"(thing "x" (layer "F.Cu") (data <5 bytes, hash fcf4c23b>))"#
        );
        let geo = compact(
            &v[0],
            Options {
                with_geometry: true,
            },
        );
        assert!(geo.contains("(at 1 2)"));
    }

    #[test]
    fn counted() {
        let mut c = Counted::default();
        c.add("b".into());
        c.add("a".into());
        c.add("b".into());
        assert_eq!(c.lines(), ["a", "b x2"]);
        assert_eq!(c.total(), 3);
    }

    #[test]
    fn numbers() {
        assert_eq!(num(90.0, 1), "90");
        assert_eq!(num(12.749, 1), "12.7");
        assert_eq!(num(-0.01, 1), "0");
    }
}
