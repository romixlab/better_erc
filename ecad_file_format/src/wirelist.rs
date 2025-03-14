use crate::netlist::{
    Component, LibName, LibPart, LibPartName, Net, Netlist, Node, Pin, PinMode, PinType,
};
use crate::text_util::read_with_unknown_encoding;
use crate::{Designator, NetName, PinId, PinName};
use anyhow::{Error, Result};
use pest::Parser;
use pest_derive::Parser;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Parser)]
#[grammar = "grammar/wirelist.pest"]
#[allow(dead_code)]
struct WireListParser;

pub(crate) fn load_wirelist_netlist(path: &Path) -> Result<Netlist> {
    let contents = read_with_unknown_encoding(path)?;

    let mut file = match WireListParser::parse(Rule::file, &contents) {
        Ok(pairs) => pairs,
        Err(e) => return Err(Error::msg(format!("{e}"))),
    };
    let mut file = file.next().unwrap().into_inner();
    let component_list = file.next().unwrap();
    let wire_list = file.next().unwrap();

    let mut components = HashMap::new();
    for component in component_list.into_inner() {
        let mut component = component.into_inner();
        let part_number = component.next().unwrap().as_str();
        let designator = component.next().unwrap().as_str();
        let footprint = component.next().unwrap().as_str();
        components.insert(
            Designator(designator.into()),
            Component {
                value: part_number.to_string(),
                description: "".to_string(),
                lib_source: (LibName("".into()), LibPartName("".into())),
                fields: [("Footprint".to_string(), footprint.to_string())].into(),
                sections: vec![],
            },
        );
    }

    let mut nets = HashMap::new();
    let mut lib_parts: HashMap<_, LibPart> = HashMap::new();
    for net in wire_list.into_inner() {
        let mut net = net.into_inner();
        let _net_index = net.next().unwrap();
        let net_name = net.next().unwrap().as_str();
        let mut nodes = HashSet::new();
        for connection in net {
            let mut connection = connection.into_inner();
            let designator = connection.next().unwrap().as_str();
            let pin_id = connection.next().unwrap().as_str();
            let pin_name = connection.next().unwrap().as_str();
            let io_type = connection.next().unwrap().as_str();
            let _part_value = connection.next().unwrap().as_str();
            nodes.insert(Node {
                designator: Designator(designator.into()),
                pin_id: PinId(pin_id.into()),
            });

            let io_type = match io_type {
                "PASSIVE" => PinType::Passive,
                "OUTPUT" => PinType::DigitalOutput,
                "INPUT" => PinType::DigitalInput,
                "I/O" => PinType::DigitalIO,
                "OPEN COLLECTOR" => PinType::OpenCollector,
                "OPEN EMITTER" => PinType::OpenEmitter,
                "POWER" => PinType::PowerUnspecified,
                _ => PinType::Passive,
            };
            let k = (LibName("".into()), LibPartName(designator.to_string()));
            lib_parts.entry(k).or_default().pins.insert(
                PinId(pin_id.into()),
                Pin {
                    name: PinName(pin_name.into()),
                    default_mode: PinMode {
                        ty: io_type,
                        pull_up: None,
                        pull_down: None,
                        io_standard: None,
                    },
                    alternate_modes: Default::default(),
                    bank_name: None,
                    section_name: None,
                },
            );
        }
        nets.insert(
            NetName(net_name.into()),
            Net {
                nodes,
                properties: Default::default(),
            },
        );
    }
    Ok(Netlist {
        lib_parts,
        nets,
        components,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_read_altium_wirelist_netlist() {
        let netlist =
            load_wirelist_netlist(Path::new("test_input/netlist_altium_wirelist.net")).unwrap();
        // let netlist = load_wirelist_netlist(Path::new(
        //     ,
        // ))
        // .unwrap();
        println!("{:#?}", netlist);
    }
}
