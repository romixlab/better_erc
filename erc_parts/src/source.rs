//! A part source: where a refresh gets part facts and footprint reviews, and where `berc fp accept` records a
//! review. [`FileSource`] keeps them in a file in the lock's own shape (hand-filled data, another board's lock);
//! the lab's part store is another source, behind its own cargo feature.

use crate::lock::{
    Cover, Coverage, LockError, LockedPair, LockedPart, LockedReview, PartMatch, PartsLock,
    Placement,
};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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

    /// Records a footprint review; the review as stored, with its id
    fn accept(&self, review: &NewReview) -> Result<LockedReview, SourceError>;
}

/// A footprint review to record: the footprint with review hash `hash` fits the land pattern and package of what
/// `covers` names, checked against a document
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NewReview {
    /// The review hash (`fp1:` and 16 hex digits)
    pub hash: String,
    /// `Lib:Name` of the footprint reviewed, for display
    pub footprint: String,
    /// Document number and revision (`DS30154`, `rev16`)
    pub doc: Option<String>,
    pub doc_rev: Option<String>,
    /// The PDF's blake3, hex
    pub pdf_blake3: Option<String>,
    /// The page of the land pattern, 1-based
    pub page: Option<u32>,
    /// mm per PDF point the page was drawn at
    pub scale: Option<f64>,
    /// Where the footprint lay on the page
    pub at: Option<Placement>,
    pub reviewer: String,
    /// `2026-11-25`
    pub reviewed_on: String,
    pub note: String,
    /// At least one
    pub covers: Vec<Cover>,
}

/// The largest number of covers one review takes
pub const COVERS_PER_REVIEW_MAX: usize = 200;

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

/// Part data from a file in the lock's shape: its `[[part]]` entries by MPN and its `[[review]]` entries. A review
/// covers a part as the lab's store judges it ([`LockedReview::covers`]): by MPN, or by the part's decoding for a
/// family. An MPN the file lacks is not found. [`Source::accept`] adds a review and, for a source opened from a
/// file, writes the file again.
#[derive(Debug)]
pub struct FileSource {
    name: String,
    data: Mutex<PartsLock>,
    path: Option<PathBuf>,
}

/// Why a [`FileSource`] refused a review
#[derive(Debug)]
pub enum AcceptRefused {
    /// Not a review hash of this version
    BadHash(String),
    /// No cover, or more than [`COVERS_PER_REVIEW_MAX`]
    Covers(usize),
    /// A family without a segment, which would cover every part of the scheme
    EmptyFamily(String),
    /// An MPN cover for an MPN the source has no single part of
    NoPart(String),
    /// No reviewer, or no date
    Who,
}

impl fmt::Display for AcceptRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcceptRefused::BadHash(h) => write!(
                f,
                "`{h}` is no review hash (`{}:` and 16 hex digits)",
                erc_model::fphash::VERSION
            ),
            AcceptRefused::Covers(n) => write!(
                f,
                "a review covers 1 to {COVERS_PER_REVIEW_MAX} MPNs or families, not {n}"
            ),
            AcceptRefused::EmptyFamily(s) => {
                write!(
                    f,
                    "family {s} names no segment: it would cover every part of the scheme"
                )
            }
            AcceptRefused::NoPart(m) => write!(f, "no single part has MPN {m}"),
            AcceptRefused::Who => write!(f, "a review needs a reviewer and a date"),
        }
    }
}

impl std::error::Error for AcceptRefused {}

impl FileSource {
    /// Reads `path`; an accepted review is written back to it
    pub fn open(path: &Path) -> Result<FileSource, LockError> {
        let data = PartsLock::read(path)?;
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let mut s = FileSource::new(format!("file {file}"), data);
        s.path = Some(path.to_path_buf());
        Ok(s)
    }

    /// A source answering from `data`, named `name` in the lock, kept in memory
    pub fn new(name: String, data: PartsLock) -> FileSource {
        FileSource {
            name,
            data: Mutex::new(data),
            path: None,
        }
    }

    /// The data as it is now, accepted reviews included
    pub fn data(&self) -> PartsLock {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PartsLock> {
        self.data.lock().expect("no panic while the data is held")
    }
}

/// The pair of `hash` and `mpn` as `data` covers it
fn pair_of(data: &PartsLock, hash: &str, mpn: &str) -> LockedPair {
    let mut pair = LockedPair {
        hash: hash.to_owned(),
        mpn: mpn.to_owned(),
        footprint: String::new(),
        maker: None,
        status: Coverage::UnknownMpn,
        covered_by: Vec::new(),
        other_hashes: Vec::new(),
    };
    let Some(part) = data.part(mpn).filter(|p| p.matched == PartMatch::Found) else {
        return pair;
    };
    for r in data.reviews.iter().filter(|r| r.covers(part)) {
        if r.hash == hash {
            pair.covered_by.push(r.id);
        } else {
            pair.other_hashes.push(r.id);
        }
    }
    pair.status = if pair.covered_by.is_empty() {
        Coverage::Uncovered
    } else {
        Coverage::Covered
    };
    pair
}

impl Source for FileSource {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn parts(&self, mpns: &[String], keys: &[String]) -> Result<PartsAnswer, SourceError> {
        let data = self.lock();
        let parts = mpns
            .iter()
            .map(|mpn| match data.part(mpn) {
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
        let data = self.lock();
        let pairs: Vec<LockedPair> = mpns.iter().map(|m| pair_of(&data, hash, m)).collect();
        let reviews = data
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

    fn accept(&self, review: &NewReview) -> Result<LockedReview, SourceError> {
        let refused = |e: AcceptRefused| SourceError {
            what: format!("review of {}", review.hash),
            cause: Box::new(e),
        };
        if !erc_model::fphash::is_hash(&review.hash) {
            return Err(refused(AcceptRefused::BadHash(review.hash.clone())));
        }
        if review.covers.is_empty() || review.covers.len() > COVERS_PER_REVIEW_MAX {
            return Err(refused(AcceptRefused::Covers(review.covers.len())));
        }
        if review.reviewer.trim().is_empty() || review.reviewed_on.trim().is_empty() {
            return Err(refused(AcceptRefused::Who));
        }
        let mut data = self.lock();
        for c in &review.covers {
            match c {
                Cover::Mpn { mpn } => {
                    if !data
                        .part(mpn)
                        .is_some_and(|p| p.matched == PartMatch::Found)
                    {
                        return Err(refused(AcceptRefused::NoPart(mpn.clone())));
                    }
                }
                Cover::Family { segments, .. } if segments.is_empty() => {
                    return Err(refused(AcceptRefused::EmptyFamily(c.describe())));
                }
                Cover::Family { .. } => {}
            }
        }
        let id = data.reviews.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        let stored = LockedReview {
            id,
            hash: review.hash.clone(),
            footprint: review.footprint.clone(),
            doc: review.doc.clone(),
            doc_rev: review.doc_rev.clone(),
            pdf_blake3: review.pdf_blake3.clone(),
            page: review.page,
            scale: review.scale,
            at: review.at,
            reviewer: review.reviewer.clone(),
            reviewed_on: review.reviewed_on.clone(),
            note: review.note.clone(),
            covers: review.covers.clone(),
            retracted: None,
        };
        let mut next = data.clone();
        next.reviews.push(stored.clone());
        next.sort();
        if let Some(path) = &self.path {
            crate::lock::write_atomic(path, &next.to_toml()).map_err(|e| SourceError {
                what: format!("writing {}", path.display()),
                cause: Box::new(e),
            })?;
        }
        *data = next;
        Ok(stored)
    }
}
