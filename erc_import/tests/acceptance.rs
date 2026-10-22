//! What the old `ecad_file_format` and `ecad_compare` tests assert, asserted on the designs of the new importers:
//! nothing the old readers gave is lost in the port.

use erc_import::Registry;
use erc_model::layout::{Pt, Side};
use erc_model::placecmp::{self, PlaceOptions};
use erc_model::*;
use std::collections::BTreeSet;

mod common;

fn import(main: &str, others: &[&str]) -> Board {
    let files = common::files(main, others);
    let imported = Registry::default()
        .import(&Source::new(main, &files), &ImportOptions::default())
        .unwrap();
    imported.design.boards.into_iter().next().unwrap()
}

fn net_pins(b: &Board, name: &str) -> BTreeSet<String> {
    let n = b.net_by_name(name).unwrap();
    b.net(n).pins.iter().map(|p| b.pin_name(*p)).collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn pin_type(b: &Board, part: &str, number: &str) -> EcadPinType {
    let p = b.part_by_designator(part).unwrap();
    let pin = b.pins_by_number(p, number).next().unwrap();
    *b.pin(pin).etype.get().unwrap()
}

/// `edif_netlist::can_read_edif_netlist`
#[test]
fn edif() {
    let b = import("netlist_altium_edif.edf", &[]);
    let sym = b.symbols.iter().find(|s| s.name == "CONN-2-3 (6)").unwrap();
    assert_eq!(sym.pins.len(), 6);
    let pin1 = sym.pins.iter().find(|p| p.number == "1").unwrap();
    assert_eq!(pin1.etype, EcadPinType::Bidirectional);
    assert_eq!(b.nets.len(), 3);
    assert_eq!(net_pins(&b, "NetR13_1"), set(&["R13.1", "SW4.4"]));
    // The old reader's components were the EDIF instances; the parts only nets name have no library symbol
    assert_eq!(b.parts.iter().filter(|p| p.symbol.is_some()).count(), 2);
    let j2 = b.part(b.part_by_designator("J2").unwrap());
    assert_eq!(
        j2.value.get(),
        Some(&Value::Text("95278-101A06LF".into())),
        "the value falls back on the Comment property"
    );
    assert_eq!(j2.field("ChannelOffset"), Some("1"));
}

/// `wirelist::can_read_altium_wirelist_netlist`, and `altium_netlist::load_altium_netlist`: the WireList's pin
/// names and types over EDIF's
#[test]
fn wirelist_and_altium() {
    let w = import("netlist_altium_wirelist.net", &[]);
    assert_eq!(w.nets.len(), 3);
    assert_eq!(pin_type(&w, "J1", "A4"), EcadPinType::PowerIn);
    let b = import("netlist_altium_edif.edf", &["netlist_altium_wirelist.net"]);
    assert_eq!(
        pin_type(&b, "J2", "1"),
        EcadPinType::Passive,
        "WireList wins"
    );
    let j2 = b.part_by_designator("J2").unwrap();
    let pin = b.pins_by_number(j2, "1").next().unwrap();
    assert_eq!(
        b.pin(pin).etype.0.len(),
        2,
        "EDIF's direction is kept as a second source"
    );
    assert_eq!(b.part(j2).field("Footprint"), Some("HEADER-2.54-2x3-VSMT"));
    let sw4 = b.part_by_designator("SW4").unwrap();
    let a1 = b.pins_by_number(sw4, "1").next().unwrap();
    assert_eq!(b.pin(a1).name, "A1");
}

/// `orcad_netlist::can_load_orcad_netlist` and erc_core's `orcad_netlist_reaches_pcb_assembly`
#[test]
fn orcad() {
    let b = import(
        "netlist_orcad_pstxnet.dat",
        &["netlist_orcad_pstxprt.dat", "netlist_orcad_pstchip.dat"],
    );
    assert_eq!(b.nets.len(), 3);
    assert_eq!(
        net_pins(&b, "TOUCH_INT_N"),
        set(&["Q34.3", "R610.2", "R636.1"])
    );
    let emmc = b
        .symbols
        .iter()
        .find(|s| s.name.contains("EMMC"))
        .expect("the EMMC primitive");
    let a3 = emmc.pins.iter().find(|p| p.number == "A3").unwrap();
    assert_eq!(a3.etype, EcadPinType::Bidirectional);
    assert_eq!(a3.unit, 1);
    assert!(emmc.unit_names.contains(&(1, "A".to_string())), "section A");
    let c1 = b.part(b.part_by_designator("C1").unwrap());
    let sym = &b.symbols[c1.symbol.unwrap().index()];
    assert_eq!(sym.pins.len(), 2);
    assert_eq!(c1.field("Value"), Some("10pF"));
    let u8 = b.part_by_designator("U8").unwrap();
    assert_eq!(pin_type(&b, "U8", "A3"), EcadPinType::Bidirectional);
    assert_eq!(b.part(u8).units.len(), 3);
    assert_eq!(
        b.sheet_path(b.part(b.part_by_designator("C1").unwrap()).sheet),
        "/page3/",
        "the section's page"
    );
}

fn footprint(b: &Board, reference: &str) -> (Pt, Side) {
    let l = b.layout.as_ref().unwrap();
    let f = l
        .footprints
        .iter()
        .find(|f| f.reference == reference)
        .unwrap();
    (f.at, f.side)
}

fn mm(text: &str) -> i64 {
    layout::Nm::parse(text).unwrap()
}

/// `pnp::can_read_pnp_*` and `csv_util::can_find_header_row_*`
#[test]
fn pnp() {
    let k = import("pnp_kicad.csv", &[]);
    let (at, side) = footprint(&k, "R1");
    assert_eq!((at.x, side), (mm("56.6"), Side::Back));
    let (at, side) = footprint(&k, "C1");
    assert_eq!((at.y, side), (mm("5.85"), Side::Front));
    for f in ["pnp_altium_no_units.csv", "pnp_altium_with_units.csv"] {
        let a = import(f, &[]);
        let (at, side) = footprint(&a, "J1");
        assert_eq!((at.x, side), (mm("1.3943"), Side::Front), "{f}");
        let (at, side) = footprint(&a, "R1");
        assert_eq!((at.y, side), (mm("15.1"), Side::Back), "{f}");
    }
    let al = import("pnp_allegro.csv", &[]);
    let (at, side) = footprint(&al, "R1");
    assert_eq!((at.x, side), (mm("30.02"), Side::Back));
    let (at, side) = footprint(&al, "C1");
    assert_eq!((at.y, side), (mm("-7.8"), Side::Front));
}

/// The header row: row 0 (KiCad), 9 (Altium), 6 (Allegro), seen in the note on the lines before it
#[test]
fn pnp_header_rows() {
    for (f, lines) in [
        ("pnp_kicad.csv", 0),
        ("pnp_altium_no_units.csv", 9),
        ("pnp_allegro.csv", 6),
    ] {
        let files = common::files(f, &[]);
        let imported = Registry::default()
            .import(&Source::new(f, &files), &ImportOptions::default())
            .unwrap();
        let said = imported
            .notes
            .iter()
            .find_map(|n| {
                n.message
                    .strip_suffix(" lines of file information before the header are not kept")
            })
            .map_or(0, |n| n.parse().unwrap());
        assert_eq!(said, lines, "{f}");
    }
}

/// `ecad_compare::pnp::compare_positions_works`, now over two designs
#[test]
fn pnp_compare() {
    let a = import("pnp_allegro.csv", &[]);
    let b = import("pnp_altium_no_units.csv", &[]);
    let d = placecmp::compare_boards(&a, &b, &PlaceOptions::default()).unwrap();
    assert_eq!(d.changed[0].reference, "R1");
    assert_eq!(d.removed, ["C1"]);
    assert_eq!(d.added, ["J1"]);
}
