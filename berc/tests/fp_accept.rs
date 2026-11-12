//! `berc fp accept`: review hashes written into library symbols and the schematic's copies, through the binary

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_kicad/tests/fixtures/footprints")
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).unwrap()
}

/// berc in `dir`'s KiCad world: no global library table, no colour, no usage record
fn berc(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(args)
        .env("BERC_USAGE", "off")
        .env("NO_COLOR", "1")
        .env("KICAD_CONFIG_HOME", dir.join("no-kicad-config"))
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

fn accept(dir: &Path, args: &[&str]) -> Output {
    let mut a = vec!["fp", "accept", path(dir)];
    a.extend(args);
    berc(dir, &a)
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("berc-fp-accept-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn library_hash(name: &str) -> String {
    let out = berc(
        Path::new("/nonexistent"),
        &[
            "fp",
            "hash",
            path(&fixtures().join(format!("Test.pretty/{name}.kicad_mod"))),
        ],
    );
    stdout(&out).trim().to_string()
}

/// A KiCad 10 symbol laid out as KiCad 10 writes it, its line indented `depth` tabs (1 in a library, 2 in a
/// schematic's `lib_symbols`)
fn symbol(name: &str, footprint: &str, review: Option<&str>, depth: usize) -> String {
    let t = |n: usize| "\t".repeat(depth + n);
    let prop = |key: &str, value: &str, hide: bool| {
        let hide = if hide {
            format!("{}(hide yes)\n", t(2))
        } else {
            String::new()
        };
        format!(
            "{0}(property \"{key}\" \"{value}\"\n{1}(at 0 0 0)\n{1}(show_name no)\n{1}(do_not_autoplace no)\n{hide}{1}(effects\n{2}(font\n{3}(size 1.27 1.27)\n{2})\n{1})\n{0})\n",
            t(1),
            t(2),
            t(3),
            t(4)
        )
    };
    let short = name.rsplit(':').next().unwrap();
    let mut s = format!(
        "{}(symbol \"{name}\"\n{1}(in_bom yes)\n{1}(on_board yes)\n",
        t(0),
        t(1)
    );
    s += &prop("Reference", "U", false);
    s += &prop("Value", short, false);
    s += &prop("Footprint", footprint, true);
    if let Some(r) = review {
        s += &prop("Footprint_Review", r, true);
    }
    s += &prop("ki_keywords", "test", true);
    s += &format!(
        "{0}(symbol \"{short}_1_1\"\n{1}(pin passive line\n{2}(at 0 0 0)\n{2}(length 1.27)\n{2}(name \"A\"\n{3}(effects\n{4}(font\n{5}(size 1.27 1.27)\n{4})\n{3})\n{2})\n{2}(number \"1\"\n{3}(effects\n{4}(font\n{5}(size 1.27 1.27)\n{4})\n{3})\n{2})\n{1})\n{0})\n",
        t(1),
        t(2),
        t(3),
        t(4),
        t(5),
        t(6)
    );
    s += &format!("{}(embedded_fonts no)\n{})\n", t(1), t(0));
    s
}

/// One library symbol: name, footprint, the library's hash, the schematic copy's hash
type Sym<'a> = (&'a str, &'a str, Option<&'a str>, Option<&'a str>);

/// A KiCad project in `dir`: the board of placed.kicad_pcb, a schematic placing U1, U2, ... from library `Rev` as
/// `uses` says (an index into `symbols` per reference), and with `table` a sym-lib-table and the library file
fn project(dir: &Path, symbols: &[Sym], uses: &[usize], table: bool) {
    std::fs::copy(
        fixtures().join("placed.kicad_pcb"),
        dir.join("acc.kicad_pcb"),
    )
    .unwrap();
    let root = "a0000000-0000-4000-8000-000000000000";
    let mut copies = String::new();
    let mut lib = String::from(
        "(kicad_symbol_lib\n\t(version 20251024)\n\t(generator \"kicad_symbol_editor\")\n\t(generator_version \"10.0\")\n",
    );
    for (name, footprint, lib_hash, sch_hash) in symbols {
        copies += &symbol(&format!("Rev:{name}"), footprint, *sch_hash, 2);
        lib += &symbol(name, footprint, *lib_hash, 1);
    }
    lib += ")\n";
    let mut placed = String::new();
    for (i, s) in uses.iter().enumerate() {
        let n = i + 1;
        let (name, footprint, ..) = symbols[*s];
        placed += &format!(
            "\t(symbol\n\t\t(lib_id \"Rev:{name}\")\n\t\t(at {x} 50 0)\n\t\t(unit 1)\n\t\t(in_bom yes)\n\t\t(on_board yes)\n\t\t(dnp no)\n\t\t(uuid \"a0000000-0000-4000-8000-0000000001{n:02}\")\n\t\t(property \"Reference\" \"U{n}\"\n\t\t\t(at {x} 45 0)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t)\n\t\t)\n\t\t(property \"Value\" \"{name}\"\n\t\t\t(at {x} 55 0)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t)\n\t\t)\n\t\t(property \"Footprint\" \"{footprint}\"\n\t\t\t(at {x} 50 0)\n\t\t\t(hide yes)\n\t\t\t(effects\n\t\t\t\t(font\n\t\t\t\t\t(size 1.27 1.27)\n\t\t\t\t)\n\t\t\t)\n\t\t)\n\t\t(pin \"1\"\n\t\t\t(uuid \"a0000000-0000-4000-8000-0000000002{n:02}\")\n\t\t)\n\t\t(instances\n\t\t\t(project \"acc\"\n\t\t\t\t(path \"/{root}\"\n\t\t\t\t\t(reference \"U{n}\")\n\t\t\t\t\t(unit 1)\n\t\t\t\t)\n\t\t\t)\n\t\t)\n\t)\n",
            x = 20 * n
        );
    }
    let sch = format!(
        "(kicad_sch\n\t(version 20260306)\n\t(generator \"eeschema\")\n\t(generator_version \"10.0\")\n\t(uuid \"{root}\")\n\t(paper \"A4\")\n\t(lib_symbols\n{copies}\t)\n{placed}\t(sheet_instances\n\t\t(path \"/\"\n\t\t\t(page \"1\")\n\t\t)\n\t)\n\t(embedded_fonts no)\n)\n"
    );
    std::fs::write(dir.join("acc.kicad_sch"), sch).unwrap();
    std::fs::write(dir.join("acc.kicad_pro"), "{}\n").unwrap();
    if table {
        std::fs::write(dir.join("Rev.kicad_sym"), lib).unwrap();
        std::fs::write(
            dir.join("sym-lib-table"),
            "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_sym\")(options \"\")(descr \"\")))\n",
        )
        .unwrap();
    }
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file)).unwrap()
}

