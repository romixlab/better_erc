//! `berc fp check`: every footprint of a board against the footprint reviews of the board's parts snapshot. A
//! footprint is ok when a review of its hash covers every MPN of its part ([`super::pairs::judge`]); its board
//! copy is also compared with its library footprint, and its `Lib:Name` with what its symbol names.

use super::pairs::{self, Overall, Pair};
use super::{FootprintLibraries, LibCheckError, LibrarySearch, SymbolLibraries};
use erc_model::fphash;
use erc_model::layout::FootprintId;
use erc_parts::PartsLock;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What `berc fp check` found for one board footprint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FpStatus {
    /// A review of its hash covers every MPN of its part
    Ok,
    /// Some MPN is covered only by a review of another hash: the footprint changed since
    Changed,
    /// Some MPN is covered only by reviews of another version of the review hash
    OtherVersion,
    /// The parts snapshot lacks a pair of it: refresh the snapshot
    NotInLock,
    /// Some MPNs covered, others not yet
    Partial,
    /// No MPN of its part covered
    Uncovered,
    /// Its part has no MPN to review the footprint against
    NoMpn,
    /// Its part is fitted in no variant: nothing to review, not a failure
    NotFitted,
    /// No schematic symbol, and not marked board-only: its symbol is gone, or it was placed on the board alone
    NoSymbol,
    /// No schematic symbol and marked board-only (a logo, a mounting hole, mouse bites): nothing to review against,
    /// not a failure
    BoardOnly,
}

impl FpStatus {
    pub fn name(self) -> &'static str {
        match self {
            FpStatus::Ok => "ok",
            FpStatus::Changed => "changed",
            FpStatus::OtherVersion => "other version",
            FpStatus::NotInLock => "not in lock",
            FpStatus::Partial => "partial",
            FpStatus::Uncovered => "uncovered",
            FpStatus::NoMpn => "no MPN",
            FpStatus::NotFitted => "not fitted",
            FpStatus::NoSymbol => "no symbol",
            FpStatus::BoardOnly => "board-only",
        }
    }

    fn of(o: Overall) -> FpStatus {
        match o {
            Overall::Ok => FpStatus::Ok,
            Overall::Changed => FpStatus::Changed,
            Overall::OtherVersion => FpStatus::OtherVersion,
            Overall::NotInLock => FpStatus::NotInLock,
            Overall::Partial => FpStatus::Partial,
            Overall::Uncovered => FpStatus::Uncovered,
        }
    }

    /// Nothing to review here: reviewed, or nothing to review against
    pub fn is_ok(self) -> bool {
        matches!(
            self,
            FpStatus::Ok | FpStatus::BoardOnly | FpStatus::NotFitted
        )
    }

    const ALL: [FpStatus; 10] = [
        FpStatus::Ok,
        FpStatus::Changed,
        FpStatus::OtherVersion,
        FpStatus::NotInLock,
        FpStatus::Partial,
        FpStatus::Uncovered,
        FpStatus::NoMpn,
        FpStatus::NoSymbol,
        FpStatus::NotFitted,
        FpStatus::BoardOnly,
    ];
}

/// One board footprint checked
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpEntry {
    pub reference: String,
    /// The footprint's library id on the board
    pub footprint: String,
    /// The schematic symbol's library id, when it has one
    pub symbol: Option<String>,
    pub status: FpStatus,
    /// The footprint's review hash now
    pub hash: String,
    /// Each MPN of its part in every variant it is fitted in, judged against the snapshot's reviews
    pub pairs: Vec<Pair>,
    /// The library footprint this board copy is compared with: the board's `Lib:Name`, else (a board footprint
    /// without a library nickname) the symbol's `Footprint` field
    pub library_footprint: Option<String>,
    /// The review hash of that library footprint, when it was found
    pub library_footprint_hash: Option<String>,
    /// The board copy differs from its library footprint: the board was not updated from the library
    pub board_differs: bool,
    /// Why the library footprint was not found, when it was not
    pub footprint_note: Option<String>,
    /// What the symbol itself names as its `Footprint`: the library symbol's field, else the schematic's copy of
    /// it. Unlike [`library_footprint`](Self::library_footprint), never the placed footprint's own `Lib:Name`
    pub symbol_footprint: Option<String>,
    /// The board places a footprint other than the one [`symbol_footprint`](Self::symbol_footprint) names: an
    /// override, deliberate or not, shown rather than silently resolved
    pub footprint_overridden: bool,
}

impl FpEntry {
    /// Nothing to review, the board copy the same as its library footprint, and no overridden footprint
    pub fn is_ok(&self) -> bool {
        self.status.is_ok() && !self.board_differs && !self.footprint_overridden
    }
}

