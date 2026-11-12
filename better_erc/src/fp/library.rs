//! `berc fp check --lib`: every symbol of a symbol library against the review hash of the library footprint its
//! `Footprint` field names, before any board uses it.

use super::{FIELD, field_of};
use erc_kicad::libtable::{FootprintLibraries, LibError, SymbolLibraries, SymbolLibrary};
use erc_model::fphash;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// What `berc fp check --lib` found for one library symbol
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SymbolStatus {
    /// The review hash matches the library footprint
    Ok,
    /// The footprint changed since the review
    Changed,
    /// No review hash on the symbol
    Unreviewed,
    /// The `Footprint` field names a footprint the footprint libraries don't give
    Missing,
    /// A review hash on a symbol that sets no footprint: it reviews nothing
    Refused,
    /// The hash was made by another version of berc's review hash
    OtherVersion,
    /// The field holds something that is no review hash
    Malformed,
    /// The symbol sets no footprint and has no hash (a generic part, a power symbol): nothing to review, not a
    /// failure
    NoFootprint,
}

impl SymbolStatus {
    pub fn name(self) -> &'static str {
        match self {
            SymbolStatus::Ok => "ok",
            SymbolStatus::Changed => "changed",
            SymbolStatus::Unreviewed => "unreviewed",
            SymbolStatus::Missing => "missing",
            SymbolStatus::Refused => "refused",
            SymbolStatus::OtherVersion => "other version",
            SymbolStatus::Malformed => "malformed",
            SymbolStatus::NoFootprint => "no footprint",
        }
    }

    const ALL: [SymbolStatus; 8] = [
        SymbolStatus::Ok,
        SymbolStatus::Changed,
        SymbolStatus::Unreviewed,
        SymbolStatus::Missing,
        SymbolStatus::Refused,
        SymbolStatus::OtherVersion,
        SymbolStatus::Malformed,
        SymbolStatus::NoFootprint,
    ];
}

/// One library symbol checked
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// `Lib:Name`, the library by its nickname
    pub symbol: String,
    /// The `Footprint` field, when the symbol sets one
    pub footprint: Option<String>,
    pub status: SymbolStatus,
    /// The footprint's review hash now, when it was found
    pub hash: Option<String>,
    /// The symbol's `Footprint_Review` field
    pub stored: Option<String>,
    /// The `.kicad_mod` read
    pub footprint_file: Option<PathBuf>,
    /// Why the footprint or the symbol could not be read
    pub note: Option<String>,
}

impl SymbolEntry {
    /// Ok, or nothing to review
    pub fn is_ok(&self) -> bool {
        matches!(self.status, SymbolStatus::Ok | SymbolStatus::NoFootprint)
    }
}

/// One symbol library read
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckedLibrary {
    /// The nickname its symbols are named by: the table's, or the file's name for a library given as a path
    pub nickname: String,
    /// The `.kicad_sym` file or `.kicad_symdir` directory
    pub path: PathBuf,
    pub symbols: usize,
}

/// Every symbol of some symbol libraries against its library footprint's review hash
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibCheck {
    /// The symbol field read ([`FIELD`])
    pub field: String,
    pub libraries: Vec<CheckedLibrary>,
    /// Library by library in the order given, symbols in natural order
    pub symbols: Vec<SymbolEntry>,
}

/// Why `berc fp check --lib` could not run (exit code 2)
#[derive(Debug)]
pub enum LibCheckError {
    /// Neither a symbol library on disk nor a nickname in a `sym-lib-table`
    NoLibrary(String, LibError),
    /// A directory without a `.kicad_sym` or `.kicad_symdir` in it
    EmptyDir(PathBuf),
    /// A symbol library that can't be read
    Unreadable(PathBuf),
    /// A `--footprints` path that is no `.pretty` directory and has none in it
    NoFootprints(PathBuf),
}

impl LibCheckError {
    pub const EXIT_CODE: u8 = crate::CheckError::EXIT_CODE;
}

impl fmt::Display for LibCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LibCheckError::NoLibrary(arg, e) => write!(
                f,
                "{arg}: no symbol library there (a .kicad_sym, a .kicad_symdir or a folder of them), and no \
                 nickname of one: {e}"
            ),
            LibCheckError::EmptyDir(p) => write!(
                f,
                "{}: no symbol library (.kicad_sym, .kicad_symdir) in it",
                p.display()
            ),
            LibCheckError::Unreadable(p) => {
                write!(f, "{}: can't read it as a symbol library", p.display())
            }
            LibCheckError::NoFootprints(p) => write!(
                f,
                "{}: no footprint library (.pretty) there or in it",
                p.display()
            ),
        }
    }
}

impl std::error::Error for LibCheckError {}

