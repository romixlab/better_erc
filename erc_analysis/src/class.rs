//! Part classes: what kind of part a designator is ([`PartClass`]: a resistor, a P-channel MOSFET, a buck
//! regulator), each a [`Fact`] with the rule that decided it.
//!
//! Resolution order (architecture §1.3):
//!
//! 1. The part database by MPN: a [`ClassSource`] given to [`PartClasses::with`] (CFG-5 plugs in here; none is
//!    built yet, so [`PartClasses::of`] has no source).
//! 2. The library symbol: the library and symbol name (`Device:R`, `Transistor_FET:BSS138`,
//!    `Regulator_Linear:AMS1117-3.3`).
//! 3. The symbol's description and keywords, and the part's Description field (`N-Channel MOSFET`, `Dual
//!    bidirectional I2C-bus voltage-level translator`).
//! 4. Net-tie or jumper pad groups in the footprint or symbol.
//! 5. The designator prefix (`R`, `Q`, `J`): the last resort, marked [`Provenance::Assumed`], so a finding that
//!    relies on it is an estimate and says so (NAME-12 style findings tell the user to fix the library instead).
//!
//! A class found by name with its kind left open (a `Transistor_FET` symbol) takes the kind from the description
//! when that names it (`P-Channel`); a kind no source names stays `None`.

use erc_model::class::*;
use erc_model::{Board, Fact, PartId, Provenance};
use serde::{Deserialize, Serialize};

pub use erc_model::PartClass;

/// A source of part classes ranked above the schematic's own: the part database by MPN (CFG-5), mx3
pub trait ClassSource {
    /// The class of `part`, with its provenance (`Provenance::PartDb`, `Provenance::Mx3`), or `None` when the
    /// source does not know the part
    fn class(&self, board: &Board, part: PartId) -> Option<Fact<PartClass>>;
}

/// The class of every part of a board, by [`PartId`]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartClasses(pub Vec<Fact<PartClass>>);

/// How many components have a class from a real source (part data, library symbol, description) rather than
/// assumed from the designator prefix or not known at all: the coverage of `Need::PartClasses`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassCoverage {
    /// Components classified by part data, the library symbol or its description
    pub known: usize,
    /// Components classified by their designator prefix only (assumed)
    pub assumed: usize,
    /// Components of unknown class
    pub unknown: usize,
}

impl ClassCoverage {
    pub fn components(&self) -> usize {
        self.known + self.assumed + self.unknown
    }
}

impl PartClasses {
    /// Classifies every part of `board` from the schematic alone
    pub fn of(board: &Board) -> PartClasses {
        PartClasses::with(board, &[])
    }

    /// Classifies every part of `board`, asking `sources` first (in order)
    pub fn with(board: &Board, sources: &[&dyn ClassSource]) -> PartClasses {
        PartClasses(
            (0..board.parts.len())
                .map(PartId::from_index)
                .map(|p| {
                    sources
                        .iter()
                        .find_map(|s| s.class(board, p))
                        .unwrap_or_else(|| classify(board, p))
                })
                .collect(),
        )
    }

    pub fn get(&self, part: PartId) -> PartClass {
        self.0[part.index()].value
    }

    /// The class with its source
    pub fn fact(&self, part: PartId) -> &Fact<PartClass> {
        &self.0[part.index()]
    }

    /// Exactly this class, kind included (`PartClass::Resistor`); for a family use the predicates of
    /// [`PartClass`] (`classes.get(p).is_mosfet()`)
    pub fn is(&self, part: PartId, class: PartClass) -> bool {
        self.get(part) == class
    }

    /// Whether the class was only assumed from the designator prefix
    pub fn assumed(&self, part: PartId) -> bool {
        matches!(self.fact(part).source, Provenance::Assumed { .. })
    }

    /// How well the components of `board` are classified
    pub fn coverage(&self, board: &Board) -> ClassCoverage {
        let mut c = ClassCoverage::default();
        for i in 0..board.parts.len() {
            let p = PartId::from_index(i);
            if !board.is_component(p) {
                continue;
            }
            if self.get(p) == PartClass::Unknown {
                c.unknown += 1;
            } else if self.assumed(p) {
                c.assumed += 1;
            } else {
                c.known += 1;
            }
        }
        c
    }
}

