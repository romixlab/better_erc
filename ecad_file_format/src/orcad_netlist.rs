use crate::netlist::{Net, Netlist, Node};
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

pub fn load_orcad_netlist(path: &PathBuf) -> Result<Netlist> {
    let contents = read_to_string(path)?;
    let pairs = match NetListParser::parse(Rule::file, &contents) {
        Ok(pairs) => pairs,
        Err(e) => return Err(Error::msg(format!("{e}"))),
    };
    let parts = HashMap::new();
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
    Ok(Netlist { parts, nets })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_load_orcad_netlist() {
        let netlist =
            load_orcad_netlist(&PathBuf::from("test_input/netlist_orcad_pstxnet.dat")).unwrap();
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
        )
    }
}
