//! The KiCad netlist importer on the committed netlists of KiCad 8, 9 and 10: a golden dump per file (`BLESS=1`
//! rewrites them), `dump → read → dump` stable, and no import notes.

use erc_kicad::netlist;
use erc_model::dump;
use std::path::Path;

const FIXTURES: &[&str] = &[
    "kicad_versions/fixture.kicad8.net",
    "kicad_versions/fixture.kicad9.net",
    "kicad_versions/fixture.kicad10.net",
    "kicad_versions/fixture_kicad10.kicad10.net",
    "netlist_kicad.net",
];

fn input(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/netlist")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn golden_dumps() {
    let mut failed = Vec::new();
    for rel in FIXTURES {
        let (board, notes) = netlist::read(&input(rel), rel).unwrap();
        let unknown: Vec<_> = notes
            .iter()
            .filter(|n| n.kind == erc_model::NoteKind::Unknown)
            .collect();
        assert!(unknown.is_empty(), "{rel}: {unknown:#?}");
        let notes: String = notes.iter().map(|n| format!("{n}\n")).collect();
        let design = erc_model::Design::single(board);
        let text = dump::write(&design);
        let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/netlist")
            .join(rel.replace('/', "__") + ".dump");
        let golden_notes = golden.with_extension("notes");
        if std::env::var_os("BLESS").is_some() {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &text).unwrap();
            if notes.is_empty() {
                let _ = std::fs::remove_file(&golden_notes);
            } else {
                std::fs::write(&golden_notes, &notes).unwrap();
            }
        }
        if std::fs::read_to_string(&golden).unwrap_or_default() != text {
            failed.push(golden.display().to_string());
        }
        if std::fs::read_to_string(&golden_notes).unwrap_or_default() != notes {
            failed.push(golden_notes.display().to_string());
        }
        let back = dump::read(&text).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(dump::write(&back), text, "{rel}: dump, read, dump");
        assert!(back == design, "{rel}: read gives back the design");
    }
    assert!(
        failed.is_empty(),
        "dumps differ (BLESS=1 to update): {failed:#?}"
    );
}

fn read(rel: &str) -> erc_model::Board {
    netlist::read(&input(rel), rel).unwrap().0
}

fn net_pins(b: &erc_model::Board, name: &str) -> Vec<String> {
    let n = b.net_by_name(name).unwrap_or_else(|| panic!("net {name}"));
    let mut pins: Vec<String> = b.net(n).pins.iter().map(|p| b.pin_name(*p)).collect();
    pins.sort();
    pins
}

/// The parts, each net with its `part.pin`s, every symbol pin with its type
type Summary = (Vec<String>, Vec<(String, Vec<String>)>, Vec<String>);

/// Nets as sets of `part.pin`, and every symbol pin's type: what must read the same from every KiCad version
fn summary(b: &erc_model::Board) -> Summary {
    let mut parts: Vec<String> = b.parts.iter().map(|p| p.designator.clone()).collect();
    parts.sort();
    let mut nets: Vec<(String, Vec<String>)> = b
        .nets
        .iter()
        .map(|n| (n.name.clone(), net_pins(b, &n.name)))
        .collect();
    nets.sort();
    let mut pins: Vec<String> = b
        .symbols
        .iter()
        .flat_map(|s| {
            s.pins
                .iter()
                .map(move |p| format!("{} {} {} {:?}", s.name, p.number, p.name, p.etype))
        })
        .collect();
    pins.sort();
    (parts, nets, pins)
}

/// What the old `ecad_file_format::kicad_netlist` tests asserted, on the board model: nothing the old reader gave
/// is lost
#[test]
fn acceptance() {
    // kicad_8_9_and_10_netlists_read_the_same
    let kicad8 = read("kicad_versions/fixture.kicad8.net");
    let kicad9 = read("kicad_versions/fixture.kicad9.net");
    let kicad10 = read("kicad_versions/fixture.kicad10.net");
    let expected = summary(&kicad8);
    assert_eq!(summary(&kicad9), expected);
    assert_eq!(summary(&kicad10), expected);
    assert_eq!(expected.0.len(), 3);
    assert_eq!(net_pins(&kicad8, "/MID"), ["JP1.2", "R1.2"]);
    let u1 = kicad8.symbols.iter().find(|s| s.pins.len() == 6).unwrap();
    let pin7 = u1.pins.iter().find(|p| p.number == "7").unwrap();
    assert_eq!(pin7.etype, erc_model::EcadPinType::Output);

    // kicad_9_and_10_netlists_keep_net_classes
    let class = |b: &erc_model::Board| {
        let n = b.net_by_name("/OUTA").unwrap();
        b.net(n).class.get().cloned()
    };
    assert_eq!(class(&kicad9).as_deref(), Some("Default"));
    assert_eq!(class(&kicad10).as_deref(), Some("Default"));
    assert_eq!(class(&kicad8), None);

    // kicad_10_units_become_component_sections
    let names = |b: &erc_model::Board| {
        let u1 = b.part(b.part_by_designator("U1").unwrap());
        b.symbols[u1.symbol.unwrap().index()].unit_names.clone()
    };
    assert_eq!(
        names(&kicad10),
        [(1, "A".to_string()), (2, "B".to_string())]
    );
    assert!(
        names(&kicad9).is_empty(),
        "before KiCad 10 the netlist has no unit list"
    );

    // kicad_10_netlist_with_variants_jumpers_and_groups_loads: the base value, not the "Lite" variant's override
    let variants = read("kicad_versions/fixture_kicad10.kicad10.net");
    assert_eq!(summary(&variants).1, summary(&kicad10).1);
    let r1 = variants.part(variants.part_by_designator("R1").unwrap());
    assert_eq!(r1.field("Value"), Some("10k"));

    // can_read_netlist_kicad_sexpr and can_read_netlist_kicad
    let (b, _) = netlist::read(&input("netlist_kicad.net"), "netlist_kicad.net").unwrap();
    assert_eq!(b.source.version.as_deref(), Some("8.0.4"));
    assert_eq!(b.nets.len(), 4);
    assert_eq!(net_pins(&b, "/Eth/RXD0"), ["R21.2", "R29.2", "U2.11"]);
}
