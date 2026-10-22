//! Part classes: what kind of part a designator is (resistor, MOSFET, connector), each a [`Fact`] with the rule
//! that decided it.
//!
//! The library symbol decides first (its name in the library, `Device:R`, `Transistor_FET:BSS138`, then its
//! description and keywords); the designator prefix (`R`, `Q`, `J`) is the last resort and is marked
//! [`Provenance::Assumed`], so a finding that relies on it is an estimate and says so. The part database
//! (CFG-5) will come before both.

use erc_model::{Board, Fact, PartId, Provenance};
use serde::{Deserialize, Serialize};

/// What kind of part
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PartClass {
    Resistor,
    Capacitor,
    Inductor,
    FerriteBead,
    Diode,
    Led,
    Mosfet,
    Bjt,
    /// A transistor whose kind is not known (a `Q` designator only)
    Transistor,
    Connector,
    TestPoint,
    /// A level translator or bus buffer between two voltage domains
    Translator,
    /// An integrated circuit with no more specific class
    Ic,
    NetTie,
    Crystal,
    Fuse,
    Switch,
    MountingHole,
    Fiducial,
    Unknown,
}

impl PartClass {
    pub fn name(self) -> &'static str {
        match self {
            PartClass::Resistor => "resistor",
            PartClass::Capacitor => "capacitor",
            PartClass::Inductor => "inductor",
            PartClass::FerriteBead => "ferrite bead",
            PartClass::Diode => "diode",
            PartClass::Led => "LED",
            PartClass::Mosfet => "MOSFET",
            PartClass::Bjt => "BJT",
            PartClass::Transistor => "transistor",
            PartClass::Connector => "connector",
            PartClass::TestPoint => "test point",
            PartClass::Translator => "translator",
            PartClass::Ic => "IC",
            PartClass::NetTie => "net tie",
            PartClass::Crystal => "crystal",
            PartClass::Fuse => "fuse",
            PartClass::Switch => "switch",
            PartClass::MountingHole => "mounting hole",
            PartClass::Fiducial => "fiducial",
            PartClass::Unknown => "unknown",
        }
    }

    /// Any transistor (MOSFET, BJT, or a `Q` of unknown kind)
    pub fn is_transistor(self) -> bool {
        matches!(
            self,
            PartClass::Mosfet | PartClass::Bjt | PartClass::Transistor
        )
    }
}

/// The class of every part of a board, by [`PartId`]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PartClasses(pub Vec<Fact<PartClass>>);

