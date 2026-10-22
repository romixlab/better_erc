//! A run: work out what data each board has, run every check whose needs are there (in parallel, over one
//! shared context per board), report the others as not run with what would enable them, fold overlapping
//! findings, order the result.

use crate::catalogue::{catalogue, needs_text, order_key};
use crate::ctx::{Availability, Config, Ctx};
use crate::finding::{Confidence, Finding, NotRun};
use crate::meta::{Check, Need, Scope, Severity, registered};
use crate::report::{BoardSummary, CheckStatus, Note, Report, Status, Summary, Timing};
use erc_model::{BoardId, Design, Imported, NoteKind};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fmt;
use std::time::Instant;

/// What to run
#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    /// The variant to check, by name; `None` for the base design
    pub variant: Option<String>,
    /// Only these check ids or categories (`I2C-3`, `BOM`); empty for all
    pub only: Vec<String>,
    pub config: Config,
}

/// Why a run could not start
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// No board of the design has this variant
    UnknownVariant(String),
    /// Neither a registered check id nor a category
    UnknownCheck(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::UnknownVariant(v) => write!(f, "no variant {v}"),
            RunError::UnknownCheck(c) => write!(f, "no check or category {c}"),
        }
    }
}

impl std::error::Error for RunError {}

/// Runs every registered check on `design`
pub fn run(design: &Design, opts: &RunOptions) -> Result<Report, RunError> {
    run_with(&registered(), design, opts)
}

/// Runs every registered check on an import, with the import's notes in the report
pub fn run_imported(imported: &Imported, opts: &RunOptions) -> Result<Report, RunError> {
    let mut report = run(&imported.design, opts)?;
    let notes = imported.notes.iter().map(|n| Note {
        kind: "import".into(),
        at: format!("{}: {}", n.file, n.at),
        message: format!(
            "{}: {}",
            match n.kind {
                NoteKind::Unknown => "unknown",
                NoteKind::Missing => "missing",
                NoteKind::Invalid => "invalid",
                NoteKind::Ignored => "ignored",
            },
            n.message
        ),
    });
    report.notes.splice(0..0, notes);
    Ok(report)
}

/// The outcome of one check over all boards
struct Outcome {
    ran: bool,
    findings: Vec<Finding>,
    micros: u64,
    not_run: Option<NotRun>,
}

