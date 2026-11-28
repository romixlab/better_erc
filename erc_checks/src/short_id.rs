//! Short finding ids: a few letters that name one finding of a run, so a person can point at it (`#k3f`) when
//! relaying it to someone else (RPT-27).
//!
//! A finding's full id is the SHA-256 of its board, fingerprint and power state, its first 65 bits written as 13
//! characters of lower-case Crockford base32 (`0-9`, `a-z` without `i`, `l`, `o`, `u`). It stays the same from run
//! to run while the finding does: same check, same subjects, same board. The short id is the shortest prefix of the
//! full id that no other finding of the run shares, and at least [`SHORT_ID_LEN_MIN`] characters, as git shortens a
//! commit hash. Two findings with the same board, fingerprint and state get the same id.

use sha2::{Digest, Sha256};

use crate::finding::Finding;

/// The fewest characters a short id has
pub const SHORT_ID_LEN_MIN: usize = 3;

/// Characters of a full id
pub const FULL_ID_LEN: usize = 13;

/// Crockford's base32 digits, lower case
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// The full id of `f`: [`FULL_ID_LEN`] base32 characters of the hash of its board, fingerprint and state
pub fn full_id(f: &Finding) -> String {
    let mut hasher = Sha256::new();
    hasher.update(f.board.as_bytes());
    hasher.update(b"\n");
    hasher.update(f.fingerprint.as_bytes());
    hasher.update(b"\n");
    hasher.update(f.state.as_deref().unwrap_or("").as_bytes());
    let digest = hasher.finalize();
    let mut bits = [0u8; 9];
    bits.copy_from_slice(&digest[..9]);
    let value = u128::from_be_bytes({
        let mut b = [0u8; 16];
        b[..9].copy_from_slice(&bits);
        b
    });
    (0..FULL_ID_LEN)
        .map(|i| {
            let shift = 128 - 5 * (i + 1);
            let digit = usize::try_from((value >> shift) & 0x1f).expect("five bits fit a usize");
            char::from(ALPHABET[digit])
        })
        .collect()
}

/// Gives each finding its short id, unique among `findings`
pub fn assign<'f>(findings: impl IntoIterator<Item = &'f mut Finding>) {
    let mut all: Vec<(String, &mut Finding)> =
        findings.into_iter().map(|f| (full_id(f), f)).collect();
    let mut ids: Vec<&str> = all.iter().map(|(id, _)| id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    let lens: Vec<usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let before = i.checked_sub(1).map_or(0, |p| common_prefix(ids[p], id));
            let after = ids.get(i + 1).map_or(0, |n| common_prefix(n, id));
            (before.max(after) + 1).clamp(SHORT_ID_LEN_MIN, FULL_ID_LEN)
        })
        .collect();
    let len_of: std::collections::BTreeMap<String, usize> = ids
        .iter()
        .zip(lens)
        .map(|(id, len)| ((*id).to_owned(), len))
        .collect();
    for (id, f) in &mut all {
        let len = len_of[id.as_str()];
        f.short_id = id[..len].to_owned();
    }
}

/// Whether `query` (`#k3f` or `k3f`, any case) names `f`: a prefix of its full id
pub fn matches(f: &Finding, query: &str) -> bool {
    let q = query.trim_start_matches('#').to_ascii_lowercase();
    !q.is_empty() && full_id(f).starts_with(&q)
}

/// Whether a selector is a finding id (`#k3f`) rather than a check id or category
pub fn is_query(selector: &str) -> bool {
    selector.starts_with('#')
}

fn common_prefix(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(board: &str, fingerprint: &str) -> Finding {
        serde_json::from_value(serde_json::json!({
            "check": "X-1", "severity": "warning", "confidence": "certain", "board": board, "subjects": [],
            "message": "", "facts": [], "fingerprint": fingerprint, "explain": "", "locations": [], "also": [],
        }))
        .expect("a minimal finding")
    }

    #[test]
    fn full_id_is_crockford_and_stable() {
        let f = finding("b", "BOM-14:part R1");
        let id = full_id(&f);
        assert_eq!(id.len(), FULL_ID_LEN);
        assert!(id.bytes().all(|c| ALPHABET.contains(&c)), "{id}");
        assert_eq!(id, full_id(&finding("b", "BOM-14:part R1")));
        assert_ne!(id, full_id(&finding("other", "BOM-14:part R1")));
    }

    #[test]
    fn short_ids_are_unique_and_at_least_three() {
        let mut fs: Vec<Finding> = (0..2000)
            .map(|i| finding("b", &format!("X-1:part R{i}")))
            .collect();
        assign(fs.iter_mut());
        let mut ids: Vec<&str> = fs.iter().map(|f| f.short_id.as_str()).collect();
        assert!(ids.iter().all(|id| id.len() >= SHORT_ID_LEN_MIN));
        for f in &fs {
            assert!(full_id(f).starts_with(&f.short_id));
        }
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "every short id names one finding");
    }

    #[test]
    fn a_shared_prefix_lengthens_both() {
        // Find two fingerprints whose full ids share their first 3 characters
        let mut seen = std::collections::BTreeMap::new();
        let (a, b) = (0..)
            .find_map(|i| {
                let fp = format!("X-1:part R{i}");
                let id = full_id(&finding("b", &fp));
                seen.insert(id[..3].to_owned(), fp.clone())
                    .map(|other| (other, fp))
            })
            .expect("a 3-character collision turns up");
        let mut fs = [finding("b", &a), finding("b", &b)];
        assign(fs.iter_mut());
        assert!(
            fs[0].short_id.len() > SHORT_ID_LEN_MIN,
            "{}",
            fs[0].short_id
        );
        assert_ne!(fs[0].short_id, fs[1].short_id);
        assert_eq!(fs[0].short_id.len(), fs[1].short_id.len());
    }

    #[test]
    fn alone_it_is_three_characters() {
        let mut fs = [finding("b", "BOM-14:part R1")];
        assign(fs.iter_mut());
        assert_eq!(fs[0].short_id.len(), SHORT_ID_LEN_MIN);
    }

    #[test]
    fn matches_a_prefix_with_or_without_hash() {
        let f = finding("b", "BOM-14:part R1");
        let id = full_id(&f);
        assert!(matches(&f, &format!("#{}", &id[..3])));
        assert!(matches(&f, &id[..5].to_ascii_uppercase()));
        assert!(!matches(&f, "#"));
        assert!(!matches(&f, &format!("#{}z", id)));
    }
}
