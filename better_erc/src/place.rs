//! Symbol fields moved clear of what they overlap, the library side of `berc place-fields` (`TOOL-13`): the plan
//! of [`erc_checks::place`] on every board of a design, written into the schematic files by
//! [`erc_kicad::fields`], and the `DRAW-9` and `DRAW-34` counts before and after.

use erc_checks::place::{FieldPlan, OverlapCount, count_overlaps, plan_fields};
use erc_checks::{CheckError, Config, Ctx, Project};
use erc_kicad::fields::{FieldPlace, PlaceError, place_fields};
use erc_model::{BoardId, Design};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub use erc_checks::place::{FieldMove, Side, SymbolPlan};

/// What to place
#[derive(Clone, Debug, Default)]
pub struct PlaceOptions {
    /// Only this sheet: its name in the parent sheet or its file, with or without `.kicad_sch`
    pub sheet: Option<String>,
    /// Write the moved fields into the files; else only plan them
    pub write: bool,
    /// The settings the margins come from (`drawing.clearance.*`)
    pub config: Config,
}

/// The fields placed, and what `DRAW-9` and `DRAW-34` find before and after
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub plan: FieldPlan,
    pub before: OverlapCount,
    /// With the moves made: read again from the written files, or from the edited text in a dry run
    pub after: OverlapCount,
    /// The files written; empty in a dry run
    pub written: Vec<PathBuf>,
}

/// Why fields could not be placed
#[derive(Debug)]
pub enum PlaceFieldsError {
    /// The design could not be read, before or after the edit
    Open(CheckError),
    /// A sheet file could not be read or written
    Io {
        file: PathBuf,
        error: std::io::Error,
    },
    /// A sheet file could not be edited
    Edit { file: PathBuf, error: PlaceError },
    /// `--sheet` names no sheet of the design
    NoSheet(String),
}

impl fmt::Display for PlaceFieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaceFieldsError::Open(e) => e.fmt(f),
            PlaceFieldsError::Io { file, error } => write!(f, "{}: {error}", file.display()),
            PlaceFieldsError::Edit { file, error } => write!(f, "{}: {error}", file.display()),
            PlaceFieldsError::NoSheet(s) => write!(f, "no sheet {s}"),
        }
    }
}

impl std::error::Error for PlaceFieldsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PlaceFieldsError::Open(e) => Some(e),
            PlaceFieldsError::Io { error, .. } => Some(error),
            PlaceFieldsError::Edit { error, .. } => Some(error),
            PlaceFieldsError::NoSheet(_) => None,
        }
    }
}

/// Plans the fields of the design at `path` (a directory or a `.kicad_pro`, as [`Project::open`] takes it) and,
/// with [`PlaceOptions::write`], writes them. Nothing is written when any file fails to edit.
pub fn place(path: &Path, opts: &PlaceOptions) -> Result<Placed, PlaceFieldsError> {
    let project = Project::open(path).map_err(PlaceFieldsError::Open)?;
    let design = &project.imported.design;
    if let Some(sheet) = &opts.sheet {
        let known = design.boards.iter().any(|b| {
            b.sheets.iter().any(|s| {
                let stem = s.file.strip_suffix(".kicad_sch").unwrap_or(&s.file);
                s.name == *sheet || s.file == *sheet || stem == sheet
            })
        });
        if !known {
            return Err(PlaceFieldsError::NoSheet(sheet.clone()));
        }
    }
    let sheet = opts.sheet.as_deref();
    let before = count(design, &opts.config, sheet);
    let mut plan = FieldPlan::default();
    for bi in 0..design.boards.len() {
        let cx = Ctx::new(design, BoardId::from_index(bi), None, &opts.config);
        plan.symbols.extend(plan_fields(&cx, sheet).symbols);
    }
    let edited = edit(&project.dir, &plan)?;
    let after_project = project
        .reopen_with(&edited)
        .map_err(PlaceFieldsError::Open)?;
    let after = count(&after_project.imported.design, &opts.config, sheet);
    let mut written = Vec::new();
    if opts.write {
        for (file, text) in &edited {
            let path = project.dir.join(file);
            std::fs::write(&path, text).map_err(|error| PlaceFieldsError::Io {
                file: path.clone(),
                error,
            })?;
            written.push(path);
        }
    }
    Ok(Placed {
        plan,
        before,
        after,
        written,
    })
}

/// The counts summed over every board
fn count(design: &Design, config: &Config, sheet: Option<&str>) -> OverlapCount {
    let mut total = OverlapCount::default();
    for bi in 0..design.boards.len() {
        let cx = Ctx::new(design, BoardId::from_index(bi), None, config);
        let c = count_overlaps(&cx, sheet);
        total.draw_9 += c.draw_9;
        total.draw_34 += c.draw_34;
        total.draw_9_fields += c.draw_9_fields;
        total.draw_34_fields += c.draw_34_fields;
    }
    total
}

/// The text of every sheet file with a moved field, edited, by file relative to `dir`
fn edit(dir: &Path, plan: &FieldPlan) -> Result<BTreeMap<String, Vec<u8>>, PlaceFieldsError> {
    let mut by_file: BTreeMap<&str, Vec<FieldPlace>> = BTreeMap::new();
    for s in &plan.symbols {
        for m in &s.moves {
            by_file.entry(&s.file).or_default().push(FieldPlace {
                symbol: s.uuid.clone(),
                field: m.field.clone(),
                at: m.at,
                halign: m.halign,
            });
        }
    }
    let mut out = BTreeMap::new();
    for (file, places) in by_file {
        let path = dir.join(file);
        let text = std::fs::read_to_string(&path).map_err(|error| PlaceFieldsError::Io {
            file: path.clone(),
            error,
        })?;
        let edited = place_fields(&text, &places)
            .map_err(|error| PlaceFieldsError::Edit { file: path, error })?;
        out.insert(file.to_owned(), edited.into_bytes());
    }
    Ok(out)
}

/// The plan as text: each symbol with the side its fields go to and what they still overlap, then the counts
pub fn render(p: &Placed) -> String {
    let mut out = String::new();
    let mut moved = 0;
    for s in &p.plan.symbols {
        let what = match s.side {
            Some(side) => {
                moved += 1;
                format!("fields {}", side.name())
            }
            None => "fields stay: no side is clearer".to_owned(),
        };
        out += &format!("{} ({}): {what}\n", s.reference, s.file);
        for (a, b) in &s.left {
            out += &format!("  still: {a} and {b}\n");
        }
    }
    let line = |id: &str, before: usize, after: usize, bf: usize, af: usize| {
        format!("{id}: {before} -> {after} (with a field: {bf} -> {af})\n")
    };
    let (b, a) = (p.before, p.after);
    out += &line(
        "DRAW-9",
        b.draw_9,
        a.draw_9,
        b.draw_9_fields,
        a.draw_9_fields,
    );
    out += &line(
        "DRAW-34",
        b.draw_34,
        a.draw_34,
        b.draw_34_fields,
        a.draw_34_fields,
    );
    let files: std::collections::BTreeSet<&str> = p
        .plan
        .symbols
        .iter()
        .filter(|s| s.side.is_some())
        .map(|s| s.file.as_str())
        .collect();
    if p.written.is_empty() {
        out += &format!(
            "Dry run: {moved} symbols in {} files to move; --write edits them.\n",
            files.len()
        );
    } else {
        out += &format!(
            "Moved the fields of {moved} symbols in {} files.\n",
            p.written.len()
        );
    }
    out
}