/// `new` is `old` with one stretch replaced: what `old` and `new` have there
fn one_hunk<'a>(old: &'a str, new: &'a str) -> (&'a str, &'a str) {
    let pre = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let suf = old[pre..]
        .bytes()
        .rev()
        .zip(new[pre..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    (&old[pre..old.len() - suf], &new[pre..new.len() - suf])
}

fn status(dir: &Path, reference: &str) -> serde_json::Value {
    let out = berc(dir, &["fp", "check", path(dir), "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    v["footprints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["reference"] == reference)
        .unwrap()
        .clone()
}

/// U1-U6 are Asym_SOT on the board, U7 and up Mixed_Pads
const SYMBOLS: [Sym; 4] = [
    ("New", "Test:Asym_SOT", None, None),
    (
        "Old",
        "Test:Asym_SOT",
        Some("fp1:0000000000000000"),
        Some("fp1:0000000000000000"),
    ),
    ("NoFootprint", "", None, None),
    ("Mixed", "Test:Mixed_Pads", None, None),
];

#[test]
fn accept_writes_library_and_schematic_then_check_is_ok() {
    let sot = library_hash("Asym_SOT");
    let dir = scratch("write");
    project(&dir, &SYMBOLS, &[0, 1, 2, 0, 0, 0, 3], true);
    let (lib0, sch0) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    assert_eq!(status(&dir, "U1")["status"], "unreviewed");

    // A new field: inserted in both files, nothing else changed
    let out = accept(&dir, &["U1", "U4"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(&format!("Rev:New {sot} for U1, U4")),
        "{text}"
    );
    assert!(text.contains("(library): Footprint_Review added"), "{text}");
    assert!(
        text.contains("(schematic copy): Footprint_Review added"),
        "{text}"
    );
    let (lib1, sch1) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    for (old, new, indent) in [(&lib0, &lib1, "\t\t"), (&sch0, &sch1, "\t\t\t")] {
        // The new property on lines of its own, and nothing else
        let start = new
            .find(&format!(
                "\n{indent}(property \"Footprint_Review\" \"{sot}\"\n"
            ))
            .unwrap();
        let end =
            start + 1 + new[start + 1..].find(&format!("\n{indent})")).unwrap() + indent.len() + 2;
        assert_eq!(format!("{}{}", &new[..start], &new[end..]), *old);
    }
    let u1 = status(&dir, "U1");
    assert_eq!(u1["status"], "ok");
    assert_eq!(u1["copy_differs"], false);

    // An old value: only the value changes
    let out = accept(&dir, &["U2"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("Footprint_Review was fp1:0000000000000000"));
    let (lib2, sch2) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    assert_eq!(one_hunk(&lib1, &lib2), ("0000000000000000", &sot[4..]));
    assert_eq!(one_hunk(&sch1, &sch2), ("0000000000000000", &sot[4..]));
    assert_eq!(status(&dir, "U2")["status"], "ok");

    // Again: nothing to change
    let out = accept(&dir, &["U2", "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["field"], "Footprint_Review");
    assert_eq!(v["symbols"][0]["files"][0]["changed"], false);
    assert_eq!(read(&dir, "Rev.kicad_sym"), lib2);
}

#[test]
fn dry_run_writes_nothing() {
    let dir = scratch("dry");
    project(&dir, &SYMBOLS, &[0], true);
    let (lib0, sch0) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    let out = accept(&dir, &["U1", "--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("2 files would change"),
        "{}",
        stdout(&out)
    );
    assert_eq!(read(&dir, "Rev.kicad_sym"), lib0);
    assert_eq!(read(&dir, "acc.kicad_sch"), sch0);
}

#[test]
fn a_hash_given_by_hand_and_no_library_table() {
    let dir = scratch("symbol");
    project(&dir, &SYMBOLS, &[0], false);
    let h = "fp1:00112233445566aa";
    let out = accept(&dir, &["--symbol", "Rev:New", "--hash", h]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    // No sym-lib-table: only the schematic's copy, and why the library was not written
    assert!(
        text.contains("library symbol not written: library Rev is in no sym-lib-table"),
        "{text}"
    );
    assert!(
        read(&dir, "acc.kicad_sch").contains(&format!("(property \"Footprint_Review\" \"{h}\""))
    );
    let u1 = status(&dir, "U1");
    assert_eq!(u1["source"], "schematic");
    assert_eq!(u1["stored"], h);
}

#[test]
fn errors_write_nothing() {
    let dir = scratch("errors");
    project(&dir, &SYMBOLS, &[0, 1, 2, 0, 0, 0, 3], true);
    let (lib0, sch0) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    for (args, says) in [
        (vec!["U1", "U3"], "symbol Rev:NoFootprint sets no footprint"),
        (vec!["U99"], "no footprint U99"),
        (vec!["U8"], "U8 has no schematic symbol"),
        (
            vec!["--symbol", "Rev:New", "--hash", "fp1:123"],
            "is no review hash",
        ),
        (
            vec!["--symbol", "Rev:Gone", "--hash", "fp1:0123456789abcdef"],
            "symbol Rev:Gone is neither",
        ),
        (
            vec![
                "U1",
                "--symbol",
                "Rev:New",
                "--hash",
                "fp1:0123456789abcdef",
            ],
            "cannot be used with",
        ),
    ] {
        let out = accept(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stdout(&out));
        let err = stderr(&out);
        assert!(
            err.contains("error: ") && err.contains(says),
            "{args:?}: {err}"
        );
        assert_eq!(read(&dir, "Rev.kicad_sym"), lib0, "{args:?}");
        assert_eq!(read(&dir, "acc.kicad_sch"), sch0, "{args:?}");
    }
    // Neither references nor --symbol: a usage error
    let out = accept(&dir, &[]);
    assert_eq!(out.status.code(), Some(2));
}

/// U7 is Mixed_Pads, placed for symbol New like U1-U6: one hash can't hold for both footprints
#[test]
fn a_symbol_placed_with_two_footprints_is_refused() {
    let dir = scratch("two");
    project(&dir, &SYMBOLS, &[0, 0, 0, 0, 0, 0, 0], true);
    let lib0 = read(&dir, "Rev.kicad_sym");
    for args in [vec!["U1"], vec!["U7"], vec!["U1", "U7"]] {
        let out = accept(&dir, &args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let err = stderr(&out);
        assert!(
            err.contains("symbol Rev:New is placed with other footprints than"),
            "{err}"
        );
    }
    assert_eq!(read(&dir, "Rev.kicad_sym"), lib0);
}

#[test]
fn references_complete_from_the_project() {
    let dir = scratch("complete");
    project(&dir, &SYMBOLS, &[0], true);
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
}

/// A directory library (`.kicad_symdir`): the symbol's own file is written, the others are not touched
#[test]
fn symdir_library() {
    let sot = library_hash("Asym_SOT");
    let dir = scratch("symdir");
    project(&dir, &SYMBOLS, &[0], false);
    let lib = dir.join("Rev.kicad_symdir");
    std::fs::create_dir_all(&lib).unwrap();
    let file = |name: &str, footprint: &str| {
        format!(
            "(kicad_symbol_lib\n\t(version 20251024)\n\t(generator \"kicad_symbol_editor\")\n\t(generator_version \"10.0\")\n{})\n",
            symbol(name, footprint, None, 1)
        )
    };
    std::fs::write(lib.join("New.kicad_sym"), file("New", "Test:Asym_SOT")).unwrap();
    // A file not named after its symbol (KiCad escapes names for the file system)
    std::fs::write(
        lib.join("Old{slash}.kicad_sym"),
        file("Old/", "Test:Asym_SOT"),
    )
    .unwrap();
    let old = read(&lib, "Old{slash}.kicad_sym");
    std::fs::write(
        dir.join("sym-lib-table"),
        "(sym_lib_table (version 7)\n  (lib (name \"Rev\")(type \"KiCad\")(uri \"${KIPRJMOD}/Rev.kicad_symdir\")(options \"\")(descr \"\")))\n",
    )
    .unwrap();
    let out = accept(&dir, &["U1"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        read(&lib, "New.kicad_sym").contains(&format!("(property \"Footprint_Review\" \"{sot}\""))
    );
    assert_eq!(read(&lib, "Old{slash}.kicad_sym"), old);
    let u1 = status(&dir, "U1");
    assert_eq!(u1["status"], "ok");
    assert_eq!(u1["source"], "library");
    let h = "fp1:00112233445566aa";
    let out = accept(&dir, &["--symbol", "Rev:Old/", "--hash", h]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        read(&lib, "Old{slash}.kicad_sym")
            .contains(&format!("(property \"Footprint_Review\" \"{h}\""))
    );
}