const BY_LIBRARY: &str = "class from the library symbol";
const BY_DESCRIPTION: &str = "class from the symbol's description and keywords";
const BY_TIES: &str = "class from the net-tie or jumper pad groups";
const BY_PREFIX: &str = "class from the designator prefix";

/// The class of one part from the schematic: library symbol, description and keywords, pad groups, then the
/// designator prefix (assumed)
pub fn classify(board: &Board, part: PartId) -> Fact<PartClass> {
    let p = board.part(part);
    let derived = |class, rule: &str| Fact::new(class, Provenance::Derived { rule: rule.into() });
    let (lib, name) = p.lib_id.split_once(':').unwrap_or(("", &p.lib_id));
    let symbol = p.symbol.map(|s| &board.symbols[s.index()]);
    let text = Text::new(&format!(
        "{} {} {} {}",
        symbol.map_or("", |s| s.description.as_str()),
        symbol.map_or("", |s| s.keywords.as_str()),
        p.field("Description").unwrap_or(""),
        name,
    ));
    if let Some(class) = by_library(lib, name) {
        return derived(refine(class, &text), BY_LIBRARY);
    }
    if let Some(class) = by_description(&text) {
        return derived(refine(class, &text), BY_DESCRIPTION);
    }
    let jumper_pins =
        symbol.is_some_and(|s| s.duplicate_pins_are_jumpers || !s.jumper_pin_groups.is_empty());
    if !p.ties.is_empty() {
        let class = if jumper_pins {
            PartClass::Jumper
        } else {
            PartClass::NetTie
        };
        return derived(class, BY_TIES);
    }
    Fact::new(
        by_prefix(&p.designator),
        Provenance::Assumed {
            rule: BY_PREFIX.into(),
        },
    )
}

/// Lower-case text with a word-boundary search: `led` is found in `LED_Small` and `green LED`, not in
/// `isolated`
struct Text(String);

impl Text {
    fn new(s: &str) -> Text {
        Text(s.to_lowercase())
    }

    /// `phrase` (lower case) in the text, not inside a longer word
    fn has(&self, phrase: &str) -> bool {
        let t = self.0.as_str();
        let alnum = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
        let mut from = 0;
        while let Some(i) = t[from..].find(phrase) {
            let at = from + i;
            let end = at + phrase.len();
            if !alnum(t[..at].chars().next_back()) && !alnum(t[end..].chars().next()) {
                return true;
            }
            from = at + t[at..].chars().next().map_or(1, char::len_utf8);
        }
        false
    }

    fn any(&self, phrases: &[&str]) -> bool {
        phrases.iter().any(|p| self.has(p))
    }

    /// A word (`_` kept inside: a symbol name is one word) starts with `prefix` (`74lvc` in `74lvc1g04`, `usb_c` in
    /// `usb_c_receptacle`)
    fn word_starting(&self, prefix: &str) -> bool {
        self.0
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|w| w.starts_with(prefix))
    }
}

