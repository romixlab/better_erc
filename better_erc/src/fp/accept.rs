//! `berc fp accept`: a footprint review recorded in a part source ([`Source::accept`]) for the board footprints
//! reviewed, then the project's parts snapshot refreshed from it so `berc fp check` sees it. Nothing is written into
//! the schematic or the libraries, except a symbol's `Footprint` when asked ([`set_symbol_footprint`]).

use super::reviewed::{Resolver, ReviewedAs};
use super::{LibrarySearch, SymbolLibraries};
use crate::parts::{PartsRefreshError, Refreshed, WriteLock, refresh_project};
use erc_model::fphash;
use erc_model::layout::{Footprint, FootprintId};
use erc_parts::lock::{Cover, LockedReview, PartKey, PartMatch};
use erc_parts::source::{MPNS_PER_CALL_MAX, NewReview, Source, SourceError};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// What was reviewed: board footprints, or a footprint by hash
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcceptWhat {
    /// Board footprints by reference: the review hash of their library footprint (one for all of them) and their
    /// parts' MPNs
    References(Vec<String>),
    /// A footprint by its review hash (a library footprint before a board uses it) and its `Lib:Name`
    Hash { hash: String, footprint: String },
}

/// Which MPNs a review covers and how. Both lists empty: every MPN of the references, each by its family where its
/// MPN decodes into one ([`Cover::family_of`]), else alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Covers {
    /// These MPNs, each alone (one part)
    pub alone: Vec<String>,
    /// The families of these MPNs
    pub family_of: Vec<String>,
}

impl Covers {
    fn is_all(&self) -> bool {
        self.alone.is_empty() && self.family_of.is_empty()
    }
}

/// What [`accept`] did, or would do with `dry_run`
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AcceptRun {
    pub dry_run: bool,
    /// The part source written to
    pub source: String,
    /// The footprint's review hash
    pub hash: String,
    /// Its `Lib:Name`
    pub footprint: String,
    /// The board footprints reviewed, empty for a hash given by hand
    pub references: Vec<String>,
    /// What the review covers
    pub covers: Vec<Cover>,
    /// MPNs of the references left out: the source has no single part of them, and they decode into no family
    pub skipped: Vec<String>,
    /// The review as the source stored it; `None` on a dry run
    pub review: Option<LockedReview>,
    /// The parts snapshot refreshed after it; `None` on a dry run or without a project
    pub lock: Option<Refreshed>,
}

/// Why `berc fp accept` recorded nothing (exit code 2)
#[derive(Debug)]
pub enum AcceptError {
    Open(crate::CheckError),
    /// A footprint or symbol library given by path that is none
    Libraries(super::LibCheckError),
    /// References to accept, but no board to take their footprints from
    NoBoard(PathBuf),
    NoFootprint(String),
    /// The footprint has no schematic symbol, so no MPN to review against
    NoSymbol(String),
    /// Its part has no MPN in any variant it is fitted in
    NoMpn(String),
    /// Footprints with different review hashes: one review is of one footprint
    Conflict(Vec<(String, String)>),
    /// Not a review hash of this version (`fp1:` and 16 hex digits)
    BadHash(String),
    /// An MPN to cover that no reference carries
    NotOnFootprint(String),
    /// An MPN whose family was asked for that decodes into none
    NoFamily(String),
    /// Nothing left to cover
    NothingToCover,
    /// The source gave no answer or refused the review
    Source(SourceError),
    /// The review was recorded, the parts snapshot not refreshed
    Refresh {
        id: i64,
        error: PartsRefreshError,
    },
    /// A symbol's `Footprint` could not be written
    Symbol(SymbolEditError),
}

impl AcceptError {
    pub const EXIT_CODE: u8 = crate::CheckError::EXIT_CODE;
}