/// Every footprint of a board against the reviews of its parts snapshot
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpCheck {
    /// The board file
    pub board: PathBuf,
    /// The parts snapshot read
    pub lock: PathBuf,
    /// By reference, in natural order
    pub footprints: Vec<FpEntry>,
}

/// Why `berc fp check` or `todo` could not run (exit code 2)
#[derive(Debug)]
pub enum FpCheckError {
    Open(crate::CheckError),
    /// Not a KiCad project, or one without a board
    NoBoard(PathBuf),
    /// A library given by path that is none
    Libraries(LibCheckError),
    /// The project has no parts snapshot yet
    NoLock(PathBuf),
}

impl FpCheckError {
    pub const EXIT_CODE: u8 = crate::CheckError::EXIT_CODE;
}

impl fmt::Display for FpCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FpCheckError::Open(e) => e.fmt(f),
            FpCheckError::NoBoard(p) => write!(
                f,
                "{}: no KiCad board (.kicad_pcb) beside a schematic to check",
                p.display()
            ),
            FpCheckError::Libraries(e) => e.fmt(f),
            FpCheckError::NoLock(p) => write!(
                f,
                "{}: no parts snapshot, which holds the footprint reviews: write it with `berc parts refresh`",
                p.display()
            ),
        }
    }
}

impl std::error::Error for FpCheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FpCheckError::Open(e) => Some(e),
            FpCheckError::Libraries(e) => Some(e),
            FpCheckError::NoBoard(_) | FpCheckError::NoLock(_) => None,
        }
    }
}

pub(super) fn field_of<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

/// The KiCad project at `path` (a directory, its `.kicad_pro`, `.kicad_sch` or `.kicad_pcb`) with a board, its
/// parts snapshot and its libraries through `search`
pub(super) struct Opened {
    pub project: crate::Project,
    pub lock: Arc<PartsLock>,
    pub syms: SymbolLibraries,
    pub fps: FootprintLibraries,
}

impl Opened {
    pub fn open(path: &Path, search: &LibrarySearch) -> Result<Opened, FpCheckError> {
        let project = crate::Project::open(path).map_err(FpCheckError::Open)?;
        let has_layout = project.importer == "kicad"
            && project
                .imported
                .design
                .boards
                .first()
                .is_some_and(|b| b.layout.is_some());
        if !has_layout {
            return Err(FpCheckError::NoBoard(path.to_path_buf()));
        }
        let lock = project
            .parts
            .clone()
            .ok_or_else(|| FpCheckError::NoLock(project.parts_lock_path()))?;
        let (syms, fps) = search.open(&project.dir).map_err(FpCheckError::Libraries)?;
        Ok(Opened {
            project,
            lock,
            syms,
            fps,
        })
    }
}

/// `id` is a `Lib:Name`, not a bare name (a footprint placed without a library nickname, legitimate in KiCad):
/// [`library_footprint`] and [`FpEntry::footprint_overridden`] only make sense of one
pub(super) fn is_lib_name(id: &str) -> bool {
    id.split_once(':')
        .is_some_and(|(l, n)| !l.is_empty() && !n.is_empty())
}

/// The `Lib:Name` a board footprint is compared with in the footprint libraries: its own when it names a library,
/// else its symbol's `Footprint` field
fn library_footprint(board_lib_id: &str, symbol_footprint: Option<&str>) -> Option<String> {
    [Some(board_lib_id), symbol_footprint]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|id| is_lib_name(id))
        .map(String::from)
}

