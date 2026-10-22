//! `berc diff --pnp`: two PnP files (the old `ecad_compare` test), a board against the PnP files kicad-cli 10.0.7
//! exported from it (`tests/fixtures/pnp`: from the page origin and from the aux origin at 100, 50), tolerances,
//! JSON and the exit codes

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
