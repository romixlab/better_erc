use crate::{Designator, DesignatorStartsWith, NetName, PinId, PinName};
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Display, Formatter};

#[derive(Debug, Default)]
pub struct Netlist {
    pub parts: HashMap<Designator, Part>,
    pub nets: HashMap<NetName, Net>,
}

#[derive(Debug)]
pub struct Part {
    pub name: String,
    pub description: String,
    pub footprints: Vec<String>,
    pub fields: HashMap<String, String>,
    pub pins: HashMap<PinName, Pin>,
    pub banks: HashMap<String, Bank>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Net {
    pub nodes: HashSet<Node>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Node {
    pub designator: Designator,
    pub pin_id: PinId,
}

#[derive(Debug)]
pub struct Pin {
    pub default_mode: PinMode,
    pub alternate_modes: HashMap<String, PinMode>,
    pub bank_name: Option<String>,
    // voltage thresholds vs bank voltage table
    // max sink/source current
    // max frequency
    // min/max voltage or from bank?
}

#[derive(Debug)]
pub struct PinMode {
    pub ty: PinType,
    pub pull_up: Option<Pull>,
    pub pull_down: Option<Pull>,
    pub io_standard: Option<IOStandard>, // pub quiescent current vs bank voltage table
}

#[derive(Debug)]
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
    /// # use ecad_file_format::{DesignatorStartsWith, NetName};
    /// let netlist = Netlist::default();
    /// netlist.find_chains(&NetName("SCL".into()), &[DesignatorStartsWith("R"), DesignatorStartsWith("R")], &NetName("SDA".into()))
    /// ```
    pub fn find_chains(
        &self,
        start: &NetName,
        goes_through: &[DesignatorStartsWith],
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
            if node.designator.0.starts_with(goes_through_first.0) {
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
                next_layer.extend(self.find_connected_parts(
                    last_start,
                    except_pin,
                    *goes_through_next,
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
    pub fn find_connected_parts(
        &self,
        start: &Designator,
        except_pin: &PinId,
        end: DesignatorStartsWith,
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
                if start != &node.designator && node.designator.0.starts_with(end.0) {
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
}

impl Display for Netlist {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parts: {:?}", self.parts)?;
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