/// Checks every footprint of the KiCad project at `path` (a directory, its `.kicad_pro`, `.kicad_sch` or
/// `.kicad_pcb`) against the footprint reviews of its parts snapshot, and each board copy against its library
/// footprint found through `search` and the project's tables
pub fn check(path: &Path, search: &LibrarySearch) -> Result<FpCheck, FpCheckError> {
    let mut opened = Opened::open(path, search)?;
    let Opened {
        project,
        lock,
        syms,
        fps,
    } = &mut opened;
    let board = &project.imported.design.boards[0];
    let lay = board.layout.as_ref().expect("checked in open");
    let mut footprints = Vec::new();
    for (i, f) in lay.footprints.iter().enumerate() {
        let hash = fphash::hash(&lay.local_footprint(FootprintId::from_index(i)));
        let mut e = FpEntry {
            reference: f.reference.clone(),
            footprint: f.lib_id.clone(),
            symbol: None,
            status: FpStatus::NoSymbol,
            hash,
            pairs: Vec::new(),
            library_footprint: None,
            library_footprint_hash: None,
            board_differs: false,
            footprint_note: None,
            symbol_footprint: None,
            footprint_overridden: false,
        };
        let Some(part_id) = f.part else {
            if f.attrs.board_only {
                e.status = FpStatus::BoardOnly;
            }
            compare_with_library(&mut e, None, fps);
            footprints.push(e);
            continue;
        };
        let part = board.part(part_id);
        e.symbol = Some(part.lib_id.clone());
        let copy = part.symbol.map(|s| &board.symbols[s.index()]);
        let lib_fields = syms.fields(&part.lib_id).ok();
        let footprint_field = part
            .field("Footprint")
            .or_else(|| copy.and_then(|s| field_of(&s.fields, "Footprint")))
            .or_else(|| lib_fields.as_deref().and_then(|f| field_of(f, "Footprint")));
        compare_with_library(&mut e, footprint_field, fps);
        e.symbol_footprint = lib_fields
            .as_deref()
            .and_then(|f| field_of(f, "Footprint"))
            .or_else(|| copy.and_then(|s| field_of(&s.fields, "Footprint")))
            .map(String::from);
        e.footprint_overridden = is_lib_name(e.footprint.trim())
            && e.symbol_footprint
                .as_deref()
                .is_some_and(|sym| sym.trim() != e.footprint.trim());
        e.status = match erc_parts::board::part_mpns(board, part_id) {
            None => FpStatus::NotFitted,
            Some(mpns) => {
                e.pairs = mpns
                    .iter()
                    .map(|m| pairs::judge(lock, &e.hash, m))
                    .collect();
                pairs::overall(&e.pairs).map_or(FpStatus::NoMpn, FpStatus::of)
            }
        };
        footprints.push(e);
    }
    footprints.sort_by(|a, b| erc_model::natural_cmp(&a.reference, &b.reference));
    Ok(FpCheck {
        board: project.dir.join(&lay.file),
        lock: project.parts_lock_path(),
        footprints,
    })
}

/// The board copy in `e` against its library footprint, read through `fps`
fn compare_with_library(
    e: &mut FpEntry,
    symbol_footprint: Option<&str>,
    fps: &mut FootprintLibraries,
) {
    e.library_footprint = library_footprint(&e.footprint, symbol_footprint);
    let Some(id) = &e.library_footprint else {
        e.footprint_note = Some(format!(
            "footprint {} names no library, and its symbol none either",
            e.footprint
        ));
        return;
    };
    match fps.footprint(id) {
        Ok(found) => {
            let lib_hash = fphash::hash(&found.footprint);
            e.board_differs = lib_hash != e.hash;
            e.library_footprint_hash = Some(lib_hash);
        }
        Err(why) => e.footprint_note = Some(why.to_string()),
    }
}

impl FpCheck {
    /// Every footprint ok, every board copy the same as its library footprint, no override
    pub fn is_ok(&self) -> bool {
        self.footprints.iter().all(FpEntry::is_ok)
    }

