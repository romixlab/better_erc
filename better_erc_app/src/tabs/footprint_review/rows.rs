//! The review list's rows: on a board one per board footprint hash and what one review of it covers (a family, else
//! one MPN), as `berc fp todo` lists them; in a symbol library one per library symbol, as `berc fp check --lib`
//! judges it. Each row knows the footprint to show and the reviews of the part source that name it.

use super::*;
use better_erc::fp::pairs::PairStatus;
use better_erc::parts::lock::{Cover, LockedReview};
use ve_theme::Tone;

/// One row of the review list
#[derive(Clone, Debug)]
pub(super) enum Row {
    /// A board footprint hash and its cover
    Board(Box<TodoEntry>),
    /// A library symbol
    Library(Box<SymbolEntry>),
}

/// A review of the part source as one row sees it
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Recorded {
    pub review: LockedReview,
    /// The row's MPNs it covers
    pub mpns: Vec<String>,
    /// Made for the footprint's hash now; else for an earlier one, the footprint changed since
    pub current: bool,
}

impl Row {
    /// What finds the row again after the list is read anew: the symbol, or the footprint, its cover and first
    /// reference
    pub fn key(&self) -> String {
        match self {
            Row::Board(e) => format!(
                "{} {} {}",
                e.footprint,
                self.cover_label(),
                e.references.first().map_or("", String::as_str)
            ),
            Row::Library(e) => e.symbol.clone(),
        }
    }

    /// The row's title: the footprint and what a review covers, or the library symbol
    pub fn title(&self) -> String {
        match self {
            Row::Board(e) => format!("{} · {}", e.footprint, self.cover_label()),
            Row::Library(e) => e.symbol.clone(),
        }
    }

    /// What a review of a board row covers, for the person
    fn cover_label(&self) -> String {
        match self {
            Row::Board(e) => e
                .cover
                .as_ref()
                .map_or_else(|| "no MPN".to_owned(), Cover::describe),
            Row::Library(_) => String::new(),
        }
    }

