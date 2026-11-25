//! A part source: where a refresh gets part facts and footprint reviews. [`FileSource`] reads them from a file in
//! the lock's own shape (hand-filled data, another board's lock); the lab's part store is another source, behind
//! its own cargo feature.

use crate::lock::{LockError, LockedPair, LockedPart, LockedReview, PartsLock};
use std::fmt;
use std::path::Path;

/// The parts a source gives for some MPNs
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartsAnswer {
    /// Asked keys the source has no definition for at all
    pub unknown_keys: Vec<String>,
    /// One entry per MPN asked
    pub parts: Vec<LockedPart>,
}

/// The footprint reviews a source gives for one board footprint hash and some MPNs
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReviewsAnswer {
    /// At least one per MPN asked (several when the source has several parts of it)
    pub pairs: Vec<LockedPair>,
    /// Every review a pair names
    pub reviews: Vec<LockedReview>,
}

/// Where part facts and footprint reviews come from
pub trait Source {
    /// The name written into the lock (`mx3`, `file parts.toml`)
    fn name(&self) -> String;

    /// The facts of `keys` for each of `mpns` (at most [`MPNS_PER_CALL_MAX`]), with each MPN's reading by its
    /// scheme
    fn parts(&self, mpns: &[String], keys: &[String]) -> Result<PartsAnswer, SourceError>;

    /// The reviews of the footprint with review hash `hash` covering each of `mpns` (at most
    /// [`MPNS_PER_CALL_MAX`])
    fn reviews(&self, hash: &str, mpns: &[String]) -> Result<ReviewsAnswer, SourceError>;
}

/// The most MPNs one call to a [`Source`] gets; a refresh asks in parts
pub const MPNS_PER_CALL_MAX: usize = 500;

/// Why a source gave no answer
#[derive(Debug)]
pub struct SourceError {
    /// What was asked (`parts of 120 MPNs`)
    pub what: String,
    pub cause: Box<dyn std::error::Error + Send + Sync>,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.what, self.cause)
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

/// Part data from a file in the lock's shape: its `[[part]]` entries by MPN, its `[[footprint]]` pairs and
/// `[[review]]` entries by hash and MPN. An MPN the file lacks is not found; a pair it lacks is not covered.
#[derive(Clone, Debug)]
pub struct FileSource {
    name: String,
    data: PartsLock,
}

impl FileSource {
    /// Reads `path`
    pub fn open(path: &Path) -> Result<FileSource, LockError> {
        let data = PartsLock::read(path)?;
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        Ok(FileSource::new(format!("file {file}"), data))
    }

    /// A source answering from `data`, named `name` in the lock
    pub fn new(name: String, data: PartsLock) -> FileSource {
        FileSource { name, data }
    }
}

impl Source for FileSource {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn parts(&self, mpns: &[String], keys: &[String]) -> Result<PartsAnswer, SourceError> {
        let parts = mpns
            .iter()
            .map(|mpn| match self.data.part(mpn) {
                Some(p) => {
                    let mut p = p.clone();
                    p.facts.retain(|f| keys.contains(&f.key));
                    p.missing = keys
                        .iter()
                        .filter(|k| p.fact(k).is_none())
                        .cloned()
                        .collect();
                    p
                }
                None => LockedPart {
                    mpn: mpn.clone(),
                    missing: keys.to_vec(),
                    ..LockedPart::default()
                },
            })
            .collect();
        Ok(PartsAnswer {
            unknown_keys: Vec::new(),
            parts,
        })
    }

    fn reviews(&self, hash: &str, mpns: &[String]) -> Result<ReviewsAnswer, SourceError> {
        let mut pairs = Vec::new();
        for mpn in mpns {
            let known: Vec<LockedPair> = self.data.pairs(hash, mpn).cloned().collect();
            if known.is_empty() {
                let status = if self.data.part(mpn).is_some() {
                    crate::lock::Coverage::Uncovered
                } else {
                    crate::lock::Coverage::UnknownMpn
                };
                pairs.push(LockedPair {
                    hash: hash.to_owned(),
                    mpn: mpn.clone(),
                    footprint: String::new(),
                    maker: None,
                    status,
                    covered_by: Vec::new(),
                    other_hashes: Vec::new(),
                });
            }
            pairs.extend(known);
        }
        let reviews = self
            .data
            .reviews
            .iter()
            .filter(|r| {
                pairs
                    .iter()
                    .any(|p| p.covered_by.contains(&r.id) || p.other_hashes.contains(&r.id))
            })
            .cloned()
            .collect();
        Ok(ReviewsAnswer { pairs, reviews })
    }
}
