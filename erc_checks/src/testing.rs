//! Running one check on a test schematic, for the checks' unit tests

use crate::{Check, RunOptions, run_with};
use erc_model::{DirFiles, ImportOptions};
use std::path::Path;

/// The fingerprints of what `check` finds on `test_schematics/sources/<schematic>.kicad_sch`, sorted
pub fn fingerprints(check: &'static dyn Check, schematic: &str) -> Vec<String> {
    fingerprints_with(check, schematic, &RunOptions::default())
}

/// [`fingerprints`] with these options
pub fn fingerprints_with(
    check: &'static dyn Check,
    schematic: &str,
    opts: &RunOptions,
) -> Vec<String> {
    let report = report_with(check, schematic, opts);
    let mut f: Vec<String> = report
        .findings
        .iter()
        .chain(&report.candidates)
        .map(|f| f.fingerprint.clone())
        .collect();
    f.sort();
    f
}

/// The directory of a test schematic: `test_schematics/sources/<schematic>/` for a case with a project of its own,
/// else `test_schematics/sources`
fn case_dir(schematic: &str) -> std::path::PathBuf {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
    let own = sources.join(schematic);
    if own.join(format!("{schematic}.kicad_sch")).is_file() {
        own
    } else {
        sources
    }
}

/// The report of `check` alone on the test schematic `<schematic>.kicad_sch` with these options
pub fn report_with(check: &'static dyn Check, schematic: &str, opts: &RunOptions) -> crate::Report {
    let imported = erc_kicad::project::read(
        &DirFiles(case_dir(schematic)),
        &format!("{schematic}.kicad_sch"),
        &ImportOptions::default(),
    )
    .unwrap();
    run_with(&[check], &imported.design, opts).unwrap()
}

/// The fingerprints of what `check` finds on a schematic given as its text, sorted
pub fn fingerprints_of(check: &'static dyn Check, schematic: &str) -> Vec<String> {
    fingerprints_of_with(check, schematic, &RunOptions::default())
}

/// [`fingerprints_of`] with these options
pub fn fingerprints_of_with(
    check: &'static dyn Check,
    schematic: &str,
    opts: &RunOptions,
) -> Vec<String> {
    let files = erc_model::MemFiles::default().with("sheet.kicad_sch", schematic);
    let imported =
        erc_kicad::project::read(&files, "sheet.kicad_sch", &ImportOptions::default()).unwrap();
    let report = run_with(&[check], &imported.design, opts).unwrap();
    let mut f: Vec<String> = report
        .findings
        .iter()
        .chain(&report.candidates)
        .map(|f| f.fingerprint.clone())
        .collect();
    f.sort();
    f
}

/// The options of a run with the parts snapshot beside the test schematic (`<schematic>.parts.lock.toml`)
pub fn with_lock(schematic: &str) -> RunOptions {
    let path = case_dir(schematic).join(format!("{schematic}.parts.lock.toml"));
    RunOptions {
        parts: Some(std::sync::Arc::new(
            erc_parts::PartsLock::read(&path).unwrap(),
        )),
        ..RunOptions::default()
    }
}
