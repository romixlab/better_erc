//! Running one check on a test schematic, for the checks' unit tests

use crate::{Check, RunOptions, run_with};
use erc_model::{DirFiles, ImportOptions};
use std::path::Path;

/// The fingerprints of what `check` finds on `test_schematics/sources/<schematic>.kicad_sch`, sorted
pub fn fingerprints(check: &'static dyn Check, schematic: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
    let imported = erc_kicad::project::read(
        &DirFiles(dir),
        &format!("{schematic}.kicad_sch"),
        &ImportOptions::default(),
    )
    .unwrap();
    let report = run_with(&[check], &imported.design, &RunOptions::default()).unwrap();
    let mut f: Vec<String> = report
        .findings
        .iter()
        .chain(&report.candidates)
        .map(|f| f.fingerprint.clone())
        .collect();
    f.sort();
    f
}