/// From the library and symbol names (`Device:R_Small`, `Transistor_FET:BSS138`, `Connector:USB_C_Receptacle`)
fn by_library(lib: &str, name: &str) -> Option<PartClass> {
    let base = name.split('_').next().unwrap_or(name);
    let starts = |p: &str| name.starts_with(p);
    let prefixed = |p: &str| lib.starts_with(p);
    Some(match lib {
        "Transistor_FET" => PartClass::Mosfet { channel: None },
        "Transistor_BJT" => PartClass::Bjt { polarity: None },
        "Connector"
        | "Connector_Generic"
        | "Connector_Generic_MountingPin"
        | "Connector_Generic_Shielded"
        | "Connector_Audio" => {
            if starts("TestPoint") {
                PartClass::TestPoint
            } else {
                PartClass::Connector {
                    kind: connector_kind(&Text::new(name)),
                }
            }
        }
        "LED" => PartClass::Diode {
            kind: Some(DiodeKind::Led),
        },
        "Diode" => PartClass::Diode { kind: None },
        "Crystal" => PartClass::Crystal,
        "Oscillator" => PartClass::Oscillator,
        "Fuse" => PartClass::Fuse { kind: None },
        "Switch" => PartClass::Switch { kind: None },
        "Relay" | "Relay_SolidState" => PartClass::Relay,
        "Motor" => PartClass::Motor,
        "Jumper" => PartClass::Jumper,
        "Mechanical" if starts("MountingHole") => PartClass::MountingHole,
        "Mechanical" if starts("Fiducial") => PartClass::Fiducial,
        "Regulator_Linear" => PartClass::Regulator {
            kind: Some(RegulatorKind::Ldo),
        },
        "Regulator_Switching" => PartClass::Regulator { kind: None },
        "Logic_LevelTranslator" => PartClass::Translator { class: None },
        "Isolator" => PartClass::Isolator,
        "Amplifier_Operational" => PartClass::Opamp,
        "Comparator" => PartClass::Comparator,
        "Memory_Flash" => PartClass::Memory {
            kind: Some(MemoryKind::Flash),
        },
        "Memory_EEPROM" => PartClass::Memory {
            kind: Some(MemoryKind::Eeprom),
        },
        "Memory_RAM" => PartClass::Memory { kind: None },
        "Interface_CAN_LIN" => PartClass::Transceiver { kind: None },
        "74xx" | "74xGxx" | "4xxx" => PartClass::Logic { family: None },
        _ if prefixed("MCU_") => PartClass::Mcu,
        _ if prefixed("CPU_") || prefixed("MPU_") => PartClass::Soc,
        _ if prefixed("FPGA_") => PartClass::Fpga,
        _ => match base {
            "R" => PartClass::Resistor,
            "C" => PartClass::Capacitor {
                polarized: starts("C_Polarized"),
            },
            "CP" => PartClass::Capacitor { polarized: true },
            "L" => PartClass::Inductor,
            "FerriteBead" => PartClass::FerriteBead,
            "LED" => PartClass::Diode {
                kind: Some(DiodeKind::Led),
            },
            "D" => PartClass::Diode { kind: None },
            "NetTie" => PartClass::NetTie,
            "TestPoint" => PartClass::TestPoint,
            "Crystal" | "Resonator" => PartClass::Crystal,
            "Fuse" => PartClass::Fuse {
                kind: Some(FuseKind::Fuse),
            },
            "Polyfuse" => PartClass::Fuse {
                kind: Some(FuseKind::Resettable),
            },
            "Battery" => PartClass::Battery,
            "Transformer" => PartClass::Transformer,
            "Jumper" | "SolderJumper" => PartClass::Jumper,
            "Q" if name.contains("MOS") => PartClass::Mosfet { channel: None },
            "Q" if name.contains("NPN") || name.contains("PNP") => {
                PartClass::Bjt { polarity: None }
            }
            _ => return None,
        },
    })
}

