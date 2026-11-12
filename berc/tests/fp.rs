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

/// A scratch directory of this test process, removed first
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("berc-fp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A symbol of the test library: one pin, a footprint, and a review hash when given
fn lib_symbol(name: &str, footprint: &str, review: Option<&str>) -> String {
    let review = review.map_or(String::new(), |r| {
        format!(r#"(property "Footprint_Review" "{r}" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))"#)
    });
    format!(
        r#"(symbol "{name}" (in_bom yes) (on_board yes)
      (property "Reference" "U" (at 0 0 0) (effects (font (size 1.27 1.27))))
      (property "Value" "{name}" (at 0 0 0) (effects (font (size 1.27 1.27))))
      (property "Footprint" "{footprint}" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
      {review}
      (symbol "{name}_1_1" (pin passive line (at 0 0 0) (length 1.27) (name "A" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))"#
    )
}

/// A KiCad project in `dir`: the KiCad 10 board of placed.kicad_pcb, a schematic whose symbols U1-U7 come from
/// library `Rev` (`(name, footprint, library hash, schematic copy's hash)` each), and with `table` a sym-lib-table
/// and the library file
fn project(dir: &Path, symbols: &[(&str, &str, Option<&str>, Option<&str>)], table: bool) {
    std::fs::copy(
        fixtures().join("placed.kicad_pcb"),
        dir.join("fpcheck.kicad_pcb"),
    )
    .unwrap();
    let root = "a0000000-0000-4000-8000-000000000000";
    let mut copies = String::new();
    let mut placed = String::new();
    let mut lib =
        String::from("(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n");
    for (i, (name, footprint, lib_hash, sch_hash)) in symbols.iter().enumerate() {
        let n = i + 1;
        copies += &lib_symbol(&format!("Rev:{name}"), footprint, *sch_hash);
        lib += &lib_symbol(name, footprint, *lib_hash);
        placed += &format!(
            r#"(symbol (lib_id "Rev:{name}") (at {x} 50 0) (unit 1) (in_bom yes) (on_board yes) (dnp no) (uuid "a0000000-0000-4000-8000-0000000001{n:02}")
    (property "Reference" "U{n}" (at {x} 45 0) (effects (font (size 1.27 1.27))))
    (property "Value" "{name}" (at {x} 55 0) (effects (font (size 1.27 1.27))))
    (property "Footprint" "{footprint}" (at {x} 50 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (pin "1" (uuid "a0000000-0000-4000-8000-0000000002{n:02}"))
    (instances (project "fpcheck" (path "/{root}" (reference "U{n}") (unit 1)))))
"#,
            x = 20 * n
        );
    }
    lib += ")\n";
    let sch = format!(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n  (uuid \"{root}\")\n  (paper \"A4\")\n  (lib_symbols {copies})\n  {placed}\n  (sheet_instances (path \"/\" (page \"1\"))))\n"
    );
    std::fs::write(dir.join("fpcheck.kicad_sch"), sch).unwrap();
    std::fs::write(dir.join("fpcheck.kicad_pro"), "{}\n").unwrap();
    if table {
        std::fs::write(dir.join("Rev.kicad_sym"), lib).unwrap();
        std::fs::write(
            dir.join("sym-lib-table"),
            "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_sym\")(options \"\")(descr \"\")))\n",
        )
        .unwrap();
    }
}

fn fp_check(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(["fp", "check", path(dir)])
        .args(args)
        .env("BERC_USAGE", "off")
        .env("NO_COLOR", "1")
        .env("KICAD_CONFIG_HOME", dir.join("no-kicad-config"))
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

fn library_hash(name: &str) -> String {
    let out = berc(&[
        "fp",
        "hash",
        path(&fixtures().join(format!("Test.pretty/{name}.kicad_mod"))),
    ]);
    stdout(&out).trim().to_string()
}

#[test]
fn check_every_status_through_the_library_table() {
    let (sot, mixed) = (library_hash("Asym_SOT"), library_hash("Mixed_Pads"));
    let other = "fp1:00000000000000000000000000000000";
    let dir = scratch("check");
    project(
        &dir,
        &[
            ("Ok", "Test:Asym_SOT", Some(&sot), Some(&sot)),
            ("Changed", "Test:Asym_SOT", Some(&mixed), Some(&mixed)),
            ("Unreviewed", "Test:Asym_SOT", None, None),
            ("NoFootprint", "", Some(&sot), Some(&sot)),
            ("Stale", "Test:Asym_SOT", Some(&sot), Some(other)),
            (
                "Future",
                "Test:Asym_SOT",
                Some("fp2:abcdef"),
                Some("fp2:abcdef"),
            ),
            (
                "Garbage",
                "Test:Mixed_Pads",
                Some("reviewed by R"),
                Some("reviewed by R"),
            ),
        ],
        true,
    );
    let out = fp_check(&dir, &["--format", "json"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["field"], "Footprint_Review");
    let fps = v["footprints"].as_array().unwrap();
    let get = |r: &str| {
        fps.iter()
            .find(|e| e["reference"] == r)
            .unwrap_or_else(|| panic!("{r}"))
    };
    let status = |r: &str| get(r)["status"].as_str().unwrap().to_string();
    assert_eq!(status("U1"), "ok");
    assert_eq!(get("U1")["source"], "library");
    assert_eq!(status("U2"), "changed");
    assert_eq!(get("U2")["hash"], sot.as_str());
    assert_eq!(status("U3"), "unreviewed");
    assert_eq!(status("U4"), "refused");
    assert_eq!(status("U5"), "ok");
    assert_eq!(get("U5")["copy_differs"], true);
    assert_eq!(get("U5")["schematic_hash"], other);
    assert_eq!(status("U6"), "other-version");
    assert_eq!(status("U7"), "malformed");
    for r in ["U8", "U9", "U10", "U11", "U12"] {
        assert_eq!(status(r), "no-symbol", "{r}");
    }
    let refs: Vec<&str> = fps
        .iter()
        .map(|e| e["reference"].as_str().unwrap())
        .collect();
    assert_eq!(
        refs,
        [
            "U1", "U2", "U3", "U4", "U5", "U6", "U7", "U8", "U9", "U10", "U11", "U12"
        ]
    );

    let out = fp_check(&dir, &[]);
    let text = stdout(&out);
    assert!(
        text.contains("unreviewed    U3  Asym_SOT    hash now fp1:"),
        "{text}"
    );
    assert!(
        text.contains("update the schematic from the library"),
        "{text}"
    );
    assert!(
        text.contains("12 footprints: 2 ok, 1 changed, 1 unreviewed, 1 refused, 5 no symbol, 1 other version, 1 malformed; 1 with a schematic copy older than the library"),
        "{text}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_falls_back_to_the_schematic_copy() {
    let sot = library_hash("Asym_SOT");
    let dir = scratch("copy");
    project(
        &dir,
        &[("Ok", "Test:Asym_SOT", Some("unused"), Some(&sot))],
        false,
    );
    let out = fp_check(&dir, &["--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let u1 = &v["footprints"][0];
    assert_eq!(
        (u1["reference"].as_str(), u1["status"].as_str()),
        (Some("U1"), Some("ok"))
    );
    assert_eq!(u1["source"], "schematic");
    assert_eq!(u1["copy_differs"], false);
    assert!(
        u1["library_note"]
            .as_str()
            .unwrap()
            .contains("in no sym-lib-table"),
        "{u1}"
    );
    // The other footprints have no symbol: not all ok
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&fp_check(&dir, &[]));
    assert!(
        text.contains(
            "read from the schematic's copy (1 footprints): library Rev is in no sym-lib-table"
        ),
        "{text}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_errors_exit_2() {
    let dir = scratch("empty");
    let out = fp_check(&dir, &[]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let lib = fixtures().join("Test.pretty/Asym_SOT.kicad_mod");
    let out = fp_check(&lib, &[]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn usage_is_recorded_per_command_and_flag() {
    let dir = scratch("usage");
    let file = dir.join("usage.tsv");
    let lib = fixtures().join("Test.pretty/Asym_SOT.kicad_mod");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_berc"))
            .args(args)
            .env("BERC_USAGE_FILE", &file)
            .env_remove("BERC_USAGE")
            .env_remove("COMPLETE")
            .output()
            .unwrap()
    };
    run(&["fp", "hash", path(&lib)]);
    run(&["fp", "hash", path(&lib), "--canonical"]);
    run(&["fp", "check", path(&dir)]);
    let lines = std::fs::read_to_string(&file).unwrap();
    let sigs: Vec<&str> = lines
        .lines()
        .map(|l| l.split('\t').nth(1).unwrap())
        .collect();
    assert_eq!(
        sigs,
        ["fp hash", "fp hash", "fp hash --canonical", "fp check"]
    );
    let summary = stdout(&run(&["usage"]));
    assert!(summary.contains("fp hash --canonical"), "{summary}");
    assert!(
        summary
            .lines()
            .any(|l| l.starts_with("textconv ") && l.ends_with(" 0")),
        "{summary}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
