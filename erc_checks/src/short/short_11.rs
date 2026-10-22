//! `SHORT-11` Two push-pull outputs on one net

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PinId};
use std::collections::BTreeSet;

pub struct Short11;

static META: CheckMeta = CheckMeta {
    id: "SHORT-11",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Variant],
    overlaps: &[],
    params: &[],
};

impl Check for Short11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Nets with two or more pins of fitted parts typed `output` (push-pull: not tri-state, open collector or
    /// open emitter); a series resistor puts them on two nets, so it breaks the count by itself
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            let mut seen = BTreeSet::new();
            let outputs: Vec<PinId> = net_pins(b, net)
                .filter(|q| {
                    let pin = b.pin(*q);
                    cx.fitted(pin.part)
                        && pin.etype.get() == Some(&EcadPinType::Output)
                        && seen.insert((pin.part, pin.number.clone()))
                })
                .collect();
            if outputs.len() < 2 {
                continue;
            }
            let names: Vec<String> = outputs
                .iter()
                .map(|q| format!("{} ({})", b.pin_name(*q), b.pin(*q).name))
                .collect();
            let mut f = cx.finding(&META).net(net);
            for q in &outputs {
                f = f.pin(*q).part(b.pin(*q).part);
            }
            out.push(
                f.message(format!(
                    "{}: push-pull outputs {} drive one net",
                    b.net(net).name,
                    names.join(", ")
                ))
                .fix("make one open-drain or tri-state, combine through a gate, or separate them with a series resistor; an assembly option is DNP")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Short11));

#[cfg(test)]
mod tests {
    #[test]
    fn output_conflict() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short11, "short-11_output_conflict"),
            ["SHORT-11:net /REF_CLK|part U1|part X1|pin U1.1|pin X1.3"]
        );
    }
}
