//! `berc check`, `berc explain` and `berc checks list` on the test schematics: every output format, the exit codes,
//! the baseline delta and `--require`

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// `test_schematics/sources`
fn sources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources")
}

/// The I2C-1 test schematic: three errors (I2C-1, I2C-3 twice)
fn i2c1() -> PathBuf {
    sources().join("i2c-1_no_pull_ups.kicad_sch")
}

fn berc(args: &[&str]) -> Output {
    berc_in(Path::new(env!("CARGO_MANIFEST_DIR")), args)
}

fn berc_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_berc"))
        .current_dir(dir)
        .args(args)
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("COMPLETE")
        .output()
        .unwrap()
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exited")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{e}: {}", stdout(out)))
}

/// A scratch directory of its own for a test
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn text_matches_the_snapshot() {
    let out = berc(&["check", i2c1().to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains('\x1b'), "colour codes on a pipe: {text:?}");
    // The run time is the one thing that changes between runs
    let (head, tail) = text.rsplit_once(" · ").unwrap();
    assert!(tail.ends_with(" ms\n"), "{tail}");
    let text = format!("{head} · N ms\n");
    let snapshot = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check_i2c-1.txt");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&snapshot, &text).unwrap();
    }
    let expected = std::fs::read_to_string(&snapshot).unwrap();
    assert_eq!(
        text,
        expected,
        "differs from {} (BLESS=1 to update)",
        snapshot.display()
    );
}

#[test]
fn json_is_the_report_schema_1() {
    let out = berc(&["check", i2c1().to_str().unwrap(), "--format", "json"]);
    assert_eq!(code(&out), 1);
    let v = json(&out);
    assert_eq!(v["schema"], 1);
    assert!(v["tool"].as_str().unwrap().starts_with("better_erc "));
    assert_eq!(v["summary"]["errors"], 3);
    let checks: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["check"].as_str().unwrap())
        .collect();
    assert_eq!(checks, ["I2C-1", "I2C-3", "I2C-3"]);
    // Reads back as a report: the same JSON is a baseline
    let report = better_erc::Report::from_json(&stdout(&out)).unwrap();
    assert_eq!(report.findings.len(), 3);
    assert!(report.baseline.is_none());
    // A symbol's location has its line in the schematic
    let locations = v["findings"][1]["locations"].as_array().unwrap();
    let r201 = locations
        .iter()
        .find(|l| l["what"] == "symbol R201")
        .unwrap();
    assert_eq!(r201["file"], "i2c-1_no_pull_ups.kicad_sch");
    assert_eq!(r201["line"], 2824);
}

