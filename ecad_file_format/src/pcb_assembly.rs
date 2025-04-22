use crate::Designator;
use crate::netlist::Netlist;
use crate::pnp::ComponentPosition;
use std::collections::HashMap;
use std::sync::Arc;

pub struct PcbAssembly {
    pub name: Arc<String>,
    pub netlist: Netlist,
    pub pnp: HashMap<Designator, ComponentPosition>,
    pub bom: (),
}
