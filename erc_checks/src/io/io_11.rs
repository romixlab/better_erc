//! `IO-11` MOSFET gate without a pull: floats while its driver is in reset or unpowered

use super::net_pulls;
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::access::series_neighbours;
use erc_analysis::graph::net_pins;
use erc_model::class::Channel;
use erc_model::{EcadPinType, NetId, PartClass, PartId, PinId};

pub struct Io11;

static META: CheckMeta = CheckMeta {
    id: "IO-11",
    category: "IO",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[],
};

/// The nets of a part's pins named one of `names`
fn named_nets(cx: &Ctx, part: PartId, names: &[&str]) -> Vec<NetId> {
    let b = cx.board();
    b.part(part)
        .pins
        .iter()
        .map(|p| b.pin(*p))
        .filter(|p| names.iter().any(|n| p.name.eq_ignore_ascii_case(n)))
        .filter_map(|p| p.net)
        .collect()
}

impl Check for Io11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each fitted MOSFET with pins named G, S, D (LIB-3): the gate net and the nets one series resistor away
    /// need a fitted resistor to the source net or to the off level (a ground for N-channel, a rail at the
    /// source's voltage for P-channel), unless a push-pull output of an IC that is no MCU (a gate driver, a logic
    /// gate) drives them: an MCU GPIO, a connector or an open-drain output is high-Z at times
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for fet in cx.view().fitted_components() {
            let class = cx.classes().get(fet);
            if !class.is_mosfet() {
                continue;
            }
            let gates = named_nets(cx, fet, &["G", "GATE"]);
            let sources = named_nets(cx, fet, &["S", "SOURCE"]);
            let ([gate], [source, ..]) = (gates.as_slice(), sources.as_slice()) else {
                continue;
            };
            let (gate, source) = (*gate, *source);
            if gate == source || power.is_power_net(gate) {
                continue;
            }
            let mut group = vec![gate];
            group.extend(
                series_neighbours(cx.view(), cx.classes(), power, gate)
                    .into_iter()
                    .map(|(_, n)| n),
            );
            let channel = match class {
                PartClass::Mosfet { channel } => channel,
                _ => None,
            };
            let off = |to: NetId| {
                to == source
                    || match channel {
                        Some(Channel::N) => power.is_ground(to) && power.is_ground(source),
                        Some(Channel::P) => power.is_rail(to) && super::same_level(cx, to, source),
                        None => false,
                    }
            };
            let mut pulls_on: Vec<PartId> = Vec::new();
            let mut pulled_off = false;
            for net in &group {
                let p = net_pulls(cx, *net);
                for r in p.up.iter().chain(&p.down) {
                    if off(r.to) {
                        pulled_off = true;
                    } else {
                        pulls_on.push(r.part);
                    }
                }
                // A resistor straight from the gate group to the source net when the source is no rail
                for pin in net_pins(b, *net) {
                    let r = b.pin(pin).part;
                    if cx.fitted(r)
                        && cx.classes().get(r).is_resistor()
                        && b.part(r).pins.len() == 2
                        && b.part(r)
                            .pins
                            .iter()
                            .any(|q| *q != pin && b.pin(*q).net == Some(source))
                    {
                        pulled_off = true;
                    }
                }
            }
            if pulled_off {
                continue;
            }
            let push_pull: Vec<PinId> = group
                .iter()
                .flat_map(|n| net_pins(b, *n))
                .filter(|p| {
                    let pin = b.pin(*p);
                    let c = cx.classes().get(pin.part);
                    cx.fitted(pin.part)
                        && pin.part != fet
                        && pin.etype.get() == Some(&EcadPinType::Output)
                        && c.is_ic()
                        && !c.is_programmable()
                })
                .collect();
            if !push_pull.is_empty() {
                continue;
            }
            let gate_pin: Vec<PinId> = b
                .part(fet)
                .pins
                .iter()
                .copied()
                .filter(|p| b.pin(*p).net == Some(gate))
                .collect();
            let how = if pulls_on.is_empty() {
                "no resistor to its source".to_string()
            } else {
                format!(
                    "only {} towards on, no resistor to its source",
                    designators(cx, pulls_on)
                )
            };
            out.push(
                cx.finding(&META)
                    .part(fet)
                    .net(gate)
                    .message(format!(
                        "{} gate {} on {}: {how}; it floats while its driver is in reset, high-Z or unpowered",
                        b.part(fet).designator,
                        pin_list(cx, &gate_pin),
                        b.net(gate).name
                    ))
                    .fix("10-100 kΩ from gate to source (a pull-down for N-channel, a pull-up to the source rail for P-channel)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Io11));

#[cfg(test)]
mod tests {
    #[test]
    fn gates_without_pulls() {
        let found = crate::testing::fingerprints(&super::Io11, "io-11_gate_pulls");
        assert_eq!(
            found,
            [
                "IO-11:net /GATE1|part Q1",
                "IO-11:net /GATE6|part Q6",
                "IO-11:net /GATE7|part Q7"
            ],
            "{found:?}"
        );
    }
}
