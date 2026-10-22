//! The context a check runs in: the design, the board, the variant, the project's overrides, and the analyses,
//! each built on first use and shared read-only by all checks running in parallel. And [`Availability`]: which
//! data a board has, worked out before a run.

use crate::finding::FindingBuilder;
use crate::meta::{CheckMeta, Need, Severity};
use erc_analysis::{I2cBuses, PartClasses};
use erc_model::{Board, BoardId, Design, PartId, Provenance, VariantId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// What a project changes about the checks: the hook the project file (CFG-2) fills
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// The project file these come from, for the provenance of an overridden threshold
    pub file: Option<String>,
    /// Severity per check id, over the catalogue's
    pub severity: BTreeMap<String, Severity>,
    /// Thresholds per `CHECK-ID.param` (`I2C-9.min_pull_up`), over the check's defaults
    pub params: BTreeMap<String, f64>,
    /// Check ids not to run
    pub disabled: BTreeSet<String>,
}

/// Whether a board has the data a [`Need`] asks for
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "availability")]
pub enum Availability {
    Available,
    /// Known for `have` of `of` items: checks run on what there is, and the report says how much that was
    Partial {
        have: usize,
        of: usize,
        what: String,
    },
    /// Not there, and why (what would provide it)
    Missing {
        why: String,
    },
}

impl Availability {
    /// What `board` of `design` has of `need`
    pub fn of(need: Need, design: &Design, board: &Board) -> Availability {
        let components: Vec<PartId> = (0..board.parts.len())
            .map(PartId::from_index)
            .filter(|p| board.is_component(*p))
            .collect();
        let missing = |why: &str| Availability::Missing { why: why.into() };
        let count = |have: usize, what: &str| {
            if components.is_empty() || have == components.len() {
                Availability::Available
            } else if have == 0 {
                Availability::Missing {
                    why: format!("no part has {what}"),
                }
            } else {
                Availability::Partial {
                    have,
                    of: components.len(),
                    what: what.into(),
                }
            }
        };
        match need {
            Need::Graph => {
                if components.is_empty() {
                    missing("the design has no parts")
                } else {
                    Availability::Available
                }
            }
            // Classes always resolve: by library, description, or the designator prefix as an assumption
            Need::PartClasses => Availability::Available,
            Need::Values => count(
                components
                    .iter()
                    .filter(|p| board.part(**p).fields.iter().any(|f| f.name == "Value"))
                    .count(),
                "a Value field",
            ),
            Need::Layout => {
                if board.layout.is_some() {
                    Availability::Available
                } else {
                    missing("no layout: import the project with its .kicad_pcb")
                }
            }
            Need::Drawing => {
                if board.source.tool == "kicad" && !board.sheets.is_empty() {
                    Availability::Available
                } else {
                    missing(
                        "no schematic drawing (labels, no-connect flags): import the .kicad_sch, not a netlist",
                    )
                }
            }
            Need::MultiBoard => {
                if design.links.is_empty() {
                    missing(
                        "one board only: join boards through connectors in the project file (NET-13)",
                    )
                } else {
                    Availability::Available
                }
            }
            Need::Rails | Need::RailVoltages => {
                missing("rails and their voltages are not derived yet (NET-5)")
            }
            Need::PinRoles | Need::PinThresholds => {
                missing("pin roles are not derived yet (NET-7)")
            }
            Need::PinFunctions => missing("pin functions are not known yet (NET-12)"),
            Need::PowerStates => missing("power states need the project file (NET-8, CFG-2)"),
            Need::Conduction => missing("signal following is not built yet (NET-10)"),
            Need::PartDb => missing("no part data: the part database is not built yet (CFG-5)"),
            Need::Project(key) => Availability::Missing {
                why: format!("no better_erc project file (CFG-2): it would set `{key}`"),
            },
        }
    }
}

/// What a run looks at: the design, one of its boards, a variant; analyses are built lazily and shared
pub struct Ctx<'a> {
    pub design: &'a Design,
    pub board_id: BoardId,
    /// The variant checked; `None` for the base design
    pub variant: Option<VariantId>,
    config: &'a Config,
    classes: OnceLock<PartClasses>,
    i2c: OnceLock<I2cBuses>,
}

impl<'a> Ctx<'a> {
    pub fn new(
        design: &'a Design,
        board_id: BoardId,
        variant: Option<VariantId>,
        config: &'a Config,
    ) -> Ctx<'a> {
        Ctx {
            design,
            board_id,
            variant,
            config,
            classes: OnceLock::new(),
            i2c: OnceLock::new(),
        }
    }

    pub fn board(&self) -> &'a Board {
        self.design.board(self.board_id)
    }

    /// Part classes (Need::PartClasses)
    pub fn classes(&self) -> &PartClasses {
        self.classes.get_or_init(|| PartClasses::of(self.board()))
    }

    /// The I2C buses (Need::Graph, Need::PartClasses)
    pub fn i2c(&self) -> &I2cBuses {
        self.i2c
            .get_or_init(|| I2cBuses::of(self.board(), self.classes()))
    }

    /// The real components of the board, fitted or not
    pub fn components(&self) -> impl Iterator<Item = PartId> + '_ {
        let b = self.board();
        (0..b.parts.len())
            .map(PartId::from_index)
            .filter(move |p| b.is_component(*p))
    }

    /// Whether a part is fitted in the variant checked
    pub fn fitted(&self, part: PartId) -> bool {
        self.board().fitted(part, self.variant)
    }

    /// A threshold: the project's value, else the check's default
    pub fn param(&self, meta: &CheckMeta, name: &str) -> f64 {
        let default = meta
            .param(name)
            .unwrap_or_else(|| panic!("{} declares no param {name}", meta.id))
            .default;
        self.config
            .params
            .get(&format!("{}.{name}", meta.id))
            .copied()
            .unwrap_or(default)
    }

    /// Where a threshold came from, for a finding's facts
    pub fn param_source(&self, meta: &CheckMeta, name: &str) -> Provenance {
        if self
            .config
            .params
            .contains_key(&format!("{}.{name}", meta.id))
        {
            Provenance::ProjectFile {
                file: self.config.file.clone().unwrap_or_default(),
                key: format!("checks.{}.{name}", meta.id),
            }
        } else {
            Provenance::Derived {
                rule: format!("{} default", meta.id),
            }
        }
    }

    /// The severity of a check's findings: the project's override, else the catalogue's
    pub fn severity(&self, meta: &CheckMeta) -> Severity {
        self.config
            .severity
            .get(meta.id)
            .copied()
            .unwrap_or(meta.severity)
    }

    /// Starts a finding of the check `meta` on this board
    pub fn finding(&self, meta: &'static CheckMeta) -> FindingBuilder<'_, 'a> {
        FindingBuilder::new(self, meta)
    }

    /// Notes of the analyses built during the run (what they could not place)
    pub fn notes(&self) -> Vec<String> {
        let mut notes = Vec::new();
        if let Some(i2c) = self.i2c.get() {
            notes.extend(i2c.notes.iter().cloned());
        }
        notes
    }
}
