//! The result of a run: [`Report`], serialisable to JSON with a versioned schema, and its text rendering.

use crate::catalogue::catalogue;
use crate::finding::{Confidence, Finding, NotRun};
use crate::meta::{Severity, Tier};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// The version of the JSON schema of [`Report`]; bumped on any incompatible change
pub const SCHEMA_VERSION: u32 = 1;

/// Everything a run found and did
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// [`SCHEMA_VERSION`]
    pub schema: u32,
    /// `better_erc 0.4.0`
    pub tool: String,
    pub summary: Summary,
    /// Every catalogue check: ran, not run (why) or planned, in catalogue order
    pub checks: Vec<CheckStatus>,
    /// Findings of the rules (certain or estimate), worst first, by category
    pub findings: Vec<Finding>,
    /// Candidates of the judgement checks, for a human or an LLM
    pub candidates: Vec<Finding>,
    /// Import notes, analysis notes, coverage notes
    pub notes: Vec<Note>,
    pub timing: Timing,
}

/// The design and the counts
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub boards: Vec<BoardSummary>,
    /// The variant checked, `None` for the base design
    pub variant: Option<String>,
    pub errors: usize,
    pub warnings: usize,
    pub suggestions: usize,
    pub infos: usize,
    pub candidates: usize,
    pub ran: usize,
    pub not_run: usize,
    pub planned: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardSummary {
    pub name: String,
    /// `kicad`, `kicad-netlist`, `altium`, `orcad`
    pub tool: String,
    pub sheets: usize,
    /// Real components (no power symbols)
    pub parts: usize,
    pub nets: usize,
    pub variants: Vec<String>,
    pub layout: bool,
}

/// What happened to one catalogue check
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckStatus {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub tier: Tier,
    #[serde(flatten)]
    pub status: Status,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum Status {
    /// It ran on every board: its finding count and time
    Ran { findings: usize, micros: u64 },
    /// A need was missing
    NotRun(NotRun),
    /// In the catalogue, no code yet
    Planned,
}

/// Something the reader should know that is not a finding
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// `import`, `analysis`, `coverage`
    pub kind: String,
    /// Where (a file and element for import notes, a board for the others)
    pub at: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    /// The checks, wall clock (they run in parallel)
    pub checks_micros: u64,
    /// The whole run
    pub total_micros: u64,
}

impl Report {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a report serialises")
    }

    pub fn from_json(text: &str) -> Result<Report, serde_json::Error> {
        serde_json::from_str(text)
    }

    /// The checks that did not run, with why
    pub fn not_run(&self) -> impl Iterator<Item = &NotRun> {
        self.checks.iter().filter_map(|c| match &c.status {
            Status::NotRun(n) => Some(n),
            _ => None,
        })
    }

    /// Plain text: findings worst first and grouped by category (in catalogue order), candidates, the checks
    /// that did not run with what would enable them, notes, and a summary line
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for b in &self.summary.boards {
            let variant = self
                .summary
                .variant
                .as_ref()
                .map_or(String::new(), |v| format!(", variant {v}"));
            writeln!(
                out,
                "{} ({}: {} sheets, {} parts, {} nets{}{variant})",
                b.name,
                b.tool,
                b.sheets,
                b.parts,
                b.nets,
                if b.layout { ", layout" } else { "" }
            )
            .unwrap();
        }
        let cat = catalogue();
        let mut category = None;
        for f in &self.findings {
            let c = f.check.split('-').next().unwrap_or("");
            if category != Some(c) {
                category = Some(c);
                let title = cat
                    .categories
                    .iter()
                    .find(|x| x.id == c)
                    .map_or(c, |x| x.title.as_str());
                writeln!(out, "\n{c} - {title}").unwrap();
            }
            write_finding(&mut out, f);
        }
        if !self.candidates.is_empty() {
            writeln!(out, "\nCandidates (to judge)").unwrap();
            for f in &self.candidates {
                write_finding(&mut out, f);
            }
        }
        let not_run: Vec<&NotRun> = self.not_run().collect();
        if !not_run.is_empty() {
            writeln!(out, "\nNot run").unwrap();
            for n in not_run {
                writeln!(out, "  {:<8} needs {}: {}", n.check, n.need, n.hint).unwrap();
            }
        }
        if !self.notes.is_empty() {
            writeln!(out, "\nNotes").unwrap();
            for n in &self.notes {
                writeln!(out, "  {} {}: {}", n.kind, n.at, n.message).unwrap();
            }
        }
        let s = &self.summary;
        let plural = |n: usize, w: &str| format!("{n} {w}{}", if n == 1 { "" } else { "s" });
        writeln!(
            out,
            "\n{}, {}, {}, {} info, {} · checks: {} ran, {} not run, {} planned · {} ms",
            plural(s.errors, "error"),
            plural(s.warnings, "warning"),
            plural(s.suggestions, "suggestion"),
            s.infos,
            plural(s.candidates, "candidate"),
            s.ran,
            s.not_run,
            s.planned,
            self.timing.total_micros / 1000
        )
        .unwrap();
        out
    }
}

fn write_finding(out: &mut String, f: &Finding) {
    let estimate = match f.confidence {
        Confidence::Estimate => " (estimate)",
        _ => "",
    };
    let also = if f.also.is_empty() {
        String::new()
    } else {
        format!(" (also {})", f.also.join(", "))
    };
    writeln!(
        out,
        "  {:<10} {:<8} {}{estimate}{also}",
        f.severity.name(),
        f.check,
        f.message
    )
    .unwrap();
    let subjects: Vec<String> = f.subjects.iter().map(|s| s.to_string()).collect();
    writeln!(out, "  {:<10} {:<8} {}", "", "", subjects.join(", ")).unwrap();
    for fact in &f.facts {
        writeln!(
            out,
            "  {:<10} {:<8} {} = {} ({})",
            "", "", fact.name, fact.value, fact.source
        )
        .unwrap();
    }
    if let Some(fix) = &f.fix {
        writeln!(out, "  {:<10} {:<8} fix: {fix}", "", "").unwrap();
    }
}
