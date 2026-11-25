//! One (footprint hash, MPN) pair judged against the footprint reviews of a parts snapshot: covered by a review of
//! this hash, or not, and why not. `berc fp check`, `todo` and `check --lib` all judge through [`judge`].

use erc_model::fphash;
use erc_parts::PartsLock;
use erc_parts::lock::{Cover, Coverage};
use serde::{Deserialize, Serialize};

/// What the reviews say of one pair. Ordered by urgency: [`PairStatus::Changed`] first, [`PairStatus::Covered`]
/// last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PairStatus {
    /// A review of another hash of this version covers the part: the footprint changed since it was reviewed
    Changed,
    /// Only reviews made with another version of the review hash cover the part: this build cannot compare them
    OtherVersion,
    /// The snapshot has no entry for the pair: `berc parts refresh` has not run since the MPN or the footprint
    /// came
    NotInLock,
    /// The part is known, no review covers it
    Uncovered,
    /// The part source has no part of this MPN: none can be reviewed until it has
    UnknownMpn,
    /// A review of this hash covers the part
    Covered,
}

impl PairStatus {
    pub fn name(self) -> &'static str {
        match self {
            PairStatus::Changed => "changed",
            PairStatus::OtherVersion => "other version",
            PairStatus::NotInLock => "not in lock",
            PairStatus::Uncovered => "uncovered",
            PairStatus::UnknownMpn => "unknown MPN",
            PairStatus::Covered => "covered",
        }
    }
}

/// A review of another hash that covers the part
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherReview {
    pub id: i64,
    /// The hash it was made for
    pub hash: String,
}

/// One MPN of a footprint, judged
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    pub mpn: String,
    pub status: PairStatus,
    /// Ids of the reviews of this hash that cover the part
    pub covered_by: Vec<i64>,
    /// Reviews of other hashes that cover the part
    pub other: Vec<OtherReview>,
    /// The part's family by its MPN's decoding ([`Cover::family_of`]): what one review can cover for every part
    /// of it. `None`: the MPN is reviewed alone
    pub family: Option<Cover>,
}

impl Pair {
    /// What a review of this pair covers: its family, else the MPN alone
    pub fn cover(&self) -> Cover {
        self.family.clone().unwrap_or_else(|| Cover::Mpn {
            mpn: self.mpn.clone(),
        })
    }
}

/// The pair of `hash` and `mpn` as `lock` covers it. Several source parts of one MPN (several makers): the
/// worst of them.
pub fn judge(lock: &PartsLock, hash: &str, mpn: &str) -> Pair {
    let family = lock.part(mpn).and_then(Cover::family_of);
    let mut pair = Pair {
        mpn: mpn.to_owned(),
        status: PairStatus::NotInLock,
        covered_by: Vec::new(),
        other: Vec::new(),
        family,
    };
    let mut worst: Option<PairStatus> = None;
    for p in lock.pairs(hash, mpn) {
        let mut other = Vec::new();
        for id in &p.other_hashes {
            if let Some(r) = lock.review(*id) {
                other.push(OtherReview {
                    id: *id,
                    hash: r.hash.clone(),
                });
            }
        }
        let status = match p.status {
            Coverage::Covered => PairStatus::Covered,
            Coverage::UnknownMpn => PairStatus::UnknownMpn,
            Coverage::Uncovered if other.iter().any(|o| same_version(&o.hash)) => {
                PairStatus::Changed
            }
            Coverage::Uncovered if !other.is_empty() => PairStatus::OtherVersion,
            Coverage::Uncovered => PairStatus::Uncovered,
        };
        worst = Some(worst.map_or(status, |w| w.min(status)));
        pair.covered_by.extend(&p.covered_by);
        for o in other {
            if !pair.other.contains(&o) {
                pair.other.push(o);
            }
        }
    }
    pair.covered_by.sort_unstable();
    pair.covered_by.dedup();
    if let Some(w) = worst {
        pair.status = w;
    }
    pair
}

/// The hash is of this build's version of the review hash
fn same_version(hash: &str) -> bool {
    hash.split_once(':')
        .is_some_and(|(v, _)| v == fphash::VERSION)
}

/// The footprint's verdict from its pairs: every one covered, or the most urgent reason one is not, `Partial` when
/// some are covered and the rest only uncovered or unknown. `None` for no pairs.
pub fn overall(pairs: &[Pair]) -> Option<Overall> {
    let worst = pairs.iter().map(|p| p.status).min()?;
    Some(match worst {
        PairStatus::Covered => Overall::Ok,
        PairStatus::Changed => Overall::Changed,
        PairStatus::OtherVersion => Overall::OtherVersion,
        PairStatus::NotInLock => Overall::NotInLock,
        PairStatus::Uncovered | PairStatus::UnknownMpn
            if pairs.iter().any(|p| p.status == PairStatus::Covered) =>
        {
            Overall::Partial
        }
        PairStatus::Uncovered | PairStatus::UnknownMpn => Overall::Uncovered,
    })
}

/// A footprint's verdict over its pairs ([`overall`])
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overall {
    Ok,
    Changed,
    OtherVersion,
    NotInLock,
    Partial,
    Uncovered,
}
