use crate::config::{I2C_ACCEPTABLE_PULL_UP_RANGE, MAX_TIE_RESISTANCE};
use crate::util::collapse_underscores;
use ecad_file_format::netlist::Netlist;
use ecad_file_format::passive_value::Ohm;
use ecad_file_format::{Designator, NetName, PinName};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct I2cBus {
    pub derived_name: String,
    pub scl_net: NetName,
    pub sda_net: NetName,
    pub pull_up: Option<I2cPullUp>,
    pub nodes: Vec<I2cNode>,
}

#[derive(Debug)]
pub struct I2cPullUp {
    pub scl: Designator,
    pub sda: Designator,
    pub v_net: NetName,
}

#[derive(Debug, PartialEq)]
pub enum I2cNode {
    Device(Designator),
    VoltageTranslator {
        part: Designator,
        other_side: String,
    },
    VoltageTranslatorDiscrete {
        scl_fet: Designator,
        sda_fet: Designator,
        other_side: String,
    },
    Connector(Designator),
    /// 0R, net tie, solder tie
    Tie {
        scl_tie: Designator,
        sda_tie: Designator,
        other_side: String,
    },
    TestPoint(Designator),
    Unknown(Designator),
}

#[derive(Debug, PartialEq)]
pub enum I2cDiagnosticKind {
    RedundantPullUps {
        redundant_pull_ups: HashSet<Designator>,
    },
    NoPullUps,
    PullUpToNowhere,
    WrongPullUpValue {
        parse_message: String,
    },
    NonStandardPullUps {
        resistance: Ohm,
    },
    NonEqualPullUps {
        scl_resistance: Ohm,
        sda_resistance: Ohm,
    },
    TieTooHighValue {
        resistance: Ohm,
        other_side: String,
    },
    UnknownNode {
        designator: Designator,
    },
}

#[derive(Debug, PartialEq)]
pub struct I2cDiagnostic {
    pub derived_name: String,
    pub kind: I2cDiagnosticKind,
}

#[derive(Debug)]
pub struct I2cBuses {
    pub by_name: HashMap<String, I2cBus>,
    pub direct_segments: Vec<HashSet<String>>,
    pub same_bus_segments: Vec<HashSet<String>>,
}

pub fn find_i2c_buses(netlist: &Netlist, diagnostics: &mut Vec<I2cDiagnostic>) -> I2cBuses {
    let mut buses = vec![];
    for scl_net in netlist.nets.keys() {
        let Some(scl_start) = scl_net.0.find("SCL") else {
            continue;
        };
        let prefix = &scl_net.0[..scl_start];
        let suffix = if scl_start + 3 < scl_net.len() {
            &scl_net.0[(scl_start + 3)..]
        } else {
            ""
        };
        let sda_net = NetName(format!("{}SDA{}", prefix, suffix));
        if !netlist.nets.keys().any(|k| k == &sda_net) {
            continue;
        }
        let derived_name = collapse_underscores(format!("{}I2C{}", prefix, suffix).as_str());
        let mut connected_parts = netlist.any_net_parts(&[scl_net, &sda_net]);
        let pull_up = find_pull_ups(
            netlist,
            diagnostics,
            scl_net,
            &sda_net,
            &derived_name,
            &mut connected_parts,
        );
        buses.push(I2cBus {
            derived_name,
            scl_net: scl_net.clone(),
            sda_net,
            pull_up,
            nodes: parts_to_nodes(netlist, connected_parts),
        });
    }
    look_for_non_standard_pull_ups(netlist, &buses, diagnostics);
    look_for_bus_interconnects(netlist, &mut buses, diagnostics);
    let mut buses = I2cBuses {
        by_name: buses
            .into_iter()
            .map(|bus| (bus.derived_name.clone(), bus))
            .collect(),
        direct_segments: vec![],
        same_bus_segments: vec![],
    };
    buses.collect_nodes(netlist);
    buses.collect_segments();
    buses.warning_unknown_nodes(diagnostics);
    buses.check_pull_ups(netlist, diagnostics);
    buses
}

