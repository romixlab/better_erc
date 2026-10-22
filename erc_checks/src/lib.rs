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
//!
//! [`project`] (cargo feature `import`, on by default) is the entry for a caller with a path: it opens the design
//! through the importer registry, runs the checks, compares with a baseline and decides the exit code as data. The
//! report renders as JSON, text, SARIF 2.1 and Markdown ([`report`]).

pub mod catalogue;
pub mod ctx;
pub mod engine;
pub mod finding;
pub mod meta;
#[cfg(feature = "import")]
pub mod project;
pub mod report;

mod ana;
mod bom;
mod clk;
mod dbg;
mod dec;
mod drv;
mod gnd;
mod i2c;
mod io;
mod leak;
#[path = "lib/mod.rs"]
mod library;
mod logic;
mod lvl;
mod mux;
mod name;
mod pin;
mod rail;
mod seq;
mod short;
mod test;
#[cfg(test)]
mod testing;
mod text;
mod wire;

pub use catalogue::{
    CheckInfo, CheckState, Explanation, ListedCheck, RunState, explain, list, list_for,
};
pub use ctx::{Availability, Config, Ctx};
pub use engine::{RunError, RunOptions, run, run_imported, run_with};
pub use finding::{BaselineState, Confidence, Finding, FindingFact, Location, NotRun, SubjectRef};
pub use meta::{
    Check, CheckMeta, Need, Param, Registered, Scope, Severity, Tier, check, registered,
};
#[cfg(feature = "import")]
pub use project::{
    CheckError, CheckOptions, CheckRun, Exit, Format, Project, Stage, Unavailable, Verdict,
    check_path, read_baseline,
};
pub use report::{Delta, Report, Status};
