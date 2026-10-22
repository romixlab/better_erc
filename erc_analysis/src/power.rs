//! Rails and grounds, the first layer of the power structure (NET-5, architecture §1.4): which nets carry power,
//! which are grounds, what voltage a rail's name gives, how grounds join into groups, and the switching nodes of
//! converters (NET-6), which are neither. Sources, the power tree and power states come later (NET-5 full, NET-8);
//! these rules stay as their first layer.
//!
//! A net is a rail or a ground by up to three signals, each kept as a [`Fact`] with its rule:
//!
//! 1. **Its name** ([`read_name`]): `+3V3`, `3V3`, `-5V`, `+3.3V`, `12V0_IN`, `VDD_1V8`, `VCC3V3`, `VBUS`,
//!    `VBAT`, `VIN`, `+V...`, `V+`, `VEE`, `5V0_NEG` for rails; `GND`, `AGND`, `PGND`, `VSS`, `GND_ISO`,
//!    `CHASSIS`, `EARTH`, `0V` for grounds. A name with a signal word (`EN`, `PG`, `SENSE`, `FB`, `SCL`, `TX`,
//!    ...) is a signal however it mentions a voltage (`VDD_EN`, `SCL1_3V3`), and an auto-named net (`Net-(U1-VDD)`)
//!    has no name to read.
//! 2. **The drawing and the pins**: a power symbol on the net, power pins (`power_in`, `power_out`) of components on
//!    it; pins named like a ground (`GND`, `VSS`, `AGND`) make a ground of a net whose name does not decide.
//! 3. **The project** ([`NetSource`], the better_erc project file, CFG-2): a declared rail, ground or signal, and
//!    a voltage, ranked above everything the schematic implies.
//!
//! A rail's voltage comes from the project, else from its name ([`Provenance::Derived`]); both are kept in one
//! [`Facts`], so a disagreement between them is visible (RAIL-3). The voltage of a name with a negative marker
//! and no sign (`5V0_NEG`) is negative, and the rule says so (NAME-11).
//!
//! Grounds joined at DC by a net tie, jumper, 0 Ω resistor, ferrite or inductor form one [`GroundGroup`]; every
//! fitted part with pins on two different grounds is a [`GroundLink`], DC or not (an R or C to chassis), which is
//! what GND-3 and GND-7 judge.

use crate::class::PartClasses;
use crate::graph::{net_pins, part_chains, part_nets};
use crate::value::Values;
use erc_model::{
    Board, EcadPinType, Fact, Facts, NetId, NetKind, PartClass, PartId, Provenance, View,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What says a net is a rail or a ground
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PowerSignal {
    /// The net name ([`read_name`])
    Name,
    /// A power symbol on the net
    PowerSymbol,
    /// Power pins (`power_in`, `power_out`) of components on the net
    PowerPin,
    /// Pins named like a ground (`GND`, `VSS`) on the net
    GroundPin,
    /// The project file declares it
    Project,
}

/// What a rail feeds, as far as its name tells
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RailKind {
    Supply,
    /// An analog supply (`VDDA`, `AVDD`, `+3V3A`, `3V3_ANA`): RAIL-19
    Analog,
    /// A reference (`VREF`, `AREF`)
    Reference,
    /// USB VBUS
    Usb,
    /// A battery (`VBAT`)
    Battery,
}

/// What kind of ground
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GroundKind {
    Signal,
    Analog,
    Digital,
    Power,
    /// An isolated side's ground (`GND_ISO`, `ISO_GND`)
    Isolated,
    /// Chassis, shield, frame
    Chassis,
    /// Protective earth
    Earth,
}

/// A rail
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rail {
    pub kind: RailKind,
    /// Every signal that makes the net a rail, with its rule
    pub signals: Vec<Fact<PowerSignal>>,
    /// The voltage from each source (the project, the name), volts; [`Facts::resolve`] gives the one to use
    pub voltage: Facts<f64>,
}

impl Rail {
    /// The resolved voltage, volts
    pub fn volts(&self) -> Option<f64> {
        self.voltage.get().copied()
    }

    pub fn has(&self, signal: PowerSignal) -> bool {
        self.signals.iter().any(|s| s.value == signal)
    }
}

/// A ground net
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ground {
    pub kind: GroundKind,
    pub signals: Vec<Fact<PowerSignal>>,
}

/// Grounds joined at DC (net ties, jumpers, 0 Ω, ferrites, inductors): one return network
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroundGroup {
    pub grounds: BTreeSet<NetId>,
    /// The parts that join them
    pub ties: BTreeSet<PartId>,
}

/// How a part joins two grounds
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Joint {
    NetTie,
    Jumper,
    ZeroOhm,
    Ferrite,
    Inductor,
    Resistor,
    Capacitor,
    /// Another part (an IC with AGND and DGND pins, a connector): the grounds meet inside or beyond it
    Part,
}

impl Joint {
    /// Joins its nets at DC (one ground group)
    pub fn dc(self) -> bool {
        matches!(
            self,
            Joint::NetTie | Joint::Jumper | Joint::ZeroOhm | Joint::Ferrite | Joint::Inductor
        )
    }
}

/// A fitted part with pins on two or more different grounds
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroundLink {
    pub part: PartId,
    pub grounds: BTreeSet<NetId>,
    pub joint: Joint,
}

/// The power structure of a board as one variant mounts it (Need::Rails, Need::RailVoltages)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Power {
    pub rails: BTreeMap<NetId, Rail>,
    pub grounds: BTreeMap<NetId, Ground>,
    pub ground_groups: Vec<GroundGroup>,
    pub ground_links: Vec<GroundLink>,
    /// Converter switching nodes (NET-6): neither rail nor ground
    pub switching_nodes: BTreeSet<NetId>,
}

