//! Symbol fields moved clear of what they overlap, the library side of `berc place-fields` (`TOOL-13`): the plan
//! of [`erc_checks::place`] on every board of a design, written into the schematic files by
//! [`erc_kicad::fields`], and the `DRAW-9` and `DRAW-34` counts before and after.

use erc_checks::place::{
    FieldPlan, OverlapCount, StyleError, count_overlaps, plan_default_hides, plan_fields,
};
use erc_checks::{CheckError, Config, Ctx, Project};
use erc_kicad::fields::{FieldChange, FieldPlace, PlaceError, place_fields};
use erc_model::{BoardId, Design};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub use erc_checks::place::{FieldMove, Remaining, Side, SymbolPlan};

/// What to place
#[derive(Clone, Debug, Default)]
pub struct PlaceOptions {
    /// Only this sheet: its name in the parent sheet or its file, with or without `.kicad_sch`
    pub sheet: Option<String>,
    /// Write the moved fields into the files; else only plan them
    pub write: bool,
    /// The settings the margins (`drawing.clearance.*`) and the field style (`drawing.fields.*`) come from
    pub config: Config,
}

/// The fields placed, and what `DRAW-9` and `DRAW-34` find before and after
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub plan: FieldPlan,
    pub before: OverlapCount,
    /// With the moves made: read again from the written files, or from the edited text in a dry run
    pub after: OverlapCount,
    /// The files written; empty in a dry run, and with `--write` when nothing needed a change
    pub written: Vec<PathBuf>,
    /// Whether the run was to write ([`PlaceOptions::write`]), not a dry run
    pub write: bool,
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
    /// A `drawing.fields.*` setting is out of its range
    Style(StyleError),
}

impl fmt::Display for PlaceFieldsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaceFieldsError::Open(e) => e.fmt(f),
            PlaceFieldsError::Io { file, error } => write!(f, "{}: {error}", file.display()),
            PlaceFieldsError::Edit { file, error } => write!(f, "{}: {error}", file.display()),
            PlaceFieldsError::NoSheet(s) => write!(f, "no sheet {s}"),
            PlaceFieldsError::Style(e) => e.fmt(f),
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
            PlaceFieldsError::Style(e) => Some(e),
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
    // drawing.fields.hide_defaults = always: the default ratings go first, then the placer sees the design without them
    let hides = plan_all(design, &opts.config, sheet, plan_default_hides)?;
    let hidden = edit(&project.dir, &hides, &BTreeMap::new())?;
    let hidden_project = if hidden.is_empty() {
        None
    } else {
        Some(
            project
                .reopen_with(&hidden)
                .map_err(PlaceFieldsError::Open)?,
        )
    };
    let placing = hidden_project
        .as_ref()
        .map_or(design, |p| &p.imported.design);
    let moves = plan_all(placing, &opts.config, sheet, plan_fields)?;
    let mut edited = hidden;
    let moved = edit(&project.dir, &moves, &edited)?;
    edited.extend(moved);
    let plan = merge(hides, moves);
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
        write: opts.write,
    })
}

/// The plan `planner` makes for every board, in one
fn plan_all(
    design: &Design,
    config: &Config,
    sheet: Option<&str>,
    planner: fn(&Ctx, Option<&str>) -> Result<FieldPlan, StyleError>,
) -> Result<FieldPlan, PlaceFieldsError> {
    let mut plan = FieldPlan::default();
    for bi in 0..design.boards.len() {
        let cx = Ctx::new(design, BoardId::from_index(bi), None, config);
        let board = planner(&cx, sheet).map_err(PlaceFieldsError::Style)?;
        plan.symbols.extend(board.symbols);
    }
    Ok(plan)
}

/// The hides and the moves after them as one plan: a symbol in both takes the moves' plan, with both its hides
fn merge(mut hides: FieldPlan, moves: FieldPlan) -> FieldPlan {
    for s in moves.symbols {
        let same = |h: &&mut SymbolPlan| h.file == s.file && h.uuid == s.uuid;
        match hides.symbols.iter_mut().find(same) {
            Some(h) => {
                let mut hidden = std::mem::take(&mut h.hidden);
                hidden.extend(s.hidden.iter().cloned());
                *h = SymbolPlan { hidden, ..s };
            }
            None => hides.symbols.push(s),
        }
    }
    hides
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

/// The text of every sheet file with a moved field, edited, by file relative to `dir`: from its text in `base`
/// when an earlier edit made one, else from the file
fn edit(
    dir: &Path,
    plan: &FieldPlan,
    base: &BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, PlaceFieldsError> {
    let mut by_file: BTreeMap<&str, Vec<FieldPlace>> = BTreeMap::new();
    for s in &plan.symbols {
        let moves = s.moves.iter().map(|m| {
            let change = FieldChange::Move {
                at: m.at,
                halign: m.halign,
            };
            (m.field.clone(), change)
        });
        let hides = s.hidden.iter().map(|h| (h.clone(), FieldChange::Hide));
        for (field, change) in moves.chain(hides) {
            by_file.entry(&s.file).or_default().push(FieldPlace {
                symbol: s.uuid.clone(),
                field,
                change,
            });
        }
    }
    let mut out = BTreeMap::new();
    for (file, places) in by_file {
        let path = dir.join(file);
        let text = match base.get(file) {
            Some(b) => {
                String::from_utf8(b.clone()).expect("an earlier edit wrote this text from a String")
            }
            None => std::fs::read_to_string(&path).map_err(|error| PlaceFieldsError::Io {
                file: path.clone(),
                error,
            })?,
        };
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
        if !s.hidden.is_empty() {
            out += &format!("  hides: {}\n", s.hidden.join(", "));
        }
        for r in &s.left {
            let (what, id) = if r.close {
                ("is too close to", "DRAW-34")
            } else {
                ("overlaps", "DRAW-9")
            };
            out += &format!("  still: {} {what} {} ({id})\n", r.field, r.other);
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
    if p.write && p.written.is_empty() {
        out += "Nothing to move: no file changed.\n";
    } else if !p.write {
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
