//! A refresh against a fake source

use super::{Part, design, schematic};
use erc_parts::board::{BoardMpns, HashedFootprint};
use erc_parts::lock::{Coverage, LockedPair, LockedPart, PartMatch};
use erc_parts::refresh::refresh_mpns;
use erc_parts::source::{MPNS_PER_CALL_MAX, PartsAnswer, ReviewsAnswer, SourceError};
use erc_parts::{FileSource, RefreshError, Source, refresh};
use std::cell::RefCell;

/// Knows every MPN, covers nothing; records the calls
#[derive(Default)]
struct Fake {
    calls: RefCell<Vec<String>>,
    /// Answer for this MPN too, not asked
    extra: Option<String>,
}

impl Source for Fake {
    fn name(&self) -> String {
        "fake".to_owned()
    }

    fn parts(&self, mpns: &[String], keys: &[String]) -> Result<PartsAnswer, SourceError> {
        self.calls
            .borrow_mut()
            .push(format!("parts {}", mpns.len()));
        let mut parts: Vec<LockedPart> = mpns
            .iter()
            .rev()
            .map(|m| LockedPart {
                mpn: m.clone(),
                matched: PartMatch::Found,
                missing: keys.to_vec(),
                ..LockedPart::default()
            })
            .collect();
        if let Some(m) = &self.extra {
            parts.push(LockedPart {
                mpn: m.clone(),
                ..LockedPart::default()
            });
        }
        Ok(PartsAnswer {
            unknown_keys: vec!["saturation_current".to_owned()],
            parts,
        })
    }

    fn reviews(&self, hash: &str, mpns: &[String]) -> Result<ReviewsAnswer, SourceError> {
        self.calls
            .borrow_mut()
            .push(format!("reviews {hash} {}", mpns.len()));
        Ok(ReviewsAnswer {
            pairs: mpns
                .iter()
                .map(|m| LockedPair {
                    hash: hash.to_owned(),
                    mpn: m.clone(),
                    footprint: String::new(),
                    maker: None,
                    status: Coverage::Uncovered,
                    covered_by: vec![],
                    other_hashes: vec![],
                })
                .collect(),
            reviews: vec![],
        })
    }

    fn accept(
        &self,
        _: &erc_parts::NewReview,
    ) -> Result<erc_parts::lock::LockedReview, SourceError> {
        unreachable!("a refresh records no review")
    }
}

fn board(n: usize) -> BoardMpns {
    let mut b = BoardMpns::default();
    for i in 0..n {
        b.mpns.insert(format!("MPN-{i:04}"), vec![format!("R{i}")]);
    }
    b.footprints.insert(
        "fp1:0123456789abcdef".to_owned(),
        HashedFootprint {
            lib_id: "Resistor_SMD:R_0402".to_owned(),
            mpns: ["MPN-0001".to_owned(), "MPN-0002".to_owned()].into(),
        },
    );
    b
}

#[test]
fn asks_in_parts_and_writes_the_same_lock_twice() {
    let fake = Fake::default();
    let n = MPNS_PER_CALL_MAX * 2 + 3;
    let lock = refresh_mpns(&board(n), &fake).unwrap();
    assert_eq!(
        *fake.calls.borrow(),
        [
            format!("parts {MPNS_PER_CALL_MAX}"),
            format!("parts {MPNS_PER_CALL_MAX}"),
            "parts 3".to_owned(),
            "reviews fp1:0123456789abcdef 2".to_owned(),
        ]
    );
    assert_eq!(lock.parts.len(), n);
    assert_eq!(lock.source, "fake");
    assert_eq!(lock.unknown_keys, ["saturation_current"]);
    assert_eq!(lock.keys, erc_parts::keys::names());
    // The pair carries the board's footprint name
    assert_eq!(lock.footprints[0].footprint, "Resistor_SMD:R_0402");
    let again = refresh_mpns(&board(n), &Fake::default()).unwrap();
    assert_eq!(again.to_toml(), lock.to_toml());
}

#[test]
fn an_answer_for_an_mpn_not_asked_is_refused() {
    let fake = Fake {
        extra: Some("OTHER".to_owned()),
        ..Fake::default()
    };
    let err = refresh_mpns(&board(2), &fake).unwrap_err();
    assert!(
        matches!(err, RefreshError::Answer { ref mpn, .. } if mpn == "OTHER"),
        "{err}"
    );
}

#[test]
fn a_design_from_a_file_source() {
    let text = schematic(&[
        Part {
            reference: "R1",
            fields: &[("Mpn1", "KNOWN-1")],
            dnp: false,
            variants: &[],
        },
        Part {
            reference: "R2",
            fields: &[("Mpn1", "NEW-1")],
            dnp: false,
            variants: &[],
        },
    ]);
    let data = erc_parts::PartsLock::parse(
        "parts.toml",
        "version = 1\nsource = \"hand\"\n\n[[part]]\nmpn = \"KNOWN-1\"\nmatch = \"found\"\nfacts = [\n    { key = \"resistance\", value = 10000.0, unit = \"Ω\", source = \"manual\", rank = 100, at = \"2026-11-25T00:00:00Z\" },\n    { key = \"colour\", text = \"blue\", source = \"manual\", rank = 100, at = \"2026-11-25T00:00:00Z\" },\n]\n",
    )
    .unwrap();
    let lock = refresh(
        &design(&text),
        &mut erc_parts::board::as_placed,
        &FileSource::new("file parts.toml".to_owned(), data),
    )
    .unwrap();
    assert_eq!(lock.source, "file parts.toml");
    let known = lock.part("KNOWN-1").unwrap();
    assert_eq!(known.matched, PartMatch::Found);
    // Only the asked keys; the others missing
    assert_eq!(known.facts.len(), 1);
    assert!(known.missing.contains(&"tolerance".to_owned()));
    assert_eq!(lock.part("NEW-1").unwrap().matched, PartMatch::NotFound);
}
