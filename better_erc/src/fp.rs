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
