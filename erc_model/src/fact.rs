//! Facts with provenance: a value is never stored bare. Every attribute the model holds (a field, a part value, a
//! DNP flag, a pin's electrical type, a net class) is a [`Fact`] that says where it came from, how much that
//! source is trusted ([`Rank`]), when it was known and why. Several sources can state the same attribute; a
//! [`Facts`] keeps all of them, and [`Facts::resolve`] picks the one a check sees: the highest rank, the newest
//! on a tie. The resolved fact still carries its source, so a finding can say "10 kΩ (schematic power.kicad_sch)".

use serde::{Deserialize, Serialize};
use std::fmt;

/// Where a fact came from
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Provenance {
    /// A KiCad schematic sheet file, path relative to the project directory
    Schematic { file: String },
    /// A netlist file (KiCad `.net`, or another tool's netlist)
    Netlist { file: String },
    /// A board file (`.kicad_pcb`)
    Board { file: String },
    /// The KiCad project file (`.kicad_pro`)
    KicadProject { file: String },
    /// The better_erc project file, with the key the fact was read from
    ProjectFile { file: String, key: String },
    /// An entry of the part database
    PartDb { file: String, entry: String },
    /// mx3, the lab's part store, by its part id
    Mx3 { part: String },
    /// The part vendor's own data (tables, configurators)
    Vendor { name: String },
    /// A datasheet read by a person or by a reviewed extraction
    Datasheet { doc: String },
    /// A distributor (Digi-Key, Mouser, LCSC)
    Distributor { name: String },
    /// An LLM reading a datasheet, not yet reviewed by a person
    Llm { model: String },
    /// A person typed or confirmed it
    Manual { who: String },
    /// Computed by an analysis from other facts, by the named rule
    Derived { rule: String },
    /// Nothing better is known: a default or a generic profile, by the named rule
    Assumed { rule: String },
}

impl Provenance {
    /// The rank a fact from this source gets unless the user ordered sources otherwise
    pub fn default_rank(&self) -> Rank {
        match self {
            Provenance::Manual { .. } => Rank::MANUAL,
            Provenance::ProjectFile { .. } => Rank::PROJECT,
            Provenance::Schematic { .. }
            | Provenance::Netlist { .. }
            | Provenance::Board { .. }
            | Provenance::KicadProject { .. } => Rank::DESIGN,
            Provenance::Vendor { .. } => Rank::VENDOR,
            Provenance::Mx3 { .. } | Provenance::PartDb { .. } | Provenance::Datasheet { .. } => {
                Rank::DATASHEET
            }
            Provenance::Distributor { .. } => Rank::DISTRIBUTOR,
            Provenance::Llm { .. } => Rank::LLM,
            Provenance::Derived { .. } => Rank::DERIVED,
            Provenance::Assumed { .. } => Rank::ASSUMED,
        }
    }

    /// Short name of the kind of source, as the dump writes it
    pub fn kind(&self) -> &'static str {
        match self {
            Provenance::Schematic { .. } => "sch",
            Provenance::Netlist { .. } => "net",
            Provenance::Board { .. } => "pcb",
            Provenance::KicadProject { .. } => "pro",
            Provenance::ProjectFile { .. } => "project",
            Provenance::PartDb { .. } => "partdb",
            Provenance::Mx3 { .. } => "mx3",
            Provenance::Vendor { .. } => "vendor",
            Provenance::Datasheet { .. } => "datasheet",
            Provenance::Distributor { .. } => "distributor",
            Provenance::Llm { .. } => "llm",
            Provenance::Manual { .. } => "manual",
            Provenance::Derived { .. } => "derived",
            Provenance::Assumed { .. } => "assumed",
        }
    }

    /// The source's arguments in order (file, key, ...), as the dump writes them
    pub fn args(&self) -> Vec<&str> {
        match self {
            Provenance::Schematic { file }
            | Provenance::Netlist { file }
            | Provenance::Board { file }
            | Provenance::KicadProject { file } => vec![file],
            Provenance::ProjectFile { file, key } => vec![file, key],
            Provenance::PartDb { file, entry } => vec![file, entry],
            Provenance::Mx3 { part } => vec![part],
            Provenance::Vendor { name } | Provenance::Distributor { name } => vec![name],
            Provenance::Datasheet { doc } => vec![doc],
            Provenance::Llm { model } => vec![model],
            Provenance::Manual { who } => vec![who],
            Provenance::Derived { rule } | Provenance::Assumed { rule } => vec![rule],
        }
    }

    /// The inverse of [`Provenance::kind`] and [`Provenance::args`]
    pub fn from_parts(kind: &str, args: &[String]) -> Option<Provenance> {
        let a = |i: usize| args.get(i).cloned();
        let n = args.len();
        let one = n == 1;
        let two = n == 2;
        Some(match kind {
            "sch" if one => Provenance::Schematic { file: a(0)? },
            "net" if one => Provenance::Netlist { file: a(0)? },
            "pcb" if one => Provenance::Board { file: a(0)? },
            "pro" if one => Provenance::KicadProject { file: a(0)? },
            "project" if two => Provenance::ProjectFile {
                file: a(0)?,
                key: a(1)?,
            },
            "partdb" if two => Provenance::PartDb {
                file: a(0)?,
                entry: a(1)?,
            },
            "mx3" if one => Provenance::Mx3 { part: a(0)? },
            "vendor" if one => Provenance::Vendor { name: a(0)? },
            "datasheet" if one => Provenance::Datasheet { doc: a(0)? },
            "distributor" if one => Provenance::Distributor { name: a(0)? },
            "llm" if one => Provenance::Llm { model: a(0)? },
            "manual" if one => Provenance::Manual { who: a(0)? },
            "derived" if one => Provenance::Derived { rule: a(0)? },
            "assumed" if one => Provenance::Assumed { rule: a(0)? },
            _ => return None,
        })
    }
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind())?;
        for a in self.args() {
            write!(f, " {a}")?;
        }
        Ok(())
    }
}

