use crate::util::collapse_underscores;
use ecad_file_format::netlist::Netlist;
use ecad_file_format::{Designator, NetName};

#[derive(Debug)]
struct I2cBus {
    derived_name: String,
    scl_net: NetName,
    scl_pull_up: Option<Designator>,
    sda_net: NetName,
    sda_pull_up: Option<Designator>,
    nodes: Vec<Designator>,
}

#[derive(Debug)]
struct I2cNode {
    designator: Designator,
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
                buses.push(I2cBus {
                    derived_name,
                    scl_net: potential_scl.clone(),
                    scl_pull_up: None,
                    sda_net,
                    sda_pull_up: None,
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
    }

    #[test]
    fn able_to_find_missing_i2c_pull_ups() {
        let path = get_netlist_path("i2c_no_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        println!("{:#?}", netlist);
    }
}
