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

/// Rewrites the text of one symbol's field in the schematic
fn state(dir: &Path, reference: &str, field: &str, old: &str, new: &str) {
    let file = dir.join("bom-12_mpn_vs_fields.kicad_sch");
    let text = std::fs::read_to_string(&file).unwrap();
    let symbol = text
        .find(&format!("(property \"Reference\" \"{reference}\""))
        .unwrap();
    let from = format!("(property \"{field}\" \"{old}\"");
    let at = symbol + text[symbol..].find(&from).unwrap();
    let to = format!("(property \"{field}\" \"{new}\"");
    let out = format!("{}{to}{}", &text[..at], &text[at + from.len()..]);
    std::fs::write(&file, out).unwrap();
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

/// The design with each `(from, to)` replaced in its snapshot
fn with_powers(name: &str, powers: &[(&str, &str)]) -> PathBuf {
    let dir = design(name);
    let lock = dir.join("parts.lock.toml");
    let mut text = std::fs::read_to_string(&lock).unwrap();
    for (from, to) in powers {
        assert!(text.contains(from), "{from}");
        text = text.replace(from, to);
    }
    std::fs::write(&lock, text).unwrap();
    dir
}

/// A coarse text rounds a rating further than the project allows: a finding that names the value to write, and
/// `fields sync` writes that value
#[test]
fn a_rounding_past_the_limit_is_written_again() {
    let sixteenth: &[(&str, &str)] = &[("value = 0.063,", "value = 0.0625,")];
    let half: &[(&str, &str)] = &[
        ("value = 0.063,", "value = 0.5,"),
        ("value = 0.2,", "value = 0.5,"),
    ];
    let volts: &[(&str, &str)] = &[("value = 50.0, unit = \"V\"", "value = 6.3, unit = \"V\"")];
    let cases = [
        // The rating to write is the board default `63mW`: the field goes
        (
            "coarse-sixteenth",
            sixteenth,
            "R3",
            "0.1W",
            "63mW",
            Op::Remove,
        ),
        (
            "coarse-half",
            half,
            "R3",
            "1W",
            "500mW",
            Op::Set("500mW".into()),
        ),
        (
            "coarse-volts",
            volts,
            "C5",
            "6V",
            "6.3V",
            Op::Set("6.3V".into()),
        ),
    ];
    for (name, lock, reference, stated, written, op) in cases {
        let dir = with_powers(name, lock);
        let (field, old) = if reference.starts_with('R') {
            ("Power", "167mW")
        } else {
            ("Voltage", "25V")
        };
        state(&dir, reference, field, old, stated);
        let want = format!("write {field} {written}");
        let findings = display_findings(&dir);
        assert!(
            findings
                .iter()
                .any(|m| m.starts_with(&format!("{reference}: ")) && m.ends_with(&want)),
            "{name}: {findings:?}"
        );
        let (s, text) = ops(&dir, true);
        let found = s
            .fields
            .iter()
            .find(|f| f.reference == reference && f.field == field)
            .unwrap_or_else(|| panic!("{name}: {text}"));
        assert_eq!(found.op, op, "{name}: {text}");
        assert_eq!(display_findings(&dir), Vec::<String>::new(), "{name}");
        let (again, text) = ops(&dir, true);
        assert!(again.fields.is_empty(), "{name}: {text}");
    }
}

/// `drawing.fields.rating_rounding_max` in the project file: at 0 the default `63mW` no longer states 62.5 mW, and
/// the exact value is written
#[test]
fn the_project_sets_how_far_a_rating_is_rounded() {
    let dir = with_powers("exact", &[("value = 0.063,", "value = 0.0625,")]);
    std::fs::write(
        dir.join("berc.toml"),
        "[drawing.fields]\nrating_rounding_max = 0\n",
    )
    .unwrap();
    let findings = display_findings(&dir);
    assert!(
        findings
            .iter()
            .any(|m| m.starts_with("R1: ") && m.ends_with("write Power 62.5mW")),
        "{findings:?}"
    );
    let (s, text) = ops(&dir, true);
    let r1 = s
        .fields
        .iter()
        .find(|f| f.reference == "R1" && f.field == "Power")
        .unwrap_or_else(|| panic!("{text}"));
    assert_eq!(r1.op, Op::Add("62.5mW".into()), "{text}");
    assert_eq!(display_findings(&dir), Vec::<String>::new());
    let (again, text) = ops(&dir, true);
    assert!(again.fields.is_empty(), "{text}");
}

/// 1/16 W parts on a board whose default is `63mW`: the default states 62.5 mW, so the parts at it have no
/// finding and no field of their own
#[test]
fn a_default_of_63mw_states_one_sixteenth_watt() {
    let dir = with_powers("sixteenth", &[("value = 0.063,", "value = 0.0625,")]);
    let power: Vec<String> = display_findings(&dir)
        .into_iter()
        .filter(|m| m.contains("write Power"))
        .collect();
    // R3 alone: 167mW stated
    assert_eq!(power.len(), 1, "{power:?}");
    assert!(power[0].ends_with("write Power 63mW"), "{power:?}");
    let (s, text) = ops(&dir, true);
    let resistors: Vec<(&str, &Op)> = s
        .fields
        .iter()
        .filter(|f| f.field == "Power")
        .map(|f| (f.reference.as_str(), &f.op))
        .collect();
    assert_eq!(resistors, [("R3", &Op::Remove)], "{text}");
    assert_eq!(display_findings(&dir), Vec::<String>::new());
    let (again, text) = ops(&dir, true);
    assert!(again.fields.is_empty(), "{text}");
}

/// A rating off the default is written in whole mW, which states it: a second run finds nothing
#[test]
fn a_rounded_power_is_written_once() {
    let dir = with_powers("rounded", &[("value = 0.063,", "value = 0.03125,")]);
    let (s, text) = ops(&dir, true);
    for r in ["R1", "R2", "R3", "R4"] {
        let found = s
            .fields
            .iter()
            .find(|f| f.reference == r && f.field == "Power")
            .unwrap_or_else(|| panic!("{r} in {text}"));
        let want = if r == "R3" {
            Op::Set("31mW".into())
        } else {
            Op::Add("31mW".into())
        };
        assert_eq!(found.op, want, "{r}: {text}");
    }
    assert_eq!(display_findings(&dir), Vec::<String>::new());
    let (again, text) = ops(&dir, true);
    assert!(again.fields.is_empty(), "{text}");
    assert!(again.written.is_empty());
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
