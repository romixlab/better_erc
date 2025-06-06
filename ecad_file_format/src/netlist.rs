use crate::passive_value::{Ohm, parse_resistance_value};
use crate::{Designator, NetName, PinId, PinName};
use anyhow::{Error, Result};
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Default)]
pub struct Netlist {
    pub lib_parts: HashMap<(LibName, LibPartName), LibPart>,
    pub nets: HashMap<NetName, Net>,
    pub components: HashMap<Designator, Component>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LibName(pub String);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LibPartName(pub String);

#[derive(Debug, Default)]
pub struct LibPart {
    pub description: String,
    pub footprints: Vec<String>,
    pub fields: HashMap<String, String>,
    pub pins: HashMap<PinId, Pin>,
    pub banks: HashMap<String, Bank>,
}

#[derive(Debug)]
pub struct Component {
    pub value: String,
    pub description: String,
    pub lib_source: (LibName, LibPartName),
    pub fields: HashMap<String, String>,
    pub sections: Vec<ComponentSection>,
}

pub struct ComponentSection {
    pub name: String,
    pub page_number: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Net {
    pub nodes: HashSet<Node>,
    pub properties: HashMap<String, String>,
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Node {
    pub designator: Designator,
    pub pin_id: PinId,
}

#[derive(Debug)]
pub struct Pin {
    pub name: PinName,
    pub default_mode: PinMode,
    pub alternate_modes: HashMap<String, PinMode>,
    pub bank_name: Option<String>,
    pub section_name: Option<String>,
    // voltage thresholds vs bank voltage table
    // max sink/source current
    // max frequency
    // min/max voltage or from bank?
}

pub struct PinMode {
    pub ty: PinType,
    pub pull_up: Option<Pull>,
    pub pull_down: Option<Pull>,
    pub io_standard: Option<IOStandard>, // pub quiescent current vs bank voltage table
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PinType {
    DigitalInput,
    DigitalOutput,
    DigitalIO,
    AnalogInput,
    AnalogOutput,
    AnalogIO,
    PowerIn,
    PowerOut,
    PowerIO,
    PowerUnspecified,
    OpenCollector,
    OpenEmitter,
    /// High, Low or High-Z
    TriState,
    /// Physically left unconnected and can be used for routing of other signals for example
    Unconnected,
    /// Unknown
    Unspecified,
    /// Unpowered (resistor, capacitor, ...)
    Passive,
}

#[derive(Debug)]
pub enum IOStandard {
    LVTTL,
    LVCMOS33,
    LVCMOS18,
    LVCMOS15,
    LVCMOS12,
}

#[derive(Debug)]
pub enum Pull {
    Unknown,
    Resistor { resistance: f32 },
    Current { current: f32 },
}

#[derive(Debug)]
pub struct Bank {
    pub total_source_current: f32,
    pub total_sink_current: f32,
    // min max voltage
}

impl Netlist {
    /// Finds all the chains of parts connected in between two nets, each going through particular designators
    ///
    /// Example finding I2C pull up resistors:
    /// ```
    /// # use ecad_file_format::netlist::Netlist;
    /// # use ecad_file_format::{Designator, NetName};
    /// # let netlist = Netlist::default();
    /// netlist.find_net_chains(&NetName("SCL".into()), &[Designator::is_resistor, Designator::is_resistor], &NetName("SDA".into()))
    /// ```
    pub fn find_net_chains<F: Fn(&Designator) -> bool>(
        &self,
        start: &NetName,
        goes_through: &[F],
        end: &NetName,
    ) -> Vec<Vec<(PinId, Designator)>> {
        let mut goes_through = goes_through.iter().collect::<VecDeque<_>>();
        let Some(goes_through_first) = goes_through.pop_front() else {
            return Vec::new();
        };
        let Some(net) = self.nets.get(start) else {
            return Vec::new();
        };
        // find to which part.pin_id's 'start' net is connected
        let mut first_layer = HashSet::new();
        for node in &net.nodes {
            if goes_through_first(&node.designator) {
                first_layer.insert((node.pin_id.clone(), node.designator.clone()));
            }
        }
        // println!("first layer {:?}", first_layer);
        // for each go-through part, find all part.pin_id's for all of its pins except entry one
        let mut links = vec![first_layer];
        while let Some(goes_through_next) = goes_through.pop_front() {
            let last_starts = links.last().unwrap();
            let mut next_layer = HashSet::new();
            for (except_pin, last_start) in last_starts {
                next_layer.extend(self.find_reachable_pins(
                    last_start,
                    except_pin,
                    goes_through_next,
                ));
            }
            // println!("next_layer {:?}", next_layer);
            links.push(next_layer);
        }
        // for the last layer, leave only the parts that are connected to 'end' net
        links
            .last_mut()
            .unwrap()
            .retain(|(_, designator)| self.is_connected(designator, end));
        // println!("final {links:?}");
        // all possible chains
        let mut chains: Vec<_> = links.iter().multi_cartesian_product().collect();
        // println!("chains {:?}", chains);
        // remove broken chains
        chains.retain(|c| {
            c.windows(2)
                .all(|w| self.are_parts_connected(&w[0].1, &w[1].1))
        });
        chains
            .into_iter()
            .map(|c| c.into_iter().cloned().collect())
            .collect()
    }

    /// Returns a set of pins that are reachable from any pins of 'start' part, except via its 'except_pin'
    pub fn find_reachable_pins<F: Fn(&Designator) -> bool>(
        &self,
        start: &Designator,
        except_pin: &PinId,
        end_filter: F,
    ) -> HashSet<(PinId, Designator)> {
        let mut found = HashSet::new();
        'outer: for (_net_name, net) in &self.nets {
            let mut potential = vec![];
            let mut start_found = false;
            for node in &net.nodes {
                if &node.designator == start && &node.pin_id == except_pin {
                    continue 'outer;
                }
                if &node.designator == start {
                    start_found = true;
                }
                if start != &node.designator && end_filter(&node.designator) {
                    potential.push((node.pin_id.clone(), node.designator.clone()));
                }
            }
            if start_found {
                found.extend(potential);
            }
        }
        found
    }

    /// Returns true if part is connected to net via any of its pins
    pub fn is_connected(&self, part: &Designator, target_net: &NetName) -> bool {
        for (net_name, net) in &self.nets {
            if net_name != target_net {
                continue;
            }
            for node in &net.nodes {
                if node.designator == *part {
                    return true;
                }
            }
        }
        false
    }

    /// Returns true if two parts are connected via any of their pins
    pub fn are_parts_connected(&self, part_a: &Designator, part_b: &Designator) -> bool {
        for (_, net) in &self.nets {
            let contains_a = net.nodes.iter().any(|n| &n.designator == part_a);
            let contains_b = net.nodes.iter().any(|n| &n.designator == part_b);
            if contains_a && contains_b {
                return true;
            }
        }
        false
    }

    /// Returns net name for the part's pin
    pub fn pin_net(&self, part: &Designator, pin: &PinId) -> Option<NetName> {
        for (net_name, net) in &self.nets {
            for node in &net.nodes {
                if node.designator == *part && node.pin_id == *pin {
                    return Some(net_name.clone());
                }
            }
        }
        None
    }

    /// Returns list of parts that have connection to any of the specified nets
    pub fn any_net_parts(&self, nets: &[&NetName]) -> HashSet<Designator> {
        let mut parts = HashSet::new();
        for net_name in nets {
            if let Some(net) = self.nets.get(net_name) {
                for node in &net.nodes {
                    if !parts.contains(&node.designator) {
                        parts.insert(node.designator.clone());
                    }
                }
            }
        }
        parts
    }

    /// Returns list of nets part is connected to
    pub fn part_nets(&self, part: &Designator) -> HashSet<NetName> {
        let mut nets = HashSet::new();
        for (net_name, net) in &self.nets {
            for node in &net.nodes {
                if node.designator == *part {
                    nets.insert(net_name.clone());
                }
            }
        }
        nets
    }

    /// Returns list of nets between two parts
    pub fn parts_common_nets(&self, part_a: &Designator, part_b: &Designator) -> HashSet<NetName> {
        let part_a_nets = self.part_nets(part_a);
        let part_b_nets = self.part_nets(part_b);
        part_a_nets.intersection(&part_b_nets).cloned().collect()
    }

    /// Returns list of nets part is connected to, excluding part pin with provided names
    pub fn part_nets_exclude_pin_names(
        &self,
        part: &Designator,
        exclude: &[&PinName],
    ) -> Vec<NetName> {
        let mut nets = vec![];
        for (net_name, net) in &self.nets {
            for node in &net.nodes {
                if node.designator != *part {
                    continue;
                }

                let Some(component) = self.components.get(&node.designator) else {
                    continue;
                };
                let Some(lib_part) = self.lib_parts.get(&component.lib_source) else {
                    continue;
                };
                let Some(pin) = lib_part.pins.get(&node.pin_id) else {
                    continue;
                };

                if !exclude.contains(&&pin.name) {
                    nets.push(net_name.clone());
                }
            }
        }
        nets
    }

    pub fn resistance(&self, designator: &Designator) -> Result<Ohm> {
        if !designator.0.starts_with('R') {
            return Err(Error::msg("{designator} is not a resistor"));
        }
        if let Some(component) = &self.components.get(designator) {
            if component.value.is_empty() {
                Err(Error::msg("{designator} has no value"))
            } else {
                let val = parse_resistance_value(component.value.as_str())?;
                Ok(val.0)
            }
        } else {
            Err(Error::msg("{designator} not found"))
        }
    }

    /// Returns all the nets that have specified pin types in them.
    pub fn find_nets_with_pin_types(&self, pin_types: &[PinType]) -> HashSet<NetName> {
        let mut nets = HashSet::new();
        for (k, lib_part) in &self.lib_parts {
            let components = self
                .components
                .iter()
                .filter(|(_d, c)| &c.lib_source == k)
                .collect::<Vec<_>>();
            for (pin_id, pin) in &lib_part.pins {
                if !pin_types.contains(&pin.default_mode.ty) {
                    continue;
                }
                for (designator, _c) in &components {
                    if let Some(net) = self.pin_net(designator, pin_id) {
                        nets.insert(net);
                    }
                }
            }
        }
        nets
    }
}

impl Display for Netlist {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parts: {:?}", self.lib_parts)?;
        for (net_name, net) in self.nets.iter() {
            write!(f, "Net: \"{net_name}\": ")?;
            for (idx, node) in net.nodes.iter().enumerate() {
                write!(f, "{}.{}", node.designator, node.pin_id)?;
                if idx < net.nodes.len() - 1 {
                    write!(f, " + ")?;
                }
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}

impl Display for LibName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LibName({})", self.0)
    }
}
impl Debug for LibName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for LibPartName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "LibPartName({})", self.0)
    }
}
impl Debug for LibPartName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node({}.{})", self.designator, self.pin_id)
    }
}
impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for PinMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PinMode(ty: {:?}, pull_up: {:?}, pull_down: {:?}, io_standard: {:?})",
            self.ty, self.pull_up, self.pull_down, self.io_standard
        )
    }
}
impl Debug for PinMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Debug for ComponentSection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ComponentSection('{}', page: {:?})",
            self.name, self.page_number
        )
    }
}
