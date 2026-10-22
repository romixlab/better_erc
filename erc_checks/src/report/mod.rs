//! The result of a run: [`Report`], serialisable to JSON with a versioned schema, compared with a baseline report,
//! and rendered as text, Markdown ([`markdown`]) and SARIF 2.1 ([`sarif`]).

pub mod markdown;
pub mod sarif;

use crate::catalogue::catalogue;
use crate::finding::{BaselineState, Confidence, Finding, NotRun};
use crate::meta::{Severity, Tier};
use anstyle::{AnsiColor, Style};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// The delta to a baseline report, when the run was compared with one ([`Report::compare`])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<Delta>,
}

/// What changed against a baseline report: each finding is marked new or unchanged, and the baseline's findings
/// that are gone are kept here
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    /// The baseline's tool line (`better_erc 0.3.8`)
    pub tool: String,
    pub new: usize,
    pub unchanged: usize,
    /// Findings of the baseline this run no longer has
    pub fixed: Vec<Finding>,
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

    /// Compares the findings with a previous report's (architecture 4.4): a finding is new or unchanged by board and
    /// fingerprint (a fingerprint found twice in the baseline matches two findings), the baseline's findings
    /// without a match are fixed. Candidates are not compared: they are lists to judge, not verdicts.
    pub fn compare(&mut self, baseline: &Report) {
        let mut left: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        for f in &baseline.findings {
            *left.entry((&f.board, &f.fingerprint)).or_default() += 1;
        }
        let (mut new, mut unchanged) = (0, 0);
        for f in &mut self.findings {
            let state = match left.get_mut(&(f.board.as_str(), f.fingerprint.as_str())) {
                Some(n) if *n > 0 => {
                    *n -= 1;
                    unchanged += 1;
                    BaselineState::Unchanged
                }
                _ => {
                    new += 1;
                    BaselineState::New
                }
            };
            f.baseline = Some(state);
        }
        let mut fixed = Vec::new();
        for f in baseline.findings.iter().rev() {
            if let Some(n) = left.get_mut(&(f.board.as_str(), f.fingerprint.as_str()))
                && *n > 0
            {
                *n -= 1;
                fixed.push(f.clone());
            }
        }
        fixed.reverse();
        self.baseline = Some(Delta {
            tool: baseline.tool.clone(),
            new,
            unchanged,
            fixed,
        });
    }

    /// The checks that did not run, with why
    pub fn not_run(&self) -> impl Iterator<Item = &NotRun> {
        self.checks.iter().filter_map(|c| match &c.status {
            Status::NotRun(n) => Some(n),
            _ => None,
        })
    }

    /// Plain text: findings worst first and grouped by category (in catalogue order), candidates, the checks
    /// that did not run with what would enable them, notes, the baseline delta, and a summary line
    pub fn to_text(&self) -> String {
        self.text(false)
    }

    /// [`Report::to_text`] with ANSI colours (severity, headings, new findings), for a terminal; a writer like
    /// `anstream` strips them again on a pipe or with `NO_COLOR`
    pub fn to_text_color(&self) -> String {
        self.text(true)
    }

    fn text(&self, color: bool) -> String {
        let st = Styles::new(color);
        let mut out = String::new();
        for b in &self.summary.boards {
            let variant = self
                .summary
                .variant
                .as_ref()
                .map_or(String::new(), |v| format!(", variant {v}"));
            writeln!(
                out,
                "{}{}{} ({}: {} sheets, {} parts, {} nets{}{variant})",
                st.bold.render(),
                b.name,
                st.bold.render_reset(),
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
                writeln!(
                    out,
                    "\n{}{c} - {title}{}",
                    st.bold.render(),
                    st.bold.render_reset()
                )
                .unwrap();
            }
            write_finding(&mut out, f, &st);
        }
        if !self.candidates.is_empty() {
            writeln!(
                out,
                "\n{}Candidates (to judge){}",
                st.bold.render(),
                st.bold.render_reset()
            )
            .unwrap();
            for f in &self.candidates {
                write_finding(&mut out, f, &st);
            }
        }
        if let Some(d) = &self.baseline
            && !d.fixed.is_empty()
        {
            writeln!(
                out,
                "\n{}Fixed since the baseline{}",
                st.bold.render(),
                st.bold.render_reset()
            )
            .unwrap();
            for f in &d.fixed {
                write_finding(&mut out, f, &st);
            }
        }
        let not_run: Vec<&NotRun> = self.not_run().collect();
        if !not_run.is_empty() {
            writeln!(
                out,
                "\n{}Not run{}",
                st.bold.render(),
                st.bold.render_reset()
            )
            .unwrap();
            for n in not_run {
                writeln!(out, "  {:<8} needs {}: {}", n.check, n.need, n.hint).unwrap();
            }
        }
        if !self.notes.is_empty() {
            writeln!(out, "\n{}Notes{}", st.bold.render(), st.bold.render_reset()).unwrap();
            for n in &self.notes {
                writeln!(out, "  {} {}: {}", n.kind, n.at, n.message).unwrap();
            }
        }
        writeln!(out, "\n{}", self.summary_line()).unwrap();
        out
    }

    /// `2 errors, 0 warnings, ... · checks: 11 ran, 0 not run, 502 planned · 3 ms`, with the baseline delta when
    /// there is one
    pub fn summary_line(&self) -> String {
        let s = &self.summary;
        let baseline = self.baseline.as_ref().map_or(String::new(), |d| {
            format!(
                " · baseline: {} new, {} unchanged, {} fixed",
                d.new,
                d.unchanged,
                d.fixed.len()
            )
        });
        format!(
            "{}, {}, {}, {} info, {}{baseline} · checks: {} ran, {} not run, {} planned · {} ms",
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
    }
}

