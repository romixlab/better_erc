//! The parts snapshot, `parts.lock.toml`: for every MPN of a board the part facts the checks use and the footprint
//! reviews covering each (board footprint hash, MPN) pair, as a part source gave them at the last refresh.
//!
//! [`PartsLock::to_toml`] writes it in one fixed order (parts by MPN, facts by key, pairs by hash and MPN, reviews
//! by id), one line per fact, and nothing that changes when the source did not (no refresh time): a refresh is a
//! diff of what changed in the source. [`PartsLock::parse`] reads it back; keys it does not know are ignored, so a
//! file written by a newer better_erc still loads.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use toml::Value;

/// The default file name, beside the project
pub const FILE_NAME: &str = "parts.lock.toml";

/// The format version [`PartsLock::to_toml`] writes
pub const VERSION: u32 = 1;

/// The largest lock read, bytes
pub const FILE_SIZE_BYTES_MAX: u64 = 16 << 20;

/// A parts snapshot
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartsLock {
    pub version: u32,
    /// The part source it was written from (`mx3`, `file parts.toml`)
    pub source: String,
    /// The fact keys asked for, sorted
    #[serde(default)]
    pub keys: Vec<String>,
    /// Keys the source has no definition for at all, sorted
    #[serde(default)]
    pub unknown_keys: Vec<String>,
    /// One entry per MPN of the board, by MPN
    #[serde(default, rename = "part")]
    pub parts: Vec<LockedPart>,
    /// One entry per (board footprint hash, MPN) pair and source part, by hash, MPN and maker
    #[serde(default, rename = "footprint")]
    pub footprints: Vec<LockedPair>,
    /// Every review a pair names, by id
    #[serde(default, rename = "review")]
    pub reviews: Vec<LockedReview>,
}

/// One MPN as the source knows it
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LockedPart {
    /// As the board writes it
    pub mpn: String,
    #[serde(rename = "match")]
    pub matched: PartMatch,
    /// The source's id of the part, when it found exactly one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// The source's reading of the MPN by its scheme, also for an MPN it holds no part for
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decode: Option<Decode>,
    /// Asked keys the part has no value for, sorted
    #[serde(default)]
    pub missing: Vec<String>,
    /// The resolved value per key, by key
    #[serde(default)]
    pub facts: Vec<LockedFact>,
}

impl LockedPart {
    /// The fact for `key`
    pub fn fact(&self, key: &str) -> Option<&LockedFact> {
        self.facts.iter().find(|f| f.key == key)
    }
}

/// Whether the source found the MPN's part
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartMatch {
    /// Exactly one part
    Found,
    /// No part has this MPN
    #[default]
    NotFound,
    /// Several parts have it (several makers): no value is resolved
    NotUnique,
}

impl PartMatch {
    fn name(self) -> &'static str {
        match self {
            PartMatch::Found => "found",
            PartMatch::NotFound => "not_found",
            PartMatch::NotUnique => "not_unique",
        }
    }
}

/// An MPN read by the maker's numbering scheme
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Decode {
    /// `decoded`, `no_scheme`, `partial`, `ambiguous`, `maker_mismatch`
    pub outcome: String,
    /// The scheme, when decoded (`yageo.rc`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    /// The decoded segments that set a parameter: key to value as the scheme writes it (`case_eia` to `0402`)
    #[serde(default)]
    pub segments: BTreeMap<String, String>,
}

/// One resolved fact of a part, with where it came from
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LockedFact {
    /// The fact key ([`crate::keys`])
    pub key: String,
    #[serde(flatten)]
    pub value: FactValue,
    /// The source's own text
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// Who stated it (`manual`, `digikey`, `mpn_decode`)
    pub source: String,
    /// Its rank among the sources: higher wins
    pub rank: i32,
    /// When the source stated it, RFC 3339
    pub at: String,
    /// Sources of a high rank disagree on it
    #[serde(default, skip_serializing_if = "is_false")]
    pub conflict: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A fact's value. Numbers are in the SI base unit without prefix (`1e-7` F for 100 nF).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FactValue {
    /// A number and its unit (`F`, `Ω`, `V`, `W`, `A`, `H`)
    Number { value: f64, unit: String },
    /// A tolerance: below and above the nominal value, in `%` of it or in the unit
    Tolerance { minus: f64, plus: f64, unit: String },
    /// From `min` to `max`
    Range { min: f64, max: f64, unit: String },
    /// A word or text (`X7R`, `0402`, `thick_film`)
    Text { text: String },
}