fn find_pull_ups(
    netlist: &Netlist,
    diagnostics: &mut Vec<I2cDiagnostic>,
    scl_net: &NetName,
    sda_net: &NetName,
    derived_name: &String,
    mut connected_parts: &mut HashSet<Designator>,
) -> Option<I2cPullUp> {
    let mut pull_up_chains = netlist.find_net_chains(
        scl_net,
        &[Designator::is_resistor, Designator::is_resistor],
        &sda_net,
    );
    let pull_up = if let Some(chain) = pull_up_chains.pop() {
        let scl_pull_up = chain[0].1.clone();
        let sda_pull_up = chain[1].1.clone();
        let v_net = netlist.pin_net(&chain[1].1, &chain[1].0).unwrap();
        let pull_up = I2cPullUp {
            scl: scl_pull_up,
            sda: sda_pull_up,
            v_net,
        };
        // remove pull-ups from parts on I2C lines
        connected_parts.remove(&pull_up.scl);
        connected_parts.remove(&pull_up.sda);
        // TODO: look in respect to any power net instead
        look_for_redundant_pull_ups(
            netlist,
            diagnostics,
            scl_net,
            &sda_net,
            &derived_name,
            &mut connected_parts,
            &pull_up,
        );
        Some(pull_up)
    } else {
        None
    };
    pull_up
}

fn look_for_redundant_pull_ups(
    netlist: &Netlist,
    diagnostics: &mut Vec<I2cDiagnostic>,
    scl_net: &NetName,
    sda_net: &NetName,
    derived_name: &String,
    connected_parts: &mut HashSet<Designator>,
    pull_up: &I2cPullUp,
) {
    // find redundant pull-ups
    let mut redundant_chains =
        netlist.find_net_chains(scl_net, &[Designator::is_resistor], &pull_up.v_net);
    redundant_chains.extend(netlist.find_net_chains(
        &sda_net,
        &[Designator::is_resistor],
        &pull_up.v_net,
    ));
    redundant_chains.retain(|c| {
        !c.iter()
            .any(|(_, d)| d == &pull_up.scl || d == &pull_up.sda)
    });
    // remove redundant pull-ups from connected parts as well as they are already accounted for
    let mut redundant_pull_ups = HashSet::new();
    for c in redundant_chains {
        for (_, d) in c {
            connected_parts.remove(&d);
            redundant_pull_ups.insert(d);
        }
    }
    if !redundant_pull_ups.is_empty() {
        diagnostics.push(I2cDiagnostic {
            derived_name: derived_name.clone(),
            kind: I2cDiagnosticKind::RedundantPullUps { redundant_pull_ups },
        })
    }
}

fn look_for_non_standard_pull_ups(
    netlist: &Netlist,
    buses: &[I2cBus],
    diagnostics: &mut Vec<I2cDiagnostic>,
) {
    for bus in buses {
        let Some(pull_up) = &bus.pull_up else {
            continue;
        };
        let scl_value = netlist.resistance(&pull_up.scl);
        let sda_value = netlist.resistance(&pull_up.sda);
        match (scl_value, sda_value) {
            (Ok(scl_value), Ok(sda_value)) => {
                check_pull_up_range(&scl_value, bus.derived_name.as_str(), diagnostics);
                if scl_value != sda_value {
                    check_pull_up_range(&sda_value, bus.derived_name.as_str(), diagnostics);
                    diagnostics.push(I2cDiagnostic {
                        derived_name: bus.derived_name.clone(),
                        kind: I2cDiagnosticKind::NonEqualPullUps {
                            scl_resistance: scl_value,
                            sda_resistance: sda_value,
                        },
                    });
                }
            }
            (Err(e), Ok(value)) | (Ok(value), Err(e)) => {
                check_pull_up_range(&value, bus.derived_name.as_str(), diagnostics);
                diagnostics.push(I2cDiagnostic {
                    derived_name: bus.derived_name.clone(),
                    kind: I2cDiagnosticKind::WrongPullUpValue {
                        parse_message: format!("{e}"),
                    },
                });
            }
            (Err(e1), Err(e2)) => {
                diagnostics.push(I2cDiagnostic {
                    derived_name: bus.derived_name.clone(),
                    kind: I2cDiagnosticKind::WrongPullUpValue {
                        parse_message: format!("{e1}"),
                    },
                });
                diagnostics.push(I2cDiagnostic {
                    derived_name: bus.derived_name.clone(),
                    kind: I2cDiagnosticKind::WrongPullUpValue {
                        parse_message: format!("{e2}"),
                    },
                });
            }
        }
    }
}

