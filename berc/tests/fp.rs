//! `berc fp`: footprint review hashes of library files and board copies, through the binary

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_kicad/tests/fixtures/footprints")
}

fn berc(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(args)
        .env_remove("COMPLETE")
        .env("BERC_USAGE", "off")
        .output()
        .unwrap()
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn hash_of_a_library_file_and_of_its_board_copies() {
    let lib = fixtures().join("Test.pretty/Asym_SOT.kicad_mod");
    let board = fixtures().join("placed.kicad_pcb");
    let out = berc(&["fp", "hash", path(&lib)]);
    assert!(out.status.success(), "{out:?}");
    let hash = stdout(&out).trim().to_string();
    assert!(hash.starts_with("fp1:") && hash.len() == 36, "{hash}");
    // U1-U6 are Asym_SOT on both sides at right and odd angles
    for r in ["U1", "U2", "U3", "U4", "U5", "U6"] {
        let out = berc(&["fp", "hash", path(&board), r]);
        assert!(out.status.success(), "{r}: {out:?}");
        assert_eq!(stdout(&out).trim(), hash, "{r}");
    }
    let mixed = berc(&["fp", "hash", path(&board), "U7"]);
    assert_ne!(stdout(&mixed).trim(), hash);
}

#[test]
fn canonical_text_and_json() {
    let lib = fixtures().join("Test.pretty/Asym_SOT.kicad_mod");
    let out = berc(&["fp", "hash", path(&lib), "--canonical"]);
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("fp1:"));
    assert!(lines.contains(&"footprint mask 0.05"), "{text}");
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("pad 3 smd roundrect at 1.1,0.3 rot 0 size 0.7,1.3")),
        "{text}"
    );
    assert!(!text.contains("SilkS") && !text.contains("F.Fab"), "{text}");
    assert!(!text.contains('\x1b'), "colour on a pipe: {text:?}");

    let out = berc(&["fp", "hash", path(&lib), "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["lib_id"], "Asym_SOT");
    assert!(v["hash"].as_str().unwrap().starts_with("fp1:"));
    assert!(v.get("canonical").is_none());
    let out = berc(&["fp", "hash", path(&lib), "--format", "json", "--canonical"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["canonical"].as_array().unwrap().len() >= 5);
}

#[test]
fn errors_exit_2() {
    let board = fixtures().join("placed.kicad_pcb");
    let lib = fixtures().join("Test.pretty/Asym_SOT.kicad_mod");
    for (args, says) in [
        (vec![path(&board)], "name the footprint by its reference"),
        (vec![path(&board), "U99"], "no footprint U99"),
        (vec![path(&lib), "U1"], "leave the reference out"),
        (vec!["/nonexistent/x.kicad_mod"], "can't read"),
        (vec!["README.md"], "neither a library footprint"),
    ] {
        let mut a = vec!["fp", "hash"];
        a.extend(args);
        let out = berc(&a);
        assert_eq!(out.status.code(), Some(2), "{a:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(
            err.starts_with("berc: error: ") && err.contains(says),
            "{a:?}: {err}"
        );
    }
}

#[test]
fn references_complete_from_the_board() {
    let board = fixtures().join("placed.kicad_pcb");
    let out = Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(["--", "berc", "fp", "hash", path(&board), "U1"])
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "4")
        .env("_CLAP_COMPLETE_COMP_TYPE", "9")
        .env("_CLAP_COMPLETE_SPACE", "true")
        .env("BERC_USAGE", "off")
        .output()
        .unwrap();
    let refs = stdout(&out);
    let refs: Vec<&str> = refs.lines().collect();
    assert_eq!(refs, ["U1", "U10", "U11", "U12"]);
}
