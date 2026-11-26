//! Which MPNs of a design a snapshot covers

use super::{Part, design, schematic};
use erc_parts::BoardMpns;

#[test]
fn every_mpn_field_of_fitted_parts_in_every_variant() {
    let text = schematic(&[
        // Two MPN fields, both count
        Part {
            reference: "R1",
            fields: &[("Mpn1", "RC0402FR-0710KL"), ("Mpn2", "ERJ-2RKF1002X")],
            dnp: false,
            variants: &[],
        },
        // DNP in the base design, fitted in Lite with its own MPN
        Part {
            reference: "R2",
            fields: &[("MPN", "RC0402FR-071KL")],
            dnp: true,
            variants: &[("Lite", false, &[("MPN", "RC0402FR-074K7L")])],
        },
        // Fitted in the base design, DNP in Lite
        Part {
            reference: "R3",
            fields: &[("Mpn1", "RC0402FR-0710KL")],
            dnp: false,
            variants: &[("Lite", true, &[])],
        },
        // An empty MPN field is no MPN
        Part {
            reference: "R4",
            fields: &[("Mpn1", ""), ("Mpn2", "~")],
            dnp: false,
            variants: &[],
        },
    ]);
    let got = BoardMpns::of(&design(&text), &mut erc_parts::board::as_placed);
    let mpns: Vec<(&str, Vec<&str>)> = got
        .mpns
        .iter()
        .map(|(m, refs)| (m.mpn.as_str(), refs.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(
        mpns,
        [
            ("ERJ-2RKF1002X", vec!["R1"]),
            ("RC0402FR-0710KL", vec!["R1", "R3"]),
            ("RC0402FR-074K7L", vec!["R2"]),
        ]
    );
    // No board file: no footprint pairs
    assert!(got.footprints.is_empty());
}

#[test]
fn the_maker_beside_each_mpn() {
    let text = schematic(&[
        // Mfg1 beside Mpn1; Mpn2 names no maker
        Part {
            reference: "U1",
            fields: &[
                ("Mpn1", "SN74LVC1G17DCKR"),
                ("Mfg1", "UMW"),
                ("Mpn2", "74LVC1G17GW"),
            ],
            dnp: false,
            variants: &[],
        },
        // The same MPN without a maker is its own entry
        Part {
            reference: "U2",
            fields: &[("MPN", "SN74LVC1G17DCKR"), ("Mfg", "~")],
            dnp: false,
            variants: &[],
        },
        // A variant names another maker
        Part {
            reference: "U3",
            fields: &[("Mpn", "SN74LVC1G17DCKR"), ("Mfg", "UMW")],
            dnp: false,
            variants: &[("Lite", false, &[("Mfg", "Texas Instruments")])],
        },
    ]);
    let got = BoardMpns::of(&design(&text), &mut erc_parts::board::as_placed);
    let keys: Vec<(String, Vec<&str>)> = got
        .mpns
        .iter()
        .map(|(k, refs)| (k.describe(), refs.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(
        keys,
        [
            ("74LVC1G17GW".to_owned(), vec!["U1"]),
            ("SN74LVC1G17DCKR".to_owned(), vec!["U2"]),
            ("SN74LVC1G17DCKR (Texas Instruments)".to_owned(), vec!["U3"]),
            ("SN74LVC1G17DCKR (UMW)".to_owned(), vec!["U1", "U3"]),
        ]
    );
}
