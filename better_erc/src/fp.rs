//! Footprint review on disk, the library side of `berc fp`: the review hash ([`erc_model::fphash`]) of a library
//! footprint file or of a footprint as placed on a board ([`hash_file`]).

use erc_kicad::footprint;
use erc_model::layout::{FootprintId, Layout};
use erc_model::{ImportError, LocalFootprint, fphash};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// A footprint's review hash and the canonical text it is taken over
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hashed {
    /// The file read: a `.kicad_mod` or a `.kicad_pcb`
    pub file: PathBuf,
    /// The board footprint's reference, for a board
    pub reference: Option<String>,
    /// The footprint's library id (`Lib:Name` on a board, the footprint's name in a library file)
    pub lib_id: String,
    /// `fp1:` and 32 hex digits
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
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match (ext, reference) {
        ("kicad_mod", None) => Ok(Hashed::of(path, None, &read_library(path)?)),
        ("kicad_mod", Some(_)) => Err(FpError::ReferenceOnLibrary(path.to_path_buf())),
        ("kicad_pcb", None) => Err(FpError::NeedReference(path.to_path_buf())),
        ("kicad_pcb", Some(r)) => {
            let lay = read_board(path)?;
            let i = lay
                .footprints
                .iter()
                .position(|f| f.reference == r)
                .ok_or_else(|| FpError::NoFootprint {
                    board: path.to_path_buf(),
                    reference: r.to_string(),
                })?;
            let local = lay.local_footprint(FootprintId::from_index(i));
            Ok(Hashed::of(path, Some(r.to_string()), &local))
        }
        _ => Err(FpError::NotFootprint(path.to_path_buf())),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// berc fp check

/// The library symbol field that holds a footprint's review hash
pub const FIELD: &str = "Footprint_Review";

/// Where the symbol libraries of a project are found: its own `sym-lib-table`, then the user's global one, with
/// `${VAR}` from the environment, KiCad's `kicad_common.json` and `KIPRJMOD`. Each library file is read once.
pub struct SymbolLibraries {
    /// The tables in the order they are searched
    tables: Vec<Vec<erc_kicad::symlib::LibEntry>>,
    vars: std::collections::HashMap<String, String>,
    files: std::collections::HashMap<PathBuf, Option<String>>,
}

/// A library symbol's fields as found through the library tables, or why they were not
#[derive(Clone, Debug, PartialEq)]
pub enum LibraryLookup {
    Found(Vec<(String, String)>),
    NotFound(String),
}

impl SymbolLibraries {
    /// The libraries of the project in `dir`; `config` is KiCad's configuration directory with the global
    /// `sym-lib-table` and `kicad_common.json` ([`SymbolLibraries::kicad_config_dir`]), `None` for none
    pub fn new(dir: &Path, config: Option<&Path>) -> SymbolLibraries {
        let mut vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        if let Some(c) = config
            && let Ok(text) = std::fs::read_to_string(c.join("kicad_common.json"))
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(map) = json["environment"]["vars"].as_object()
        {
            for (k, v) in map {
                if let Some(v) = v.as_str() {
                    vars.insert(k.clone(), v.to_string());
                }
            }
        }
        vars.insert("KIPRJMOD".into(), dir.to_string_lossy().into_owned());
        let mut tables = Vec::new();
        for t in [
            Some(dir.join("sym-lib-table")),
            config.map(|c| c.join("sym-lib-table")),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(entries) = std::fs::read_to_string(&t)
                .ok()
                .and_then(|text| erc_kicad::symlib::read_lib_table(&text))
            {
                tables.push(entries);
            }
        }
        SymbolLibraries {
            tables,
            vars,
            files: std::collections::HashMap::new(),
        }
    }

    /// KiCad's configuration directory of the newest KiCad that has one: `$KICAD_CONFIG_HOME`, else
    /// `$XDG_CONFIG_HOME/kicad/<version>` or `~/.config/kicad/<version>`
    pub fn kicad_config_dir() -> Option<PathBuf> {
        if let Some(d) = std::env::var_os("KICAD_CONFIG_HOME") {
            return Some(d.into());
        }
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?
            .join("kicad");
        ["10.0", "9.0", "8.0"]
            .iter()
            .map(|v| base.join(v))
            .find(|d| d.join("sym-lib-table").is_file())
    }

    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name)
            .ok()
            .or_else(|| self.vars.get(name).cloned())
    }

    /// The fields of library symbol `lib_id` (`Lib:Name`)
    pub fn fields(&mut self, lib_id: &str) -> LibraryLookup {
        let Some((nick, name)) = lib_id.split_once(':') else {
            return LibraryLookup::NotFound(format!("`{lib_id}` names no library"));
        };
        let Some(entry) = self
            .tables
            .iter()
            .find_map(|t| t.iter().find(|e| e.name == nick))
            .cloned()
        else {
            return LibraryLookup::NotFound(format!("library {nick} is in no sym-lib-table"));
        };
        let Some(path) = erc_kicad::symlib::expand(&entry.uri, &|v| self.var(v)) else {
            return LibraryLookup::NotFound(format!(
                "library {nick}: a variable of {} is not set",
                entry.uri
            ));
        };
        let path = PathBuf::from(path);
        let text = self
            .files
            .entry(path.clone())
            .or_insert_with(|| std::fs::read_to_string(&path).ok());
        let Some(text) = text else {
            return LibraryLookup::NotFound(format!(
                "library {nick}: can't read {}",
                path.display()
            ));
        };
        match erc_kicad::symlib::symbol_fields(text, name) {
            Some(f) => LibraryLookup::Found(f),
            None => LibraryLookup::NotFound(format!("no symbol {name} in {}", path.display())),
        }
    }
}

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
    /// No schematic symbol: a board-only footprint, or one whose symbol is gone
    NoSymbol,
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
            FpStatus::OtherVersion => "other version",
            FpStatus::Malformed => "malformed",
        }
    }

    const ALL: [FpStatus; 7] = [
        FpStatus::Ok,
        FpStatus::Changed,
        FpStatus::Unreviewed,
        FpStatus::Refused,
        FpStatus::NoSymbol,
        FpStatus::OtherVersion,
        FpStatus::Malformed,
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
}

