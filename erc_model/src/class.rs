//! Part classes: what kind of part a designator is (a resistor, a P-channel MOSFET, a buck regulator, a USB-C
//! receptacle). The class is a fact like any other: `erc_analysis::class` resolves it from the part database (when
//! there is one), the library symbol, its keywords and description, and as the last resort from the designator
//! prefix, marked assumed.
//!
//! A kind the source does not tell (a `D` whose diode kind is unknown, a MOSFET whose channel is not named) is
//! `None` in its field, never a guess.

use crate::value::Unit;
use serde::{Deserialize, Serialize};

/// What kind of part
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PartClass {
    Resistor,
    /// A capacitor; `polarized` for electrolytic, tantalum and polymer parts
    Capacitor {
        polarized: bool,
    },
    Inductor,
    FerriteBead,
    Diode {
        kind: Option<DiodeKind>,
    },
    Bjt {
        polarity: Option<BjtPolarity>,
    },
    Mosfet {
        channel: Option<Channel>,
    },
    /// A transistor whose kind (BJT or MOSFET) is not known: a `Q` designator only
    Transistor,
    Connector {
        kind: Option<ConnectorKind>,
    },
    TestPoint,
    Switch {
        kind: Option<SwitchKind>,
    },
    Relay,
    Crystal,
    /// A crystal oscillator or other active clock source
    Oscillator,
    Regulator {
        kind: Option<RegulatorKind>,
    },
    /// A load switch or eFuse: a supply switched or limited by an IC
    LoadSwitch,
    /// A level translator or bus buffer between two voltage domains
    Translator {
        class: Option<TranslatorClass>,
    },
    /// A digital isolator (no galvanic path between its sides)
    Isolator,
    Mcu,
    /// A system on chip, application processor
    Soc,
    Fpga,
    Memory {
        kind: Option<MemoryKind>,
    },
    Transceiver {
        kind: Option<TransceiverKind>,
    },
    Opamp,
    Comparator,
    /// A logic gate, flip-flop, buffer or mux of a logic family
    Logic {
        family: Option<LogicFamily>,
    },
    Fuse {
        kind: Option<FuseKind>,
    },
    Battery,
    Transformer,
    /// A net tie: two nets joined on purpose (a copper short in the footprint)
    NetTie,
    /// A solder jumper or a jumper header
    Jumper,
    Fiducial,
    MountingHole,
    /// An integrated circuit with no more specific class
    Ic,
    Unknown,
}