/// What the project file declares about a net
#[derive(Clone, Debug, PartialEq)]
pub struct NetDecl {
    pub role: Fact<DeclaredRole>,
    pub voltage: Option<Fact<f64>>,
}

/// A net's role as the project declares it
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclaredRole {
    Rail(RailKind),
    Ground(GroundKind),
    /// Not power, whatever the name says
    Signal,
}

/// Net facts ranked above the schematic: the project file (CFG-2) plugs in here
pub trait NetSource {
    fn net(&self, board: &Board, net: NetId) -> Option<NetDecl>;
}

/// How rails are read from names
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerOptions {
    /// Read a voltage only from `+xVy` (`+3V3`, `+12V0`: a `+`, the volts, `V`, the decimals), as the first
    /// better_erc did; NAME-7 asks whether a design has any. Off by default: the whole grammar of [`read_name`].
    pub strict: bool,
}

const BY_NAME: &str = "net name";
const BY_SYMBOL: &str = "power symbol on the net";
const BY_POWER_PIN: &str = "power pin on the net";
const BY_GROUND_PIN: &str = "ground-named pin on the net";
const VOLTAGE_BY_NAME: &str = "voltage from the net name";
const VOLTAGE_BY_NEG: &str = "voltage from the net name, negative by its NEG marker";

impl Power {
    /// Rails, grounds, ground groups and switching nodes of `view` from the schematic alone
    pub fn of(view: View, classes: &PartClasses, values: &Values) -> Power {
        Power::with(view, classes, values, PowerOptions::default(), &[])
    }

