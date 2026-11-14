//! Where a footprint review was made, recorded in the symbol beside its hash: one line per datasheet, each in a
//! property of its own (`Footprint_Review`, `Footprint_Review2`, ...), the review hash first and the source after
//! it:
//!
//! ```text
//! fp1:7726812df851763a mpn=BSS138W-7-F,BSS138WQ-7-F doc=DS30154@Rev16 pdf=b3:0123456789abcdef page=3 scale=0.3528 at=12.5,40.2,90,mirror by=roman on=13nov2026
//! ```
//!
//! A line reviews the MPNs it names (`mpn=`), a line without them the whole symbol; a value with only the hash (as
//! written before the source was recorded) is such a line, a review with no source. [`assess`] tells whether every
//! MPN of a symbol is reviewed by a line whose hash is the footprint's, [`merge`] puts a new line among the ones a
//! symbol has.

use super::FIELD;
use erc_model::fphash;
use serde::{Deserialize, Serialize};

/// The name of review property `i` (0-based): [`FIELD`], then `Footprint_Review2`, `Footprint_Review3`, ...
pub fn property_name(i: usize) -> String {
    match i {
        0 => FIELD.to_string(),
        _ => format!("{FIELD}{}", i + 1),
    }
}

/// Which review property `name` is (0-based), `None` when it is none
pub fn property_index(name: &str) -> Option<usize> {
    let rest = name.strip_prefix(FIELD)?;
    if rest.is_empty() {
        return Some(0);
    }
    if rest.starts_with('0') || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: usize = rest.parse().ok()?;
    (n >= 2).then(|| n - 1)
}

/// The review properties among `fields` that have a value, as name and value, in the order of their numbers
pub fn review_values(fields: &[(String, String)]) -> Vec<(String, String)> {
    let mut out: Vec<(usize, String, String)> = fields
        .iter()
        .filter(|(_, v)| !v.trim().is_empty())
        .filter_map(|(k, v)| Some((property_index(k)?, k.clone(), v.trim().to_string())))
        .collect();
    out.sort_by_key(|(i, _, _)| *i);
    out.dedup_by_key(|(i, _, _)| *i);
    out.into_iter().map(|(_, k, v)| (k, v)).collect()
}

