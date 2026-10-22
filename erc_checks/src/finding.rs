//! [`Finding`]: one type for what every check reports, and [`NotRun`] for a check that could not run.

use crate::ctx::Ctx;
use crate::meta::{CheckMeta, Severity};
use erc_model::{Board, Provenance, Subject, natural_cmp};
use serde::{Deserialize, Serialize};
use std::fmt;

/// How sure a finding is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// Every fact it rests on comes from the design or a stated source
    Certain,
    /// It rests on an assumed fact (a default, a designator prefix) or a reconstructed one (a PDF property bound to
    /// its part by position, nets traced from a drawing): the facts name the assumption
    Estimate,
    /// Not a rule: a list for a human or an LLM to judge, never counted as an error
    Candidate,
}

/// What a finding is about, with its display name
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubjectRef {
    pub subject: Subject,
    /// `part`, `pin`, `net`, `sheet`, `label`, `pad`, `board`
    pub kind: String,
    /// `R1`, `R1.2`, `/I2C/SDA`, `/MCU/`
    pub name: String,
}

impl fmt::Display for SubjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.kind, self.name)
    }
}

/// A fact a finding rests on, with its source (`SCL pull-up = 10 kΩ`, from the schematic)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingFact {
    pub name: String,
    pub value: String,
    pub source: Provenance,
}

/// Where to look in the design files
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    /// The file, relative to the project directory
    pub file: String,
    /// What is there (`symbol R1`, `label SDA`)
    pub what: String,
    /// The line in the file, when the importer knows it (a KiCad symbol)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Position in millimetres, when known
    pub at: Option<(f64, f64)>,
}

/// One thing a check found
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// The catalogue id (`I2C-1`)
    pub check: String,
    /// The catalogue's severity, or the project's override
    pub severity: Severity,
    pub confidence: Confidence,
    /// Board name (a design can have several)
    pub board: String,
    pub subjects: Vec<SubjectRef>,
    /// The variant it was found in, when the run was for one
    pub variant: Option<String>,
    /// The power state it was found in (NET-8)
    pub state: Option<String>,
    pub message: String,
    pub facts: Vec<FindingFact>,
    /// Check id and the sorted subject names: stable across re-annotation-free edits, what waivers and baselines
    /// key on
    pub fingerprint: String,
    /// The detail page section (`docs/checks/i2c.md#i2c-1`); the text is [`crate::explain`]
    pub explain: String,
    /// How to fix it
    pub fix: Option<String>,
    pub locations: Vec<Location>,
    /// Overlapping checks whose findings on the same subjects this one supersedes (`also RAIL-1`)
    pub also: Vec<String>,
    /// New or unchanged against the baseline, when the run was compared with one ([`crate::Report::compare`])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<BaselineState>,
}

/// A finding against a baseline report, matched by board and fingerprint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BaselineState {
    /// Not in the baseline
    New,
    /// In the baseline too
    Unchanged,
}

impl Finding {
    /// The subject names as a sorted set, for comparing findings of different checks
    pub fn subject_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.subjects.iter().map(|s| s.to_string()).collect();
        v.sort();
        v
    }
}

/// A check that did not run: which need was missing and what would provide it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotRun {
    pub check: String,
    pub need: String,
    /// Why it is missing and what would enable the check, then what the detail page says the check needs
    pub hint: String,
}

/// Builds a [`Finding`] for a check on the context's board: `cx.finding(&META).net(n).message(..).build()`
pub struct FindingBuilder<'c, 'a> {
    cx: &'c Ctx<'a>,
    meta: &'static CheckMeta,
    subjects: Vec<Subject>,
    message: String,
    facts: Vec<FindingFact>,
    fix: Option<String>,
    candidate: bool,
    estimate: bool,
}

