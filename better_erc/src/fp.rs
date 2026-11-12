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

// ---------------------------------------------------------------------------------------------------------------
// berc fp accept

/// One file an accept writes the review hash into
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Written {
    pub file: PathBuf,
    /// The library symbol in its `.kicad_sym`, or the schematic's copy of it
    pub source: HashSource,
    /// The symbol's name in the file: `Name` in a library, `Lib:Name` in a schematic
    pub symbol: String,
    /// The field's value before, `None` when the symbol did not have it
    pub old: Option<String>,
    /// The file changes: the field was missing or held another value
    pub changed: bool,
}

/// A review hash accepted for one library symbol: every file that holds it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Accepted {
    /// The library symbol (`Lib:Name`)
    pub symbol: String,
    pub hash: String,
    /// The board footprints whose review it is, empty for a hash given by hand
    pub references: Vec<String>,
    /// The library's file first, when found, then the schematic sheets with a copy of the symbol
    pub files: Vec<Written>,
    /// Why the library symbol was not written, when it was not
    pub library_note: Option<String>,
}

/// What [`accept`] did, or would do with `dry_run`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AcceptRun {
    pub field: String,
    pub dry_run: bool,
    pub symbols: Vec<Accepted>,
}

/// What to accept: board footprints by reference, or a hash for a library symbol given by hand
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcceptWhat {
    /// Each footprint's review hash now, for its schematic symbol's library symbol
    References(Vec<String>),
    /// This hash for library symbol `symbol` (`Lib:Name`)
    Symbol { symbol: String, hash: String },
    /// This hash, of the library footprint reviewed, for the symbols of these board footprints: their board copies
    /// may differ from it (the board not yet updated from the library), `berc fp check` says so
    Reviewed {
        references: Vec<String>,
        hash: String,
    },
}

/// Why `berc fp accept` wrote nothing (exit code 2)
#[derive(Debug)]
pub enum AcceptError {
    Open(crate::CheckError),
    /// Not a KiCad project
    NotKicad(PathBuf),
    /// References to accept, but no board to take their footprints from
    NoBoard(PathBuf),
    NoFootprint(String),
    /// The footprint has no schematic symbol to hold the hash
    NoSymbol(String),
    /// Its symbol sets no footprint: a review means nothing
    Refused {
        reference: String,
        symbol: String,
    },
    /// Footprints of one symbol with different review hashes: a symbol used with several footprints, or board
    /// copies that differ
    Conflict {
        symbol: String,
        hashes: Vec<(String, String)>,
    },
    /// The symbol is placed with other footprints too: one review hash can't hold for all of them
    OtherFootprints {
        symbol: String,
        footprint: String,
        others: Vec<(String, String)>,
    },
    /// Not a review hash of this version (`fp1:` and 16 hex digits)
    BadHash(String),
    /// Neither the library symbol nor a schematic copy of it was found
    Nowhere {
        symbol: String,
        why: String,
    },
    Read(PathBuf, std::io::Error),
    Edit(PathBuf, erc_kicad::symlib::EditError),
    Write(PathBuf, std::io::Error),
}

impl AcceptError {
    pub const EXIT_CODE: u8 = crate::CheckError::EXIT_CODE;
}