/// The symbol's MPNs: the values of its `Mpn`, `Mpn1`, `Mpn2`, ... fields (any case), in the order of their
/// numbers, each once
pub fn mpns(fields: &[(String, String)]) -> Vec<String> {
    let mut found: Vec<(usize, String)> = fields
        .iter()
        .filter_map(|(k, v)| {
            let rest = k.get(..3).filter(|p| p.eq_ignore_ascii_case("mpn"))?;
            let n = &k[rest.len()..];
            let n = if n.is_empty() {
                0
            } else {
                n.parse::<usize>().ok()?
            };
            let v = v.trim();
            (!v.is_empty() && v != "~").then(|| (n, v.to_string()))
        })
        .collect();
    found.sort_by_key(|(n, _)| *n);
    let mut out: Vec<String> = Vec::new();
    for (_, v) in found {
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// How the footprint sits on the datasheet page: its origin in page points (from the page's top left, y down),
/// turned counter-clockwise on screen, mirrored (seen from below) before turning
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct At {
    pub x: f64,
    pub y: f64,
    /// Degrees
    pub rot: f64,
    pub mirror: bool,
}

/// One review line: the review hash and where the review was made
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReviewLine {
    /// The first token: `fp1:` and 16 hex digits for a review hash of this version
    pub hash: String,
    /// The MPNs this datasheet covers; empty: the whole symbol
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mpns: Vec<String>,
    /// The document number and revision, `DS30154@Rev16`, where the document library knows them: they survive a
    /// download of the same document with other bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// The PDF by the document library's content hash: `b3:` and 16 hex digits of its blake3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf: Option<String>,
    /// 1-based, as a viewer shows it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// The drawing's scale on that page, mm per PDF point
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
    /// Who reviewed it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When, as `13nov2026`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<String>,
    /// Anything else on the line, as written: keys this build doesn't know, values it can't read. Kept, so a
    /// rewrite of the line loses nothing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<(String, String)>,
}

/// `,`, `%` and blanks in a value as `%2C`, `%25`, `%20`, so a line splits at spaces and a list at commas
fn encode(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '%' | ',' => out.push_str(&format!("%{:02X}", c as u32)),
            c if c.is_whitespace() => {
                let mut b = [0; 4];
                for byte in c.encode_utf8(&mut b).bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// [`encode`] undone; a `%` not followed by two hex digits stays as it is
fn decode(v: &str) -> String {
    let b = v.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && let Some(hex) = v.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| v.to_string())
}

/// A number as short as it reads: at most `decimals` places, no trailing zeros
fn number(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        &s
    };
    if s == "-0" { "0".into() } else { s.into() }
}

fn parse_at(v: &str) -> Option<At> {
    let parts: Vec<&str> = v.split(',').collect();
    let (nums, mirror) = match parts.as_slice() {
        [x, y, r] => ([*x, *y, *r], false),
        [x, y, r, "mirror"] => ([*x, *y, *r], true),
        _ => return None,
    };
    let [x, y, rot] = nums.map(|n| n.parse::<f64>().ok().filter(|v| v.is_finite()));
    Some(At {
        x: x?,
        y: y?,
        rot: rot?,
        mirror,
    })
}

impl ReviewLine {
    /// Reads a review property's value. Never fails: the first token is the hash whatever it is (a value that is
    /// no review hash shows in [`assess`]), and what can't be read goes into [`other`](Self::other)
    pub fn parse(value: &str) -> ReviewLine {
        let mut tokens = value.split_whitespace();
        let mut line = ReviewLine {
            hash: tokens.next().unwrap_or("").to_string(),
            ..ReviewLine::default()
        };
        for t in tokens {
            let Some((k, v)) = t.split_once('=') else {
                line.other.push((String::new(), t.to_string()));
                continue;
            };
            let known = match k {
                "mpn" if line.mpns.is_empty() => {
                    line.mpns = v.split(',').filter(|m| !m.is_empty()).map(decode).collect();
                    true
                }
                "doc" if line.doc.is_none() => {
                    line.doc = Some(decode(v));
                    true
                }
                "pdf" if line.pdf.is_none() => {
                    line.pdf = Some(decode(v));
                    true
                }
                "page" if line.page.is_none() => {
                    line.page = v.parse().ok().filter(|p| *p > 0);
                    line.page.is_some()
                }
                "scale" if line.scale.is_none() => {
                    line.scale = v.parse().ok().filter(|s: &f64| s.is_finite() && *s > 0.0);
                    line.scale.is_some()
                }
                "at" if line.at.is_none() => {
                    line.at = parse_at(v);
                    line.at.is_some()
                }
                "by" if line.by.is_none() => {
                    line.by = Some(decode(v));
                    true
                }
                "on" if line.on.is_none() => {
                    line.on = Some(decode(v));
                    true
                }
                _ => false,
            };
            if !known {
                line.other.push((k.to_string(), v.to_string()));
            }
        }
        line
    }

    /// The property value: the hash, then the source in a fixed order, then whatever else the line had
    pub fn to_value(&self) -> String {
        let mut out = self.hash.clone();
        let mut add = |k: &str, v: &str| {
            out.push(' ');
            out.push_str(k);
            out.push('=');
            out.push_str(v);
        };
        if !self.mpns.is_empty() {
            let list: Vec<String> = self.mpns.iter().map(|m| encode(m)).collect();
            add("mpn", &list.join(","));
        }
        if let Some(d) = &self.doc {
            add("doc", &encode(d));
        }
        if let Some(p) = &self.pdf {
            add("pdf", &encode(p));
        }
        if let Some(p) = self.page {
            add("page", &p.to_string());
        }
        if let Some(s) = self.scale {
            add("scale", &number(s, 6));
        }
        if let Some(a) = &self.at {
            let mut v = format!("{},{},{}", number(a.x, 2), number(a.y, 2), number(a.rot, 2));
            if a.mirror {
                v.push_str(",mirror");
            }
            add("at", &v);
        }
        if let Some(b) = &self.by {
            add("by", &encode(b));
        }
        if let Some(o) = &self.on {
            add("on", &encode(o));
        }
        for (k, v) in &self.other {
            out.push(' ');
            if !k.is_empty() {
                out.push_str(k);
                out.push('=');
            }
            out.push_str(v);
        }
        out
    }

    /// Where the review was made is on the line: a document, a PDF or a page
    pub fn has_source(&self) -> bool {
        self.doc.is_some() || self.pdf.is_some() || self.page.is_some()
    }

    /// The line names the datasheet it was reviewed on: its document number or its content hash
    pub fn names_document(&self) -> bool {
        self.doc.is_some() || self.pdf.is_some()
    }

    /// The line reviews `mpn`: it names it, or names none (the whole symbol)
    pub fn covers(&self, mpn: &str) -> bool {
        self.mpns.is_empty() || self.mpns.iter().any(|m| m == mpn)
    }

    /// Both lines are about one datasheet: the same PDF, or the same document number (the revision aside, a newer
    /// revision of a datasheet replaces the older's line)
    pub fn same_document(&self, other: &ReviewLine) -> bool {
        let doc_no = |d: &str| d.split('@').next().unwrap_or("").to_string();
        match (&self.pdf, &other.pdf) {
            (Some(a), Some(b)) if a == b => return true,
            _ => {}
        }
        match (&self.doc, &other.doc) {
            (Some(a), Some(b)) => doc_no(a) == doc_no(b),
            _ => false,
        }
    }
}

/// How one review line's hash compares with the footprint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineStatus {
    /// The footprint is the one reviewed
    Ok,
    /// The footprint changed since: the line is stale
    Changed,
    /// Made by another version of berc's review hash
    OtherVersion,
    /// The first token is no review hash
    Malformed,
}

/// One review property of a symbol, read and compared
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckedLine {
    /// `Footprint_Review`, `Footprint_Review2`, ...
    pub property: String,
    pub value: String,
    pub line: ReviewLine,
    pub status: LineStatus,
}

/// The review lines of a symbol against its footprint now
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Assessed {
    pub lines: Vec<CheckedLine>,
    /// The symbol's MPNs ([`mpns`])
    pub mpns: Vec<String>,
    /// Those no line reviews
    pub unreviewed_mpns: Vec<String>,
    /// The symbol has no MPNs and every line names some (MPNs it no longer has): no line reviews it as a whole
    #[serde(default)]
    pub whole_unreviewed: bool,
}

/// What the review lines say overall
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overall {
    /// Every line's hash is the footprint's, every MPN reviewed
    Ok,
    /// No line at all
    Unreviewed,
    /// Every line's hash is the footprint's, but some MPNs have no line
    Partial,
    Changed,
    OtherVersion,
    Malformed,
}

impl Assessed {
    /// The worst of the lines first (malformed, other version, changed), then MPNs not reviewed
    pub fn overall(&self) -> Overall {
        let any = |s: LineStatus| self.lines.iter().any(|l| l.status == s);
        if self.lines.is_empty() {
            Overall::Unreviewed
        } else if any(LineStatus::Malformed) {
            Overall::Malformed
        } else if any(LineStatus::OtherVersion) {
            Overall::OtherVersion
        } else if any(LineStatus::Changed) {
            Overall::Changed
        } else if !self.unreviewed_mpns.is_empty() || self.whole_unreviewed {
            Overall::Partial
        } else {
            Overall::Ok
        }
    }

    /// The hash to show as the one stored: the first stale line's, else the first line's
    pub fn stored(&self) -> Option<String> {
        self.lines
            .iter()
            .find(|l| l.status != LineStatus::Ok)
            .or(self.lines.first())
            .map(|l| l.line.hash.clone())
    }

    /// The lines whose hash is not the footprint's, by property
    pub fn stale(&self) -> Vec<&CheckedLine> {
        self.lines
            .iter()
            .filter(|l| l.status != LineStatus::Ok)
            .collect()
    }
}

/// The review lines among a symbol's `fields`, each compared by `compare` (the stored hash against the footprint;
/// `None`: no footprint to compare with, every line reads as ok), and the symbol's MPNs no line reviews
pub fn assess(
    fields: &[(String, String)],
    compare: Option<&dyn Fn(&str) -> fphash::Compared>,
) -> Assessed {
    let lines: Vec<CheckedLine> = review_values(fields)
        .into_iter()
        .map(|(property, value)| {
            let line = ReviewLine::parse(&value);
            let status = match compare.map(|c| c(&line.hash)) {
                None | Some(fphash::Compared::Same) => LineStatus::Ok,
                Some(fphash::Compared::Changed { .. }) => LineStatus::Changed,
                Some(fphash::Compared::OtherVersion { .. }) => LineStatus::OtherVersion,
                Some(fphash::Compared::Malformed) => LineStatus::Malformed,
            };
            CheckedLine {
                property,
                value,
                line,
                status,
            }
        })
        .collect();
    let mpns = mpns(fields);
    let unreviewed_mpns = if lines.is_empty() {
        Vec::new()
    } else {
        mpns.iter()
            .filter(|m| !lines.iter().any(|l| l.line.covers(m)))
            .cloned()
            .collect()
    };
    let whole_unreviewed =
        mpns.is_empty() && !lines.is_empty() && lines.iter().all(|l| !l.line.mpns.is_empty());
    Assessed {
        lines,
        mpns,
        unreviewed_mpns,
        whole_unreviewed,
    }
}

/// The review values a symbol is to hold once `new` is accepted, given the ones it has (`old`, as
/// [`review_values`] gives them) and its MPNs (`symbol_mpns`):
///
/// - a line about the same datasheet ([`ReviewLine::same_document`]) is replaced, in its place;
/// - `new` without MPNs (the whole symbol) replaces every line without MPNs, except another datasheet's: a line
///   naming a document other than `new`'s stays (one line per datasheet, never rewritten from another's);
/// - the MPNs `new` names leave the other lines; a line left with none goes;
/// - a line with only a hash, no source, goes once the lines with MPNs review every MPN of the symbol;
/// - a new line goes last.
pub fn merge(old: &[String], new: &ReviewLine, symbol_mpns: &[String]) -> Vec<String> {
    let mut lines: Vec<Option<ReviewLine>> = Vec::new();
    let mut placed = false;
    for v in old {
        let mut l = ReviewLine::parse(v);
        let other_document = l.names_document() && new.names_document();
        let replaced =
            l.same_document(new) || (new.mpns.is_empty() && l.mpns.is_empty() && !other_document);
        if replaced {
            if !placed {
                lines.push(Some(new.clone()));
                placed = true;
            }
            continue;
        }
        if !l.mpns.is_empty() && !new.mpns.is_empty() {
            l.mpns.retain(|m| !new.mpns.contains(m));
            if l.mpns.is_empty() {
                continue;
            }
        }
        lines.push(Some(l));
    }
    if !placed {
        lines.push(Some(new.clone()));
    }
    let named = |m: &String| {
        lines
            .iter()
            .flatten()
            .any(|l| !l.mpns.is_empty() && l.mpns.contains(m))
    };
    if !symbol_mpns.is_empty() && symbol_mpns.iter().all(named) {
        for l in lines.iter_mut() {
            if l.as_ref()
                .is_some_and(|l| l.mpns.is_empty() && !l.has_source())
            {
                *l = None;
            }
        }
    }
    lines.into_iter().flatten().map(|l| l.to_value()).collect()
}

/// The UTC day of Unix time `secs` as review lines write it: `13nov2026`
pub fn date_of(secs: i64) -> String {
    let t = erc_model::fact::Time(secs).to_string();
    let n = |r: std::ops::Range<usize>| t.get(r).and_then(|v| v.parse::<i64>().ok()).unwrap_or(1);
    date(n(0..4), n(5..7) as u32, n(8..10) as u32)
}

/// Today (UTC) as review lines write it
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    date_of(secs)
}

