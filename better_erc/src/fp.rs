//! Footprint review on disk, the library side of `berc fp`: the review hash ([`erc_model::fphash`]) of a library
//! footprint file or of a footprint as placed on a board ([`hash_file`]), read through one [`FootprintSource`].

use erc_kicad::footprint;
use erc_model::layout::{FootprintId, Layout};
use erc_model::{ImportError, LocalFootprint, fphash};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

mod library;
mod todo;
pub use library::{
    CheckedLibrary, LibCheck, LibCheckError, LibCheckOptions, LibrarySearch, SymbolEntry,
    SymbolStatus, check_libraries,
};
pub use todo::{FpTodo, TodoEntry, todo};

/// A footprint's review hash and the canonical text it is taken over
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hashed {
    /// The file read: a `.kicad_mod` or a `.kicad_pcb`
    pub file: PathBuf,
    /// The board footprint's reference, for a board
    pub reference: Option<String>,
    /// The footprint's library id (`Lib:Name` on a board, the footprint's name in a library file)
    pub lib_id: String,
    /// `fp1:` and 16 hex digits
    pub hash: String,
    /// What the hash covers, one sorted line per item
    pub canonical: Vec<String>,
}

impl Hashed {
    fn of(file: &Path, reference: Option<String>, f: &LocalFootprint) -> Hashed {
        Hashed {
            file: file.to_path_buf(),
            reference,
            lib_id: f.footprint.lib_id.clone(),
            hash: fphash::hash(f),
            canonical: fphash::canonical(f),
        }
    }
}

/// Why a footprint could not be read (exit code 2)
#[derive(Debug)]
pub enum FpError {
    Read(PathBuf, std::io::Error),
    Import(ImportError),
    /// A board needs the reference of the footprint to hash
    NeedReference(PathBuf),
    /// No footprint with that reference on the board
    NoFootprint {
        board: PathBuf,
        reference: String,
    },
    /// A reference given for a library footprint file, which has only one footprint
    ReferenceOnLibrary(PathBuf),
    /// Neither a `.kicad_mod` nor a `.kicad_pcb`
    NotFootprint(PathBuf),
    /// A footprint named by its `Lib:Name` that the footprint library tables don't give
    Library(String, erc_kicad::libtable::LibError),
}

impl FpError {
    pub const EXIT_CODE: u8 = 2;
}

impl fmt::Display for FpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FpError::Read(p, e) => write!(f, "can't read {}: {e}", p.display()),
            FpError::Import(e) => e.fmt(f),
            FpError::NeedReference(p) => write!(
                f,
                "{} is a board: name the footprint by its reference (`berc fp hash {} U1`)",
                p.display(),
                p.display()
            ),
            FpError::NoFootprint { board, reference } => {
                write!(f, "no footprint {reference} on {}", board.display())
            }
            FpError::ReferenceOnLibrary(p) => write!(
                f,
                "{} is a library footprint, one footprint without a reference: leave the reference out",
                p.display()
            ),
            FpError::NotFootprint(p) => write!(
                f,
                "{}: neither a library footprint (.kicad_mod) nor a board (.kicad_pcb)",
                p.display()
            ),
            FpError::Library(lib_id, e) => write!(f, "footprint {lib_id}: {e}"),
        }
    }
}

impl std::error::Error for FpError {}

fn read_text(path: &Path) -> Result<String, FpError> {
    std::fs::read_to_string(path).map_err(|e| FpError::Read(path.to_path_buf(), e))
}

/// A board file read alone, without its schematic
pub fn read_board(path: &Path) -> Result<Layout, FpError> {
    let text = read_text(path)?;
    let (lay, _) = erc_kicad::pcb::read(&text, &path.to_string_lossy()).map_err(FpError::Import)?;
    Ok(lay)
}

/// A library footprint file in its own frame
pub fn read_library(path: &Path) -> Result<LocalFootprint, FpError> {
    let text = read_text(path)?;
    let (f, _) = footprint::read(&text, &path.to_string_lossy()).map_err(FpError::Import)?;
    Ok(f)
}

/// The review hash of a library footprint file (`reference` `None`), or of the footprint `reference` on a board
pub fn hash_file(path: &Path, reference: Option<&str>) -> Result<Hashed, FpError> {
    let source = FootprintSource::from_path(path, reference)?;
    let s = source.read()?;
    Ok(Hashed::of(path, reference.map(String::from), &s.footprint))
}

