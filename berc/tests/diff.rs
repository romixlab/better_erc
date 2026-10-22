//! `berc diff --pnp`: two PnP files (the old `ecad_compare` test), a board against the PnP files kicad-cli 10.0.7
//! exported from it (`tests/fixtures/pnp`: from the page origin and from the aux origin at 100, 50), tolerances,
//! JSON and the exit codes. `berc diff --nets`: the synthetic Smart PDF of `erc_import` against a WireList of the
//! same board written by hand (`tests/fixtures/smart_pdf_wirelist.net`), as Altium would name its nets

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p: PathBuf = if name.starts_with("pnp_") {
        dir.join("../erc_import/tests/fixtures").join(name)
    } else {
        dir.join("tests/fixtures/pnp").join(name)
    };
    p.to_str().unwrap().to_string()
}

fn berc(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

fn diff(a: &str, b: &str, more: &[&str]) -> Output {
    let (a, b) = (fixture(a), fixture(b));
    let mut args = vec!["diff", "--pnp", "--format", "json", a.as_str(), b.as_str()];
    args.extend_from_slice(more);
    berc(&args)
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn refs(v: &Value, list: &str) -> Vec<String> {
    v["diff"][list]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            r.as_str()
                .map_or_else(|| r["reference"].to_string(), String::from)
        })
        .collect()
}

/// `ecad_compare::pnp::compare_positions_works`
#[test]
fn two_pnp_files() {
    let out = diff("pnp_allegro.csv", "pnp_altium_no_units.csv", &[]);
    assert_eq!(out.status.code(), Some(1));
    let v = json(&out);
    assert_eq!(refs(&v, "changed"), ["\"R1\""]);
    assert_eq!(refs(&v, "removed"), ["C1"]);
    assert_eq!(refs(&v, "added"), ["J1"]);
    assert_eq!(v["a"]["footprints"], 2);
    assert_eq!(v["origin"], Value::Null);
}

#[test]
fn board_against_its_pnp_export() {
    for origin in ["page", "aux"] {
        let csv = format!("board.{origin}.csv");
        let out = diff("board.kicad_pcb", &csv, &["--origin", origin]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{origin}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        let v = json(&out);
        assert_eq!(v["origin"], origin);
        assert_eq!(v["a"]["board_file"], true);
        assert_eq!(
            v["a"]["footprints"], 3,
            "the test point and the net tie are not in position files"
        );
        // Either side can be the board
        let out = diff(&csv, "board.kicad_pcb", &["--origin", origin]);
        assert_eq!(out.status.code(), Some(0), "{origin}, board second");
    }
    let out = diff("board.kicad_pcb", "board.aux.csv", &[]);
    assert_eq!(
        refs(&json(&out), "changed").len(),
        3,
        "the page origin against an aux export"
    );
}

#[test]
fn tolerances() {
    let out = diff("board.kicad_pcb", "board.moved.csv", &["--origin", "aux"]);
    assert_eq!(out.status.code(), Some(1));
    let v = json(&out);
    let r1 = &v["diff"]["changed"][0];
    assert_eq!(r1["reference"], "R1");
    assert_eq!(r1["changes"][0]["attribute"], "position");
    assert_eq!(r1["changes"][0]["b"]["x"], 10_500_000);
    let out = diff(
        "board.kicad_pcb",
        "board.moved.csv",
        &["--origin", "aux", "--tolerance", "0.5"],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "0.5 mm is within --tolerance 0.5"
    );
}

#[test]
fn text_and_errors() {
    let (a, b) = (fixture("board.aux.csv"), fixture("board.moved.csv"));
    let out = berc(&["diff", "--pnp", &a, &b]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("changed R1: position 10, -50 -> 10.5, -50"),
        "{text}"
    );
    assert!(text.ends_with("1 changed, 0 removed, 0 added\n"), "{text}");
    let sch = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test_schematics/sources/i2c-1_no_pull_ups.kicad_sch");
    let out = berc(&["diff", "--pnp", sch.to_str().unwrap(), &b]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no placements"));
    let out = berc(&["diff", &a, &b]);
    assert_eq!(out.status.code(), Some(2), "--pnp is the only mode so far");
}

fn smart_pdf() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_import/tests/fixtures/smart_pdf.pdf");
    p.to_str().unwrap().to_string()
}

fn wirelist() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/smart_pdf_wirelist.net")
}

#[test]
fn nets_of_a_pdf_and_a_wirelist() {
    let w = wirelist();
    let out = berc(&["diff", "--nets", &smart_pdf(), w.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "names differ");
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("(altium-pdf: 11 nets, 22 pins)"), "{text}");
    assert!(text.contains("(altium: 11 nets, 22 pins)"), "{text}");
    assert!(
        text.contains("renamed /U_Top/U_MCU/EN -> NetR4_1"),
        "a net named per sheet in the PDF: {text}"
    );
    assert!(
        text.ends_with("8 same, 3 renamed, 0 class changes, 0 removed, 0 added\n"),
        "{text}"
    );

    let same = berc(&["diff", "--nets", &smart_pdf(), &smart_pdf()]);
    assert_eq!(same.status.code(), Some(0));
    assert!(String::from_utf8(same.stdout).unwrap().ends_with("same nets (11)\n"));
}

#[test]
fn nets_that_differ() {
    // J1.4 moved from EN_uC onto GND
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("diff_nets");
    std::fs::create_dir_all(&dir).unwrap();
    let text = std::fs::read_to_string(wirelist()).unwrap();
    let moved = text
        .replace("[00004] EN_uC\r\n        J1         4 ", "[00004] EN_uC\r\n        R9         4 ")
        .replace(
            "[00001] GND\r\n",
            "[00001] GND\r\n        J1         4       4              PASSIVE     HEADER4\r\n",
        );
    assert_ne!(moved, text);
    let b = dir.join("moved.net");
    std::fs::write(&b, moved).unwrap();
    let out = berc(&["diff", "--nets", "--format", "json", &smart_pdf(), b.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let v = json(&out);
    let names = |list: &str| -> Vec<String> {
        v["diff"][list]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e[0].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(names("only_a"), ["EN_µC", "GND"]);
    assert_eq!(names("only_b"), ["EN_uC", "GND"]);
    assert_eq!(v["diff"]["only_b"][1][1].as_array().unwrap().len(), 6);

    let pnp = fixture("pnp_kicad.csv");
    let out = berc(&["diff", "--nets", &smart_pdf(), &pnp]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no nets"));
    let out = berc(&["diff", &smart_pdf(), &smart_pdf()]);
    assert_eq!(out.status.code(), Some(2), "--pnp or --nets");
}