/// Runs `checks` on `design`
pub fn run_with(
    checks: &[&'static dyn Check],
    design: &Design,
    opts: &RunOptions,
) -> Result<Report, RunError> {
    let start = Instant::now();
    let selected = select(checks, opts)?;
    let variants: Vec<Option<erc_model::VariantId>> = design
        .boards
        .iter()
        .map(|b| opts.variant.as_ref().and_then(|v| b.variant_by_name(v)))
        .collect();
    if let Some(v) = &opts.variant
        && variants.iter().all(Option::is_none)
    {
        return Err(RunError::UnknownVariant(v.clone()));
    }

    let mut outcomes: BTreeMap<&'static str, Outcome> = selected
        .iter()
        .map(|c| {
            (
                c.meta().id,
                Outcome {
                    ran: false,
                    findings: Vec::new(),
                    micros: 0,
                    not_run: None,
                },
            )
        })
        .collect();
    let mut notes = Vec::new();
    let checks_start = Instant::now();
    for (bi, board) in design.boards.iter().enumerate() {
        let board_id = BoardId::from_index(bi);
        // A system check runs once, on the first board's context
        let here: Vec<&&'static dyn Check> = selected
            .iter()
            .filter(|c| c.meta().scope == Scope::Board || bi == 0)
            .collect();
        let mut avail: BTreeMap<String, Availability> = BTreeMap::new();
        let mut runnable = Vec::new();
        for c in here {
            let mut missing = None;
            for need in c.meta().needs {
                let a = avail
                    .entry(need.name())
                    .or_insert_with(|| Availability::of(*need, design, board))
                    .clone();
                if let Availability::Missing { why } = a {
                    missing = Some((*need, why));
                    break;
                }
            }
            match missing {
                Some((need, why)) => {
                    let o = outcomes.get_mut(c.meta().id).expect("selected");
                    o.not_run
                        .get_or_insert_with(|| not_run(c.meta().id, need, &why));
                }
                None => runnable.push(*c),
            }
        }
        for (need, a) in &avail {
            if let Availability::Partial { have, of, what } = a {
                notes.push(Note {
                    kind: "coverage".into(),
                    at: board.name.clone(),
                    message: format!(
                        "{need}: {have} of {of} parts have {what}; checks ran on those"
                    ),
                });
            }
        }
        let cx = Ctx::new(design, board_id, variants[bi], &opts.config);
        let results: Vec<(&'static str, Vec<Finding>, u64)> = runnable
            .par_iter()
            .map(|c| {
                let t = Instant::now();
                let f = c.run(&cx);
                (c.meta().id, f, t.elapsed().as_micros() as u64)
            })
            .collect();
        for (id, findings, micros) in results {
            let o = outcomes.get_mut(id).expect("selected");
            o.ran = true;
            o.findings.extend(findings);
            o.micros += micros;
        }
        notes.extend(cx.notes().into_iter().map(|m| Note {
            kind: "analysis".into(),
            at: board.name.clone(),
            message: m,
        }));
    }
    let checks_micros = checks_start.elapsed().as_micros() as u64;

    let mut all: Vec<Finding> = Vec::new();
    let mut statuses = Vec::new();
    let mut status_of = |o: &mut Outcome| match o.not_run.take() {
        Some(n) if !o.ran => Status::NotRun(n),
        _ => {
            all.append(&mut o.findings);
            Status::Ran {
                findings: 0,
                micros: o.micros,
            }
        }
    };
    for e in &catalogue().entries {
        let status = match outcomes.remove(e.id.as_str()) {
            Some(mut o) => status_of(&mut o),
            // Registered, not selected for this run
            None if checks.iter().any(|c| c.meta().id == e.id) => continue,
            None => Status::Planned,
        };
        statuses.push(CheckStatus {
            id: e.id.clone(),
            title: e.title.clone(),
            severity: e.severity,
            tier: e.tier,
            status,
        });
    }
    // Checks outside the catalogue (the catalogue sync test keeps the registry free of them; tests run their own)
    for (id, mut o) in outcomes {
        let meta = checks
            .iter()
            .find(|c| c.meta().id == id)
            .expect("selected")
            .meta();
        statuses.push(CheckStatus {
            id: id.to_string(),
            title: String::new(),
            severity: meta.severity,
            tier: meta.tier,
            status: status_of(&mut o),
        });
    }
    let all = fold_overlaps(checks, all);
    for s in &mut statuses {
        if let Status::Ran { findings, .. } = &mut s.status {
            *findings = all.iter().filter(|f| f.check == s.id).count();
        }
    }
    let (mut candidates, mut findings): (Vec<Finding>, Vec<Finding>) = all
        .into_iter()
        .partition(|f| f.confidence == Confidence::Candidate);
    sort(&mut findings);
    sort(&mut candidates);

    let count = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let summary = Summary {
        boards: design
            .boards
            .iter()
            .map(|b| BoardSummary {
                name: b.name.clone(),
                tool: b.source.tool.clone(),
                sheets: b.sheets.len(),
                parts: (0..b.parts.len())
                    .filter(|i| b.is_component(erc_model::PartId::from_index(*i)))
                    .count(),
                nets: b.nets.len(),
                variants: b.variants.iter().map(|v| v.name.clone()).collect(),
                layout: b.layout.is_some(),
            })
            .collect(),
        variant: opts.variant.clone(),
        errors: count(Severity::Error),
        warnings: count(Severity::Warning),
        suggestions: count(Severity::Suggestion),
        infos: count(Severity::Info),
        candidates: candidates.len(),
        ran: statuses
            .iter()
            .filter(|s| matches!(s.status, Status::Ran { .. }))
            .count(),
        not_run: statuses
            .iter()
            .filter(|s| matches!(s.status, Status::NotRun(_)))
            .count(),
        planned: statuses
            .iter()
            .filter(|s| matches!(s.status, Status::Planned))
            .count(),
    };
    Ok(Report {
        schema: crate::report::SCHEMA_VERSION,
        tool: format!("better_erc {}", env!("CARGO_PKG_VERSION")),
        summary,
        checks: statuses,
        findings,
        candidates,
        notes,
        timing: Timing {
            checks_micros,
            total_micros: start.elapsed().as_micros() as u64,
        },
        baseline: None,
    })
}

/// The checks `opts` selects: all but the disabled ones, or the ids and categories of `only`
fn select(
    checks: &[&'static dyn Check],
    opts: &RunOptions,
) -> Result<Vec<&'static dyn Check>, RunError> {
    for o in &opts.only {
        let known = checks
            .iter()
            .any(|c| c.meta().id == o || c.meta().category == o)
            || catalogue().categories.iter().any(|c| &c.id == o);
        if !known {
            return Err(RunError::UnknownCheck(o.clone()));
        }
    }
    Ok(checks
        .iter()
        .copied()
        .filter(|c| !opts.config.disabled.contains(c.meta().id))
        .filter(|c| {
            opts.only.is_empty()
                || opts
                    .only
                    .iter()
                    .any(|o| o == c.meta().id || o == c.meta().category)
        })
        .collect())
}

fn not_run(id: &str, need: Need, why: &str) -> NotRun {
    let hint = match needs_text(id) {
        Some(n) => format!("{why} (the check needs: {n})"),
        None => why.to_string(),
    };
    NotRun {
        check: id.to_string(),
        need: need.name(),
        hint,
    }
}

/// A finding of a check that overlaps another (`≈`) supersedes that one's finding on the same subjects: the other
/// is dropped and named in `also`
fn fold_overlaps(checks: &[&'static dyn Check], mut all: Vec<Finding>) -> Vec<Finding> {
    let overlaps = |id: &str| -> &'static [&'static str] {
        checks
            .iter()
            .find(|c| c.meta().id == id)
            .map_or(&[], |c| c.meta().overlaps)
    };
    let keys: Vec<(String, Vec<String>)> = all
        .iter()
        .map(|f| (f.check.clone(), f.subject_names()))
        .collect();
    let mut drop = vec![false; all.len()];
    for (i, (check, subjects)) in keys.iter().enumerate() {
        for other in overlaps(check) {
            for (j, (c2, s2)) in keys.iter().enumerate() {
                if !drop[j] && c2 == other && s2 == subjects && all[i].board == all[j].board {
                    drop[j] = true;
                    if !all[i].also.iter().any(|a| a == other) {
                        all[i].also.push(other.to_string());
                    }
                }
            }
        }
    }
    all.into_iter()
        .zip(drop)
        .filter(|(_, d)| !d)
        .map(|(f, _)| f)
        .collect()
}

/// Worst first, then catalogue order (category, number), then the fingerprint (DIAG-4)
fn sort(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| order_key(&a.check).cmp(&order_key(&b.check)))
            .then_with(|| a.board.cmp(&b.board))
            .then_with(|| a.fingerprint.cmp(&b.fingerprint))
    });
}
