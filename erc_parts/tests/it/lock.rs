//! The lock's text: one fixed order, read back the same

use erc_parts::lock::{
    Cover, Coverage, Decode, FactValue, LockedFact, LockedMaker, LockedPair, LockedPart,
    LockedReview, PartKey, PartMatch, PartsLock,
};

fn fact(key: &str, value: FactValue) -> LockedFact {
    LockedFact {
        key: key.to_owned(),
        value,
        raw: None,
        source: "digikey".to_owned(),
        rank: 50,
        at: "2026-11-20T10:00:00Z".to_owned(),
        conflict: false,
    }
}

/// A lock with every kind of entry, its lists out of order
pub fn sample() -> PartsLock {
    PartsLock {
        version: 1,
        source: "fake".to_owned(),
        keys: vec!["tolerance".to_owned(), "capacitance".to_owned()],
        unknown_keys: vec![],
        parts: vec![
            LockedPart {
                mpn: "RC0402FR-0710KL".to_owned(),
                maker: None,
                matched: PartMatch::Found,
                part: Some("12".to_owned()),
                makers: vec![],
                decode: Some(Decode {
                    outcome: "decoded".to_owned(),
                    scheme: Some("yageo.rc".to_owned()),
                    segments: [("case_eia".to_owned(), "0402".to_owned())].into(),
                }),
                missing: vec!["power".to_owned()],
                facts: vec![
                    fact(
                        "tolerance",
                        FactValue::Tolerance {
                            minus: 1.0,
                            plus: 1.0,
                            unit: "%".to_owned(),
                        },
                    ),
                    fact(
                        "resistance",
                        FactValue::Number {
                            value: 10e3,
                            unit: "Ω".to_owned(),
                        },
                    ),
                    LockedFact {
                        raw: Some("X7R \"class 2\"".to_owned()),
                        conflict: true,
                        ..fact(
                            "dielectric",
                            FactValue::Text {
                                text: "X7R".to_owned(),
                            },
                        )
                    },
                    fact(
                        "rated_voltage",
                        FactValue::Range {
                            min: 0.0,
                            max: 50.0,
                            unit: "V".to_owned(),
                        },
                    ),
                ],
            },
            LockedPart {
                mpn: "GRM155R61A105KE15D".to_owned(),
                ..LockedPart::default()
            },
            // One MPN of two makers: named on one part, not on another
            LockedPart {
                mpn: "SN74LVC1G17DCKR".to_owned(),
                maker: Some("UMW".to_owned()),
                matched: PartMatch::Found,
                part: Some("2000".to_owned()),
                ..LockedPart::default()
            },
            LockedPart {
                mpn: "SN74LVC1G17DCKR".to_owned(),
                matched: PartMatch::NotUnique,
                makers: vec![
                    LockedMaker {
                        part: "1999".to_owned(),
                        maker: "Texas Instruments".to_owned(),
                    },
                    LockedMaker {
                        part: "2000".to_owned(),
                        maker: "UMW".to_owned(),
                    },
                ],
                ..LockedPart::default()
            },
        ],
        footprints: vec![
            LockedPair {
                hash: "fp1:bbbbbbbbbbbbbbbb".to_owned(),
                mpn: "RC0402FR-0710KL".to_owned(),
                footprint: "Resistor_SMD:R_0402".to_owned(),
                maker: None,
                status: Coverage::Covered,
                covered_by: vec![7, 3],
                other_hashes: vec![],
            },
            LockedPair {
                hash: "fp1:aaaaaaaaaaaaaaaa".to_owned(),
                mpn: "GRM155R61A105KE15D".to_owned(),
                footprint: "Capacitor_SMD:C_0402".to_owned(),
                maker: None,
                status: Coverage::UnknownMpn,
                covered_by: vec![],
                other_hashes: vec![],
            },
        ],
        reviews: vec![
            LockedReview {
                id: 7,
                hash: "fp1:bbbbbbbbbbbbbbbb".to_owned(),
                footprint: "Resistor_SMD:R_0402".to_owned(),
                reviewer: "Jane Doe".to_owned(),
                reviewed_on: "2026-11-02".to_owned(),
                covers: vec![Cover::Mpn {
                    mpn: "RC0402FR-0710KL".to_owned(),
                }],
                retracted: Some("2026-11-03 Jane Doe: wrong page".to_owned()),
                ..LockedReview::default()
            },
            LockedReview {
                id: 3,
                hash: "fp1:bbbbbbbbbbbbbbbb".to_owned(),
                footprint: "Resistor_SMD:R_0402".to_owned(),
                doc: Some("RC_Series".to_owned()),
                doc_rev: Some("rev12".to_owned()),
                page: Some(4),
                reviewer: "Jane Doe".to_owned(),
                reviewed_on: "2026-11-01".to_owned(),
                note: "pads per the land pattern table".to_owned(),
                covers: vec![Cover::Family {
                    scheme: "yageo.rc".to_owned(),
                    segments: [("case_eia".to_owned(), "0402".to_owned())].into(),
                }],
                ..LockedReview::default()
            },
        ],
    }
}

