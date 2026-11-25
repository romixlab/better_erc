//! `berc fields sync` on `bom-12_mpn_vs_fields`: the display fields written from its parts snapshot, a field at
//! the board default removed, a new one added and placed, and BOM-12's display findings gone afterwards

use better_erc::fields::{Op, SyncOptions, render, sync};
use better_erc::{CheckOptions, Project};
use std::path::{Path, PathBuf};

fn design(name: &str) -> PathBuf {
    let from = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test_schematics/sources/bom-12_mpn_vs_fields");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("fields-sync-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in std::fs::read_dir(from).unwrap() {
        let f = f.unwrap().path();
        let name = f.file_name().unwrap().to_str().unwrap();
        // The snapshot sits beside a design as parts.lock.toml
        let to = if name.ends_with(".parts.lock.toml") {
            "parts.lock.toml"
        } else {
            name
        };
        std::fs::copy(&f, dir.join(to)).unwrap();
    }
    dir
}

/// The BOM-12 findings on a display field (`item C2 Voltage`), by subject
fn display_findings(dir: &Path) -> Vec<String> {
    let p = Project::open(dir).unwrap();
    let run = p.check(&CheckOptions::default()).unwrap();
    let mut out: Vec<String> = run
        .report
        .findings
        .iter()
        .filter(|f| f.check == "BOM-12")
        .map(|f| f.message.clone())
        .filter(|m| {
            ["Voltage", "Power", "Dielectric"]
                .iter()
                .any(|n| m.contains(&format!("write {n}")))
        })
        .collect();
    out.sort();
    out
}

fn ops(dir: &Path, write: bool) -> (better_erc::fields::Synced, String) {
    let s = sync(
        dir,
        &SyncOptions {
            write,
            ..SyncOptions::default()
        },
    )
    .unwrap();
    let text = render(&s);
    (s, text)
}

#[test]
fn dry_run_writes_nothing() {
    let dir = design("dry");
    let file = dir.join("bom-12_mpn_vs_fields.kicad_sch");
    let before = std::fs::read_to_string(&file).unwrap();
    let (s, text) = ops(&dir, false);
    assert!(s.written.is_empty());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    assert!(text.contains("Dry run:"), "{text}");
}

#[test]
fn writes_the_worst_of_the_mpns() {
    let dir = design("write");
    assert_eq!(
        display_findings(&dir).len(),
        4,
        "{:?}",
        display_findings(&dir)
    );
    let (s, text) = ops(&dir, true);
    let find = |r: &str, f: &str| {
        s.fields
            .iter()
            .find(|x| x.reference == r && x.field == f)
            .unwrap_or_else(|| panic!("{r} {f} in {text}"))
    };
    // Off the default and without the field: added; the lowest of the MPNs
    assert_eq!(find("C2", "Voltage").op, Op::Add("25V".into()));
    // Stated 25V, the one MPN is 50V, which is the default: removed
    assert_eq!(find("C5", "Voltage").op, Op::Remove);
    // The worst code is the default: the field goes
    assert_eq!(find("C3", "Dielectric").op, Op::Remove);
    // 167mW stated, the lowest MPN is 63mW, the default: removed
    assert_eq!(find("R3", "Power").op, Op::Remove);
    // C4's MPN has no rated voltage: left alone and listed
    assert!(
        s.skipped
            .iter()
            .any(|k| k.reference == "C4" && k.field == "Voltage"),
        "{text}"
    );
    assert!(text.contains("C4 Voltage: left alone"), "{text}");
    assert!(!s.written.is_empty());
    // The BOM-12 display findings are gone on the same snapshot
    assert_eq!(display_findings(&dir), Vec::<String>::new());
    // A second run finds nothing to write
    let (again, text) = ops(&dir, true);
    assert!(again.fields.is_empty(), "{text}");
    assert!(again.written.is_empty());
    assert!(text.contains("Nothing to write"), "{text}");
    // C2's new field stands in the file, shown
    let sch = std::fs::read_to_string(dir.join("bom-12_mpn_vs_fields.kicad_sch")).unwrap();
    assert!(sch.contains("(property \"Voltage\" \"25V\""), "{sch}");
}