impl fmt::Display for AcceptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcceptError::Open(e) => e.fmt(f),
            AcceptError::Libraries(e) => e.fmt(f),
            AcceptError::NoBoard(p) => write!(
                f,
                "{}: no KiCad board (.kicad_pcb) to take the footprints from",
                p.display()
            ),
            AcceptError::NoFootprint(r) => write!(f, "no footprint {r} on the board"),
            AcceptError::NoSymbol(r) => write!(
                f,
                "footprint {r} has no schematic symbol, so no MPN to review it against"
            ),
            AcceptError::NoMpn(r) => {
                write!(f, "{r} has no Mpn field to review its footprint against")
            }
            AcceptError::Conflict(hashes) => {
                let list: Vec<String> = hashes.iter().map(|(r, h)| format!("{r} {h}")).collect();
                write!(
                    f,
                    "the footprints differ ({}): one review is of one footprint, accept them apart",
                    list.join(", ")
                )
            }
            AcceptError::BadHash(h) => write!(
                f,
                "`{h}` is no review hash (`{}:` and 16 hex digits)",
                fphash::VERSION
            ),
            AcceptError::NotOnFootprint(m) => {
                write!(f, "no footprint accepted carries MPN {m}")
            }
            AcceptError::NoFamily(m) => write!(
                f,
                "MPN {m} decodes into no family with a package segment: cover it alone (--mpn)"
            ),
            AcceptError::NothingToCover => write!(
                f,
                "nothing to cover: the part source has none of the MPNs, and none decodes into a family"
            ),
            AcceptError::Source(e) => e.fmt(f),
            AcceptError::Refresh { id, error } => write!(
                f,
                "review #{id} recorded, but the parts snapshot was not refreshed: {error}"
            ),
            AcceptError::Symbol(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for AcceptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AcceptError::Open(e) => Some(e),
            AcceptError::Libraries(e) => Some(e),
            AcceptError::Source(e) => Some(e),
            AcceptError::Refresh { error, .. } => Some(error),
            AcceptError::Symbol(e) => Some(e),
            _ => None,
        }
    }
}

/// Today (UTC) as reviews date it: `2026-11-25`
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    date_of(secs)
}

/// The UTC day of Unix time `secs`: `2026-11-25`
pub fn date_of(secs: i64) -> String {
    let t = erc_model::fact::Time(secs).to_string();
    t.get(..10).unwrap_or(&t).to_owned()
}

/// The board footprints `refs` of the KiCad project at `path`: the one review hash and `Lib:Name` of the footprint
/// their review is of (their library footprint found through `search`, else the board copy as placed,
/// [`super::reviewed`]) and the MPNs of their parts, in reference order
fn board_footprints(
    path: &Path,
    refs: &[String],
    search: &LibrarySearch,
) -> Result<(String, String, Vec<String>), AcceptError> {
    let project = crate::Project::open_without_parts(path).map_err(AcceptError::Open)?;
    let (mut syms, mut fps) = search.open(&project.dir).map_err(AcceptError::Libraries)?;
    let mut resolver = Resolver {
        syms: &mut syms,
        fps: &mut fps,
    };
    let no_board = || AcceptError::NoBoard(path.to_path_buf());
    let board = project
        .imported
        .design
        .boards
        .first()
        .ok_or_else(no_board)?;
    let lay = board.layout.as_ref().ok_or_else(no_board)?;
    let mut hashes: Vec<(String, String)> = Vec::new();
    let mut lib_id = String::new();
    let mut mpns: Vec<String> = Vec::new();
    for r in refs {
        let i = lay
            .footprints
            .iter()
            .position(|f| &f.reference == r)
            .ok_or_else(|| AcceptError::NoFootprint(r.clone()))?;
        let f: &Footprint = &lay.footprints[i];
        let part = f.part.ok_or_else(|| AcceptError::NoSymbol(r.clone()))?;
        let part_mpns = erc_parts::board::part_mpns(board, part).unwrap_or_default();
        if part_mpns.is_empty() {
            return Err(AcceptError::NoMpn(r.clone()));
        }
        for m in part_mpns {
            if !mpns.contains(&m) {
                mpns.push(m);
            }
        }
        let reviewed = resolver.resolve(board, FootprintId::from_index(i));
        if lib_id.is_empty() {
            lib_id = match (reviewed.reviewed_as, reviewed.library_footprint) {
                (ReviewedAs::Library, Some(l)) => l,
                _ => f.lib_id.clone(),
            };
        }
        hashes.push((r.clone(), reviewed.hash));
    }
    let Some((_, hash)) = hashes.first().cloned() else {
        return Err(AcceptError::NothingToCover);
    };
    if hashes.iter().any(|(_, h)| *h != hash) {
        return Err(AcceptError::Conflict(hashes));
    }
    Ok((hash, lib_id, mpns))
}

