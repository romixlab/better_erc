//! The test schematics are the spec: every `test_schematics/sources/<id>_<slug>.kicad_sch` has a sidecar
//! `<id>_<slug>.expect.toml` listing the findings the engine must report on it (check, subjects, confidence, the
//! checks it folds in) and the checks it must report as not run. This test imports each schematic, runs every
//! registered check, and compares; a schematic without expectations, expectations without a schematic, or a
//! registered check that no expectation shows firing fails it. `BLESS=1` writes the sidecars from what the engine
//! finds, for review in the diff.

use erc_checks::catalogue::catalogue;
use erc_checks::{Confidence, Report, RunOptions, registered, run_imported};
use erc_model::{DirFiles, ImportOptions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    #[serde(default, rename = "finding")]
    findings: Vec<ExpectFinding>,
    #[serde(default, rename = "not_run")]
    not_run: Vec<ExpectNotRun>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectFinding {
    check: String,
    subjects: Vec<String>,
    #[serde(default = "certain")]
    confidence: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    also: Vec<String>,
}

fn certain() -> String {
    "certain".into()
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectNotRun {
    check: String,
    need: String,
}

fn sources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test_schematics/sources")
        .canonicalize()
        .unwrap()
}

/// What the engine found, in the sidecar's terms
fn actual(report: &Report) -> Expect {
    let mut findings: Vec<ExpectFinding> = report
        .findings
        .iter()
        .chain(&report.candidates)
        .map(|f| ExpectFinding {
            check: f.check.clone(),
            subjects: f.subject_names(),
            confidence: match f.confidence {
                Confidence::Certain => "certain",
                Confidence::Estimate => "estimate",
                Confidence::Candidate => "candidate",
            }
            .into(),
            also: f.also.clone(),
        })
        .collect();
    findings.sort();
    let mut not_run: Vec<ExpectNotRun> = report
        .not_run()
        .map(|n| ExpectNotRun {
            check: n.check.clone(),
            need: n.need.clone(),
        })
        .collect();
    not_run.sort();
    Expect { findings, not_run }
}

fn write_sidecar(path: &Path, name: &str, e: &Expect) {
    let mut out = format!(
        "# What better_erc finds on {name}.kicad_sch (erc_checks/tests/schematics.rs; BLESS=1 rewrites it)\n"
    );
    for f in &e.findings {
        writeln!(out, "\n[[finding]]\ncheck = {:?}", f.check).unwrap();
        writeln!(out, "subjects = {:?}", f.subjects).unwrap();
        if f.confidence != "certain" {
            writeln!(out, "confidence = {:?}", f.confidence).unwrap();
        }
        if !f.also.is_empty() {
            writeln!(out, "also = {:?}", f.also).unwrap();
        }
    }
    for n in &e.not_run {
        writeln!(
            out,
            "\n[[not_run]]\ncheck = {:?}\nneed = {:?}",
            n.check, n.need
        )
        .unwrap();
    }
    std::fs::write(path, out).unwrap();
}

#[test]
fn schematics_match_their_expectations() {
    let dir = sources();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter_map(|n| n.strip_suffix(".kicad_sch").map(str::to_string))
        // The project root that joins all sheets into one project
        .filter(|n| n != "test_schematics")
        .collect();
    names.sort();
    let bless = std::env::var_os("BLESS").is_some();
    let mut failed = Vec::new();
    let mut fired = BTreeSet::new();
    for name in &names {
        let id = name.split('_').next().unwrap().to_uppercase();
        if catalogue().entry(&id).is_none() {
            failed.push(format!(
                "{name}.kicad_sch: name does not start with a catalogue id (`i2c-3_slug`)"
            ));
        }
        let imported = erc_kicad::project::read(
            &DirFiles(dir.clone()),
            &format!("{name}.kicad_sch"),
            &ImportOptions::default(),
        )
        .unwrap();
        let report = run_imported(&imported, &RunOptions::default()).unwrap();
        let got = actual(&report);
        fired.extend(got.findings.iter().map(|f| f.check.clone()));
        let sidecar = dir.join(format!("{name}.expect.toml"));
        if bless {
            write_sidecar(&sidecar, name, &got);
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&sidecar) else {
            failed.push(format!(
                "{name}.kicad_sch: no {name}.expect.toml (BLESS=1 writes it)"
            ));
            continue;
        };
        let mut want: Expect =
            toml::from_str(&text).unwrap_or_else(|e| panic!("{name}.expect.toml: {e}"));
        for f in &mut want.findings {
            f.subjects.sort();
        }
        want.findings.sort();
        want.not_run.sort();
        if want != got {
            let missing: Vec<_> = want
                .findings
                .iter()
                .filter(|f| !got.findings.contains(f))
                .collect();
            let extra: Vec<_> = got
                .findings
                .iter()
                .filter(|f| !want.findings.contains(f))
                .collect();
            failed.push(format!(
                "{name}: expected but not found {missing:#?}\nfound but not expected {extra:#?}\nnot run: expected {:?}, got {:?}\n{}",
                want.not_run,
                got.not_run,
                report.to_text()
            ));
        }
    }
    for e in std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()) {
        let n = e.file_name().to_string_lossy().to_string();
        if let Some(stem) = n.strip_suffix(".expect.toml")
            && !names.iter().any(|x| x == stem)
        {
            failed.push(format!("{n}: no {stem}.kicad_sch"));
        }
    }
    for c in registered() {
        if !fired.contains(c.meta().id) {
            failed.push(format!(
                "{}: no test schematic shows it firing (add test_schematics/sources/{}_<slug>.kicad_sch with expectations)",
                c.meta().id,
                c.meta().id.to_lowercase()
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n\n"));
}
