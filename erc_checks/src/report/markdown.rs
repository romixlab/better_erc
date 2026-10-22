//! The report as Markdown for a pull request comment: a headline with the counts, the baseline delta, a table of
//! the findings worst first, and the fixed findings, candidates, not-run checks and notes folded away.

use super::{Report, plural};
use crate::finding::{BaselineState, Confidence, Finding};
use crate::meta::Severity;
use std::fmt::Write;

impl Report {
    /// GitHub-flavoured Markdown, for a PR comment
    pub fn to_markdown(&self) -> String {
        let s = &self.summary;
        let mut out = String::new();
        let headline = if s.errors + s.warnings + s.suggestions + s.infos == 0 {
            "no findings".to_string()
        } else {
            format!(
                "{}, {}, {}",
                plural(s.errors, "error"),
                plural(s.warnings, "warning"),
                plural(s.suggestions, "suggestion")
            )
        };
        writeln!(out, "### better_erc: {headline}\n").unwrap();
        for b in &s.boards {
            let variant = s
                .variant
                .as_ref()
                .map_or(String::new(), |v| format!(", variant `{v}`"));
            writeln!(
                out,
                "**{}** ({}: {} sheets, {} parts, {} nets{}{variant})  ",
                cell(&b.name),
                b.tool,
                b.sheets,
                b.parts,
                b.nets,
                if b.layout { ", layout" } else { "" }
            )
            .unwrap();
        }
        if let Some(d) = &self.baseline {
            writeln!(
                out,
                "Against the baseline ({}): **{} new**, {} unchanged, {} fixed",
                d.tool,
                d.new,
                d.unchanged,
                d.fixed.len()
            )
            .unwrap();
        }
        if !self.findings.is_empty() {
            out.push('\n');
            table(&mut out, &self.findings);
        }
        if let Some(d) = &self.baseline
            && !d.fixed.is_empty()
        {
            details(&mut out, &format!("Fixed since the baseline ({})", d.fixed.len()), |o| {
                table(o, &d.fixed)
            });
        }
        if !self.candidates.is_empty() {
            details(
                &mut out,
                &format!("Candidates to judge ({})", self.candidates.len()),
                |o| table(o, &self.candidates),
            );
        }
        let not_run: Vec<_> = self.not_run().collect();
        if !not_run.is_empty() {
            details(&mut out, &format!("Not run ({})", not_run.len()), |o| {
                for n in &not_run {
                    writeln!(o, "- `{}` needs {}: {}", n.check, n.need, n.hint).unwrap();
                }
            });
        }
        if !self.notes.is_empty() {
            details(&mut out, &format!("Notes ({})", self.notes.len()), |o| {
                for n in &self.notes {
                    writeln!(o, "- {} {}: {}", n.kind, n.at, n.message).unwrap();
                }
            });
        }
        writeln!(
            out,
            "\n<sub>{} · checks: {} ran, {} not run, {} planned · `berc explain ID` for any check</sub>",
            self.tool, s.ran, s.not_run, s.planned
        )
        .unwrap();
        out
    }
}

fn table(out: &mut String, findings: &[Finding]) {
    writeln!(out, "| | Check | Finding | Subjects |").unwrap();
    writeln!(out, "|---|---|---|---|").unwrap();
    for f in findings {
        let mut message = cell(&f.message);
        if f.confidence == Confidence::Estimate {
            message += " *(estimate)*";
        }
        if !f.also.is_empty() {
            write!(message, " (also {})", f.also.join(", ")).unwrap();
        }
        if f.baseline == Some(BaselineState::New) {
            message += " **new**";
        }
        if let Some(fix) = &f.fix {
            write!(message, "<br>Fix: {}", cell(fix)).unwrap();
        }
        let subjects: Vec<String> = f.subjects.iter().map(|s| s.to_string()).collect();
        writeln!(
            out,
            "| {} | `{}` | {message} | {} |",
            emoji(f),
            f.check,
            cell(&subjects.join(", "))
        )
        .unwrap();
    }
}

fn details(out: &mut String, summary: &str, body: impl FnOnce(&mut String)) {
    writeln!(out, "\n<details><summary>{summary}</summary>\n").unwrap();
    body(out);
    writeln!(out, "\n</details>").unwrap();
}

/// The catalogue's severity marks
fn emoji(f: &Finding) -> &'static str {
    if f.confidence == Confidence::Candidate {
        return "🔎";
    }
    match f.severity {
        Severity::Error => "🛑",
        Severity::Warning => "⚠️",
        Severity::Suggestion => "💡",
        Severity::Info => "ℹ️",
    }
}

/// Text safe in a table cell: one line, `|` and HTML escaped
fn cell(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "\\|")
        .replace('\n', " ")
}
