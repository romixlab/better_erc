//! `berc parts refresh`: a new parts snapshot of a design from a [`Source`]. It holds the board's MPNs and
//! footprint pairs only, so an MPN taken off the board leaves the lock too.

use crate::board::BoardMpns;
use crate::keys;
use crate::lock::{LockedPart, PartsLock, VERSION};
use crate::source::{MPNS_PER_CALL_MAX, Source, SourceError};
use erc_model::Design;
use std::collections::BTreeSet;
use std::fmt;

/// Why a refresh wrote nothing
#[derive(Debug)]
pub enum RefreshError {
    /// The source gave no answer
    Source(SourceError),
    /// The source answered for an MPN not asked, or not for one asked
    Answer { what: String, mpn: String },
}

impl fmt::Display for RefreshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefreshError::Source(e) => write!(f, "{e}"),
            RefreshError::Answer { what, mpn } => write!(f, "the part source {what} {mpn}"),
        }
    }
}

impl std::error::Error for RefreshError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RefreshError::Source(e) => Some(e),
            RefreshError::Answer { .. } => None,
        }
    }
}

impl From<SourceError> for RefreshError {
    fn from(e: SourceError) -> Self {
        RefreshError::Source(e)
    }
}

/// The snapshot of `design`'s MPNs and footprint pairs from `source`, asking for [`keys::FACT_KEYS`]
pub fn refresh(design: &Design, source: &dyn Source) -> Result<PartsLock, RefreshError> {
    refresh_mpns(&BoardMpns::of(design), source)
}

/// The snapshot of `board`'s MPNs and footprint pairs from `source`
pub fn refresh_mpns(board: &BoardMpns, source: &dyn Source) -> Result<PartsLock, RefreshError> {
    let keys = keys::names();
    let mpns: Vec<String> = board.mpns.keys().cloned().collect();
    let mut lock = PartsLock {
        version: VERSION,
        source: source.name(),
        keys: keys.clone(),
        ..PartsLock::default()
    };
    for chunk in mpns.chunks(MPNS_PER_CALL_MAX) {
        let answer = source.parts(chunk, &keys)?;
        let parts = matched(chunk, answer.parts, "gave parts for")?;
        lock.unknown_keys.extend(answer.unknown_keys);
        lock.parts.extend(parts);
    }
    for (hash, fp) in &board.footprints {
        let mpns: Vec<String> = fp.mpns.iter().cloned().collect();
        for chunk in mpns.chunks(MPNS_PER_CALL_MAX) {
            let answer = source.reviews(hash, chunk)?;
            let asked: BTreeSet<&String> = chunk.iter().collect();
            let mut seen = BTreeSet::new();
            for mut pair in answer.pairs {
                if pair.hash != *hash || !asked.contains(&pair.mpn) {
                    return Err(RefreshError::Answer {
                        what: format!("gave a review pair for {} with", pair.hash),
                        mpn: pair.mpn,
                    });
                }
                seen.insert(pair.mpn.clone());
                pair.footprint.clone_from(&fp.lib_id);
                lock.footprints.push(pair);
            }
            if let Some(m) = chunk.iter().find(|m| !seen.contains(*m)) {
                return Err(RefreshError::Answer {
                    what: format!("gave no review pair for {hash} with"),
                    mpn: m.clone(),
                });
            }
            lock.reviews.extend(answer.reviews);
        }
    }
    lock.sort();
    Ok(lock)
}

/// The answer's parts for exactly the MPNs asked, once each
fn matched(
    asked: &[String],
    parts: Vec<LockedPart>,
    what: &str,
) -> Result<Vec<LockedPart>, RefreshError> {
    let want: BTreeSet<&String> = asked.iter().collect();
    let mut got = BTreeSet::new();
    for p in &parts {
        if !want.contains(&p.mpn) || !got.insert(p.mpn.clone()) {
            return Err(RefreshError::Answer {
                what: format!("{what} an MPN not asked or twice:"),
                mpn: p.mpn.clone(),
            });
        }
    }
    if let Some(m) = asked.iter().find(|m| !got.contains(*m)) {
        return Err(RefreshError::Answer {
            what: "gave no part entry for".to_owned(),
            mpn: m.clone(),
        });
    }
    Ok(parts)
}
