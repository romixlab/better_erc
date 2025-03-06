mod csv_util;
pub mod kicad_netlist;
pub mod netlist;
pub mod orcad_netlist;
pub mod passive_value;
pub mod pnp;
mod wirelist;

pub use kicad_netlist::load_kicad_netlist;
pub use orcad_netlist::load_orcad_netlist;
pub use pnp::load_component_positions;
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Designator(pub String);

#[derive(Copy, Clone)]
pub struct DesignatorStartsWith<'a>(pub &'a str);

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct NetName(pub String);

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct PinName(pub String);

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct PinId(pub String);

impl Display for Designator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Designator({})", self.0)
    }
}

impl Display for NetName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "NetName({})", self.0)
    }
}

impl Display for PinName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PinName({})", self.0)
    }
}

impl Display for PinId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PinId({})", self.0)
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
