//! A file source: coverage by MPN and by family, a review accepted and read back

use erc_parts::lock::{Cover, Coverage, Decode, LockedPart, PartKey, PartMatch, Placement};
use erc_parts::{FileSource, NewReview, PartsLock, Source};
use std::collections::BTreeMap;

const HASH: &str = "fp1:0123456789abcdef";
const OTHER: &str = "fp1:fedcba9876543210";

fn rc(mpn: &str, case: &str) -> LockedPart {
    LockedPart {
        mpn: mpn.to_owned(),
        matched: PartMatch::Found,
        decode: Some(Decode {
            outcome: "decoded".to_owned(),
            scheme: Some("yageo.rc".to_owned()),
            segments: [
                ("case_eia".to_owned(), case.to_owned()),
                ("resistance".to_owned(), "10kΩ".to_owned()),
            ]
            .into(),
        }),
        ..LockedPart::default()
    }
}

fn data() -> PartsLock {
    PartsLock {
        version: 1,
        source: "hand".to_owned(),
        parts: vec![
            rc("RC0402FR-0710KL", "0402"),
            rc("RC0402FR-071KL", "0402"),
            rc("RC0603FR-0710KL", "0603"),
            LockedPart {
                mpn: "TPS62840DLCR".to_owned(),
                matched: PartMatch::Found,
                ..LockedPart::default()
            },
        ],
        ..PartsLock::default()
    }
}

fn review(hash: &str, covers: Vec<Cover>) -> NewReview {
    NewReview {
        hash: hash.to_owned(),
        footprint: "Resistor_SMD:R_0402".to_owned(),
        doc: Some("RC_Series".to_owned()),
        page: Some(4),
        scale: Some(0.25),
        at: Some(Placement {
            x: 100.0,
            y: 200.0,
            rotation: 90.0,
            mirror: false,
        }),
        reviewer: "Jane Doe".to_owned(),
        reviewed_on: "2026-11-25".to_owned(),
        covers,
        ..NewReview::default()
    }
}

fn family(case: &str) -> Cover {
    Cover::Family {
        scheme: "yageo.rc".to_owned(),
        segments: [("case_eia".to_owned(), case.to_owned())].into(),
    }
}

fn mpns(list: &[&str]) -> Vec<String> {
    list.iter().map(|m| (*m).to_owned()).collect()
}

#[test]
fn a_family_covers_its_package_only() {
    let s = FileSource::new("hand".to_owned(), data());
    let r = s.accept(&review(HASH, vec![family("0402")])).unwrap();
    assert_eq!(r.id, 1);
    let a = s
        .reviews(
            HASH,
            &mpns(&[
                "RC0402FR-0710KL",
                "RC0402FR-071KL",
                "RC0603FR-0710KL",
                "NOPE",
            ]),
        )
        .unwrap();
    let status: Vec<Coverage> = a.pairs.iter().map(|p| p.status).collect();
    assert_eq!(
        status,
        [
            Coverage::Covered,
            Coverage::Covered,
            Coverage::Uncovered,
            Coverage::UnknownMpn
        ]
    );
    assert_eq!(a.pairs[0].covered_by, [1]);
    assert_eq!(a.reviews.len(), 1);
}

#[test]
fn a_review_of_another_hash_is_named_not_counted() {
    let s = FileSource::new("hand".to_owned(), data());
    s.accept(&review(
        OTHER,
        vec![Cover::Mpn {
            mpn: "TPS62840DLCR".to_owned(),
        }],
    ))
    .unwrap();
    let a = s.reviews(HASH, &mpns(&["TPS62840DLCR"])).unwrap();
    assert_eq!(a.pairs[0].status, Coverage::Uncovered);
    assert_eq!(a.pairs[0].other_hashes, [1]);
    assert_eq!(a.reviews[0].hash, OTHER);
}

