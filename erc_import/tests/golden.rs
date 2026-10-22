//! Every fixture through the registry: the importer it picks, a committed canonical dump of the design and of the
//! import notes (`tests/golden`, `BLESS=1` rewrites them), and `dump → read → dump` stable.

use erc_import::Registry;
use erc_model::{Design, ImportOptions, MemFiles, Source, dump};
use std::path::Path;

/// A golden name, the importer the registry must pick, the main file and the files beside it
pub const CASES: &[(&str, &str, &str, &[&str])] = &[
    (
        "altium",
        "altium",
        "netlist_altium_edif.edf",
        &["netlist_altium_wirelist.net"],
    ),
    (
        "altium_from_wirelist",
        "altium",
        "netlist_altium_wirelist.net",
        &["netlist_altium_edif.edf"],
    ),
    ("altium_edif_only", "altium", "netlist_altium_edif.edf", &[]),
    (
        "altium_wirelist_only",
        "altium",
        "netlist_altium_wirelist.net",
        &[],
    ),
    (
        "orcad",
        "orcad",
        "netlist_orcad_pstxnet.dat",
        &["netlist_orcad_pstxprt.dat", "netlist_orcad_pstchip.dat"],
    ),
    (
        "orcad_from_pstchip",
        "orcad",
        "netlist_orcad_pstchip.dat",
        &["netlist_orcad_pstxprt.dat", "netlist_orcad_pstxnet.dat"],
    ),
    ("orcad_pstxnet_only", "orcad", "netlist_orcad_pstxnet.dat", &[]),
    ("pnp_kicad", "pnp", "pnp_kicad.csv", &[]),
    ("pnp_altium_no_units", "pnp", "pnp_altium_no_units.csv", &[]),
    ("pnp_altium_with_units", "pnp", "pnp_altium_with_units.csv", &[]),
    ("pnp_allegro", "pnp", "pnp_allegro.csv", &[]),
];

pub fn fixture(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

pub fn files(main: &str, others: &[&str]) -> MemFiles {
    let mut f = MemFiles::default().with(main, fixture(main));
    for o in others {
        f = f.with(*o, fixture(o));
    }
    f
}

#[test]
fn golden_dumps() {
    let reg = Registry::default();
    let mut failed = Vec::new();
    for (name, importer, main, others) in CASES {
        let files = files(main, others);
        let src = Source::new(*main, &files);
        let (picked, _) = reg.pick(&src).unwrap_or_else(|| panic!("{name}: no importer"));
        assert_eq!(picked.name(), *importer, "{name}");
        let imported = reg
            .import(&src, &ImportOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let text = dump::write(&imported.design);
        let notes: String = imported.notes.iter().map(|n| format!("{n}\n")).collect();
        let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(format!("{name}.dump"));
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
        let back: Design = dump::read(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(dump::write(&back), text, "{name}: dump, read, dump");
        assert!(back == imported.design, "{name}: read gives back the design");
    }
    assert!(
        failed.is_empty(),
        "dumps differ (BLESS=1 to update): {failed:#?}"
    );
}