/// How much a source is trusted: higher wins. The constants are the defaults; a user can reorder sources per
/// source, part or parameter (a person's value can be lowered below a supplier's without deleting it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Rank(pub u16);

impl Rank {
    /// A person typed or confirmed it
    pub const MANUAL: Rank = Rank(1000);
    /// The better_erc project file
    pub const PROJECT: Rank = Rank(900);
    /// The design files themselves (schematic, board, KiCad project, netlist)
    pub const DESIGN: Rank = Rank(800);
    /// The vendor's own data
    pub const VENDOR: Rank = Rank(700);
    /// Datasheet-derived (a person, a reviewed extraction, the part database, mx3)
    pub const DATASHEET: Rank = Rank(600);
    /// Distributors
    pub const DISTRIBUTOR: Rank = Rank(500);
    /// An unreviewed LLM extraction
    pub const LLM: Rank = Rank(300);
    /// Derived by an analysis
    pub const DERIVED: Rank = Rank(200);
    /// Generic profiles and defaults
    pub const ASSUMED: Rank = Rank(100);
}

/// A point in time, seconds since 1970-01-01 UTC. The dump writes it as `2026-10-22T09:30:00Z`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Time(pub i64);

impl Time {
    /// From a calendar date and time in UTC
    pub fn from_utc(y: i64, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> Time {
        Time(days_from_civil(y, mo, d) * 86400 + (h * 3600 + mi * 60 + s) as i64)
    }

    /// Parses `2026-10-22T09:30:00Z` or `2026-10-22`
    pub fn parse(text: &str) -> Option<Time> {
        let (date, time) = match text.split_once('T') {
            Some((d, t)) => (d, Some(t.strip_suffix('Z')?)),
            None => (text, None),
        };
        let mut d = date.splitn(3, '-');
        let y: i64 = d.next()?.parse().ok()?;
        let mo: u32 = d.next()?.parse().ok()?;
        let day: u32 = d.next()?.parse().ok()?;
        if !(1..=12).contains(&mo) || !(1..=31).contains(&day) {
            return None;
        }
        let (h, mi, s) = match time {
            Some(t) => {
                let mut t = t.splitn(3, ':');
                let h: u32 = t.next()?.parse().ok()?;
                let mi: u32 = t.next()?.parse().ok()?;
                let s: u32 = t.next()?.parse().ok()?;
                if h > 23 || mi > 59 || s > 60 {
                    return None;
                }
                (h, mi, s)
            }
            None => (0, 0, 0),
        };
        Some(Time::from_utc(y, mo, day, h, mi, s))
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.0.div_euclid(86400);
        let secs = self.0.rem_euclid(86400);
        let (y, m, d) = civil_from_days(days);
        write!(
            f,
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            secs / 3600,
            secs / 60 % 60,
            secs % 60
        )
    }
}

// Howard Hinnant's civil calendar algorithms (public domain)
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// One statement of one attribute by one source
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fact<T> {
    pub value: T,
    pub source: Provenance,
    pub rank: Rank,
    /// When the source stated it; `None` for facts read from design files (their time is the file's commit)
    pub at: Option<Time>,
    /// Why, or anything a reader of the finding should know (`"from the Value field"`)
    pub note: Option<String>,
}

impl<T> Fact<T> {
    /// A fact with its source's default rank, no time and no note
    pub fn new(value: T, source: Provenance) -> Fact<T> {
        let rank = source.default_rank();
        Fact {
            value,
            source,
            rank,
            at: None,
            note: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Fact<T> {
        self.note = Some(note.into());
        self
    }

    pub fn with_time(mut self, at: Time) -> Fact<T> {
        self.at = Some(at);
        self
    }

    pub fn with_rank(mut self, rank: Rank) -> Fact<T> {
        self.rank = rank;
        self
    }

    /// The same fact with its value mapped (a parsed form of a text fact keeps the text's source)
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Fact<U> {
        Fact {
            value: f(self.value),
            source: self.source,
            rank: self.rank,
            at: self.at,
            note: self.note,
        }
    }

    /// Whether `self` wins over `other`: the higher rank, then the newer
    fn beats(&self, other: &Fact<T>) -> bool {
        (self.rank, self.at) > (other.rank, other.at)
    }
}

/// Every fact known for one attribute, one per source. A source stating it again replaces its own fact only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Facts<T>(pub Vec<Fact<T>>);

impl<T> Default for Facts<T> {
    fn default() -> Self {
        Facts(Vec::new())
    }
}

impl<T: PartialEq> Facts<T> {
    /// One fact
    pub fn one(fact: Fact<T>) -> Facts<T> {
        Facts(vec![fact])
    }

    /// Adds a fact; a fact from the same source replaces the earlier one from it
    pub fn set(&mut self, fact: Fact<T>) {
        if let Some(old) = self.0.iter_mut().find(|f| f.source == fact.source) {
            *old = fact;
        } else {
            self.0.push(fact);
        }
    }

    /// The fact a check sees: the highest rank, the newest on a tie (the first in canonical order, by source, on
    /// a full tie)
    pub fn resolve(&self) -> Option<&Fact<T>> {
        let mut best: Option<&Fact<T>> = None;
        for f in &self.0 {
            if best.is_none_or(|b| f.beats(b)) {
                best = Some(f);
            }
        }
        best
    }

    /// The resolved value
    pub fn get(&self) -> Option<&T> {
        self.resolve().map(|f| &f.value)
    }

    /// Facts whose value differs from the resolved one, highest rank first: a disagreement between sources, which
    /// a check can report instead of silently choosing
    pub fn disagreements(&self) -> Vec<&Fact<T>> {
        let Some(best) = self.resolve() else {
            return Vec::new();
        };
        let mut out: Vec<&Fact<T>> = self.0.iter().filter(|f| f.value != best.value).collect();
        out.sort_by_key(|f| std::cmp::Reverse((f.rank, f.at)));
        out
    }
}

impl<T> Facts<T> {
    /// Sorts the facts by source, rank, time and note: the canonical order
    pub fn canonicalize(&mut self) {
        self.0.sort_by(|a, b| {
            (&a.source, a.rank, a.at, &a.note).cmp(&(&b.source, b.rank, b.at, &b.note))
        });
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Fact<T>> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sch() -> Provenance {
        Provenance::Schematic {
            file: "a.kicad_sch".into(),
        }
    }

    #[test]
    fn highest_rank_wins_then_newest() {
        let mut f = Facts::one(Fact::new(
            "13k3",
            Provenance::Distributor {
                name: "Digi-Key".into(),
            },
        ));
        f.set(Fact::new(
            "13k7",
            Provenance::Manual {
                who: "Roman".into(),
            },
        ));
        assert_eq!(f.get(), Some(&"13k7"));
        assert_eq!(f.disagreements().len(), 1);
        assert_eq!(f.disagreements()[0].value, "13k3");

        let mut g = Facts::one(Fact::new(1, sch()).with_time(Time(10)).with_rank(Rank(5)));
        g.set(
            Fact::new(2, Provenance::Derived { rule: "x".into() })
                .with_time(Time(20))
                .with_rank(Rank(5)),
        );
        assert_eq!(g.get(), Some(&2));
        g.set(Fact::new(3, sch()).with_time(Time(30)).with_rank(Rank(5)));
        assert_eq!(
            g.get(),
            Some(&3),
            "a source restating replaces its own fact"
        );
        assert_eq!(g.0.len(), 2);
    }

    #[test]
    fn time_round_trip() {
        let t = Time::from_utc(2026, 10, 22, 9, 30, 5);
        assert_eq!(t.to_string(), "2026-10-22T09:30:05Z");
        assert_eq!(Time::parse("2026-10-22T09:30:05Z"), Some(t));
        assert_eq!(Time::parse("1970-01-01"), Some(Time(0)));
        assert_eq!(Time::parse("1969-12-31T23:59:59Z"), Some(Time(-1)));
        assert_eq!(Time(-1).to_string(), "1969-12-31T23:59:59Z");
        assert_eq!(Time::parse("2026-13-01"), None);
    }

    #[test]
    fn provenance_parts_round_trip() {
        for p in [
            sch(),
            Provenance::ProjectFile {
                file: "b.berc.toml".into(),
                key: "nets.+5V.voltage".into(),
            },
            Provenance::Assumed {
                rule: "generic-cmos".into(),
            },
        ] {
            let args: Vec<String> = p.args().iter().map(|s| s.to_string()).collect();
            assert_eq!(Provenance::from_parts(p.kind(), &args), Some(p));
        }
    }
}
