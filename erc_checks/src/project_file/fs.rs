//! The project file on disk: found beside the design, read with a size limit, a waiver appended.

use super::waiver::{Waiver, entry_text};
use super::{FILE_SIZE_BYTES_MAX, ProjectFile, ProjectFileError, candidate_names};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// The project file of a design whose main file is `main` in `dir`: its name, when one is there
pub fn find(dir: &Path, main: &str) -> Option<String> {
    candidate_names(main)
        .into_iter()
        .find(|n| dir.join(n).is_file())
}

/// Reads the project file `name` in `dir`
pub fn load(dir: &Path, name: &str) -> Result<ProjectFile, ProjectFileError> {
    let read_err = |error| ProjectFileError::Read {
        file: name.to_owned(),
        error,
    };
    let f = std::fs::File::open(dir.join(name)).map_err(read_err)?;
    let mut bytes = Vec::new();
    f.take(FILE_SIZE_BYTES_MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(read_err)?;
    if bytes.len() as u64 > FILE_SIZE_BYTES_MAX {
        return Err(ProjectFileError::TooLarge {
            file: name.to_owned(),
            size_bytes: bytes.len() as u64,
        });
    }
    let text = String::from_utf8(bytes)
        .map_err(|e| read_err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    ProjectFile::parse(name, &text)
}

/// Appends `waiver` to the project file of the design `main` in `dir`, creating `<name>.berc.toml` when there is
/// none; the path written. The existing text is kept as it is.
pub fn append_waiver(dir: &Path, main: &str, waiver: &Waiver) -> std::io::Result<PathBuf> {
    let name = find(dir, main).unwrap_or_else(|| {
        let [own, _] = candidate_names(main);
        own
    });
    let path = dir.join(name);
    let mut text = entry_text(waiver);
    let existing = std::fs::read(&path).unwrap_or_default();
    if existing.is_empty() {
        // A new file: no blank line before its first entry
        text.remove(0);
    } else if existing.last() != Some(&b'\n') {
        text.insert(0, '\n');
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    f.write_all(text.as_bytes())?;
    Ok(path)
}
