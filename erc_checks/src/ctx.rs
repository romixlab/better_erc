//! The context a check runs in: the design, the board, the variant, the project's overrides, and the analyses,
//! each built on first use and shared read-only by all checks running in parallel. And [`Availability`]: which
//! data a board has, worked out before a run.

use crate::finding::FindingBuilder;
use crate::meta::{CheckMeta, Need, PATHS, Severity};
use crate::project_file::derating::Derating;
use crate::project_file::nets::{NetFacts, ProjectNets};
use crate::units::Units;
use erc_analysis::drawing::Drawing;
use erc_analysis::scene::Scene;
use erc_analysis::{Feeds, I2cBuses, PartClasses, Power, PowerOptions, Values};
use erc_model::geometry::Theme;
use erc_model::{Board, BoardId, Design, PartId, Provenance, VariantId, View};
use erc_parts::PartsLock;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

/// What a project changes about the checks: the hook the project file (CFG-2) fills
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// The project file these come from, for the provenance of an overridden threshold
    pub file: Option<String>,
    /// Severity per check id, over the catalogue's
    pub severity: BTreeMap<String, Severity>,
    /// Thresholds per `CHECK-ID.param` (`I2C-9.min_pull_up`), over the check's defaults; a shared setting by its
    /// own name (`drawing.pin_lead`)
    pub params: BTreeMap<String, f64>,
    /// Check ids not to run
    pub disabled: BTreeSet<String>,
    /// The colour themes the sheets are looked at on
    #[serde(default)]
    pub themes: Vec<Theme>,
    /// What the project states about nets, by net name
    #[serde(default)]
    pub nets: BTreeMap<String, NetFacts>,
    /// Derating factors per part class and technology
    #[serde(default)]
    pub derating: Derating,
    /// The files each path setting names (`drawing.notes`), relative to the project directory, as given
    #[serde(default)]
    pub paths: BTreeMap<String, Vec<String>>,
    /// The text of each file of [`Config::paths`], by its path as given; [`Config::read_paths`] fills it
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// The units of positions and lengths in messages (CFG-10); `None`: not set here, millimetres unless a
    /// config laid over it sets them
    #[serde(default)]
    pub units: Option<Units>,
}

/// The largest file a path setting may name
const PATH_FILE_BYTES_MAX: u64 = 4 << 20;

impl Config {
    /// Whether the project sets `key`: a section (`nets`), a net (`nets.+3V3`) or a net's fact
    /// (`nets.+3V3.voltage`)
    pub fn sets(&self, key: &str) -> bool {
        match key.split_once('.') {
            None if key == "nets" => !self.nets.is_empty(),
            None => false,
            Some(("nets", rest)) => {
                let (name, fact) = match rest.rsplit_once('.') {
                    Some((n, "voltage")) => (n, Some("voltage")),
                    _ => (rest, None),
                };
                let name = name.trim_matches('"');
                self.nets
                    .get(name)
                    .is_some_and(|n| fact.is_none() || n.voltage.is_some())
            }
            Some(_) => self.params.contains_key(key) || self.paths.contains_key(key),
        }
    }