    /// [`Power::of`] with options and net sources (the project file) ranked above the schematic
    pub fn with(
        view: View,
        classes: &PartClasses,
        values: &Values,
        opts: PowerOptions,
        sources: &[&dyn NetSource],
    ) -> Power {
        let board = view.board;
        let mut power = Power::default();
        let derived = |rule: &str| Provenance::Derived { rule: rule.into() };
        for (i, net) in board.nets.iter().enumerate() {
            let id = NetId::from_index(i);
            let decl = sources.iter().find_map(|s| s.net(board, id));
            let name = if matches!(net.kind, NetKind::Auto | NetKind::Unconnected) {
                NetName::default()
            } else {
                read_name(&net.name, opts.strict)
            };
            let symbol = net.labels.iter().any(|l| {
                let l = board.label(*l);
                l.kind.is_power() && l.text != "PWR_FLAG"
            });
            let pins: Vec<_> = net_pins(board, id).map(|p| board.pin(p)).collect();
            let is_power_pin = |p: &&&erc_model::PinInst| {
                matches!(
                    p.etype.get(),
                    Some(EcadPinType::PowerIn | EcadPinType::PowerOut)
                )
            };
            let power_pins = pins.iter().filter(is_power_pin).count();
            let ground_named = pins
                .iter()
                .filter(|p| ground_pin_word(&p.name).is_some())
                .count();
            // Most power pins on the net are named like a ground; a pin alone (`unconnected-(J3-Shield-Pad6)`) is
            // no ground
            let ground_pins = net.kind != NetKind::Unconnected
                && ground_named > 0
                && ground_named * 2 > power_pins;
            let power_pins = power_pins > 0;
            let fact = |s, rule: &str| Fact::new(s, derived(rule));
            // The project decides first
            if let Some(d) = &decl {
                let sig = Fact {
                    value: PowerSignal::Project,
                    source: d.role.source.clone(),
                    rank: d.role.rank,
                    at: d.role.at,
                    note: d.role.note.clone(),
                };
                match d.role.value {
                    DeclaredRole::Signal => continue,
                    DeclaredRole::Ground(kind) => {
                        power.grounds.insert(
                            id,
                            Ground {
                                kind,
                                signals: vec![sig],
                            },
                        );
                        continue;
                    }
                    DeclaredRole::Rail(kind) => {
                        let mut voltage = Facts::default();
                        if let Some(v) = &d.voltage {
                            voltage.set(v.clone());
                        }
                        if let Some(v) = name.voltage {
                            voltage.set(Fact::new(v, derived(name.voltage_rule())));
                        }
                        power.rails.insert(
                            id,
                            Rail {
                                kind,
                                signals: vec![sig],
                                voltage,
                            },
                        );
                        continue;
                    }
                }
            }
            match name.role {
                NameRole::Signal => {
                    // A sense or enable line: only a power symbol makes it power
                    if !symbol {
                        continue;
                    }
                }
                NameRole::Ground(kind) => {
                    let mut signals = vec![fact(PowerSignal::Name, BY_NAME)];
                    if symbol {
                        signals.push(fact(PowerSignal::PowerSymbol, BY_SYMBOL));
                    }
                    if ground_pins {
                        signals.push(fact(PowerSignal::GroundPin, BY_GROUND_PIN));
                    }
                    power.grounds.insert(id, Ground { kind, signals });
                    continue;
                }
                NameRole::Rail(_) | NameRole::None => {}
            }
            if name.role == NameRole::None && ground_pins {
                power.grounds.insert(
                    id,
                    Ground {
                        kind: GroundKind::Signal,
                        signals: vec![fact(PowerSignal::GroundPin, BY_GROUND_PIN)],
                    },
                );
                continue;
            }
            let mut signals = Vec::new();
            if let NameRole::Rail(_) = name.role {
                signals.push(fact(PowerSignal::Name, BY_NAME));
            }
            if symbol {
                signals.push(fact(PowerSignal::PowerSymbol, BY_SYMBOL));
            }
            if power_pins && name.role != NameRole::Signal {
                signals.push(fact(PowerSignal::PowerPin, BY_POWER_PIN));
            }
            if signals.is_empty() {
                continue;
            }
            let mut voltage = Facts::default();
            if let Some(v) = name.voltage {
                voltage.set(Fact::new(v, derived(name.voltage_rule())));
            }
            let kind = match name.role {
                NameRole::Rail(k) => k,
                _ => RailKind::Supply,
            };
            power.rails.insert(
                id,
                Rail {
                    kind,
                    signals,
                    voltage,
                },
            );
        }
        power.switching_nodes = power.find_switching_nodes(board, classes);
        for net in &power.switching_nodes {
            power.rails.remove(net);
        }
        power.find_ground_links(view, classes, values);
        power
    }

    /// Whether a net is a rail or a ground
    pub fn is_power_net(&self, net: NetId) -> bool {
        self.rails.contains_key(&net) || self.grounds.contains_key(&net)
    }

    pub fn is_rail(&self, net: NetId) -> bool {
        self.rails.contains_key(&net)
    }

    pub fn is_ground(&self, net: NetId) -> bool {
        self.grounds.contains_key(&net)
    }