fn check_pull_up_range(value: &Ohm, bus_name: &str, diagnostics: &mut Vec<I2cDiagnostic>) {
    if value <= I2C_ACCEPTABLE_PULL_UP_RANGE.start() || value >= I2C_ACCEPTABLE_PULL_UP_RANGE.end()
    {
        diagnostics.push(I2cDiagnostic {
            derived_name: bus_name.to_string(),
            kind: I2cDiagnosticKind::NonStandardPullUps {
                resistance: value.clone(),
            },
        })
    }
}

fn parts_to_nodes(netlist: &Netlist, parts: HashSet<Designator>) -> Vec<I2cNode> {
    // TODO: implement I2C parts to nodes
    let mut nodes = vec![];
    for designator in parts {
        if designator.0.starts_with('J') {
            nodes.push(I2cNode::Connector(designator));
        } else if designator.0.starts_with('U') {
            if let Some(component) = netlist.components.get(&designator) {
                if let Some(lib_part) = netlist.lib_parts.get(&component.lib_source) {
                    let d = lib_part.description.to_lowercase();
                    if d.contains("shifter") || d.contains("translator") {
                        // put as unknown for now, collect later
                        nodes.push(I2cNode::Unknown(designator));
                    } else {
                        nodes.push(I2cNode::Device(designator));
                    }
                } else {
                    nodes.push(I2cNode::Unknown(designator));
                }
            } else {
                nodes.push(I2cNode::Unknown(designator));
            }
        } else if designator.0.starts_with("TP") {
            nodes.push(I2cNode::TestPoint(designator));
        } else {
            nodes.push(I2cNode::Unknown(designator));
        }
    }
    nodes
}

impl I2cNode {
    fn mentions_part(&self, designator: &Designator) -> bool {
        match self {
            I2cNode::VoltageTranslatorDiscrete {
                scl_fet, sda_fet, ..
            } => scl_fet == designator || sda_fet == designator,
            I2cNode::Tie {
                scl_tie, sda_tie, ..
            } => scl_tie == designator || sda_tie == designator,
            _ => false,
        }
    }
}

