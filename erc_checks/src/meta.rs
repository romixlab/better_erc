//! What a check is: the [`Check`] trait, its [`CheckMeta`] (catalogue id, severity, tier, scope, declared data
//! [`Need`]s, overlaps, thresholds) and the static registry every check file adds itself to.

use crate::ctx::Ctx;
use crate::finding::Finding;
use serde::{Deserialize, Serialize};
use std::fmt;

/// How bad a finding is: the catalogue's three levels (🛑 ⚠️ 💡) plus info, worst first
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The board will not work or gets damaged
    Error,
    /// A risk or a margin that depends on the design
    Warning,
    /// Debug aids, tidiness, conventions
    Suggestion,
    /// Facts worth knowing, nothing to fix
    Info,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Suggestion => "suggestion",
            Severity::Info => "info",
        }
    }

    pub fn from_name(name: &str) -> Option<Severity> {
        Some(match name {
            "error" => Severity::Error,
            "warning" => Severity::Warning,
            "suggestion" => Severity::Suggestion,
            "info" => Severity::Info,
            _ => return None,
        })
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How hard a check is to implement, as the catalogue's `impl:` says
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// The netlist alone
    Easy,
    /// Part data or a project setting first
    Mid,
    /// Analysis such as power states, signal following, layout geometry
    Hard,
    /// Not a rule: a human or an LLM judges candidates
    Judgement,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Easy => "easy",
            Tier::Mid => "mid",
            Tier::Hard => "hard",
            Tier::Judgement => "judgement",
        }
    }

    pub fn from_name(name: &str) -> Option<Tier> {
        Some(match name {
            "easy" => Tier::Easy,
            "mid" => Tier::Mid,
            "hard" => Tier::Hard,
            "judgement" => Tier::Judgement,
            _ => return None,
        })
    }
}

/// What a check looks at: one board, or the system of boards joined through connectors
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Board,
    System,
}

/// A piece of data a check needs: the explicit data contract. Before a run the engine works out per board which
/// needs are available ([`crate::Availability`]); a check with a missing need is reported as not run, with what
/// would enable it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Need {
    /// Parts, pins and nets
    Graph,
    /// What kind each part is (resistor, MOSFET, connector)
    PartClasses,
    /// The Value fields
    Values,
    /// Power rails and grounds (NET-5)
    Rails,
    /// Rail voltages with their sources (NET-5)
    RailVoltages,
    /// What each pin is for (NET-7)
    PinRoles,
    /// Input thresholds and output levels of pins (NET-7)
    PinThresholds,
    /// Alternate and reset-time pin functions (NET-12)
    PinFunctions,
    /// Power states from the project file (NET-8)
    PowerStates,
    /// Signal following through parts (NET-10)
    Conduction,
    /// The board file
    Layout,
    /// The schematic drawing: labels, no-connect flags, sheet pins
    Drawing,
    /// Part data from the part database or mx3 (CFG-5)
    PartDb,
    /// A key of the better_erc project file (CFG-2)
    Project(&'static str),
    /// Several boards joined through connectors (NET-13)
    MultiBoard,
}

impl Need {
    /// A short name (`layout`, `project:nets.voltage`)
    pub fn name(self) -> String {
        match self {
            Need::Graph => "graph".into(),
            Need::PartClasses => "part classes".into(),
            Need::Values => "values".into(),
            Need::Rails => "rails".into(),
            Need::RailVoltages => "rail voltages".into(),
            Need::PinRoles => "pin roles".into(),
            Need::PinThresholds => "pin thresholds".into(),
            Need::PinFunctions => "pin functions".into(),
            Need::PowerStates => "power states".into(),
            Need::Conduction => "signal following".into(),
            Need::Layout => "layout".into(),
            Need::Drawing => "schematic drawing".into(),
            Need::PartDb => "part data".into(),
            Need::Project(key) => format!("project file: {key}"),
            Need::MultiBoard => "multi-board".into(),
        }
    }
}

/// A threshold of a check with its default; the project file will override it per project (CFG-2)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub default: f64,
    /// `ohm`, `V`, `A`, or empty for a plain number
    pub unit: &'static str,
    pub doc: &'static str,
}

/// Everything about a check but its code. Id, severity and tier must match the catalogue line (a test asserts it).
#[derive(Debug)]
pub struct CheckMeta {
    /// The catalogue id (`I2C-3`)
    pub id: &'static str,
    /// The catalogue category (`I2C`)
    pub category: &'static str,
    /// The catalogue's severity; a project can override it
    pub severity: Severity,
    pub tier: Tier,
    pub scope: Scope,
    pub needs: &'static [Need],
    /// Checks this one supersedes on the same subjects (the catalogue's `≈`): their finding is folded into this
    /// one's as "also ..."
    pub overlaps: &'static [&'static str],
    pub params: &'static [Param],
}

impl CheckMeta {
    /// The default of a threshold
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    /// The detail page section explaining the check: `docs/checks/i2c.md#i2c-3`
    pub fn doc_link(&self) -> String {
        doc_link(self.id)
    }
}

/// `docs/checks/i2c.md#i2c-3` for `I2C-3`
pub fn doc_link(id: &str) -> String {
    let category = id.split('-').next().unwrap_or(id).to_lowercase();
    format!("docs/checks/{category}.md#{}", id.to_lowercase())
}

/// A check: a pure function from the context (the design and its analyses) to findings
pub trait Check: Sync {
    fn meta(&self) -> &'static CheckMeta;

    /// The findings on the context's board; only called when every need is available
    fn run(&self, cx: &Ctx) -> Vec<Finding>;
}

/// A check in the static registry; each check file submits one with `inventory::submit!`
pub struct Registered(pub &'static dyn Check);

inventory::collect!(Registered);

/// Every registered check, in catalogue order (category order of the catalogue, then number)
pub fn registered() -> Vec<&'static dyn Check> {
    let mut checks: Vec<&'static dyn Check> = inventory::iter::<Registered>
        .into_iter()
        .map(|r| r.0)
        .collect();
    checks.sort_by_key(|c| crate::catalogue::order_key(c.meta().id));
    checks
}

/// The registered check with this id
pub fn check(id: &str) -> Option<&'static dyn Check> {
    inventory::iter::<Registered>
        .into_iter()
        .map(|r| r.0)
        .find(|c| c.meta().id == id)
}
