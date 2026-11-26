//! `berc fp check --lib`: every symbol of a symbol library that names its MPNs, each (library footprint hash,
//! MPN) pair judged against the reviews a part source holds, before any board uses the symbol. A generic symbol
//! (MPNs on the instances) is reviewed on each board instead.

use super::check::field_of;
use super::pairs::{self, Overall, Pair};
use erc_kicad::libtable::{FootprintLibraries, LibError, SymbolLibraries, SymbolLibrary};
use erc_model::fphash;
use erc_parts::PartsLock;
use erc_parts::lock::{LockedReview, PartKey};
use erc_parts::source::{MPNS_PER_CALL_MAX, Source, SourceError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// What `berc fp check --lib` found for one library symbol
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SymbolStatus {
    /// A review of the footprint's hash covers every MPN of the symbol
    Ok,
    /// Some MPN is covered only by a review of another hash: the footprint changed since
    Changed,
    /// Some MPN is covered only by reviews of another version of the review hash
    OtherVersion,
    /// Some MPNs covered, others not yet
    Partial,
    /// No MPN covered
    Uncovered,
    /// The `Footprint` field names a footprint the footprint libraries don't give
    Missing,
    /// A footprint but no MPN: a generic symbol, reviewed on each board with the MPNs its instances carry
    NoMpn,
    /// No part source given to judge the MPNs against
    Unjudged,
    /// The symbol sets no footprint (a generic part, a power symbol): nothing to review, not a failure
    NoFootprint,
}

impl SymbolStatus {
    /// The footprint needs a review: some MPN not covered by a review of its hash now
    pub fn needs_review(self) -> bool {
        matches!(
            self,
            SymbolStatus::Changed
                | SymbolStatus::OtherVersion
                | SymbolStatus::Partial
                | SymbolStatus::Uncovered
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            SymbolStatus::Ok => "ok",
            SymbolStatus::Changed => "changed",
            SymbolStatus::OtherVersion => "other version",
            SymbolStatus::Partial => "partial",
            SymbolStatus::Uncovered => "uncovered",
            SymbolStatus::Missing => "missing",
            SymbolStatus::NoMpn => "no MPN",
            SymbolStatus::Unjudged => "unjudged",
            SymbolStatus::NoFootprint => "no footprint",
        }
    }

    /// Nothing to do here: reviewed, or nothing to review on the library side
    pub fn is_ok(self) -> bool {
        matches!(
            self,
            SymbolStatus::Ok | SymbolStatus::NoMpn | SymbolStatus::NoFootprint
        )
    }

    fn of(o: Overall) -> SymbolStatus {
        match o {
            Overall::Ok => SymbolStatus::Ok,
            Overall::Changed => SymbolStatus::Changed,
            Overall::OtherVersion => SymbolStatus::OtherVersion,
            // A pair the source gave no entry for is one it did not cover
            Overall::Partial => SymbolStatus::Partial,
            Overall::NotInLock | Overall::Uncovered => SymbolStatus::Uncovered,
        }
    }

    const ALL: [SymbolStatus; 9] = [
        SymbolStatus::Ok,
        SymbolStatus::Changed,
        SymbolStatus::OtherVersion,
        SymbolStatus::Partial,
        SymbolStatus::Uncovered,
        SymbolStatus::Missing,
        SymbolStatus::Unjudged,
        SymbolStatus::NoMpn,
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
    /// The symbol's MPNs (`Mpn1`, `Mpn2`, ...), in field order
    #[serde(default)]
    pub mpns: Vec<String>,
    /// Each MPN against the source's reviews of [`hash`](Self::hash)
    #[serde(default)]
    pub pairs: Vec<Pair>,
    /// The `.kicad_mod` read
    pub footprint_file: Option<PathBuf>,
    /// The symbol's `Datasheet` field, what the footprint is reviewed against
    #[serde(default)]
    pub datasheet: Option<String>,
    /// Why the footprint or the symbol could not be read
    pub note: Option<String>,
    /// The symbol's own library (the nickname of `symbol`), when `allowed` named some and it is none of them
    #[serde(default)]
    pub symbol_library_disallowed: Option<String>,
    /// The library its `Footprint` field names, when `allowed` named some and it is none of them
    #[serde(default)]
    pub footprint_library_disallowed: Option<String>,
}

impl SymbolEntry {
    /// Ok, or nothing to review, and no library outside what is allowed
    pub fn is_ok(&self) -> bool {
        self.status.is_ok()
            && self.symbol_library_disallowed.is_none()
            && self.footprint_library_disallowed.is_none()
    }
}

/// `nickname` matches one of `patterns`: an exact name, or (a pattern ending `*`) a prefix. No patterns: always
/// allowed, there is no rule
pub(super) fn lib_allowed(nickname: &str, patterns: &[String]) -> bool {
    patterns.is_empty()
        || patterns.iter().any(|p| match p.strip_suffix('*') {
            Some(prefix) => nickname.starts_with(prefix),
            None => nickname == p,
        })
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

/// Every symbol of some symbol libraries, its MPNs against the reviews of its library footprint's hash
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibCheck {
    /// The part source judged against, `None` when none was given
    pub source: Option<String>,
    pub libraries: Vec<CheckedLibrary>,
    /// Library by library in the order given, symbols in natural order
    pub symbols: Vec<SymbolEntry>,
    /// Every review a symbol's pairs name, by id
    #[serde(default)]
    pub reviews: Vec<LockedReview>,
    /// Every file this was made from, sorted: the library tables looked at, the symbol libraries checked and the
    /// files read from them, the footprint files used or looked for in vain. When one of them changes the check is
    /// out of date (the review tab watches them)
    #[serde(default)]
    pub files: Vec<PathBuf>,
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
    /// A `--symbols` path that is no symbol library and has none in it
    NoSymbols(PathBuf),
    /// The part source gave no answer
    Source(SourceError),
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
            LibCheckError::NoSymbols(p) => write!(
                f,
                "{}: no symbol library (.kicad_sym, .kicad_symdir) there or in it",
                p.display()
            ),
            LibCheckError::Source(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for LibCheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LibCheckError::NoLibrary(_, e) => Some(e),
            LibCheckError::Source(e) => Some(e),
            _ => None,
        }
    }
}

/// Where library symbols and footprints are found besides a project's own tables: libraries given by path,
/// searched first, and the global tables in KiCad's configuration directory
#[derive(Clone, Debug, Default)]
pub struct LibrarySearch {
    /// KiCad's configuration directory ([`super::kicad_config_dir`]), for the global tables
    pub config: Option<PathBuf>,
    /// Symbol libraries searched before the tables: each a `.kicad_sym` file, a `.kicad_symdir` directory or a
    /// folder of them, named by their file names
    pub symbols: Vec<PathBuf>,
    /// Footprint libraries searched before the tables: each a `.pretty` directory, or a folder of them, named by
    /// their directory names
    pub footprints: Vec<PathBuf>,
    /// The library nicknames a symbol and the footprint its `Footprint` field names may be in (an exact name, or a
    /// prefix with a trailing `*`, [`lib_allowed`]): `berc fp check` (board and `--lib`) and `berc fp todo` report
    /// any other.
    /// Empty: no rule
    pub allowed: Vec<String>,
}

impl LibrarySearch {
    /// The global tables of the KiCad configuration this PC has, no libraries by path
    pub fn kicad() -> LibrarySearch {
        LibrarySearch {
            config: super::kicad_config_dir(),
            ..LibrarySearch::default()
        }
    }

    /// The symbol and footprint libraries of the project in `dir`: the libraries by path first, then the
    /// project's tables, then the global ones
    pub fn open(&self, dir: &Path) -> Result<(SymbolLibraries, FootprintLibraries), LibCheckError> {
        let mut syms = SymbolLibraries::new(dir, self.config.as_deref());
        let mut fps = FootprintLibraries::new(dir, self.config.as_deref());
        // Later ones go first, so the first one given wins as in a table
        for s in self.symbols.iter().rev() {
            let libs = libraries_in(s, is_symbol_library);
            if libs.is_empty() {
                return Err(LibCheckError::NoSymbols(s.clone()));
            }
            for l in libs.iter().rev() {
                syms.add_first(&SymbolLibrary::new(l).default_nickname(), l);
            }
        }
        for f in self.footprints.iter().rev() {
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
        Ok((syms, fps))
    }
}

/// What `berc fp check --lib` checks and where it finds the libraries
#[derive(Clone, Debug, Default)]
pub struct LibCheckOptions {
    /// The symbol libraries: each a `.kicad_sym` file, a `.kicad_symdir` directory, a folder holding such
    /// libraries (each of them), or a nickname in the project's or the global `sym-lib-table`
    pub libraries: Vec<String>,
    /// The project directory whose `sym-lib-table` and `fp-lib-table` are read, besides the global ones
    pub project: PathBuf,
    pub search: LibrarySearch,
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

/// Checks every symbol of `opts.libraries`: its MPNs against the reviews `source` holds for its library footprint's
/// hash. No source: the footprints are read and hashed, the MPNs not judged ([`SymbolStatus::Unjudged`]).
pub fn check_libraries(
    opts: &LibCheckOptions,
    source: Option<&dyn Source>,
) -> Result<LibCheck, LibCheckError> {
    let (mut sym_tables, mut fps) = opts.search.open(&opts.project)?;
    let mut libraries = Vec::new();
    for arg in &opts.libraries {
        libraries.extend(resolve(arg, &mut sym_tables)?);
    }
    let mut checked = Vec::new();
    let mut symbols = Vec::new();
    let mut files: Vec<PathBuf> = sym_tables.files();
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
            let fields = lib.fields(&name);
            symbols.push(read_symbol(
                &format!("{nick}:{name}"),
                fields,
                &mut fps,
                &opts.search.allowed,
            ));
        }
        files.extend(lib.files());
    }
    let reviews = match source {
        Some(source) => judge_all(&mut symbols, source).map_err(LibCheckError::Source)?,
        None => {
            for e in symbols
                .iter_mut()
                .filter(|e| e.status == SymbolStatus::Uncovered)
            {
                e.status = SymbolStatus::Unjudged;
            }
            Vec::new()
        }
    };
    files.extend(fps.files());
    files.sort();
    files.dedup();
    Ok(LibCheck {
        source: source.map(Source::name),
        libraries: checked,
        symbols,
        reviews,
        files,
    })
}

/// The symbols' pairs judged against `source`: their MPNs' parts asked once, then each hash's reviews once. The
/// reviews the pairs name, by id.
fn judge_all(
    symbols: &mut [SymbolEntry],
    source: &dyn Source,
) -> Result<Vec<LockedReview>, SourceError> {
    let mut by_hash: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for e in symbols
        .iter()
        .filter(|e| e.status == SymbolStatus::Uncovered)
    {
        if let Some(h) = &e.hash {
            let mpns = by_hash.entry(h.clone()).or_default();
            mpns.extend(e.mpns.iter().cloned());
        }
    }
    let mut lock = PartsLock::default();
    let mut all: Vec<String> = by_hash.values().flatten().cloned().collect();
    all.sort();
    all.dedup();
    for chunk in all.chunks(MPNS_PER_CALL_MAX) {
        let keys: Vec<PartKey> = chunk.iter().map(|m| PartKey::mpn(m)).collect();
        lock.parts.extend(source.parts(&keys, &[])?.parts);
    }
    for (hash, mut mpns) in by_hash {
        mpns.sort();
        mpns.dedup();
        for chunk in mpns.chunks(MPNS_PER_CALL_MAX) {
            let a = source.reviews(&hash, chunk)?;
            lock.footprints.extend(a.pairs);
            lock.reviews.extend(a.reviews);
        }
    }
    lock.sort();
    for e in symbols
        .iter_mut()
        .filter(|e| e.status == SymbolStatus::Uncovered)
    {
        let Some(hash) = &e.hash else { continue };
        e.pairs = e
            .mpns
            .iter()
            .map(|m| pairs::judge(&lock, hash, m))
            .collect();
        if let Some(o) = pairs::overall(&e.pairs) {
            e.status = SymbolStatus::of(o);
        }
    }
    let mut reviews = lock.reviews;
    reviews.sort_by_key(|r| r.id);
    reviews.dedup_by_key(|r| r.id);
    Ok(reviews)
}

/// Library symbol `symbol` (`Lib:Name`) with its `fields` as read, its library footprint read and hashed, its
/// pairs not judged yet (status [`SymbolStatus::Uncovered`] when there are some); `allowed` lists the library
/// nicknames it and its footprint may be in ([`lib_allowed`])
pub(super) fn read_symbol(
    symbol: &str,
    fields: Result<Vec<(String, String)>, LibError>,
    fps: &mut FootprintLibraries,
    allowed: &[String],
) -> SymbolEntry {
    let mut e = SymbolEntry {
        symbol: symbol.to_string(),
        footprint: None,
        status: SymbolStatus::NoFootprint,
        hash: None,
        mpns: Vec::new(),
        pairs: Vec::new(),
        footprint_file: None,
        datasheet: None,
        note: None,
        symbol_library_disallowed: None,
        footprint_library_disallowed: None,
    };
    if let Some((lib, _)) = symbol.split_once(':')
        && !lib_allowed(lib, allowed)
    {
        e.symbol_library_disallowed = Some(lib.to_string());
    }
    let fields = match fields {
        Ok(f) => f,
        Err(why) => {
            e.status = SymbolStatus::Missing;
            e.note = Some(why.to_string());
            return e;
        }
    };
    e.footprint = field_of(&fields, "Footprint").map(String::from);
    if let Some((lib, _)) = e.footprint.as_deref().and_then(|f| f.split_once(':'))
        && !lib_allowed(lib, allowed)
    {
        e.footprint_library_disallowed = Some(lib.to_string());
    }
    e.datasheet = field_of(&fields, "Datasheet")
        .filter(|d| *d != "~")
        .map(|d| {
            // an mx3: link with the symbol's fields filled in
            erc_parts::mpn::datasheet_link::resolve(d, |n| {
                fields
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(n))
                    .map(|(_, v)| v.as_str())
            })
            .unwrap_or_else(|| d.to_owned())
        });
    for (k, v) in &fields {
        let v = v.trim();
        if erc_parts::mpn::mpn_field(k)
            && !v.is_empty()
            && v != "~"
            && !e.mpns.iter().any(|m| m == v)
        {
            e.mpns.push(v.to_owned());
        }
    }
    let Some(footprint) = e.footprint.clone() else {
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
    e.status = if e.mpns.is_empty() {
        SymbolStatus::NoMpn
    } else {
        SymbolStatus::Uncovered
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
    /// Symbols without a footprint are listed last under their own heading; `details`: every MPN under its symbol,
    /// covered ones too
    pub fn to_text_color(&self, details: bool) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let style = |s: SymbolStatus| match s {
            SymbolStatus::Ok => AnsiColor::Green.on_default(),
            SymbolStatus::Changed | SymbolStatus::Missing => AnsiColor::Red.on_default().bold(),
            SymbolStatus::OtherVersion | SymbolStatus::Partial | SymbolStatus::Uncovered => {
                AnsiColor::Yellow.on_default().bold()
            }
            SymbolStatus::Unjudged => AnsiColor::Cyan.on_default(),
            SymbolStatus::NoMpn | SymbolStatus::NoFootprint => Style::new().dimmed(),
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
            let now = e.hash.as_deref().unwrap_or("");
            match e.status {
                SymbolStatus::Ok | SymbolStatus::NoFootprint => {}
                SymbolStatus::Missing => {
                    let _ = write!(out, "  {dim}{}{dim:#}", e.note.as_deref().unwrap_or(""));
                }
                SymbolStatus::NoMpn => {
                    let _ = write!(
                        out,
                        "  {dim}no Mpn field: reviewed on each board with its instances' MPNs{dim:#}"
                    );
                }
                SymbolStatus::Unjudged => {
                    let _ = write!(
                        out,
                        "  {dim}hash now{dim:#} {now}  {dim}no part source to judge {} MPNs against{dim:#}",
                        e.mpns.len()
                    );
                }
                _ => {
                    let _ = write!(out, "  {dim}hash now{dim:#} {now}");
                }
            }
            // The footprint column's padding, where nothing follows it
            out.truncate(out.trim_end_matches(' ').len());
            for l in super::check::pair_notes(&e.pairs, details) {
                let _ = write!(out, "\n{:13} {:width$} {l}", "", "");
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
        let _ = write!(
            out,
            "{libs}: {} symbols: {}",
            self.symbols.len(),
            if counts.is_empty() {
                "none".to_string()
            } else {
                counts.join(", ")
            }
        );
        let disallowed = self
            .symbols
            .iter()
            .filter(|e| {
                e.symbol_library_disallowed.is_some() || e.footprint_library_disallowed.is_some()
            })
            .count();
        if disallowed > 0 {
            let _ = write!(out, "; {disallowed} outside the allowed libraries");
        }
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::lib_allowed;

    /// No patterns: no rule, everything is allowed; a pattern without a trailing `*` matches only that exact
    /// name; one with it matches names starting with what is before it
    #[test]
    fn exact_names_and_prefixes() {
        assert!(lib_allowed("VHRD_Package_QFP", &[]));
        assert!(lib_allowed("VHRD_Package_QFP", &["VHRD_*".into()]));
        assert!(lib_allowed("VHRD_", &["VHRD_*".into()]));
        assert!(!lib_allowed("Package_QFP", &["VHRD_*".into()]));
        assert!(lib_allowed("Test", &["Test".into(), "VHRD_*".into()]));
        assert!(!lib_allowed("Test2", &["Test".into()]));
        assert!(!lib_allowed("", &["VHRD_*".into()]));
    }
}
