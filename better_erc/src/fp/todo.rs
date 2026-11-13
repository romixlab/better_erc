//! `berc fp todo`: the library symbols a board's footprints come from that need a footprint review, in the order a
//! review steps through them. The review itself is of the library: each symbol against the library footprint its
//! `Footprint` field names, as `berc fp check --lib` checks it.

use super::library::check_symbol;
use super::{FIELD, FpCheckError, LibrarySearch, Opened, SymbolStatus, is_lib_name};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One library symbol a board uses
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TodoEntry {
    /// `Lib:Name`, as the schematic names it
    pub symbol: String,
    pub status: SymbolStatus,
    /// The library symbol's `Footprint` field, when it sets one
    pub footprint: Option<String>,
    /// The `.kicad_mod` read
    pub footprint_file: Option<PathBuf>,
    /// The library footprint's review hash now, when it was found: what the symbol's `Footprint_Review` gets once
    /// it is reviewed
    pub hash: Option<String>,
    /// The review hash on the library symbol: its first stale review line's, else its first line's
    pub stored: Option<String>,
    /// The library symbol's review lines, each against the footprint, and its MPNs no line reviews
    #[serde(default)]
    pub reviews: super::record::Assessed,
    /// The library symbol's `Datasheet` field, what the footprint is reviewed against
    pub datasheet: Option<String>,
    /// The board footprints placed from this symbol, in natural order
    pub references: Vec<String>,
    /// References placed with a footprint other than [`footprint`](Self::footprint), the symbol's own: an
    /// override, deliberate or not, shown rather than silently resolved (reference, the footprint placed)
    #[serde(default)]
    pub overridden: Vec<(String, String)>,
    /// The symbol's own library (the nickname of `symbol`), when `allowed` named some and it is none of them
    #[serde(default)]
    pub symbol_library_disallowed: Option<String>,
    /// The library [`footprint`](Self::footprint) names, when `allowed` named some and it is none of them
    #[serde(default)]
    pub footprint_library_disallowed: Option<String>,
    /// Why the library symbol or its footprint could not be read
    pub note: Option<String>,
}

impl TodoEntry {
    /// The footprint needs a review: never reviewed, changed since, or a hash berc can't compare
    pub fn needs_review(&self) -> bool {
        self.status.needs_review()
    }

    /// Nothing to do: reviewed and unchanged, or no footprint to review
    pub fn is_ok(&self) -> bool {
        matches!(self.status, SymbolStatus::Ok | SymbolStatus::NoFootprint)
    }

    /// A board places a footprint other than what this symbol itself names, for one of its references
    pub fn is_overridden(&self) -> bool {
        !self.overridden.is_empty()
    }

    /// The footprint placed, when every overridden reference agrees on one; `None` when there is none to
    /// override, or they disagree among themselves
    pub fn overridden_footprint(&self) -> Option<&str> {
        match self.overridden.as_slice() {
            [] => None,
            [first, rest @ ..] => rest
                .iter()
                .all(|(_, p)| p == &first.1)
                .then_some(first.1.as_str()),
        }
    }

    /// Nothing outside what is allowed
    pub fn is_allowed(&self) -> bool {
        self.symbol_library_disallowed.is_none() && self.footprint_library_disallowed.is_none()
    }
}

/// Where a status goes in the list: what needs a review first (changed ones before those never reviewed), then
/// what can't be reviewed, then what is done
fn rank(s: SymbolStatus) -> u8 {
    match s {
        SymbolStatus::Changed => 0,
        SymbolStatus::OtherVersion => 1,
        SymbolStatus::Malformed => 2,
        SymbolStatus::Unreviewed => 3,
        SymbolStatus::Partial => 4,
        SymbolStatus::Missing => 5,
        SymbolStatus::Refused => 6,
        SymbolStatus::Ok => 7,
        SymbolStatus::NoFootprint => 8,
    }
}

