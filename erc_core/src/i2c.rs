use crate::util::collapse_underscores;
use ecad_file_format::netlist::Netlist;
use ecad_file_format::{Designator, DesignatorStartsWith, NetName};

#[derive(Debug)]
pub struct I2cBus {
    pub derived_name: String,
    pub scl_net: NetName,
    pub sda_net: NetName,
    pub pull_up: Option<I2cPullUp>,
    pub nodes: Vec<Designator>,
}

#[derive(Debug)]
pub struct I2cPullUp {
    pub scl: Designator,
    pub sda: Designator,
    pub net: NetName,
}

#[derive(Debug)]
pub enum I2cNode {
    Device(Designator),
    VoltageTranslator(Designator),
    VoltageTranslatorDiscrete {
        scl_fet: Designator,
        sda_fet: Designator,
    },
    Connector(Designator),
    /// 0R, net tie, solder tie
    Tie {
        scl_tie: Designator,
        sda_tie: Designator,
    },
}

fn find_i2c_buses(netlist: &Netlist) -> Vec<I2cBus> {
    let mut buses = vec![];
    for potential_scl in netlist.nets.keys() {
        if let Some(scl_start) = potential_scl.0.find("SCL") {
            let prefix = &potential_scl.0[..scl_start];
            let suffix = if scl_start + 3 < potential_scl.len() {
                &potential_scl.0[(scl_start + 3)..]
            } else {
                ""
            };
            let sda_net = NetName(format!("{}SDA{}", prefix, suffix));
            if netlist.nets.keys().any(|k| k == &sda_net) {
                let derived_name =
                    collapse_underscores(format!("{}I2C{}", prefix, suffix).as_str());
                let pull_up_chains = netlist.find_chains(
                    potential_scl,
                    &[DesignatorStartsWith("R"), DesignatorStartsWith("R")],
                    &sda_net,
                );
                let pull_up = if let Some(chain) = pull_up_chains.first() {
                    Some(I2cPullUp {
                        scl: chain[0].1.clone(),
                        sda: chain[1].1.clone(),
                        net: netlist.connected_net(&chain[1].1, &chain[1].0).unwrap(),
                    })
                } else {
                    None
                };
                buses.push(I2cBus {
                    derived_name,
                    scl_net: potential_scl.clone(),
                    sda_net,
                    pull_up,
                    nodes: vec![],
                });
            }
        }
    }
    buses
}

#[cfg(test)]
mod tests {
    use crate::i2c::find_i2c_buses;
    use ecad_file_format::kicad_netlist::load_kicad_netlist;
    use generate_netlists::get_netlist_path;

    #[test]
    fn able_to_recognize_i2c_bus_segments() {
        let path = get_netlist_path("i2c_segments");
        let netlist = load_kicad_netlist(&path).unwrap();
        // println!("{:#?}", netlist);
        let buses = find_i2c_buses(&netlist);
        println!("buses: {buses:#?}");
        // let chains = netlist.find_chains(
        //     &NetName("/SCL1_3V3".into()),
        //     &[DesignatorStartsWith("R"), DesignatorStartsWith("R")],
        //     &NetName("/SDA1_3V3".into()),
        // );
        // println!("{:?}", chains);
        // let paths = netlist.find_connected_parts(
        //     &Designator("Q1".into()),
        //     &PinId("3".into()),
        //     DesignatorStartsWith("R"),
        // );
        // println!("{:?}", paths);
    }

    #[test]
    fn able_to_find_missing_i2c_pull_ups() {
        let path = get_netlist_path("i2c_no_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        println!("{:#?}", netlist);
    }
}
