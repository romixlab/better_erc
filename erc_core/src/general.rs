use ecad_file_format::netlist::{Netlist, Node, PinType};

pub fn input_without_driving_source(netlist: &Netlist) {
    let mut nets_with_inputs = vec![];
    for (k, lib_part) in &netlist.lib_parts {
        let components = netlist
            .components
            .iter()
            .filter(|(_d, c)| &c.lib_source == k)
            .collect::<Vec<_>>();
        for (pin_id, pin) in &lib_part.pins {
            if pin.default_mode.ty != PinType::DigitalInput {
                continue;
            }
            for (net_name, net) in &netlist.nets {
                for (designator, _component) in &components {
                    if net.nodes.contains(&Node {
                        designator: (*designator).clone(),
                        pin_id: pin_id.clone(),
                    }) {
                        nets_with_inputs.push((net_name.clone(), *designator, pin_id));
                    }
                }
            }
        }
    }
    println!("{:#?}", nets_with_inputs);
}

#[cfg(test)]
mod tests {
    use crate::Pcba;
    use crate::general::input_without_driving_source;
    use crate::power::derive_power_structure;
    use ecad_file_format::Designator;
    use std::path::Path;

    #[test]
    fn able_to_find_unconnected_inputs() {
        // let path = Path::new("/Users/roman/Downloads/test_projects/vb135a_fdcan_iso_usb_hw.net");
        // let netlist = ecad_file_format::load_kicad_netlist(&path).unwrap();
        // let netlist = ecad_file_format::load_altium_netlist(Path::new("/Users/roman/Downloads/test_projects/typec_sbu_serial_revb/typec_sbu_serial.NET.EDF"), Path::new("/Users/roman/Downloads/test_projects/typec_sbu_serial_revb/typec_sbu_serial.NET")).unwrap();
        let path = Path::new("/Users/roman/Downloads/test_projects/c_a6/pstxnet.dat");
        let netlist = ecad_file_format::load_orcad_netlist(&path).unwrap();
        let pcba = Pcba::new(netlist);
        let k = &pcba
            .netlist
            .components
            .get(&Designator("U12".into()))
            .unwrap()
            .lib_source;
        let lib_part = pcba.netlist.lib_parts.get(k).unwrap();
        println!("{:#?}", lib_part);

        // println!("{:#?}", pcba.switching_nodes);
        // println!("{:#?}", pcba.power);
        // input_without_driving_source(&netlist);
        // let power = find_power_structure(&netlist);
        // println!("{power:#?}");
    }
}