#[test]
fn a_retracted_review_covers_nothing() {
    let mut d = data();
    let s = FileSource::new("hand".to_owned(), d.clone());
    let mut r = s.accept(&review(HASH, vec![family("0402")])).unwrap();
    r.retracted = Some("2026-11-26 Jane Doe: wrong page".to_owned());
    d.reviews.push(r);
    let s = FileSource::new("hand".to_owned(), d);
    let a = s.reviews(HASH, &mpns(&["RC0402FR-0710KL"])).unwrap();
    assert_eq!(a.pairs[0].status, Coverage::Uncovered);
}

#[test]
fn refusals() {
    let s = FileSource::new("hand".to_owned(), data());
    let err = |r: NewReview| s.accept(&r).unwrap_err().cause.to_string();
    assert!(err(review("R_0402", vec![family("0402")])).contains("no review hash"));
    assert!(err(review(HASH, vec![])).contains("1 to 200"));
    let empty = Cover::Family {
        scheme: "yageo.rc".to_owned(),
        segments: BTreeMap::new(),
    };
    assert!(err(review(HASH, vec![empty])).contains("names no segment"));
    let unknown = Cover::Mpn {
        mpn: "NOPE".to_owned(),
    };
    assert_eq!(
        err(review(HASH, vec![unknown])),
        "no single part has MPN NOPE"
    );
    let mut nobody = review(HASH, vec![family("0402")]);
    nobody.reviewer.clear();
    assert!(err(nobody).contains("reviewer"));
    assert!(s.data().reviews.is_empty(), "nothing refused was kept");
}

#[test]
fn accepted_into_the_file_and_read_back() {
    let dir = std::env::temp_dir().join(format!("erc_parts-source-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("parts.toml");
    std::fs::write(&path, data().to_toml()).unwrap();
    let s = FileSource::open(&path).unwrap();
    s.accept(&review(HASH, vec![family("0402")])).unwrap();
    let back = PartsLock::read(&path).unwrap();
    let r = &back.reviews[0];
    assert_eq!(r.scale, Some(0.25));
    assert_eq!(r.at.unwrap().rotation, 90.0);
    assert_eq!(r.covers, [family("0402")]);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("at = { x = 100.0, y = 200.0, rotation = 90.0, mirror = false }"),
        "{text}"
    );
    // A second source on the file sees it and numbers on
    let again = FileSource::open(&path).unwrap();
    assert_eq!(
        again
            .accept(&review(HASH, vec![family("0603")]))
            .unwrap()
            .id,
        2
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_family_of_a_part() {
    assert_eq!(
        Cover::family_of(&rc("RC0402FR-0710KL", "0402")),
        Some(family("0402"))
    );
    let ic = LockedPart {
        mpn: "TPS62840DLCR".to_owned(),
        ..LockedPart::default()
    };
    assert_eq!(Cover::family_of(&ic), None);
}

#[test]
fn parts_by_mpn_and_maker() {
    let mut d = data();
    d.parts.push(LockedPart {
        maker: Some("Yageo".to_owned()),
        part: Some("12".to_owned()),
        ..rc("RC0402FR-0710KL", "0402")
    });
    let s = FileSource::new("hand".to_owned(), d);
    let asked = [
        PartKey::mpn("RC0402FR-0710KL"),
        PartKey::new("RC0402FR-0710KL", Some("Yageo")),
        PartKey::new("RC0402FR-0710KL", Some("Vishay")),
    ];
    let a = s.parts(&asked, &[]).unwrap();
    let got: Vec<(PartKey, PartMatch, Option<&str>)> = a
        .parts
        .iter()
        .map(|p| (p.key(), p.matched, p.part.as_deref()))
        .collect();
    assert_eq!(
        got,
        [
            (asked[0].clone(), PartMatch::Found, None),
            (asked[1].clone(), PartMatch::Found, Some("12")),
            (asked[2].clone(), PartMatch::NotFound, None),
        ]
    );
}
