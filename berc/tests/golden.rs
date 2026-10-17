//! Golden output: the fixtures in `tests/fixtures` rendered as they are expected to look. After an intended change
//! of the output format, regenerate with `BLESS=1 cargo test -p berc --test golden` and review the diff.

use berc::{Options, textconv};
use std::path::Path;

fn check(fixture: &str, golden: &str, opts: Options) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input = std::fs::read_to_string(dir.join(fixture)).unwrap();
    let out = textconv(&input, opts);
    let golden_path = dir.join(golden);
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&golden_path, &out).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden_path).unwrap_or_default();
    assert_eq!(
        out, expected,
        "{fixture} doesn't match {golden}; BLESS=1 regenerates it"
    );
}

#[test]
fn schematic() {
    check("small.kicad_sch", "small.kicad_sch.txt", Options::default());
}

#[test]
fn schematic_with_geometry() {
    check(
        "small.kicad_sch",
        "small.kicad_sch.geometry.txt",
        Options {
            with_geometry: true,
            ..Options::default()
        },
    );
}

#[test]
fn board() {
    check("small.kicad_pcb", "small.kicad_pcb.txt", Options::default());
}

/// Reordering items and changing UUIDs must not change the output
#[test]
fn stable_under_reordering() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let input = std::fs::read_to_string(dir.join("small.kicad_sch")).unwrap();
    let parsed = berc::sexpr::parse(&input).unwrap();
    let root = &parsed[0];
    // Rebuild the file with the top-level items reversed and every uuid renamed
    let mut items: Vec<String> = root
        .args()
        .iter()
        .map(|i| {
            berc::common::compact(
                i,
                Options {
                    with_geometry: true,
                    ..Options::default()
                },
            )
        })
        .collect();
    items.reverse();
    let shuffled = format!("(kicad_sch {})", items.join("\n"));
    let shuffled = shuffled.replace("(uuid \"", "(uuid \"x");
    assert_eq!(
        textconv(&shuffled, Options::default()),
        textconv(&input, Options::default())
    );
}

#[test]
fn project() {
    check(
        "cannify_micro.old.kicad_pro",
        "cannify_micro.old.kicad_pro.txt",
        Options::default(),
    );
    check(
        "cannify_micro.new.kicad_pro",
        "cannify_micro.new.kicad_pro.txt",
        Options::default(),
    );
}

/// Real fixture (hw/b129_cannify_micro): opening the project in a newer KiCad added ~70 lines of new JSON, all
/// but a handful of it KiCad 10.0.6's own defaults (`pro_defaults`). textconv on the two sides should differ by
/// only a few lines: the net classes schema version bump, the new sheet's filename, and the folded-count
/// summary lines moving - not a line per new-but-default setting.
#[test]
fn project_upgrade_is_a_small_diff() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let old = textconv(
        &std::fs::read_to_string(dir.join("cannify_micro.old.kicad_pro")).unwrap(),
        Options::default(),
    );
    let new = textconv(
        &std::fs::read_to_string(dir.join("cannify_micro.new.kicad_pro")).unwrap(),
        Options::default(),
    );
    let changed = symmetric_line_diff(&old, &new);
    assert!(
        changed <= 15,
        "{changed} lines differ between old and new textconv output, expected a handful at most"
    );
}

/// Lines present on only one side, counted with repeats (a cheap stand-in for a real diff's changed-line count)
fn symmetric_line_diff(a: &str, b: &str) -> usize {
    let mut counts: std::collections::HashMap<&str, i32> = std::collections::HashMap::new();
    for l in a.lines() {
        *counts.entry(l).or_default() += 1;
    }
    for l in b.lines() {
        *counts.entry(l).or_default() -= 1;
    }
    counts.values().map(|n| n.unsigned_abs() as usize).sum()
}

#[test]
fn not_kicad_passes_through() {
    let out = textconv(
        "EESchema Schematic File Version 4\n(oops",
        Options::default(),
    );
    assert!(
        out.starts_with("# berc: not a KiCad s-expression file"),
        "{out}"
    );
    assert!(
        out.ends_with("EESchema Schematic File Version 4\n(oops"),
        "{out}"
    );
}
