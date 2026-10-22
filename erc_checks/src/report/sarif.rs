//! The report as SARIF 2.1.0 (OASIS), for GitHub code scanning and other SARIF viewers: one run, a rule per check
//! that ran or found something (id = catalogue id, help = its detail page section), a result per finding at the
//! schematic file and line of its symbols (the design file alone where no line is known), the checks that did not
//! run as notifications. Our own small structs, only the properties used here.

use super::{Report, Status};
use crate::catalogue::{catalogue, detail};
use crate::finding::{BaselineState, Confidence, Finding};
use crate::meta::Severity;
use serde::Serialize;
use std::collections::BTreeMap;

/// The schema URI SARIF files name
pub const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";

#[derive(Serialize)]
pub struct Log {
    #[serde(rename = "$schema")]
    pub schema: &'static str,
    pub version: &'static str,
    pub runs: Vec<Run>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub tool: Tool,
    pub invocations: Vec<Invocation>,
    pub results: Vec<SarifResult>,
}

#[derive(Serialize)]
pub struct Tool {
    pub driver: Driver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    pub name: &'static str,
    pub semantic_version: String,
    pub rules: Vec<Rule>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub short_description: Text,
    pub help: Help,
    pub default_configuration: Configuration,
    pub properties: RuleProperties,
}

#[derive(Serialize)]
pub struct Text {
    pub text: String,
}

#[derive(Serialize)]
pub struct Help {
    pub text: String,
    pub markdown: String,
}

#[derive(Serialize)]
pub struct Configuration {
    pub level: &'static str,
}

#[derive(Serialize)]
pub struct RuleProperties {
    /// The category, for filtering (`I2C`)
    pub tags: Vec<String>,
    pub tier: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invocation {
    pub execution_successful: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_configuration_notifications: Vec<Notification>,
}

#[derive(Serialize)]
pub struct Notification {
    pub descriptor: Descriptor,
    pub level: &'static str,
    pub message: Text,
}

#[derive(Serialize)]
pub struct Descriptor {
    pub id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    pub rule_index: usize,
    /// `review` for a judgement check's candidate, `fail` otherwise
    pub kind: &'static str,
    pub level: &'static str,
    pub message: Text,
    pub locations: Vec<SarifLocation>,
    pub partial_fingerprints: BTreeMap<&'static str, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_state: Option<&'static str>,
    pub properties: ResultProperties,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifLocation {
    pub physical_location: PhysicalLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Text>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Region>,
}

#[derive(Serialize)]
pub struct ArtifactLocation {
    pub uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub start_line: u32,
}

#[derive(Serialize)]
pub struct ResultProperties {
    pub board: String,
    pub confidence: Confidence,
    pub subjects: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// error → error, warning → warning, suggestion and info → note
pub fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Suggestion | Severity::Info => "note",
    }
}

impl Report {
    /// The SARIF log. `uri_base` goes before every file name of the design (the project directory as the SARIF
    /// consumer resolves it: relative to the repository root for GitHub, `hw/board/`); `main` is the design file,
    /// the location of a finding whose subjects have no file of their own.
    pub fn to_sarif_log(&self, uri_base: &str, main: &str) -> Log {
        let cat = catalogue();
        let mut rules: Vec<Rule> = Vec::new();
        let mut index: BTreeMap<String, usize> = BTreeMap::new();
        let mut rule_index = |id: &str, rules: &mut Vec<Rule>| -> usize {
            if let Some(i) = index.get(id) {
                return *i;
            }
            let entry = cat.entry(id);
            let status = self.checks.iter().find(|c| c.id == id);
            let title = entry
                .map(|e| e.title.clone())
                .or_else(|| status.map(|s| s.title.clone()))
                .unwrap_or_default();
            let severity = entry
                .map(|e| e.severity)
                .or_else(|| status.map(|s| s.severity))
                .unwrap_or(Severity::Warning);
            let tier = entry
                .map(|e| e.tier)
                .or_else(|| status.map(|s| s.tier))
                .map_or("easy", |t| t.name());
            let markdown = detail(id).unwrap_or(&title).to_string();
            rules.push(Rule {
                id: id.to_string(),
                short_description: Text {
                    text: title.clone(),
                },
                help: Help {
                    text: format!("{id}: {title}. `berc explain {id}` shows the full explanation."),
                    markdown,
                },
                default_configuration: Configuration {
                    level: level(severity),
                },
                properties: RuleProperties {
                    tags: vec![id.split('-').next().unwrap_or(id).to_string()],
                    tier,
                },
            });
            index.insert(id.to_string(), rules.len() - 1);
            rules.len() - 1
        };
        for c in &self.checks {
            if matches!(c.status, Status::Ran { .. }) {
                rule_index(&c.id, &mut rules);
            }
        }
        let mut results = Vec::new();
        for f in self.findings.iter().chain(&self.candidates) {
            let i = rule_index(&f.check, &mut rules);
            results.push(result(f, i, uri_base, main));
        }
        let notifications = self
            .not_run()
            .map(|n| Notification {
                descriptor: Descriptor {
                    id: n.check.clone(),
                },
                level: "note",
                message: Text {
                    text: format!("{} not run: needs {}: {}", n.check, n.need, n.hint),
                },
            })
            .collect();
        Log {
            schema: SCHEMA,
            version: "2.1.0",
            runs: vec![Run {
                tool: Tool {
                    driver: Driver {
                        name: "better_erc",
                        semantic_version: self
                            .tool
                            .rsplit(' ')
                            .next()
                            .unwrap_or_default()
                            .to_string(),
                        rules,
                    },
                },
                invocations: vec![Invocation {
                    execution_successful: true,
                    tool_configuration_notifications: notifications,
                }],
                results,
            }],
        }
    }

