//! A board's todo entries by footprint hash: one review of a footprint against a datasheet covers every family and
//! MPN placed with it that the datasheet covers, so the review steps through footprints, each with what is still
//! to review and how the parts of its MPNs differ.

use super::todo::{TodoEntry, TodoStatus};
use erc_parts::lock::{FactValue, LockedPart};

/// The todo entries of one board footprint hash
#[derive(Clone, Debug, PartialEq)]
pub struct FootprintTodo {
    /// The board footprint's review hash
    pub hash: String,
    /// Its entries (a family or an MPN each), in [`super::FpTodo::entries`] order
    pub entries: Vec<TodoEntry>,
    /// How the parts of its MPNs differ, by the parts snapshot's facts
    pub facts: FactTable,
}

impl FootprintTodo {
    /// The worst status of its entries: what the footprint needs first
    pub fn status(&self) -> TodoStatus {
        self.entries
            .iter()
            .map(|e| e.status)
            .min()
            .unwrap_or(TodoStatus::Covered)
    }

    /// The first entry's `Lib:Name`
    pub fn footprint(&self) -> &str {
        self.entries.first().map_or("", |e| e.footprint.as_str())
    }

    /// Every `Lib:Name` the board places this hash as, in entry order
    pub fn footprints(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for e in &self.entries {
            if !out.contains(&e.footprint.as_str()) {
                out.push(&e.footprint);
            }
        }
        out
    }

    /// Every MPN of its entries, in entry order
    pub fn mpns(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in self.entries.iter().flat_map(|e| &e.mpns) {
            if !out.contains(m) {
                out.push(m.clone());
            }
        }
        out
    }

    /// Every board footprint of its entries, in natural order
    pub fn references(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for r in self.entries.iter().flat_map(|e| &e.references) {
            if !out.contains(r) {
                out.push(r.clone());
            }
        }
        out.sort_by(|a, b| erc_model::natural_cmp(a, b));
        out
    }

    /// The entries a review is the next step for
    pub fn to_review(&self) -> impl Iterator<Item = &TodoEntry> {
        self.entries.iter().filter(|e| e.needs_review())
    }

    /// A review is the next step for one of its entries
    pub fn needs_review(&self) -> bool {
        self.entries.iter().any(TodoEntry::needs_review)
    }

    /// Every entry reviewed
    pub fn is_ok(&self) -> bool {
        self.entries.iter().all(TodoEntry::is_ok)
    }
}

/// `entries` grouped by footprint hash: groups by their worst status, then footprint and hash; each group's entries
/// in their order in `entries`. `parts` are the parts snapshot's parts the facts are read from.
pub fn by_footprint(entries: &[TodoEntry], parts: &[LockedPart]) -> Vec<FootprintTodo> {
    let mut groups: Vec<FootprintTodo> = Vec::new();
    for e in entries {
        match groups.iter_mut().find(|g| g.hash == e.hash) {
            Some(g) => g.entries.push(e.clone()),
            None => groups.push(FootprintTodo {
                hash: e.hash.clone(),
                entries: vec![e.clone()],
                facts: FactTable::default(),
            }),
        }
    }
    for g in &mut groups {
        g.facts = FactTable::of(parts, &g.mpns());
    }
    groups.sort_by(|a, b| {
        a.status()
            .cmp(&b.status())
            .then_with(|| erc_model::natural_cmp(a.footprint(), b.footprint()))
            .then_with(|| a.hash.cmp(&b.hash))
    });
    groups
}

/// The facts of some MPNs' parts side by side: the keys whose values differ between them, each MPN's value, and
/// the keys all of them share one value of
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FactTable {
    /// The MPNs compared, in the order given
    pub mpns: Vec<String>,
    /// Keys whose values differ (or that some parts lack), with one value per MPN (none: no value)
    pub differ: Vec<(String, Vec<Option<String>>)>,
    /// Keys every part has the same value of
    pub same: Vec<(String, String)>,
}

