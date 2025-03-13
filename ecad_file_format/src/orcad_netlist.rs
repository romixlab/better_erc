use crate::netlist::{LibName, LibPart, LibPartName, Net, Netlist, Node};
use crate::{Designator, NetName, PinId};
use anyhow::{Error, Result};
use pest::Parser;
use pest_derive::Parser;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::path::PathBuf;

#[derive(Parser)]
#[grammar = "grammar/orcad_capture_netlist.pest"]
struct NetListParser;

pub fn load_orcad_netlist(pstxnet_path: &PathBuf) -> Result<Netlist> {
    let contents = read_to_string(pstxnet_path)?;
    let pstxprt_path = PathBuf::from(
        pstxnet_path
            .to_str()
            .unwrap()
            .replace("pstxnet.dat", "pstxprt.dat"),
    );
    let mut components = part_list_parser::load_lib_parts(&pstxprt_path)?;
    let pstchip_path = PathBuf::from(
        pstxnet_path
            .to_str()
            .unwrap()
            .replace("pstxnet.dat", "pstchip.dat"),
    );
    let lib_parts = lib_parts_parser::load_lib_parts(&pstchip_path, &mut components)?;

    let pairs = match NetListParser::parse(Rule::file, &contents) {
        Ok(pairs) => pairs,
        Err(e) => return Err(Error::msg(format!("{e}"))),
    };
    let mut nets = HashMap::new();
    // println!("{pairs:#?}");
    let rule_file = pairs.into_iter().next().unwrap();
    for pair in rule_file.into_inner().into_iter() {
        match pair.as_rule() {
            Rule::exporter_comment => {}
            Rule::net => {
                let mut net_name = None;
                let mut nodes = HashSet::new();
                for pair in pair.into_inner().into_iter() {
                    match pair.as_rule() {
                        Rule::net_name => {
                            net_name = Some(pair.into_inner().next().unwrap().as_str().to_string());
                        }
                        Rule::full_net_name => {}
                        Rule::c_signal => {}
                        Rule::node => {
                            let mut part_ref = None;
                            let mut part_pin = None;
                            for pair in pair.into_inner().into_iter() {
                                match pair.as_rule() {
                                    Rule::designator => {
                                        part_ref = Some(pair.as_str().to_string());
                                    }
                                    Rule::pin_id => {
                                        part_pin = Some(pair.as_str().to_string());
                                    }
                                    Rule::instance_name => {}
                                    Rule::pin_name => {}
                                    _ => {}
                                }
                            }
                            if let (Some(part_ref), Some(part_pin)) = (part_ref, part_pin) {
                                nodes.insert(Node {
                                    designator: Designator(part_ref),
                                    pin_id: PinId(part_pin),
                                });
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(net_name) = net_name {
                    // TODO: Emit warning if replaces existing net?
                    nets.insert(NetName(net_name), Net { nodes });
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

mod lib_parts_parser {
    use crate::netlist::{Component, LibName, LibPart, LibPartName, Pin, PinMode, PinType};
    use crate::{Designator, PinId, PinName};
    use anyhow::Error;
    use pest::Parser;
    use pest_derive::Parser;
    use std::collections::HashMap;
    use std::fs::read_to_string;
    use std::path::PathBuf;

    #[derive(Parser)]
    #[grammar = "grammar/orcad_capture_library_parts.pest"]
    struct LibPartsParser;

    pub(super) fn load_lib_parts(
        pstchip_path: &PathBuf,
        components: &mut HashMap<Designator, Component>,
    ) -> anyhow::Result<HashMap<(LibName, LibPartName), LibPart>> {
        let contents = read_to_string(pstchip_path)?;

        let pairs = match LibPartsParser::parse(Rule::file, &contents) {
            Ok(pairs) => pairs,
            Err(e) => return Err(Error::msg(format!("{e}"))),
        };
        let mut lib_parts = HashMap::new();
        let rule_file = pairs.into_iter().next().unwrap();
        for pair in rule_file.into_inner().into_iter() {
            match pair.as_rule() {
                Rule::exporter_comment => {}
                Rule::primitive => {
                    let mut primitive = pair.into_inner();
                    let primitive_name = primitive
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str();
                    let lib_part_name = LibPartName(primitive_name.into());
                    let pins = primitive.next().unwrap();
                    let body = primitive.next().unwrap();

                    let component_sections = components
                        .values()
                        .find(|c| c.lib_source.1 == lib_part_name)
                        .map(|c| c.sections.as_slice());
                    let mut pins_collect = HashMap::new();
                    for pin in pins.into_inner() {
                        let mut pin = pin.into_inner();
                        let pin_name = pin.next().unwrap().into_inner().next().unwrap().as_str();
                        let mut params = HashMap::new();
                        for param in pin {
                            let mut param = param.into_inner();
                            let param_name = param.next().unwrap().as_str();
                            let param_value =
                                param.next().unwrap().into_inner().next().unwrap().as_str();
                            params.insert(param_name, param_value);
                        }

                        let Some(pin_id) = params.get("PIN_NUMBER") else {
                            continue;
                        };
                        let pin_id = &pin_id[1..pin_id.len() - 1];
                        let mut section_name = None;
                        let pin_id = if let Some(sections) = component_sections {
                            if sections.len() > 1 {
                                let nulls_and_name = pin_id.split(',').into_iter();
                                let idx_pin_id =
                                    nulls_and_name.enumerate().find(|(_, n)| n != &"0");
                                if let Some((idx, pin_id)) = idx_pin_id {
                                    section_name = Some(sections[idx].name.clone());
                                    pin_id
                                } else {
                                    pin_id
                                }
                            } else {
                                pin_id
                            }
                        } else {
                            pin_id
                        };

                        let pin_ty = if params.get("BIDIRECTIONAL") == Some(&"TRUE") {
                            PinType::DigitalIO
                        } else if params.get("PINUSE") == Some(&"POWER") {
                            PinType::PowerUnspecified
                        } else if params.contains_key("OUTPUT_LOAD") {
                            if params.get("OUTPUT_TYPE") == Some(&"(OC,AND)") {
                                PinType::OpenCollector
                            } else {
                                PinType::DigitalOutput
                            }
                        } else if params.contains_key("INPUT_LOAD") {
                            PinType::DigitalInput
                        } else {
                            PinType::Passive
                        };

                        pins_collect.insert(
                            PinId(pin_id.to_string()),
                            Pin {
                                name: PinName(pin_name.into()),
                                default_mode: PinMode {
                                    ty: pin_ty,
                                    pull_up: None,
                                    pull_down: None,
                                    io_standard: None,
                                },
                                alternate_modes: Default::default(),
                                bank_name: None,
                                section_name,
                            },
                        );
                    }

                    let mut fields = HashMap::new();
                    for param in body.into_inner() {
                        let mut param = param.into_inner();
                        let param_name = param.next().unwrap().as_str();
                        let param_value =
                            param.next().unwrap().into_inner().next().unwrap().as_str();
                        fields.insert(param_name.to_string(), param_value.to_string());
                    }
                    if let Some(value) = fields.get("VALUE") {
                        if !value.is_empty() {
                            for component in components.values_mut() {
                                if component.lib_source.1.0 == primitive_name {
                                    component.value = value.to_string();
                                }
                            }
                        }
                    }

                    let k = (LibName("pstchip".into()), lib_part_name);
                    lib_parts.insert(
                        k,
                        LibPart {
                            description: "".to_string(),
                            footprints: fields
                                .get("JEDEC_TYPE")
                                .map(|v| vec![v.to_string()])
                                .unwrap_or(vec![]),
                            fields,
                            pins: pins_collect,
                            banks: Default::default(),
                        },
                    );
                }
                Rule::EOI => {}
                _ => {}
            }
        }

        Ok(lib_parts)
    }
}

mod part_list_parser {
    use crate::Designator;
    use crate::netlist::{Component, ComponentSection, LibName, LibPart, LibPartName};
    use anyhow::Error;
    use pest::Parser;
    use pest_derive::Parser;
    use regex::Regex;
    use std::collections::HashMap;
    use std::fs::read_to_string;
    use std::path::PathBuf;

    #[derive(Parser)]
    #[grammar = "grammar/orcad_capture_part_list.pest"]
    struct PartListParser;

    pub(super) fn load_lib_parts(
        pstchip_path: &PathBuf,
    ) -> anyhow::Result<HashMap<Designator, Component>> {
        let contents = read_to_string(pstchip_path)?;

        let pairs = match PartListParser::parse(Rule::file, &contents) {
            Ok(pairs) => pairs,
            Err(e) => return Err(Error::msg(format!("{e}"))),
        };
        let mut components = HashMap::new();
        let rule_file = pairs.into_iter().next().unwrap();
        let page_re = Regex::new("@[\\w.()]+:page(\\d+).*")?;
        for pair in rule_file.into_inner().into_iter() {
            match pair.as_rule() {
                Rule::exporter_comment => {}
                Rule::directives => {}
                Rule::parts => {
                    for part in pair.into_inner() {
                        let mut part = part.into_inner();
                        let designator = part.next().unwrap().as_str();
                        let lib_part_name =
                            part.next().unwrap().into_inner().next().unwrap().as_str();
                        let mut sections = vec![];
                        for section in part.next().unwrap().into_inner() {
                            let mut section = section.into_inner();
                            let _section_number = section.next().unwrap();
                            let _instance_name = section.next().unwrap();
                            let _c_path = section.next().unwrap();
                            let p_path = section.next().unwrap().as_str();
                            let page_number = if let Some(caps) = page_re.captures(p_path) {
                                if let Some(c) = caps.get(1) {
                                    c.as_str().parse::<u32>().ok()
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            let _prim_file = section.next().unwrap();
                            let section_name = section
                                .next()
                                .unwrap()
                                .into_inner()
                                .next()
                                .unwrap()
                                .as_str();
                            sections.push(ComponentSection {
                                name: section_name.to_string(),
                                page_number,
                            })
                        }
                        components.insert(
                            Designator(designator.to_string()),
                            Component {
                                value: "".to_string(),
                                description: "".to_string(),
                                lib_source: (
                                    LibName("pstchip".into()),
                                    LibPartName(lib_part_name.into()),
                                ),
                                fields: Default::default(),
                                sections,
                            },
                        );
                    }
                }
                Rule::EOI => {}
                _ => {}
            }
        }

        Ok(components)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netlist::PinType;

    #[test]
    fn can_load_orcad_netlist() {
        let netlist =
            load_orcad_netlist(&PathBuf::from("test_input/netlist_orcad_pstxnet.dat")).unwrap();
        // println!("{:#?}", netlist);
        assert_eq!(netlist.nets.len(), 3);
        assert_eq!(
            netlist.nets.get(&NetName("TOUCH_INT_N".into())),
            Some(&Net {
                nodes: [
                    Node {
                        designator: Designator("R610".to_string()),
                        pin_id: PinId("2".to_string())
                    },
                    Node {
                        designator: Designator("Q34".to_string()),
                        pin_id: PinId("3".to_string())
                    },
                    Node {
                        designator: Designator("R636".to_string()),
                        pin_id: PinId("1".to_string())
                    }
                ]
                .into(),
            })
        );
        let mut found = false;
        for ((_, lib_part_name), part) in &netlist.lib_parts {
            if !lib_part_name.0.contains("EMMC") {
                continue;
            }
            found = true;
            let pin = part.pins.get(&PinId("A3".into())).unwrap();
            assert_eq!(pin.section_name, Some("A".to_string()));
            assert_eq!(pin.default_mode.ty, PinType::DigitalIO);
        }
        assert!(found);
    }
}