    /// [`Report::to_sarif_log`] as pretty JSON
    pub fn to_sarif(&self, uri_base: &str, main: &str) -> String {
        serde_json::to_string_pretty(&self.to_sarif_log(uri_base, main)).expect("SARIF serialises")
    }
}

fn result(f: &Finding, rule_index: usize, uri_base: &str, main: &str) -> SarifResult {
    let uri = |file: &str| format!("{uri_base}{}", uri_encode(file));
    let mut locations: Vec<SarifLocation> = f
        .locations
        .iter()
        .map(|l| SarifLocation {
            physical_location: PhysicalLocation {
                artifact_location: ArtifactLocation { uri: uri(&l.file) },
                region: l.line.map(|n| Region { start_line: n }),
            },
            message: Some(Text {
                text: l.what.clone(),
            }),
        })
        .collect();
    // Symbols with a line first: the first location is the one a viewer shows
    locations.sort_by_key(|l| l.physical_location.region.is_none());
    if locations.is_empty() {
        locations.push(SarifLocation {
            physical_location: PhysicalLocation {
                artifact_location: ArtifactLocation { uri: uri(main) },
                region: None,
            },
            message: None,
        });
    }
    let subjects: Vec<String> = f.subjects.iter().map(|s| s.to_string()).collect();
    let mut text = f.message.clone();
    if !subjects.is_empty() {
        text = format!("{text} ({})", subjects.join(", "));
    }
    SarifResult {
        rule_id: f.check.clone(),
        rule_index,
        kind: if f.confidence == Confidence::Candidate {
            "review"
        } else {
            "fail"
        },
        level: if f.confidence == Confidence::Candidate {
            "none"
        } else {
            level(f.severity)
        },
        message: Text { text },
        locations,
        partial_fingerprints: BTreeMap::from([("berc/v1", format!("{}:{}", f.board, f.fingerprint))]),
        baseline_state: f.baseline.map(|b| match b {
            BaselineState::New => "new",
            BaselineState::Unchanged => "unchanged",
        }),
        properties: ResultProperties {
            board: f.board.clone(),
            confidence: f.confidence,
            subjects,
            variant: f.variant.clone(),
            facts: f
                .facts
                .iter()
                .map(|x| format!("{} = {} ({})", x.name, x.value, x.source))
                .collect(),
            fix: f.fix.clone(),
        },
    }
}

/// A relative path as a URI reference: everything but unreserved characters and `/` percent-encoded
pub fn uri_encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
