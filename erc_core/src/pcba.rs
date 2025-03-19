use crate::diagnostics::Diagnostics;
use crate::i2c::{I2cBuses, find_i2c_buses};
use crate::power::{Power, derive_power_structure};
use crate::style::check_style;
use ecad_file_format::netlist::Netlist;
use ecad_file_format::{Designator, NetName};
use std::collections::HashSet;

pub struct Pcba {
    pub netlist: Netlist,
    pub power: Power,
    pub switching_nodes: HashSet<NetName>,
    pub i2c_buses: I2cBuses,
    pub diagnostics: Diagnostics,
}

impl Pcba {
    pub fn new(netlist: Netlist) -> Self {
        let mut diagnostics = Diagnostics::default();
        let power = derive_power_structure(&netlist, true); // TODO: move strict to config
        let i2c_buses = find_i2c_buses(&netlist, &mut diagnostics.i2c);
        check_style(&netlist, &mut diagnostics.style);

        let mut pcba = Self {
            netlist,
            power,
            switching_nodes: HashSet::new(),
            i2c_buses,
            diagnostics,
        };

        let switching_nodes = crate::power::find_switching_nodes(&pcba);
        // remove switching nodes from power rails
        for net_name in &switching_nodes {
            pcba.power.power_rails.remove(net_name);
        }
        pcba.switching_nodes = switching_nodes;

        // remove I2C buses containing voltage from power nets, as it is more likely
        // that e.g. I2C1_SCL_3V3 and I2C1_SDA_3V3 are signals and not power
        for bus in pcba.i2c_buses.by_name.values() {
            pcba.power.power_rails.remove(&bus.scl_net);
            pcba.power.power_rails.remove(&bus.sda_net);
        }

        pcba
    }

    /// Returns a set of parts with a particular designator that are connected to 'from' part.
    /// Optionally ignoring parts connected through power nets, which is probably what is needed most of the time.
    pub fn find_connected_parts<F: Fn(&Designator) -> bool>(
        &self,
        from: &Designator,
        to_filter: F,
        ignore_power_nets: bool,
    ) -> HashSet<Designator> {
        let mut parts = HashSet::new();
        for designator in self.netlist.components.keys() {
            if to_filter(designator) {
                let mut common_nets = self.netlist.parts_common_nets(from, designator);
                if ignore_power_nets {
                    for ground_net in &self.power.ground_nets {
                        common_nets.remove(ground_net);
                    }
                    for power_net in self.power.power_rails.keys() {
                        common_nets.remove(power_net);
                    }
                }
                if !common_nets.is_empty() {
                    parts.insert(designator.clone());
                }
            }
        }
        parts
    }

    /// Find connected parts with particular designators.
    /// Optionally ignoring parts connected through power nets, which is probably what is needed most of the time.
    ///
    /// Example finding potential DC-DC converters switching nodes
    /// ```
    /// # use ecad_file_format::netlist::Netlist;
    /// # use ecad_file_format::{Designator};
    /// # use erc_core::Pcba;
    /// # let netlist = Netlist::default();
    /// # let pcba = Pcba::new(netlist);
    /// pcba.find_part_chains(&[Designator::is_ic, Designator::is_inductor][..], true);
    /// // if there is a sense resistor before inductor
    /// pcba.find_part_chains(&[Designator::is_ic, Designator::is_resistor, Designator::is_inductor][..], true);
    /// ```
    pub fn find_part_chains<F: Fn(&Designator) -> bool>(
        &self,
        goes_through: &[F],
        ignore_power_nets: bool,
    ) -> Vec<Vec<Designator>> {
        if goes_through.len() < 2 {
            return Vec::new();
        }
        let mut chains = Vec::new();
        let is_start = &goes_through[0];
        for designator in self.netlist.components.keys() {
            if is_start(designator) {
                chains.push(vec![designator.clone()]);
            }
        }
        for next_link in &goes_through[1..] {
            let mut next_chains = Vec::new();
            for chain in &chains {
                let last_part = chain.last().unwrap();
                let parts = self.find_connected_parts(last_part, next_link, ignore_power_nets);
                for part in parts {
                    let mut chain = chain.clone();
                    chain.push(part);
                    next_chains.push(chain);
                }
            }
            chains = next_chains;
        }
        chains
    }
}
