use crate::netlist::{
    Component, LibName, LibPart, LibPartName, Net, Netlist, Node, Pin, PinMode, PinType,
};
use crate::text_util::read_with_unknown_encoding;
use crate::{Designator, NetName, PinId, PinName};
use anyhow::{Error, Result};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Parser)]
#[grammar = "grammar/edif.pest"]
struct EdifParser;

pub fn load_edif_netlist(path: &Path) -> Result<Netlist> {
    let contents = read_with_unknown_encoding(path)?;

    let mut pairs = match EdifParser::parse(Rule::file, &contents) {
        Ok(pairs) => pairs,
        Err(e) => return Err(Error::msg(format!("{e}"))),
    };
    let mut lib_parts = HashMap::new();
    let mut nets = HashMap::new();
    let mut components = HashMap::new();
    // let mut symbol_rename_map: HashMap<&str, &str> = HashMap::new();
    // let mut symbol_pin_rename_map: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    for p in pairs.next().unwrap().into_inner() {
        match p.as_rule() {
            Rule::board_name => {}
            Rule::item => {
                let item = p.into_inner().next().unwrap();
                match item.as_rule() {
                    Rule::library => {
                        let mut library = item.into_inner();
                        let lib_name = library.next().unwrap().as_str();
                        let _edif_level = library.next().unwrap();
                        let _technology = library.next().unwrap();
                        for cell in library {
                            let mut cell = cell.into_inner();
                            let lib_part_name = symbol_or_rename_get(cell.next().unwrap()).0;
                            let _cell_type = cell.next().unwrap();
                            let view = cell.next().unwrap();
                            let mut view = view.into_inner();
                            let _view_name = view.next().unwrap();
                            let _view_type = view.next().unwrap();
                            if lib_name == "COMPONENT_LIB" {
                                let view_interface = view.next().unwrap();
                                let mut pins = HashMap::new();
                                for port in view_interface.into_inner() {
                                    let mut port = port.into_inner();
                                    let pin_id = symbol_or_rename_get(port.next().unwrap()).0;
                                    let pin_id = pin_id.strip_prefix('&').unwrap_or(pin_id);
                                    let direction = port.next().unwrap().as_str();
                                    let pin_ty = if direction == "INOUT" {
                                        PinType::DigitalIO
                                    } else if direction == "INPUT" {
                                        PinType::DigitalInput
                                    } else if direction == "OUTPUT" {
                                        PinType::DigitalOutput
                                    } else {
                                        PinType::Passive
                                    };
                                    pins.insert(
                                        PinId(pin_id.into()),
                                        Pin {
                                            name: PinName(String::new()),
                                            default_mode: PinMode {
                                                ty: pin_ty,
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
                                let k = (
                                    LibName("COMPONENT_LIB".to_string()),
                                    LibPartName(lib_part_name.into()),
                                );
                                lib_parts.insert(
                                    k,
                                    LibPart {
                                        description: "".to_string(),
                                        footprints: vec![],
                                        fields: Default::default(),
                                        pins,
                                        banks: Default::default(),
                                    },
                                );
                            } else if lib_name == "SHEET_LIB" {
                                let _view_interface = view.next().unwrap();
                                let view_contents = view.next().unwrap().into_inner();
                                for instance_or_net in view_contents {
                                    if instance_or_net.as_rule() == Rule::instance {
                                        let mut instance = instance_or_net.into_inner();
                                        let designator = instance.next().unwrap().as_str();
                                        let view_ref = instance.next().unwrap();
                                        let lib_part_name =
                                            view_ref.into_inner().skip(1).next().unwrap().as_str();
                                        let mut fields = HashMap::new();
                                        for property in instance {
                                            let mut property = property.into_inner();
                                            let name =
                                                symbol_or_rename_get(property.next().unwrap()).1;
                                            let _ty = property.next().unwrap();
                                            let value = property
                                                .next()
                                                .unwrap()
                                                .into_inner()
                                                .next()
                                                .unwrap()
                                                .as_str();
                                            if !value.is_empty() {
                                                fields.insert(name.to_string(), value.to_string());
                                            }
                                        }
                                        let value = if let Some(v) = fields.get("Value") {
                                            if !v.is_empty() {
                                                v.clone()
                                            } else {
                                                fields.get("Comment").cloned().unwrap_or_default()
                                            }
                                        } else {
                                            fields.get("Comment").cloned().unwrap_or_default()
                                        };
                                        let k = (
                                            LibName("COMPONENT_LIB".into()),
                                            LibPartName(lib_part_name.into()),
                                        );
                                        if let Some(footprint) = fields.get("Footprint") {
                                            if !footprint.is_empty() {
                                                if let Some(lib_part) = lib_parts.get_mut(&k) {
                                                    lib_part.footprints.push(footprint.clone());
                                                }
                                            }
                                        }
                                        components.insert(
                                            Designator(designator.into()),
                                            Component {
                                                value,
                                                description: fields
                                                    .get("Description")
                                                    .cloned()
                                                    .unwrap_or_default(),
                                                lib_source: k,
                                                fields,
                                                sections: vec![],
                                            },
                                        );
                                    } else if instance_or_net.as_rule() == Rule::net {
                                        let mut net = instance_or_net.into_inner();
                                        let net_name = symbol_or_rename_get(net.next().unwrap()).1;
                                        let joined = net.next().unwrap();
                                        let mut nodes = HashSet::new();
                                        for port_ref in joined.into_inner() {
                                            let mut port_ref = port_ref.into_inner();
                                            let pin_id = port_ref.next().unwrap().as_str();
                                            let pin_id = pin_id.strip_prefix('&').unwrap_or(pin_id);
                                            let designator = port_ref.next().unwrap().as_str();
                                            nodes.insert(Node {
                                                designator: Designator(designator.into()),
                                                pin_id: PinId(pin_id.into()),
                                            });
                                        }
                                        let mut properties = HashMap::new();
                                        for property in net {
                                            let mut property = property.into_inner();
                                            let name =
                                                symbol_or_rename_get(property.next().unwrap()).1;
                                            let _ty = property.next().unwrap();
                                            let value = property
                                                .next()
                                                .unwrap()
                                                .into_inner()
                                                .next()
                                                .unwrap()
                                                .as_str();
                                            properties.insert(name.to_string(), value.to_string());
                                        }
                                        nets.insert(
                                            NetName(net_name.into()),
                                            Net { nodes, properties },
                                        );
                                    } else {
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    Rule::edif_version => {}
                    Rule::edif_level => {}
                    Rule::keyword_map => {}
                    Rule::status => {}
                    Rule::design => {}
                    _ => {}
                }
            }
            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(Netlist {
        lib_parts,
        nets,
        components,
    })
}

fn symbol_or_rename_get(symbol_or_rename: Pair<Rule>) -> (&str, &str) {
    match symbol_or_rename.as_rule() {
        Rule::symbol => (symbol_or_rename.as_str(), symbol_or_rename.as_str()),
        Rule::rename => {
            let mut rename = symbol_or_rename.into_inner();
            let from = rename.next().unwrap().as_str();
            let to = rename.next().unwrap().into_inner().next().unwrap().as_str();
            (from, to)
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn can_read_edif_netlist() {
        let path = Path::new("test_input/netlist_altium_edif.edf");
        let netlist = load_edif_netlist(&path).unwrap();
        println!("{:#?}", netlist);
        let part = netlist
            .lib_parts
            .get(&(
                LibName("COMPONENT_LIB".to_string()),
                LibPartName("CONN_2_3__6_".into()),
            ))
            .unwrap();
        assert_eq!(part.pins.iter().count(), 6);
        let pin = part.pins.get(&PinId("1".into())).unwrap();
        assert_eq!(pin.default_mode.ty, PinType::DigitalIO);

        assert_eq!(netlist.nets.iter().count(), 3);
        let net = netlist.nets.get(&NetName("NetR13_1".into())).unwrap();
        assert_eq!(
            net.nodes,
            [
                Node {
                    designator: Designator("R13".into()),
                    pin_id: PinId("1".into())
                },
                Node {
                    designator: Designator("SW4".into()),
                    pin_id: PinId("4".into())
                }
            ]
            .into()
        );

        assert_eq!(netlist.components.iter().count(), 2);
        let component = netlist.components.get(&Designator("J2".into())).unwrap();
        assert_eq!(component.value, "95278-101A06LF");
        assert_eq!(component.fields.get("ChannelOffset").unwrap(), "1");

        // for (line_nr, raw_line) in buf.split(|b| b == &b'\n').enumerate() {
        //     if core::str::from_utf8(raw_line).is_err() {
        //         let (decoded, _, any_malformed) = encoding.decode(raw_line);
        //         let decoded = decoded.replace('\r', "");
        //         println!("line_nr: {line_nr} {decoded} {any_malformed}");
        //     }
        // }
    }
}