/// One (board footprint hash, MPN) pair and the reviews that cover it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LockedPair {
    /// The board footprint's review hash (`fp1:` and 16 hex digits)
    pub hash: String,
    pub mpn: String,
    /// The footprint's `Lib:Name` on the board, for display
    #[serde(default)]
    pub footprint: String,
    /// The maker, when the source has several parts of this MPN
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker: Option<String>,
    pub status: Coverage,
    /// Ids of the reviews of this hash that cover the part, ascending
    #[serde(default)]
    pub covered_by: Vec<i64>,
    /// Ids of reviews of other hashes that cover the part (the footprint changed since, or another was reviewed)
    #[serde(default)]
    pub other_hashes: Vec<i64>,
}

/// Whether a review covers a pair
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    /// A review of this hash covers the part
    Covered,
    /// The part is known, no review of this hash covers it
    Uncovered,
    /// The source has no part of this MPN
    UnknownMpn,
}

impl Coverage {
    fn name(self) -> &'static str {
        match self {
            Coverage::Covered => "covered",
            Coverage::Uncovered => "uncovered",
            Coverage::UnknownMpn => "unknown_mpn",
        }
    }
}

/// A footprint review: a person checked the footprint with this hash against a document
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LockedReview {
    pub id: i64,
    pub hash: String,
    /// `Lib:Name` at review time
    #[serde(default)]
    pub footprint: String,
    /// Document number and revision (`DS30154`, `rev16`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_rev: Option<String>,
    /// The PDF's blake3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf_blake3: Option<String>,
    /// The page of the land pattern, 1-based
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    pub reviewer: String,
    /// `2026-11-25`
    pub reviewed_on: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// What it covers, in the source's order
    #[serde(default)]
    pub covers: Vec<Cover>,
    /// Set once retracted: who, when, why
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retracted: Option<String>,
}

/// What one review covers
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cover {
    /// One part, by its MPN at review time
    Mpn { mpn: String },
    /// Every part whose MPN decodes under `scheme` with these segment values
    Family {
        scheme: String,
        segments: BTreeMap<String, String>,
    },
}

/// Why a lock cannot be read
#[derive(Debug)]
pub enum LockError {
    /// The file cannot be read
    Read { file: String, error: std::io::Error },
    /// Larger than [`FILE_SIZE_BYTES_MAX`]
    TooLarge { file: String, size_bytes: u64 },
    /// Not TOML, or not the lock's shape
    Syntax {
        file: String,
        error: Box<toml::de::Error>,
    },
    /// Written by a newer format version
    Version { file: String, version: u32 },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::Read { file, error } => write!(f, "{file}: {error}"),
            LockError::TooLarge { file, size_bytes } => write!(
                f,
                "{file}: {size_bytes} bytes, a parts lock is at most {FILE_SIZE_BYTES_MAX}"
            ),
            LockError::Syntax { file, error } => {
                write!(f, "{file}: {}", error.to_string().trim_end())
            }
            LockError::Version { file, version } => write!(
                f,
                "{file}: format version {version}, this better_erc reads up to {VERSION}"
            ),
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockError::Read { error, .. } => Some(error),
            LockError::Syntax { error, .. } => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl PartsLock {
    /// Reads a lock's text, sorted ([`PartsLock::sort`]); `file` names it in errors
    pub fn parse(file: &str, text: &str) -> Result<PartsLock, LockError> {
        let mut lock: PartsLock = toml::from_str(text).map_err(|e| LockError::Syntax {
            file: file.to_owned(),
            error: Box::new(e),
        })?;
        if lock.version > VERSION {
            return Err(LockError::Version {
                file: file.to_owned(),
                version: lock.version,
            });
        }
        lock.sort();
        Ok(lock)
    }

    /// Reads the lock at `path`, at most [`FILE_SIZE_BYTES_MAX`]
    pub fn read(path: &std::path::Path) -> Result<PartsLock, LockError> {
        use std::io::Read;
        let file = path.display().to_string();
        let read_err = |error| LockError::Read {
            file: file.clone(),
            error,
        };
        let f = std::fs::File::open(path).map_err(read_err)?;
        let mut bytes = Vec::new();
        f.take(FILE_SIZE_BYTES_MAX + 1)
            .read_to_end(&mut bytes)
            .map_err(read_err)?;
        if bytes.len() as u64 > FILE_SIZE_BYTES_MAX {
            return Err(LockError::TooLarge {
                file,
                size_bytes: bytes.len() as u64,
            });
        }
        let text = String::from_utf8(bytes)
            .map_err(|e| read_err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
        PartsLock::parse(&file, &text)
    }

    /// The entry of `mpn`, as the board writes it
    pub fn part(&self, mpn: &str) -> Option<&LockedPart> {
        self.parts.iter().find(|p| p.mpn == mpn)
    }

    /// The pairs of a footprint hash and an MPN
    pub fn pairs<'a>(
        &'a self,
        hash: &'a str,
        mpn: &'a str,
    ) -> impl Iterator<Item = &'a LockedPair> {
        self.footprints
            .iter()
            .filter(move |p| p.hash == hash && p.mpn == mpn)
    }