fn look_for_bus_interconnects(
    netlist: &Netlist,
    buses: &mut Vec<I2cBus>,
    diagnostics: &mut Vec<I2cDiagnostic>,
) {
    let mut modifications: HashMap<String, Vec<I2cNode>> = HashMap::new();
    let mut create_buses: Vec<I2cBus> = vec![];
    for bus in buses.iter() {
        for node in &bus.nodes {
            let I2cNode::Unknown(designator) = &node else {
                continue;
            };
            if modifications
                .values()
                .any(|m| m.iter().any(|n| n.mentions_part(designator)))
            {
                continue;
            }
            if designator.is_resistor() || designator.is_transistor() {
                let nets = netlist.part_nets_exclude_pin_names(
                    designator,
                    &[&PinName("G".into()), &PinName("GATE".into())],
                );
                let is_scl = nets.iter().any(|n| n == &bus.scl_net);
                let other_side = if is_scl {
                    nets.iter().find(|n| *n != &bus.scl_net)
                } else {
                    nets.iter().find(|n| *n != &bus.sda_net)
                };
                let Some(other_side) = other_side else {
                    // shouldn't happen
                    continue;
                };
                let complementary_net = if is_scl { &bus.sda_net } else { &bus.scl_net };
                let other_bus = buses
                    .iter()
                    .find(|b| b.scl_net == *other_side || b.sda_net == *other_side);
                if let Some(other_bus) = other_bus {
                    let other_complementary_net = if is_scl {
                        &other_bus.sda_net
                    } else {
                        &other_bus.scl_net
                    };
                    let goes_through = if designator.is_resistor() {
                        Designator::is_resistor
                    } else {
                        Designator::is_transistor
                    };
                    let complementary_part = netlist.find_net_chains(
                        other_complementary_net,
                        &[goes_through],
                        complementary_net,
                    );
                    if let Some(complementary_part) = complementary_part.first() {
                        if let Some(p) = complementary_part.first() {
                            let complementary_part = &p.1;
                            if designator.is_resistor() {
                                modifications
                                    .entry(bus.derived_name.clone())
                                    .or_default()
                                    .push(I2cNode::Tie {
                                        scl_tie: designator.clone(),
                                        sda_tie: complementary_part.clone(),
                                        other_side: other_bus.derived_name.clone(),
                                    });
                                modifications
                                    .entry(other_bus.derived_name.clone())
                                    .or_default()
                                    .push(I2cNode::Tie {
                                        scl_tie: designator.clone(),
                                        sda_tie: complementary_part.clone(),
                                        other_side: bus.derived_name.clone(),
                                    });
                                check_tie_resistance(
                                    netlist,
                                    diagnostics,
                                    bus.derived_name.as_str(),
                                    designator,
                                    other_bus.derived_name.as_str(),
                                    complementary_part,
                                );
                            } else {
                                modifications
                                    .entry(bus.derived_name.clone())
                                    .or_default()
                                    .push(I2cNode::VoltageTranslatorDiscrete {
                                        scl_fet: designator.clone(),
                                        sda_fet: complementary_part.clone(),
                                        other_side: other_bus.derived_name.clone(),
                                    });
                                modifications
                                    .entry(other_bus.derived_name.clone())
                                    .or_default()
                                    .push(I2cNode::VoltageTranslatorDiscrete {
                                        scl_fet: designator.clone(),
                                        sda_fet: complementary_part.clone(),
                                        other_side: bus.derived_name.clone(),
                                    });
                            }
                        }
                    }
                } else {
                    // other side might be an IC or connector connected through 0R with unnamed nets
                    // find parts on the other side
                    let mut other_side_parts = netlist.any_net_parts(&[other_side]);
                    other_side_parts.remove(designator);
                    // println!("other_side: {other_side:?}, parts: {other_side_parts:?}");
                    let mut potential_targets = vec![];
                    // find other resistors or transistors in this bus connected to one of found parts
                    for node in &bus.nodes {
                        let I2cNode::Unknown(complementary_part) = &node else {
                            continue;
                        };
                        if complementary_part == designator {
                            continue;
                        }
                        if !complementary_part.is_resistor() && !complementary_part.is_transistor()
                        {
                            continue;
                        }
                        for other_part in &other_side_parts {
                            if netlist.are_parts_connected(complementary_part, other_part) {
                                potential_targets
                                    .push((other_part.clone(), complementary_part.clone()));
                            }
                        }
                    }
                    // println!(
                    //     "other_side: {other_side:?} potential_targets: {:?}",
                    //     potential_targets
                    // );
                    if potential_targets.len() == 1 {
                        let (target, complementary_part) = potential_targets.pop().unwrap();
                        let adhoc_name = format!("{}_to_{}", bus.derived_name, target.0);
                        let other_part_nets = netlist.part_nets(&complementary_part);
                        let other_side_complementary_net = other_part_nets
                            .iter()
                            .find(|n| *n != complementary_net)
                            .unwrap()
                            .clone();
                        let (scl_net, sda_net) = if is_scl {
                            (other_side.clone(), other_side_complementary_net)
                        } else {
                            (other_side_complementary_net, other_side.clone())
                        };
                        let mut connected_parts = netlist.any_net_parts(&[&scl_net, &sda_net]);
                        let pull_up = find_pull_ups(
                            netlist,
                            diagnostics,
                            &scl_net,
                            &sda_net,
                            &adhoc_name,
                            &mut connected_parts,
                        );
                        create_buses.push(I2cBus {
                            derived_name: adhoc_name.clone(),
                            scl_net,
                            sda_net,
                            pull_up,
                            nodes: parts_to_nodes(netlist, connected_parts),
                        });
                        if designator.is_resistor() {
                            check_tie_resistance(
                                netlist,
                                diagnostics,
                                bus.derived_name.as_str(),
                                designator,
                                adhoc_name.as_str(),
                                &complementary_part,
                            );
                        }
                        modifications
                            .entry(bus.derived_name.clone())
                            .or_default()
                            .push(I2cNode::Tie {
                                scl_tie: designator.clone(),
                                sda_tie: complementary_part.clone(),
                                other_side: adhoc_name.clone(),
                            });
                        modifications
                            .entry(adhoc_name)
                            .or_default()
                            .push(I2cNode::Tie {
                                scl_tie: designator.clone(),
                                sda_tie: complementary_part.clone(),
                                other_side: bus.derived_name.clone(),
                            });
                    }
                }
            }
        }
    }
    // println!("create buses: {create_buses:?}");
    for create_bus in create_buses {
        buses.push(create_bus);
    }
    // println!("modifications: {modifications:?}");
    for (to_bus, add_node) in modifications {
        let Some(bus) = buses.iter_mut().find(|bus| bus.derived_name == to_bus) else {
            continue;
        };
        for node in add_node {
            bus.nodes.retain(|n| {
                if let I2cNode::Unknown(d) = n {
                    !node.mentions_part(d)
                } else {
                    true
                }
            });
            bus.nodes.push(node);
        }
    }
}