/// From the description, keywords and symbol name, lower case; the order matters (a `gate driver` is no
/// MOSFET, a `crystal oscillator` no crystal, a `load switch` no switch)
fn by_description(t: &Text) -> Option<PartClass> {
    use PartClass as P;
    Some(
        if t.any(&[
            "translator",
            "level shifter",
            "level-shifter",
            "level translator",
        ]) || t.any(&["bus buffer", "bus repeater", "i2c repeater"])
        {
            P::Translator { class: None }
        } else if t.any(&["isolator", "digital isolator"]) {
            P::Isolator
        } else if t.any(&["microcontroller", "mcu"]) {
            P::Mcu
        } else if t.has("fpga") {
            P::Fpga
        } else if t.any(&[
            "application processor",
            "soc",
            "system on chip",
            "microprocessor",
        ]) {
            P::Soc
        } else if t.has("driver") {
            // Gate, LED, motor, relay drivers are ICs, not the part they drive
            P::Ic
        } else if t.any(&[
            "load switch",
            "efuse",
            "e-fuse",
            "power switch",
            "power distribution switch",
        ]) {
            P::LoadSwitch
        } else if t.any(&[
            "ldo",
            "low dropout",
            "low-dropout",
            "linear regulator",
            "voltage regulator",
            "buck",
            "step-down",
            "step down",
            "boost",
            "step-up",
            "step up",
            "dc-dc",
            "dc/dc",
            "switching regulator",
            "charger",
        ]) {
            P::Regulator { kind: None }
        } else if t.any(&[
            "can transceiver",
            "can bus transceiver",
            "lin transceiver",
            "rs-485",
            "rs485",
            "rs-232",
            "rs232",
            "ethernet phy",
            "ethernet transceiver",
            "usb phy",
            "usb transceiver",
        ]) {
            P::Transceiver { kind: None }
        } else if t.any(&["operational amplifier", "op-amp", "opamp", "op amp"]) {
            P::Opamp
        } else if t.has("comparator") {
            P::Comparator
        } else if t.any(&[
            "eeprom",
            "flash memory",
            "nor flash",
            "nand flash",
            "sram",
            "sdram",
            "ddr",
            "ddr3",
            "ddr4",
            "lpddr4",
            "fram",
            "memory",
        ]) {
            P::Memory { kind: None }
        } else if t.any(&[
            "mosfet",
            "n-channel",
            "p-channel",
            "n channel",
            "p channel",
            "nmos",
            "pmos",
        ]) {
            P::Mosfet { channel: None }
        } else if t.any(&["npn", "pnp", "bipolar transistor", "bjt"]) {
            P::Bjt { polarity: None }
        } else if t.has("oscillator") {
            P::Oscillator
        } else if t.any(&["crystal", "resonator"]) {
            P::Crystal
        } else if t.has("relay") {
            P::Relay
        } else if t.any(&["motor", "stepper", "brushless"]) {
            P::Motor
        } else if t.any(&["solenoid", "solenoid valve", "valve"]) {
            P::Solenoid
        } else if t.has("transformer") {
            P::Transformer
        } else if t.any(&["polyfuse", "ptc", "resettable fuse", "fuse"]) {
            P::Fuse { kind: None }
        } else if t.any(&["battery cell", "battery holder", "coin cell", "battery"]) {
            P::Battery
        } else if t.any(&[
            "zener",
            "tvs",
            "transient voltage",
            "esd protection",
            "schottky",
            "rectifier",
            "diode",
            "led",
            "light emitting",
        ]) {
            P::Diode { kind: None }
        } else if t.has("resistor") && !t.any(&["array", "network"]) {
            P::Resistor
        } else if t.has("capacitor") {
            P::Capacitor {
                polarized: polarized(t),
            }
        } else if t.any(&["ferrite", "ferrite bead"]) {
            P::FerriteBead
        } else if t.has("inductor") {
            P::Inductor
        } else if t.any(&["net tie", "nettie"]) {
            P::NetTie
        } else if t.any(&["jumper", "solder jumper"]) {
            P::Jumper
        } else if t.any(&["test point", "testpoint"]) {
            P::TestPoint
        } else if t.any(&["mounting hole"]) {
            P::MountingHole
        } else if t.has("fiducial") {
            P::Fiducial
        } else if t.any(&[
            "connector",
            "receptacle",
            "plug",
            "jack",
            "pin header",
            "socket",
            "terminal block",
        ]) {
            P::Connector { kind: None }
        } else if t.any(&["switch", "push button", "pushbutton", "tactile"]) {
            P::Switch { kind: None }
        } else {
            return None;
        },
    )
}

fn polarized(t: &Text) -> bool {
    t.any(&[
        "polarized",
        "polarised",
        "electrolytic",
        "tantalum",
        "polymer",
    ])
}

fn connector_kind(t: &Text) -> Option<ConnectorKind> {
    use ConnectorKind as K;
    Some(
        if t.word_starting("usb_c") || t.any(&["usb c", "usb-c", "type-c", "type c"]) {
            K::UsbC
        } else if t.word_starting("usb_b_micro")
            || t.word_starting("usb_micro")
            || t.any(&["micro usb", "micro-usb", "microusb", "usb micro"])
        {
            K::UsbMicro
        } else if t.word_starting("usb_a") || t.any(&["usb a", "usb-a", "usb type a"]) {
            K::UsbA
        } else if t.word_starting("usb_b") || t.any(&["usb b", "usb-b"]) {
            K::UsbB
        } else if t.word_starting("hdmi_d")
            || t.any(&["micro hdmi", "micro-hdmi", "microhdmi", "hdmi type d"])
        {
            K::MicroHdmi
        } else if t.word_starting("hdmi") {
            K::Hdmi
        } else if t.word_starting("rj45") || t.has("8p8c") {
            K::Rj45
        } else if t.word_starting("barrel") || t.has("dc jack") {
            K::Barrel
        } else if t.word_starting("screw_terminal") || t.any(&["terminal block", "screw terminal"])
        {
            K::TerminalBlock
        } else if t.word_starting("micro_sd")
            || t.word_starting("sd_card")
            || t.any(&["sd card", "microsd", "micro sd", "card socket"])
        {
            K::Card
        } else if t.any(&["fpc", "ffc"]) {
            K::Fpc
        } else if t.word_starting("conn_coaxial")
            || t.any(&["coaxial", "sma", "u.fl", "ufl", "mmcx"])
        {
            K::Coax
        } else if t.word_starting("conn_") || t.any(&["pin header", "header", "pin socket"]) {
            K::Header
        } else {
            return None;
        },
    )
}

