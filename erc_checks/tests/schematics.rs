//! The test schematics are the spec: every `test_schematics/sources/<id>_<slug>.kicad_sch` has a sidecar
//! `<id>_<slug>.expect.toml` listing the findings the engine must report on it (check, subjects, confidence, the
//! checks it folds in), compared exactly in both directions, and, written by hand where a page says so, checks
//! that must not run on it with the need they lack, compared as a subset (other checks not running is fine, so a
//! new check does not rewrite every sidecar). This test imports each schematic, runs every registered check, and
//! compares; a schematic without expectations, expectations without a schematic, or a registered check that no
//! expectation shows firing fails it. `BLESS=1` rewrites the findings from what the engine finds, for review in
//! the diff, and keeps the sidecar's `[[not_run]]` blocks as they are (it never adds any). `DRAW` findings are
//! compared on the `draw-*` schematics only: the others were written for their circuit, not drawn for style. A
//! `<id>_<slug>.theme.json` beside a schematic is the colour theme of its run, a `<id>_<slug>.parts.lock.toml` its
//! parts snapshot. A case that needs a KiCad project of its own (text variables) lives in a directory
//! `<id>_<slug>/` holding `<id>_<slug>.kicad_sch`, its `.kicad_pro` and its sidecars, so the sources directory
//! keeps one project.

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

/// Every test case by name with the directory it is in: each `<name>.kicad_sch` of `sources` but the project root,
/// and each directory `<name>/` holding `<name>.kicad_sch`; sorted by name
fn cases(sources: &Path) -> Vec<(String, PathBuf)> {
    let mut cases: Vec<(String, PathBuf)> = std::fs::read_dir(sources)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_str()?.to_owned();
            let p = e.path();
            if p.is_dir() {
                p.join(format!("{n}.kicad_sch")).is_file().then_some((n, p))
            } else {
                let stem = n.strip_suffix(".kicad_sch")?;
                // The project root that joins all sheets into one project
                (stem != "test_schematics").then(|| (stem.to_owned(), sources.to_path_buf()))
            }
        })
        .collect();
    cases.sort();
    cases
}

#[test]
fn schematics_match_their_expectations() {
    let sources = sources();
    let cases = cases(&sources);
    let bless = std::env::var_os("BLESS").is_some();
    let mut failed = Vec::new();
    let mut fired = BTreeSet::new();
    for (name, dir) in &cases {
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
        let mut opts = RunOptions::default();
        if let Ok(text) = std::fs::read_to_string(dir.join(format!("{name}.theme.json"))) {
            opts.config
                .themes
                .push(erc_kicad::theme::read(&text).unwrap());
        }
        if let Ok(text) = std::fs::read_to_string(dir.join(format!("{name}.parts.lock.toml"))) {
            let lock =
                erc_parts::PartsLock::parse(&format!("{name}.parts.lock.toml"), &text).unwrap();
            opts.parts = Some(std::sync::Arc::new(lock));
        }
        let report = run_imported(&imported, &opts).unwrap();
        let mut got = actual(&report);
        if !name.starts_with("draw-") {
            got.findings.retain(|f| !f.check.starts_with("DRAW-"));
        }
        fired.extend(got.findings.iter().map(|f| f.check.clone()));
        let sidecar = dir.join(format!("{name}.expect.toml"));
        let existing = std::fs::read_to_string(&sidecar).ok();
        let read = |text: &str| -> Expect {
            toml::from_str(text).unwrap_or_else(|e| panic!("{name}.expect.toml: {e}"))
        };
        if bless {
            let kept = existing.as_deref().map(read).unwrap_or_default().not_run;
            write_sidecar(
                &sidecar,
                name,
                &Expect {
                    findings: got.findings,
                    not_run: kept,
                },
            );
            continue;
        }
        let Some(text) = existing else {
            failed.push(format!(
                "{name}.kicad_sch: no {name}.expect.toml (BLESS=1 writes it)"
            ));
            continue;
        };
        let mut want = read(&text);
        for f in &mut want.findings {
            f.subjects.sort();
        }
        want.findings.sort();
        want.not_run.sort();
        let not_run_missing: Vec<_> = want
            .not_run
            .iter()
            .filter(|n| !got.not_run.contains(n))
            .collect();
        if want.findings != got.findings || !not_run_missing.is_empty() {
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
                "{name}: expected but not found {missing:#?}\nfound but not expected {extra:#?}\nexpected not to run, but ran or lacked another need: {not_run_missing:?}\n{}",
                report.to_text()
            ));
        }
    }
    let case_dirs: Vec<PathBuf> = std::iter::once(sources.clone())
        .chain(cases.iter().map(|(_, d)| d.clone()))
        .collect();
    for d in case_dirs.iter().collect::<BTreeSet<_>>() {
        for e in std::fs::read_dir(d).unwrap().filter_map(|e| e.ok()) {
            let n = e.file_name().to_string_lossy().to_string();
            if let Some(stem) = n.strip_suffix(".expect.toml")
                && !cases.iter().any(|(x, xd)| x == stem && xd == d)
            {
                failed.push(format!("{n}: no {stem}.kicad_sch beside it"));
            }
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
