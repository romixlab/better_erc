//! The context a check runs in: the design, the board, the variant, the project's overrides, and the analyses,
//! each built on first use and shared read-only by all checks running in parallel. And [`Availability`]: which
//! data a board has, worked out before a run.

use crate::finding::FindingBuilder;
use crate::meta::{CheckMeta, Need, Severity};
use erc_analysis::drawing::Drawing;
use erc_analysis::{I2cBuses, PartClasses, Power, Values};
use erc_model::{Board, BoardId, Design, PartId, Provenance, VariantId, View};
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
    /// What the context's board has of `need`, from the analyses the checks will use (built here when first
    /// asked, then shared)
    pub fn of(need: Need, cx: &Ctx) -> Availability {
        let design = cx.design;
        let board = cx.board();
        let components: Vec<PartId> = cx.components().collect();
        let missing = |why: &str| Availability::Missing { why: why.into() };
        let count = |have: usize, of: usize, what: &str| {
            if of == 0 || have == of {
                Availability::Available
            } else if have == 0 {
                Availability::Missing {
                    why: format!("no part has {what}"),
                }
            } else {
                Availability::Partial {
                    have,
                    of,
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
            // Classes always resolve, by the designator prefix as an assumption at worst: partial, never missing
            Need::PartClasses => {
                let c = cx.classes().coverage(board);
                if c.known == c.components() {
                    Availability::Available
                } else {
                    Availability::Partial {
                        have: c.known,
                        of: c.components(),
                        what: format!(
                            "a class from the library symbol or its description ({} by designator prefix, assumed; {} unknown)",
                            c.assumed, c.unknown
                        ),
                    }
                }
            }
            Need::Values => count(
                components
                    .iter()
                    .filter(|p| erc_analysis::value::value_text(cx.view(), **p).is_some())
                    .count(),
                components.len(),
                "a value",
            ),
            Need::Layout => {
                if board.layout.is_some() {
                    Availability::Available
                } else if board.source.tool == "altium-pdf" {
                    missing("no layout: a schematic PDF carries none (its PCB prints are drawings only)")
                } else {
                    missing("no layout: import the project with its .kicad_pcb")
                }
            }
            Need::Drawing => {
                if Drawing::available(board) {
                    Availability::Available
                } else if board.source.tool == "altium-pdf" {
                    missing(
                        "no schematic drawing: a Smart PDF gives net labels and ports but no wires or no-connect flags",
                    )
                } else {
                    missing(
                        "no schematic drawing (labels, no-connect flags): import the .kicad_sch, not a netlist",
                    )
                }
            }
            Need::Variant => {
                let stated = matches!(board.source.tool.as_str(), "kicad" | "kicad-netlist")
                    || !board.variants.is_empty()
                    || board.parts.iter().any(|p| !p.flags.dnp.is_empty());
                if stated {
                    Availability::Available
                } else {
                    missing(
                        "the design states no DNP flags or variants: every part counts as fitted",
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
            Need::Rails => {
                let p = cx.power();
                if p.rails.is_empty() && p.grounds.is_empty() {
                    missing(
                        "no net reads as a rail or ground: name rails `+3V3`, `VDD_1V8`, grounds `GND`, or use power symbols and power pins (NET-5)",
                    )
                } else {
                    Availability::Available
                }
            }
            Need::RailVoltages => {
                let (have, of) = cx.power().voltage_coverage();
                if of == 0 {
                    missing("no rails (NET-5)")
                } else if have == 0 {
                    missing(
                        "no rail name carries a voltage (`+3V3`, `VDD_1V8`, `-5V`); the project file will set them (CFG-2)",
                    )
                } else if have == of {
                    Availability::Available
                } else {
                    Availability::Partial {
                        have,
                        of,
                        what: "rails with a voltage".into(),
                    }
                }
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
    values: OnceLock<Values>,
    power: OnceLock<Power>,
    i2c: OnceLock<I2cBuses>,
    drawing: OnceLock<Drawing>,
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
            values: OnceLock::new(),
            power: OnceLock::new(),
            i2c: OnceLock::new(),
            drawing: OnceLock::new(),
        }
    }

    pub fn board(&self) -> &'a Board {
        self.design.board(self.board_id)
    }

    /// The board as the variant checked mounts it (Need::Variant)
    pub fn view(&self) -> View<'a> {
        self.board().view(self.variant)
    }

    /// What the board has of `need`
    pub fn availability(&self, need: Need) -> Availability {
        Availability::of(need, self)
    }

    /// Part classes (Need::PartClasses)
    pub fn classes(&self) -> &PartClasses {
        self.classes.get_or_init(|| PartClasses::of(self.board()))
    }

    /// Every part's Value field read in its class's unit, in the variant checked (Need::Values)
    pub fn values(&self) -> &Values {
        self.values
            .get_or_init(|| Values::of(self.view(), self.classes()))
    }

    /// Rails, grounds, ground groups and switching nodes in the variant checked (Need::Rails,
    /// Need::RailVoltages)
    pub fn power(&self) -> &Power {
        self.power
            .get_or_init(|| Power::of(self.view(), self.classes(), self.values()))
    }

    /// The I2C buses of the parts fitted in the variant checked (Need::Graph, Need::PartClasses)
    pub fn i2c(&self) -> &I2cBuses {
        self.i2c
            .get_or_init(|| I2cBuses::on(self.view(), self.classes()))
    }

    /// Labels with their scopes, sheet pins, no-connect flags (Need::Drawing)
    pub fn drawing(&self) -> &Drawing {
        self.drawing.get_or_init(|| Drawing::of(self.board()))
    }

    /// The real components of the board, fitted or not
    pub fn components(&self) -> impl Iterator<Item = PartId> + 'a {
        self.view().components()
    }

    /// Whether a part is fitted in the variant checked
    pub fn fitted(&self, part: PartId) -> bool {
        self.view().fitted(part)
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
