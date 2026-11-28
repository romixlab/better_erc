//! better_erc for C++ hosts: a cxx bridge that runs the checks on an editor's open schematic.
//!
//! [`check`] opens a design file with some of its files replaced by the editor's texts
//! ([`erc_checks::Project::open_edited`]), runs every registered check with the project's settings and returns
//! the findings no waiver covers as [`ffi::Violation`]s. An error or a panic reaches C++ as a `rust::Error`.

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use erc_checks::{CheckError, CheckOptions, Finding, Project};

#[cxx::bridge(namespace = "berc")]
mod ffi {
    /// A file as the editor holds it, by path relative to the project's directory (`power.kicad_sch`)
    struct EditedFile {
        path: String,
        text: String,
    }

    /// How bad a violation is, as the check catalogue (or the project's override) rates it
    enum Severity {
        /// The board will not work or gets damaged
        Error,
        /// A risk or a margin that depends on the design
        Warning,
        /// Debug aids, tidiness, conventions
        Suggestion,
        /// Facts worth knowing, nothing to fix
        Info,
    }

    /// What a violation is about: `part R1`, `pin R1.2`, `net /I2C/SDA`, `sheet /MCU/`
    struct Subject {
        kind: String,
        name: String,
    }

    /// Where to look: the file relative to the project's directory, what is there, the line (0: unknown) and the
    /// position in millimetres when `has_pos`
    struct Location {
        file: String,
        what: String,
        line: u32,
        has_pos: bool,
        x_mm: f64,
        y_mm: f64,
    }

    /// One finding of a check
    struct Violation {
        /// The catalogue id (`I2C-1`)
        check: String,
        severity: Severity,
        message: String,
        /// How to fix it, empty when the check gives no advice
        fix: String,
        /// The catalogue section that explains the check (`docs/checks/i2c.md#i2c-1`)
        explain: String,
        /// Stable across edits that keep the subjects' names: what waivers and baselines key on
        fingerprint: String,
        subjects: Vec<Subject>,
        locations: Vec<Location>,
    }

    extern "Rust" {
        /// Runs every check on the design file `project` with `edited` in place of those files
        fn check(project: &str, edited: &[EditedFile]) -> Result<Vec<Violation>>;

        /// better_erc's version (`0.6.8`)
        fn version() -> String;
    }
}

/// Why a [`check`] produced no violations
#[derive(Debug)]
enum BridgeError {
    /// Opening or checking the design failed
    Check(CheckError),
    /// A check panicked; the message is the panic's
    Panic(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeError::Check(e) => write!(f, "{e}"),
            BridgeError::Panic(m) => write!(f, "better_erc panicked: {m}"),
        }
    }
}

impl std::error::Error for BridgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BridgeError::Check(e) => Some(e),
            BridgeError::Panic(_) => None,
        }
    }
}

fn check(project: &str, edited: &[ffi::EditedFile]) -> Result<Vec<ffi::Violation>, BridgeError> {
    let edited: BTreeMap<String, Vec<u8>> = edited
        .iter()
        .map(|f| (f.path.clone(), f.text.clone().into_bytes()))
        .collect();
    let run = panic::catch_unwind(AssertUnwindSafe(|| {
        let project = Project::open_edited(Path::new(project), &edited)?;
        project.check(&CheckOptions::default())
    }))
    .map_err(|payload| BridgeError::Panic(panic_message(payload.as_ref())))?
    .map_err(BridgeError::Check)?;
    Ok(run
        .report
        .findings
        .iter()
        .filter(|f| !f.waived())
        .map(violation)
        .collect())
}

fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

fn violation(f: &Finding) -> ffi::Violation {
    ffi::Violation {
        check: f.check.clone(),
        severity: match f.severity {
            erc_checks::Severity::Error => ffi::Severity::Error,
            erc_checks::Severity::Warning => ffi::Severity::Warning,
            erc_checks::Severity::Suggestion => ffi::Severity::Suggestion,
            erc_checks::Severity::Info => ffi::Severity::Info,
        },
        message: f.message.clone(),
        fix: f.fix.clone().unwrap_or_default(),
        explain: f.explain.clone(),
        fingerprint: f.fingerprint.clone(),
        subjects: f
            .subjects
            .iter()
            .map(|s| ffi::Subject {
                kind: s.kind.clone(),
                name: s.name.clone(),
            })
            .collect(),
        locations: f
            .locations
            .iter()
            .map(|l| ffi::Location {
                file: l.file.clone(),
                what: l.what.clone(),
                line: l.line.unwrap_or(0),
                has_pos: l.at.is_some(),
                x_mm: l.at.map_or(0.0, |(x, _)| x),
                y_mm: l.at.map_or(0.0, |(_, y)| y),
            })
            .collect(),
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "no message".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../test_schematics/sources");

    fn ids(v: &[ffi::Violation]) -> Vec<String> {
        let mut out: Vec<String> = v.iter().map(|v| v.check.clone()).collect();
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn checks_the_file_on_disk() {
        let main = format!("{SOURCES}/ana-3_shunt_filters.kicad_sch");
        let found = check(&main, &[]).unwrap();
        let ids = ids(&found);
        assert!(ids.contains(&"ANA-3".to_owned()), "{ids:?}");
        let ana3 = found.iter().find(|v| v.check == "ANA-3").unwrap();
        let names: Vec<&str> = ana3.subjects.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["R1", "U1"]);
    }

    #[test]
    fn edited_text_replaces_the_file() {
        let main = format!("{SOURCES}/ana-3_shunt_filters.kicad_sch");
        let other =
            std::fs::read_to_string(format!("{SOURCES}/ana-10_opamp_feedback.kicad_sch")).unwrap();
        let edited = vec![ffi::EditedFile {
            path: "ana-3_shunt_filters.kicad_sch".to_owned(),
            text: other,
        }];
        let ids = ids(&check(&main, &edited).unwrap());
        assert!(!ids.contains(&"ANA-3".to_owned()), "{ids:?}");
        assert!(ids.contains(&"ANA-10".to_owned()), "{ids:?}");
    }

    #[test]
    fn missing_project_is_an_error() {
        let result = check("/nonexistent/board.kicad_sch", &[]);
        assert!(matches!(result, Err(BridgeError::Check(_))));
    }
}