    /// The review with this id
    pub fn review(&self, id: i64) -> Option<&LockedReview> {
        self.reviews.iter().find(|r| r.id == id)
    }

    /// Puts every list in the order [`PartsLock::to_toml`] writes, duplicates removed
    pub fn sort(&mut self) {
        self.keys.sort();
        self.keys.dedup();
        self.unknown_keys.sort();
        self.unknown_keys.dedup();
        for p in &mut self.parts {
            p.missing.sort();
            p.missing.dedup();
            p.facts.sort_by(|a, b| a.key.cmp(&b.key));
            p.facts.dedup_by(|a, b| a.key == b.key);
        }
        self.parts.sort_by(|a, b| a.mpn.cmp(&b.mpn));
        self.parts.dedup_by(|a, b| a.mpn == b.mpn);
        for p in &mut self.footprints {
            p.covered_by.sort_unstable();
            p.covered_by.dedup();
            p.other_hashes.sort_unstable();
            p.other_hashes.dedup();
        }
        self.footprints
            .sort_by(|a, b| (&a.hash, &a.mpn, &a.maker).cmp(&(&b.hash, &b.mpn, &b.maker)));
        self.footprints
            .dedup_by(|a, b| (&a.hash, &a.mpn, &a.maker) == (&b.hash, &b.mpn, &b.maker));
        self.reviews.sort_by_key(|r| r.id);
        self.reviews.dedup_by_key(|r| r.id);
    }

    /// The lock as TOML, sorted ([`PartsLock::sort`]), one line per fact
    pub fn to_toml(&self) -> String {
        let mut lock = self.clone();
        lock.sort();
        let mut out = String::new();
        out += "# The part facts and footprint reviews the checks read. Written by `berc parts refresh`; edits are\n";
        out += "# lost on the next refresh.\n";
        line(&mut out, "version", &int(VERSION.into()));
        line(&mut out, "source", &s(&lock.source));
        line(&mut out, "keys", &strings(&lock.keys));
        line(&mut out, "unknown_keys", &strings(&lock.unknown_keys));
        for p in &lock.parts {
            out += "\n[[part]]\n";
            line(&mut out, "mpn", &s(&p.mpn));
            line(&mut out, "match", &s(p.matched.name()));
            if let Some(id) = &p.part {
                line(&mut out, "part", &s(id));
            }
            if let Some(d) = &p.decode {
                line(&mut out, "decode", &decode(d));
            }
            line(&mut out, "missing", &strings(&p.missing));
            if p.facts.is_empty() {
                line(&mut out, "facts", "[]");
            } else {
                out += "facts = [\n";
                for f in &p.facts {
                    out += "    ";
                    out += &fact(f);
                    out += ",\n";
                }
                out += "]\n";
            }
        }
        for p in &lock.footprints {
            out += "\n[[footprint]]\n";
            line(&mut out, "hash", &s(&p.hash));
            line(&mut out, "mpn", &s(&p.mpn));
            line(&mut out, "footprint", &s(&p.footprint));
            if let Some(m) = &p.maker {
                line(&mut out, "maker", &s(m));
            }
            line(&mut out, "status", &s(p.status.name()));
            line(&mut out, "covered_by", &ints(&p.covered_by));
            line(&mut out, "other_hashes", &ints(&p.other_hashes));
        }
        for r in &lock.reviews {
            out += "\n[[review]]\n";
            line(&mut out, "id", &int(r.id));
            line(&mut out, "hash", &s(&r.hash));
            line(&mut out, "footprint", &s(&r.footprint));
            for (k, v) in [
                ("doc", &r.doc),
                ("doc_rev", &r.doc_rev),
                ("pdf_blake3", &r.pdf_blake3),
            ] {
                if let Some(v) = v {
                    line(&mut out, k, &s(v));
                }
            }
            if let Some(p) = r.page {
                line(&mut out, "page", &int(p.into()));
            }
            line(&mut out, "reviewer", &s(&r.reviewer));
            line(&mut out, "reviewed_on", &s(&r.reviewed_on));
            if !r.note.is_empty() {
                line(&mut out, "note", &s(&r.note));
            }
            let covers: Vec<String> = r.covers.iter().map(cover).collect();
            line(&mut out, "covers", &format!("[{}]", covers.join(", ")));
            if let Some(why) = &r.retracted {
                line(&mut out, "retracted", &s(why));
            }
        }
        out
    }
}