impl PartClasses {
    /// Classifies every part of `board`
    pub fn of(board: &Board) -> PartClasses {
        PartClasses(
            (0..board.parts.len())
                .map(|i| classify(board, PartId::from_index(i)))
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

    pub fn is(&self, part: PartId, class: PartClass) -> bool {
        self.get(part) == class
    }
}

const BY_LIBRARY: &str = "class from the library symbol";
const BY_DESCRIPTION: &str = "class from the symbol description";
const BY_PREFIX: &str = "class from the designator prefix";

/// The class of one part: library symbol, then description and keywords, then designator prefix (assumed)
pub fn classify(board: &Board, part: PartId) -> Fact<PartClass> {
    let p = board.part(part);
    let derived = |class, rule: &str| Fact::new(class, Provenance::Derived { rule: rule.into() });
    let (lib, name) = p.lib_id.split_once(':').unwrap_or(("", &p.lib_id));
    if let Some(class) = by_library(lib, name) {
        return derived(class, BY_LIBRARY);
    }
    let symbol = p.symbol.map(|s| &board.symbols[s.index()]);
    let text = format!(
        "{} {} {}",
        symbol.map_or("", |s| s.description.as_str()),
        symbol.map_or("", |s| s.keywords.as_str()),
        p.field("Description").unwrap_or("")
    )
    .to_lowercase();
    if let Some(class) = by_description(&text) {
        return derived(class, BY_DESCRIPTION);
    }
    Fact::new(
        by_prefix(&p.designator),
        Provenance::Assumed {
            rule: BY_PREFIX.into(),
        },
    )
}

/// From the library and symbol names (`Device:R_Small`, `Transistor_FET:BSS138`, `Connector:Conn_01x03_Pin`)
fn by_library(lib: &str, name: &str) -> Option<PartClass> {
    let base = name.split('_').next().unwrap_or(name);
    Some(match lib {
        "Transistor_FET" => PartClass::Mosfet,
        "Transistor_BJT" => PartClass::Bjt,
        "Connector" | "Connector_Generic" | "Connector_Audio" => {
            if name.starts_with("TestPoint") {
                PartClass::TestPoint
            } else {
                PartClass::Connector
            }
        }
        "LED" => PartClass::Led,
        "Diode" => PartClass::Diode,
        "Crystal" => PartClass::Crystal,
        "Fuse" => PartClass::Fuse,
        "Switch" => PartClass::Switch,
        "Mechanical" if name.starts_with("MountingHole") => PartClass::MountingHole,
        "Mechanical" if name.starts_with("Fiducial") => PartClass::Fiducial,
        _ => match base {
            "R" => PartClass::Resistor,
            "C" | "CP" => PartClass::Capacitor,
            "L" => PartClass::Inductor,
            "FerriteBead" => PartClass::FerriteBead,
            "LED" => PartClass::Led,
            "D" => PartClass::Diode,
            "NetTie" => PartClass::NetTie,
            "TestPoint" => PartClass::TestPoint,
            "Crystal" => PartClass::Crystal,
            "Fuse" | "Polyfuse" => PartClass::Fuse,
            "Q" if name.contains("MOS") => PartClass::Mosfet,
            "Q" if name.contains("NPN") || name.contains("PNP") => PartClass::Bjt,
            _ => return None,
        },
    })
}

/// From the description and keywords, lower case
fn by_description(text: &str) -> Option<PartClass> {
    let has = |w: &str| text.contains(w);
    Some(
        if has("mosfet")
            || has("n-channel")
            || has("p-channel")
            || has("n channel")
            || has("p channel")
        {
            PartClass::Mosfet
        } else if has("translator") || has("level shifter") || has("level-shifter") {
            PartClass::Translator
        } else if has("resistor") && !has("array") && !has("network") {
            PartClass::Resistor
        } else if has("capacitor") {
            PartClass::Capacitor
        } else if has("inductor") {
            PartClass::Inductor
        } else if has("ferrite") {
            PartClass::FerriteBead
        } else if has("connector") {
            PartClass::Connector
        } else if has("test point") {
            PartClass::TestPoint
        } else {
            return None;
        },
    )
}

/// The designator prefix (letters before the number): the last resort
fn by_prefix(designator: &str) -> PartClass {
    let prefix: String = designator
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_uppercase();
    match prefix.as_str() {
        "R" | "RN" => PartClass::Resistor,
        "C" => PartClass::Capacitor,
        "L" => PartClass::Inductor,
        "FB" => PartClass::FerriteBead,
        "D" => PartClass::Diode,
        "LED" => PartClass::Led,
        "Q" | "T" => PartClass::Transistor,
        "J" | "P" | "CN" | "X" => PartClass::Connector,
        "TP" => PartClass::TestPoint,
        "U" | "IC" => PartClass::Ic,
        "NT" => PartClass::NetTie,
        "Y" => PartClass::Crystal,
        "F" => PartClass::Fuse,
        "SW" | "S" => PartClass::Switch,
        "H" | "MH" => PartClass::MountingHole,
        "FID" => PartClass::Fiducial,
        _ => PartClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_then_description_then_prefix() {
        assert_eq!(by_library("Device", "R_Small"), Some(PartClass::Resistor));
        assert_eq!(by_library("Fixture", "R"), Some(PartClass::Resistor));
        assert_eq!(by_library("Device", "C"), Some(PartClass::Capacitor));
        assert_eq!(
            by_library("Transistor_FET", "BSS138"),
            Some(PartClass::Mosfet)
        );
        assert_eq!(by_library("Device", "Q_NMOS_GSD"), Some(PartClass::Mosfet));
        assert_eq!(by_library("Device", "Q_NPN_BEC"), Some(PartClass::Bjt));
        assert_eq!(
            by_library("Connector", "Conn_01x03_Pin"),
            Some(PartClass::Connector)
        );
        assert_eq!(by_library("Interface", "PCA9306"), None);
        assert_eq!(by_library("Fixture", "NMOS"), None);
        assert_eq!(by_description("n-channel mosfet"), Some(PartClass::Mosfet));
        assert_eq!(
            by_description("dual bidirectional i2c-bus and smbus voltage-level translator"),
            Some(PartClass::Translator)
        );
        assert_eq!(by_description("4-bit i2c-bus and smbus i/o port"), None);
        assert_eq!(by_prefix("U12"), PartClass::Ic);
        assert_eq!(by_prefix("TP3"), PartClass::TestPoint);
        assert_eq!(by_prefix("LED1"), PartClass::Led);
        assert_eq!(by_prefix("#PWR01"), PartClass::Unknown);
    }
}