/// Where a footprint to review comes from. Every way of getting a footprint goes through [`FootprintSource::read`],
/// so a review does not care which one it was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FootprintSource {
    /// A library footprint file (`.kicad_mod`)
    File(PathBuf),
    /// The copy of a library footprint placed on a board, by its reference
    Board { board: PathBuf, reference: String },
    /// A library footprint by its `Lib:Name`, found through the footprint library tables (`fp-lib-table`) of the
    /// project in `project` and the global one in KiCad's configuration directory `config`
    /// ([`kicad_config_dir`])
    Library {
        lib_id: String,
        project: PathBuf,
        config: Option<PathBuf>,
    },
}

/// A footprint read from its [`FootprintSource`], in its own frame, with its review hash
#[derive(Clone, Debug, PartialEq)]
pub struct Sourced {
    pub source: FootprintSource,
    pub footprint: LocalFootprint,
    /// `fp1:` and 16 hex digits
    pub hash: String,
    /// The footprint's fields (`Datasheet`, the `Footprint_Review` a board copy took over from its symbol, ...),
    /// those with a value
    pub fields: Vec<(String, String)>,
}

impl Sourced {
    fn new(source: FootprintSource, footprint: LocalFootprint) -> Sourced {
        let fields = footprint
            .footprint
            .fields
            .iter()
            .filter_map(|f| Some((f.name.clone(), f.value.get()?.clone())))
            .filter(|(_, v)| !v.trim().is_empty())
            .collect();
        Sourced {
            source,
            hash: fphash::hash(&footprint),
            footprint,
            fields,
        }
    }

    /// Footprint `i` of a board already read from `board`
    pub fn from_board(board: &Path, lay: &Layout, i: FootprintId) -> Sourced {
        let source = FootprintSource::Board {
            board: board.to_path_buf(),
            reference: lay.footprints[i.index()].reference.clone(),
        };
        Sourced::new(source, lay.local_footprint(i))
    }

    /// A field's value, when the footprint has it
    pub fn field(&self, name: &str) -> Option<&str> {
        field_of(&self.fields, name)
    }

    /// The footprint's library id: `Lib:Name` for a board copy, the footprint's name for a library file
    pub fn lib_id(&self) -> &str {
        &self.footprint.footprint.lib_id
    }
}

impl FootprintSource {
    /// A `.kicad_mod` file (no reference), or the footprint `reference` on a `.kicad_pcb`
    pub fn from_path(path: &Path, reference: Option<&str>) -> Result<FootprintSource, FpError> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match (ext, reference) {
            ("kicad_mod", None) => Ok(FootprintSource::File(path.to_path_buf())),
            ("kicad_mod", Some(_)) => Err(FpError::ReferenceOnLibrary(path.to_path_buf())),
            ("kicad_pcb", None) => Err(FpError::NeedReference(path.to_path_buf())),
            ("kicad_pcb", Some(r)) => Ok(FootprintSource::Board {
                board: path.to_path_buf(),
                reference: r.to_string(),
            }),
            _ => Err(FpError::NotFootprint(path.to_path_buf())),
        }
    }

    /// Reads the footprint and takes its review hash
    pub fn read(&self) -> Result<Sourced, FpError> {
        match self {
            FootprintSource::File(path) => Ok(Sourced::new(self.clone(), read_library(path)?)),
            FootprintSource::Board { board, reference } => {
                let lay = read_board(board)?;
                let i = lay
                    .footprints
                    .iter()
                    .position(|f| &f.reference == reference)
                    .ok_or_else(|| FpError::NoFootprint {
                        board: board.clone(),
                        reference: reference.clone(),
                    })?;
                Ok(Sourced::from_board(board, &lay, FootprintId::from_index(i)))
            }
            FootprintSource::Library {
                lib_id,
                project,
                config,
            } => {
                let mut libs =
                    erc_kicad::libtable::FootprintLibraries::new(project, config.as_deref());
                let found = libs
                    .footprint(lib_id)
                    .map_err(|e| FpError::Library(lib_id.clone(), e))?;
                Ok(Sourced::new(self.clone(), found.footprint.clone()))
            }
        }
    }

    /// A short line for the person: the file, or the board and reference, or the library id
    pub fn describe(&self) -> String {
        match self {
            FootprintSource::File(p) => p.display().to_string(),
            FootprintSource::Board { board, reference } => {
                format!("{reference} on {}", board.display())
            }
            FootprintSource::Library { lib_id, .. } => format!("{lib_id} (library)"),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// berc fp check

/// The library symbol field that holds a footprint's review hash
pub const FIELD: &str = "Footprint_Review";

pub use erc_kicad::libtable::{FootprintLibraries, SymbolLibraries, kicad_config_dir};

/// What `berc fp check` found for one board footprint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FpStatus {
    /// The review hash on its symbol matches
    Ok,
    /// The footprint changed since the review
    Changed,
    /// Its symbol has no review hash
    Unreviewed,
    /// Its symbol sets no footprint, so a review of this one means nothing
    Refused,
    /// No schematic symbol, and not marked board-only: its symbol is gone, or it was placed on the board alone
    NoSymbol,
    /// No schematic symbol and marked board-only (a logo, a mounting hole, mouse bites): nothing to review against,
    /// not a failure
    BoardOnly,
    /// The hash was made by another version of berc's review hash
    OtherVersion,
    /// The field holds something that is no review hash
    Malformed,
}

impl FpStatus {
    pub fn name(self) -> &'static str {
        match self {
            FpStatus::Ok => "ok",
            FpStatus::Changed => "changed",
            FpStatus::Unreviewed => "unreviewed",
            FpStatus::Refused => "refused",
            FpStatus::NoSymbol => "no symbol",
            FpStatus::BoardOnly => "board-only",
            FpStatus::OtherVersion => "other version",
            FpStatus::Malformed => "malformed",
        }
    }

    const ALL: [FpStatus; 8] = [
        FpStatus::Ok,
        FpStatus::Changed,
        FpStatus::Unreviewed,
        FpStatus::Refused,
        FpStatus::NoSymbol,
        FpStatus::OtherVersion,
        FpStatus::Malformed,
        FpStatus::BoardOnly,
    ];
}