fn check_tie_resistance(
    netlist: &Netlist,
    diagnostics: &mut Vec<I2cDiagnostic>,
    bus_name: &str,
    scl_tie: &Designator,
    other_side: &str,
    sda_tie: &Designator,
) {
    let scl_tie_resistance = netlist.resistance(scl_tie).unwrap_or(Ohm(0.0));
    let sda_tie_resistance = netlist.resistance(sda_tie).unwrap_or(Ohm(0.0));
    if scl_tie_resistance > MAX_TIE_RESISTANCE || sda_tie_resistance > MAX_TIE_RESISTANCE {
        diagnostics.push(I2cDiagnostic {
            derived_name: bus_name.to_string(),
            kind: I2cDiagnosticKind::TieTooHighValue {
                resistance: Ohm(scl_tie_resistance.0.max(sda_tie_resistance.0)),
                other_side: other_side.to_string(),
            },
        });
    }
}

impl I2cBuses {
    fn warning_unknown_nodes(&self, diagnostics: &mut Vec<I2cDiagnostic>) {
        for (bus_name, bus) in &self.by_name {
            for node in &bus.nodes {
                if let I2cNode::Unknown(designator) = node {
                    diagnostics.push(I2cDiagnostic {
                        derived_name: bus_name.to_string(),
                        kind: I2cDiagnosticKind::UnknownNode {
                            designator: designator.clone(),
                        },
                    });
                }
            }
        }
    }

    fn collect_nodes(&mut self, netlist: &Netlist) {
        let mut add_nodes = HashMap::new();
        let mut remove_nodes = HashMap::new();
        for (bus_name, bus) in &self.by_name {
            for node in &bus.nodes {
                if let I2cNode::Unknown(d) = node {
                    // description of this part contains shifter or translator, as all other IC nodes were already converted to Device
                    // find to which one bus it leads for it to be a voltage translator
                    let mut nets = netlist.part_nets(d);
                    nets.remove(&bus.scl_net);
                    nets.remove(&bus.sda_net);
                    let mut other_buses = vec![];
                    for (other_bus_name, bus) in &self.by_name {
                        if nets.contains(&bus.scl_net) {
                            other_buses.push(other_bus_name.clone());
                        }
                    }
                    if other_buses.len() == 1 {
                        let other_bus_name = other_buses.pop().unwrap();
                        add_nodes.insert(
                            bus_name.clone(),
                            I2cNode::VoltageTranslator {
                                part: d.clone(),
                                other_side: other_bus_name,
                            },
                        );
                    } else {
                        // must be some kind of mux or a microcontroller connected to multiple buses
                        add_nodes.insert(bus_name.clone(), I2cNode::Device(d.clone()));
                    }
                    remove_nodes.insert(bus_name.clone(), I2cNode::Unknown(d.clone()));
                }
            }
        }
        for (bus_name, remove_node) in remove_nodes {
            let Some(bus) = self.by_name.get_mut(&bus_name) else {
                continue;
            };
            bus.nodes.retain(|n| n != &remove_node);
        }
        for (bus_name, add_node) in add_nodes {
            let Some(bus) = self.by_name.get_mut(&bus_name) else {
                continue;
            };
            bus.nodes.push(add_node);
        }
    }

    fn collect_segments(&mut self) {
        fn merge_or_push(sub_segment: HashSet<String>, segments: &mut Vec<HashSet<String>>) {
            let idx = segments
                .iter()
                .enumerate()
                .find(|(_, m)| m.intersection(&sub_segment).count() >= 1)
                .map(|(idx, _)| idx);
            if let Some(idx) = idx {
                segments[idx].extend(sub_segment);
            } else {
                segments.push(sub_segment);
            }
        }
        for bus in self.by_name.values() {
            let mut tie: HashSet<_> = [bus.derived_name.clone()].into();
            let mut translate: HashSet<_> = [bus.derived_name.clone()].into();
            for node in &bus.nodes {
                match node {
                    I2cNode::VoltageTranslator { other_side, .. }
                    | I2cNode::VoltageTranslatorDiscrete { other_side, .. } => {
                        translate.insert(other_side.to_string());
                    }
                    I2cNode::Tie { other_side, .. } => {
                        tie.insert(other_side.to_string());
                        translate.insert(other_side.to_string());
                    }
                    _ => {}
                }
            }
            merge_or_push(tie, &mut self.direct_segments);
            merge_or_push(translate, &mut self.same_bus_segments);
        }
    }

