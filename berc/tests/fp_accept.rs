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

/// `acc.kicad_pcb` in `dir` (as [`project`] wrote it) with `reference`'s footprint's `Lib:Name` changed from `old`
/// to `new`: the geometry stays, only the name
fn rename_footprint(dir: &Path, reference: &str, old: &str, new: &str) {
    let board = read(dir, "acc.kicad_pcb");
    let ref_marker = format!("(property \"Reference\" \"{reference}\"");
    let ref_at = board
        .find(&ref_marker)
        .unwrap_or_else(|| panic!("no {reference} in acc.kicad_pcb"));
    let old_marker = format!("(footprint \"{old}\"");
    let fp_at = board[..ref_at]
        .rfind(&old_marker)
        .unwrap_or_else(|| panic!("no {old_marker:?} before {reference}"));
    let mut out = board[..fp_at].to_string();
    out += &format!("(footprint \"{new}\"");
    out += &board[fp_at + old_marker.len()..];
    std::fs::write(dir.join("acc.kicad_pcb"), out).unwrap();
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

/// `--footprint` also sets the library symbol's own `Footprint`, in the library and the schematic copy, the same
/// as `Footprint_Review`; a value already there and the same changes nothing
#[test]
fn accept_footprint_sets_it_in_both_files() {
    let dir = scratch("footprint");
    project(&dir, &SYMBOLS, &[0], true);
    rename_footprint(&dir, "U1", "Asym_SOT", "VHRD_Test:Asym_SOT");

    let out = accept(&dir, &["U1", "--footprint", "VHRD_Test:Asym_SOT"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("(library): Footprint was Test:Asym_SOT"),
        "{text}"
    );
    assert!(
        text.contains("(schematic copy): Footprint was Test:Asym_SOT"),
        "{text}"
    );
    for (file, indent) in [("Rev.kicad_sym", "\t\t"), ("acc.kicad_sch", "\t\t\t")] {
        assert!(
            read(&dir, file).contains(&format!(
                "\n{indent}(property \"Footprint\" \"VHRD_Test:Asym_SOT\"\n"
            )),
            "{file}: {}",
            read(&dir, file)
        );
    }

    // Already there: nothing changes
    let out = accept(
        &dir,
        &[
            "U1",
            "--footprint",
            "VHRD_Test:Asym_SOT",
            "--format",
            "json",
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let footprint = v["symbols"][0]["footprint"].as_array().unwrap();
    assert_eq!(footprint.len(), 2, "{footprint:?}");
    assert!(
        footprint.iter().all(|w| w["changed"] == false),
        "{footprint:?}"
    );

    // --dry-run writes nothing
    let (lib, sch) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    let out = accept(&dir, &["U1", "--footprint", "Test:Other", "--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(read(&dir, "Rev.kicad_sym"), lib);
    assert_eq!(read(&dir, "acc.kicad_sch"), sch);
}

/// `--footprint` for references of more than one symbol is ambiguous: one value can't hold for several, nothing
/// is written
#[test]
fn accept_footprint_for_several_symbols_is_refused() {
    let dir = scratch("footprint-ambiguous");
    project(&dir, &SYMBOLS, &[0, 3], true);
    let (lib, sch) = (read(&dir, "Rev.kicad_sym"), read(&dir, "acc.kicad_sch"));
    let out = accept(&dir, &["U1", "U2", "--footprint", "Test:Other"]);
    assert_eq!(out.status.code(), Some(2), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("footprint names a Footprint for several library symbols"),
        "{}",
        stderr(&out)
    );
    assert_eq!(read(&dir, "Rev.kicad_sym"), lib);
    assert_eq!(read(&dir, "acc.kicad_sch"), sch);
}

/// `project` with `Mpn1` A-7-F and `Mpn2` B,115 on every symbol (a Nexperia-style MPN with a comma)
fn with_mpns(dir: &Path) {
    for f in ["Rev.kicad_sym", "acc.kicad_sch"] {
        let t = read(dir, f).replace(
            "(property \"Footprint\" \"Test:Asym_SOT\"",
            "(property \"Mpn1\" \"A-7-F\") (property \"Mpn2\" \"B,115\") (property \"Footprint\" \"Test:Asym_SOT\"",
        );
        std::fs::write(dir.join(f), t).unwrap();
    }
}

/// The value of `property` of library symbol `New` in `file`, `None` without it
fn property(dir: &Path, file: &str, property: &str) -> Option<String> {
    let t = read(dir, file);
    let sym = t.find("(symbol \"").unwrap();
    let marker = format!("(property \"{property}\" \"");
    let at = t[sym..].find(&marker)? + sym + marker.len();
    Some(t[at..at + t[at..].find('"').unwrap()].to_string())
}

/// One review line per datasheet: a line for one maker's MPNs leaves the other MPN unreviewed (partial), a second
/// line completes it, a third datasheet that covers both takes their MPNs and the old lines go; a stale hash and
/// every line are shown with --details
#[test]
fn one_review_line_per_datasheet() {
    let sot = library_hash("Asym_SOT");
    let dir = scratch("record");
    project(&dir, &SYMBOLS[..1], &[0], true);
    with_mpns(&dir);
    let src = [
        "--doc",
        "DS1@Rev2",
        "--pdf",
        "b3:0123456789abcdef",
        "--page",
        "3",
        "--by",
        "tester",
    ];
    let mut args = vec!["U1", "--mpn", "A-7-F"];
    args.extend(src);
    let out = accept(&dir, &args);
    assert!(out.status.success(), "{}", stderr(&out));
    let first = property(&dir, "Rev.kicad_sym", "Footprint_Review").unwrap();
    let prefix =
        format!("{sot} mpn=A-7-F doc=DS1@Rev2 pdf=b3:0123456789abcdef page=3 by=tester on=");
    assert!(first.starts_with(&prefix), "{first}");
    assert_eq!(
        property(&dir, "acc.kicad_sch", "Footprint_Review").as_deref(),
        Some(first.as_str())
    );
    let u1 = status(&dir, "U1");
    assert_eq!(u1["status"], "partial");
    assert_eq!(u1["reviews"]["unreviewed_mpns"][0], "B,115");
    assert_eq!(u1["copy_differs"], false);
    let text = stdout(&berc(&dir, &["fp", "check", path(&dir)]));
    assert!(text.contains("MPNs not reviewed yet: B,115"), "{text}");
    let lib = stdout(&berc(
        &dir,
        &[
            "fp",
            "check",
            "--lib",
            path(&dir.join("Rev.kicad_sym")),
            "--footprints",
            path(&fixtures().join("Test.pretty")),
            path(&dir),
        ],
    ));
    assert!(lib.contains("partial"), "{lib}");

    // The other maker's datasheet: a line of its own, the comma in its MPN escaped
    let out = accept(
        &dir,
        &["U1", "--mpn", "B,115", "--doc", "DS2", "--by", "tester"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("Footprint_Review2 added"),
        "{}",
        stdout(&out)
    );
    let second = property(&dir, "Rev.kicad_sym", "Footprint_Review2").unwrap();
    assert!(
        second.starts_with(&format!("{sot} mpn=B%2C115 doc=DS2 by=tester on=")),
        "{second}"
    );
    assert_eq!(status(&dir, "U1")["status"], "ok");
    let details = stdout(&berc(&dir, &["fp", "check", path(&dir), "--details"]));
    assert!(
        details.contains(&format!("ok Footprint_Review: {first}")),
        "{details}"
    );
    assert!(
        details.contains(&format!("ok Footprint_Review2: {second}")),
        "{details}"
    );

    // A stale hash on one line: changed, and that line named
    for f in ["Rev.kicad_sym", "acc.kicad_sch"] {
        let t = read(&dir, f).replace(
            &format!("\"{sot} mpn=B%2C115"),
            "\"fp1:0000000000000000 mpn=B%2C115",
        );
        std::fs::write(dir.join(f), t).unwrap();
    }
    assert_eq!(status(&dir, "U1")["status"], "changed");
    let text = stdout(&berc(&dir, &["fp", "check", path(&dir)]));
    assert!(
        text.contains("stale: Footprint_Review2 (DS2) fp1:0000000000000000"),
        "{text}"
    );
    let details = stdout(&berc(&dir, &["fp", "check", path(&dir), "--details"]));
    assert!(details.contains("stale Footprint_Review2:"), "{details}");

    // One datasheet for both: their MPNs leave the old lines, which go, in the library and the schematic alike
    let out = accept(
        &dir,
        &["U1", "--mpn", "A-7-F", "--mpn", "B,115", "--doc", "DS3"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("Footprint_Review2 removed"),
        "{}",
        stdout(&out)
    );
    for f in ["Rev.kicad_sym", "acc.kicad_sch"] {
        let only = property(&dir, f, "Footprint_Review").unwrap();
        assert!(
            only.starts_with(&format!("{sot} mpn=A-7-F,B%2C115 doc=DS3")),
            "{f}: {only}"
        );
        assert_eq!(property(&dir, f, "Footprint_Review2"), None, "{f}");
    }
    assert_eq!(status(&dir, "U1")["status"], "ok");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A value with the hash alone (written before review lines had a source) stays valid for every MPN; a plain
/// accept without a source replaces it, in place
#[test]
fn a_hash_alone_stays_valid() {
    let sot = library_hash("Asym_SOT");
    let dir = scratch("legacy");
    project(
        &dir,
        &[("New", "Test:Asym_SOT", Some(&sot), Some(&sot))],
        &[0],
        true,
    );
    with_mpns(&dir);
    let u1 = status(&dir, "U1");
    assert_eq!(u1["status"], "ok");
    assert_eq!(u1["reviews"]["lines"][0]["line"]["hash"], sot.as_str());
    let details = stdout(&berc(&dir, &["fp", "check", path(&dir), "--details"]));
    assert!(details.contains("(no source recorded)"), "{details}");
    let out = accept(&dir, &["U1", "--dry-run"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("Footprint_Review already has it"),
        "{}",
        stdout(&out)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
