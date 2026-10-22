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
        .join("../ecad_file_format/test_input")
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