    /// The resolved voltage of a rail, volts; 0 for a ground
    pub fn volts(&self, net: NetId) -> Option<f64> {
        if self.is_ground(net) {
            return Some(0.0);
        }
        self.rails.get(&net)?.volts()
    }

    /// The ground group a ground is in
    pub fn ground_group(&self, net: NetId) -> Option<&GroundGroup> {
        self.ground_groups.iter().find(|g| g.grounds.contains(&net))
    }

    /// Rails with a voltage, of all rails: the coverage of Need::RailVoltages
    pub fn voltage_coverage(&self) -> (usize, usize) {
        (
            self.rails.values().filter(|r| r.volts().is_some()).count(),
            self.rails.len(),
        )
    }

    fn find_switching_nodes(&self, board: &Board, classes: &PartClasses) -> BTreeSet<NetId> {
        let ic = |p: PartId| classes.get(p).is_ic();
        let inductor = |p: PartId| classes.get(p) == PartClass::Inductor;
        let mut nodes = BTreeSet::new();
        for chain in part_chains(board, &[&ic, &inductor], &|_| false) {
            let [ic, l] = chain[..] else { continue };
            let l_nets = part_nets(board, l);
            let switching = part_nets(board, ic).into_iter().find(|n| {
                let name = &board.net(*n).name;
                l_nets.contains(n)
                    && !self.grounds.contains_key(n)
                    && (!self.rails.get(n).is_some_and(|r| r.has(PowerSignal::Name))
                        || name.contains("LX")
                        || name.contains("SW"))
            });
            let Some(switching) = switching else {
                continue;
            };
            let Some(other) = l_nets.iter().find(|n| **n != switching) else {
                continue;
            };
            let rf = |n: NetId| board.net(n).name.contains("RF");
            if !rf(switching) && !rf(*other) {
                nodes.insert(switching);
            }
        }
        nodes
    }

    fn find_ground_links(&mut self, view: View, classes: &PartClasses, values: &Values) {
        let board = view.board;
        let mut groups: BTreeMap<NetId, NetId> = self.grounds.keys().map(|n| (*n, *n)).collect();
        fn root(groups: &mut BTreeMap<NetId, NetId>, n: NetId) -> NetId {
            let mut r = n;
            while groups[&r] != r {
                r = groups[&r];
            }
            groups.insert(n, r);
            r
        }
        let mut ties: BTreeMap<NetId, BTreeSet<PartId>> = BTreeMap::new();
        for p in view.fitted_components() {
            let grounds: BTreeSet<NetId> = part_nets(board, p)
                .into_iter()
                .filter(|n| self.grounds.contains_key(n))
                .collect();
            if grounds.len() < 2 {
                continue;
            }
            let class = classes.get(p);
            let joint = match class {
                PartClass::NetTie => Joint::NetTie,
                PartClass::Jumper => Joint::Jumper,
                PartClass::FerriteBead => Joint::Ferrite,
                PartClass::Inductor => Joint::Inductor,
                PartClass::Resistor if values.get(p).value == Some(0.0) => Joint::ZeroOhm,
                PartClass::Resistor => Joint::Resistor,
                PartClass::Capacitor { .. } => Joint::Capacitor,
                _ => Joint::Part,
            };
            if joint.dc() {
                let mut it = grounds.iter();
                let first = *it.next().expect("two grounds");
                for g in it {
                    let (a, b) = (root(&mut groups, first), root(&mut groups, *g));
                    groups.insert(a.max(b), a.min(b));
                }
            }
            self.ground_links.push(GroundLink {
                part: p,
                grounds,
                joint,
            });
        }
        let mut by_root: BTreeMap<NetId, GroundGroup> = BTreeMap::new();
        let nets: Vec<NetId> = groups.keys().copied().collect();
        for n in nets {
            let r = root(&mut groups, n);
            by_root.entry(r).or_default().grounds.insert(n);
        }
        for link in &self.ground_links {
            if link.joint.dc() {
                let first = *link.grounds.iter().next().expect("two grounds");
                let r = root(&mut groups, first);
                ties.entry(r).or_default().insert(link.part);
            }
        }
        for (r, g) in &mut by_root {
            g.ties = ties.remove(r).unwrap_or_default();
        }
        self.ground_groups = by_root.into_values().collect();
    }
}

