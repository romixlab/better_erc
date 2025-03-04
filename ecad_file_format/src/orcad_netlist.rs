use crate::netlist::{Net, Netlist, Node};
use anyhow::{Error, Result};
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::PathBuf;

pub fn load_orcad_netlist(path: &PathBuf) -> Result<Netlist> {
    let contents = read_to_string(path)?;
    let pairs = match NetListParser::parse(Rule::file, &contents) {
        Ok(pairs) => pairs,
        Err(e) => return Err(Error::msg(format!("{e}"))),
    };
    let mut parts = HashMap::new();
    let mut nets = HashMap::new();
    // println!("{pairs:#?}");
    let rule_file = pairs.into_iter().next().unwrap();
    for pair in rule_file.into_inner().into_iter() {
        match pair.as_rule() {
            Rule::exporter_comment => {}
            Rule::net => {
                let mut net_name = None;
                let mut nodes = vec![];
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
                                nodes.push(Node { part_ref, part_pin });
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(net_name) = net_name {
                    // TODO: Emit warning if replaces existing net?
                    nets.insert(net_name, Net { nodes });
                }
            }
            Rule::EOI => {}
            _ => {}
        }
    }
    Ok(Netlist { parts, nets })
}

#[derive(Parser)]
#[grammar = "grammar/orcad_capture_netlist.pest"]
pub struct NetListParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_load_orcad_netlist() {
        let netlist =
            load_orcad_netlist(&PathBuf::from("test_input/netlist_orcad_pstxnet.dat")).unwrap();
        println!("{}", netlist);
    }
}