impl FpEntry {
    /// Ok, with the schematic's copy agreeing with the library
    pub fn is_ok(&self) -> bool {
        self.status == FpStatus::Ok && !self.copy_differs
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

/// Checks every footprint of the KiCad project at `path` (a directory, its `.kicad_pro`, `.kicad_sch` or
/// `.kicad_pcb`) against the review hash on its symbol, read through `libs` ([`SymbolLibraries::new`] for the
/// project's directory) or, where the library is not found, from the schematic's copy
pub fn check(
    path: &Path,
    libs: &mut dyn FnMut(&Path) -> SymbolLibraries,
) -> Result<FpCheck, FpCheckError> {
    let project = crate::Project::open(path).map_err(FpCheckError::Open)?;
    let no_board = || FpCheckError::NoBoard(path.to_path_buf());
    if project.importer != "kicad" {
        return Err(no_board());
    }
    let board = project
        .imported
        .design
        .boards
        .first()
        .ok_or_else(no_board)?;
    let lay = board.layout.as_ref().ok_or_else(no_board)?;
    let mut libs = libs(&project.dir);
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
        };
        let Some(part) = f.part.map(|p| board.part(p)) else {
            footprints.push(e);
            continue;
        };
        e.symbol = Some(part.lib_id.clone());
        let copy = part.symbol.map(|s| &board.symbols[s.index()]);
        e.schematic_hash = copy
            .and_then(|s| field_of(&s.fields, FIELD))
            .map(String::from);
        let lib_fields = match libs.fields(&part.lib_id) {
            LibraryLookup::Found(fields) => Some(fields),
            LibraryLookup::NotFound(why) => {
                e.library_note = Some(why);
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
        for e in &self.footprints {
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
                    let _ = write!(out, "  {dim}no schematic symbol{dim:#}");
                }
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
            out.push('\n');
        }
        // Why libraries were not found, once each: their symbols' hashes came from the schematic's copies
        let mut notes: Vec<(&str, usize)> = Vec::new();
        for why in self
            .footprints
            .iter()
            .filter(|e| e.source == Some(HashSource::Schematic))
            .filter_map(|e| e.library_note.as_deref())
        {
            match notes.iter_mut().find(|(w, _)| *w == why) {
                Some(n) => n.1 += 1,
                None => notes.push((why, 1)),
            }
        }
        notes.sort();
        for (why, n) in notes {
            let _ = writeln!(
                out,
                "{dim}read from the schematic's copy ({n} footprints): {why}{dim:#}"
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
        out.push('\n');
        out
    }
}