impl FactTable {
    /// The facts of the parts of `mpns` in `parts`, by key; an MPN without a part has no values
    pub fn of(parts: &[LockedPart], mpns: &[String]) -> FactTable {
        let of: Vec<Option<&LockedPart>> = mpns
            .iter()
            .map(|m| parts.iter().find(|p| p.mpn == *m))
            .collect();
        let mut keys: Vec<&str> = of
            .iter()
            .flatten()
            .flat_map(|p| p.facts.iter().map(|f| f.key.as_str()))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        let mut table = FactTable {
            mpns: mpns.to_vec(),
            ..FactTable::default()
        };
        for key in keys {
            let facts: Vec<_> = of.iter().map(|p| p.and_then(|p| p.fact(key))).collect();
            let first = facts.first().copied().flatten();
            let all_same = first.is_some()
                && facts
                    .iter()
                    .all(|f| f.is_some_and(|f| Some(&f.value) == first.map(|g| &g.value)));
            match (all_same, first) {
                (true, Some(f)) => table
                    .same
                    .push((key.to_owned(), shown(&f.value, f.raw.as_deref()))),
                _ => table.differ.push((
                    key.to_owned(),
                    facts
                        .iter()
                        .map(|f| f.map(|f| shown(&f.value, f.raw.as_deref())))
                        .collect(),
                )),
            }
        }
        table
    }

    /// No facts to show
    pub fn is_empty(&self) -> bool {
        self.differ.is_empty() && self.same.is_empty()
    }
}

/// A fact's value for the person: the source's own text where it kept one, else the value and its unit
fn shown(v: &FactValue, raw: Option<&str>) -> String {
    if let Some(r) = raw.map(str::trim).filter(|r| !r.is_empty()) {
        return r.to_owned();
    }
    match v {
        FactValue::Number { value, unit } => format!("{value} {unit}"),
        FactValue::Tolerance { minus, plus, unit } => format!("-{minus}/+{plus} {unit}"),
        FactValue::Range { min, max, unit } => format!("{min} to {max} {unit}"),
        FactValue::Text { text } => text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erc_parts::lock::{LockedFact, PartMatch};

    fn part(mpn: &str, facts: &[(&str, &str)]) -> LockedPart {
        LockedPart {
            mpn: mpn.to_owned(),
            matched: PartMatch::default(),
            part: None,
            decode: None,
            missing: Vec::new(),
            facts: facts
                .iter()
                .map(|(k, v)| LockedFact {
                    key: (*k).to_owned(),
                    value: FactValue::Text {
                        text: (*v).to_owned(),
                    },
                    raw: None,
                    source: "manual".into(),
                    rank: 100,
                    at: String::new(),
                    conflict: false,
                })
                .collect(),
        }
    }

    /// Keys of one value are said once; a key whose values differ, or that one part lacks, has a value per MPN
    #[test]
    fn facts_differ_and_same() {
        let parts = [
            part("A-1", &[("case_eia", "0402"), ("resistance", "10k")]),
            part("A-2", &[("case_eia", "0402"), ("resistance", "1k")]),
            part(
                "A-3",
                &[
                    ("case_eia", "0402"),
                    ("resistance", "1k"),
                    ("power", "0.1W"),
                ],
            ),
        ];
        let mpns: Vec<String> = ["A-1", "A-2", "A-3"].map(String::from).to_vec();
        let t = FactTable::of(&parts, &mpns);
        assert_eq!(t.same, [("case_eia".to_owned(), "0402".to_owned())]);
        assert_eq!(
            t.differ,
            [
                (
                    "power".to_owned(),
                    vec![None, None, Some("0.1W".to_owned())]
                ),
                (
                    "resistance".to_owned(),
                    vec![
                        Some("10k".to_owned()),
                        Some("1k".to_owned()),
                        Some("1k".to_owned())
                    ]
                ),
            ]
        );
    }
}
