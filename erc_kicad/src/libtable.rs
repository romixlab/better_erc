//! KiCad's library tables: where the library behind a nickname (`Device` in `Device:R`) is found, for symbols
//! (`sym-lib-table`) and footprints (`fp-lib-table`), with the lookups on top that the footprint review needs.
//!
//! - [`LibTables`]: the project's table, then the user's global one (KiCad's configuration directory,
//!   [`kicad_config_dir`]), nested tables (`(type "Table")`) read in place, disabled libraries skipped.
//! - [`KicadVars`]: the `${VAR}` of a library's URI: the environment, then `kicad_common.json`'s
//!   `environment.vars`, `KIPRJMOD` (the project's directory) and the stock libraries of an installed KiCad
//!   (`KICAD9_FOOTPRINT_DIR`, ...) when nothing else sets them.
//! - [`SymbolLibraries`]: `Lib:Name` to the fields of a library symbol (`.kicad_sym` files and KiCad 9+
//!   `.kicad_symdir` directories), through [`crate::symlib`].
//! - [`FootprintLibraries`]: `Lib:Name` to a parsed library footprint (`Lib.pretty/Name.kicad_mod`), through
//!   [`crate::footprint::read`].
//!
//! Every file is read once per [`SymbolLibraries`] or [`FootprintLibraries`]; make a new one to see changes.

use crate::sexpr;
use erc_model::{ImportNote, LocalFootprint};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// The KiCad versions whose configuration and stock libraries are looked for, newest first
const VERSIONS: [u32; 5] = [10, 9, 8, 7, 6];

/// Symbol or footprint libraries: which table file they are listed in
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LibKind {
    Symbol,
    Footprint,
}

impl LibKind {
    /// The table's file name, `sym-lib-table` or `fp-lib-table`
    pub fn table_file(self) -> &'static str {
        match self {
            LibKind::Symbol => "sym-lib-table",
            LibKind::Footprint => "fp-lib-table",
        }
    }

    fn table_head(self) -> &'static str {
        match self {
            LibKind::Symbol => "sym_lib_table",
            LibKind::Footprint => "fp_lib_table",
        }
    }
}

/// One library of a library table
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibEntry {
    /// The nickname symbols and footprints name it by (`Device` in `Device:R`)
    pub name: String,
    /// `KiCad`, `Legacy`, `Table`, ...
    pub kind: String,
    /// The file or directory, with `${VAR}` references as written
    pub uri: String,
    /// The table's description of it
    pub descr: String,
    /// Marked `(disabled)`: KiCad does not load it
    pub disabled: bool,
}

/// The libraries of a `sym-lib-table` or `fp-lib-table` file, in their order; `None` when it is neither
pub fn read_lib_table(text: &str) -> Option<Vec<LibEntry>> {
    let parsed = sexpr::parse(text).ok()?;
    let root = parsed.iter().find(|r| {
        [LibKind::Symbol, LibKind::Footprint]
            .iter()
            .any(|k| r.head() == Some(k.table_head()))
    })?;
    Some(
        root.children("lib")
            .filter_map(|l| {
                Some(LibEntry {
                    name: l.value("name")?.to_string(),
                    kind: l.value("type").unwrap_or("KiCad").to_string(),
                    uri: l.value("uri")?.to_string(),
                    descr: l.value("descr").unwrap_or("").to_string(),
                    disabled: l.flag("disabled").unwrap_or(false),
                })
            })
            .collect(),
    )
}