impl<'c, 'a> FindingBuilder<'c, 'a> {
    pub(crate) fn new(cx: &'c Ctx<'a>, meta: &'static CheckMeta) -> Self {
        FindingBuilder {
            cx,
            meta,
            subjects: Vec::new(),
            message: String::new(),
            facts: Vec::new(),
            fix: None,
            candidate: false,
            estimate: false,
        }
    }

    pub fn subject(mut self, s: Subject) -> Self {
        if !self.subjects.contains(&s) {
            self.subjects.push(s);
        }
        self
    }

    pub fn part(self, p: erc_model::PartId) -> Self {
        let b = self.cx.board_id;
        self.subject(Subject::Part(b, p))
    }

    pub fn parts(self, parts: impl IntoIterator<Item = erc_model::PartId>) -> Self {
        parts.into_iter().fold(self, |f, p| f.part(p))
    }

    pub fn net(self, n: erc_model::NetId) -> Self {
        let b = self.cx.board_id;
        self.subject(Subject::Net(b, n))
    }

    pub fn nets(self, nets: impl IntoIterator<Item = erc_model::NetId>) -> Self {
        nets.into_iter().fold(self, |f, n| f.net(n))
    }

    pub fn pin(self, p: erc_model::PinId) -> Self {
        let b = self.cx.board_id;
        self.subject(Subject::Pin(b, p))
    }

    pub fn pins(self, pins: impl IntoIterator<Item = erc_model::PinId>) -> Self {
        pins.into_iter().fold(self, |f, p| f.pin(p))
    }

    pub fn message(mut self, m: impl Into<String>) -> Self {
        self.message = m.into();
        self
    }

    /// A fact the finding rests on; an assumed or reconstructed one ([`Provenance::is_estimate`]) makes the finding
    /// an estimate
    pub fn fact(
        mut self,
        name: impl Into<String>,
        value: impl fmt::Display,
        source: Provenance,
    ) -> Self {
        if source.is_estimate() {
            self.estimate = true;
        }
        self.facts.push(FindingFact {
            name: name.into(),
            value: value.to_string(),
            source,
        });
        self
    }

    pub fn fix(mut self, f: impl Into<String>) -> Self {
        self.fix = Some(f.into());
        self
    }

    /// A candidate for a human or an LLM to judge, not a rule's verdict
    pub fn candidate(mut self) -> Self {
        self.candidate = true;
        self
    }

    pub fn build(self) -> Finding {
        let board = self.cx.board();
        let mut subjects: Vec<SubjectRef> = self
            .subjects
            .iter()
            .map(|s| subject_ref(board, *s))
            .collect();
        subjects.sort_by(|a, b| {
            kind_order(&a.kind)
                .cmp(&kind_order(&b.kind))
                .then_with(|| natural_cmp(&a.name, &b.name))
        });
        let mut names: Vec<String> = subjects.iter().map(|s| s.to_string()).collect();
        names.sort();
        let fingerprint = format!("{}:{}", self.meta.id, names.join("|"));
        let mut locations: Vec<Location> = self
            .subjects
            .iter()
            .flat_map(|s| locations(board, *s))
            .collect();
        locations.sort_by(|a, b| {
            natural_cmp(&a.file, &b.file).then_with(|| natural_cmp(&a.what, &b.what))
        });
        locations.dedup();
        Finding {
            check: self.meta.id.to_string(),
            severity: self.cx.severity(self.meta),
            confidence: if self.candidate {
                Confidence::Candidate
            } else if self.estimate
                || board
                    .source
                    .connectivity
                    .as_ref()
                    .is_some_and(Provenance::is_estimate)
            {
                // A board whose nets were traced from a drawing: every finding on it is as sure as the tracing
                Confidence::Estimate
            } else {
                Confidence::Certain
            },
            board: board.name.clone(),
            subjects,
            variant: self
                .cx
                .variant
                .map(|v| board.variants[v.index()].name.clone()),
            state: None,
            message: self.message,
            facts: self.facts,
            fingerprint,
            explain: self.meta.doc_link(),
            fix: self.fix,
            locations,
            also: Vec::new(),
            baseline: None,
        }
    }
}