    /// `other` over this: its settings, path settings and their files, severities, disabled checks, themes, nets and
    /// set derating factors win
    pub fn overlay(&mut self, other: &Config) {
        if other.file.is_some() {
            self.file.clone_from(&other.file);
        }
        self.severity
            .extend(other.severity.iter().map(|(k, v)| (k.clone(), *v)));
        self.params
            .extend(other.params.iter().map(|(k, v)| (k.clone(), *v)));
        self.disabled.extend(other.disabled.iter().cloned());
        self.themes.extend(other.themes.iter().cloned());
        self.nets
            .extend(other.nets.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.derating.overlay(&other.derating);
        self.paths
            .extend(other.paths.iter().map(|(k, v)| (k.clone(), v.clone())));
        self.files
            .extend(other.files.iter().map(|(k, v)| (k.clone(), v.clone())));
        if other.units.is_some() {
            self.units = other.units;
        }
    }

    /// Sets a threshold by its key: a setting of `CHECK-ID.param`, or a shared one by its own name. A key no
    /// registered check declares is refused.
    pub fn set(&mut self, key: &str, value: f64) -> Result<(), String> {
        let p = declared(key)?;
        if p.unit == PATHS {
            return Err(format!("{key} takes file paths, not {value}"));
        }
        let index = value as usize;
        if !p.choices.is_empty()
            && (value.fract() != 0.0 || value < 0.0 || index >= p.choices.len())
        {
            return Err(format!(
                "{key} takes {}, not {value}",
                p.choices.join(" or ")
            ));
        }
        self.params.insert(key.to_string(), value);
        Ok(())
    }

    /// Sets a setting from its text: a number, or one of a word setting's words (`drawing.clearance.spacing=sparse`)
    pub fn set_text(&mut self, key: &str, value: &str) -> Result<(), String> {
        let p = declared(key)?;
        if p.unit == PATHS {
            let paths: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .collect();
            if paths.is_empty() {
                return Err(format!("{key} takes file paths, separated by commas"));
            }
            self.paths.insert(key.to_string(), paths);
            return Ok(());
        }
        if let Some(i) = p.choices.iter().position(|c| *c == value) {
            return self.set(key, i as f64);
        }
        match value.parse() {
            Ok(v) => self.set(key, v),
            Err(_) if !p.choices.is_empty() => Err(format!(
                "{key} takes {}, not {value}",
                p.choices.join(" or ")
            )),
            Err(_) => Err(format!("{key} takes a number, not {value}")),
        }
    }

    /// Reads the files the path settings name, relative to `dir`, into [`Config::files`]
    ///
    /// # Errors
    ///
    /// A file that cannot be read, is not UTF-8 or is larger than 4 MiB.
    pub fn read_paths(&mut self, dir: &Path) -> Result<(), String> {
        for (key, paths) in &self.paths {
            for path in paths {
                let full = dir.join(path);
                let why = |e: &dyn std::fmt::Display| format!("{key}: {}: {e}", full.display());
                let file = std::fs::File::open(&full).map_err(|e| why(&e))?;
                let mut text = String::new();
                file.take(PATH_FILE_BYTES_MAX + 1)
                    .read_to_string(&mut text)
                    .map_err(|e| why(&e))?;
                if text.len() as u64 > PATH_FILE_BYTES_MAX {
                    return Err(why(&"larger than 4 MiB"));
                }
                self.files.insert(path.clone(), text);
            }
        }
        Ok(())
    }
}

/// The setting a registered check declares under `key`
pub(crate) fn declared(key: &str) -> Result<crate::meta::Param, String> {
    crate::meta::registered()
        .into_iter()
        .find_map(|c| {
            let m = c.meta();
            m.params
                .iter()
                .find(|p| param_key(m, p.name) == key)
                .copied()
        })
        .ok_or_else(|| format!("no check has the setting {key}"))
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
                    missing(
                        "no layout: a schematic PDF carries none (its PCB prints are drawings only)",
                    )
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
            Need::Geometry => {
                if board.geometry.is_some() {
                    Availability::Available
                } else {
                    missing(
                        "no drawing geometry: import the .kicad_sch, not a netlist or a PDF (NET-26)",
                    )
                }
            }
            Need::ColourTheme => {
                if cx.themes().is_empty() {
                    missing("no colour theme: pass KiCad's colors/*.json with --theme")
                } else {
                    Availability::Available
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
                        "no rail name carries a voltage (`+3V3`, `VDD_1V8`, `-5V`); set them under `[nets]` in the project file (CFG-2)",
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
            Need::PartsLock => {
                if cx.parts.is_some() {
                    Availability::Available
                } else {
                    missing(
                        "no parts snapshot beside the project: `berc parts refresh` writes parts.lock.toml (CFG-9)",
                    )
                }
            }
            // Derating factors have defaults in code: never missing
            Need::Project(key) if key == "derating" || key.starts_with("derating.") => {
                Availability::Available
            }
            Need::Project(key) if declared(key).is_ok_and(|p| p.unit == PATHS) => {
                match cx.path_files(key) {
                    Some(Ok(_)) => Availability::Available,
                    Some(Err(path)) => {
                        missing(&format!("`{key}` names {path}, which was not read"))
                    }
                    None => missing(&format!(
                        "`{key}` is not set: --set {key}=FILE or the better_erc project file (CFG-2)"
                    )),
                }
            }
            Need::Project(key) => match &cx.config.file {
                None => Availability::Missing {
                    why: format!("no better_erc project file (CFG-2): it would set `{key}`"),
                },
                Some(file) if !cx.config.sets(key) => Availability::Missing {
                    why: format!("{file} does not set `{key}`"),
                },
                Some(_) => Availability::Available,
            },
        }
    }
}

/// The prefix of a setting several checks share
const SHARED: &str = "drawing.";

/// The key of a threshold in [`Config::params`]
fn param_key(meta: &CheckMeta, name: &str) -> String {
    if name.starts_with(SHARED) {
        name.to_string()
    } else {
        format!("{}.{name}", meta.id)
    }
}

/// What a run looks at: the design, one of its boards, a variant; analyses are built lazily and shared
pub struct Ctx<'a> {
    pub design: &'a Design,
    pub board_id: BoardId,
    /// The variant checked; `None` for the base design
    pub variant: Option<VariantId>,
    config: &'a Config,
    parts: Option<&'a PartsLock>,
    classes: OnceLock<PartClasses>,
    values: OnceLock<Values>,
    power: OnceLock<Power>,
    feeds: OnceLock<Feeds>,
    i2c: OnceLock<I2cBuses>,
    drawing: OnceLock<Drawing>,
    scenes: OnceLock<Vec<Scene<'a>>>,
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
            parts: None,
            classes: OnceLock::new(),
            values: OnceLock::new(),
            power: OnceLock::new(),
            feeds: OnceLock::new(),
            i2c: OnceLock::new(),
            drawing: OnceLock::new(),
            scenes: OnceLock::new(),
        }
    }

    /// The units messages write positions and lengths in (CFG-10)
    pub fn units(&self) -> Units {
        self.config.units.unwrap_or_default()
    }

    /// The context with the project's parts snapshot (Need::PartsLock)
    pub fn with_parts(mut self, parts: Option<&'a PartsLock>) -> Ctx<'a> {
        self.parts = parts;
        self
    }

    /// The parts snapshot beside the project (Need::PartsLock)
    pub fn parts(&self) -> Option<&'a PartsLock> {
        self.parts
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
        self.power.get_or_init(|| {
            let project = ProjectNets {
                config: self.config,
            };
            Power::with(
                self.view(),
                self.classes(),
                self.values(),
                PowerOptions::default(),
                &[&project],
            )
        })
    }

    /// The source pins of each net and the parts that carry power between nets, in the variant checked
    /// (Need::Rails)
    pub fn feeds(&self) -> &Feeds {
        self.feeds
            .get_or_init(|| Feeds::of(self.view(), self.classes(), self.values(), self.power()))
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

    /// Each sheet as drawn, with its item boxes and wire ties (Need::Geometry)
    pub fn scenes(&self) -> &[Scene<'a>] {
        self.scenes.get_or_init(|| Scene::all(self.board()))
    }

    /// The real components of the board, fitted or not
    pub fn components(&self) -> impl Iterator<Item = PartId> + 'a {
        self.view().components()
    }

    /// Whether a part is fitted in the variant checked
    pub fn fitted(&self, part: PartId) -> bool {
        self.view().fitted(part)
    }

    /// The files a path setting names with their text, in the order given: `None` when it is unset, the first path
    /// not read when one is missing
    pub fn path_files(&self, key: &str) -> Option<Result<Vec<(&str, &str)>, &str>> {
        let paths = self.config.paths.get(key)?;
        Some(
            paths
                .iter()
                .map(|p| match self.config.files.get(p) {
                    Some(text) => Ok((p.as_str(), text.as_str())),
                    None => Err(p.as_str()),
                })
                .collect(),
        )
    }

    /// The colour themes given to the run (Need::ColourTheme)
    pub fn themes(&self) -> &[Theme] {
        &self.config.themes
    }

    /// A threshold: the project's value, else the check's default
    pub fn param(&self, meta: &CheckMeta, name: &str) -> f64 {
        let default = meta
            .param(name)
            .unwrap_or_else(|| panic!("{} declares no param {name}", meta.id))
            .default;
        self.config
            .params
            .get(&param_key(meta, name))
            .copied()
            .unwrap_or(default)
    }

    /// A word setting: the word it is set to, or its default
    ///
    /// # Panics
    ///
    /// When the check declares no word setting `name`.
    pub fn choice(&self, meta: &CheckMeta, name: &str) -> &'static str {
        let p = meta
            .param(name)
            .unwrap_or_else(|| panic!("{} declares no param {name}", meta.id));
        assert!(
            !p.choices.is_empty(),
            "{} param {name} is a number",
            meta.id
        );
        // `Config::set` holds a word setting to an index of its words; a default off the list is the declaration's bug
        let i = self.param(meta, name) as usize;
        p.choices
            .get(i)
            .copied()
            .unwrap_or_else(|| panic!("{} param {name}: index {i} past its words", meta.id))
    }

    /// Where a threshold came from, for a finding's facts
    pub fn param_source(&self, meta: &CheckMeta, name: &str) -> Provenance {
        if self.config.params.contains_key(&param_key(meta, name)) {
            Provenance::ProjectFile {
                file: self.config.file.clone().unwrap_or_default(),
                key: if name.starts_with(SHARED) {
                    name.to_string()
                } else {
                    format!("checks.{}.{name}", meta.id)
                },
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

    /// The severity of one finding: the project's override, else `chosen` (a finding the check rates differently),
    /// else the catalogue's
    pub fn severity_or(&self, meta: &CheckMeta, chosen: Option<Severity>) -> Severity {
        self.config
            .severity
            .get(meta.id)
            .copied()
            .or(chosen)
            .unwrap_or(meta.severity)
    }

    /// The derating factors of the project, defaults where it sets none
    pub fn derating(&self) -> &Derating {
        &self.config.derating
    }

    /// What the project states about a net of this board, by its name
    pub fn project_net(&self, net: erc_model::NetId) -> Option<&NetFacts> {
        self.config.nets.get(&self.board().net(net).name)
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
