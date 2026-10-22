//! The registry picks the importer by the main file's name and contents, KiCad's among them

use erc_import::Registry;
use erc_model::{Confidence, DirFiles, ImportError, ImportOptions, MemFiles, Source};
use std::path::Path;

mod common;

#[test]
fn picks_by_contents_and_name() {
    let reg = Registry::default();
    let names: Vec<&str> = reg.importers().map(|i| i.name()).collect();
    assert_eq!(names, ["kicad", "kicad-netlist", "altium", "orcad", "pnp"]);

    let kicad_net = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../ecad_file_format/test_input/netlist_kicad.net"),
    )
    .unwrap();
    let files = MemFiles::default()
        .with("board.net", kicad_net)
        .with("wires.NET", common::fixture("netlist_altium_wirelist.net"))
        .with("odd.edf", "not EDIF at all")
        .with("bom.csv", "Item,Qty\n1,2\n")
        .with("notes.txt", "hello");
    let pick = |main: &str| {
        reg.pick(&Source::new(main, &files))
            .map(|(i, c)| (i.name(), c))
    };
    assert_eq!(pick("board.net"), Some(("kicad-netlist", Confidence::Yes)));
    assert_eq!(
        pick("wires.NET"),
        Some(("altium", Confidence::Yes)),
        "contents over the extension"
    );
    assert_eq!(pick("odd.edf"), Some(("altium", Confidence::Maybe)));
    assert_eq!(pick("bom.csv"), Some(("pnp", Confidence::Maybe)));
    assert_eq!(pick("notes.txt"), None);
    assert_eq!(pick("missing.kicad_sch"), None);
    assert!(matches!(
        reg.import(&Source::new("notes.txt", &files), &ImportOptions::default()),
        Err(ImportError::Unsupported(_))
    ));
}

#[test]
fn scan_a_directory() {
    let reg = Registry::default();
    let dir = DirFiles(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let found: Vec<(String, &str)> = reg
        .scan(&dir)
        .into_iter()
        .filter(|c| c.confidence == Confidence::Yes)
        .map(|c| (c.main, c.importer))
        .collect();
    assert_eq!(found.len(), 9, "{found:#?}");
    assert!(found.contains(&("netlist_orcad_pstchip.dat".into(), "orcad")));
    assert!(found.contains(&("pnp_allegro.csv".into(), "pnp")));
    // EDIF found beside the WireList in a real directory
    let imported = reg
        .import(
            &Source::new("netlist_altium_wirelist.net", &dir),
            &ImportOptions::default(),
        )
        .unwrap();
    assert_eq!(imported.design.boards[0].source.files.len(), 2);
}