/// What a net name says
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NetName {
    pub role: NameRole,
    /// Volts, signed; for a ground none
    pub voltage: Option<f64>,
    /// A negative marker (`NEG`, `VEE`, `V-`, `VNEG`) in the name
    pub negative: bool,
    /// The sign was taken from the negative marker, not written (`5V0_NEG`)
    pub sign_from_marker: bool,
}

impl NetName {
    fn voltage_rule(&self) -> &'static str {
        if self.sign_from_marker {
            VOLTAGE_BY_NEG
        } else {
            VOLTAGE_BY_NAME
        }
    }
}

/// What a name makes a net
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NameRole {
    /// The name says nothing about power
    #[default]
    None,
    Rail(RailKind),
    Ground(GroundKind),
    /// A signal by a signal word (`VDD_EN`, `SCL_3V3`)
    Signal,
}

/// Words that make a name a signal even with a voltage or a power word in it
const SIGNAL_WORDS: &[&str] = &[
    "EN", "ENA", "ENABLE", "NEN", "PG", "PGOOD", "PWRGD", "PWROK", "POK", "OK", "SENSE", "SNS",
    "FB", "DET", "DETECT", "MON", "ADC", "CTRL", "CTL", "SEL", "INT", "IRQ", "ALERT", "FAULT",
    "NFAULT", "SCL", "SDA", "SCK", "SCLK", "MOSI", "MISO", "CS", "TX", "RX", "TXD", "RXD", "CLK",
    "DIV", "GATE", "DRV", "LED", "STAT", "STATUS", "CHG", "ON", "OFF", "RST", "RESET", "IN_DET",
];

/// Power words a rail name starts a token with
const POWER_WORDS: &[(&str, RailKind)] = &[
    ("VDDA", RailKind::Analog),
    ("VCCA", RailKind::Analog),
    ("AVDD", RailKind::Analog),
    ("AVCC", RailKind::Analog),
    ("VREF", RailKind::Reference),
    ("AREF", RailKind::Reference),
    ("VBUS", RailKind::Usb),
    ("VBAT", RailKind::Battery),
    ("VBATT", RailKind::Battery),
    ("BATT", RailKind::Battery),
    ("BAT", RailKind::Battery),
    ("VDD", RailKind::Supply),
    ("VCC", RailKind::Supply),
    ("DVDD", RailKind::Supply),
    ("IOVDD", RailKind::Supply),
    ("PVDD", RailKind::Supply),
    ("VIN", RailKind::Supply),
    ("PVIN", RailKind::Supply),
    ("VSYS", RailKind::Supply),
    ("VOUT", RailKind::Supply),
    ("VIO", RailKind::Supply),
    ("VPP", RailKind::Supply),
    ("VMOT", RailKind::Supply),
    ("VPWR", RailKind::Supply),
    ("VSUP", RailKind::Supply),
    ("VSUPPLY", RailKind::Supply),
    ("VEE", RailKind::Supply),
    ("VNEG", RailKind::Supply),
    ("VPOS", RailKind::Supply),
    ("PWR", RailKind::Supply),
    ("POWER", RailKind::Supply),
    ("V+", RailKind::Supply),
    ("V-", RailKind::Supply),
    ("+V", RailKind::Supply),
];

/// The ground word a token is, with its kind
fn ground_word(t: &str) -> Option<GroundKind> {
    Some(match t {
        "AGND" | "GNDA" | "VSSA" | "AVSS" | "AGND1" => GroundKind::Analog,
        "DGND" | "GNDD" | "DVSS" => GroundKind::Digital,
        "PGND" | "GNDPWR" | "PWRGND" => GroundKind::Power,
        "GNDISO" | "ISOGND" | "IGND" => GroundKind::Isolated,
        "CHASSIS" | "SHIELD" | "SHLD" | "FRAME" | "CGND" | "GNDCHASSIS" => GroundKind::Chassis,
        "EARTH" | "PE" => GroundKind::Earth,
        "GND" | "VSS" | "0V" | "SGND" | "GNDREF" => GroundKind::Signal,
        _ if t.starts_with("GND") && t[3..].chars().all(|c| c.is_ascii_digit()) => {
            GroundKind::Signal
        }
        _ if t.starts_with("VSS") && t[3..].chars().all(|c| c.is_ascii_alphanumeric()) => {
            GroundKind::Signal
        }
        _ => return None,
    })
}