    fn check_pull_ups(&self, netlist: &Netlist, diagnostics: &mut Vec<I2cDiagnostic>) {
        for direct_segment in &self.direct_segments {
            let mut pull_up_count = 0;
            let mut pull_ups = HashSet::new();
            for bus_name in direct_segment {
                let Some(bus) = self.by_name.get(bus_name) else {
                    continue;
                };
                if let Some(pull_up) = &bus.pull_up {
                    pull_ups.insert(pull_up.scl.clone());
                    pull_ups.insert(pull_up.sda.clone());
                    if let Some(pull_up_net) = netlist.nets.get(&pull_up.v_net) {
                        let node_count = pull_up_net.nodes.iter().count();
                        if node_count == 2 {
                            diagnostics.push(I2cDiagnostic {
                                derived_name: direct_segment
                                    .iter()
                                    .next()
                                    .cloned()
                                    .unwrap_or_default(),
                                kind: I2cDiagnosticKind::PullUpToNowhere,
                            });
                        } else if node_count == 3 {
                            let mut parts = netlist.any_net_parts(&[&pull_up.v_net]);
                            parts.remove(&pull_up.scl);
                            parts.remove(&pull_up.sda);
                            if let Some(third_resistor) = parts.iter().find(|d| d.is_resistor()) {
                                let mut nets = netlist.part_nets(third_resistor);
                                nets.remove(&pull_up.v_net);
                                if let Some(should_be_power_net) = nets.iter().next() {
                                    if let Some(net) = netlist.nets.get(should_be_power_net) {
                                        if net.nodes.iter().count() == 1 {
                                            diagnostics.push(I2cDiagnostic {
                                                derived_name: direct_segment
                                                    .iter()
                                                    .next()
                                                    .cloned()
                                                    .unwrap_or_default(),
                                                kind: I2cDiagnosticKind::PullUpToNowhere,
                                            });
                                        } else {
                                            // TODO: check that there is at least on IO or power out pin in this net
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if bus.pull_up.is_some() {
                    pull_up_count += 1
                }
            }
            if pull_up_count == 0 {
                diagnostics.push(I2cDiagnostic {
                    derived_name: direct_segment.iter().next().cloned().unwrap_or_default(),
                    kind: I2cDiagnosticKind::NoPullUps,
                });
            } else if pull_up_count > 1 {
                diagnostics.push(I2cDiagnostic {
                    derived_name: direct_segment.iter().next().cloned().unwrap_or_default(),
                    kind: I2cDiagnosticKind::RedundantPullUps {
                        redundant_pull_ups: pull_ups,
                    },
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecad_file_format::kicad_netlist::load_kicad_netlist;
    use generate_netlists::get_netlist_path;

    #[test]
    fn able_to_recognize_i2c_bus_segments() {
        let path = get_netlist_path("i2c_segments");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        let buses = find_i2c_buses(&netlist, &mut diagnostics);
        // println!("buses: {buses:#?}");
        // println!("diagnostics: {diagnostics:#?}");
        // println!("ds: {:?}", buses.direct_segments);
        assert!(
            buses
                .direct_segments
                .contains(&["/I2C1_1V8".to_string()].into())
        );
        assert!(
            buses
                .direct_segments
                .contains(&["/I2C1_3V3_VCC2".to_string()].into())
        );
        assert!(
            buses.direct_segments.contains(
                &[
                    "/I2C1_3V3".to_string(),
                    "/I2C1_3V3_to_U408".to_string(),
                    "/I2C_U7".to_string()
                ]
                .into()
            )
        );
        assert!(
            buses
                .direct_segments
                .contains(&["/I2C1_1V2".to_string()].into())
        );
        assert!(
            buses
                .direct_segments
                .contains(&["/I2C2_3V3".to_string()].into())
        );
        assert!(
            buses
                .same_bus_segments
                .contains(&["/I2C2_3V3".to_string()].into())
        );
        assert!(
            buses.same_bus_segments.contains(
                &[
                    "/I2C_U7".to_string(),
                    "/I2C1_1V8".to_string(),
                    "/I2C1_3V3_VCC2".to_string(),
                    "/I2C1_3V3_to_U408".to_string(),
                    "/I2C1_3V3".to_string(),
                    "/I2C1_1V2".to_string()
                ]
                .into()
            )
        );
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C1_3V3_VCC2".to_string(),
            kind: I2cDiagnosticKind::NonStandardPullUps {
                resistance: Ohm(2200.0),
            },
        }));
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C1_3V3".to_string(),
            kind: I2cDiagnosticKind::NonStandardPullUps {
                resistance: Ohm(10_000.0),
            },
        }));
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C2_3V3".to_string(),
            kind: I2cDiagnosticKind::NonStandardPullUps {
                resistance: Ohm(10_000.0),
            },
        }));
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C1_3V3".to_string(),
            kind: I2cDiagnosticKind::TieTooHighValue {
                resistance: Ohm(1000.0),
                other_side: "/I2C1_3V3_to_U408".to_string(),
            },
        }));

        let expected = [
            Designator("R401".into()),
            Designator("R403".into()),
            Designator("R415".into()),
            Designator("R416".into()),
        ]
        .into();
        let mut found = false;
        for d in &diagnostics {
            if let I2cDiagnosticKind::RedundantPullUps { redundant_pull_ups } = &d.kind {
                assert_eq!(redundant_pull_ups, &expected);
                found = true;
            }
        }
        assert!(found);
    }

