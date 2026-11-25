//! The display fields `Voltage`, `Power`, `Dielectric` and `Package` written from the parts snapshot, the library
//! side of `berc fields sync` (`TOOL-14`): the plan of [`erc_checks::fields_sync`] on every board of a design,
//! written into the schematic files by [`erc_kicad::fields`], and the new fields placed as `berc place-fields`
//! places fields.

use crate::place::{PlaceFieldsError, edit};
use erc_checks::fields_sync::{FieldOp, FieldSync, Skipped, plan_sync};
use erc_checks::place::{FieldPlan, plan_fields};
use erc_checks::{CheckError, Config, Ctx, Project};
use erc_kicad::fields::{PlaceError, ValueChange, ValueEdit, edit_values};
use erc_model::BoardId;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

pub use erc_checks::fields_sync::FieldOp as Op;

/// What to sync
#[derive(Clone, Debug, Default)]
pub struct SyncOptions {
    /// Write the files; else only plan
    pub write: bool,
    /// The settings the placement of new fields takes (`drawing.*`), over the project file's
    pub config: Config,
}

/// The fields synced
#[derive(Clone, Debug, PartialEq)]
pub struct Synced {
    pub fields: Vec<FieldSync>,
    /// Fields left as they are: the facts of the MPNs do not settle them
    pub skipped: Vec<Skipped>,
    /// The symbols that got a new field, which are placed again, by reference
    pub placed: Vec<String>,
    /// The files written; empty in a dry run, and with `--write` when nothing changed
    pub written: Vec<PathBuf>,
    pub write: bool,
}

/// Why the fields could not be synced
#[derive(Debug)]
pub enum SyncError {
    /// The design could not be read, before or after the edit
    Open(CheckError),
    /// The design has no parts snapshot
    NoParts,
    Io {
        file: PathBuf,
        error: std::io::Error,
    },
    Edit {
        file: PathBuf,
        error: PlaceError,
    },
    /// The new fields could not be placed
    Place(PlaceFieldsError),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Open(e) => e.fmt(f),
            SyncError::NoParts => f.write_str(
                "no parts snapshot beside the project: `berc parts refresh` writes parts.lock.toml",
            ),
            SyncError::Io { file, error } => write!(f, "{}: {error}", file.display()),
            SyncError::Edit { file, error } => write!(f, "{}: {error}", file.display()),
            SyncError::Place(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SyncError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SyncError::Open(e) => Some(e),
            SyncError::Io { error, .. } => Some(error),
            SyncError::Edit { error, .. } => Some(error),
            SyncError::Place(e) => Some(e),
            SyncError::NoParts => None,
        }
    }
}

