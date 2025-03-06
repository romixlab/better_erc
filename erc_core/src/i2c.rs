use crate::util::collapse_underscores;
use ecad_file_format::netlist::Netlist;
use ecad_file_format::passive_value::Ohm;
use ecad_file_format::{Designator, DesignatorStartsWith, NetName};
use std::collections::HashSet;

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

#[derive(Debug, PartialEq)]
pub enum I2cDiagnosticKind {
    RedundantPullUps {
        redundant_pull_ups: HashSet<Designator>,
    },
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
}

#[derive(Debug, PartialEq)]
pub struct I2cDiagnostic {
    pub derived_name: String,
    pub kind: I2cDiagnosticKind,
}

fn find_i2c_buses(netlist: &Netlist, diagnostics: &mut Vec<I2cDiagnostic>) -> Vec<I2cBus> {
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
        let mut pull_up_chains = netlist.find_chains(
            scl_net,
            &[DesignatorStartsWith("R"), DesignatorStartsWith("R")],
            &sda_net,
        );
        let mut connected_parts = netlist.any_net_parts(&[scl_net, &sda_net]);
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
        buses.push(I2cBus {
            derived_name,
            scl_net: scl_net.clone(),
            sda_net,
            pull_up,
            nodes: parts_to_nodes(netlist, connected_parts),
        });
    }
    look_for_non_standard_pull_ups(netlist, &buses, diagnostics);
    buses
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
        netlist.find_chains(scl_net, &[DesignatorStartsWith("R")], &pull_up.v_net);
    redundant_chains.extend(netlist.find_chains(
        &sda_net,
        &[DesignatorStartsWith("R")],
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
    if value.0 <= 2200.0 || value.0 >= 10_000.0 {
        diagnostics.push(I2cDiagnostic {
            derived_name: bus_name.to_string(),
            kind: I2cDiagnosticKind::NonStandardPullUps {
                resistance: value.clone(),
            },
        })
    }
}

fn parts_to_nodes(_netlist: &Netlist, parts: HashSet<Designator>) -> Vec<I2cNode> {
    // TODO: implement I2C parts to nodes
    let mut nodes = vec![];
    for part in parts {
        nodes.push(I2cNode::Device(part));
    }
    nodes
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
        println!("{:#?}", netlist);
        // let mut diagnostics = Vec::new();
        // let buses = find_i2c_buses(&netlist, &mut diagnostics);
        // println!("buses: {buses:#?}");
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
        // TODO: merge buses through 0R or low R before this can work
    }

    #[test]
    fn able_to_find_non_standard_pull_ups() {
        let path = get_netlist_path("i2c_non_standard_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        let _buses = find_i2c_buses(&netlist, &mut diagnostics);
        println!("{:#?}", diagnostics);
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

        let i2c1_1v8_bus = buses
            .iter()
            .find(|b| b.derived_name == "/I2C1_1V8")
            .unwrap();
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

        // println!("diagnostics: {:?}", diagnostics);
    }
}