#[test]
fn sarif_is_structurally_valid() {
    // Run from the repository root with a relative path, as a CI job does
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let out = berc_in(
        &root,
        &[
            "check",
            "test_schematics/sources/i2c-1_no_pull_ups.kicad_sch",
            "--format",
            "sarif",
        ],
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let v = json(&out);
    assert_eq!(v["version"], "2.1.0");
    assert_eq!(
        v["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    let runs = v["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    let driver = &run["tool"]["driver"];
    assert_eq!(driver["name"], "better_erc");
    let rules = driver["rules"].as_array().unwrap();
    // A rule per check that ran
    assert_eq!(rules.len(), 11);
    for r in rules {
        let id = r["id"].as_str().unwrap();
        assert!(
            better_erc::explain(id).is_some(),
            "rule {id} is a catalogue id"
        );
        assert!(!r["shortDescription"]["text"].as_str().unwrap().is_empty());
        assert!(
            ["error", "warning", "note"]
                .contains(&r["defaultConfiguration"]["level"].as_str().unwrap())
        );
    }
    let results = run["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    let mut lines = 0;
    for r in results {
        let i = r["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(rules[i]["id"], r["ruleId"]);
        // The catalogue's 🛑 is error
        assert_eq!(r["level"], "error");
        assert_eq!(r["kind"], "fail");
        assert!(!r["message"]["text"].as_str().unwrap().is_empty());
        assert!(
            r["partialFingerprints"]["berc/v1"]
                .as_str()
                .unwrap()
                .contains(':')
        );
        let locations = r["locations"].as_array().unwrap();
        assert!(!locations.is_empty());
        for l in locations {
            let pl = &l["physicalLocation"];
            assert_eq!(
                pl["artifactLocation"]["uri"],
                "test_schematics/sources/i2c-1_no_pull_ups.kicad_sch"
            );
            if let Some(line) = pl["region"]["startLine"].as_u64() {
                assert!(line >= 1);
                lines += 1;
            }
        }
    }
    assert!(lines > 0, "symbols have their line");
    assert_eq!(run["invocations"][0]["executionSuccessful"], true);
}

#[test]
fn markdown_for_a_pr_comment() {
    let out = berc(&["check", i2c1().to_str().unwrap(), "--format", "md"]);
    assert_eq!(code(&out), 1);
    let md = stdout(&out);
    assert!(
        md.starts_with("### better_erc: 3 errors, 0 warnings, 0 suggestions\n"),
        "{md}"
    );
    assert!(
        md.contains("| | Check | Finding | Subjects |\n|---|---|---|---|\n"),
        "{md}"
    );
    assert!(
        md.contains("| 🛑 | `I2C-1` | I2C bus /I2C1: no pull-ups on SCL and SDA"),
        "{md}"
    );
    assert!(md.contains("<details><summary>Notes ("), "{md}");
}

#[test]
fn exit_codes() {
    let path = i2c1();
    let p = path.to_str().unwrap();
    // 0: no findings in the checks run
    assert_eq!(code(&berc(&["check", p, "--only", "BOM"])), 0);
    // 1: errors; warnings only fail with --fail-on warning or lower
    assert_eq!(code(&berc(&["check", p])), 1);
    let warnings = sources().join("i2c-10_non_equal_pull_ups.kicad_sch");
    let w = warnings.to_str().unwrap();
    assert_eq!(code(&berc(&["check", w, "--only", "I2C-10"])), 0);
    assert_eq!(
        code(&berc(&[
            "check",
            w,
            "--only",
            "I2C-10",
            "--fail-on",
            "warning"
        ])),
        1
    );
    assert_eq!(
        code(&berc(&[
            "check",
            w,
            "--only",
            "I2C-10",
            "--fail-on",
            "suggestion"
        ])),
        1
    );
    // 2: usage and import errors
    for args in [
        &["check", "/nonexistent/board.kicad_pro"][..],
        &["check", p, "--variant", "NoSuchVariant"],
        &["check", p, "--only", "NOPE"],
        &["check", p, "--format", "xml"],
        &["check", p, "--require", "no-such-need"],
        &["check", p, "--baseline", "/nonexistent/report.json"],
        &["explain", "NOPE-1"],
    ] {
        let out = berc(args);
        assert_eq!(code(&out), 2, "{args:?}: {}", stderr(&out));
        assert!(out.stdout.is_empty(), "{args:?}");
    }
    // A broken design file is an import error
    let dir = scratch("broken");
    std::fs::write(dir.join("broken.kicad_sch"), "(kicad_sch (version").unwrap();
    let out = berc(&["check", dir.join("broken.kicad_sch").to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(
        stderr(&out).starts_with("berc: error: "),
        "{}",
        stderr(&out)
    );
}

#[test]
fn state_is_refused_not_faked() {
    let out = berc(&["check", i2c1().to_str().unwrap(), "--state", "sleep"]);
    assert_eq!(code(&out), 2);
    assert!(
        stderr(&out).contains("--state is reserved for power states"),
        "{}",
        stderr(&out)
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn require_an_unavailable_need_is_3() {
    let p = i2c1();
    // A schematic without its board file has no layout; 3 wins over the findings' 1
    let out = berc(&[
        "check",
        p.to_str().unwrap(),
        "--require",
        "layout",
        "--format",
        "json",
    ]);
    assert_eq!(code(&out), 3);
    assert!(
        stderr(&out).contains("required layout not available on i2c-1_no_pull_ups"),
        "{}",
        stderr(&out)
    );
    assert_eq!(json(&out)["schema"], 1, "the report is still written");
    // What is there passes
    assert_eq!(
        code(&berc(&[
            "check",
            p.to_str().unwrap(),
            "--require",
            "graph,values",
            "--only",
            "BOM"
        ])),
        0
    );
    // The project with its board file has a layout
    let out = berc(&[
        "check",
        sources().to_str().unwrap(),
        "--require",
        "layout",
        "--only",
        "BOM",
    ]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
}

#[test]
fn baseline_delta() {
    let dir = scratch("baseline");
    let sch = dir.join("i2c-1_no_pull_ups.kicad_sch");
    let original = std::fs::read_to_string(i2c1()).unwrap();
    std::fs::write(&sch, &original).unwrap();
    let s = sch.to_str().unwrap();
    let first = berc(&["check", s, "--format", "json"]);
    let baseline = dir.join("baseline.json");
    std::fs::write(&baseline, &first.stdout).unwrap();
    let b = baseline.to_str().unwrap();

    // The same design again: everything unchanged, and unchanged findings do not fail the run
    let out = berc(&["check", s, "--baseline", b, "--format", "json"]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let v = json(&out);
    assert_eq!(v["baseline"]["new"], 0);
    assert_eq!(v["baseline"]["unchanged"], 3);
    assert_eq!(v["baseline"]["fixed"].as_array().unwrap().len(), 0);
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["baseline"] == "unchanged")
    );

    // One more finding: a placeholder value on the fitted 0 Ω R205 (BOM-6, folding BOM-5 in)
    assert_eq!(original.matches("(property \"Value\" \"0R\"").count(), 1);
    std::fs::write(
        &sch,
        original.replace("(property \"Value\" \"0R\"", "(property \"Value\" \"TBD\""),
    )
    .unwrap();
    let out = berc(&["check", s, "--baseline", b, "--format", "json"]);
    assert_eq!(code(&out), 1, "a new error fails the run");
    let v = json(&out);
    assert_eq!(v["baseline"]["new"], 1);
    assert_eq!(v["baseline"]["unchanged"], 3);
    let new: Vec<&Value> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["baseline"] == "new")
        .collect();
    assert_eq!(new.len(), 1);
    assert_eq!(new[0]["check"], "BOM-6");
    assert_eq!(new[0]["subjects"][0]["name"], "R205");
    // The text and the other formats show it too
    let text = stdout(&berc(&["check", s, "--baseline", b]));
    assert!(text.contains("[new]"), "{text}");
    assert!(
        text.contains("baseline: 1 new, 3 unchanged, 0 fixed"),
        "{text}"
    );
    let sarif = json(&berc(&["check", s, "--baseline", b, "--format", "sarif"]));
    let states: Vec<&str> = sarif["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["baselineState"].as_str().unwrap())
        .collect();
    assert_eq!(states.iter().filter(|s| **s == "new").count(), 1);
    assert!(stdout(&berc(&["check", s, "--baseline", b, "--format", "md"])).contains("**1 new**"));

    // The new finding becomes the baseline; going back fixes it
    let with_tbd = dir.join("with_tbd.json");
    std::fs::write(&with_tbd, berc(&["check", s, "--format", "json"]).stdout).unwrap();
    std::fs::write(&sch, &original).unwrap();
    let out = berc(&[
        "check",
        s,
        "--baseline",
        with_tbd.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(code(&out), 0);
    let v = json(&out);
    assert_eq!(v["baseline"]["fixed"][0]["check"], "BOM-6");
}

#[test]
fn baseline_of_another_schema_is_refused() {
    let dir = scratch("old_schema");
    let old = dir.join("old.json");
    std::fs::write(&old, r#"{"schema": 0, "findings": []}"#).unwrap();
    let out = berc(&[
        "check",
        i2c1().to_str().unwrap(),
        "--baseline",
        old.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 2);
    assert!(
        stderr(&out).contains("report schema 0, this better_erc reads schema 1"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_directory_with_several_designs_names_them() {
    let dir = scratch("two_designs");
    for name in ["i2c-1_no_pull_ups.kicad_sch", "bom-10_values.kicad_sch"] {
        std::fs::copy(sources().join(name), dir.join(name)).unwrap();
    }
    let out = berc(&["check", dir.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    let err = stderr(&out);
    assert!(err.contains("several files could be the design, name one: bom-10_values.kicad_sch, i2c-1_no_pull_ups.kicad_sch"), "{err}");
    // A directory with a .kicad_pro is that project
    let out = berc(&["check", sources().to_str().unwrap(), "--format", "json"]);
    assert_eq!(
        json(&out)["summary"]["boards"][0]["name"],
        "test_schematics"
    );
}

#[test]
fn explain_text_and_markdown() {
    let out = berc(&["explain", "i2c-3"]);
    assert_eq!(code(&out), 0);
    let text = stdout(&out);
    assert!(
        text.starts_with("I2C-3 Pull-up from two series resistors to nowhere\n"),
        "{text}"
    );
    assert!(!text.contains("**"));
    let md = stdout(&berc(&["explain", "I2C-3", "--format", "md"]));
    assert!(md.starts_with("## `I2C-3` "), "{md}");
    assert!(md.contains("**Fix.**"));
    // A planned check has its page too
    let planned = stdout(&berc(&["explain", "PIN-1"]));
    assert!(
        planned.contains("planned: better_erc does not run this check yet"),
        "{planned}"
    );
}

#[test]
fn checks_list() {
    let all = json(&berc(&["checks", "list", "--format", "json"]));
    let all = all.as_array().unwrap();
    assert!(all.len() > 500);
    let implemented = all.iter().filter(|c| c["state"] == "implemented").count();
    assert_eq!(implemented, 11);
    let hard = json(&berc(&[
        "checks", "list", "--tier", "hard", "--format", "json",
    ]));
    assert!(hard.as_array().unwrap().iter().all(|c| c["tier"] == "hard"));
    let planned = json(&berc(&[
        "checks", "list", "--status", "planned", "--format", "json",
    ]));
    assert_eq!(planned.as_array().unwrap().len(), all.len() - 11);
    // ran and not-run are about a design
    let out = berc(&["checks", "list", "--status", "ran"]);
    assert_eq!(code(&out), 2);
    let ran = json(&berc(&[
        "checks",
        "list",
        "--status",
        "ran",
        "--project",
        i2c1().to_str().unwrap(),
        "--format",
        "json",
    ]));
    let ids: Vec<&str> = ran
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 11, "{ids:?}");
    assert!(ids.contains(&"I2C-3"));
    let text = stdout(&berc(&["checks", "list", "--tier", "easy"]));
    assert!(
        text.lines()
            .any(|l| l.starts_with("I2C-3    error      easy      implemented")),
        "{text}"
    );
}

#[test]
fn completion_offers_catalogue_values() {
    let complete = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_berc"))
            .env("COMPLETE", "fish")
            .arg("--")
            .arg("berc")
            .args(args)
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    let ids = complete(&["explain", "I2C-1"]);
    assert!(
        ids.contains("I2C-10\tSCL and SDA pull-ups of different value"),
        "{ids}"
    );
    let only = complete(&["check", ".", "--only", "I2C,BO"]);
    assert!(only.contains("I2C,BOM-5\t"), "{only}");
    let needs = complete(&["check", ".", "--require", "lay"]);
    assert!(needs.contains("layout\tthe board file"), "{needs}");
    // Variants of the design in the current directory
    let dir = scratch("variants");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../erc_kicad/tests/fixtures/netlist/kicad_versions/fixture_kicad10.kicad_sch"),
        dir.join("fixture_kicad10.kicad_sch"),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_berc"))
        .current_dir(&dir)
        .env("COMPLETE", "fish")
        .args(["--", "berc", "check", ".", "--variant", ""])
        .output()
        .unwrap();
    let variants = String::from_utf8(out.stdout).unwrap();
    assert!(
        variants.contains("Lite\tvariant of fixture_kicad10.kicad_sch"),
        "{variants}"
    );
}

/// An Altium Smart PDF end to end (the synthetic one of `erc_import`): read, checked on its graph and values, with
/// the import notes in the report and locations on the PDF's pages
#[test]
fn smart_pdf() {
    let pdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_import/tests/fixtures/smart_pdf.pdf");
    let out = berc(&["check", "--format", "json", pdf.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let r = json(&out);
    let board = &r["summary"]["boards"][0];
    assert_eq!(board["tool"], "altium-pdf");
    assert_eq!((board["sheets"].as_u64(), board["parts"].as_u64()), (Some(4), Some(7)));
    assert!(r["summary"]["ran"].as_u64().unwrap() > 0);
    let no_value: Vec<&str> = r["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "BOM-9")
        .map(|f| f["subjects"][0]["name"].as_str().unwrap())
        .collect();
    assert_eq!(no_value, ["R3", "R4"], "U1 and J1 have theirs in Comment");
    let r4 = &r["findings"][1];
    assert_eq!(r4["confidence"], "certain", "read from the bookmarks");
    assert_eq!(r4["locations"][0]["file"], "smart_pdf.pdf");
    assert_eq!(r4["locations"][0]["what"], "symbol R4 on page 3 (MCU.SchDoc)");
    assert!(
        r["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["at"] == "smart_pdf.pdf: nets"),
        "{}",
        r["notes"]
    );
}