    #[test]
    fn able_to_find_missing_i2c_pull_ups() {
        let path = get_netlist_path("i2c_no_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        let _buses = find_i2c_buses(&netlist, &mut diagnostics);
        // println!("buses: {buses:#?}");
        // println!("diagnostics: {diagnostics:#?}");
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C1".to_string(),
            kind: I2cDiagnosticKind::NoPullUps,
        }));
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C2".to_string(),
            kind: I2cDiagnosticKind::PullUpToNowhere,
        }));
        assert!(diagnostics.contains(&I2cDiagnostic {
            derived_name: "/I2C3".to_string(),
            kind: I2cDiagnosticKind::PullUpToNowhere,
        }));
    }

    #[test]
    fn able_to_find_non_standard_pull_ups() {
        let path = get_netlist_path("i2c_non_standard_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        let _buses = find_i2c_buses(&netlist, &mut diagnostics);
        assert_eq!(
            diagnostics[0],
            I2cDiagnostic {
                derived_name: "/I2C".to_string(),
                kind: I2cDiagnosticKind::NonStandardPullUps {
                    resistance: Ohm(1000.0)
                },
            }
        );
    }

    #[test]
    fn able_to_find_non_equal_pull_ups() {
        let path = get_netlist_path("i2c_non_equal_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        let _buses = find_i2c_buses(&netlist, &mut diagnostics);
        assert_eq!(
            diagnostics[0],
            I2cDiagnostic {
                derived_name: "/I2C".to_string(),
                kind: I2cDiagnosticKind::NonEqualPullUps {
                    scl_resistance: Ohm(3000.0),
                    sda_resistance: Ohm(3300.0),
                }
            }
        );
    }

    #[test]
    fn able_to_find_multiple_i2c_pull_ups() {
        let path = get_netlist_path("i2c_multiple_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        let buses = find_i2c_buses(&netlist, &mut diagnostics);

        let i2c1_1v8_bus = buses.by_name.get("/I2C1_1V8").unwrap();
        let pull_up = i2c1_1v8_bus.pull_up.as_ref().unwrap();
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.derived_name == "/I2C1_1V8")
            .unwrap();
        let I2cDiagnosticKind::RedundantPullUps { redundant_pull_ups } = &diagnostic.kind else {
            panic!("Wrong diagnostic kind");
        };
        // Since there are multiple combinations and HashSet, it is non-deterministic which ones will be picked
        let mut expected_redundant_pull_ups = ["R507", "R508", "R509", "R510", "R511"]
            .map(|d| Designator(d.into()))
            .into_iter()
            .collect::<HashSet<_>>();
        expected_redundant_pull_ups.remove(&pull_up.scl);
        expected_redundant_pull_ups.remove(&pull_up.sda);
        assert_eq!(&expected_redundant_pull_ups, redundant_pull_ups);
    }
}
