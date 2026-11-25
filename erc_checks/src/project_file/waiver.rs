//! `[[waiver]]`: a finding accepted for this project, keyed by its fingerprint, optionally pinned to the hash of
//! the inputs that were judged. Applied after the run: a waived finding stays in the report, marked, and does not
//! fail the run; a waiver whose input hash no longer matches, or whose expiry has passed, is not applied and says
//! why.

use crate::finding::Finding;
use crate::meta::Severity;
use crate::report::Report;
use erc_model::Time;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// One `[[waiver]]` entry
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiver {
    /// The finding's fingerprint (`I2C-1:net /SDA|part R5`)
    pub fingerprint: String,
    /// Only on this board, when the design has several
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board: Option<String>,
    /// The finding's input hash when it was judged (`in1:...`); a different hash now makes the waiver stale
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_hash: Option<String>,
    pub who: String,
    pub date: Time,
    pub reason: String,
    /// The day after which it no longer applies
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<Time>,
}

/// What became of a waiver on one finding
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum WaiverStatus {
    /// The finding is waived
    Applied,
    /// The inputs judged changed: the finding stands
    Stale {
        /// The hash the waiver holds
        was: String,
        /// The finding's hash now, `None` when the check no longer gives one
        now: Option<String>,
    },
    /// Past its expiry: the finding stands
    Expired,
}

/// A waiver as it met a finding, on the finding
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingWaiver {
    #[serde(flatten)]
    pub status: WaiverStatus,
    pub who: String,
    pub date: Time,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<Time>,
}

/// A waiver of the project file and what it met, on the report
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaiverOutcome {
    pub waiver: Waiver,
    /// What it did to the findings it matched; `None` when it matched none (the finding is fixed, or its
    /// subjects changed)
    pub status: Option<WaiverStatus>,
    /// The findings it matched
    pub findings: usize,
}

impl Waiver {
    /// Whether it is for this finding
    fn matches(&self, f: &Finding) -> bool {
        self.fingerprint == f.fingerprint && self.board.as_ref().is_none_or(|b| *b == f.board)
    }

    /// What it does to a finding it matches, on `today`
    fn judge(&self, f: &Finding, today: Time) -> WaiverStatus {
        if let Some(was) = &self.input_hash
            && f.input_hash.as_ref() != Some(was)
        {
            return WaiverStatus::Stale {
                was: was.clone(),
                now: f.input_hash.clone(),
            };
        }
        match self.expires {
            // An expiry date covers that whole day
            Some(e) if today.0 >= e.0 + 86_400 => WaiverStatus::Expired,
            _ => WaiverStatus::Applied,
        }
    }
}

impl FindingWaiver {
    /// Whether the finding is waived
    pub fn applied(&self) -> bool {
        self.status == WaiverStatus::Applied
    }
}

/// Applies the waivers to the report's findings and candidates, on `today`: marks each finding a waiver matches,
/// takes the waived ones out of the severity counts into `summary.waived`, and lists every waiver with what it met
pub fn apply(report: &mut Report, waivers: &[Waiver], today: Time) {
    let mut outcomes: Vec<WaiverOutcome> = waivers
        .iter()
        .map(|w| WaiverOutcome {
            waiver: w.clone(),
            status: None,
            findings: 0,
        })
        .collect();
    let mut waived = Vec::new();
    for f in report
        .findings
        .iter_mut()
        .chain(report.candidates.iter_mut())
    {
        // The first waiver that applies wins; else the first that matched, to say why it did not
        let mut chosen: Option<(usize, WaiverStatus)> = None;
        for (i, w) in waivers.iter().enumerate() {
            if !w.matches(f) {
                continue;
            }
            let status = w.judge(f, today);
            outcomes[i].findings += 1;
            if outcomes[i].status != Some(WaiverStatus::Applied) {
                outcomes[i].status = Some(status.clone());
            }
            let better = status == WaiverStatus::Applied
                && chosen
                    .as_ref()
                    .is_none_or(|(_, s)| *s != WaiverStatus::Applied);
            if chosen.is_none() || better {
                chosen = Some((i, status));
            }
        }
        if let Some((i, status)) = chosen {
            let w = &waivers[i];
            if status == WaiverStatus::Applied
                && f.confidence != crate::finding::Confidence::Candidate
            {
                waived.push(f.severity);
            }
            f.waiver = Some(FindingWaiver {
                status,
                who: w.who.clone(),
                date: w.date,
                reason: w.reason.clone(),
                expires: w.expires,
            });
        }
    }
    let s = &mut report.summary;
    for sev in waived {
        let n = match sev {
            Severity::Error => &mut s.errors,
            Severity::Warning => &mut s.warnings,
            Severity::Suggestion => &mut s.suggestions,
            Severity::Info => &mut s.infos,
        };
        *n = n.saturating_sub(1);
        s.waived += 1;
    }
    report.waivers = outcomes;
}

/// A day as `2026-11-25`
pub fn day(t: Time) -> String {
    let mut s = t.to_string();
    s.truncate(10);
    s
}

/// The `[[waiver]]` entry for `w` as TOML text, a blank line first, for appending to a project file
pub fn entry_text(w: &Waiver) -> String {
    let q = |s: &str| toml::Value::String(s.to_owned()).to_string();
    let mut out = String::from("\n[[waiver]]\n");
    writeln!(out, "fingerprint = {}", q(&w.fingerprint)).expect("writes to a String");
    if let Some(b) = &w.board {
        writeln!(out, "board = {}", q(b)).expect("writes to a String");
    }
    if let Some(h) = &w.input_hash {
        writeln!(out, "input_hash = {}", q(h)).expect("writes to a String");
    }
    writeln!(out, "who = {}", q(&w.who)).expect("writes to a String");
    writeln!(out, "date = {}", day(w.date)).expect("writes to a String");
    writeln!(out, "reason = {}", q(&w.reason)).expect("writes to a String");
    if let Some(e) = w.expires {
        writeln!(out, "expires = {}", day(e)).expect("writes to a String");
    }
    out
}