/// Where the stored review hash was read
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HashSource {
    /// The library symbol in its `.kicad_sym`, found through the library tables
    Library,
    /// The schematic's copy of the library symbol (`lib_symbols`): the library was not found
    Schematic,
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
    /// The review hash on the symbol, from [`source`](Self::source)
    pub stored: Option<String>,
    pub source: Option<HashSource>,
    /// The hash in the library symbol, when the library was found
    pub library_hash: Option<String>,
    /// The hash in the schematic's copy of the library symbol
    pub schematic_hash: Option<String>,
    /// The library and the schematic's copy hold different hashes: the schematic was not updated from the library
    pub copy_differs: bool,
    /// Why the library symbol was not found, when it was not
    pub library_note: Option<String>,
    /// The library footprint this board copy is compared with: the board's `Lib:Name`, else (a board footprint
    /// without a library nickname) the symbol's `Footprint` field
    pub library_footprint: Option<String>,
    /// The review hash of that library footprint, when it was found
    pub library_footprint_hash: Option<String>,
    /// The board copy differs from its library footprint: the board was not updated from the library
    pub board_differs: bool,
    /// Why the library footprint was not found, when it was not
    pub footprint_note: Option<String>,
}

impl FpEntry {
    /// Ok, with the schematic's copy agreeing with the library and the board copy with its library footprint, or
    /// a board-only footprint
    pub fn is_ok(&self) -> bool {
        matches!(self.status, FpStatus::Ok | FpStatus::BoardOnly)
            && !self.copy_differs
            && !self.board_differs
    }
}

/// Every footprint of a board against the review hash on its symbol
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpCheck {
    /// The board file
    pub board: PathBuf,
    /// The symbol field read ([`FIELD`])
    pub field: String,
    /// By reference, in natural order
    pub footprints: Vec<FpEntry>,
}

/// Why `berc fp check` could not run (exit code 2)
#[derive(Debug)]
pub enum FpCheckError {
    Open(crate::CheckError),
    /// Not a KiCad project, or one without a board
    NoBoard(PathBuf),
    /// A library given by path that is none
    Libraries(LibCheckError),
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
        }
    }
}

impl std::error::Error for FpCheckError {}

fn field_of<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

/// The KiCad project at `path` (a directory, its `.kicad_pro`, `.kicad_sch` or `.kicad_pcb`), its board and
/// layout, and its libraries through `search`
struct Opened {
    project: crate::Project,
    syms: SymbolLibraries,
    fps: FootprintLibraries,
}

impl Opened {
    fn open(path: &Path, search: &LibrarySearch) -> Result<Opened, FpCheckError> {
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
        let (syms, fps) = search.open(&project.dir).map_err(FpCheckError::Libraries)?;
        Ok(Opened { project, syms, fps })
    }
}