    /// How many footprints have each status, the statuses that occur, in a fixed order
    pub fn counts(&self) -> Vec<(FpStatus, usize)> {
        FpStatus::ALL
            .iter()
            .map(|s| {
                (
                    *s,
                    self.footprints.iter().filter(|e| e.status == *s).count(),
                )
            })
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// One line per footprint and a summary, statuses in colour (the caller's stream drops it off a terminal);
    /// `details`: every MPN under its footprint, covered ones too, with the reviews
    pub fn to_text_color(&self, details: bool) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let dim = Style::new().dimmed();
        let width = self
            .footprints
            .iter()
            .map(|e| e.reference.len())
            .max()
            .unwrap_or(0);
        let fp_width = self
            .footprints
            .iter()
            .map(|e| e.footprint.len())
            .max()
            .unwrap_or(0);
        let mut out = String::new();
        // Board-only footprints last, under their own heading: listed, but apart from what needs a look
        let (board_only, rest): (Vec<&FpEntry>, Vec<&FpEntry>) = self
            .footprints
            .iter()
            .partition(|e| e.status == FpStatus::BoardOnly);
        let heading = format!(
            "{dim}board-only footprints without a symbol, nothing to review ({}):{dim:#}\n",
            board_only.len()
        );
        for (i, e) in rest.iter().chain(&board_only).enumerate() {
            if i == rest.len() {
                out += &heading;
            }
            let st = style(e.status);
            let _ = write!(
                out,
                "{st}{:<13}{st:#} {:<width$} {:<fp_width$}",
                e.status.name(),
                e.reference,
                e.footprint
            );
            let symbol = e.symbol.as_deref().unwrap_or("");
            match e.status {
                FpStatus::NoSymbol => {
                    let _ = write!(
                        out,
                        "  {dim}no schematic symbol, and not marked board-only{dim:#}"
                    );
                }
                FpStatus::NoMpn => {
                    let _ = write!(
                        out,
                        "  {dim}{symbol} has no Mpn field to review the footprint against{dim:#}"
                    );
                }
                FpStatus::Ok | FpStatus::BoardOnly | FpStatus::NotFitted => {}
                _ => {
                    let _ = write!(out, "  {dim}hash now{dim:#} {}", e.hash);
                }
            }
            for l in pair_notes(&e.pairs, details) {
                let _ = write!(out, "\n{:13} {:width$} {l}", "", "");
            }
            if e.board_differs {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {:width$} {warn}the board copy has {}, library footprint {} has {}: update the board \
                     from the library{warn:#}",
                    "",
                    "",
                    e.hash,
                    e.library_footprint.as_deref().unwrap_or(""),
                    e.library_footprint_hash.as_deref().unwrap_or(""),
                );
            }
            if e.footprint_overridden {
                let note = AnsiColor::Cyan.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {:width$} {note}the board places {}, symbol {symbol} sets {}: an override, shown \
                     rather than resolved silently{note:#}",
                    "",
                    "",
                    e.footprint,
                    e.symbol_footprint.as_deref().unwrap_or(""),
                );
            }
            out.push('\n');
        }
        let footprint_notes = self
            .footprints
            .iter()
            .filter_map(|e| e.footprint_note.as_deref());
        for (why, n) in once_each(footprint_notes) {
            let _ = writeln!(
                out,
                "{dim}not compared with a library footprint ({n} footprints): {why}{dim:#}"
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
            "{}: {} footprints: {}",
            self.board.display(),
            self.footprints.len(),
            counts.join(", ")
        );
        let differs = self.footprints.iter().filter(|e| e.board_differs).count();
        if differs > 0 {
            let _ = write!(
                out,
                "; {differs} board copies differ from their library footprint"
            );
        }
        let overridden = self
            .footprints
            .iter()
            .filter(|e| e.footprint_overridden)
            .count();
        if overridden > 0 {
            let _ = write!(
                out,
                "; {overridden} board copies place an overridden footprint"
            );
        }
        if self
            .footprints
            .iter()
            .any(|e| e.status == FpStatus::NotInLock)
        {
            let _ = write!(
                out,
                "; {} lacks pairs: run `berc parts refresh`",
                self.lock.display()
            );
        }
        out.push('\n');
        out
    }
}

fn style(s: FpStatus) -> anstyle::Style {
    use anstyle::{AnsiColor, Style};
    match s {
        FpStatus::Ok => AnsiColor::Green.on_default(),
        FpStatus::Changed => AnsiColor::Red.on_default().bold(),
        FpStatus::OtherVersion
        | FpStatus::NotInLock
        | FpStatus::Partial
        | FpStatus::Uncovered
        | FpStatus::NoMpn => AnsiColor::Yellow.on_default().bold(),
        FpStatus::NoSymbol => AnsiColor::Cyan.on_default(),
        FpStatus::BoardOnly | FpStatus::NotFitted => Style::new().dimmed(),
    }
}

/// The MPNs of a footprint not covered, one line each with why; with `details` the covered ones too, with their
/// reviews
pub(super) fn pair_notes(pairs: &[Pair], details: bool) -> Vec<String> {
    use super::pairs::PairStatus;
    use anstyle::{AnsiColor, Style};
    let dim = Style::new().dimmed();
    let mut out = Vec::new();
    for p in pairs {
        if p.status == PairStatus::Covered && !details {
            continue;
        }
        let st = match p.status {
            PairStatus::Covered => AnsiColor::Green.on_default(),
            PairStatus::Changed => AnsiColor::Red.on_default(),
            _ => AnsiColor::Yellow.on_default(),
        };
        let mut line = format!("{st}{}{st:#} {}", p.status.name(), p.mpn);
        if let Some(f) = &p.family {
            line += &format!(" {dim}(family {}){dim:#}", f.describe());
        }
        if !p.covered_by.is_empty() {
            let ids: Vec<String> = p.covered_by.iter().map(|i| format!("#{i}")).collect();
            line += &format!(" {dim}by review{dim:#} {}", ids.join(" "));
        }
        if !p.other.is_empty() {
            let others: Vec<String> = p
                .other
                .iter()
                .map(|o| format!("#{} {}", o.id, o.hash))
                .collect();
            line += &format!(" {dim}reviewed as{dim:#} {}", others.join(", "));
        }
        out.push(line);
    }
    out
}

/// Each distinct line once, with how often it came, sorted
fn once_each<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<(&'a str, usize)> {
    let mut out: Vec<(&str, usize)> = Vec::new();
    for l in lines {
        match out.iter_mut().find(|(w, _)| *w == l) {
            Some(n) => n.1 += 1,
            None => out.push((l, 1)),
        }
    }
    out.sort();
    out
}
