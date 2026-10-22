//! better_erc as a library: one dependency for the whole core.
//!
//! - [`model`] ([`erc_model`]): the board model, values, facts with provenance, the importer trait, the canonical
//!   dump, the netlist and placement comparisons.
//! - [`kicad`] ([`erc_kicad`]): KiCad schematics, boards, project files and netlists, the s-expression reader.
//! - [`import`] ([`erc_import`]): the importer registry with KiCad, Altium, OrCAD and pick-and-place files.
//! - [`analysis`] ([`erc_analysis`]): part classes, values, graph queries, the power structure, I2C buses.
//! - [`checks`] ([`erc_checks`]): the check engine, the catalogue, findings and reports (JSON, SARIF, Markdown).
//! - [`diff`]: comparisons of two designs on disk, the library side of `berc diff`.
//!
//! The run entry is re-exported at the top: [`Project::open`] reads a design from a path (a directory with one
//! design, or a design file) through the registry, [`Project::check`] runs the checks into a [`CheckRun`] with its
//! [`Report`] and [`Verdict`]; [`check_path`] does both.
//!
//! ```no_run
//! use better_erc::{CheckOptions, Project};
//!
//! # fn main() -> Result<(), better_erc::CheckError> {
//! let project = Project::open(std::path::Path::new("hw/board"))?;
//! let run = project.check(&CheckOptions::default())?;
//! for f in &run.report.findings {
//!     println!("{} {}", f.check, f.message);
//! }
//! # Ok(())
//! # }
//! ```

pub use erc_analysis as analysis;
pub use erc_checks as checks;
pub use erc_import as import;
pub use erc_kicad as kicad;
pub use erc_model as model;

pub mod diff;

pub use erc_checks::{
    Availability, BaselineState, CheckError, CheckInfo, CheckOptions, CheckRun, CheckState,
    Confidence, Delta, Exit, Explanation, Finding, FindingFact, Format, ListedCheck, Location,
    Need, NotRun, Project, Report, RunOptions, RunState, Severity, Stage, Status, SubjectRef, Tier,
    Unavailable, Verdict, check_path, explain, list, list_for, read_baseline,
};
pub use erc_model::{Board, Design};