    pub fn status_name(&self) -> &'static str {
        match self {
            Row::Board(e) => e.status.name(),
            Row::Library(e) => e.status.name(),
        }
    }

    /// What the status means, as a tone (the word says it, the tone backs it up)
    pub fn tone(&self) -> Tone {
        match self {
            Row::Board(e) => match e.status {
                TodoStatus::Covered => Tone::Good,
                TodoStatus::Changed | TodoStatus::NoMpn => Tone::Crit,
                TodoStatus::OtherVersion | TodoStatus::Uncovered => Tone::Warn,
                TodoStatus::NotInLock | TodoStatus::UnknownMpn => Tone::Accent,
            },
            Row::Library(e) => match e.status {
                SymbolStatus::Ok => Tone::Good,
                SymbolStatus::Changed | SymbolStatus::Missing => Tone::Crit,
                SymbolStatus::OtherVersion | SymbolStatus::Partial | SymbolStatus::Uncovered => {
                    Tone::Warn
                }
                SymbolStatus::Unjudged => Tone::Accent,
                SymbolStatus::NoMpn | SymbolStatus::NoFootprint => Tone::Off,
            },
        }
    }

    /// A review is the next step
    pub fn needs_review(&self) -> bool {
        match self {
            Row::Board(e) => e.needs_review(),
            Row::Library(e) => e.status.needs_review(),
        }
    }

    /// Nothing open: reviewed, or nothing to review
    pub fn is_done(&self) -> bool {
        match self {
            Row::Board(e) => e.is_ok(),
            Row::Library(e) => e.status.is_ok(),
        }
    }

    /// The symbol (`Lib:Name`): the library symbol, or the first reference's on a board
    pub fn symbol(&self) -> Option<&str> {
        match self {
            Row::Board(e) => e.symbol.as_deref(),
            Row::Library(e) => Some(&e.symbol),
        }
    }

    /// The footprint's `Lib:Name`: placed on the board, or the symbol's `Footprint` field
    pub fn footprint(&self) -> Option<&str> {
        match self {
            Row::Board(e) => Some(&e.footprint),
            Row::Library(e) => e.footprint.as_deref(),
        }
    }

    /// The review hash a review of the row is made for: the board footprint's, or the library footprint's
    pub fn hash(&self) -> Option<&str> {
        match self {
            Row::Board(e) => Some(&e.hash),
            Row::Library(e) => e.hash.as_deref(),
        }
    }

    pub fn datasheet(&self) -> Option<&str> {
        match self {
            Row::Board(e) => e.datasheet.as_deref(),
            Row::Library(e) => e.datasheet.as_deref(),
        }
    }

    /// The MPNs a review of the row is for
    pub fn mpns(&self) -> &[String] {
        match self {
            Row::Board(e) => &e.mpns,
            Row::Library(e) => &e.mpns,
        }
    }

    /// The board footprints of a board row, in natural order; none for a library row
    pub fn references(&self) -> &[String] {
        match self {
            Row::Board(e) => &e.references,
            Row::Library(_) => &[],
        }
    }

    /// The family `mpn` is in, when its MPN decodes into one: what one review covers for every part of it
    pub fn family_of(&self, mpn: &str) -> Option<Cover> {
        match self {
            Row::Board(e) => e
                .cover
                .clone()
                .filter(|c| matches!(c, Cover::Family { .. }) && e.mpns.iter().any(|m| m == mpn)),
            Row::Library(e) => e
                .pairs
                .iter()
                .find(|p| p.mpn == mpn)
                .and_then(|p| p.family.clone()),
        }
    }

    /// The footprint the board places for every reference instead of its symbol's own `Footprint`, when it is the
    /// same for all of them: the symbol's `Footprint` can be set to it on Accept
    pub fn overridden_footprint(&self) -> Option<&str> {
        let Row::Board(e) = self else { return None };
        (e.overridden.len() == e.references.len() && !e.overridden.is_empty())
            .then_some(e.footprint.as_str())
    }

    /// The symbol's own `Footprint` where the board places another
    pub fn symbol_footprint(&self) -> Option<&str> {
        match self {
            Row::Board(e) => e.overridden.first().map(|(_, f)| f.as_str()),
            Row::Library(e) => e.footprint.as_deref(),
        }
    }

    pub fn disallowed(&self) -> impl Iterator<Item = &String> {
        let (a, b) = match self {
            Row::Board(e) => (
                &e.symbol_library_disallowed,
                &e.footprint_library_disallowed,
            ),
            Row::Library(e) => (
                &e.symbol_library_disallowed,
                &e.footprint_library_disallowed,
            ),
        };
        a.iter().chain(b)
    }

    /// Why the row's footprint or symbol could not be read, or what it says of the library footprint
    pub fn note(&self) -> Option<String> {
        match self {
            Row::Board(e) => match (&e.library_hash, &e.library_file) {
                (Some(h), _) if *h != e.hash => Some(format!(
                    "the board's copy is not its library's footprint now ({h}): update the board from the library, or \
                     review the copy as placed"
                )),
                (None, None) if e.footprint.contains(':') => Some(format!(
                    "{} is in no footprint library found: the board's copy is reviewed",
                    e.footprint
                )),
                _ => None,
            },
            Row::Library(e) => e.note.clone(),
        }
    }

    /// Text the list filter matches: the title, the symbol, the footprint, the references and the MPNs
    pub fn matches(&self, filter: &str) -> bool {
        let hit = |t: &str| t.to_lowercase().contains(filter);
        filter.is_empty()
            || hit(&self.title())
            || self.symbol().is_some_and(hit)
            || self.footprint().is_some_and(hit)
            || self.references().iter().any(|r| hit(r))
            || self.mpns().iter().any(|m| hit(m))
    }

    /// The reviews of `reviews` that name the row: those of its hash now that cover its MPNs, and those of
    /// earlier hashes
    pub fn recorded(&self, reviews: &[LockedReview]) -> Vec<Recorded> {
        let mut out: Vec<Recorded> = Vec::new();
        let mut add = |id: i64, mpns: &[String], current: bool| {
            let Some(r) = reviews.iter().find(|r| r.id == id) else {
                return;
            };
            match out.iter_mut().find(|o| o.review.id == id) {
                Some(o) => {
                    for m in mpns {
                        if !o.mpns.contains(m) {
                            o.mpns.push(m.clone());
                        }
                    }
                }
                None => out.push(Recorded {
                    review: r.clone(),
                    mpns: mpns.to_vec(),
                    current,
                }),
            }
        };
        match self {
            Row::Board(e) => {
                for id in &e.covered_by {
                    add(*id, &e.mpns, true);
                }
                for o in &e.other {
                    add(o.id, &e.mpns, false);
                }
            }
            Row::Library(e) => {
                for p in &e.pairs {
                    let one = std::slice::from_ref(&p.mpn);
                    for id in &p.covered_by {
                        add(*id, one, true);
                    }
                    for o in &p.other {
                        add(o.id, one, false);
                    }
                }
            }
        }
        out
    }

    /// The MPNs of the row the part source has no part of
    pub fn unknown_mpns(&self) -> Vec<String> {
        match self {
            Row::Board(e) if e.status == TodoStatus::UnknownMpn => e.mpns.clone(),
            Row::Board(_) => Vec::new(),
            Row::Library(e) => e
                .pairs
                .iter()
                .filter(|p| p.status == PairStatus::UnknownMpn)
                .map(|p| p.mpn.clone())
                .collect(),
        }
    }
}

/// What a review was made against, for the person: the document and revision, else the PDF, and the page
pub(super) fn review_source(r: &LockedReview) -> String {
    let what = match (&r.doc, &r.doc_rev, &r.pdf_blake3) {
        (Some(d), Some(rev), _) => format!("{d} {rev}"),
        (Some(d), None, _) => d.clone(),
        (None, _, Some(p)) => format!("PDF b3:{}", p.get(..16).unwrap_or(p)),
        (None, _, None) => "no document recorded".to_owned(),
    };
    match r.page {
        Some(p) => format!("#{} {what}, page {p}", r.id),
        None => format!("#{} {what}", r.id),
    }
}
