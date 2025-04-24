mod altium_netlist;
mod csv_util;
mod edif_netlist;
pub mod kicad_netlist;
pub mod netlist;
pub mod orcad_netlist;
pub mod passive_value;
pub mod pnp;
mod text_util;
mod wirelist;

pub use altium_netlist::load_altium_netlist;
pub use kicad_netlist::load_kicad_netlist;
pub use orcad_netlist::load_orcad_netlist;
pub use pnp::load_component_positions;
use std::fmt::{Debug, Display, Formatter};

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct Designator(pub String);

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct NetName(pub String);

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PinName(pub String);

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PinId(pub String);

impl Display for Designator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Designator({})", self.0)
    }
}
impl Debug for Designator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for NetName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NetName({})", self.0)
    }
}
impl Debug for NetName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for PinName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PinName({})", self.0)
    }
}
impl Debug for PinName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for PinId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PinId({})", self.0)
    }
}
impl Debug for PinId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Designator {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl NetName {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl PinName {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl PinId {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Designator {
    pub fn is_resistor(&self) -> bool {
        self.0.starts_with('R')
    }

    pub fn is_capacitor(&self) -> bool {
        self.0.starts_with('C')
    }

    pub fn is_inductor(&self) -> bool {
        if !self.0.starts_with('L') {
            return false;
        }
        if let Some(c) = self.0.chars().skip(1).next() {
            // ignore LEDx, LDx, etc
            c.is_numeric()
        } else {
            true
        }
    }

    pub fn is_transistor(&self) -> bool {
        self.0.starts_with('Q')
    }

    pub fn is_ic(&self) -> bool {
        self.0.starts_with('U')
    }
}
