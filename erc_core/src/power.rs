use crate::Pcba;
use ecad_file_format::netlist::{Netlist, PinType};
use ecad_file_format::{Designator, NetName};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub struct Power {
    pub power_rails: HashMap<NetName, PowerRail>,
    pub ground_nets: HashSet<NetName>,
}

pub struct PowerRail {
    pub voltage: Option<Volt>,
}

#[derive(Debug, Copy, Clone)]
pub struct Volt(pub f32);

/// If strict is true, then only power nets containing +xVy will be picked up
pub fn derive_power_structure(netlist: &Netlist, strict: bool) -> Power {
    // 0 tie, current sense tie, pwr switch IC, pwr FET to other power nets
    // sources: LDOs, DC-DCs, ICs, connectors

    let mut power_rails = HashMap::new();

    let re_mvn = if strict {
        Regex::new(".*\\+(\\d+)V(\\d+).*")
    } else {
        Regex::new(".*(\\d+)V(\\d+).*")
    }
    .unwrap();
    for name in netlist.nets.keys() {
        if let Some(c) = re_mvn.captures(name.0.as_str()) {
            let integer: u32 = c.get(1).unwrap().as_str().parse().unwrap();
            let fractional_str = c.get(2).unwrap().as_str();
            let fractional: u32 = fractional_str.parse().unwrap();
            let voltage =
                integer as f32 + (fractional as f32 / (fractional_str.len() as f32 * 10.0));
            power_rails.insert(
                name.clone(),
                PowerRail {
                    voltage: Some(Volt(voltage)),
                },
            );
            continue;
        }

        if name.0.contains("+V") {
            if !power_rails.contains_key(name) {
                power_rails.insert(name.clone(), PowerRail { voltage: None });
            }
            continue;
        }

        if name.0.contains("VDD") || name.0.contains("VCC") {
            if !power_rails.contains_key(name) {
                power_rails.insert(name.clone(), PowerRail { voltage: None });
            }
        }
    }

    let mut ground_nets = HashSet::new();
    for name in netlist.nets.keys() {
        let n = name.0.as_str();
        if n.contains("GND") || n.contains("VSS") || n.contains("VEE") || n.starts_with("ISO") {
            ground_nets.insert(name.clone());
        }
    }

    let nets_with_power_pins = netlist.find_nets_with_pin_types(&[
        PinType::PowerIn,
        PinType::PowerOut,
        PinType::PowerUnspecified,
        PinType::PowerIO,
    ]);
    for net in nets_with_power_pins {
        // do not replace nets with voltage in their name, also ignore ground nets
        if !power_rails.contains_key(&net) && !ground_nets.contains(&net) {
            power_rails.insert(net, PowerRail { voltage: None });
        }
    }

    Power {
        power_rails,
        ground_nets,
    }
}

pub fn find_switching_nodes(pcba: &Pcba) -> HashSet<NetName> {
    let mut switching_nodes = HashSet::new();
    let chains = pcba.find_part_chains(&[Designator::is_ic, Designator::is_inductor][..], false);
    for chain in &chains {
        // nets will contain power and ground rails if inductor is connected to them and to an IC, but power rails
        // will also contain LX/SW nets that were picked up through pin type == power if it was set.
        let mut nets = pcba.netlist.parts_common_nets(&chain[0], &chain[1]);
        // remove ground nets
        for ground_net in &pcba.power.ground_nets {
            nets.remove(ground_net);
        }
        // remove power nets, but only if they do not contain LX and SW
        for power_net in pcba.power.power_rails.keys() {
            if power_net.0.contains("LX") || power_net.0.contains("SW") {
                continue;
            }
            nets.remove(power_net);
        }
        // TODO: issue warning if zero or more than one net remained
        let Some(switching_net) = nets.into_iter().next() else {
            continue;
        };
        let mut inductor_nets = pcba.netlist.part_nets(&chain[1]);
        inductor_nets.remove(&switching_net);
        let Some(other_side_net) = inductor_nets.iter().next() else {
            continue;
        };
        // check that other side is a power net - this fails if other side is not a power net (pins not marked power or net names not implying power)
        // if pcba.power.is_power_net(other_side_net) {
        //     switching_nodes.insert(switching_net);
        // }
        // at least remove RF ICs connected to inductors
        if !switching_net.0.contains("RF") && !other_side_net.0.contains("RF") {
            switching_nodes.insert(switching_net);
        }
    }
    switching_nodes
}

impl Power {
    /// Returns true if net is a power or ground net
    pub fn is_power_net(&self, net_name: &NetName) -> bool {
        self.power_rails.contains_key(net_name) || self.ground_nets.contains(net_name)
    }
}

impl Debug for PowerRail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(voltage) = self.voltage {
            write!(f, "PowerRail({} V)", voltage.0)
        } else {
            write!(f, "PowerRail(? V)")
        }
    }
}