/// A voltage token: `3V3`, `+3V3`, `-5V`, `+5V`, `12V0`, `3.3V`, `+3.3V`, `3P3V`. Strict: only `+` with the RKM
/// decimals (`+3V3`, `+12V0`). Returns the signed volts and whether a sign was written.
pub fn voltage_token(t: &str, strict: bool) -> Option<(f64, bool)> {
    let (sign, body) = match t.as_bytes().first()? {
        b'+' => (Some(1.0), &t[1..]),
        b'-' => (Some(-1.0), &t[1..]),
        _ => (None, t),
    };
    if strict && sign != Some(1.0) {
        return None;
    }
    let b = body.as_bytes();
    let whole = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if whole == 0 {
        return None;
    }
    let rest = &body[whole..];
    let v: f64 = if let Some(frac) = rest.strip_prefix('V') {
        // `3V3`, `5V`
        if !frac.chars().all(|c| c.is_ascii_digit()) || strict && frac.is_empty() {
            return None;
        }
        format!(
            "{}.{}",
            &body[..whole],
            if frac.is_empty() { "0" } else { frac }
        )
        .parse()
        .ok()?
    } else if !strict
        && let Some(r) = rest.strip_prefix(['.', 'P'])
        && let Some(frac) = r.strip_suffix('V')
        && !frac.is_empty()
        && frac.chars().all(|c| c.is_ascii_digit())
    {
        // `3.3V`, `3P3V`
        format!("{}.{}", &body[..whole], frac).parse().ok()?
    } else {
        return None;
    };
    Some((v * sign.unwrap_or(1.0), sign.is_some()))
}

/// A name KiCad gives a net nobody named: `Net-(R1-Pad2)`, `unconnected-(J3-Shield-Pad6)`. Its words are a part's
/// and a pin's, never a rail's or a ground's.
pub fn is_auto_name(full: &str) -> bool {
    let leaf = full.rsplit('/').next().unwrap_or(full);
    leaf.starts_with("Net-(") || leaf.starts_with("unconnected-(")
}

/// The ground a pin name states, for the ground-pin rule: `GND`, `VSS`, `AGND`, `PGND`; a shield, chassis or earth
/// pin is left out (a connector shell goes to ground through an RC on purpose, it does not make its net a ground)
pub fn ground_pin_word(name: &str) -> Option<GroundKind> {
    match read_name(name, false).role {
        NameRole::Ground(GroundKind::Chassis | GroundKind::Earth) => None,
        NameRole::Ground(k) => Some(k),
        _ => None,
    }
}

