//! `berc parts refresh` (CFG-9) through the binary: a board with footprints, part data from a file, the lock
//! written once and unchanged after, `--check` in CI, BOM-20 before and after

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn berc(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .args(args)
        .env("BERC_USAGE", "off")
        .env("NO_COLOR", "1")
        .env("KICAD_CONFIG_HOME", "/nonexistent")
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8(b.to_vec()).unwrap()
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("berc-parts-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// One placed symbol of a KiCad 10 schematic, with an MPN field and, when given, a maker field beside it
fn placed(n: usize, mpn: &str, dnp: bool) -> String {
    let dnp = if dnp { "yes" } else { "no" };
    let (mpn, maker) = mpn.split_once('@').map_or((mpn, ""), |(m, k)| (m, k));
    let x = 20 * n;
    let maker = if maker.is_empty() {
        String::new()
    } else {
        format!(
            "\t\t(property \"Mfg1\" \"{maker}\" (at {x} 55 0) (hide yes) (effects (font (size 1.27 1.27))))\n"
        )
    };
    format!(
        "\t(symbol (lib_id \"Fix:U\") (at {x} 50 0) (unit 1) (in_bom yes) (on_board yes) (dnp {dnp}) (uuid \"a0000000-0000-4000-8000-0000000001{n:02}\")\n\
         \t\t(property \"Reference\" \"U{n}\" (at {x} 45 0) (effects (font (size 1.27 1.27))))\n\
         \t\t(property \"Value\" \"U\" (at {x} 55 0) (effects (font (size 1.27 1.27))))\n\
         \t\t(property \"Mpn1\" \"{mpn}\" (at {x} 55 0) (hide yes) (effects (font (size 1.27 1.27))))\n{maker}\
         \t\t(pin \"1\" (uuid \"a0000000-0000-4000-8000-0000000002{n:02}\"))\n\
         \t\t(instances (project \"acc\" (path \"/a0000000-0000-4000-8000-000000000000\" (reference \"U{n}\") (unit 1))))\n\t)\n"
    )
}

/// A KiCad project in a fresh directory: the board of erc_kicad's placed.kicad_pcb (U1 and U2 an Asym_SOT, ...), a schematic placing U1 with `PART-A`, U2 with `PART-B` and a DNP U3 with `PART-DNP`
fn project(name: &str) -> PathBuf {
    project_of(name, "PART-A", "PART-B")
}

/// [`project`] with U1's and U2's MPNs given (`MPN@Maker` adds `Mfg1`)
fn project_of(name: &str, u1: &str, u2: &str) -> PathBuf {
    let dir = scratch(name);
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_kicad/tests/fixtures/footprints");
    std::fs::copy(fixtures.join("placed.kicad_pcb"), dir.join("acc.kicad_pcb")).unwrap();
    let lib = "\t\t(symbol \"Fix:U\" (in_bom yes) (on_board yes)\n\
               \t\t\t(property \"Reference\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n\
               \t\t\t(property \"Value\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n\
               \t\t\t(symbol \"U_1_1\" (pin passive line (at 0 0 0) (length 1.27) (name \"A\" (effects (font (size 1.27 1.27)))) (number \"1\" (effects (font (size 1.27 1.27))))))\n\
               \t\t)\n";
    let sch = format!(
        "(kicad_sch (version 20260306) (generator \"eeschema\") (generator_version \"10.0\")\n\
         \t(uuid \"a0000000-0000-4000-8000-000000000000\")\n\t(paper \"A4\")\n\t(lib_symbols\n{lib}\t)\n{}{}{}\
         \t(sheet_instances (path \"/\" (page \"1\")))\n)\n",
        placed(1, u1, false),
        placed(2, u2, false),
        placed(3, "PART-DNP", true),
    );
    std::fs::write(dir.join("acc.kicad_sch"), sch).unwrap();
    std::fs::write(dir.join("acc.kicad_pro"), "{}\n").unwrap();
    dir
}

fn hash(dir: &Path, reference: &str) -> String {
    let out = berc(&["fp", "hash", path(&dir.join("acc.kicad_pcb")), reference]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    text(&out.stdout).trim().to_owned()
}

/// Part data: PART-A with a resistance and a review of U1's footprint covering it
fn parts_file(dir: &Path, resistance: f64) -> PathBuf {
    let h = hash(dir, "U1");
    let file = dir.join("parts.toml");
    std::fs::write(
        &file,
        format!(
            "version = 1\nsource = \"hand\"\n\n[[part]]\nmpn = \"PART-A\"\nmatch = \"found\"\nfacts = [\n    {{ key = \"resistance\", value = {resistance:?}, unit = \"Ω\", source = \"manual\", rank = 100, at = \"2026-11-25T00:00:00Z\" }},\n]\n\n\
             [[footprint]]\nhash = \"{h}\"\nmpn = \"PART-A\"\nstatus = \"covered\"\ncovered_by = [1]\n\n\
             [[review]]\nid = 1\nhash = \"{h}\"\nfootprint = \"Test:Asym_SOT\"\nreviewer = \"Jane Doe\"\nreviewed_on = \"2026-11-25\"\ncovers = [{{ kind = \"mpn\", mpn = \"PART-A\" }}]\n"
        ),
    )
    .unwrap();
    file
}

fn bom_20(dir: &Path) -> Vec<String> {
    let out = berc(&["check", path(dir), "--format", "json", "--only", "BOM-20"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}{}", text(&out.stdout), text(&out.stderr)));
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["message"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn refresh_writes_the_lock_once_and_the_check_sees_it() {
    let dir = project("refresh");
    let from = parts_file(&dir, 10e3);
    let lock = dir.join("parts.lock.toml");

    // Before: no lock, BOM-20 does not run
    assert!(bom_20(&dir).is_empty());

    let out = berc(&["parts", "refresh", path(&dir), "--from", path(&from)]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        text(&out.stdout),
        format!(
            "{}: 2 MPNs (1 not found), 2 footprint pairs (1 covered) from file parts.toml: written\n",
            lock.display()
        )
    );
    let first = std::fs::read_to_string(&lock).unwrap();
    // The fitted parts' MPNs only, each with the pair of its footprint
    assert!(
        first.contains("mpn = \"PART-A\"\nmatch = \"found\""),
        "{first}"
    );
    assert!(
        first.contains("mpn = \"PART-B\"\nmatch = \"not_found\""),
        "{first}"
    );
    assert!(!first.contains("PART-DNP"), "{first}");
    let h1 = hash(&dir, "U1");
    assert!(
        first.contains(&format!("hash = \"{h1}\"\nmpn = \"PART-A\"\nfootprint = \"Asym_SOT\"\nstatus = \"covered\"\ncovered_by = [1]")),
        "{first}"
    );
    assert!(first.contains("status = \"unknown_mpn\""), "{first}");
    assert!(first.contains("reviewer = \"Jane Doe\""), "{first}");

    // Again: the same bytes, nothing written
    let out = berc(&["parts", "refresh", path(&dir), "--from", path(&from)]);
    assert!(
        text(&out.stdout).ends_with(": unchanged\n"),
        "{}",
        text(&out.stdout)
    );
    assert_eq!(std::fs::read_to_string(&lock).unwrap(), first);
    assert!(bom_20(&dir).is_empty(), "every MPN is in the lock");

    // A new MPN on the board: BOM-20 until the next refresh
    let sch = std::fs::read_to_string(dir.join("acc.kicad_sch")).unwrap();
    std::fs::write(dir.join("acc.kicad_sch"), sch.replace("PART-B", "PART-C")).unwrap();
    assert_eq!(
        bom_20(&dir),
        [
            "MPN PART-C on U2 is not in the parts snapshot: its facts and footprint reviews are not checked"
        ]
    );
    assert!(
        berc(&["parts", "refresh", path(&dir), "--from", path(&from)])
            .status
            .success()
    );
    assert!(bom_20(&dir).is_empty());
}

#[test]
fn check_says_out_of_date_and_writes_nothing() {
    let dir = project("check");
    let from = parts_file(&dir, 10e3);
    assert!(
        berc(&["parts", "refresh", path(&dir), "--from", path(&from)])
            .status
            .success()
    );
    let lock = std::fs::read_to_string(dir.join("parts.lock.toml")).unwrap();
    let ok = berc(&[
        "parts",
        "refresh",
        path(&dir),
        "--from",
        path(&from),
        "--check",
    ]);
    assert_eq!(ok.status.code(), Some(0));
    // The source changed a fact
    let from = parts_file(&dir, 4.7e3);
    let stale = berc(&[
        "parts",
        "refresh",
        path(&dir),
        "--from",
        path(&from),
        "--check",
    ]);
    assert_eq!(stale.status.code(), Some(1));
    assert!(text(&stale.stdout).ends_with(": out of date (not written)\n"));
    assert_eq!(
        std::fs::read_to_string(dir.join("parts.lock.toml")).unwrap(),
        lock
    );
}

#[cfg(not(feature = "mx3"))]
#[test]
fn a_source_is_needed() {
    let dir = project("no-source");
    let out = berc(&["parts", "refresh", path(&dir)]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        text(&out.stderr),
        "berc: error: no part source: name a file with --from (this berc is built without the mx3 part store, cargo \
         feature mx3)\n"
    );
}

/// berc with the mx3 part source at `url`, `token` in MX3_TOKEN (none: unset), and a home without a token file
#[cfg(feature = "mx3")]
fn berc_mx3(args: &[&str], url: &str, token: Option<&str>) -> Output {
    let home = scratch("mx3-home");
    let mut c = Command::new(env!("CARGO_BIN_EXE_berc"));
    c.args(args)
        .env("BERC_USAGE", "off")
        .env("NO_COLOR", "1")
        .env("KICAD_CONFIG_HOME", "/nonexistent")
        .env("HOME", &home)
        .env("TPM_ROOT", home.join("no-tpm"))
        .env("MX3_URL", url)
        .env_remove("COMPLETE");
    match token {
        Some(t) => c.env("MX3_TOKEN", t),
        None => c.env_remove("MX3_TOKEN"),
    };
    c.output().unwrap()
}

#[cfg(feature = "mx3")]
#[test]
fn refresh_and_accept_through_mx3() {
    use erc_fake_mx3::{CAPACITOR, FakeMx3, RESISTOR, TOKEN};
    let fake = FakeMx3::start();
    let dir = project_of("mx3", RESISTOR, CAPACITOR);
    let lock = dir.join("parts.lock.toml");

    let out = berc_mx3(&["parts", "refresh", path(&dir)], fake.url(), Some(TOKEN));
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        text(&out.stdout),
        format!(
            "{}: 2 MPNs (0 not found), 2 footprint pairs (0 covered) from mx3: written\n",
            lock.display()
        )
    );
    let first = std::fs::read_to_string(&lock).unwrap();
    assert!(first.contains("source = \"mx3\""), "{first}");
    assert!(first.contains("key = \"resistance\""), "{first}");
    assert!(first.contains("status = \"uncovered\""), "{first}");

    let out = berc_mx3(
        &[
            "fp",
            "accept",
            path(&dir),
            "U1",
            "--doc",
            "RC_Series@rev12",
            "--page",
            "4",
            "--by",
            "tester",
        ],
        fake.url(),
        Some(TOKEN),
    );
    assert!(out.status.success(), "{}", text(&out.stderr));
    let reviews = fake.reviews();
    assert_eq!(reviews.len(), 1);
    let r = &reviews[0];
    assert_eq!(r.hash, hash(&dir, "U1"));
    assert_eq!(
        (r.source.doc.as_deref(), r.source.doc_rev.as_deref()),
        (Some("RC_Series"), Some("rev12"))
    );
    assert_eq!((r.reviewer.as_str(), r.source.page), ("tester", Some(4)));
    let after = std::fs::read_to_string(&lock).unwrap();
    assert!(after.contains("covered_by = [7]"), "{after}");
    assert!(after.contains("[[review]]\nid = 7"), "{after}");

    // the lock is current: --check against mx3 finds nothing to write
    let out = berc_mx3(
        &["parts", "refresh", path(&dir), "--check"],
        fake.url(),
        Some(TOKEN),
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(
        fake.requests()
            .iter()
            .all(|r| r.authorization.as_deref() == Some(format!("Bearer {TOKEN}").as_str()))
    );
}

#[cfg(feature = "mx3")]
#[test]
fn the_maker_beside_an_mpn_chooses_its_part() {
    use erc_fake_mx3::{FakeMx3, MAKER_A, MAKER_A_SYNONYM, MAKER_B, NO_MAKER, SHARED, TOKEN};
    let fake = FakeMx3::start();
    let dir = project_of("mx3-makers", &format!("{SHARED}@{MAKER_A_SYNONYM}"), SHARED);
    let out = berc_mx3(&["parts", "refresh", path(&dir)], fake.url(), Some(TOKEN));
    assert!(out.status.success(), "{}", text(&out.stderr));
    let lock = std::fs::read_to_string(dir.join("parts.lock.toml")).unwrap();
    // U1 names its maker: that part, with its facts; U2 does not: the makers to choose from
    assert!(
        lock.contains(&format!(
            "mpn = \"{SHARED}\"\nmaker = \"{MAKER_A_SYNONYM}\"\nmatch = \"found\"\npart = \"21\"\n"
        )),
        "{lock}"
    );
    assert!(lock.contains("key = \"power\", value = 0.0625"), "{lock}");
    assert!(
        lock.contains(&format!(
            "mpn = \"{SHARED}\"\nmatch = \"not_unique\"\nmakers = [{{ part = \"21\", maker = \"{MAKER_A}\" }}, {{ part = \"22\", maker = \"{MAKER_B}\" }}, {{ part = \"23\", maker = \"{NO_MAKER}\" }}]\n"
        )),
        "{lock}"
    );

    let out = berc(&["check", path(&dir), "--format", "json", "--only", "BOM-21"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}{}", text(&out.stdout), text(&out.stderr)));
    let messages: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert_eq!(
        messages,
        [format!(
            "U2: MPN {SHARED} (Mpn1) matches 3 parts in the part source ({MAKER_A}, {MAKER_B}, {NO_MAKER}): which one \
             is meant is not said, so its facts go unchecked"
        )]
    );
}

#[cfg(feature = "mx3")]
#[test]
fn mx3_needs_a_token_and_never_shows_it() {
    let fake = erc_fake_mx3::FakeMx3::start();
    let dir = project("mx3-token");
    let out = berc_mx3(&["parts", "refresh", path(&dir)], fake.url(), None);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("no mx3 token: set MX3_TOKEN"),
        "{}",
        text(&out.stderr)
    );
    assert!(fake.requests().is_empty());

    let secret = "mx3_not-the-right-token";
    let out = berc_mx3(&["parts", "refresh", path(&dir)], fake.url(), Some(secret));
    assert_eq!(out.status.code(), Some(2));
    let shown = text(&out.stdout) + &text(&out.stderr);
    assert!(shown.contains("401"), "{shown}");
    assert!(!shown.contains(secret), "{shown}");
    assert!(!dir.join("parts.lock.toml").exists());
}

#[cfg(feature = "mx3")]
#[test]
fn help_names_where_mx3_is_found() {
    for cmd in [["parts", "refresh"], ["fp", "accept"]] {
        let out = berc(&[cmd[0], cmd[1], "--help"]);
        let help = text(&out.stdout);
        for needle in [
            "MX3_URL",
            "[share] mx3_url",
            "MX3_TOKEN",
            "~/.config/mx3/token",
            "never printed",
        ] {
            assert!(
                help.contains(needle),
                "{} {}: {needle}\n{help}",
                cmd[0],
                cmd[1]
            );
        }
    }
}