/// `uri` with every `${VAR}` (and `$(VAR)`) replaced through `var`; `None` when one is not known
pub fn expand(uri: &str, var: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let mut out = String::new();
    let mut rest = uri;
    while let Some(i) = rest.find('$') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let close = match after.chars().next() {
            Some('{') => '}',
            Some('(') => ')',
            _ => {
                out.push('$');
                rest = after;
                continue;
            }
        };
        let end = after.find(close)?;
        out.push_str(&var(&after[1..end])?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// KiCad's configuration directory of the newest KiCad that has one: `$KICAD_CONFIG_HOME`, else
/// `<config>/kicad/<version>` (`$XDG_CONFIG_HOME` or `~/.config` on Linux, `~/Library/Preferences` on macOS,
/// `%APPDATA%` on Windows). One with a library table wins over one with only `kicad_common.json`.
pub fn kicad_config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("KICAD_CONFIG_HOME") {
        return Some(d.into());
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let bases: Vec<PathBuf> = [
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        home.as_ref().map(|h| h.join(".config")),
        home.as_ref().map(|h| h.join("Library/Preferences")),
        std::env::var_os("APPDATA").map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    .map(|b| b.join("kicad"))
    .collect();
    let dirs: Vec<PathBuf> = VERSIONS
        .iter()
        .flat_map(|v| bases.iter().map(move |b| b.join(format!("{v}.0"))))
        .collect();
    let has = |d: &PathBuf, files: &[&str]| files.iter().any(|f| d.join(f).is_file());
    dirs.iter()
        .find(|d| has(d, &["sym-lib-table", "fp-lib-table"]))
        .or_else(|| dirs.iter().find(|d| has(d, &["kicad_common.json"])))
        .cloned()
}

/// The data directories (`symbols`, `footprints`, ...) of installed KiCads, where the stock libraries are
fn stock_data_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = ["/usr/share/kicad", "/usr/local/share/kicad"]
        .iter()
        .map(PathBuf::from)
        .collect();
    dirs.push("/Applications/KiCad/KiCad.app/Contents/SharedSupport".into());
    if let Some(p) = std::env::var_os("ProgramFiles") {
        for v in VERSIONS {
            dirs.push(PathBuf::from(&p).join(format!("KiCad/{v}.0/share/kicad")));
        }
    }
    dirs.into_iter().filter(|d| d.is_dir()).collect()
}

/// The values of `${VAR}` in library URIs, as KiCad sees them for one project
#[derive(Clone, Debug, Default)]
pub struct KicadVars {
    /// From `kicad_common.json` and `KIPRJMOD`; the environment goes first
    set: HashMap<String, String>,
    /// The stock library directories, for `KICAD<N>_SYMBOL_DIR` and the like nothing else sets
    stock: HashMap<String, String>,
}

impl KicadVars {
    /// The variables of the project in `dir`, with `config` KiCad's configuration directory ([`kicad_config_dir`])
    pub fn new(dir: &Path, config: Option<&Path>) -> KicadVars {
        let mut set = HashMap::new();
        if let Some(c) = config
            && let Ok(text) = std::fs::read_to_string(c.join("kicad_common.json"))
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(map) = json["environment"]["vars"].as_object()
        {
            for (k, v) in map {
                if let Some(v) = v.as_str() {
                    set.insert(k.clone(), v.to_string());
                }
            }
        }
        set.insert("KIPRJMOD".into(), dir.to_string_lossy().into_owned());
        let mut stock = HashMap::new();
        if let Some(data) = stock_data_dirs().first() {
            for (var, sub) in [
                ("SYMBOL_DIR", "symbols"),
                ("FOOTPRINT_DIR", "footprints"),
                ("3DMODEL_DIR", "3dmodels"),
                ("TEMPLATE_DIR", "template"),
            ] {
                let path = data.join(sub).to_string_lossy().into_owned();
                for v in VERSIONS {
                    stock.insert(format!("KICAD{v}_{var}"), path.clone());
                }
            }
        }
        KicadVars { set, stock }
    }

    /// The value of `name`: the environment, then KiCad's settings and `KIPRJMOD`, then the stock libraries
    pub fn get(&self, name: &str) -> Option<String> {
        std::env::var(name)
            .ok()
            .or_else(|| self.set.get(name).cloned())
            .or_else(|| self.stock.get(name).cloned())
    }

    /// `uri` with its variables replaced, `None` when one is not set
    pub fn expand(&self, uri: &str) -> Option<String> {
        expand(uri, &|v| self.get(v))
    }
}

/// Why a library, or something in one, was not found
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibError {
    /// The id has no `Lib:` part
    NoNickname(String),
    /// No table lists the nickname
    NotInTable { nick: String, kind: LibKind },
    /// The table lists it as disabled
    Disabled { nick: String },
    /// Its URI uses a variable that is not set
    Variable { nick: String, uri: String },
    /// A library type other than `KiCad` (`Legacy`, `Eagle`, ...)
    Unsupported { nick: String, kind: String },
    /// The library file or directory can't be read
    Unreadable { nick: String, path: PathBuf },
    /// The library has no such symbol or footprint
    NotFound { name: String, path: PathBuf },
    /// The footprint file is there but is no footprint KiCad 8+ wrote
    Invalid { path: PathBuf, message: String },
}

impl fmt::Display for LibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LibError::NoNickname(id) => write!(f, "`{id}` names no library"),
            LibError::NotInTable { nick, kind } => {
                write!(f, "library {nick} is in no {}", kind.table_file())
            }
            LibError::Disabled { nick } => write!(f, "library {nick} is disabled in its table"),
            LibError::Variable { nick, uri } => {
                write!(f, "library {nick}: a variable of {uri} is not set")
            }
            LibError::Unsupported { nick, kind } => {
                write!(f, "library {nick}: type {kind} is not read, only KiCad")
            }
            LibError::Unreadable { nick, path } => {
                write!(f, "library {nick}: can't read {}", path.display())
            }
            LibError::NotFound { name, path } => write!(f, "no {name} in {}", path.display()),
            LibError::Invalid { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for LibError {}

/// The library tables of one kind for a project: its own table, then the global one, nested tables in place
#[derive(Clone, Debug)]
pub struct LibTables {
    kind: LibKind,
    /// Every library in search order: the first of a nickname wins
    entries: Vec<LibEntry>,
    vars: KicadVars,
    /// The table files looked at: the project's and the global one (there or not), and every nested table
    files: Vec<PathBuf>,
}

impl LibTables {
    /// The `kind` libraries of the project in `dir`, with `config` KiCad's configuration directory
    /// ([`kicad_config_dir`]) for the global table and `kicad_common.json`, `None` for none
    pub fn new(kind: LibKind, dir: &Path, config: Option<&Path>) -> LibTables {
        let vars = KicadVars::new(dir, config);
        let mut entries = Vec::new();
        let mut files = Vec::new();
        for t in [Some(dir), config].into_iter().flatten() {
            read_nested(
                &t.join(kind.table_file()),
                &vars,
                0,
                &mut entries,
                &mut files,
            );
        }
        LibTables {
            kind,
            entries,
            vars,
            files,
        }
    }

    pub fn kind(&self) -> LibKind {
        self.kind
    }

    /// Library `nick` at `path`, searched before every table's (a library given on the command line)
    pub fn add_first(&mut self, nick: &str, path: &Path) {
        self.entries.insert(
            0,
            LibEntry {
                name: nick.to_string(),
                kind: "KiCad".into(),
                uri: path.to_string_lossy().into_owned(),
                descr: String::new(),
                disabled: false,
            },
        );
    }

    pub fn vars(&self) -> &KicadVars {
        &self.vars
    }

    /// The table files looked at, whether they were there or not: the project's, the global one, every nested
    /// table listed; what to watch for the tables to change
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Every library KiCad would load, in search order, one per nickname (disabled ones left out)
    pub fn libraries(&self) -> Vec<&LibEntry> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .filter(|e| seen.insert(e.name.as_str()))
            .filter(|e| !e.disabled)
            .collect()
    }

    /// The table entry of library `nick`
    pub fn entry(&self, nick: &str) -> Result<&LibEntry, LibError> {
        let e = self
            .entries
            .iter()
            .find(|e| e.name == nick)
            .ok_or_else(|| LibError::NotInTable {
                nick: nick.to_string(),
                kind: self.kind,
            })?;
        if e.disabled {
            return Err(LibError::Disabled {
                nick: nick.to_string(),
            });
        }
        Ok(e)
    }

    /// The file or directory of library `nick`, its variables replaced; not checked to exist
    pub fn path(&self, nick: &str) -> Result<PathBuf, LibError> {
        let e = self.entry(nick)?;
        if !e.kind.eq_ignore_ascii_case("KiCad") {
            return Err(LibError::Unsupported {
                nick: nick.to_string(),
                kind: e.kind.clone(),
            });
        }
        self.vars
            .expand(&e.uri)
            .map(PathBuf::from)
            .ok_or_else(|| LibError::Variable {
                nick: nick.to_string(),
                uri: e.uri.clone(),
            })
    }
}

/// The libraries of the table at `path` into `out`, a nested table's (`(type "Table")`) where it is listed
fn read_nested(
    path: &Path,
    vars: &KicadVars,
    depth: usize,
    out: &mut Vec<LibEntry>,
    files: &mut Vec<PathBuf>,
) {
    files.push(path.to_path_buf());
    let Some(entries) = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| read_lib_table(&t))
    else {
        return;
    };
    for e in entries {
        if e.kind.eq_ignore_ascii_case("Table") {
            if depth < 8
                && !e.disabled
                && let Some(p) = vars.expand(&e.uri)
            {
                read_nested(Path::new(&p), vars, depth + 1, out, files);
            }
        } else {
            out.push(e);
        }
    }
}

/// `Lib:Name` into its nickname and name
fn split_id(lib_id: &str) -> Result<(&str, &str), LibError> {
    lib_id
        .split_once(':')
        .filter(|(n, _)| !n.is_empty())
        .ok_or_else(|| LibError::NoNickname(lib_id.to_string()))
}

/// One symbol library on disk: a `.kicad_sym` file, or a KiCad 9+ `.kicad_symdir` directory with one `.kicad_sym`
/// per symbol named after it. Every file is read once.
#[derive(Clone, Debug)]
pub struct SymbolLibrary {
    path: PathBuf,
    /// The text of each file read, `None` for one that can't be
    files: HashMap<PathBuf, Option<String>>,
}

impl SymbolLibrary {
    /// The library at `path`, not read yet
    pub fn new(path: &Path) -> SymbolLibrary {
        SymbolLibrary {
            path: path.to_path_buf(),
            files: HashMap::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A directory library (`.kicad_symdir`)
    pub fn is_dir(&self) -> bool {
        self.path.extension().is_some_and(|e| e == "kicad_symdir")
    }

    /// The nickname KiCad gives the library by default: its file or directory name without the extension
    pub fn default_nickname(&self) -> String {
        self.path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
    }

    /// The library itself (its file, or its directory, where a symbol file may come or go) and every file read
    /// from it, sorted
    pub fn files(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = std::iter::once(self.path.clone())
            .chain(self.files.keys().cloned())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn text(&mut self, path: &Path) -> Option<String> {
        self.files
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read_to_string(path).ok())
            .clone()
    }

    /// The names of the library's symbols, sorted; `None` when it can't be read or is no symbol library
    pub fn names(&mut self) -> Option<Vec<String>> {
        let mut names = Vec::new();
        if self.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&self.path)
                .ok()?
                .filter_map(|e| Some(e.ok()?.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "kicad_sym"))
                .collect();
            files.sort();
            for f in files {
                if let Some(n) = self.text(&f).and_then(|t| crate::symlib::symbol_names(&t)) {
                    names.extend(n);
                }
            }
        } else {
            let path = self.path.clone();
            names = crate::symlib::symbol_names(&self.text(&path)?)?;
        }
        names.sort_by(|a, b| erc_model::natural_cmp(a, b));
        names.dedup();
        Some(names)
    }

    /// The file that defines symbol `name`: the library's own file, or in a directory library `Name.kicad_sym`, else
    /// the file that defines it (a name KiCad escaped for the file system); `None` when there is none
    pub fn file_of(&mut self, name: &str) -> Option<PathBuf> {
        if !self.is_dir() {
            return self.path.is_file().then(|| self.path.clone());
        }
        let named = self.path.join(format!("{name}.kicad_sym"));
        if named.is_file() {
            return Some(named);
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.path)
            .ok()?
            .filter_map(|e| Some(e.ok()?.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "kicad_sym"))
            .collect();
        files.sort();
        files.into_iter().find(|f| {
            self.text(f)
                .and_then(|t| crate::symlib::symbol_names(&t))
                .is_some_and(|names| names.iter().any(|s| s == name))
        })
    }

    /// The fields of symbol `name`, a derived symbol's on top of its parent's ([`crate::symlib::resolve_fields`])
    pub fn fields(&mut self, name: &str) -> Result<Vec<(String, String)>, LibError> {
        let not_found = |path: &Path| LibError::NotFound {
            name: format!("symbol {name}"),
            path: path.to_path_buf(),
        };
        let nick = self.default_nickname();
        let unreadable = |path: &Path| LibError::Unreadable {
            nick: nick.clone(),
            path: path.to_path_buf(),
        };
        let path = self.path.clone();
        if self.is_dir() {
            if !path.is_dir() {
                return Err(unreadable(&path));
            }
            let read = |n: &str| {
                let f = self.file_of(n)?;
                self.text(&f)
            };
            return crate::symlib::resolve_fields(name, read).ok_or_else(|| not_found(&path));
        }
        let text = self.text(&path).ok_or_else(|| unreadable(&path))?;
        crate::symlib::symbol_fields(&text, name).ok_or_else(|| not_found(&path))
    }
}

/// The symbol libraries of a project: the fields of a library symbol by its `Lib:Name`
#[derive(Clone, Debug)]
pub struct SymbolLibraries {
    tables: LibTables,
    libraries: HashMap<PathBuf, SymbolLibrary>,
}

impl SymbolLibraries {
    /// The symbol libraries of the project in `dir` ([`LibTables::new`])
    pub fn new(dir: &Path, config: Option<&Path>) -> SymbolLibraries {
        SymbolLibraries {
            tables: LibTables::new(LibKind::Symbol, dir, config),
            libraries: HashMap::new(),
        }
    }

    pub fn tables(&self) -> &LibTables {
        &self.tables
    }

    /// The symbol library `path` (a `.kicad_sym` or `.kicad_symdir`) as library `nick`, searched before the tables
    pub fn add_first(&mut self, nick: &str, path: &Path) {
        self.tables.add_first(nick, path);
    }

    /// The table files looked at ([`LibTables::files`]) and the files of every library used
    /// ([`SymbolLibrary::files`]), sorted
    pub fn files(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = self.tables.files().to_vec();
        out.extend(self.libraries.values().flat_map(SymbolLibrary::files));
        out.sort();
        out.dedup();
        out
    }

    /// Library `nick`, through the tables
    pub fn library(&mut self, nick: &str) -> Result<&mut SymbolLibrary, LibError> {
        let path = self.tables.path(nick)?;
        Ok(self
            .libraries
            .entry(path.clone())
            .or_insert_with(|| SymbolLibrary::new(&path)))
    }

    /// The fields of library symbol `lib_id` (`Lib:Name`), a derived symbol's on top of its parent's
    pub fn fields(&mut self, lib_id: &str) -> Result<Vec<(String, String)>, LibError> {
        let (nick, name) = split_id(lib_id)?;
        let lib = self.library(nick)?;
        lib.fields(name).map_err(|e| match e {
            LibError::Unreadable { path, .. } => LibError::Unreadable {
                nick: nick.to_string(),
                path,
            },
            e => e,
        })
    }

    /// The file that holds library symbol `lib_id` (`Lib:Name`) and its name there, to write it
    /// ([`crate::symlib::set_property`]): the library's `.kicad_sym`, or the symbol's file in a `.kicad_symdir`
    /// ([`SymbolLibrary::file_of`]). A flat library's file is checked to exist, not to hold the symbol.
    pub fn symbol_file(&mut self, lib_id: &str) -> Result<(PathBuf, String), LibError> {
        let (nick, name) = split_id(lib_id)?;
        let lib = self.library(nick)?;
        let path = lib.path().to_path_buf();
        if lib.is_dir() && !path.is_dir() || !lib.is_dir() && !path.is_file() {
            return Err(LibError::Unreadable {
                nick: nick.to_string(),
                path,
            });
        }
        let file = lib.file_of(name).ok_or_else(|| LibError::NotFound {
            name: format!("symbol {name}"),
            path,
        })?;
        Ok((file, name.to_string()))
    }
}

/// A library footprint as read from its `.kicad_mod`
#[derive(Clone, Debug, PartialEq)]
pub struct LibFootprint {
    /// The file it was read from
    pub path: PathBuf,
    /// The footprint in its own frame, comparable with a board copy ([`erc_model::Layout::local_footprint`])
    pub footprint: LocalFootprint,
    /// What the reader did not know in it
    pub notes: Vec<ImportNote>,
}

/// The footprint libraries of a project: a library footprint by its `Lib:Name`
#[derive(Clone, Debug)]
pub struct FootprintLibraries {
    tables: LibTables,
    read: HashMap<String, Result<LibFootprint, LibError>>,
}

impl FootprintLibraries {
    /// The footprint libraries of the project in `dir` ([`LibTables::new`])
    pub fn new(dir: &Path, config: Option<&Path>) -> FootprintLibraries {
        FootprintLibraries {
            tables: LibTables::new(LibKind::Footprint, dir, config),
            read: HashMap::new(),
        }
    }

    pub fn tables(&self) -> &LibTables {
        &self.tables
    }

    /// The `.pretty` directory `path` as library `nick`, searched before the tables
    pub fn add_first(&mut self, nick: &str, path: &Path) {
        self.tables.add_first(nick, path);
        self.read
            .retain(|id, _| !id.starts_with(&format!("{nick}:")));
    }

    /// The table files looked at ([`LibTables::files`]) and the `.kicad_mod` file of every footprint asked for,
    /// read or looked for in vain (a footprint that comes later is a change too), sorted
    pub fn files(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = self.tables.files().to_vec();
        out.extend(self.read.iter().filter_map(|(id, r)| match r {
            Ok(f) => Some(f.path.clone()),
            Err(_) => self.path(id).ok(),
        }));
        out.sort();
        out.dedup();
        out
    }

    /// The `.kicad_mod` file of `lib_id` (`Lib:Name`): `Name.kicad_mod` in the library's `.pretty` directory; not
    /// checked to exist
    pub fn path(&self, lib_id: &str) -> Result<PathBuf, LibError> {
        let (nick, name) = split_id(lib_id)?;
        Ok(self.tables.path(nick)?.join(format!("{name}.kicad_mod")))
    }

    /// The footprint `lib_id` (`Lib:Name`), parsed; read once, later calls answer from what was read
    pub fn footprint(&mut self, lib_id: &str) -> Result<&LibFootprint, LibError> {
        if !self.read.contains_key(lib_id) {
            let found = self.read_footprint(lib_id);
            self.read.insert(lib_id.to_string(), found);
        }
        self.read[lib_id].as_ref().map_err(Clone::clone)
    }

    fn read_footprint(&self, lib_id: &str) -> Result<LibFootprint, LibError> {
        let (nick, name) = split_id(lib_id)?;
        let dir = self.tables.path(nick)?;
        if !dir.is_dir() {
            return Err(LibError::Unreadable {
                nick: nick.to_string(),
                path: dir,
            });
        }
        let path = dir.join(format!("{name}.kicad_mod"));
        let text = std::fs::read_to_string(&path).map_err(|_| LibError::NotFound {
            name: format!("footprint {name}"),
            path: dir.clone(),
        })?;
        let (footprint, notes) =
            crate::footprint::read(&text, &path.to_string_lossy()).map_err(|e| {
                LibError::Invalid {
                    path: path.clone(),
                    message: match e {
                        erc_model::ImportError::Invalid { message, .. } => message,
                        e => e.to_string(),
                    },
                }
            })?;
        Ok(LibFootprint {
            path,
            footprint,
            notes,
        })
    }

    /// The footprint names in library `nick` (its `.kicad_mod` files), sorted
    pub fn names(&self, nick: &str) -> Result<Vec<String>, LibError> {
        let dir = self.tables.path(nick)?;
        let unreadable = || LibError::Unreadable {
            nick: nick.to_string(),
            path: dir.clone(),
        };
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .map_err(|_| unreadable())?
            .filter_map(|e| {
                let p = e.ok()?.path();
                (p.extension()? == "kicad_mod")
                    .then(|| p.file_stem()?.to_str().map(String::from))?
            })
            .collect();
        names.sort_by(|a, b| erc_model::natural_cmp(a, b));
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_and_variables() {
        let t = r#"(sym_lib_table (version 7)
            (lib (name "Lib")(type "KiCad")(uri "${KIPRJMOD}/lib/Lib.kicad_sym")(options "")(descr ""))
            (lib (name "Device")(type "KiCad")(uri "${KICAD10_SYMBOL_DIR}/Device.kicad_sym")(options "")(descr "")))"#;
        let libs = read_lib_table(t).unwrap();
        assert_eq!(libs.len(), 2);
        assert_eq!(libs[0].name, "Lib");
        let var = |v: &str| (v == "KIPRJMOD").then(|| "/p".to_string());
        assert_eq!(
            expand(&libs[0].uri, &var).as_deref(),
            Some("/p/lib/Lib.kicad_sym")
        );
        assert_eq!(expand(&libs[1].uri, &var), None);
        assert_eq!(expand("$(KIPRJMOD)/a$b", &var).as_deref(), Some("/p/a$b"));
        assert!(read_lib_table("(kicad_sch)").is_none());
    }

    #[test]
    fn footprint_table_flags() {
        let t = r#"(fp_lib_table (version 7)
            (lib (name "A")(type "KiCad")(uri "/a.pretty")(options "")(descr "first")(disabled))
            (lib (name "B")(type "Legacy")(uri "/b.mod")(options "")(descr "")))"#;
        let libs = read_lib_table(t).unwrap();
        assert!(libs[0].disabled);
        assert_eq!(libs[0].descr, "first");
        assert!(!libs[1].disabled);
        assert_eq!(libs[1].kind, "Legacy");
    }

    /// A directory library lists the symbols of its files and finds one whose file is not named after it; a flat
    /// file lists its top-level symbols
    #[test]
    fn symbol_library_names_and_fields() {
        let dir = std::env::temp_dir().join(format!("erc-symlib-{}", std::process::id()));
        let symdir = dir.join("Lib.kicad_symdir");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&symdir).unwrap();
        let file = |name: &str, extra: &str| {
            format!(
                "(kicad_symbol_lib (version 20241209)\n  (symbol \"{name}\" {extra} (property \"Value\" \"{name}\") (symbol \"{name}_1_1\")))\n"
            )
        };
        std::fs::write(symdir.join("B.kicad_sym"), file("B", "")).unwrap();
        std::fs::write(symdir.join("A_B.kicad_sym"), file("A/B", "(extends \"B\")")).unwrap();
        std::fs::write(symdir.join("notes.txt"), "not a symbol").unwrap();
        let mut lib = SymbolLibrary::new(&symdir);
        assert_eq!(lib.default_nickname(), "Lib");
        assert_eq!(lib.names().unwrap(), ["A/B", "B"]);
        let f = lib.fields("A/B").unwrap();
        assert!(f.contains(&("Value".into(), "A/B".into())), "{f:?}");
        assert!(matches!(lib.fields("C"), Err(LibError::NotFound { .. })));
        let flat = dir.join("Flat.kicad_sym");
        std::fs::write(&flat, file("Z", "")).unwrap();
        assert_eq!(SymbolLibrary::new(&flat).names().unwrap(), ["Z"]);
        assert!(
            SymbolLibrary::new(&dir.join("None.kicad_sym"))
                .names()
                .is_none()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
