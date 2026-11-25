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
    let got = BoardMpns::of(&design(&text));
    let mpns: Vec<(&str, Vec<&str>)> = got
        .mpns
        .iter()
        .map(|(m, refs)| (m.as_str(), refs.iter().map(String::as_str).collect()))
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