/// The library symbols of a board's footprints and what each needs
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpTodo {
    /// The board file
    pub board: PathBuf,
    /// The symbol field read ([`FIELD`])
    pub field: String,
    /// Every library symbol a board footprint comes from, once each: those that need a review first, in the order
    /// to review them ([`FpTodo::to_review`]), then those that can't be reviewed, then the ok ones, then those
    /// without a footprint; by symbol in natural order within each status
    pub symbols: Vec<TodoEntry>,
    /// Every file this was made from, sorted: the project's files read (its schematics, its board, its
    /// `.kicad_pro`), the library tables looked at, the symbol libraries and the footprint files used or looked for
    /// in vain. When one of them changes the list is out of date (the review tab watches them)
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

/// The library symbols the footprints of the KiCad project at `path` come from (a directory, its `.kicad_pro`,
/// `.kicad_sch` or `.kicad_pcb`), each checked against its library footprint, the libraries found through
/// `search` and the project's tables
pub fn todo(path: &Path, search: &LibrarySearch) -> Result<FpTodo, FpCheckError> {
    let mut opened = Opened::open(path, search)?;
    let Opened { project, syms, fps } = &mut opened;
    let board = &project.imported.design.boards[0];
    let lay = board.layout.as_ref().expect("checked in open");
    // Library id -> (reference, the footprint placed), in the board's order
    let mut used: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for f in &lay.footprints {
        let Some(part) = f.part.map(|p| board.part(p)) else {
            continue;
        };
        let placed = (f.reference.clone(), f.lib_id.clone());
        match used.iter_mut().find(|(id, _)| *id == part.lib_id) {
            Some((_, refs)) => refs.push(placed),
            None => used.push((part.lib_id.clone(), vec![placed])),
        }
    }
    let mut symbols: Vec<TodoEntry> = used
        .into_iter()
        .map(|(lib_id, mut placed)| {
            placed.sort_by(|a, b| erc_model::natural_cmp(&a.0, &b.0));
            placed.dedup();
            let e = check_symbol(&lib_id, syms.fields(&lib_id), fps, &search.allowed);
            let overridden: Vec<(String, String)> = match &e.footprint {
                Some(sym_fp) => placed
                    .iter()
                    .filter(|(_, fp)| is_lib_name(fp.trim()) && fp.trim() != sym_fp.trim())
                    .cloned()
                    .collect(),
                None => Vec::new(),
            };
            let references = placed.into_iter().map(|(r, _)| r).collect();
            TodoEntry {
                symbol: e.symbol,
                status: e.status,
                footprint: e.footprint,
                footprint_file: e.footprint_file,
                hash: e.hash,
                stored: e.stored,
                reviews: e.reviews,
                datasheet: e.datasheet,
                references,
                overridden,
                symbol_library_disallowed: e.symbol_library_disallowed,
                footprint_library_disallowed: e.footprint_library_disallowed,
                note: e.note,
            }
        })
        .collect();
    symbols.sort_by(|a, b| {
        rank(a.status)
            .cmp(&rank(b.status))
            .then_with(|| erc_model::natural_cmp(&a.symbol, &b.symbol))
    });
    let mut files: Vec<PathBuf> = board
        .source
        .files
        .iter()
        .map(|f| project.dir.join(f))
        .chain([project.dir.join(&lay.file)])
        .chain(syms.files())
        .chain(fps.files())
        .collect();
    files.sort();
    files.dedup();
    Ok(FpTodo {
        board: project.dir.join(&lay.file),
        field: FIELD.to_string(),
        symbols,
        files,
    })
}

impl FpTodo {
    /// The symbols whose footprints need a review, in the order to review them
    pub fn to_review(&self) -> impl Iterator<Item = &TodoEntry> {
        self.symbols.iter().filter(|e| e.needs_review())
    }

    /// Nothing to review and nothing that can't be, no overridden footprint and no library outside what is
    /// allowed
    pub fn is_ok(&self) -> bool {
        self.symbols
            .iter()
            .all(|e| e.is_ok() && !e.is_overridden() && e.is_allowed())
    }

    /// How many symbols have each status, the statuses that occur, in list order
    pub fn counts(&self) -> Vec<(SymbolStatus, usize)> {
        let mut out: Vec<(SymbolStatus, usize)> = Vec::new();
        for e in &self.symbols {
            match out.iter_mut().find(|(s, _)| *s == e.status) {
                Some(n) => n.1 += 1,
                None => out.push((e.status, 1)),
            }
        }
        out
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// One line per symbol to review, then those that can't be under their own heading, then (with `all`) the ok
    /// ones and those without a footprint, else their count and how to list them; statuses in colour (the caller's
    /// stream drops it off a terminal)
    pub fn to_text_color(&self, all: bool) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let style = |s: SymbolStatus| match s {
            SymbolStatus::Ok => AnsiColor::Green.on_default(),
            SymbolStatus::Changed | SymbolStatus::Malformed | SymbolStatus::Missing => {
                AnsiColor::Red.on_default().bold()
            }
            SymbolStatus::Unreviewed | SymbolStatus::OtherVersion | SymbolStatus::Partial => {
                AnsiColor::Yellow.on_default().bold()
            }
            SymbolStatus::Refused => AnsiColor::Cyan.on_default(),
            SymbolStatus::NoFootprint => Style::new().dimmed(),
        };
        let dim = Style::new().dimmed();
        let shown: Vec<&TodoEntry> = self
            .symbols
            .iter()
            .filter(|e| all || !e.is_ok() || e.is_overridden() || !e.is_allowed())
            .collect();
        let width = shown.iter().map(|e| e.symbol.len()).max().unwrap_or(0);
        let fp_width = shown
            .iter()
            .filter_map(|e| e.footprint.as_ref().map(String::len))
            .max()
            .unwrap_or(0);
        let mut out = String::new();
        let blocked = shown
            .iter()
            .filter(|e| !e.needs_review() && !e.is_ok())
            .count();
        let done = self.symbols.iter().filter(|e| e.is_ok()).count();
        let mut heading_for = None;
        for e in &shown {
            let group = if e.needs_review() {
                0
            } else if e.is_ok() {
                2
            } else {
                1
            };
            if heading_for != Some(group) {
                heading_for = Some(group);
                match group {
                    1 => {
                        let _ = writeln!(
                            out,
                            "{dim}can't be reviewed until fixed ({blocked}):{dim:#}"
                        );
                    }
                    2 => {
                        let _ = writeln!(
                            out,
                            "{dim}reviewed, or no footprint to review ({done}):{dim:#}"
                        );
                    }
                    _ => {}
                }
            }
            let st = style(e.status);
            let fp = e.footprint.as_deref().unwrap_or("");
            let _ = write!(
                out,
                "{st}{:<13}{st:#} {:<width$} {fp:<fp_width$}",
                e.status.name(),
                e.symbol,
            );
            let stored = e.stored.as_deref().unwrap_or("");
            let now = e.hash.as_deref().unwrap_or("");
            match e.status {
                SymbolStatus::Ok | SymbolStatus::NoFootprint | SymbolStatus::Partial => {}
                SymbolStatus::Changed => {
                    let _ = write!(
                        out,
                        "  {dim}reviewed{dim:#} {stored}  {dim}now{dim:#} {now}"
                    );
                }
                SymbolStatus::Unreviewed => {
                    let _ = write!(out, "  {dim}hash now{dim:#} {now}");
                }
                SymbolStatus::Missing => {
                    let _ = write!(out, "  {dim}{}{dim:#}", e.note.as_deref().unwrap_or(""));
                }
                SymbolStatus::Refused => {
                    let _ = write!(
                        out,
                        "  {dim}{FIELD} is{dim:#} {stored}  {dim}but the library symbol sets no footprint{dim:#}"
                    );
                }
                SymbolStatus::OtherVersion | SymbolStatus::Malformed => {
                    let _ = write!(
                        out,
                        "  {dim}{FIELD} is{dim:#} {stored}  {dim}now{dim:#} {now}"
                    );
                }
            }
            let _ = write!(out, "  {dim}on{dim:#} {}", e.references.join(" "));
            for l in super::review_notes(&e.reviews, false) {
                let _ = write!(out, "\n{:13} {:width$} {l}", "", "");
            }
            if !e.overridden.is_empty() {
                let note = AnsiColor::Cyan.on_default();
                let placed: Vec<String> = e
                    .overridden
                    .iter()
                    .map(|(r, fp)| format!("{r} ({fp})"))
                    .collect();
                let _ = write!(
                    out,
                    "\n{:13} {:width$} {note}placed as {}: review follows the board, not the symbol's own \
                     Footprint{note:#}",
                    "",
                    "",
                    placed.join(", "),
                );
            }
            if let Some(lib) = &e.symbol_library_disallowed {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {:width$} {warn}its own library {lib} is not in the allowed libraries{warn:#}",
                    "", "",
                );
            }
            if let Some(lib) = &e.footprint_library_disallowed {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {:width$} {warn}its footprint library {lib} is not in the allowed libraries{warn:#}",
                    "", "",
                );
            }
            out.push('\n');
        }
        if !all && done > 0 {
            let _ = writeln!(
                out,
                "{dim}{done} reviewed or without a footprint to review, not listed: --all lists them{dim:#}"
            );
        }
        let counts: Vec<String> = self
            .counts()
            .iter()
            .map(|(s, n)| {
                let st = style(*s);
                format!("{st}{n} {}{st:#}", s.name())
            })
            .collect();
        let _ = write!(
            out,
            "{}: {} library symbols, {} to review: {}",
            self.board.display(),
            self.symbols.len(),
            self.to_review().count(),
            if counts.is_empty() {
                "none".to_string()
            } else {
                counts.join(", ")
            }
        );
        let overridden = self.symbols.iter().filter(|e| e.is_overridden()).count();
        if overridden > 0 {
            let _ = write!(out, "; {overridden} with an overridden footprint");
        }
        let disallowed = self.symbols.iter().filter(|e| !e.is_allowed()).count();
        if disallowed > 0 {
            let _ = write!(out, "; {disallowed} outside the allowed libraries");
        }
        out.push('\n');
        out
    }
}