pub(crate) fn plural(n: usize, w: &str) -> String {
    format!("{n} {w}{}", if n == 1 { "" } else { "s" })
}

/// The colours of the text rendering; all plain without colour
struct Styles {
    bold: Style,
    new: Style,
    dim: Style,
    error: Style,
    warning: Style,
    suggestion: Style,
}

impl Styles {
    fn new(color: bool) -> Styles {
        let s = |style: Style| if color { style } else { Style::new() };
        Styles {
            bold: s(Style::new().bold()),
            new: s(AnsiColor::Magenta.on_default().bold()),
            dim: s(Style::new().dimmed()),
            error: s(AnsiColor::Red.on_default().bold()),
            warning: s(AnsiColor::Yellow.on_default().bold()),
            suggestion: s(AnsiColor::Cyan.on_default()),
        }
    }

    fn severity(&self, s: Severity) -> Style {
        match s {
            Severity::Error => self.error,
            Severity::Warning => self.warning,
            Severity::Suggestion => self.suggestion,
            Severity::Info => self.dim,
        }
    }
}

fn write_finding(out: &mut String, f: &Finding, st: &Styles) {
    let estimate = match f.confidence {
        Confidence::Estimate => " (estimate)",
        _ => "",
    };
    let also = if f.also.is_empty() {
        String::new()
    } else {
        format!(" (also {})", f.also.join(", "))
    };
    let new = match f.baseline {
        Some(BaselineState::New) => format!(" {}[new]{}", st.new.render(), st.new.render_reset()),
        _ => String::new(),
    };
    let sev = st.severity(f.severity);
    writeln!(
        out,
        "  {}{:<10}{} {:<8} {}{estimate}{also}{new}",
        sev.render(),
        f.severity.name(),
        sev.render_reset(),
        f.check,
        f.message
    )
    .unwrap();
    let subjects: Vec<String> = f.subjects.iter().map(|s| s.to_string()).collect();
    writeln!(out, "  {:<10} {:<8} {}", "", "", subjects.join(", ")).unwrap();
    for fact in &f.facts {
        writeln!(
            out,
            "  {:<10} {:<8} {}{} = {} ({}){}",
            "",
            "",
            st.dim.render(),
            fact.name,
            fact.value,
            fact.source,
            st.dim.render_reset()
        )
        .unwrap();
    }
    if let Some(fix) = &f.fix {
        writeln!(out, "  {:<10} {:<8} fix: {fix}", "", "").unwrap();
    }
}
