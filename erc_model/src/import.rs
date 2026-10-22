//! One trait for every importer: KiCad (schematic, board, project), the KiCad netlist, Altium, OrCAD. An importer
//! reads a [`Source`] (a set of files behind [`Files`], so the same code runs on a directory, in memory and in a
//! wasm GUI) and returns a [`Design`] with [`ImportNote`]s for everything it did not understand: nothing is
//! silently dropped and nothing panics.

use crate::design::Design;
use std::collections::BTreeMap;
use std::fmt;

/// The files an importer may read, by path relative to the project directory (`/` separated)
pub trait Files {
    /// The contents of a file, `None` when it does not exist
    fn read(&self, path: &str) -> Option<Vec<u8>>;

    /// All file paths, sorted
    fn list(&self) -> Vec<String>;
}

/// Files held in memory (tests, a GUI that got them dropped in, wasm)
#[derive(Clone, Debug, Default)]
pub struct MemFiles(pub BTreeMap<String, Vec<u8>>);

impl MemFiles {
    pub fn with(mut self, path: impl Into<String>, contents: impl Into<Vec<u8>>) -> MemFiles {
        self.0.insert(path.into(), contents.into());
        self
    }
}

impl Files for MemFiles {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0.get(path).cloned()
    }

    fn list(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }
}

/// The files of a directory on disk; [`Files::list`] lists its top level
#[cfg(feature = "fs")]
#[derive(Clone, Debug)]
pub struct DirFiles(pub std::path::PathBuf);

#[cfg(feature = "fs")]
impl Files for DirFiles {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(path)).ok()
    }

    fn list(&self) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(&self.0)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .collect();
        out.sort();
        out
    }
}

/// What to import: the main file (a `.kicad_pro`, `.kicad_sch`, `.net`) among the files
pub struct Source<'a> {
    pub main: String,
    pub files: &'a dyn Files,
}

impl<'a> Source<'a> {
    pub fn new(main: impl Into<String>, files: &'a dyn Files) -> Source<'a> {
        Source {
            main: main.into(),
            files,
        }
    }

    /// A file as text (lossy UTF-8 is an error: KiCad files are UTF-8)
    pub fn text(&self, path: &str) -> Result<String, ImportError> {
        let bytes = self
            .files
            .read(path)
            .ok_or_else(|| ImportError::Missing(path.to_string()))?;
        String::from_utf8(bytes).map_err(|_| ImportError::Invalid {
            file: path.to_string(),
            message: "not UTF-8".into(),
        })
    }
}

/// How sure an importer is that it can read a source
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    No,
    /// By the file extension only
    Maybe,
    /// The contents say so
    Yes,
}

/// Options for an import
#[derive(Clone, Debug, Default)]
pub struct ImportOptions {
    /// Skip the board file even when there is one
    pub no_layout: bool,
    /// Read the design as this KiCad major version would (8, 9, 10): connectivity rules changed between
    /// versions. `None`: the version that wrote the newest file of the project.
    pub kicad_version: Option<u32>,
}

/// What an import produced
#[derive(Clone, Debug, Default)]
pub struct Imported {
    pub design: Design,
    pub notes: Vec<ImportNote>,
}

/// Something the importer read past: an unknown s-expression, a pin without a type, an override of a field that
/// does not exist
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportNote {
    pub file: String,
    /// Where in the file, as the importer can say it (`symbol R1`, `line 12`)
    pub at: String,
    pub kind: NoteKind,
    pub message: String,
}

/// The kind of an import note
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteKind {
    /// An element this importer does not know (a newer file format)
    Unknown,
    /// Something expected that is not there
    Missing,
    /// Present but not readable
    Invalid,
    /// Known, read, and not kept in the model
    Ignored,
    /// Two links or sources of the same fact disagree (a footprint's schematic path and its designator)
    Conflict,
}

impl fmt::Display for ImportNote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            NoteKind::Unknown => "unknown",
            NoteKind::Missing => "missing",
            NoteKind::Invalid => "invalid",
            NoteKind::Ignored => "ignored",
            NoteKind::Conflict => "conflict",
        };
        write!(f, "{}: {}: {kind}: {}", self.file, self.at, self.message)
    }
}

/// Why an import failed as a whole
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    /// A file the source names is not there
    Missing(String),
    /// A file is there but cannot be read as this format
    Invalid { file: String, message: String },
    /// No importer reads this source
    Unsupported(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Missing(p) => write!(f, "{p}: no such file"),
            ImportError::Invalid { file, message } => write!(f, "{file}: {message}"),
            ImportError::Unsupported(p) => write!(f, "{p}: no importer reads this"),
        }
    }
}

impl std::error::Error for ImportError {}

/// An importer of one tool's files
pub trait Importer {
    /// A short name (`kicad`, `kicad-netlist`)
    fn name(&self) -> &'static str;

    /// Whether this importer reads `src`
    fn sniff(&self, src: &Source) -> Confidence;

    /// Reads `src` into a design
    fn import(&self, src: &Source, opts: &ImportOptions) -> Result<Imported, ImportError>;
}