fn line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push_str(" = ");
    out.push_str(value);
    out.push('\n');
}

/// A TOML string, quoted and escaped
fn s(v: &str) -> String {
    Value::String(v.to_owned()).to_string()
}

fn int(v: i64) -> String {
    Value::Integer(v).to_string()
}

fn float(v: f64) -> String {
    Value::Float(v).to_string()
}

fn strings(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|x| s(x)).collect();
    format!("[{}]", items.join(", "))
}

fn ints(v: &[i64]) -> String {
    let items: Vec<String> = v.iter().map(|x| int(*x)).collect();
    format!("[{}]", items.join(", "))
}

/// A key: bare when TOML allows it, else quoted
fn key(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        k.to_owned()
    } else {
        s(k)
    }
}

fn segments(m: &BTreeMap<String, String>) -> String {
    let items: Vec<String> = m
        .iter()
        .map(|(k, v)| format!("{} = {}", key(k), s(v)))
        .collect();
    if items.is_empty() {
        "{}".to_owned()
    } else {
        format!("{{ {} }}", items.join(", "))
    }
}

fn decode(d: &Decode) -> String {
    let mut items = vec![format!("outcome = {}", s(&d.outcome))];
    if let Some(sc) = &d.scheme {
        items.push(format!("scheme = {}", s(sc)));
    }
    items.push(format!("segments = {}", segments(&d.segments)));
    format!("{{ {} }}", items.join(", "))
}

fn fact(f: &LockedFact) -> String {
    let mut items = vec![format!("key = {}", s(&f.key))];
    match &f.value {
        FactValue::Number { value, unit } => {
            items.push(format!("value = {}", float(*value)));
            items.push(format!("unit = {}", s(unit)));
        }
        FactValue::Tolerance { minus, plus, unit } => {
            items.push(format!("minus = {}", float(*minus)));
            items.push(format!("plus = {}", float(*plus)));
            items.push(format!("unit = {}", s(unit)));
        }
        FactValue::Range { min, max, unit } => {
            items.push(format!("min = {}", float(*min)));
            items.push(format!("max = {}", float(*max)));
            items.push(format!("unit = {}", s(unit)));
        }
        FactValue::Text { text } => items.push(format!("text = {}", s(text))),
    }
    if let Some(raw) = &f.raw {
        items.push(format!("raw = {}", s(raw)));
    }
    items.push(format!("source = {}", s(&f.source)));
    items.push(format!("rank = {}", int(f.rank.into())));
    items.push(format!("at = {}", s(&f.at)));
    if f.conflict {
        items.push("conflict = true".to_owned());
    }
    format!("{{ {} }}", items.join(", "))
}

fn cover(c: &Cover) -> String {
    match c {
        Cover::Mpn { mpn } => format!("{{ kind = \"mpn\", mpn = {} }}", s(mpn)),
        Cover::Family {
            scheme,
            segments: seg,
        } => format!(
            "{{ kind = \"family\", scheme = {}, segments = {} }}",
            s(scheme),
            segments(seg)
        ),
    }
}