/// What kind of diode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiodeKind {
    Rectifier,
    Schottky,
    Zener,
    /// A TVS; `bidirectional` when the source says which (`None`: not said)
    Tvs {
        bidirectional: Option<bool>,
    },
    Led,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BjtPolarity {
    Npn,
    Pnp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Channel {
    N,
    P,
}

/// What kind of connector, where the symbol tells
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConnectorKind {
    UsbA,
    UsbB,
    UsbMicro,
    UsbC,
    Hdmi,
    MicroHdmi,
    Rj45,
    Barrel,
    TerminalBlock,
    /// SD or microSD card socket
    Card,
    /// FPC or FFC
    Fpc,
    /// Coaxial (SMA, U.FL)
    Coax,
    /// Pin header or socket
    Header,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SwitchKind {
    Momentary,
    Toggle,
    Slide,
    Dip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RegulatorKind {
    Ldo,
    Buck,
    Boost,
    BuckBoost,
    /// A battery charger
    Charger,
}

/// How a translator passes a level (signal following, NET-10, models each)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TranslatorClass {
    /// Open-drain pass FET, pull-ups on both sides (PCA9306, TXS0102 class)
    PassFet,
    /// Auto-direction push-pull (TXB0104 class)
    AutoDirection,
    /// A direction pin sets the way (74LVC245, SN74AVC class)
    DirectionPin,
    /// A bus buffer or repeater that splits a bus into segments (PCA9515, TCA9517)
    Buffer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryKind {
    Flash,
    Eeprom,
    Sram,
    Dram,
    Fram,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransceiverKind {
    Can,
    Lin,
    Rs485,
    Rs232,
    Usb,
    EthPhy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogicFamily {
    Hc,
    Hct,
    Ahc,
    Ahct,
    Lvc,
    Lv,
    Avc,
    Auc,
    Ac,
    Act,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FuseKind {
    /// A one-time fuse
    Fuse,
    /// A resettable PTC fuse (polyfuse)
    Resettable,
}

impl PartClass {
    /// The class's name without its kind (`MOSFET`, `capacitor`)
    pub fn name(self) -> &'static str {
        match self {
            PartClass::Resistor => "resistor",
            PartClass::Capacitor { .. } => "capacitor",
            PartClass::Inductor => "inductor",
            PartClass::FerriteBead => "ferrite bead",
            PartClass::Diode {
                kind: Some(DiodeKind::Led),
            } => "LED",
            PartClass::Diode { .. } => "diode",
            PartClass::Bjt { .. } => "BJT",
            PartClass::Mosfet { .. } => "MOSFET",
            PartClass::Transistor => "transistor",
            PartClass::Connector { .. } => "connector",
            PartClass::TestPoint => "test point",
            PartClass::Switch { .. } => "switch",
            PartClass::Relay => "relay",
            PartClass::Crystal => "crystal",
            PartClass::Oscillator => "oscillator",
            PartClass::Regulator { .. } => "regulator",
            PartClass::LoadSwitch => "load switch",
            PartClass::Translator { .. } => "translator",
            PartClass::Isolator => "isolator",
            PartClass::Mcu => "MCU",
            PartClass::Soc => "SoC",
            PartClass::Fpga => "FPGA",
            PartClass::Memory { .. } => "memory",
            PartClass::Transceiver { .. } => "transceiver",
            PartClass::Opamp => "op-amp",
            PartClass::Comparator => "comparator",
            PartClass::Logic { .. } => "logic",
            PartClass::Fuse { .. } => "fuse",
            PartClass::Battery => "battery",
            PartClass::Transformer => "transformer",
            PartClass::NetTie => "net tie",
            PartClass::Jumper => "jumper",
            PartClass::Fiducial => "fiducial",
            PartClass::MountingHole => "mounting hole",
            PartClass::Ic => "IC",
            PartClass::Unknown => "unknown",
        }
    }

    /// The class with its kind, for messages and the dump (`P-channel MOSFET`, `polarized capacitor`, `buck
    /// regulator`)
    pub fn describe(self) -> String {
        let kind = match self {
            PartClass::Capacitor { polarized: true } => Some("polarized"),
            PartClass::Diode { kind: Some(k) } => match k {
                DiodeKind::Rectifier => Some("rectifier"),
                DiodeKind::Schottky => Some("Schottky"),
                DiodeKind::Zener => Some("Zener"),
                DiodeKind::Tvs {
                    bidirectional: Some(true),
                } => Some("bidirectional TVS"),
                DiodeKind::Tvs {
                    bidirectional: Some(false),
                } => Some("unidirectional TVS"),
                DiodeKind::Tvs {
                    bidirectional: None,
                } => Some("TVS"),
                DiodeKind::Led => None,
            },
            PartClass::Bjt { polarity: Some(p) } => Some(match p {
                BjtPolarity::Npn => "NPN",
                BjtPolarity::Pnp => "PNP",
            }),
            PartClass::Mosfet { channel: Some(c) } => Some(match c {
                Channel::N => "N-channel",
                Channel::P => "P-channel",
            }),
            PartClass::Connector { kind: Some(k) } => Some(match k {
                ConnectorKind::UsbA => "USB-A",
                ConnectorKind::UsbB => "USB-B",
                ConnectorKind::UsbMicro => "micro-USB",
                ConnectorKind::UsbC => "USB-C",
                ConnectorKind::Hdmi => "HDMI",
                ConnectorKind::MicroHdmi => "micro-HDMI",
                ConnectorKind::Rj45 => "RJ45",
                ConnectorKind::Barrel => "barrel",
                ConnectorKind::TerminalBlock => "terminal block",
                ConnectorKind::Card => "card",
                ConnectorKind::Fpc => "FPC",
                ConnectorKind::Coax => "coaxial",
                ConnectorKind::Header => "header",
            }),
            PartClass::Switch { kind: Some(k) } => Some(match k {
                SwitchKind::Momentary => "momentary",
                SwitchKind::Toggle => "toggle",
                SwitchKind::Slide => "slide",
                SwitchKind::Dip => "DIP",
            }),
            PartClass::Regulator { kind: Some(k) } => Some(match k {
                RegulatorKind::Ldo => "LDO",
                RegulatorKind::Buck => "buck",
                RegulatorKind::Boost => "boost",
                RegulatorKind::BuckBoost => "buck-boost",
                RegulatorKind::Charger => "charger",
            }),
            PartClass::Translator { class: Some(c) } => Some(match c {
                TranslatorClass::PassFet => "pass-FET",
                TranslatorClass::AutoDirection => "auto-direction",
                TranslatorClass::DirectionPin => "direction-pin",
                TranslatorClass::Buffer => "buffer",
            }),
            PartClass::Memory { kind: Some(k) } => Some(match k {
                MemoryKind::Flash => "flash",
                MemoryKind::Eeprom => "EEPROM",
                MemoryKind::Sram => "SRAM",
                MemoryKind::Dram => "DRAM",
                MemoryKind::Fram => "FRAM",
            }),
            PartClass::Transceiver { kind: Some(k) } => Some(match k {
                TransceiverKind::Can => "CAN",
                TransceiverKind::Lin => "LIN",
                TransceiverKind::Rs485 => "RS-485",
                TransceiverKind::Rs232 => "RS-232",
                TransceiverKind::Usb => "USB",
                TransceiverKind::EthPhy => "Ethernet PHY",
            }),
            PartClass::Logic { family: Some(f) } => Some(match f {
                LogicFamily::Hc => "74HC",
                LogicFamily::Hct => "74HCT",
                LogicFamily::Ahc => "74AHC",
                LogicFamily::Ahct => "74AHCT",
                LogicFamily::Lvc => "74LVC",
                LogicFamily::Lv => "74LV",
                LogicFamily::Avc => "74AVC",
                LogicFamily::Auc => "74AUC",
                LogicFamily::Ac => "74AC",
                LogicFamily::Act => "74ACT",
            }),
            PartClass::Fuse { kind: Some(k) } => Some(match k {
                FuseKind::Fuse => "one-time",
                FuseKind::Resettable => "resettable",
            }),
            _ => None,
        };
        match kind {
            Some(k) => format!("{k} {}", self.name()),
            None => self.name().to_string(),
        }
    }

    /// The unit a bare number in the Value field is read in (`10k` on a resistor is 10 kΩ, `100n` on a
    /// capacitor 100 nF, `16M` on a crystal 16 MHz); `None` when the Value field names a part, not a quantity
    pub fn value_unit(self) -> Option<Unit> {
        Some(match self {
            PartClass::Resistor => Unit::Ohm,
            PartClass::Capacitor { .. } => Unit::Farad,
            PartClass::Inductor => Unit::Henry,
            PartClass::Crystal | PartClass::Oscillator => Unit::Hertz,
            // A ferrite is specified by its impedance at 100 MHz (`600R@100MHz`)
            PartClass::FerriteBead => Unit::Ohm,
            PartClass::Fuse { .. } => Unit::Ampere,
            PartClass::Diode {
                kind: Some(DiodeKind::Zener | DiodeKind::Tvs { .. }),
            } => Unit::Volt,
            _ => return None,
        })
    }

    pub fn is_resistor(self) -> bool {
        self == PartClass::Resistor
    }

    pub fn is_capacitor(self) -> bool {
        matches!(self, PartClass::Capacitor { .. })
    }

    /// Any diode, LEDs included
    pub fn is_diode(self) -> bool {
        matches!(self, PartClass::Diode { .. })
    }

    pub fn is_led(self) -> bool {
        matches!(
            self,
            PartClass::Diode {
                kind: Some(DiodeKind::Led)
            }
        )
    }

    pub fn is_mosfet(self) -> bool {
        matches!(self, PartClass::Mosfet { .. })
    }

    pub fn is_bjt(self) -> bool {
        matches!(self, PartClass::Bjt { .. })
    }

    /// Any transistor (MOSFET, BJT, or a `Q` of unknown kind)
    pub fn is_transistor(self) -> bool {
        matches!(
            self,
            PartClass::Mosfet { .. } | PartClass::Bjt { .. } | PartClass::Transistor
        )
    }

    pub fn is_connector(self) -> bool {
        matches!(self, PartClass::Connector { .. })
    }

    pub fn is_translator(self) -> bool {
        matches!(self, PartClass::Translator { .. })
    }

    pub fn is_regulator(self) -> bool {
        matches!(self, PartClass::Regulator { .. })
    }

    pub fn is_fuse(self) -> bool {
        matches!(self, PartClass::Fuse { .. })
    }

    pub fn is_switch(self) -> bool {
        matches!(self, PartClass::Switch { .. })
    }

    /// A two-terminal part that joins its nets at DC: a ferrite, a net tie, a jumper (a 0 Ω resistor is a
    /// resistor; its value decides)
    pub fn is_dc_tie(self) -> bool {
        matches!(
            self,
            PartClass::FerriteBead | PartClass::NetTie | PartClass::Jumper
        )
    }

    /// An integrated circuit of any class (an MCU, a regulator, a generic IC)
    pub fn is_ic(self) -> bool {
        matches!(
            self,
            PartClass::Regulator { .. }
                | PartClass::LoadSwitch
                | PartClass::Translator { .. }
                | PartClass::Isolator
                | PartClass::Mcu
                | PartClass::Soc
                | PartClass::Fpga
                | PartClass::Memory { .. }
                | PartClass::Transceiver { .. }
                | PartClass::Opamp
                | PartClass::Comparator
                | PartClass::Logic { .. }
                | PartClass::Oscillator
                | PartClass::Ic
        )
    }

    /// A part that is not in the electrical circuit (fiducials, mounting holes)
    pub fn is_mechanical(self) -> bool {
        matches!(self, PartClass::Fiducial | PartClass::MountingHole)
    }
}

impl std::fmt::Display for PartClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.describe())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_units() {
        let p = PartClass::Mosfet {
            channel: Some(Channel::P),
        };
        assert_eq!(p.describe(), "P-channel MOSFET");
        assert_eq!(p.name(), "MOSFET");
        assert!(p.is_transistor() && p.is_mosfet());
        let led = PartClass::Diode {
            kind: Some(DiodeKind::Led),
        };
        assert_eq!(led.describe(), "LED");
        assert!(led.is_led() && led.is_diode());
        assert_eq!(
            PartClass::Capacitor { polarized: true }.value_unit(),
            Some(Unit::Farad)
        );
        assert_eq!(PartClass::Crystal.value_unit(), Some(Unit::Hertz));
        assert_eq!(PartClass::Mcu.value_unit(), None);
        assert_eq!(
            PartClass::Diode {
                kind: Some(DiodeKind::Zener)
            }
            .value_unit(),
            Some(Unit::Volt)
        );
        assert!(PartClass::Regulator { kind: None }.is_ic());
    }
}
