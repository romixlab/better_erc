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