/// The `Lib:Name` a board footprint is compared with in the footprint libraries: its own when it names a library,
/// else its symbol's `Footprint` field
fn library_footprint(board_lib_id: &str, symbol_footprint: Option<&str>) -> Option<String> {
    [Some(board_lib_id), symbol_footprint]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|id| {
            id.split_once(':')
                .is_some_and(|(l, n)| !l.is_empty() && !n.is_empty())
        })
        .map(String::from)
}

/// Checks every footprint of the KiCad project at `path` (a directory, its `.kicad_pro`, `.kicad_sch` or
/// `.kicad_pcb`) against the review hash on its symbol, read from the library symbol found through `search` and the
/// project's tables or, where the library is not found, from the schematic's copy; and each board copy against its
/// library footprint
pub fn check(path: &Path, search: &LibrarySearch) -> Result<FpCheck, FpCheckError> {
    let mut opened = Opened::open(path, search)?;
    let Opened {
        project,
        syms: libs,
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
            stored: None,
            source: None,
            library_hash: None,
            schematic_hash: None,
            copy_differs: false,
            library_note: None,
            library_footprint: None,
            library_footprint_hash: None,
            board_differs: false,
            footprint_note: None,
        };
        let Some(part) = f.part.map(|p| board.part(p)) else {
            if f.attrs.board_only {
                e.status = FpStatus::BoardOnly;
            }
            compare_with_library(&mut e, None, fps);
            footprints.push(e);
            continue;
        };
        e.symbol = Some(part.lib_id.clone());
        let copy = part.symbol.map(|s| &board.symbols[s.index()]);
        e.schematic_hash = copy
            .and_then(|s| field_of(&s.fields, FIELD))
            .map(String::from);
        let lib_fields = match libs.fields(&part.lib_id) {
            Ok(fields) => Some(fields),
            Err(why) => {
                e.library_note = Some(why.to_string());
                None
            }
        };
        e.library_hash = lib_fields
            .as_deref()
            .and_then(|f| field_of(f, FIELD))
            .map(String::from);
        e.copy_differs = lib_fields.is_some() && e.library_hash != e.schematic_hash;
        (e.stored, e.source) = match &lib_fields {
            Some(_) => (e.library_hash.clone(), Some(HashSource::Library)),
            None => (e.schematic_hash.clone(), Some(HashSource::Schematic)),
        };
        let footprint_field = part
            .field("Footprint")
            .or_else(|| copy.and_then(|s| field_of(&s.fields, "Footprint")))
            .or_else(|| lib_fields.as_deref().and_then(|f| field_of(f, "Footprint")));
        compare_with_library(&mut e, footprint_field, fps);
        e.status = if footprint_field.is_none_or(|v| v.trim().is_empty()) {
            FpStatus::Refused
        } else {
            match &e.stored {
                None => FpStatus::Unreviewed,
                Some(stored) => {
                    match fphash::compare(stored, &lay.local_footprint(FootprintId::from_index(i)))
                    {
                        fphash::Compared::Same => FpStatus::Ok,
                        fphash::Compared::Changed { .. } => FpStatus::Changed,
                        fphash::Compared::OtherVersion { .. } => FpStatus::OtherVersion,
                        fphash::Compared::Malformed => FpStatus::Malformed,
                    }
                }
            }
        };
        footprints.push(e);
    }
    footprints.sort_by(|a, b| erc_model::natural_cmp(&a.reference, &b.reference));
    Ok(FpCheck {
        board: project.dir.join(&lay.file),
        field: FIELD.to_string(),
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
    /// Every footprint ok, every schematic copy up to date
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

    /// One line per footprint and a summary, statuses in colour (the caller's stream drops it off a terminal)
    pub fn to_text_color(&self) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let style = |s: FpStatus| match s {
            FpStatus::Ok => AnsiColor::Green.on_default(),
            FpStatus::Changed | FpStatus::Malformed => AnsiColor::Red.on_default().bold(),
            FpStatus::Unreviewed | FpStatus::OtherVersion => AnsiColor::Yellow.on_default().bold(),
            FpStatus::Refused | FpStatus::NoSymbol => AnsiColor::Cyan.on_default(),
            FpStatus::BoardOnly => Style::new().dimmed(),
        };
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
                FpStatus::Ok => {}
                FpStatus::Changed => {
                    let _ = write!(
                        out,
                        "  {dim}reviewed{dim:#} {}  {dim}now{dim:#} {}",
                        e.stored.as_deref().unwrap_or(""),
                        e.hash
                    );
                }
                FpStatus::Unreviewed => {
                    let _ = write!(
                        out,
                        "  {dim}hash now{dim:#} {}  {dim}for {FIELD} of {symbol}{dim:#}",
                        e.hash
                    );
                }
                FpStatus::Refused => {
                    let _ = write!(out, "  {dim}symbol {symbol} sets no footprint{dim:#}");
                }
                FpStatus::NoSymbol => {
                    let _ = write!(
                        out,
                        "  {dim}no schematic symbol, and not marked board-only{dim:#}"
                    );
                }
                FpStatus::BoardOnly => {}
                FpStatus::OtherVersion | FpStatus::Malformed => {
                    let _ = write!(
                        out,
                        "  {dim}{FIELD} of {symbol} is{dim:#} {}  {dim}now{dim:#} {}",
                        e.stored.as_deref().unwrap_or(""),
                        e.hash
                    );
                }
            }
            if e.copy_differs {
                let warn = AnsiColor::Yellow.on_default();
                let _ = write!(
                    out,
                    "\n{:13} {:width$} {warn}the schematic's copy of {symbol} has {}, the library {}: update the \
                     schematic from the library{warn:#}",
                    "",
                    "",
                    e.schematic_hash.as_deref().unwrap_or("no hash"),
                    e.library_hash.as_deref().unwrap_or("no hash"),
                );
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
            out.push('\n');
        }
        // Why libraries were not found, once each: their symbols' hashes came from the schematic's copies, and their
        // board copies were not compared with a library footprint
        let symbol_notes = self
            .footprints
            .iter()
            .filter(|e| e.source == Some(HashSource::Schematic))
            .filter_map(|e| e.library_note.as_deref());
        for (why, n) in once_each(symbol_notes) {
            let _ = writeln!(
                out,
                "{dim}read from the schematic's copy ({n} footprints): {why}{dim:#}"
            );
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
        let stale = self.footprints.iter().filter(|e| e.copy_differs).count();
        let _ = write!(
            out,
            "{}: {} footprints: {}",
            self.board.display(),
            self.footprints.len(),
            counts.join(", ")
        );
        if stale > 0 {
            let _ = write!(
                out,
                "; {stale} with a schematic copy older than the library"
            );
        }
        let differs = self.footprints.iter().filter(|e| e.board_differs).count();
        if differs > 0 {
            let _ = write!(
                out,
                "; {differs} board copies differ from their library footprint"
            );
        }
        out.push('\n');
        out
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_kicad/tests/fixtures/footprints")
    }

    /// The library file and a board copy of it give the same footprint hash through one source API; a library id
    /// reads the same through the project's fp-lib-table
    #[test]
    fn sources() {
        let file = FootprintSource::File(fixtures().join("Test.pretty/Asym_SOT.kicad_mod"))
            .read()
            .unwrap();
        let lay = read_board(&fixtures().join("placed.kicad_pcb")).unwrap();
        let i = lay
            .footprints
            .iter()
            .position(|f| f.lib_id.ends_with("Asym_SOT"))
            .unwrap();
        let reference = lay.footprints[i].reference.clone();
        let board =
            FootprintSource::from_path(&fixtures().join("placed.kicad_pcb"), Some(&reference))
                .unwrap()
                .read()
                .unwrap();
        assert_eq!(file.hash, board.hash);
        assert_eq!(board.lib_id(), lay.footprints[i].lib_id);
        assert!(matches!(board.source, FootprintSource::Board { .. }));
        // By library id through a project's fp-lib-table: the same footprint; a library the table lacks says so
        let dir = std::env::temp_dir().join(format!("berc-fp-source-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("fp-lib-table"),
            format!(
                "(fp_lib_table (version 7)\n  (lib (name \"Test\")(type \"KiCad\")(uri \"{}\")(options \"\")(descr \"\"))\n)\n",
                fixtures().join("Test.pretty").display()
            ),
        )
        .unwrap();
        let lib = |lib_id: &str| FootprintSource::Library {
            lib_id: lib_id.into(),
            project: dir.clone(),
            config: None,
        };
        assert_eq!(lib("Test:Asym_SOT").read().unwrap().hash, file.hash);
        let e = lib("Other:Asym_SOT").read().unwrap_err();
        assert!(matches!(e, FpError::Library(..)));
        assert!(e.to_string().contains("fp-lib-table"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
