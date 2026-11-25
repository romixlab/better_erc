//! Footprint review, the library side of `berc fp`: the review hash ([`erc_model::fphash`]) of a library footprint
//! file or of a footprint as placed on a board ([`hash_file`]), read through one [`FootprintSource`]; each (board
//! footprint hash, MPN) pair judged against the footprint reviews of the parts snapshot ([`check`], [`todo`]), and
//! a review recorded in a part source ([`accept`]).

use erc_kicad::footprint;
use erc_model::layout::{FootprintId, Layout};
use erc_model::{ImportError, LocalFootprint, fphash};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

mod accept;
mod check;
pub mod group;
mod library;
pub mod model;
pub mod outline;
pub mod pairs;
mod todo;
pub use accept::{
    AcceptError, AcceptRun, AcceptWhat, Covers, SymbolEditError, Written, accept, date_of,
    set_symbol_footprint, sheets_and_symbols, today,
};
pub use check::{FpCheck, FpCheckError, FpEntry, FpStatus, check};
pub use group::{FactTable, FootprintTodo};
pub use library::{
    CheckedLibrary, LibCheck, LibCheckError, LibCheckOptions, LibrarySearch, SymbolEntry,
    SymbolStatus, check_libraries,
};
pub use todo::{FpTodo, TodoEntry, TodoStatus, todo};

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
    /// Neither a footprint, a footprint library nor a project with a board (`berc fp model`)
    NoBoard(PathBuf),
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
            FpError::NoBoard(p) => write!(
                f,
                "{}: not a footprint (.kicad_mod), a footprint library (.pretty) or a project with a board (.kicad_pcb)",
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
    /// The footprint's fields (`Datasheet`, the MPNs a board copy took over from its symbol, ...), those with a
    /// value
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
        check::field_of(&self.fields, name)
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

pub use erc_kicad::libtable::{FootprintLibraries, SymbolLibraries, kicad_config_dir};

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

    /// Review hashes are written into symbols: a footprint that did not change keeps its `fp1:` hash byte for byte,
    /// also now that a package's vias are drawn as vias (they are pads, in the hash as they always were)
    #[test]
    fn written_hashes_stay() {
        for (name, want) in [
            ("Asym_SOT.kicad_mod", "fp1:34265c0d8697c34e"),
            ("Mixed_Pads.kicad_mod", "fp1:f460b103cd5189b2"),
        ] {
            let got = hash_file(&fixtures().join("Test.pretty").join(name), None).unwrap();
            assert_eq!(got.hash, want, "{name}:\n{}", got.canonical.join("\n"));
        }
    }
}