/// Reads a net name by the power name grammar (architecture §1.4); the sheet path (`/Power/`) is left out, and a
/// KiCad auto name ([`is_auto_name`]) reads as nothing
pub fn read_name(full: &str, strict: bool) -> NetName {
    if is_auto_name(full) {
        return NetName::default();
    }
    let base = full.rsplit('/').next().unwrap_or(full).to_uppercase();
    // `~{...}` overbars are not part of the word
    let base = base.replace("~{", "").replace('}', "");
    let tokens: Vec<&str> = base.split(['_', ' ']).filter(|t| !t.is_empty()).collect();
    let mut out = NetName::default();
    if tokens.is_empty() {
        return out;
    }
    // `SCL1`, `EN2`: a signal word with its number
    if tokens
        .iter()
        .any(|t| SIGNAL_WORDS.contains(&t.trim_end_matches(|c: char| c.is_ascii_digit())))
    {
        out.role = NameRole::Signal;
        return out;
    }
    out.negative = tokens
        .iter()
        .any(|t| matches!(*t, "NEG" | "VEE" | "VNEG" | "V-" | "N") || t.starts_with("VEE"));
    if let Some(kind) = tokens.iter().find_map(|t| ground_word(t)) {
        // `GND_ISO`, `ISO_GND`: an isolated ground
        let kind = if kind == GroundKind::Signal && tokens.contains(&"ISO") {
            GroundKind::Isolated
        } else {
            kind
        };
        out.role = NameRole::Ground(kind);
        return out;
    }
    let mut role = None;
    let mut voltage = None;
    for t in &tokens {
        if let Some(v) = voltage_token(t, strict) {
            voltage.get_or_insert(v);
            role.get_or_insert(RailKind::Supply);
            continue;
        }
        // A power word, alone or with a voltage or a number glued on (`VCC3V3`, `VDD1V8`, `VIN1`); `VDD` and
        // `VCC` also with letters (`VDDIO`, `VCCINT`, `VDDCORE`); `+V...` only with its `+`
        let matched = POWER_WORDS.iter().find_map(|(w, kind)| {
            let rest = if w.starts_with('+') || w.ends_with(['+', '-']) {
                t.strip_prefix(w)?
            } else {
                t.trim_start_matches('+').strip_prefix(w)?
            };
            let glued = voltage_token(rest, strict);
            let letters = matches!(*w, "VDD" | "VCC" | "AVDD" | "DVDD" | "PVDD")
                && rest.chars().all(|c| c.is_ascii_alphanumeric());
            (rest.is_empty()
                || glued.is_some()
                || rest.chars().all(|c| c.is_ascii_digit())
                || letters
                || *w == "+V")
                .then_some((*kind, glued))
        });
        if let Some((kind, glued)) = matched {
            if let Some(v) = glued {
                voltage.get_or_insert(v);
            }
            // The kind of a specific word wins over a plain supply
            if role.is_none_or(|r| r == RailKind::Supply) {
                role = Some(kind);
            }
            continue;
        }
        // `3V3A`: an analog rail by its suffix
        if let Some(v) = t.strip_suffix('A').and_then(|s| voltage_token(s, strict)) {
            voltage.get_or_insert(v);
            role = Some(RailKind::Analog);
        }
    }
    if let Some(mut kind) = role {
        if tokens
            .iter()
            .any(|t| matches!(*t, "A" | "ANA" | "ANALOG" | "AVDD"))
            && kind == RailKind::Supply
        {
            kind = RailKind::Analog;
        }
        out.role = NameRole::Rail(kind);
        if let Some((v, signed)) = voltage {
            if !signed && out.negative && v > 0.0 {
                out.voltage = Some(-v);
                out.sign_from_marker = true;
            } else {
                out.voltage = Some(v);
            }
        }
    }
    out
}