/// Where the libraries are found and what is checked
#[derive(Clone, Debug, Default)]
pub struct LibCheckOptions {
    /// The symbol libraries: each a `.kicad_sym` file, a `.kicad_symdir` directory, a folder holding such
    /// libraries (each of them), or a nickname in the project's or the global `sym-lib-table`
    pub libraries: Vec<String>,
    /// The project directory whose `sym-lib-table` and `fp-lib-table` are read, besides the global ones
    pub project: PathBuf,
    /// KiCad's configuration directory ([`super::kicad_config_dir`]), for the global tables
    pub config: Option<PathBuf>,
    /// Footprint libraries searched before the tables: each a `.pretty` directory, or a folder of them, named by
    /// their directory names
    pub footprints: Vec<PathBuf>,
}

fn is_symbol_library(p: &Path) -> bool {
    match p.extension().and_then(|e| e.to_str()) {
        Some("kicad_sym") => p.is_file(),
        Some("kicad_symdir") => p.is_dir(),
        _ => false,
    }
}

fn is_footprint_library(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == "pretty") && p.is_dir()
}

/// `dir` when it is a library (`keep`), else the libraries in it, sorted
fn libraries_in(dir: &Path, keep: fn(&Path) -> bool) -> Vec<PathBuf> {
    if keep(dir) {
        return vec![dir.to_path_buf()];
    }
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| keep(p))
        .collect();
    out.sort();
    out
}

/// The symbol libraries `arg` stands for, each with its nickname
fn resolve(
    arg: &str,
    tables: &mut SymbolLibraries,
) -> Result<Vec<(String, PathBuf)>, LibCheckError> {
    let p = Path::new(arg);
    if is_symbol_library(p) {
        return Ok(vec![(SymbolLibrary::new(p).default_nickname(), p.into())]);
    }
    if p.is_dir() {
        let libs = libraries_in(p, is_symbol_library);
        if libs.is_empty() {
            return Err(LibCheckError::EmptyDir(p.into()));
        }
        return Ok(libs
            .into_iter()
            .map(|l| (SymbolLibrary::new(&l).default_nickname(), l))
            .collect());
    }
    let path = tables
        .tables()
        .path(arg)
        .map_err(|e| LibCheckError::NoLibrary(arg.into(), e))?;
    Ok(vec![(arg.to_string(), path)])
}

/// Checks every symbol of `opts.libraries` against the review hash of its library footprint
pub fn check_libraries(opts: &LibCheckOptions) -> Result<LibCheck, LibCheckError> {
    let mut sym_tables = SymbolLibraries::new(&opts.project, opts.config.as_deref());
    let mut fps = FootprintLibraries::new(&opts.project, opts.config.as_deref());
    // Later --footprints go first, so the first one given wins as in a table
    for f in opts.footprints.iter().rev() {
        let pretty = libraries_in(f, is_footprint_library);
        if pretty.is_empty() {
            return Err(LibCheckError::NoFootprints(f.clone()));
        }
        for p in pretty.iter().rev() {
            let nick = p
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            fps.add_first(&nick, p);
        }
    }
    let mut libraries = Vec::new();
    for arg in &opts.libraries {
        libraries.extend(resolve(arg, &mut sym_tables)?);
    }
    let mut checked = Vec::new();
    let mut symbols = Vec::new();
    for (nick, path) in libraries {
        let mut lib = SymbolLibrary::new(&path);
        let names = lib
            .names()
            .ok_or_else(|| LibCheckError::Unreadable(path.clone()))?;
        checked.push(CheckedLibrary {
            nickname: nick.clone(),
            path: path.clone(),
            symbols: names.len(),
        });
        for name in names {
            symbols.push(check_symbol(&nick, &name, &mut lib, &mut fps));
        }
    }
    Ok(LibCheck {
        field: FIELD.to_string(),
        libraries: checked,
        symbols,
    })
}