#[test]
fn written_sorted_and_read_back() {
    let lock = sample();
    let text = lock.to_toml();
    let back = PartsLock::parse("parts.lock.toml", &text).unwrap();
    let mut sorted = lock.clone();
    sorted.sort();
    assert_eq!(back, sorted);
    assert_eq!(back.to_toml(), text, "a second write changes nothing");
    // Parts by MPN, facts by key, pairs by hash, reviews by id
    let order = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle}\n{text}"))
    };
    assert!(order("mpn = \"GRM155R61A105KE15D\"") < order("mpn = \"RC0402FR-0710KL\""));
    assert!(order("key = \"dielectric\"") < order("key = \"resistance\""));
    assert!(order("hash = \"fp1:aaaaaaaaaaaaaaaa\"") < order("hash = \"fp1:bbbbbbbbbbbbbbbb\""));
    assert!(order("id = 3") < order("id = 7"));
    assert!(text.contains("covered_by = [3, 7]"), "{text}");
    // An MPN without a maker before the same MPN with one; the makers to choose from on one line
    assert!(order("match = \"not_unique\"") < order("maker = \"UMW\"\n"));
    assert!(
        text.contains("makers = [{ part = \"1999\", maker = \"Texas Instruments\" }, { part = \"2000\", maker = \"UMW\" }]"),
        "{text}"
    );
}

#[test]
fn entries_by_mpn_and_maker() {
    let lock = sample();
    let shared = "SN74LVC1G17DCKR";
    assert_eq!(lock.part(shared).unwrap().matched, PartMatch::NotUnique);
    let umw = lock.part_of(&PartKey::new(shared, Some("UMW"))).unwrap();
    assert_eq!(umw.part.as_deref(), Some("2000"));
    assert!(lock.part_of(&PartKey::new(shared, Some("TI"))).is_none());
    assert_eq!(
        lock.part(shared).unwrap().maker_names(),
        ["Texas Instruments", "UMW"]
    );
}

#[test]
fn one_line_per_fact() {
    let text = sample().to_toml();
    let line = text
        .lines()
        .find(|l| l.contains("key = \"resistance\""))
        .unwrap();
    assert_eq!(
        line,
        "    { key = \"resistance\", value = 10000.0, unit = \"Ω\", source = \"digikey\", rank = 50, at = \"2026-11-20T10:00:00Z\" },"
    );
}

#[test]
fn unknown_keys_are_ignored_and_newer_versions_refused() {
    let text = "version = 1\nsource = \"x\"\nfuture = 3\n\n[[part]]\nmpn = \"A\"\nmatch = \"found\"\nnew_thing = true\n";
    let lock = PartsLock::parse("parts.lock.toml", text).unwrap();
    assert_eq!(lock.parts[0].mpn, "A");
    let err = PartsLock::parse("parts.lock.toml", "version = 2\nsource = \"x\"\n").unwrap_err();
    assert_eq!(
        err.to_string(),
        "parts.lock.toml: format version 2, this better_erc reads up to 1"
    );
}
