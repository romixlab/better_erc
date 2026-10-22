//! The board reader on one board saved by KiCad 8 (`hierarchy.kicad_pcb`, read with its schematic), 9 and 10
//! (`board_kicad9.kicad_pcb`, `board_kicad10.kicad_pcb`, read alone): golden dumps (`BLESS=1` rewrites them), no
//! import notes, the same geometry and nets from all three formats, and the board linked to the schematic.

use erc_model::{Design, DirFiles, ImportOptions, Layout, MemFiles, NoteKind, dump};
use std::path::Path;

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn import(main: &str) -> Design {
    let imported = erc_kicad::project::read(&DirFiles(fixtures()), main, &ImportOptions::default())
        .unwrap_or_else(|e| panic!("{main}: {e}"));
    let unknown: Vec<_> = imported
        .notes
        .iter()
        .filter(|n| n.kind == NoteKind::Unknown || n.file.ends_with(".kicad_pcb"))
        .collect();
    assert!(unknown.is_empty(), "{main}: {unknown:#?}");
    imported.design
}

/// What every format must agree on: copper, graphics, pads with their nets
fn geometry(lay: &Layout) -> String {
    let mut out = String::new();
    for t in &lay.tracks {
        out += &format!("{}\n", dump::track_tokens(t).join(" "));
    }
    for v in &lay.vias {
        out += &format!("{}\n", dump::via_tokens(v).join(" "));
    }
    for z in &lay.zones {
        out += &format!("{}\n", dump::zone_tokens(z).join(" "));
    }
    for s in &lay.graphics {
        out += &format!("{}\n", dump::shape_tokens(s).join(" "));
    }
    for f in &lay.footprints {
        out += &format!("fp {} {} {} {:?}\n", f.reference, f.at, f.rotation, f.side);
        for s in &f.graphics {
            out += &format!("  {}\n", dump::shape_tokens(s).join(" "));
        }
    }
    for p in &lay.pads {
        out += &format!(
            "pad {} {} {} {:?} {:?}\n",
            lay.footprint(p.footprint).reference,
            p.number,
            p.at,
            p.drill,
            p.net_name
        );
    }
    let mut nets = lay.net_names.clone();
    nets.sort();
    out += &format!("nets {nets:?}\n");
    out
}

#[test]
fn boards_of_three_kicad_versions() {
    let mut failed = Vec::new();
    let mut layouts = Vec::new();
    for main in [
        "hierarchy.kicad_pcb",
        "board_kicad9.kicad_pcb",
        "board_kicad10.kicad_pcb",
    ] {
        let design = import(main);
        let text = dump::write(&design);
        let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/pcb")
            .join(format!("{main}.dump"));
        if std::env::var_os("BLESS").is_some() {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &text).unwrap();
        }
        if std::fs::read_to_string(&golden).unwrap_or_default() != text {
            failed.push(format!("{main}: dump differs (BLESS=1 to update)"));
        }
        let back = dump::read(&text).unwrap_or_else(|e| panic!("{main}: {e}"));
        assert_eq!(dump::write(&back), text, "{main}: dump, read, dump");
        assert!(back == design, "{main}: read gives back the design");
        layouts.push(design.boards[0].layout.clone().expect("a layout"));
    }
    assert!(failed.is_empty(), "{failed:#?}");
    let g8 = geometry(&layouts[0]);
    assert_eq!(g8, geometry(&layouts[1]), "KiCad 8 and 9");
    assert_eq!(g8, geometry(&layouts[2]), "KiCad 8 and 10");
}

#[test]
fn board_linked_to_schematic() {
    let design = import("hierarchy.kicad_sch");
    let b = &design.boards[0];
    let lay = b
        .layout
        .as_ref()
        .expect("the board is read with the schematic");
    for f in &lay.footprints {
        // Board-only footprints (test point, net tie, a header) have no part
        assert_eq!(
            f.part.is_some(),
            !f.attrs.board_only,
            "footprint {}",
            f.reference
        );
        if let Some(p) = f.part {
            assert_eq!(b.part(p).designator, f.reference);
        }
    }
    for p in &lay.pads {
        if let Some(n) = &p.net_name {
            let net = p.net.map(|n| b.net(n).name.as_str());
            assert_eq!(net, Some(n.as_str()), "pad net {n} is a schematic net");
        }
    }
    assert!(
        lay.pads
            .iter()
            .any(|p| p.property.as_deref() == Some("pad_prop_testpoint"))
    );
    assert_eq!(
        lay.footprints
            .iter()
            .find(|f| f.reference == "NT1")
            .unwrap()
            .net_tie_pad_groups,
        [vec!["1".to_string(), "2".to_string()]]
    );
}

/// The hierarchy fixture with its board text changed by `edit`
fn import_edited(edit: impl Fn(&str) -> String) -> erc_model::Imported {
    let read = |f: &str| std::fs::read(fixtures().join(f)).unwrap();
    let pcb = String::from_utf8(read("hierarchy.kicad_pcb")).unwrap();
    let files = MemFiles::default()
        .with("hierarchy.kicad_sch", read("hierarchy.kicad_sch"))
        .with("child.kicad_sch", read("child.kicad_sch"))
        .with("hierarchy.kicad_pcb", edit(&pcb));
    erc_kicad::project::read(&files, "hierarchy.kicad_sch", &ImportOptions::default()).unwrap()
}

fn conflicts(i: &erc_model::Imported) -> Vec<String> {
    i.notes
        .iter()
        .filter(|n| n.kind == NoteKind::Conflict)
        .map(|n| format!("{}: {}", n.at, n.message))
        .collect()
}

/// A re-annotation after the board was updated: the footprint says R99, its schematic path is still R1's symbol
#[test]
fn footprint_linked_by_path_not_designator() {
    let i = import_edited(|t| t.replace("\"Reference\" \"R1\"", "\"Reference\" \"R99\""));
    let b = &i.design.boards[0];
    let fp = b
        .layout
        .as_ref()
        .unwrap()
        .footprints
        .iter()
        .find(|f| f.reference == "R99")
        .unwrap();
    assert_eq!(b.part(fp.part.expect("linked by path")).designator, "R1");
    let c = conflicts(&i);
    assert_eq!(c.len(), 1, "{c:#?}");
    assert!(
        c[0].contains("footprint R99") && c[0].contains("R1"),
        "{c:?}"
    );
}

/// No path (or one that names no symbol): the designator links, and a dead path is noted
#[test]
fn footprint_designator_is_the_fallback() {
    let none = import_edited(|t| t.replace("(path \"/30000000-0000-4000-8000-000000000001\")", ""));
    assert!(conflicts(&none).is_empty());
    let b = &none.design.boards[0];
    let r1 = b
        .layout
        .as_ref()
        .unwrap()
        .footprints
        .iter()
        .find(|f| f.reference == "R1")
        .unwrap();
    assert_eq!(b.part(r1.part.unwrap()).designator, "R1");

    let dead = import_edited(|t| t.replace("/30000000-0000-4000-8000-000000000009\"", "/dead\""));
    let c = conflicts(&dead);
    assert_eq!(c.len(), 1, "{c:#?}");
    assert!(c[0].contains("names no symbol"), "{c:?}");
    let b = &dead.design.boards[0];
    let r1 = b
        .layout
        .as_ref()
        .unwrap()
        .footprints
        .iter()
        .find(|f| f.reference == "R1")
        .unwrap();
    assert!(r1.part.is_some());
}