/// Records a review of `what` in `source`: `review` gives where it was made and who made it (its hash, footprint
/// and covers are set here), `covers` which MPNs it covers. For board footprints the MPNs must be theirs, the
/// review is of their library footprint found through `search`, and the parts snapshot of the project at
/// `project` is refreshed from `source` afterwards. `dry_run` records nothing and says what would be.
pub fn accept(
    project: Option<&Path>,
    search: &LibrarySearch,
    what: &AcceptWhat,
    source: &dyn Source,
    review: &NewReview,
    covers: &Covers,
    dry_run: bool,
) -> Result<AcceptRun, AcceptError> {
    let (hash, footprint, board_mpns, references) = match what {
        AcceptWhat::References(refs) => {
            let path = project.ok_or_else(|| AcceptError::NoBoard(PathBuf::from(".")))?;
            let (hash, lib_id, mpns) = board_footprints(path, refs, search)?;
            (hash, lib_id, Some(mpns), refs.clone())
        }
        AcceptWhat::Hash { hash, footprint } => {
            if !fphash::is_hash(hash.trim()) {
                return Err(AcceptError::BadHash(hash.clone()));
            }
            (hash.trim().to_owned(), footprint.clone(), None, Vec::new())
        }
    };
    if let Some(on_board) = &board_mpns
        && let Some(m) = covers
            .alone
            .iter()
            .chain(&covers.family_of)
            .find(|m| !on_board.contains(m))
    {
        return Err(AcceptError::NotOnFootprint(m.clone()));
    }
    let wanted: Vec<String> = if covers.is_all() {
        board_mpns.unwrap_or_default()
    } else {
        covers
            .alone
            .iter()
            .chain(&covers.family_of)
            .cloned()
            .collect()
    };
    let mut parts = Vec::new();
    for chunk in wanted.chunks(MPNS_PER_CALL_MAX) {
        let keys: Vec<PartKey> = chunk.iter().map(|m| PartKey::mpn(m)).collect();
        parts.extend(source.parts(&keys, &[]).map_err(AcceptError::Source)?.parts);
    }
    let part = |m: &str| parts.iter().find(|p| p.mpn == m);
    let mut list: Vec<Cover> = Vec::new();
    let mut skipped = Vec::new();
    let mut push = |c: Cover| {
        if !list.contains(&c) {
            list.push(c);
        }
    };
    if covers.is_all() {
        for m in &wanted {
            match part(m).and_then(Cover::family_of) {
                Some(f) => push(f),
                None if part(m).is_some_and(|p| p.matched == PartMatch::Found) => {
                    push(Cover::Mpn { mpn: m.clone() });
                }
                None => skipped.push(m.clone()),
            }
        }
    } else {
        for m in &covers.alone {
            push(Cover::Mpn { mpn: m.clone() });
        }
        for m in &covers.family_of {
            push(
                part(m)
                    .and_then(Cover::family_of)
                    .ok_or_else(|| AcceptError::NoFamily(m.clone()))?,
            );
        }
    }
    if list.is_empty() {
        return Err(AcceptError::NothingToCover);
    }
    let mut run = AcceptRun {
        dry_run,
        source: source.name(),
        hash: hash.clone(),
        footprint: footprint.clone(),
        references,
        covers: list.clone(),
        skipped,
        review: None,
        lock: None,
    };
    if dry_run {
        return Ok(run);
    }
    let stored = source
        .accept(&NewReview {
            hash,
            footprint,
            covers: list,
            ..review.clone()
        })
        .map_err(AcceptError::Source)?;
    let id = stored.id;
    run.review = Some(stored);
    if let (Some(p), AcceptWhat::References(_)) = (project, what) {
        run.lock = Some(
            refresh_project(p, search, source, WriteLock::Changed)
                .map_err(|error| AcceptError::Refresh { id, error })?,
        );
    }
    Ok(run)
}

impl AcceptRun {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// The review, what it covers, and the snapshot written
    pub fn to_text_color(&self) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let dim = Style::new().dimmed();
        let green = AnsiColor::Green.on_default();
        let mut out = String::new();
        let refs = if self.references.is_empty() {
            String::new()
        } else {
            format!(" {dim}on{dim:#} {}", self.references.join(" "))
        };
        let id = match &self.review {
            Some(r) => format!("review #{} ", r.id),
            None => "would record a review ".to_owned(),
        };
        let _ = writeln!(
            out,
            "{id}{dim}in{dim:#} {}: {} {green}{}{green:#}{refs}",
            self.source, self.footprint, self.hash
        );
        for c in &self.covers {
            let what = match c {
                Cover::Mpn { .. } => "MPN",
                Cover::Family { .. } => "family",
            };
            let _ = writeln!(out, "  {dim}covers {what}{dim:#} {}", c.describe());
        }
        if !self.skipped.is_empty() {
            let warn = AnsiColor::Yellow.on_default();
            let _ = writeln!(
                out,
                "  {warn}left out, the part source has no part of: {}{warn:#}",
                self.skipped.join(", ")
            );
        }
        if let Some(l) = &self.lock {
            out += "  ";
            out += &l.to_text();
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------------------------
// A symbol's Footprint set to the footprint placed

/// One file a symbol's `Footprint` is written into
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Written {
    pub file: PathBuf,
    /// The library's `.kicad_sym` (`true`), or a schematic sheet's copy of the symbol
    pub library: bool,
    /// The symbol's name in the file: `Name` in a library, `Lib:Name` in a schematic
    pub symbol: String,
    /// The value before, `None` when the symbol did not have it
    pub old: Option<String>,
    /// The file changes
    pub changed: bool,
}

/// Why a symbol's `Footprint` was not written
#[derive(Debug)]
pub enum SymbolEditError {
    /// Neither the library symbol nor a schematic copy of it was found
    Nowhere {
        symbol: String,
        why: String,
    },
    Read(PathBuf, std::io::Error),
    Edit(PathBuf, erc_kicad::symlib::EditError),
    Write(PathBuf, std::io::Error),
}

impl fmt::Display for SymbolEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolEditError::Nowhere { symbol, why } => write!(
                f,
                "symbol {symbol} is neither in a library found ({why}) nor in a schematic sheet's lib_symbols"
            ),
            SymbolEditError::Read(p, e) => write!(f, "can't read {}: {e}", p.display()),
            SymbolEditError::Edit(p, e) => write!(f, "{}: {e}", p.display()),
            SymbolEditError::Write(p, e) => write!(f, "can't write {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for SymbolEditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SymbolEditError::Read(_, e) | SymbolEditError::Write(_, e) => Some(e),
            SymbolEditError::Edit(_, e) => Some(e),
            SymbolEditError::Nowhere { .. } => None,
        }
    }
}