/// Fills the kind a class left open from the text (description, keywords, symbol name)
fn refine(class: PartClass, t: &Text) -> PartClass {
    use PartClass as P;
    match class {
        P::Capacitor { polarized: false } => P::Capacitor {
            polarized: polarized(t),
        },
        P::Mosfet { channel: None } => P::Mosfet {
            channel: if t.any(&["p-channel", "p channel", "pmos", "pfet", "p-mosfet"])
                || t.word_starting("q_pmos")
            {
                Some(Channel::P)
            } else if t.any(&["n-channel", "n channel", "nmos", "nfet", "n-mosfet"])
                || t.word_starting("q_nmos")
            {
                Some(Channel::N)
            } else {
                None
            },
        },
        P::Bjt { polarity: None } => P::Bjt {
            polarity: if t.has("pnp") || t.word_starting("q_pnp") {
                Some(BjtPolarity::Pnp)
            } else if t.has("npn") || t.word_starting("q_npn") {
                Some(BjtPolarity::Npn)
            } else {
                None
            },
        },
        P::Diode { kind: None } => P::Diode {
            kind: if t.any(&["led", "light emitting"]) {
                Some(DiodeKind::Led)
            } else if t.any(&["tvs", "transient voltage", "esd protection"])
                || t.word_starting("d_tvs")
            {
                Some(DiodeKind::Tvs {
                    bidirectional: if t.any(&["bidirectional", "bi-directional", "bidir"]) {
                        Some(true)
                    } else if t.any(&["unidirectional", "uni-directional"]) {
                        Some(false)
                    } else {
                        None
                    },
                })
            } else if t.has("zener") || t.word_starting("d_zener") {
                Some(DiodeKind::Zener)
            } else if t.has("schottky") || t.word_starting("d_schottky") {
                Some(DiodeKind::Schottky)
            } else if t.any(&[
                "rectifier",
                "switching diode",
                "standard diode",
                "fast recovery",
            ]) {
                Some(DiodeKind::Rectifier)
            } else {
                None
            },
        },
        P::Connector { kind: None } => P::Connector {
            kind: connector_kind(t),
        },
        P::Switch { kind: None } => P::Switch {
            kind: if t.any(&["push button", "pushbutton", "push", "tactile", "momentary"])
                || t.word_starting("sw_push")
            {
                Some(SwitchKind::Momentary)
            } else if t.has("slide") || t.word_starting("sw_slide") {
                Some(SwitchKind::Slide)
            } else if t.has("dip") || t.word_starting("sw_dip") {
                Some(SwitchKind::Dip)
            } else if t.any(&["toggle", "rocker", "spdt", "spst", "dpdt"]) {
                Some(SwitchKind::Toggle)
            } else {
                None
            },
        },
        P::Regulator { kind: None } => P::Regulator {
            kind: if t.any(&["charger", "battery charger", "charge management"]) {
                Some(RegulatorKind::Charger)
            } else if t.any(&["buck-boost", "buck boost", "buckboost"]) {
                Some(RegulatorKind::BuckBoost)
            } else if t.any(&["buck", "step-down", "step down"]) {
                Some(RegulatorKind::Buck)
            } else if t.any(&["boost", "step-up", "step up"]) {
                Some(RegulatorKind::Boost)
            } else if t.any(&["ldo", "low dropout", "low-dropout", "linear regulator"]) {
                Some(RegulatorKind::Ldo)
            } else {
                None
            },
        },
        P::Translator { class: None } => P::Translator {
            class: if t.any(&["bus buffer", "repeater", "buffer"]) && !t.has("translator") {
                Some(TranslatorClass::Buffer)
            } else if t.any(&[
                "direction control",
                "direction pin",
                "dir pin",
                "bus transceiver",
            ]) {
                Some(TranslatorClass::DirectionPin)
            } else if t.any(&["auto-direction", "auto direction", "push-pull"])
                && !t.has("open-drain")
            {
                Some(TranslatorClass::AutoDirection)
            } else if t.any(&[
                "open-drain",
                "open drain",
                "i2c",
                "smbus",
                "pass fet",
                "pass-fet",
            ]) {
                Some(TranslatorClass::PassFet)
            } else {
                None
            },
        },
        P::Memory { kind: None } => P::Memory {
            kind: if t.any(&["eeprom"]) {
                Some(MemoryKind::Eeprom)
            } else if t.any(&["fram", "feram"]) {
                Some(MemoryKind::Fram)
            } else if t.any(&[
                "sdram", "dram", "ddr", "ddr2", "ddr3", "ddr3l", "ddr4", "lpddr4",
            ]) {
                Some(MemoryKind::Dram)
            } else if t.any(&["sram", "psram"]) {
                Some(MemoryKind::Sram)
            } else if t.any(&["flash"]) {
                Some(MemoryKind::Flash)
            } else {
                None
            },
        },
        P::Transceiver { kind: None } => P::Transceiver {
            kind: if t.any(&["lin transceiver", "lin bus"]) {
                Some(TransceiverKind::Lin)
            } else if t.any(&["can", "can fd", "can-fd"]) {
                Some(TransceiverKind::Can)
            } else if t.any(&["rs-485", "rs485", "rs-422"]) {
                Some(TransceiverKind::Rs485)
            } else if t.any(&["rs-232", "rs232"]) {
                Some(TransceiverKind::Rs232)
            } else if t.any(&["ethernet", "phy"]) && !t.has("usb") {
                Some(TransceiverKind::EthPhy)
            } else if t.has("usb") {
                Some(TransceiverKind::Usb)
            } else {
                None
            },
        },
        P::Logic { family: None } => P::Logic {
            family: logic_family(t),
        },
        P::Fuse { kind: None } => P::Fuse {
            kind: if t.any(&["polyfuse", "ptc", "resettable"]) {
                Some(FuseKind::Resettable)
            } else if t.has("fuse") {
                Some(FuseKind::Fuse)
            } else {
                None
            },
        },
        other => other,
    }
}