/// Plans the display fields of the design at `path` (a directory or a `.kicad_pro`) from its parts snapshot and,
/// with [`SyncOptions::write`], writes them. Nothing is written when any file fails to edit.
pub fn sync(path: &Path, opts: &SyncOptions) -> Result<Synced, SyncError> {
    let project = Project::open(path).map_err(SyncError::Open)?;
    let lock = project.parts.clone().ok_or(SyncError::NoParts)?;
    let design = &project.imported.design;
    let config = project.config(&opts.config);
    let mut fields = Vec::new();
    let mut skipped = Vec::new();
    for bi in 0..design.boards.len() {
        let cx = Ctx::new(design, BoardId::from_index(bi), None, &config).with_parts(Some(&lock));
        let plan = plan_sync(&cx);
        for f in plan.fields {
            if !fields.contains(&f) {
                fields.push(f);
            }
        }
        skipped.extend(plan.skipped);
    }
    let mut by_file: BTreeMap<&str, Vec<ValueEdit>> = BTreeMap::new();
    for f in &fields {
        let change = match &f.op {
            FieldOp::Set(v) | FieldOp::Add(v) => ValueChange::Put(v.clone()),
            FieldOp::Remove => ValueChange::Remove,
        };
        by_file.entry(&f.file).or_default().push(ValueEdit {
            symbol: f.uuid.clone(),
            field: f.field.to_owned(),
            change,
        });
    }
    let mut edited: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for (file, edits) in by_file {
        let path = project.dir.join(file);
        let text = std::fs::read_to_string(&path).map_err(|error| SyncError::Io {
            file: path.clone(),
            error,
        })?;
        let out =
            edit_values(&text, &edits).map_err(|error| SyncError::Edit { file: path, error })?;
        edited.insert(file.to_owned(), out.into_bytes());
    }
    // A symbol with a new field has its fields placed again, as `berc place-fields` would
    let added: BTreeSet<(&str, &str)> = fields
        .iter()
        .filter(|f| matches!(f.op, FieldOp::Add(_)))
        .map(|f| (f.file.as_str(), f.uuid.as_str()))
        .collect();
    let mut placed = Vec::new();
    if !added.is_empty() {
        let after = project.reopen_with(&edited).map_err(SyncError::Open)?;
        let design = &after.imported.design;
        let mut plan = FieldPlan::default();
        for bi in 0..design.boards.len() {
            let cx = Ctx::new(design, BoardId::from_index(bi), None, &config);
            let board =
                plan_fields(&cx, None).map_err(|e| SyncError::Place(PlaceFieldsError::Style(e)))?;
            plan.symbols.extend(
                board
                    .symbols
                    .into_iter()
                    .filter(|s| added.contains(&(s.file.as_str(), s.uuid.as_str()))),
            );
        }
        let moved = edit(&project.dir, &plan, &edited).map_err(SyncError::Place)?;
        edited.extend(moved);
        placed = plan.symbols.iter().map(|s| s.reference.clone()).collect();
    }
    let mut written = Vec::new();
    if opts.write {
        for (file, text) in &edited {
            let path = project.dir.join(file);
            std::fs::write(&path, text).map_err(|error| SyncError::Io {
                file: path.clone(),
                error,
            })?;
            written.push(path);
        }
    }
    Ok(Synced {
        fields,
        skipped,
        placed,
        written,
        write: opts.write,
    })
}

/// The plan as text: one line per field, the fields left alone with why, then the totals
pub fn render(s: &Synced) -> String {
    let mut out = String::new();
    for f in &s.fields {
        let line = match (&f.op, &f.old) {
            (FieldOp::Set(v), Some(o)) => format!("{} {}: {o} -> {v}", f.reference, f.field),
            (FieldOp::Set(v) | FieldOp::Add(v), _) => {
                format!("{} {}: add {v}", f.reference, f.field)
            }
            (FieldOp::Remove, o) => format!(
                "{} {}: remove {} (the board default)",
                f.reference,
                f.field,
                o.as_deref().unwrap_or("")
            ),
        };
        out += &line;
        out += "\n";
    }
    for k in &s.skipped {
        out += &format!("{} {}: left alone, {}\n", k.reference, k.field, k.why);
    }
    let count = |pick: fn(&FieldOp) -> bool| s.fields.iter().filter(|f| pick(&f.op)).count();
    let (set, add, remove) = (
        count(|o| matches!(o, FieldOp::Set(_))),
        count(|o| matches!(o, FieldOp::Add(_))),
        count(|o| matches!(o, FieldOp::Remove)),
    );
    let files: BTreeSet<&str> = s.fields.iter().map(|f| f.file.as_str()).collect();
    let totals = format!(
        "{set} fields set, {add} added, {remove} removed in {} files; {} left alone",
        files.len(),
        s.skipped.len()
    );
    if s.fields.is_empty() {
        out += &format!(
            "Nothing to write: every display field states its MPNs; {} left alone.\n",
            s.skipped.len()
        );
    } else if s.write {
        out += &format!("Wrote {totals}.\n");
    } else {
        out += &format!("Dry run: {totals}; --write edits the files.\n");
    }
    out
}
