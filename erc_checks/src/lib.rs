//! The better_erc check engine.
//!
//! A check is one file under `src/<category>/` implementing [`Check`]: its [`CheckMeta`] (catalogue id,
//! severity, tier, scope, the data it [`Need`]s, the checks it overlaps, its thresholds) and a pure `run` over a
//! [`Ctx`], which holds the design and builds the analyses (part classes, I2C buses) on first use. Checks
//! register themselves in a static registry; [`run`] works out per board which needs are available, runs the
//! checks that can run in parallel, reports the others as [`NotRun`] with what would enable them, folds
//! overlapping findings and returns a [`Report`] (JSON with a versioned schema, or text).
//!
//! The check catalogue (`docs/checks.md`) and its detail pages are embedded at build time: [`explain`] and the
//! not-run hints work offline, [`list`] gives every catalogue check as implemented or planned, and a test holds
//! every registered check's id, severity and tier to its catalogue line.

pub mod catalogue;
pub mod ctx;
pub mod engine;
pub mod finding;
pub mod meta;
pub mod report;

mod bom;
mod i2c;
#[path = "lib/mod.rs"]
mod library;
#[cfg(test)]
mod testing;

pub use catalogue::{CheckInfo, CheckState, Explanation, explain, list};
pub use ctx::{Availability, Config, Ctx};
pub use engine::{RunError, RunOptions, run, run_imported, run_with};
pub use finding::{Confidence, Finding, FindingFact, Location, NotRun, SubjectRef};
pub use meta::{
    Check, CheckMeta, Need, Param, Registered, Scope, Severity, Tier, check, registered,
};
pub use report::{Report, Status};