/// Sets library symbol `symbol`'s (`Lib:Name`) `Footprint` property to `footprint` in its library found through
/// `libs` and in every copy of it in `sheets`, through the formatting-preserving writer: the fix for a board that
/// places another footprint than its symbol names, when the board is right. Every edit is made before any file is
/// written; `dry_run` writes nothing.
pub fn set_symbol_footprint(
    libs: &mut SymbolLibraries,
    sheets: &[PathBuf],
    symbol: &str,
    footprint: &str,
    dry_run: bool,
) -> Result<Vec<Written>, SymbolEditError> {
    use erc_kicad::symlib::{EditError, set_property};
    let mut texts: Vec<(PathBuf, String, String)> = Vec::new();
    let mut written = Vec::new();
    let lib_file = libs.symbol_file(symbol);
    let mut targets: Vec<(PathBuf, String, bool)> = Vec::new();
    let mut why = String::new();
    match &lib_file {
        Ok((file, name)) => targets.push((file.clone(), name.clone(), true)),
        Err(e) => why = e.to_string(),
    }
    targets.extend(sheets.iter().map(|s| (s.clone(), symbol.to_owned(), false)));
    for (file, name, library) in targets {
        let i = match texts.iter().position(|t| t.0 == file) {
            Some(i) => i,
            None => {
                let text = std::fs::read_to_string(&file)
                    .map_err(|e| SymbolEditError::Read(file.clone(), e))?;
                texts.push((file.clone(), text.clone(), text));
                texts.len() - 1
            }
        };
        let text = &mut texts[i].2;
        let e = match set_property(text, &name, "Footprint", footprint) {
            Ok(e) => e,
            Err(EditError::NoSymbol(_)) => continue,
            Err(e) => return Err(SymbolEditError::Edit(file, e)),
        };
        *text = e.text;
        written.push(Written {
            file,
            library,
            symbol: name,
            old: e.old,
            changed: e.changed,
        });
    }
    if written.is_empty() {
        return Err(SymbolEditError::Nowhere {
            symbol: symbol.to_owned(),
            why,
        });
    }
    if !dry_run {
        for (file, before, after) in &texts {
            if before != after {
                erc_parts::lock::write_atomic(file, after)
                    .map_err(|e| SymbolEditError::Write(file.clone(), e))?;
            }
        }
    }
    Ok(written)
}

/// The schematic sheet files of the project at `path`, each once, and the symbols (`Lib:Name`) of board footprints
/// `refs`
pub fn sheets_and_symbols(
    path: &Path,
    refs: &[String],
) -> Result<(Vec<PathBuf>, Vec<String>), AcceptError> {
    let project = crate::Project::open_without_parts(path).map_err(AcceptError::Open)?;
    let mut sheets: Vec<PathBuf> = Vec::new();
    for b in &project.imported.design.boards {
        for s in &b.sheets {
            let p = project.dir.join(&s.file);
            if !sheets.contains(&p) {
                sheets.push(p);
            }
        }
    }
    let mut symbols = Vec::new();
    if let Some(board) = project.imported.design.boards.first()
        && let Some(lay) = &board.layout
    {
        for r in refs {
            let f = lay
                .footprints
                .iter()
                .find(|f| &f.reference == r)
                .ok_or_else(|| AcceptError::NoFootprint(r.clone()))?;
            let part = f.part.ok_or_else(|| AcceptError::NoSymbol(r.clone()))?;
            let id = board.part(part).lib_id.clone();
            if !symbols.contains(&id) {
                symbols.push(id);
            }
        }
    }
    Ok((sheets, symbols))
}