/// The voltage a rail name gives, volts, signed (`+3V3` is 3.3, `VDD_1V8` 1.8, `-5V` -5, `5V0_NEG` -5). Strict:
/// only from `+xVy`.
pub fn name_voltage(name: &str, strict: bool) -> Option<f64> {
    read_name(name, strict).voltage
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(n: &str) -> NameRole {
        read_name(n, false).role
    }

    #[test]
    fn voltages_in_names() {
        let strict = |n| name_voltage(n, true);
        assert_eq!(strict("+3V3"), Some(3.3));
        assert_eq!(strict("/VDD_+1V25"), Some(1.25));
        assert_eq!(strict("+12V0_IN"), Some(12.0));
        assert_eq!(strict("3V3"), None, "only with the +");
        assert_eq!(strict("+5V"), None, "no decimals");
        let loose = |n| name_voltage(n, false);
        assert_eq!(loose("VDD_1V8"), Some(1.8));
        assert_eq!(loose("VDD_12V5"), Some(12.5));
        assert_eq!(loose("+3.3V"), Some(3.3));
        assert_eq!(loose("+5V"), Some(5.0));
        assert_eq!(loose("-5V"), Some(-5.0));
        assert_eq!(loose("-12V0"), Some(-12.0));
        assert_eq!(loose("5V0_NEG"), Some(-5.0));
        assert_eq!(loose("VCC3V3"), Some(3.3));
        assert_eq!(loose("3P3V"), Some(3.3));
        assert_eq!(loose("/Power/+3V3_LDO"), Some(3.3));
        assert_eq!(loose("VBUS"), None);
        assert_eq!(loose("SCL1_3V3"), None, "a signal");
        assert!(read_name("5V0_NEG", false).sign_from_marker);
        assert!(!read_name("-5V0", false).sign_from_marker);
    }

    #[test]
    fn auto_names_read_as_nothing() {
        for n in [
            "unconnected-(J3-Shield-Pad6)",
            "Net-(U1-GND)",
            "Net-(R1-Pad2)",
            "/Power/Net-(C3-Pad1)",
            "unconnected-(U2-VDD-Pad1)",
        ] {
            assert!(is_auto_name(n), "{n}");
            assert_eq!(role(n), NameRole::None, "{n}");
        }
        assert!(!is_auto_name("NET_1"));
        assert_eq!(role("SHIELD"), NameRole::Ground(GroundKind::Chassis));
    }

    #[test]
    fn ground_pin_words() {
        assert_eq!(ground_pin_word("GND"), Some(GroundKind::Signal));
        assert_eq!(ground_pin_word("AGND"), Some(GroundKind::Analog));
        assert_eq!(ground_pin_word("VSS"), Some(GroundKind::Signal));
        assert_eq!(
            ground_pin_word("Shield"),
            None,
            "a shell pin makes no ground"
        );
        assert_eq!(ground_pin_word("EARTH"), None);
        assert_eq!(ground_pin_word("VDD"), None);
    }

    #[test]
    fn roles_by_name() {
        assert_eq!(role("+3V3"), NameRole::Rail(RailKind::Supply));
        assert_eq!(role("VBUS"), NameRole::Rail(RailKind::Usb));
        assert_eq!(role("/USB/VBUS"), NameRole::Rail(RailKind::Usb));
        assert_eq!(role("VBAT"), NameRole::Rail(RailKind::Battery));
        assert_eq!(role("VDDA"), NameRole::Rail(RailKind::Analog));
        assert_eq!(role("+3V3A"), NameRole::Rail(RailKind::Analog));
        assert_eq!(role("3V3_ANA"), NameRole::Rail(RailKind::Analog));
        assert_eq!(role("VREF"), NameRole::Rail(RailKind::Reference));
        assert_eq!(role("+VIN"), NameRole::Rail(RailKind::Supply));
        assert_eq!(role("+VDC"), NameRole::Rail(RailKind::Supply));
        assert_eq!(role("V+"), NameRole::Rail(RailKind::Supply));
        assert_eq!(role("VIN1"), NameRole::Rail(RailKind::Supply));
        assert_eq!(
            role("VEE"),
            NameRole::Rail(RailKind::Supply),
            "a negative rail, no ground"
        );
        assert!(read_name("VEE", false).negative);
        assert_eq!(role("GND"), NameRole::Ground(GroundKind::Signal));
        assert_eq!(role("AGND"), NameRole::Ground(GroundKind::Analog));
        assert_eq!(role("PGND"), NameRole::Ground(GroundKind::Power));
        assert_eq!(role("GND_ISO"), NameRole::Ground(GroundKind::Isolated));
        assert_eq!(role("ISO_GND"), NameRole::Ground(GroundKind::Isolated));
        assert_eq!(role("CHASSIS"), NameRole::Ground(GroundKind::Chassis));
        assert_eq!(role("VSS"), NameRole::Ground(GroundKind::Signal));
        assert_eq!(role("GND2"), NameRole::Ground(GroundKind::Signal));
        assert_eq!(role("VDD_EN"), NameRole::Signal);
        assert_eq!(role("PWR_EN"), NameRole::Signal);
        assert_eq!(role("SCL1_3V3"), NameRole::Signal);
        assert_eq!(role("PGND_SNS"), NameRole::Signal);
        assert_eq!(
            role("ISO_5V"),
            NameRole::Rail(RailKind::Supply),
            "ISO is no ground by itself"
        );
        assert_eq!(role("/MCU/SWDIO"), NameRole::None);
        assert_eq!(role("VDDIO"), NameRole::Rail(RailKind::Supply));
        assert_eq!(role("VCCINT_1V0"), NameRole::Rail(RailKind::Supply));
        assert_eq!(role("VSYNC"), NameRole::None, "+V only with its +");
        assert_eq!(role("VINT"), NameRole::None);
    }
}
