//! `berc fp`: review hashes of library files and board copies, and footprints judged on (hash, MPN) pairs against
//! the reviews of a parts snapshot, through the binary

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

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("berc-fp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// berc in `dir`'s KiCad world: no global library table, no colour, no usage record
fn berc_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(args)
        .env("BERC_USAGE", "off")
        .env("NO_COLOR", "1")
        .env("KICAD_CONFIG_HOME", dir.join("no-kicad-config"))
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

fn fp_check(dir: &Path, args: &[&str]) -> Output {
    let mut a = vec!["fp", "check", path(dir)];
    a.extend(args);
    berc_in(dir, &a)
}

fn fp_todo(dir: &Path, args: &[&str]) -> Output {
    let mut a = vec!["fp", "todo", path(dir)];
    a.extend(args);
    berc_in(dir, &a)
}

fn fp_accept(dir: &Path, args: &[&str]) -> Output {
    let parts = dir.join(PARTS);
    let mut a = vec!["fp", "accept", path(dir), "--from", path(&parts)];
    a.extend(args);
    berc_in(dir, &a)
}

fn library_hash(name: &str) -> String {
    let out = berc(&[
        "fp",
        "hash",
        path(&fixtures().join(format!("Test.pretty/{name}.kicad_mod"))),
    ]);
    stdout(&out).trim().to_string()
}

/// A symbol of the test library: one pin, a footprint, its MPNs
fn lib_symbol(name: &str, footprint: &str, mpns: &[&str]) -> String {
    let mpns: String = mpns
        .iter()
        .enumerate()
        .map(|(i, m)| {
            format!(
                r#"(property "Mpn{}" "{m}" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))"#,
                i + 1
            )
        })
        .collect();
    format!(
        r#"(symbol "{name}" (in_bom yes) (on_board yes)
      (property "Reference" "U" (at 0 0 0) (effects (font (size 1.27 1.27))))
      (property "Value" "{name}" (at 0 0 0) (effects (font (size 1.27 1.27))))
      (property "Footprint" "{footprint}" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
      (property "Datasheet" "docs/{name}.pdf" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
      {mpns}
      (symbol "{name}_1_1" (pin passive line (at 0 0 0) (length 1.27) (name "A" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))"#
    )
}

/// One symbol placed: its name in library `Rev`, its footprint, its instance's MPNs
type Sym<'a> = (&'a str, &'a str, &'a [&'a str]);

/// The symbols U1-U7 of [`SYMBOLS`]: the board's U1-U6 are Asym_SOT, U7 is Mixed_Pads
const SYMBOLS: [Sym; 7] = [
    ("Ok", "Test:Asym_SOT", &["A-1"]),
    ("Fam", "Test:Asym_SOT", &["RC0402FR-0710KL"]),
    ("Changed", "Test:Asym_SOT", &["B-1"]),
    ("Unc", "Test:Asym_SOT", &["RC0603FR-0710KL"]),
    ("Partial", "Test:Asym_SOT", &["A-1", "RC0603FR-071KL"]),
    ("Future", "Test:Asym_SOT", &["E-1"]),
    ("NoMpn", "Test:Mixed_Pads", &[]),
];

/// The part source of the tests
const PARTS: &str = "parts.toml";

/// A KiCad project in `dir`: the KiCad 10 board of placed.kicad_pcb, a schematic whose symbols U1-U7 come from
/// library `Rev` (a sym-lib-table and the library file, each symbol with its MPNs), and the part source
/// [`PARTS`]. Every Mixed_Pads copy but the last in the file (U11) is marked board-only: U8, U9, U10 and U12
/// without a symbol, U7 with one.
fn project(dir: &Path, symbols: &[Sym]) {
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
    for (i, (name, footprint, mpns)) in symbols.iter().enumerate() {
        let n = i + 1;
        if !symbols[..i].iter().any(|s| s.0 == *name) {
            copies += &lib_symbol(&format!("Rev:{name}"), footprint, mpns);
            lib += &lib_symbol(name, footprint, mpns);
        }
        let mpn_props: String = mpns
            .iter()
            .enumerate()
            .map(|(k, m)| {
                format!(
                    "    (property \"Mpn{}\" \"{m}\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))\n",
                    k + 1
                )
            })
            .collect();
        placed += &format!(
            r#"(symbol (lib_id "Rev:{name}") (at {x} 50 0) (unit 1) (in_bom yes) (on_board yes) (dnp no) (uuid "a0000000-0000-4000-8000-0000000001{n:02}")
    (property "Reference" "U{n}" (at {x} 45 0) (effects (font (size 1.27 1.27))))
    (property "Value" "{name}" (at {x} 55 0) (effects (font (size 1.27 1.27))))
    (property "Footprint" "{footprint}" (at {x} 50 0) (effects (font (size 1.27 1.27)) (hide yes)))
    (property "Datasheet" "docs/{name}.pdf" (at {x} 50 0) (effects (font (size 1.27 1.27)) (hide yes)))
{mpn_props}    (pin "1" (uuid "a0000000-0000-4000-8000-0000000002{n:02}"))
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
    std::fs::write(dir.join("Rev.kicad_sym"), lib).unwrap();
    std::fs::write(
        dir.join("sym-lib-table"),
        "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_sym\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    std::fs::write(dir.join(PARTS), parts_source(&library_hash("Asym_SOT"))).unwrap();
}

/// The part source: A-1 reviewed alone and RC 0402 as a family for the Asym_SOT hash `sot`, B-1 reviewed for
/// another hash, E-1 with another version of the hash; C-1 and the RC 0603 parts never
fn parts_source(sot: &str) -> String {
    let rc = |mpn: &str, case: &str| {
        format!(
            "\n[[part]]\nmpn = \"{mpn}\"\nmatch = \"found\"\ndecode = {{ outcome = \"decoded\", scheme = \"yageo.rc\", segments = {{ case_eia = \"{case}\", resistance = \"10kΩ\" }} }}\n"
        )
    };
    let plain = |mpn: &str| format!("\n[[part]]\nmpn = \"{mpn}\"\nmatch = \"found\"\n");
    let review = |id: u32, hash: &str, covers: &str| {
        format!(
            "\n[[review]]\nid = {id}\nhash = \"{hash}\"\nfootprint = \"Test:Asym_SOT\"\ndoc = \"DS{id}\"\npage = 3\nscale = 0.35\nat = {{ x = 10.0, y = 20.0, rotation = 90.0, mirror = false }}\nreviewer = \"Jane Doe\"\nreviewed_on = \"2026-11-20\"\ncovers = [{covers}]\n"
        )
    };
    let mut s = String::from("version = 1\nsource = \"hand\"\n");
    for m in ["A-1", "B-1", "C-1", "E-1"] {
        s += &plain(m);
    }
    s += &rc("RC0402FR-0710KL", "0402");
    s += &rc("RC0603FR-0710KL", "0603");
    s += &rc("RC0603FR-071KL", "0603");
    s += &review(1, sot, "{ kind = \"mpn\", mpn = \"A-1\" }");
    s += &review(
        2,
        sot,
        "{ kind = \"family\", scheme = \"yageo.rc\", segments = { case_eia = \"0402\" } }",
    );
    s += &review(
        3,
        "fp1:0000000000000000",
        "{ kind = \"mpn\", mpn = \"B-1\" }",
    );
    s += &review(
        4,
        "fp2:00000000000000000000",
        "{ kind = \"mpn\", mpn = \"E-1\" }",
    );
    s
}

/// `berc parts refresh` from [`PARTS`]
fn refresh(dir: &Path) {
    let out = berc_in(
        dir,
        &[
            "parts",
            "refresh",
            path(dir),
            "--from",
            path(&dir.join(PARTS)),
        ],
    );
    assert!(out.status.success(), "{out:?}");
}

fn json(out: &Output) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {out:?}"))
}

/// The entry of `reference` in a `berc fp check --format json`
fn entry<'a>(v: &'a serde_json::Value, reference: &str) -> &'a serde_json::Value {
    v["footprints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["reference"] == reference)
        .unwrap_or_else(|| panic!("{reference}"))
}

#[test]
fn check_every_status_from_the_lock() {
    let dir = scratch("check");
    project(&dir, &SYMBOLS);
    let out = fp_check(&dir, &[]);
    assert_eq!(out.status.code(), Some(2), "no lock yet: {out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("berc parts refresh"),
        "{out:?}"
    );
    refresh(&dir);
    let out = fp_check(
        &dir,
        &["--format", "json", "--footprints", path(&fixtures())],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let v = json(&out);
    let status = |r: &str| entry(&v, r)["status"].as_str().unwrap().to_owned();
    assert_eq!(status("U1"), "ok", "A-1 reviewed alone");
    assert_eq!(status("U2"), "ok", "RC 0402 reviewed as a family");
    assert_eq!(
        entry(&v, "U2")["pairs"][0]["covered_by"],
        serde_json::json!([2])
    );
    assert_eq!(status("U3"), "changed");
    assert_eq!(
        entry(&v, "U3")["pairs"][0]["other"][0]["hash"],
        "fp1:0000000000000000"
    );
    assert_eq!(status("U4"), "uncovered");
    assert_eq!(status("U5"), "partial");
    assert_eq!(status("U6"), "other-version");
    assert_eq!(status("U7"), "no-mpn");
    for r in ["U8", "U9", "U10", "U12"] {
        assert_eq!(status(r), "board-only", "{r}");
    }
    assert_eq!(status("U11"), "no-symbol");
    assert_eq!(entry(&v, "U1")["board_differs"], false);

    let text = stdout(&fp_check(&dir, &["--footprints", path(&fixtures())]));
    assert!(
        text.contains("12 footprints: 2 ok, 1 changed, 1 other version, 1 partial, 1 uncovered, 1 no MPN, 1 no symbol, 4 board-only"),
        "{text}"
    );
    assert!(
        text.contains("changed B-1 reviewed as #3 fp1:0000000000000000"),
        "{text}"
    );
    assert!(
        text.contains("uncovered RC0603FR-071KL (family yageo.rc case_eia=0603)"),
        "{text}"
    );
    assert!(
        !text.contains("covered A-1"),
        "covered pairs only with --details: {text}"
    );
    let details = stdout(&fp_check(
        &dir,
        &["--details", "--footprints", path(&fixtures())],
    ));
    assert!(details.contains("covered A-1 by review #1"), "{details}");
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
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A footprint renamed keeps its review (the hash is the same), an edited one loses it; an MPN the lock lacks
/// asks for a refresh
#[test]
fn hash_rules_and_a_stale_lock() {
    let dir = scratch("hash-rules");
    project(&dir, &SYMBOLS[..1]);
    refresh(&dir);
    rename_footprint(&dir, "U1", "Asym_SOT", "Renamed:Other_Name");
    refresh(&dir);
    let v = json(&fp_check(&dir, &["--format", "json"]));
    assert_eq!(entry(&v, "U1")["status"], "ok", "a rename keeps the review");
    edit_pad(&dir, "U1");
    let v = json(&fp_check(&dir, &["--format", "json"]));
    assert_eq!(
        entry(&v, "U1")["status"],
        "not-in-lock",
        "the new hash is not in the lock"
    );
    let text = stdout(&fp_check(&dir, &[]));
    assert!(
        text.contains("lacks pairs: run `berc parts refresh`"),
        "{text}"
    );
    refresh(&dir);
    let v = json(&fp_check(&dir, &["--format", "json"]));
    assert_eq!(
        entry(&v, "U1")["status"],
        "changed",
        "an edit loses the review"
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
    run(&["fp", "check", "--lib", "X", "--footprints", path(&dir)]);
    run(&["fp", "todo", path(&dir), "--symbols", path(&dir)]);
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
            "fp check --footprints",
            "fp todo",
            "fp todo --symbols"
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
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The library symbols of [`SYMBOLS`] with their own MPNs, judged against the part source before a board uses
/// them; a symbol whose footprint is not found, one without a footprint
#[test]
fn check_a_symbol_library() {
    let dir = scratch("lib");
    project(&dir, &SYMBOLS);
    let lib = dir.join("Rev.kicad_sym");
    let mut text = std::fs::read_to_string(&lib).unwrap();
    let end = text.rfind(')').unwrap();
    text.insert_str(
        end,
        &format!(
            "\n{}\n{}\n",
            lib_symbol("Lost", "Test:Gone", &["A-1"]),
            lib_symbol("Power", "", &[])
        ),
    );
    std::fs::write(&lib, text).unwrap();
    let fps = fixtures();
    let args = |extra: &[&'static str]| {
        let mut a = vec!["--lib", path(&lib), "--footprints", path(&fps)];
        a.extend(extra);
        a
    };
    let out = fp_check(
        &dir,
        &[
            args(&["--from"]),
            vec![path(&dir.join(PARTS)), "--format", "json"],
        ]
        .concat(),
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let v = json(&out);
    let status = |s: &str| {
        v["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["symbol"] == s)
            .unwrap_or_else(|| panic!("{s}"))["status"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(v["source"], "file parts.toml");
    assert_eq!(status("Rev:Ok"), "ok");
    assert_eq!(status("Rev:Fam"), "ok");
    assert_eq!(status("Rev:Changed"), "changed");
    assert_eq!(status("Rev:Unc"), "uncovered");
    assert_eq!(status("Rev:Partial"), "partial");
    assert_eq!(status("Rev:Future"), "other-version");
    assert_eq!(status("Rev:NoMpn"), "no-mpn");
    assert_eq!(status("Rev:Lost"), "missing");
    assert_eq!(status("Rev:Power"), "no-footprint");
    // No part source: hashed, not judged
    let v = json(&fp_check(&dir, &args(&["--format", "json"])));
    assert_eq!(v["symbols"][0]["status"], "unjudged", "{v}");
    let text = stdout(&fp_check(&dir, &args(&[])));
    assert!(
        text.contains("no part source to judge 1 MPNs against"),
        "{text}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

fn changed_footprints(dir: &Path) -> PathBuf {
    let lib = dir.join("changed/Test.pretty");
    std::fs::create_dir_all(&lib).unwrap();
    for name in ["Asym_SOT", "Mixed_Pads"] {
        let text =
            std::fs::read_to_string(fixtures().join(format!("Test.pretty/{name}.kicad_mod")))
                .unwrap()
                .replacen(
                    "(at -1.1 -0.95) (size 1.05 0.6)",
                    "(at -1.1 -0.95) (size 1.2 0.6)",
                    1,
                );
        std::fs::write(lib.join(format!("{name}.kicad_mod")), text).unwrap();
    }
    dir.join("changed")
}

#[test]
fn check_compares_board_copies_with_their_library_footprint() {
    let sot = library_hash("Asym_SOT");
    let dir = scratch("board-differs");
    project(&dir, &SYMBOLS[..1]);
    refresh(&dir);
    let out = fp_check(
        &dir,
        &["--format", "json", "--footprints", path(&fixtures())],
    );
    let v = json(&out);
    let u1 = entry(&v, "U1");
    assert_eq!(u1["library_footprint"], "Test:Asym_SOT", "{u1}");
    assert_eq!(u1["library_footprint_hash"], sot.as_str());
    assert_eq!(u1["board_differs"], false);
    // A footprint without a symbol and without a library nickname can't be looked up
    assert!(
        entry(&v, "U8")["footprint_note"]
            .as_str()
            .unwrap()
            .contains("names no library"),
        "{v}"
    );

    let changed = changed_footprints(&dir);
    let out = fp_check(&dir, &["--format", "json", "--footprints", path(&changed)]);
    assert_eq!(out.status.code(), Some(1));
    let v = json(&out);
    let u1 = entry(&v, "U1");
    assert_eq!(
        u1["status"], "ok",
        "the review hash is the board copy's: {u1}"
    );
    assert_eq!(u1["board_differs"], true, "{u1}");
    let text = stdout(&fp_check(&dir, &["--footprints", path(&changed)]));
    assert!(
        text.contains(&format!(
            "the board copy has {sot}, library footprint Test:Asym_SOT has fp1:"
        )),
        "{text}"
    );
    assert!(
        text.contains("; 1 board copies differ from their library footprint"),
        "{text}"
    );
    assert!(
        text.contains("not compared with a library footprint (6 footprints): footprint Mixed_Pads names no library"),
        "{text}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// One line per footprint and family (or MPN alone), with its MPNs and references, in review order
#[test]
fn todo_lists_pairs_in_review_order() {
    let dir = scratch("todo");
    project(&dir, &SYMBOLS);
    refresh(&dir);
    let fps = fixtures();
    let out = fp_todo(&dir, &["--footprints", path(&fps), "--format", "json"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let v = json(&out);
    let entries = v["entries"].as_array().unwrap();
    let rows: Vec<(String, String, String)> = entries
        .iter()
        .map(|e| {
            (
                e["status"].as_str().unwrap().to_owned(),
                e["mpns"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(" "),
                e["references"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
        .collect();
    let row = |a: &str, b: &str, c: &str| (a.to_owned(), b.to_owned(), c.to_owned());
    assert_eq!(
        rows,
        [
            row("changed", "B-1", "U3"),
            row("other-version", "E-1", "U6"),
            row("uncovered", "RC0603FR-0710KL RC0603FR-071KL", "U4 U5"),
            row("no-mpn", "", "U7"),
            row("covered", "A-1", "U1 U5"),
            row("covered", "RC0402FR-0710KL", "U2"),
        ]
    );
    let family = &entries[2];
    assert_eq!(
        family["cover"],
        serde_json::json!({"kind": "family", "scheme": "yageo.rc", "segments": {"case_eia": "0603"}})
    );
    assert_eq!(family["hash"], library_hash("Asym_SOT").as_str());
    assert_eq!(family["datasheet"], "docs/Unc.pdf");
    assert!(
        family["library_file"]
            .as_str()
            .is_none_or(|f| f.ends_with("Asym_SOT.kicad_mod")),
        "{family}"
    );
    assert_eq!(entries[0]["other"][0]["id"], 3);

    let text = stdout(&fp_todo(&dir, &["--footprints", path(&fps)]));
    assert!(text.contains("family yageo.rc case_eia=0603"), "{text}");
    assert!(
        text.contains("MPNs RC0603FR-0710KL RC0603FR-071KL  on U4 U5"),
        "{text}"
    );
    assert!(
        text.contains("can't be reviewed until fixed (1):"),
        "{text}"
    );
    assert!(
        text.contains("2 reviewed, not listed: --all lists them"),
        "{text}"
    );
    assert!(text.contains("6 footprint pairs, 3 to review"), "{text}");
    let all = stdout(&fp_todo(&dir, &["--footprints", path(&fps), "--all"]));
    assert!(all.contains("reviewed (2):"), "{all}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The first pad of board footprint `reference` made wider: its review hash changes
fn edit_pad(dir: &Path, reference: &str) {
    let board = std::fs::read_to_string(dir.join("fpcheck.kicad_pcb")).unwrap();
    let at = board
        .find(&format!("(property \"Reference\" \"{reference}\""))
        .unwrap();
    let start = board[..at].rfind("(footprint ").unwrap();
    let pad = start + board[start..].find("(pad ").unwrap();
    let pad = pad + board[pad..].find("(size ").unwrap();
    let end = pad + board[pad..].find(')').unwrap();
    let mut out = board[..pad].to_owned();
    out += "(size 1.7 0.9";
    out += &board[end..];
    std::fs::write(dir.join("fpcheck.kicad_pcb"), out).unwrap();
}

fn rename_footprint(dir: &Path, reference: &str, old: &str, new: &str) {
    let board = std::fs::read_to_string(dir.join("fpcheck.kicad_pcb")).unwrap();
    let ref_marker = format!("(property \"Reference\" \"{reference}\"");
    let ref_at = board.find(&ref_marker).unwrap_or_else(|| {
        panic!(
            "no {reference} in {}",
            dir.join("fpcheck.kicad_pcb").display()
        )
    });
    let old_marker = format!("(footprint \"{old}\"");
    let fp_at = board[..ref_at]
        .rfind(&old_marker)
        .unwrap_or_else(|| panic!("no {old_marker:?} before {reference}"));
    let mut out = board[..fp_at].to_string();
    out += &format!("(footprint \"{new}\"");
    out += &board[fp_at + old_marker.len()..];
    std::fs::write(dir.join("fpcheck.kicad_pcb"), out).unwrap();
}

/// U1 placed on the board as `VHRD_Test:Asym_SOT`, its symbol's own `Footprint` still `Test:Asym_SOT` (as a
/// library migration might leave it, half done): `fp check` and `fp todo` name both rather than silently taking
/// the symbol's; a footprint without a library nickname is no override
#[test]
fn check_and_todo_report_an_overridden_footprint() {
    let dir = scratch("override");
    project(&dir, &SYMBOLS[..1]);
    refresh(&dir);
    let fps = fixtures();
    let v = json(&fp_check(
        &dir,
        &["--footprints", path(&fps), "--format", "json"],
    ));
    assert_eq!(entry(&v, "U1")["footprint"], "Asym_SOT");
    assert_eq!(
        entry(&v, "U1")["footprint_overridden"],
        false,
        "a bare name is no override"
    );

    rename_footprint(&dir, "U1", "Asym_SOT", "VHRD_Test:Asym_SOT");
    let v = json(&fp_check(
        &dir,
        &["--footprints", path(&fps), "--format", "json"],
    ));
    let u1 = entry(&v, "U1");
    assert_eq!(u1["symbol_footprint"], "Test:Asym_SOT");
    assert_eq!(u1["footprint_overridden"], true);
    assert_eq!(u1["status"], "ok", "the hash itself still matches: {u1}");
    let text = stdout(&fp_check(&dir, &["--footprints", path(&fps)]));
    assert!(
        text.contains(
            "the board places VHRD_Test:Asym_SOT, symbol Rev:Ok sets Test:Asym_SOT: an override"
        ),
        "{text}"
    );
    assert!(
        text.contains("1 board copies place an overridden footprint"),
        "{text}"
    );

    let out = fp_todo(&dir, &["--footprints", path(&fps), "--format", "json"]);
    assert_eq!(out.status.code(), Some(1), "an override fails todo too");
    let v = json(&out);
    assert_eq!(
        v["entries"][0]["overridden"],
        serde_json::json!([["U1", "Test:Asym_SOT"]])
    );
    let text = stdout(&fp_todo(&dir, &["--footprints", path(&fps)]));
    assert!(
        text.contains("placed as VHRD_Test:Asym_SOT: U1 (symbol sets Test:Asym_SOT)"),
        "{text}"
    );
    assert!(text.contains("1 with an overridden footprint"), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `--allow` lists the library nicknames symbols and their footprints may be in: `fp check --lib` and `fp todo`
/// report any other
#[test]
fn allow_reports_libraries_outside_it() {
    let dir = scratch("allow");
    project(&dir, &SYMBOLS[..1]);
    refresh(&dir);
    let lib = dir.join("Rev.kicad_sym");
    let fps = fixtures();
    let lib_check = |allow: &[&str]| {
        let mut a = vec![
            "--lib",
            path(&lib),
            "--footprints",
            path(&fps),
            "--format",
            "json",
        ];
        for x in allow {
            a.extend(["--allow", *x]);
        }
        json(&fp_check(&dir, &a))
    };
    let v = lib_check(&[]);
    assert_eq!(
        v["symbols"][0]["symbol_library_disallowed"],
        serde_json::Value::Null
    );
    let v = lib_check(&["Test"]);
    assert_eq!(v["symbols"][0]["symbol_library_disallowed"], "Rev");
    assert_eq!(
        v["symbols"][0]["footprint_library_disallowed"],
        serde_json::Value::Null
    );
    let v = lib_check(&["Rev*", "Test"]);
    assert_eq!(
        v["symbols"][0]["symbol_library_disallowed"],
        serde_json::Value::Null
    );
    let text = stdout(&fp_check(
        &dir,
        &[
            "--lib",
            path(&lib),
            "--footprints",
            path(&fps),
            "--allow",
            "Test",
        ],
    ));
    assert!(
        text.contains("its own library Rev is not in the allowed libraries"),
        "{text}"
    );

    let v = json(&fp_todo(
        &dir,
        &[
            "--footprints",
            path(&fps),
            "--format",
            "json",
            "--allow",
            "Test",
        ],
    ));
    assert_eq!(v["entries"][0]["symbol_library_disallowed"], "Rev");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn todo_errors_exit_2() {
    let dir = scratch("todo-errors");
    project(&dir, &SYMBOLS[..1]);
    refresh(&dir);
    let out = fp_todo(&dir, &["--footprints", path(&dir.join("nothing"))]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no footprint library (.pretty)"),
        "{out:?}"
    );
    let out = fp_todo(&dir, &["--symbols", path(&dir.join("nothing"))]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let out = fp_todo(&dir.join("nothing"), &[]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------------------------------------------
// berc fp accept

/// Accepting U4 records a family review of RC 0603 for its hash in the part source and refreshes the lock: U4 and
/// U5 (whose other MPN is reviewed already) are ok after it; nothing in the schematic or library changes
#[test]
fn accept_round_trip_through_the_part_source() {
    let dir = scratch("accept");
    project(&dir, &SYMBOLS);
    refresh(&dir);
    let before = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap();
    let (sch, lib) = (before("fpcheck.kicad_sch"), before("Rev.kicad_sym"));
    let dry = fp_accept(&dir, &["U4", "--dry-run", "--format", "json"]);
    assert!(dry.status.success(), "{dry:?}");
    let v = json(&dry);
    assert_eq!(v["review"], serde_json::Value::Null);
    assert_eq!(
        v["covers"],
        serde_json::json!([{"kind": "family", "scheme": "yageo.rc", "segments": {"case_eia": "0603"}}])
    );
    assert!(
        !before(PARTS).contains("id = 5"),
        "a dry run records nothing"
    );

    let out = fp_accept(
        &dir,
        &[
            "U4",
            "--doc",
            "RC_Series@rev12",
            "--pdf",
            "b3:0123456789abcdef",
            "--page",
            "4",
            "--by",
            "Jane Doe",
        ],
    );
    assert!(out.status.success(), "{out:?}");
    let text = stdout(&out);
    assert!(text.contains("review #5 in file parts.toml"), "{text}");
    assert!(
        text.contains("covers family yageo.rc case_eia=0603"),
        "{text}"
    );
    assert!(text.contains("parts.lock.toml"), "{text}");
    let parts = before(PARTS);
    assert!(
        parts.contains("doc = \"RC_Series\"\ndoc_rev = \"rev12\""),
        "{parts}"
    );
    assert!(
        parts.contains("pdf_blake3 = \"0123456789abcdef\""),
        "{parts}"
    );
    assert!(parts.contains("reviewer = \"Jane Doe\""), "{parts}");
    let v = json(&fp_check(&dir, &["--format", "json"]));
    assert_eq!(entry(&v, "U4")["status"], "ok");
    assert_eq!(
        entry(&v, "U5")["status"],
        "ok",
        "the family covers U5's MPN too"
    );
    assert_eq!(before("fpcheck.kicad_sch"), sch);
    assert_eq!(before("Rev.kicad_sym"), lib);

    // One MPN alone, by hand for a hash: covers B-1 for the current hash
    let sot = library_hash("Asym_SOT");
    let out = berc_in(
        &dir,
        &[
            "fp",
            "accept",
            "--hash",
            &sot,
            "--footprint-id",
            "Test:Asym_SOT",
            "--mpn",
            "B-1",
            "--from",
            path(&dir.join(PARTS)),
        ],
    );
    assert!(out.status.success(), "{out:?}");
    refresh(&dir);
    let v = json(&fp_check(&dir, &["--format", "json"]));
    assert_eq!(entry(&v, "U3")["status"], "ok");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn accept_refusals_record_nothing() {
    let dir = scratch("accept-refused");
    project(&dir, &SYMBOLS);
    refresh(&dir);
    let parts = std::fs::read_to_string(dir.join(PARTS)).unwrap();
    for (args, says) in [
        (vec!["U99"], "no footprint U99 on the board"),
        (vec!["U7"], "U7 has no Mpn field"),
        (vec!["U11"], "has no schematic symbol"),
        (
            vec!["U1", "--mpn", "B-1"],
            "no footprint accepted carries MPN B-1",
        ),
        (vec!["U1", "--family-of", "A-1"], "decodes into no family"),
    ] {
        let out = fp_accept(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(says), "{args:?}: {err}");
    }
    let out = berc_in(&dir, &["fp", "accept", path(&dir), "U1"]);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--from"),
        "{out:?}"
    );
    let out = berc_in(
        &dir,
        &[
            "fp",
            "accept",
            "--hash",
            "fp1:nothex",
            "--footprint-id",
            "X:Y",
            "--mpn",
            "A-1",
            "--from",
            path(&dir.join(PARTS)),
        ],
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no review hash"),
        "{out:?}"
    );
    assert_eq!(std::fs::read_to_string(dir.join(PARTS)).unwrap(), parts);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `--footprint` sets the symbol's Footprint to the placed one, in the library and the schematic's copy
#[test]
fn accept_footprint_sets_the_symbol_footprint() {
    let dir = scratch("accept-footprint");
    project(&dir, &SYMBOLS[..2]);
    refresh(&dir);
    rename_footprint(&dir, "U1", "Asym_SOT", "VHRD_Test:Asym_SOT");
    let out = fp_accept(&dir, &["U1", "--footprint", "VHRD_Test:Asym_SOT"]);
    assert!(out.status.success(), "{out:?}");
    let text = stdout(&out);
    assert!(text.contains("Footprint was Test:Asym_SOT"), "{text}");
    for f in ["Rev.kicad_sym", "fpcheck.kicad_sch"] {
        let t = std::fs::read_to_string(dir.join(f)).unwrap();
        assert!(
            t.contains("\"Footprint\" \"VHRD_Test:Asym_SOT\""),
            "{f}: {t}"
        );
    }
    let out = fp_accept(&dir, &["U1", "U2", "--footprint", "VHRD_Test:Asym_SOT"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("several symbols"),
        "{out:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn accept_references_complete_from_the_project() {
    let dir = scratch("complete");
    project(&dir, &SYMBOLS[..1]);
    let out = Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(["--", "berc", "fp", "accept", path(&dir), "U1"])
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
    std::fs::remove_dir_all(&dir).unwrap();
}