fn check_symbol(
    nick: &str,
    name: &str,
    lib: &mut SymbolLibrary,
    fps: &mut FootprintLibraries,
) -> SymbolEntry {
    let mut e = SymbolEntry {
        symbol: format!("{nick}:{name}"),
        footprint: None,
        status: SymbolStatus::NoFootprint,
        hash: None,
        stored: None,
        footprint_file: None,
        note: None,
    };
    let fields = match lib.fields(name) {
        Ok(f) => f,
        Err(why) => {
            e.status = SymbolStatus::Missing;
            e.note = Some(why.to_string());
            return e;
        }
    };
    e.footprint = field_of(&fields, "Footprint").map(String::from);
    e.stored = field_of(&fields, FIELD).map(String::from);
    let Some(footprint) = e.footprint.clone() else {
        if e.stored.is_some() {
            e.status = SymbolStatus::Refused;
        }
        return e;
    };
    let found = match fps.footprint(&footprint) {
        Ok(f) => f,
        Err(why) => {
            e.status = SymbolStatus::Missing;
            e.note = Some(why.to_string());
            return e;
        }
    };
    e.footprint_file = Some(found.path.clone());
    e.hash = Some(fphash::hash(&found.footprint));
    e.status = match &e.stored {
        None => SymbolStatus::Unreviewed,
        Some(stored) => match fphash::compare(stored, &found.footprint) {
            fphash::Compared::Same => SymbolStatus::Ok,
            fphash::Compared::Changed { .. } => SymbolStatus::Changed,
            fphash::Compared::OtherVersion { .. } => SymbolStatus::OtherVersion,
            fphash::Compared::Malformed => SymbolStatus::Malformed,
        },
    };
    e
}

impl LibCheck {
    /// Every symbol ok or without a footprint
    pub fn is_ok(&self) -> bool {
        self.symbols.iter().all(SymbolEntry::is_ok)
    }

    /// How many symbols have each status, the statuses that occur, in a fixed order
    pub fn counts(&self) -> Vec<(SymbolStatus, usize)> {
        SymbolStatus::ALL
            .iter()
            .map(|s| (*s, self.symbols.iter().filter(|e| e.status == *s).count()))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// One line per symbol and a summary, statuses in colour (the caller's stream drops it off a terminal).
    /// Symbols without a footprint are listed last under their own heading.
    pub fn to_text_color(&self) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let style = |s: SymbolStatus| match s {
            SymbolStatus::Ok => AnsiColor::Green.on_default(),
            SymbolStatus::Changed | SymbolStatus::Malformed | SymbolStatus::Missing => {
                AnsiColor::Red.on_default().bold()
            }
            SymbolStatus::Unreviewed | SymbolStatus::OtherVersion => {
                AnsiColor::Yellow.on_default().bold()
            }
            SymbolStatus::Refused => AnsiColor::Cyan.on_default(),
            SymbolStatus::NoFootprint => Style::new().dimmed(),
        };
        let dim = Style::new().dimmed();
        let width = self
            .symbols
            .iter()
            .map(|e| e.symbol.len())
            .max()
            .unwrap_or(0);
        let fp_width = self
            .symbols
            .iter()
            .filter_map(|e| e.footprint.as_ref().map(String::len))
            .max()
            .unwrap_or(0);
        let mut out = String::new();
        let (none, rest): (Vec<&SymbolEntry>, Vec<&SymbolEntry>) = self
            .symbols
            .iter()
            .partition(|e| e.status == SymbolStatus::NoFootprint);
        let heading = format!(
            "{dim}symbols without a footprint, nothing to review ({}):{dim:#}\n",
            none.len()
        );
        for (i, e) in rest.iter().chain(&none).enumerate() {
            if i == rest.len() {
                out += &heading;
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
                SymbolStatus::Ok | SymbolStatus::NoFootprint => {}
                SymbolStatus::Changed => {
                    let _ = write!(
                        out,
                        "  {dim}reviewed{dim:#} {stored}  {dim}now{dim:#} {now}"
                    );
                }
                SymbolStatus::Unreviewed => {
                    let _ = write!(out, "  {dim}hash now{dim:#} {now}  {dim}for {FIELD}{dim:#}");
                }
                SymbolStatus::Missing => {
                    let _ = write!(out, "  {dim}{}{dim:#}", e.note.as_deref().unwrap_or(""));
                }
                SymbolStatus::Refused => {
                    let _ = write!(
                        out,
                        "  {dim}{FIELD} is{dim:#} {stored}  {dim}but the symbol sets no footprint{dim:#}"
                    );
                }
                SymbolStatus::OtherVersion | SymbolStatus::Malformed => {
                    let _ = write!(
                        out,
                        "  {dim}{FIELD} is{dim:#} {stored}  {dim}now{dim:#} {now}"
                    );
                }
            }
            // The footprint column's padding, where nothing follows it
            out.truncate(out.trim_end_matches(' ').len());
            out.push('\n');
        }
        let counts: Vec<String> = self
            .counts()
            .iter()
            .map(|(s, n)| {
                let st = style(*s);
                format!("{st}{n} {}{st:#}", s.name())
            })
            .collect();
        let libs = match self.libraries.as_slice() {
            [one] => one.path.display().to_string(),
            many => format!("{} libraries", many.len()),
        };
        let _ = writeln!(
            out,
            "{libs}: {} symbols: {}",
            self.symbols.len(),
            if counts.is_empty() {
                "none".to_string()
            } else {
                counts.join(", ")
            }
        );
        out
    }
}