impl fmt::Display for AcceptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcceptError::Open(e) => e.fmt(f),
            AcceptError::NotKicad(p) => write!(f, "{}: not a KiCad project", p.display()),
            AcceptError::NoBoard(p) => write!(
                f,
                "{}: no KiCad board (.kicad_pcb) to take the footprints from",
                p.display()
            ),
            AcceptError::NoFootprint(r) => write!(f, "no footprint {r} on the board"),
            AcceptError::NoSymbol(r) => write!(
                f,
                "footprint {r} has no schematic symbol to hold its review hash"
            ),
            AcceptError::Refused { reference, symbol } => write!(
                f,
                "{reference}: symbol {symbol} sets no footprint, so a review of it means nothing"
            ),
            AcceptError::Conflict { symbol, hashes } => {
                let list: Vec<String> = hashes.iter().map(|(r, h)| format!("{r} {h}")).collect();
                write!(
                    f,
                    "the footprints of symbol {symbol} differ ({}): a library symbol holds one review hash, \
                     accepted only for footprints that are the same",
                    list.join(", ")
                )
            }
            AcceptError::OtherFootprints {
                symbol,
                footprint,
                others,
            } => {
                let mut list: Vec<String> = others
                    .iter()
                    .take(4)
                    .map(|(r, f)| format!("{r} {f}"))
                    .collect();
                if others.len() > 4 {
                    list.push(format!("{} more", others.len() - 4));
                }
                write!(
                    f,
                    "symbol {symbol} is placed with other footprints than {footprint} too ({}): a library symbol \
                     holds one review hash, for one footprint",
                    list.join(", ")
                )
            }
            AcceptError::BadHash(h) => {
                write!(
                    f,
                    "`{h}` is no review hash (`{}:` and 16 hex digits)",
                    fphash::VERSION
                )
            }
            AcceptError::Nowhere { symbol, why } => write!(
                f,
                "symbol {symbol} is neither in a library found ({why}) nor in a schematic sheet's lib_symbols"
            ),
            AcceptError::Read(p, e) => write!(f, "can't read {}: {e}", p.display()),
            AcceptError::Edit(p, e) => write!(f, "{}: {e}", p.display()),
            AcceptError::Write(p, e) => write!(f, "can't write {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for AcceptError {}

/// Writes review hashes into the library symbols of the KiCad project at `path` (a directory, its `.kicad_pro`,
/// `.kicad_sch` or `.kicad_pcb`) and into the schematic sheets' copies of them, so `berc fp check` finds them ok:
/// the [`FIELD`] property set or inserted by [`erc_kicad::symlib::set_property`], every other byte of each file
/// kept. The library is found through `libs` as [`check`] finds it. Every edit is made before any file is written,
/// so a failing one writes nothing; `dry_run` writes nothing at all.
pub fn accept(
    path: &Path,
    what: &AcceptWhat,
    libs: &mut dyn FnMut(&Path) -> SymbolLibraries,
    dry_run: bool,
) -> Result<AcceptRun, AcceptError> {
    let project = crate::Project::open(path).map_err(AcceptError::Open)?;
    if project.importer != "kicad" {
        return Err(AcceptError::NotKicad(path.to_path_buf()));
    }
    let board = project.imported.design.boards.first();
    let mut wanted: Vec<Wanted> = Vec::new();
    match what {
        AcceptWhat::Symbol { symbol, hash } => {
            if !fphash::is_hash(hash.trim()) {
                return Err(AcceptError::BadHash(hash.clone()));
            }
            wanted.push((symbol.clone(), hash.trim().to_string(), Vec::new()));
        }
        AcceptWhat::References(refs)
        | AcceptWhat::Reviewed {
            references: refs, ..
        } => {
            let reviewed = match what {
                AcceptWhat::Reviewed { hash, .. } => {
                    if !fphash::is_hash(hash.trim()) {
                        return Err(AcceptError::BadHash(hash.clone()));
                    }
                    Some(hash.trim().to_string())
                }
                _ => None,
            };
            let no_board = || AcceptError::NoBoard(path.to_path_buf());
            let board = board.ok_or_else(no_board)?;
            let lay = board.layout.as_ref().ok_or_else(no_board)?;
            for r in refs {
                let i = lay
                    .footprints
                    .iter()
                    .position(|f| &f.reference == r)
                    .ok_or_else(|| AcceptError::NoFootprint(r.clone()))?;
                let f = &lay.footprints[i];
                let part = f
                    .part
                    .map(|p| board.part(p))
                    .ok_or_else(|| AcceptError::NoSymbol(r.clone()))?;
                let copy = part.symbol.map(|s| &board.symbols[s.index()]);
                let sets_footprint = part
                    .field("Footprint")
                    .or_else(|| copy.and_then(|s| field_of(&s.fields, "Footprint")))
                    .is_some_and(|v| !v.trim().is_empty());
                if !sets_footprint {
                    return Err(AcceptError::Refused {
                        reference: r.clone(),
                        symbol: part.lib_id.clone(),
                    });
                }
                // Every footprint placed for this symbol is this one: the hash is to hold for them all
                let others: Vec<(String, String)> = lay
                    .footprints
                    .iter()
                    .filter(|o| o.lib_id != f.lib_id)
                    .filter(|o| o.part.is_some_and(|p| board.part(p).lib_id == part.lib_id))
                    .map(|o| (o.reference.clone(), o.lib_id.clone()))
                    .collect();
                if !others.is_empty() {
                    return Err(AcceptError::OtherFootprints {
                        symbol: part.lib_id.clone(),
                        footprint: f.lib_id.clone(),
                        others,
                    });
                }
                let hash = reviewed.clone().unwrap_or_else(|| {
                    fphash::hash(&lay.local_footprint(FootprintId::from_index(i)))
                });
                match wanted.iter_mut().find(|w| w.0 == part.lib_id) {
                    Some(w) => w.2.push((r.clone(), hash)),
                    None => {
                        wanted.push((part.lib_id.clone(), hash.clone(), vec![(r.clone(), hash)]))
                    }
                }
            }
            for (symbol, _, refs) in &wanted {
                if refs.iter().any(|(_, h)| *h != refs[0].1) {
                    return Err(AcceptError::Conflict {
                        symbol: symbol.clone(),
                        hashes: refs.clone(),
                    });
                }
            }
        }
    }
    // The schematic sheets, each file once
    let mut sheets: Vec<PathBuf> = Vec::new();
    for b in &project.imported.design.boards {
        for s in &b.sheets {
            let p = project.dir.join(&s.file);
            if !sheets.contains(&p) {
                sheets.push(p);
            }
        }
    }
    let mut libs = libs(&project.dir);
    write_hashes(wanted, &sheets, &mut libs, dry_run)
}

/// The library symbols to write, each with its hash and the references it came from, with theirs
type Wanted = (String, String, Vec<(String, String)>);

/// Writes the review hash `hash` into library symbol `symbol` (`Lib:Name`) found through `libs`, without a project:
/// a symbol picked from a library, before any schematic uses it. Like [`accept`], every other byte of the file is
/// kept; `dry_run` writes nothing.
pub fn accept_library(
    libs: &mut SymbolLibraries,
    symbol: &str,
    hash: &str,
    dry_run: bool,
) -> Result<AcceptRun, AcceptError> {
    if !fphash::is_hash(hash.trim()) {
        return Err(AcceptError::BadHash(hash.to_string()));
    }
    let wanted = vec![(symbol.to_string(), hash.trim().to_string(), Vec::new())];
    write_hashes(wanted, &[], libs, dry_run)
}

/// Writes each wanted hash into its library symbol through `libs` and into the copies in `sheets`
fn write_hashes(
    wanted: Vec<Wanted>,
    sheets: &[PathBuf],
    libs: &mut SymbolLibraries,
    dry_run: bool,
) -> Result<AcceptRun, AcceptError> {
    // Every edit first, by file: one file can hold several symbols
    let mut texts: Vec<(PathBuf, String, String)> = Vec::new();
    let mut edit =
        |file: &Path, symbol: &str, hash: &str| -> Result<Option<Written>, AcceptError> {
            let i = match texts.iter().position(|t| t.0 == file) {
                Some(i) => i,
                None => {
                    let text = std::fs::read_to_string(file)
                        .map_err(|e| AcceptError::Read(file.to_path_buf(), e))?;
                    texts.push((file.to_path_buf(), text.clone(), text));
                    texts.len() - 1
                }
            };
            let e = match erc_kicad::symlib::set_property(&texts[i].2, symbol, FIELD, hash) {
                Ok(e) => e,
                Err(erc_kicad::symlib::EditError::NoSymbol(_)) => return Ok(None),
                Err(e) => return Err(AcceptError::Edit(file.to_path_buf(), e)),
            };
            texts[i].2 = e.text;
            Ok(Some(Written {
                file: file.to_path_buf(),
                source: HashSource::Library,
                symbol: symbol.to_string(),
                old: e.old,
                changed: e.changed,
            }))
        };
    let mut symbols = Vec::new();
    for (symbol, hash, refs) in wanted {
        let mut files = Vec::new();
        let library_note = match libs.symbol_file(&symbol) {
            Ok((file, name)) => match edit(&file, &name, &hash)? {
                Some(w) => {
                    files.push(w);
                    None
                }
                None => Some(format!("no symbol {name} in {}", file.display())),
            },
            Err(why) => Some(why.to_string()),
        };
        for sheet in sheets {
            if let Some(mut w) = edit(sheet, &symbol, &hash)? {
                w.source = HashSource::Schematic;
                files.push(w);
            }
        }
        if files.is_empty() {
            return Err(AcceptError::Nowhere {
                symbol,
                why: library_note.unwrap_or_default(),
            });
        }
        symbols.push(Accepted {
            symbol,
            hash,
            references: refs.into_iter().map(|(r, _)| r).collect(),
            files,
            library_note,
        });
    }
    if !dry_run {
        for (file, before, after) in &texts {
            if before != after {
                write_atomic(file, after).map_err(|e| AcceptError::Write(file.clone(), e))?;
            }
        }
    }
    Ok(AcceptRun {
        field: FIELD.to_string(),
        dry_run,
        symbols,
    })
}

/// `text` into `path` through a temporary file beside it, renamed over it: a reader never sees half a file, and the
/// file keeps its permissions
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.berc-{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

impl AcceptRun {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// One line per file written, a note per library not found
    pub fn to_text_color(&self) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let dim = Style::new().dimmed();
        let green = AnsiColor::Green.on_default();
        let mut out = String::new();
        for a in &self.symbols {
            let refs = if a.references.is_empty() {
                String::new()
            } else {
                format!(" {dim}for{dim:#} {}", a.references.join(", "))
            };
            let _ = writeln!(out, "{} {green}{}{green:#}{refs}", a.symbol, a.hash);
            for w in &a.files {
                let what = match (&w.old, w.changed) {
                    (_, false) => "already has it".to_string(),
                    (None, true) => format!("{FIELD} added"),
                    (Some(old), true) => format!("{FIELD} was {old}"),
                };
                let kind = match w.source {
                    HashSource::Library => "library",
                    HashSource::Schematic => "schematic copy",
                };
                let _ = writeln!(out, "  {} {dim}({kind}): {what}{dim:#}", w.file.display());
            }
            if let Some(n) = &a.library_note {
                let warn = AnsiColor::Yellow.on_default();
                let _ = writeln!(
                    out,
                    "  {warn}library symbol not written: {n}; update the library by hand{warn:#}"
                );
            }
        }
        let n: usize = self
            .symbols
            .iter()
            .map(|a| a.files.iter().filter(|w| w.changed).count())
            .sum();
        let verb = if self.dry_run {
            "would change"
        } else {
            "changed"
        };
        let _ = writeln!(out, "{} symbols, {n} files {verb}", self.symbols.len());
        out
    }
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

    /// A symbol picked from a library, no project: accept_library writes the hash into its file alone, and
    /// check_libraries then finds it ok; a bad hash or a symbol the library lacks writes nothing
    #[test]
    fn accept_into_a_library() {
        let dir = std::env::temp_dir().join(format!("berc-fp-acceptlib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("Rev.kicad_sym");
        std::fs::write(
            &lib,
            "(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n  (symbol \"Asym\" (in_bom yes) (on_board yes)\n    (property \"Reference\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n    (property \"Footprint\" \"Test:Asym_SOT\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))\n    (property \"Datasheet\" \"~\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))))\n",
        )
        .unwrap();
        let search = LibrarySearch {
            config: None,
            symbols: vec![lib.clone()],
            footprints: vec![fixtures().join("Test.pretty")],
        };
        let check = || {
            check_libraries(&LibCheckOptions {
                libraries: vec![lib.display().to_string()],
                project: dir.clone(),
                search: search.clone(),
            })
            .unwrap()
        };
        let before = check();
        assert_eq!(before.symbols[0].status, SymbolStatus::Unreviewed);
        assert_eq!(before.symbols[0].datasheet, None, "~ is no datasheet");
        let hash = before.symbols[0].hash.clone().unwrap();
        let (mut syms, _) = search.open(&dir).unwrap();
        assert!(matches!(
            accept_library(&mut syms, "Rev:Asym", "fp1:nothex", false),
            Err(AcceptError::BadHash(_))
        ));
        assert!(matches!(
            accept_library(&mut syms, "Rev:Other", &hash, false),
            Err(AcceptError::Nowhere { .. })
        ));
        let run = accept_library(&mut syms, "Rev:Asym", &hash, false).unwrap();
        assert_eq!(run.symbols[0].files.len(), 1);
        assert_eq!(run.symbols[0].files[0].file, lib);
        assert!(run.symbols[0].files[0].changed);
        assert_eq!(check().symbols[0].status, SymbolStatus::Ok);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
