use ecad_file_format::netlist::{Netlist, Node, PinType};
use ecad_file_format::{Designator, NetName, PinId};

/// IO-1, a stub so far: prints every input pin with its net; telling driven inputs from floating ones is still to
/// come
pub fn input_without_driving_source(netlist: &Netlist) {
    println!("{:#?}", inputs(netlist));
}

/// Every digital input pin with the net it is on
fn inputs(netlist: &Netlist) -> Vec<(NetName, &Designator, &PinId)> {
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
    nets_with_inputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PcbAssembly;
    use ecad_file_format::kicad_netlist::load_kicad_netlist;
    use ecad_file_format::{Designator, NetName, PinId};
    use generate_netlists::get_netlist_path;
    use std::path::Path;

    /// `test_schematics/sources/io-1_inputs.kicad_sch`: U1A's +INA driven by its own output, -INA only reaching
    /// R1. IO-1 is a stub so far: it collects every input pin, driven or not.
    #[test]
    fn able_to_find_unconnected_inputs() {
        let netlist = load_kicad_netlist(&get_netlist_path("io-1_inputs")).unwrap();
        let mut inputs = inputs(&netlist);
        inputs.sort_by(|a, b| a.0.0.cmp(&b.0.0));
        let inputs: Vec<_> = inputs
            .iter()
            .map(|(net, d, pin)| (net.0.as_str(), d.0.as_str(), pin.0.as_str()))
            .collect();
        assert_eq!(
            inputs,
            [("/SIG_DRIVEN", "U1", "3"), ("/SIG_FLOATING", "U1", "2")]
        );
    }

    /// The OrCAD path the IO checks take: netlist, library parts with pin types and sections, `PcbAssembly`
    #[test]
    fn orcad_netlist_reaches_pcb_assembly() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../erc_import/tests/fixtures/netlist_orcad_pstxnet.dat");
        let netlist = ecad_file_format::load_orcad_netlist(&path).unwrap();
        let pcba = PcbAssembly::new(netlist);
        let c1 = &pcba.netlist.components[&Designator("C1".into())];
        let lib_part = &pcba.netlist.lib_parts[&c1.lib_source];
        assert_eq!(lib_part.pins.len(), 2);
        let touch_int = &pcba.netlist.nets[&NetName("TOUCH_INT_N".into())];
        assert!(
            touch_int
                .nodes
                .iter()
                .any(|n| n.designator.0 == "Q34" && n.pin_id == PinId("3".into()))
        );
        assert!(inputs(&pcba.netlist).is_empty());
    }
}