/// `74LVC1G04` is LVC: the family letters after `74` (longest first)
fn logic_family(t: &Text) -> Option<LogicFamily> {
    use LogicFamily as F;
    for (p, f) in [
        ("74ahct", F::Ahct),
        ("74ahc", F::Ahc),
        ("74hct", F::Hct),
        ("74hc", F::Hc),
        ("74lvc", F::Lvc),
        ("74lv", F::Lv),
        ("74avc", F::Avc),
        ("74auc", F::Auc),
        ("74act", F::Act),
        ("74ac", F::Ac),
        ("sn74ahct", F::Ahct),
        ("sn74ahc", F::Ahc),
        ("sn74hct", F::Hct),
        ("sn74hc", F::Hc),
        ("sn74lvc", F::Lvc),
        ("sn74lv", F::Lv),
        ("sn74avc", F::Avc),
        ("sn74auc", F::Auc),
        ("sn74act", F::Act),
        ("sn74ac", F::Ac),
    ] {
        if t.word_starting(p) {
            return Some(f);
        }
    }
    None
}

/// The designator prefix (letters before the number): the last resort
fn by_prefix(designator: &str) -> PartClass {
    use PartClass as P;
    let prefix: String = designator
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_uppercase();
    match prefix.as_str() {
        "R" | "RN" => P::Resistor,
        "C" => P::Capacitor { polarized: false },
        "L" => P::Inductor,
        "FB" => P::FerriteBead,
        "D" => P::Diode { kind: None },
        "LED" => P::Diode {
            kind: Some(DiodeKind::Led),
        },
        "ZD" => P::Diode {
            kind: Some(DiodeKind::Zener),
        },
        "Q" | "T" => P::Transistor,
        "J" | "P" | "CN" | "X" => P::Connector { kind: None },
        "TP" => P::TestPoint,
        "U" | "IC" => P::Ic,
        "NT" => P::NetTie,
        "JP" | "SJ" => P::Jumper,
        "Y" => P::Crystal,
        "F" => P::Fuse { kind: None },
        "SW" | "S" => P::Switch { kind: None },
        "K" | "RL" => P::Relay,
        "M" | "MOT" => P::Motor,
        "BT" => P::Battery,
        "TR" => P::Transformer,
        "H" | "MH" => P::MountingHole,
        "FID" => P::Fiducial,
        _ => P::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PartClass as P;

    fn desc(text: &str) -> Option<PartClass> {
        let t = Text::new(text);
        by_description(&t).map(|c| refine(c, &t))
    }

    fn lib(lib_id: &str, description: &str) -> Option<PartClass> {
        let (l, n) = lib_id.split_once(':').unwrap();
        by_library(l, n).map(|c| refine(c, &Text::new(&format!("{description} {n}"))))
    }

    #[test]
    fn words() {
        let t = Text::new("Isolated DC-DC, controlled; green LED_Small");
        assert!(t.has("led"));
        assert!(t.has("dc-dc"));
        assert!(!Text::new("isolated controlled").has("led"));
    }

    #[test]
    fn library_then_description_then_prefix() {
        assert_eq!(lib("Device:R_Small", ""), Some(P::Resistor));
        assert_eq!(lib("Fixture:R", ""), Some(P::Resistor));
        assert_eq!(lib("Device:C", ""), Some(P::Capacitor { polarized: false }));
        assert_eq!(
            lib("Device:C_Polarized", ""),
            Some(P::Capacitor { polarized: true })
        );
        assert_eq!(
            lib("Device:CP_Small", ""),
            Some(P::Capacitor { polarized: true })
        );
        assert_eq!(
            lib(
                "Transistor_FET:BSS138",
                "50V Vds, 0.22A Id, N-Channel MOSFET, SOT-23"
            ),
            Some(P::Mosfet {
                channel: Some(Channel::N)
            })
        );
        assert_eq!(
            lib(
                "Transistor_FET:AO3401A",
                "-4.0A Id, -30V Vds, P-Channel MOSFET, SOT-23"
            ),
            Some(P::Mosfet {
                channel: Some(Channel::P)
            })
        );
        assert_eq!(
            lib("Device:Q_NMOS_GSD", ""),
            Some(P::Mosfet {
                channel: Some(Channel::N)
            })
        );
        assert_eq!(
            lib("Device:Q_PNP_BEC", ""),
            Some(P::Bjt {
                polarity: Some(BjtPolarity::Pnp)
            })
        );
        assert_eq!(
            lib("Connector:Conn_01x03_Pin", ""),
            Some(P::Connector {
                kind: Some(ConnectorKind::Header)
            })
        );
        assert_eq!(
            lib("Connector:USB_C_Receptacle_USB2.0_16P", ""),
            Some(P::Connector {
                kind: Some(ConnectorKind::UsbC)
            })
        );
        assert_eq!(
            lib("Connector:HDMI_D_1.4", ""),
            Some(P::Connector {
                kind: Some(ConnectorKind::MicroHdmi)
            })
        );
        assert_eq!(lib("Connector:TestPoint", ""), Some(P::TestPoint));
        assert_eq!(
            lib("Device:D_Zener", ""),
            Some(P::Diode {
                kind: Some(DiodeKind::Zener)
            })
        );
        assert_eq!(
            lib(
                "Diode:SMAJ5.0CA",
                "400W bidirectional Transient Voltage Suppressor, 5.0Vr, SMA(DO-214AC)"
            ),
            Some(P::Diode {
                kind: Some(DiodeKind::Tvs {
                    bidirectional: Some(true)
                })
            })
        );
        assert_eq!(
            lib("Diode:BAT54W", "30V 0.2A Schottky diode"),
            Some(P::Diode {
                kind: Some(DiodeKind::Schottky)
            })
        );
        assert_eq!(
            lib("Device:LED_Small", ""),
            Some(P::Diode {
                kind: Some(DiodeKind::Led)
            })
        );
        assert_eq!(
            lib(
                "Regulator_Linear:AMS1117-3.3",
                "1A Low Dropout regulator, positive, 3.3V fixed output"
            ),
            Some(P::Regulator {
                kind: Some(RegulatorKind::Ldo)
            })
        );
        assert_eq!(
            lib(
                "Regulator_Switching:TPS62130",
                "3-17V 3A Step-Down Converter"
            ),
            Some(P::Regulator {
                kind: Some(RegulatorKind::Buck)
            })
        );
        assert_eq!(lib("MCU_ST_STM32G0:STM32G071KBTx", ""), Some(P::Mcu));
        assert_eq!(
            lib("74xGxx:74LVC1G04", "Single NOT Gate, Low-Voltage CMOS"),
            Some(P::Logic {
                family: Some(LogicFamily::Lvc)
            })
        );
        assert_eq!(
            lib("Device:Polyfuse_Small", ""),
            Some(P::Fuse {
                kind: Some(FuseKind::Resettable)
            })
        );
        assert_eq!(
            lib("Switch:SW_Push", ""),
            Some(P::Switch {
                kind: Some(SwitchKind::Momentary)
            })
        );
        assert_eq!(lib("Interface:PCA9306", ""), None);
        assert_eq!(lib("Fixture:NMOS", ""), None);

        assert_eq!(
            desc("n-channel mosfet"),
            Some(P::Mosfet {
                channel: Some(Channel::N)
            })
        );
        assert_eq!(
            desc("dual bidirectional i2c-bus and smbus voltage-level translator"),
            Some(P::Translator {
                class: Some(TranslatorClass::PassFet)
            })
        );
        assert_eq!(
            desc(
                "4-bit bidirectional voltage-level translator with auto-direction sensing, push-pull"
            ),
            Some(P::Translator {
                class: Some(TranslatorClass::AutoDirection)
            })
        );
        assert_eq!(
            desc("Dual bidirectional I2C bus buffer"),
            Some(P::Translator {
                class: Some(TranslatorClass::Buffer)
            })
        );
        assert_eq!(desc("4-bit i2c-bus and smbus i/o port"), None);
        assert_eq!(
            desc("Half-bridge gate driver for N-channel MOSFETs"),
            Some(P::Ic)
        );
        assert_eq!(
            desc("High-speed CAN transceiver"),
            Some(P::Transceiver {
                kind: Some(TransceiverKind::Can)
            })
        );
        assert_eq!(desc("Low power op-amp, rail to rail"), Some(P::Opamp));
        assert_eq!(desc("DC Motor"), Some(P::Motor));
        assert_eq!(
            desc("Stepper motor driver"),
            Some(P::Ic),
            "a driver, not the motor"
        );
        assert_eq!(desc("12 V solenoid valve"), Some(P::Solenoid));
        assert_eq!(desc("32.768 kHz crystal oscillator"), Some(P::Oscillator));
        assert_eq!(
            desc("Isolated DC-DC converter"),
            Some(P::Regulator { kind: None }),
            "isolated is not an isolator"
        );
        assert_eq!(
            desc("Aluminium electrolytic capacitor"),
            Some(P::Capacitor { polarized: true })
        );
        assert_eq!(
            desc("USB 2.0 Type-C receptacle"),
            Some(P::Connector {
                kind: Some(ConnectorKind::UsbC)
            })
        );
        assert_eq!(
            desc("2.5 A load switch with reverse current blocking"),
            Some(P::LoadSwitch)
        );
        assert_eq!(
            desc("Tactile switch, SPST"),
            Some(P::Switch {
                kind: Some(SwitchKind::Momentary)
            })
        );
        assert_eq!(
            desc("64 Mbit serial flash memory"),
            Some(P::Memory {
                kind: Some(MemoryKind::Flash)
            })
        );

        assert_eq!(by_prefix("U12"), P::Ic);
        assert_eq!(by_prefix("TP3"), P::TestPoint);
        assert_eq!(
            by_prefix("LED1"),
            P::Diode {
                kind: Some(DiodeKind::Led)
            }
        );
        assert_eq!(by_prefix("BT1"), P::Battery);
        assert_eq!(by_prefix("#PWR01"), P::Unknown);
    }
}
