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
    assert!(hash.starts_with("fp1:") && hash.len() == 20, "{hash}");
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
/// and the library file. Every Mixed_Pads copy but the last in the file (U11) is marked board-only: U8, U9, U10 and
/// U12 without a symbol, U7 with one.
fn project(dir: &Path, symbols: &[(&str, &str, Option<&str>, Option<&str>)], table: bool) {
    let board = std::fs::read_to_string(fixtures().join("placed.kicad_pcb")).unwrap();
    let parts: Vec<&str> = board.split("(attr through_hole)").collect();
    let last = parts.len() - 2;
    let mut marked = String::from(parts[0]);
    for (i, p) in parts[1..].iter().enumerate() {
        marked += if i == last {
            "(attr through_hole)"
        } else {
            "(attr through_hole board_only)"
        };
        marked += p;
    }
    std::fs::write(dir.join("fpcheck.kicad_pcb"), marked).unwrap();
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
    let other = "fp1:0000000000000000";
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
    for r in ["U8", "U9", "U10", "U12"] {
        assert_eq!(status(r), "board-only", "{r}");
    }
    assert_eq!(status("U11"), "no-symbol");
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
        text.contains("12 footprints: 2 ok, 1 changed, 1 unreviewed, 1 refused, 1 no symbol, 1 other version, 1 malformed, 4 board-only; 1 with a schematic copy older than the library"),
        "{text}"
    );
    // Board-only footprints come last, under their heading, every one of them
    let lines: Vec<&str> = text.lines().collect();
    let h = lines
        .iter()
        .position(|l| {
            l.starts_with("board-only footprints without a symbol, nothing to review (4):")
        })
        .unwrap_or_else(|| panic!("{text}"));
    let below: Vec<&str> = lines[h + 1..h + 5]
        .iter()
        .map(|l| l.split_whitespace().nth(1).unwrap())
        .collect();
    assert_eq!(below, ["U8", "U9", "U10", "U12"], "{text}");
    assert!(lines[..h].iter().any(|l| l.contains(" U11 ")), "{text}");
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
fn check_reads_a_symbol_directory_library() {
    // KiCad 9+ directory symbol library (`.kicad_symdir`): one `.kicad_sym` per symbol, "Ok" extending "Base" in
    // the directory's other file
    let sot = library_hash("Asym_SOT");
    let dir = scratch("symdir");
    project(
        &dir,
        &[
            ("Base", "Test:Asym_SOT", Some(&sot), Some(&sot)),
            ("Ok", "Test:Asym_SOT", None, Some(&sot)),
        ],
        false,
    );
    let symdir = dir.join("Rev.kicad_symdir");
    std::fs::create_dir(&symdir).unwrap();
    let wrap = |s: String| {
        format!("(kicad_symbol_lib (version 20231120) (generator \"kicad_symbol_editor\")\n{s})\n")
    };
    std::fs::write(
        symdir.join("Base.kicad_sym"),
        wrap(lib_symbol("Base", "Test:Asym_SOT", Some(&sot))),
    )
    .unwrap();
    std::fs::write(
        symdir.join("Ok.kicad_sym"),
        wrap(r#"(symbol "Ok" (extends "Base")
      (property "Footprint" "Test:Asym_SOT" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes))))"#
            .to_string()),
    )
    .unwrap();
    std::fs::write(
        dir.join("sym-lib-table"),
        "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_symdir\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    let out = fp_check(&dir, &["--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let fps = v["footprints"].as_array().unwrap();
    let get = |r: &str| fps.iter().find(|e| e["reference"] == r).unwrap();
    // "Ok" sets no Footprint_Review of its own: the hash comes from "Base", in the directory's other file
    assert_eq!(get("U2")["status"], "ok", "{v}");
    assert_eq!(get("U2")["source"], "library", "{v}");
    assert_eq!(get("U2")["library_hash"], sot.as_str(), "{v}");
    assert_eq!(get("U2")["copy_differs"], false, "{v}");
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
    run(&["fp", "check", "--lib", "X", "--footprints", path(&dir)]);
    let lines = std::fs::read_to_string(&file).unwrap();
    let sigs: Vec<&str> = lines
        .lines()
        .map(|l| l.split('\t').nth(1).unwrap())
        .collect();
    assert_eq!(
        sigs,
        [
            "fp hash",
            "fp hash",
            "fp hash --canonical",
            "fp check",
            "fp check",
            "fp check --lib",
            "fp check --footprints"
        ]
    );
    // The app writes its steps into the same record, counted in a column of their own
    std::fs::write(&file, lines + "0\tapp fp-review accept\tapp\n").unwrap();
    let summary = stdout(&run(&["usage"]));
    assert!(summary.contains("fp hash --canonical"), "{summary}");
    assert!(summary.contains("fp check --lib"), "{summary}");
    assert!(
        summary
            .lines()
            .any(|l| l.starts_with("app fp-review accept ") && l.ends_with("  0       0     1")),
        "{summary}"
    );
    assert!(
        summary
            .lines()
            .any(|l| l.starts_with("textconv ") && l.ends_with(" 0")),
        "{summary}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A one-symbol `.kicad_sym` file as KiCad writes into a `.kicad_symdir`
fn symdir_file(symbol: &str) -> String {
    format!("(kicad_symbol_lib (version 20241209) (generator \"kicad_symbol_editor\")\n{symbol})\n")
}

/// A symbol library to check: a `.kicad_symdir` with a symbol of each status (one derived, one whose file name is
/// not its name), a flat `.kicad_sym` beside it, the footprints from the fixtures' `Test.pretty`
fn symbol_library(dir: &Path) -> (String, String) {
    let (sot, mixed) = (library_hash("Asym_SOT"), library_hash("Mixed_Pads"));
    let lib = dir.join("symbols/Rev.kicad_symdir");
    std::fs::create_dir_all(&lib).unwrap();
    let write = |file: &str, symbol: String| {
        std::fs::write(lib.join(format!("{file}.kicad_sym")), symdir_file(&symbol)).unwrap()
    };
    write("Ok", lib_symbol("Ok", "Test:Asym_SOT", Some(&sot)));
    write(
        "Changed",
        lib_symbol("Changed", "Test:Asym_SOT", Some(&mixed)),
    );
    write(
        "Unreviewed",
        lib_symbol("Unreviewed", "Test:Mixed_Pads", None),
    );
    write("Missing", lib_symbol("Missing", "Test:Nope", Some(&sot)));
    write("Refused", lib_symbol("Refused", "", Some(&sot)));
    write("Generic", lib_symbol("Generic", "", None));
    write(
        "Future",
        lib_symbol("Future", "Test:Asym_SOT", Some("fp2:ab")),
    );
    write(
        "Child",
        r#"(symbol "Child" (extends "Ok") (property "Value" "Child" (at 0 0 0) (effects (font (size 1.27 1.27)))))"#
            .into(),
    );
    write(
        "Odd_Name",
        lib_symbol("Odd/Name", "Test:Asym_SOT", Some(&sot)),
    );
    let flat = format!(
        "(kicad_symbol_lib (version 20241209)\n{}\n{})\n",
        lib_symbol("A", "Test:Asym_SOT", Some(&sot)),
        lib_symbol("B", "", None)
    );
    std::fs::write(dir.join("symbols/Flat.kicad_sym"), flat).unwrap();
    (sot, mixed)
}

#[test]
fn check_a_symbol_library() {
    let dir = scratch("lib");
    let (sot, mixed) = symbol_library(&dir);
    let lib = dir.join("symbols/Rev.kicad_symdir");
    let out = fp_check(
        &dir,
        &[
            "--lib",
            path(&lib),
            "--footprints",
            path(&fixtures()),
            "--format",
            "json",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["libraries"][0]["nickname"], "Rev");
    assert_eq!(v["libraries"][0]["symbols"], 9);
    let syms = v["symbols"].as_array().unwrap();
    let get = |s: &str| {
        syms.iter()
            .find(|e| e["symbol"] == format!("Rev:{s}"))
            .unwrap_or_else(|| panic!("{s}: {syms:?}"))
    };
    let status = |s: &str| get(s)["status"].as_str().unwrap().to_string();
    assert_eq!(status("Ok"), "ok");
    assert_eq!(status("Child"), "ok", "the parent's footprint and hash");
    assert_eq!(status("Odd/Name"), "ok");
    assert_eq!(status("Changed"), "changed");
    assert_eq!(get("Changed")["hash"], sot.as_str());
    assert_eq!(get("Changed")["stored"], mixed.as_str());
    assert_eq!(status("Unreviewed"), "unreviewed");
    assert_eq!(get("Unreviewed")["hash"], mixed.as_str());
    assert_eq!(status("Missing"), "missing");
    assert!(
        get("Missing")["note"]
            .as_str()
            .unwrap()
            .contains("no footprint Nope"),
        "{:?}",
        get("Missing")
    );
    assert_eq!(status("Refused"), "refused");
    assert_eq!(status("Generic"), "no-footprint");
    assert_eq!(status("Future"), "other-version");
    assert!(
        get("Ok")["footprint_file"]
            .as_str()
            .unwrap()
            .ends_with("Test.pretty/Asym_SOT.kicad_mod")
    );

    // Text: one line per symbol, the symbols without a footprint last under their heading, a summary
    let out = fp_check(
        &dir,
        &["--lib", path(&lib), "--footprints", path(&fixtures())],
    );
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("unreviewed    Rev:Unreviewed") && l.contains("hash now fp1:")),
        "{text}"
    );
    let heading = lines
        .iter()
        .position(|l| l.starts_with("symbols without a footprint"))
        .unwrap();
    assert!(
        lines[heading + 1].starts_with("no footprint  Rev:Generic"),
        "{text}"
    );
    assert!(lines.iter().all(|l| !l.ends_with(' ')), "{text:?}");
    assert!(
        lines.last().unwrap().ends_with(
            "9 symbols: 3 ok, 1 changed, 1 unreviewed, 1 missing, 1 refused, 1 other version, 1 no footprint"
        ),
        "{text}"
    );

    // Without --footprints the footprints come through the project's fp-lib-table
    std::fs::write(
        dir.join("fp-lib-table"),
        format!(
            "(fp_lib_table (version 7)\n  (lib (name \"Test\")(type \"KiCad\")(uri \"{}\")(options \"\")(descr \"\")))\n",
            fixtures().join("Test.pretty").display()
        ),
    )
    .unwrap();
    let out = fp_check(&dir, &["--lib", path(&lib), "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let ok = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["status"] == "ok")
        .count();
    assert_eq!(ok, 3, "{v}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_libraries_by_folder_and_nickname() {
    let dir = scratch("libs");
    symbol_library(&dir);
    // A folder: every library in it, in name order
    let out = fp_check(
        &dir,
        &[
            "--lib",
            path(&dir.join("symbols")),
            "--footprints",
            path(&fixtures().join("Test.pretty")),
            "--format",
            "json",
        ],
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let nicks: Vec<&str> = v["libraries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["nickname"].as_str().unwrap())
        .collect();
    assert_eq!(nicks, ["Flat", "Rev"]);
    assert_eq!(v["symbols"][0]["symbol"], "Flat:A");
    assert_eq!(v["symbols"][0]["status"], "ok");
    assert_eq!(v["symbols"][1]["status"], "no-footprint");

    // A nickname through the project's sym-lib-table, which names the library its own way; all ok exits 0
    std::fs::write(
        dir.join("sym-lib-table"),
        "(sym_lib_table (version 7)\n  (lib (name \"MyFlat\")(type \"KiCad\")(uri \"${KIPRJMOD}/symbols/Flat.kicad_sym\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    let out = fp_check(
        &dir,
        &["--lib", "MyFlat", "--footprints", path(&fixtures())],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = stdout(&out);
    assert!(text.starts_with("ok            MyFlat:A"), "{text}");

    // What is no library exits 2 and says why
    for (args, says) in [
        (vec!["--lib", "Nope"], "library Nope is in no sym-lib-table"),
        (
            vec!["--lib", path(&fixtures())],
            "no symbol library (.kicad_sym, .kicad_symdir) in it",
        ),
        (
            vec!["--lib", "MyFlat", "--footprints", path(&dir)],
            "no footprint library (.pretty)",
        ),
    ] {
        let out = fp_check(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(err.contains(says), "{args:?}: {err}");
    }
    let out = fp_check(&dir, &["--footprints", path(&fixtures())]);
    assert_eq!(out.status.code(), Some(2), "--footprints needs --lib");
    std::fs::remove_dir_all(&dir).unwrap();
}