fn kind_order(kind: &str) -> usize {
    ["board", "sheet", "net", "part", "pin", "label", "pad"]
        .iter()
        .position(|k| *k == kind)
        .unwrap_or(usize::MAX)
}

/// The display name of a subject
pub fn subject_ref(board: &Board, s: Subject) -> SubjectRef {
    let (kind, name) = match s {
        Subject::Board(_) => ("board", board.name.clone()),
        Subject::Part(_, p) => ("part", board.part(p).designator.clone()),
        Subject::Pin(_, p) => ("pin", board.pin_name(p)),
        Subject::Net(_, n) => ("net", board.net(n).name.clone()),
        Subject::Sheet(_, s) => ("sheet", board.sheet_path(s)),
        Subject::Label(_, l) => ("label", board.label(l).text.clone()),
        Subject::Pad(_, p) => (
            "pad",
            board.layout.as_ref().map_or_else(
                || format!("#{}", p.0),
                |lay| {
                    let pad = lay.pad(p);
                    format!(
                        "{}.{}",
                        lay.footprints[pad.footprint.index()].reference,
                        pad.number
                    )
                },
            ),
        ),
    };
    SubjectRef {
        subject: s,
        kind: kind.to_string(),
        name,
    }
}

fn mm(p: erc_model::Pt) -> (f64, f64) {
    (p.x as f64 / 1e6, p.y as f64 / 1e6)
}

/// Where a subject is drawn: a part's placed units, a pin's part, a net's labels
fn locations(board: &Board, s: Subject) -> Vec<Location> {
    // A board read from a PDF: its sheets are pages of that file, the sheet files it names are not there
    let pdf = match &board.source.connectivity {
        Some(Provenance::Pdf { file, .. }) => Some(file.as_str()),
        _ => None,
    };
    let sheet_file = |s: erc_model::SheetId| match pdf {
        Some(f) => f.to_string(),
        None => board.sheet(s).file.clone(),
    };
    // ` on page 3 (MCU.SchDoc)` for a PDF, nothing otherwise
    let on_page = |s: erc_model::SheetId| {
        let sheet = board.sheet(s);
        match (pdf, &sheet.page) {
            (Some(_), Some(page)) if sheet.file.is_empty() => format!(" on page {page}"),
            (Some(_), Some(page)) => format!(" on page {page} ({})", sheet.file),
            _ => String::new(),
        }
    };
    let part_locations = |p: erc_model::PartId| {
        let part = board.part(p);
        part.units
            .iter()
            .map(|u| Location {
                file: sheet_file(u.sheet),
                what: format!("symbol {}{}", part.designator, on_page(u.sheet)),
                line: u.line,
                at: u.at.map(|a| mm(a.at)),
            })
            .collect::<Vec<_>>()
    };
    match s {
        Subject::Part(_, p) => part_locations(p),
        Subject::Pin(_, p) => part_locations(board.pin(p).part),
        Subject::Net(_, n) => board
            .net(n)
            .labels
            .iter()
            .map(|l| board.label(*l))
            .filter(|l| !l.kind.is_power())
            .take(5)
            .map(|l| Location {
                file: sheet_file(l.sheet),
                what: format!("{} label {}{}", l.kind.name(), l.text, on_page(l.sheet)),
                line: None,
                at: l.at.map(mm),
            })
            .collect(),
        Subject::Sheet(_, s) => vec![Location {
            file: sheet_file(s),
            what: format!("sheet {}{}", board.sheet_path(s), on_page(s)),
            line: None,
            at: None,
        }],
        Subject::Label(_, l) => {
            let l = board.label(l);
            vec![Location {
                file: sheet_file(l.sheet),
                what: format!("{} label {}{}", l.kind.name(), l.text, on_page(l.sheet)),
                line: None,
                at: l.at.map(mm),
            }]
        }
        Subject::Board(_) | Subject::Pad(..) => Vec::new(),
    }
}
