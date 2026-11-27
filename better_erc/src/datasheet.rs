//! Stale `mx3:` Datasheet links rewritten, the library side of `berc fix datasheet-links` (`TOOL-19`, the fix of
//! `LIB-30`): each placed symbol whose link [`erc_parts::datasheet::stale`] calls stale gets the link of its own
//! MPN fields ([`erc_parts::datasheet::link_of`]), and each copy in a schematic's `lib_symbols` whose `Datasheet`
//! is an `mx3:` link takes the library symbol's text. Nothing is written while KiCad holds a file of the project.

use crate::fp::{LibCheckError, LibrarySearch};
use erc_checks::{CheckError, Project};
use erc_kicad::fields::{PlaceError, ValueChange, ValueEdit, edit_values, placed_symbols};
use erc_kicad::libtable::{LibError, SymbolLibraries};
use erc_kicad::lock::{self, KicadLock};
use erc_kicad::symlib::{self, EditError};
use erc_parts::datasheet::{link_of, stale};
use erc_parts::mpn::datasheet_link;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

/// What to fix
#[derive(Clone, Debug, Default)]
pub struct FixOptions {
    /// Write the files; else only plan
    pub write: bool,
    /// Where the library symbols are found besides the project's `sym-lib-table`: libraries by path first, then
    /// KiCad's global table ([`LibrarySearch::kicad`])
    pub libraries: LibrarySearch,
}

/// One placed symbol's link rewritten
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkFix {
    /// The sheet file, relative to the project
    pub file: String,
    /// The symbol's Reference field (`U5`; `U?` before annotation)
    pub reference: String,
    pub old: String,
    pub new: String,
}

/// One `lib_symbols` copy's `Datasheet` set to the library's
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyFix {
    /// The sheet file, relative to the project
    pub file: String,
    /// The copy's name in `lib_symbols` (`Lib:Name`)
    pub symbol: String,
    pub old: String,
    pub new: String,
}

/// A stale link or copy left as it is, with why
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    pub file: String,
    /// The placed symbol's Reference field, or the copy's name in `lib_symbols`
    pub what: String,
    pub why: String,
}

/// The links fixed
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fixed {
    pub links: Vec<LinkFix>,
    pub copies: Vec<CopyFix>,
    pub skipped: Vec<Skipped>,
    /// The files written; empty in a dry run, and with `--write` when nothing changed
    pub written: Vec<PathBuf>,
    pub write: bool,
}

/// Why the links could not be fixed
#[derive(Debug)]
pub enum FixError {
    /// The design could not be read
    Open(CheckError),
    /// The design has no KiCad schematic
    NotKicad,
    /// A symbol library given by path holds no library
    Libraries(LibCheckError),
    /// KiCad has a file of the project open: its next save would overwrite the fix
    Locked { file: PathBuf, lock: KicadLock },
    Io {
        file: PathBuf,
        error: std::io::Error,
    },
    /// A placed symbol's link could not be written
    Edit { file: PathBuf, error: PlaceError },
    /// A `lib_symbols` copy could not be read or written
    Copy { file: PathBuf, error: EditError },
}

impl fmt::Display for FixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixError::Open(e) => e.fmt(f),
            FixError::NotKicad => f.write_str("no KiCad schematic in this design"),
            FixError::Libraries(e) => e.fmt(f),
            FixError::Locked { file, lock } => write!(
                f,
                "{} is open in KiCad (its lock file {}{}): close it there first, else KiCad's next save \
                 overwrites the fix",
                file.display(),
                lock.path.display(),
                lock.owner
                    .as_deref()
                    .map(|o| format!(", {o}"))
                    .unwrap_or_default()
            ),
            FixError::Io { file, error } => write!(f, "{}: {error}", file.display()),
            FixError::Edit { file, error } => write!(f, "{}: {error}", file.display()),
            FixError::Copy { file, error } => write!(f, "{}: {error}", file.display()),
        }
    }
}

impl std::error::Error for FixError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FixError::Open(e) => Some(e),
            FixError::Io { error, .. } => Some(error),
            FixError::Edit { error, .. } => Some(error),
            FixError::Copy { error, .. } => Some(error),
            FixError::Libraries(e) => Some(e),
            FixError::NotKicad | FixError::Locked { .. } => None,
        }
    }
}

/// Plans the fix of the design at `path` (a directory, a `.kicad_pro` or a `.kicad_sch`) and, with
/// [`FixOptions::write`], writes it. Nothing is written when any file fails to edit, or while KiCad holds the
/// project file or a sheet file ([`lock::held_now`]).
pub fn fix(path: &Path, opts: &FixOptions) -> Result<Fixed, FixError> {
    let project = Project::open_without_parts(path).map_err(FixError::Open)?;
    let files: BTreeSet<&str> = project
        .imported
        .design
        .boards
        .iter()
        .flat_map(|b| b.sheets.iter().map(|s| s.file.as_str()))
        .filter(|f| f.ends_with(".kicad_sch"))
        .collect();
    if files.is_empty() {
        return Err(FixError::NotKicad);
    }
    let (mut libs, _) = opts
        .libraries
        .open(&project.dir)
        .map_err(FixError::Libraries)?;
    let mut out = Fixed {
        write: opts.write,
        ..Fixed::default()
    };
    let mut edited: BTreeMap<&str, String> = BTreeMap::new();
    for &file in &files {
        let path = project.dir.join(file);
        let text = std::fs::read_to_string(&path).map_err(|error| FixError::Io {
            file: path.clone(),
            error,
        })?;
        let new = fix_sheet(file, &text, &mut libs, &mut out).map_err(|e| match e {
            SheetError::Edit(error) => FixError::Edit {
                file: path.clone(),
                error,
            },
            SheetError::Copy(error) => FixError::Copy {
                file: path.clone(),
                error,
            },
        })?;
        if new != text {
            edited.insert(file, new);
        }
    }
    if opts.write && !edited.is_empty() {
        let mut held: Vec<PathBuf> = Vec::new();
        if project.main.ends_with(".kicad_pro") {
            held.push(project.dir.join(&project.main));
        }
        held.extend(files.iter().map(|f| project.dir.join(f)));
        if let Some((file, lock)) = held
            .into_iter()
            .find_map(|f| lock::held_now(&f).map(|l| (f, l)))
        {
            return Err(FixError::Locked { file, lock });
        }
        for (file, text) in &edited {
            let path = project.dir.join(file);
            std::fs::write(&path, text).map_err(|error| FixError::Io {
                file: path.clone(),
                error,
            })?;
            out.written.push(path);
        }
    }
    Ok(out)
}

