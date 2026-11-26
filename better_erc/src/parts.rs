//! `berc parts refresh` (CFG-9): a design's parts snapshot written again from a part source, beside the project
//! or where its project file's `[parts] lock` says. The rest of [`erc_parts`] is re-exported here.

pub use erc_parts::*;

use erc_checks::{CheckError, Project};
use erc_parts::lock::{Coverage, PartMatch};
use serde::Serialize;
use std::fmt;
use std::path::{Path, PathBuf};

/// What a refresh did
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Refreshed {
    /// The lock file
    pub path: PathBuf,
    pub source: String,
    /// MPNs, and those the source has no single part for
    pub mpns: usize,
    pub not_found: usize,
    /// (footprint hash, MPN) pairs, and those a review of the hash covers
    pub pairs: usize,
    pub covered: usize,
    /// The new text differs from the file's
    pub changed: bool,
    /// The file was written
    pub written: bool,
    #[serde(skip)]
    pub lock: PartsLock,
}

impl Refreshed {
    /// One line: the file, the counts, written or not
    pub fn to_text(&self) -> String {
        let state = match (self.changed, self.written) {
            (false, _) => "unchanged",
            (true, true) => "written",
            (true, false) => "out of date (not written)",
        };
        format!(
            "{}: {} MPNs ({} not found), {} footprint pairs ({} covered) from {}: {state}\n",
            self.path.display(),
            self.mpns,
            self.not_found,
            self.pairs,
            self.covered,
            self.source
        )
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("plain data serialises")
    }
}

/// Why a refresh wrote nothing
#[derive(Debug)]
pub enum PartsRefreshError {
    /// The design cannot be opened
    Open(CheckError),
    /// A footprint or symbol library given by path that is none
    Libraries(crate::fp::LibCheckError),
    /// The source gave no answer, or a wrong one
    Refresh(RefreshError),
    /// The lock cannot be written
    Write {
        path: PathBuf,
        error: std::io::Error,
    },
}

impl PartsRefreshError {
    pub const EXIT_CODE: u8 = 2;
}

impl fmt::Display for PartsRefreshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartsRefreshError::Open(e) => e.fmt(f),
            PartsRefreshError::Libraries(e) => e.fmt(f),
            PartsRefreshError::Refresh(e) => e.fmt(f),
            PartsRefreshError::Write { path, error } => {
                write!(f, "can't write {}: {error}", path.display())
            }
        }
    }
}

impl std::error::Error for PartsRefreshError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PartsRefreshError::Open(e) => Some(e),
            PartsRefreshError::Libraries(e) => Some(e),
            PartsRefreshError::Refresh(e) => Some(e),
            PartsRefreshError::Write { error, .. } => Some(error),
        }
    }
}

/// Whether a refresh writes the lock
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteLock {
    /// Write it when it changed
    Changed,
    /// Only compare: [`Refreshed::changed`] says whether it is out of date
    Never,
}

/// The snapshot of the design at `project` from `source`, written to its lock when it changed and `write` says so;
/// each board footprint paired by the footprint its review is of, its library footprint found through `search`
/// ([`crate::fp::reviewed`])
pub fn refresh_project(
    project: &Path,
    search: &crate::fp::LibrarySearch,
    source: &dyn Source,
    write: WriteLock,
) -> Result<Refreshed, PartsRefreshError> {
    let p = Project::open_without_parts(project).map_err(PartsRefreshError::Open)?;
    let (mut syms, mut fps) = search.open(&p.dir).map_err(PartsRefreshError::Libraries)?;
    let mut resolver = crate::fp::Resolver {
        syms: &mut syms,
        fps: &mut fps,
    };
    let mut reviewed = |board: &erc_model::Board, i| {
        let r = resolver.resolve(board, i);
        let lib_id = r
            .library_footprint
            .filter(|_| r.reviewed_as == crate::fp::ReviewedAs::Library);
        let lay = board.layout.as_ref().expect("resolved from a layout");
        (
            r.hash,
            lib_id.unwrap_or_else(|| lay.footprints[i.index()].lib_id.clone()),
        )
    };
    let lock = erc_parts::refresh(&p.imported.design, &mut reviewed, source)
        .map_err(PartsRefreshError::Refresh)?;
    let path = p.parts_lock_path();
    let text = lock.to_toml();
    let changed = std::fs::read(&path).ok().as_deref() != Some(text.as_bytes());
    let written = changed && write == WriteLock::Changed;
    if written {
        erc_parts::lock::write_atomic(&path, &text).map_err(|error| PartsRefreshError::Write {
            path: path.clone(),
            error,
        })?;
    }
    Ok(Refreshed {
        path,
        source: lock.source.clone(),
        mpns: lock.parts.len(),
        not_found: lock
            .parts
            .iter()
            .filter(|p| p.matched != PartMatch::Found)
            .count(),
        pairs: lock.footprints.len(),
        covered: lock
            .footprints
            .iter()
            .filter(|p| p.status == Coverage::Covered)
            .count(),
        changed,
        written,
        lock,
    })
}

/// The datasheet URLs of the design at `project` by MPN ([`datasheets::of`], TOOL-18), with the design's name (its
/// main file without the extension)
pub fn project_datasheets(
    project: &Path,
) -> Result<(String, Vec<datasheets::MpnDatasheets>), CheckError> {
    let p = Project::open_without_parts(project)?;
    let name = Path::new(&p.main)
        .file_stem()
        .map_or_else(|| p.main.clone(), |s| s.to_string_lossy().into_owned());
    Ok((name, datasheets::of(&p.imported.design)))
}