/// A date as review lines write it: `13nov2026`
pub fn date(year: i64, month: u32, day: u32) -> String {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let m = MONTHS[(month.clamp(1, 12) - 1) as usize];
    format!("{day}{m}{year}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "fp1:7726812df851763a mpn=BSS138W-7-F,BSS138WQ-7-F doc=DS30154@Rev16 \
                        pdf=b3:0123456789abcdef page=3 scale=0.3528 at=12.5,40.2,90,mirror by=roman on=13nov2026";

    fn f(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// A line read into its parts and written back the same; a comma or a space inside an MPN is escaped, so
    /// Nexperia's `BSS138BKW,115` stays one MPN
    #[test]
    fn line_round_trip() {
        let l = ReviewLine::parse(LINE);
        assert_eq!(l.hash, "fp1:7726812df851763a");
        assert_eq!(l.mpns, ["BSS138W-7-F", "BSS138WQ-7-F"]);
        assert_eq!(l.doc.as_deref(), Some("DS30154@Rev16"));
        assert_eq!(l.pdf.as_deref(), Some("b3:0123456789abcdef"));
        assert_eq!(l.page, Some(3));
        assert_eq!(l.scale, Some(0.3528));
        assert_eq!(
            l.at,
            Some(At {
                x: 12.5,
                y: 40.2,
                rot: 90.0,
                mirror: true
            })
        );
        assert_eq!(l.by.as_deref(), Some("roman"));
        assert_eq!(l.on.as_deref(), Some("13nov2026"));
        assert!(l.other.is_empty());
        assert_eq!(l.to_value(), LINE);
        let nx = ReviewLine {
            hash: "fp1:7726812df851763a".into(),
            mpns: vec!["BSS138BKW,115".into(), "A B%".into()],
            ..ReviewLine::default()
        };
        assert_eq!(
            nx.to_value(),
            "fp1:7726812df851763a mpn=BSS138BKW%2C115,A%20B%25"
        );
        assert_eq!(ReviewLine::parse(&nx.to_value()), nx);
    }

    /// A value with the hash alone (as written before the source was recorded) is a line with no source; what
    /// can't be read stays on the line, written back as it was
    #[test]
    fn hash_only_and_unknown() {
        let l = ReviewLine::parse("  fp1:34265c0d8697c34e ");
        assert_eq!(l.hash, "fp1:34265c0d8697c34e");
        assert!(!l.has_source());
        assert!(l.covers("anything"));
        assert_eq!(l.to_value(), "fp1:34265c0d8697c34e");
        let odd = ReviewLine::parse("fp1:34265c0d8697c34e page=x future=1 loose at=1,2");
        assert_eq!(odd.page, None);
        assert_eq!(odd.at, None);
        assert_eq!(
            odd.to_value(),
            "fp1:34265c0d8697c34e page=x future=1 loose at=1,2"
        );
    }

    #[test]
    fn property_names() {
        assert_eq!(property_name(0), "Footprint_Review");
        assert_eq!(property_name(1), "Footprint_Review2");
        assert_eq!(property_index("Footprint_Review"), Some(0));
        assert_eq!(property_index("Footprint_Review2"), Some(1));
        assert_eq!(property_index("Footprint_Review12"), Some(11));
        for no in [
            "Footprint_Review1",
            "Footprint_Review02",
            "Footprint_Reviewx",
            "Footprint",
        ] {
            assert_eq!(property_index(no), None, "{no}");
        }
        let fields = f(&[
            ("Footprint_Review3", "c"),
            ("Footprint_Review", "a"),
            ("Footprint_Review2", " "),
            ("Value", "x"),
        ]);
        assert_eq!(
            review_values(&fields),
            [
                ("Footprint_Review".to_string(), "a".to_string()),
                ("Footprint_Review3".to_string(), "c".to_string())
            ]
        );
    }

    #[test]
    fn mpn_fields() {
        let fields = f(&[
            ("Mpn2", "BSS138BKW,115"),
            ("Mpn1", "BSS138W-7-F"),
            ("MPN", "BSS138W-7-F"),
            ("Mpn3", "~"),
            ("Mpnx", "no"),
            ("Manufacturer", "Diodes"),
        ]);
        assert_eq!(mpns(&fields), ["BSS138W-7-F", "BSS138BKW,115"]);
    }

    /// Ok when every MPN is on a line whose hash is the footprint's; an MPN without one is partial; a line with
    /// another hash is stale; a hash alone covers every MPN, a symbol without MPNs needs a line without them
    #[test]
    fn coverage() {
        const NOW: &str = "fp1:1111111111111111";
        let compare = |stored: &str| {
            if stored == NOW {
                fphash::Compared::Same
            } else if fphash::is_hash(stored) {
                fphash::Compared::Changed { now: NOW.into() }
            } else {
                fphash::Compared::Malformed
            }
        };
        let compare: &dyn Fn(&str) -> fphash::Compared = &compare;
        let mut fields = f(&[("Mpn1", "A-7-F"), ("Mpn2", "B,115")]);
        let a = assess(&fields, Some(compare));
        assert_eq!(a.overall(), Overall::Unreviewed);
        assert!(
            a.unreviewed_mpns.is_empty(),
            "no line: unreviewed, not partial"
        );
        fields.push((FIELD.into(), format!("{NOW} mpn=A-7-F doc=DS1")));
        let a = assess(&fields, Some(compare));
        assert_eq!(a.overall(), Overall::Partial);
        assert_eq!(a.unreviewed_mpns, ["B,115"]);
        fields.push(("Footprint_Review2".into(), format!("{NOW} mpn=B%2C115")));
        assert_eq!(assess(&fields, Some(compare)).overall(), Overall::Ok);
        fields[3].1 = "fp1:2222222222222222 mpn=B%2C115".into();
        let a = assess(&fields, Some(compare));
        assert_eq!(a.overall(), Overall::Changed);
        assert_eq!(a.stored().as_deref(), Some("fp1:2222222222222222"));
        assert_eq!(a.stale()[0].property, "Footprint_Review2");
        // The hash alone: a review of the whole symbol, its MPNs too
        let legacy = f(&[("Mpn1", "A-7-F"), (FIELD, NOW)]);
        assert_eq!(assess(&legacy, Some(compare)).overall(), Overall::Ok);
        // No MPNs: it needs a line without them; one naming MPNs it no longer has reviews nothing of it
        let plain = f(&[(FIELD, &format!("{NOW} mpn=X"))]);
        let a = assess(&plain, Some(compare));
        assert!(a.whole_unreviewed);
        assert_eq!(a.overall(), Overall::Partial);
        let plain = f(&[(FIELD, &format!("{NOW} doc=DS1"))]);
        assert_eq!(assess(&plain, Some(compare)).overall(), Overall::Ok);
        let bad = f(&[(FIELD, "nonsense doc=DS1")]);
        assert_eq!(assess(&bad, Some(compare)).overall(), Overall::Malformed);
    }

    /// Accepting a datasheet's line among the ones a symbol has
    #[test]
    fn merging() {
        let line = |s: &str| ReviewLine::parse(s);
        let mpns = ["A".to_string(), "B".to_string(), "C".to_string()];
        // The same document: replaced in place, its newer revision too
        let old = vec![
            "fp1:aaaaaaaaaaaaaaaa mpn=A,B doc=DS1@1".to_string(),
            "fp1:aaaaaaaaaaaaaaaa mpn=C doc=DS2@1".to_string(),
        ];
        let new = line("fp1:bbbbbbbbbbbbbbbb mpn=A,B doc=DS1@2");
        assert_eq!(
            merge(&old, &new, &mpns),
            [
                "fp1:bbbbbbbbbbbbbbbb mpn=A,B doc=DS1@2",
                "fp1:aaaaaaaaaaaaaaaa mpn=C doc=DS2@1"
            ]
        );
        // MPNs move to the new datasheet; a line left with none goes
        let new = line("fp1:bbbbbbbbbbbbbbbb mpn=B,C doc=DS3");
        assert_eq!(
            merge(&old, &new, &mpns),
            [
                "fp1:aaaaaaaaaaaaaaaa mpn=A doc=DS1@1",
                "fp1:bbbbbbbbbbbbbbbb mpn=B,C doc=DS3"
            ]
        );
        // A hash alone stays while some MPN has no line of its own, goes once all have
        let old = vec!["fp1:aaaaaaaaaaaaaaaa".to_string()];
        let ab = line("fp1:bbbbbbbbbbbbbbbb mpn=A,B pdf=b3:0123456789abcdef");
        let kept = merge(&old, &ab, &mpns);
        assert_eq!(
            kept,
            [
                "fp1:aaaaaaaaaaaaaaaa",
                "fp1:bbbbbbbbbbbbbbbb mpn=A,B pdf=b3:0123456789abcdef"
            ]
        );
        let c = line("fp1:bbbbbbbbbbbbbbbb mpn=C doc=DS2");
        assert_eq!(
            merge(&kept, &c, &mpns),
            [
                "fp1:bbbbbbbbbbbbbbbb mpn=A,B pdf=b3:0123456789abcdef",
                "fp1:bbbbbbbbbbbbbbbb mpn=C doc=DS2"
            ]
        );
        // A line without MPNs (a symbol without them, or a hash given by hand) replaces those without MPNs
        let whole = line("fp1:cccccccccccccccc doc=DS9");
        assert_eq!(
            merge(&["fp1:aaaaaaaaaaaaaaaa".to_string()], &whole, &[]),
            ["fp1:cccccccccccccccc doc=DS9"]
        );
        assert_eq!(merge(&[], &whole, &[]), ["fp1:cccccccccccccccc doc=DS9"]);
        // ... but not another datasheet's line: each datasheet keeps its own
        assert_eq!(
            merge(&["fp1:aaaaaaaaaaaaaaaa doc=DS1".to_string()], &whole, &[]),
            [
                "fp1:aaaaaaaaaaaaaaaa doc=DS1",
                "fp1:cccccccccccccccc doc=DS9"
            ]
        );
        assert_eq!(
            merge(&["fp1:aaaaaaaaaaaaaaaa doc=DS9@2".to_string()], &whole, &[]),
            ["fp1:cccccccccccccccc doc=DS9"]
        );
    }

    #[test]
    fn dates() {
        assert_eq!(date(2026, 11, 13), "13nov2026");
        assert_eq!(date(2027, 1, 2), "2jan2027");
        assert_eq!(
            date_of(erc_model::fact::Time::from_utc(2026, 11, 13, 23, 59, 0).0),
            "13nov2026"
        );
    }
}