/// Why one sheet could not be fixed
enum SheetError {
    Edit(PlaceError),
    Copy(EditError),
}

/// `text`, sheet `file`, with its stale links and `lib_symbols` copies fixed, each recorded in `out`
fn fix_sheet(
    file: &str,
    text: &str,
    libs: &mut SymbolLibraries,
    out: &mut Fixed,
) -> Result<String, SheetError> {
    let symbols = placed_symbols(text).map_err(SheetError::Edit)?;
    let mut edits = Vec::new();
    for s in &symbols {
        let prop = |n: &str| {
            s.properties
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, v)| v.as_str())
        };
        let Some(old) = prop("Datasheet") else {
            continue;
        };
        let names = || s.properties.iter().map(|(k, _)| k.as_str());
        if stale(old, names()).is_none() {
            continue;
        }
        let reference = prop("Reference").unwrap_or("?").to_owned();
        match link_of(names()) {
            Some(new) => {
                edits.push(ValueEdit {
                    symbol: s.uuid.clone(),
                    field: "Datasheet".to_owned(),
                    change: ValueChange::Put(new.clone()),
                });
                out.links.push(LinkFix {
                    file: file.to_owned(),
                    reference,
                    old: old.to_owned(),
                    new,
                });
            }
            None => out.skipped.push(Skipped {
                file: file.to_owned(),
                what: reference,
                why: "no MPN field to reference: add an Mpn1, or set the Datasheet by hand"
                    .to_owned(),
            }),
        }
    }
    let mut text = if edits.is_empty() {
        text.to_owned()
    } else {
        edit_values(text, &edits).map_err(SheetError::Edit)?
    };
    let copies: BTreeSet<(&str, &str)> = symbols
        .iter()
        .map(|s| (s.lib_symbol.as_str(), s.lib_id.as_str()))
        .collect();
    for (copy, lib_id) in copies {
        let old = match symlib::property(&text, copy, "Datasheet") {
            Ok(Some(old)) => old,
            Ok(None) | Err(EditError::NoSymbol(_)) => continue,
            Err(e) => return Err(SheetError::Copy(e)),
        };
        if !datasheet_link::is_link(&old) {
            continue;
        }
        let new = match library_datasheet(libs, lib_id) {
            Ok(new) => new,
            Err(why) => {
                out.skipped.push(Skipped {
                    file: file.to_owned(),
                    what: copy.to_owned(),
                    why,
                });
                continue;
            }
        };
        if new == old {
            continue;
        }
        text = symlib::set_property(&text, copy, "Datasheet", &new)
            .map_err(SheetError::Copy)?
            .text;
        out.copies.push(CopyFix {
            file: file.to_owned(),
            symbol: copy.to_owned(),
            old,
            new,
        });
    }
    Ok(text)
}

/// The `Datasheet` of library symbol `lib_id`, or why it can't be read
fn library_datasheet(libs: &mut SymbolLibraries, lib_id: &str) -> Result<String, String> {
    let fields = libs
        .fields(lib_id)
        .map_err(|e: LibError| format!("the library symbol {lib_id} can't be read: {e}"))?;
    Ok(fields
        .into_iter()
        .find(|(k, _)| k == "Datasheet")
        .map(|(_, v)| v)
        .unwrap_or_default())
}

/// The plan as text: one line per link and copy, the ones left alone with why, then the totals
pub fn render(f: &Fixed) -> String {
    let mut out = String::new();
    for l in &f.links {
        out += &format!("{} {}: {} -> {}\n", l.file, l.reference, l.old, l.new);
    }
    for c in &f.copies {
        out += &format!(
            "{} lib_symbols {}: {} -> {}\n",
            c.file, c.symbol, c.old, c.new
        );
    }
    for s in &f.skipped {
        out += &format!("{} {}: left alone, {}\n", s.file, s.what, s.why);
    }
    let files: BTreeSet<&str> = f
        .links
        .iter()
        .map(|l| l.file.as_str())
        .chain(f.copies.iter().map(|c| c.file.as_str()))
        .collect();
    let totals = format!(
        "{} links and {} lib_symbols copies in {} files; {} left alone",
        f.links.len(),
        f.copies.len(),
        files.len(),
        f.skipped.len()
    );
    if f.links.is_empty() && f.copies.is_empty() {
        out += &format!(
            "Nothing to write: every mx3: link references its symbol's MPN fields; {} left alone.\n",
            f.skipped.len()
        );
    } else if f.write {
        out += &format!("Wrote {totals}.\n");
    } else {
        out += &format!("Dry run: {totals}; --write edits the files.\n");
    }
    out
}
