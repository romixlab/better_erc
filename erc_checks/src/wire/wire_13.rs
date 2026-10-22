//! `WIRE-13` Output pin whose net reaches no input

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::{net_pins, part_nets};
use erc_model::{EcadPinType, NetId, PartClass, PinId};

pub struct Wire13;

static META: CheckMeta = CheckMeta {
    id: "WIRE-13",
    category: "WIRE",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[Param {
        name: "max_hops",
        default: 3.0,
        unit: "",
        doc: "how many series parts (R, C, L, ferrite, jumper) the search for a load looks through",
    }],
};

/// A pin that drives a net
fn output(t: Option<&EcadPinType>) -> bool {
    matches!(
        t,
        Some(
            EcadPinType::Output
                | EcadPinType::TriState
                | EcadPinType::OpenCollector
                | EcadPinType::OpenEmitter
                | EcadPinType::PowerOut
        )
    )
}

/// Parts a signal passes through in series
fn series(c: PartClass) -> bool {
    matches!(
        c,
        PartClass::Resistor
            | PartClass::Capacitor { .. }
            | PartClass::Inductor
            | PartClass::FerriteBead
            | PartClass::Jumper
            | PartClass::NetTie
    )
}

impl Check for Wire13 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Signal nets driven by an output pin of a fitted part where no other pin takes the signal: only passives
    /// to a rail, test points or other outputs, looking through series parts (`max_hops`). An IC pin of any
    /// type but output, a connector, a diode or a transistor is a load. A pin alone on its net is an unused
    /// output, not this check's (WIRE-8).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let power = cx.power();
        let hops = cx.param(&META, "max_hops") as usize;
        let mut out = Vec::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            if power.is_power_net(net) {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, net)
                .filter(|q| cx.fitted(b.pin(*q).part))
                .collect();
            let drivers: Vec<PinId> = pins
                .iter()
                .copied()
                .filter(|q| output(b.pin(*q).etype.get()))
                .collect();
            if drivers.is_empty() || pins.len() < 2 {
                continue;
            }
            // Breadth first over nets, through series parts
            let mut seen = vec![net];
            let mut front = vec![net];
            let mut load = false;
            for depth in 0..=hops {
                let mut next = Vec::new();
                for n in &front {
                    for q in net_pins(b, *n) {
                        let pin = b.pin(q);
                        if drivers.contains(&q) || !cx.fitted(pin.part) {
                            continue;
                        }
                        let class = classes.get(pin.part);
                        if series(class) {
                            let pn = part_nets(b, pin.part);
                            if depth < hops && pn.len() == 2 {
                                for m in pn {
                                    if !seen.contains(&m) && !power.is_power_net(m) {
                                        seen.push(m);
                                        next.push(m);
                                    }
                                }
                            }
                            continue;
                        }
                        let inert = matches!(
                            class,
                            PartClass::TestPoint | PartClass::Fiducial | PartClass::MountingHole
                        );
                        if !inert && !output(pin.etype.get()) {
                            load = true;
                        }
                    }
                }
                if load || next.is_empty() {
                    break;
                }
                front = next;
            }
            if load {
                continue;
            }
            let names: Vec<String> = drivers
                .iter()
                .map(|q| format!("{} ({})", b.pin_name(*q), b.pin(*q).name))
                .collect();
            let mut f = cx.finding(&META).net(net);
            for q in &drivers {
                f = f.pin(*q);
            }
            out.push(
                f.message(format!(
                    "{}: driven by {} and taken by no input, only passives, test points or other outputs",
                    b.net(net).name,
                    names.join(", ")
                ))
                .fix("connect the consumer, or note why the output is spare (a test point if it should be measurable)")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Wire13));

#[cfg(test)]
mod tests {
    #[test]
    fn outputs_into_nothing() {
        assert_eq!(
            crate::testing::fingerprints(&super::Wire13, "wire-13_output_to_nowhere"),
            ["WIRE-13:net /QH_CHK|pin U1.9"]
        );
    }
}
