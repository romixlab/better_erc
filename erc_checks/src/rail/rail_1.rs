//! `RAIL-1` Power input pins of parts not powered

use super::{MAX_FEED, designators, fed, ground_named, one_part, pin_list};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::power_in_on;
use erc_model::{NetId, NetKind, PinId};
use std::collections::BTreeMap;

pub struct Rail1;

static META: CheckMeta = CheckMeta {
    id: "RAIL-1",
    category: "RAIL",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[MAX_FEED],
};

impl Check for Rail1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Power-input pins of fitted parts (ground-named ones left out) on a net that joins nothing else, on a net
    /// that is no rail, or on a rail no source reaches (`RAIL-5` says that one from the rail's side)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let fed = fed(cx, &META);
        let mut by_net: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            if power.is_ground(net) {
                continue;
            }
            let pins: Vec<PinId> = power_in_on(cx.view(), net)
                .into_iter()
                .filter(|p| !ground_named(cx, *p))
                .collect();
            if !pins.is_empty() {
                by_net.insert(net, pins);
            }
        }
        let mut out = Vec::new();
        for (net, pins) in by_net {
            let name = &b.net(net).name;
            let parts = designators(cx, pins.iter().map(|p| b.pin(*p).part));
            let f = cx.finding(&META).net(net);
            let found = if let Some(part) = one_part(cx, net) {
                let what = if b.net(net).kind == NetKind::Unconnected {
                    "connected to nothing".to_string()
                } else {
                    format!("on {name}, which joins nothing else")
                };
                f.part(part)
                    .message(format!(
                        "{} {} {what}: the part is not powered",
                        if pins.len() == 1 { "power pin" } else { "power pins" },
                        pin_list(cx, &pins)
                    ))
                    .fix("wire the pin to its rail; place the power unit of a multi-unit symbol; make hidden power pins visible in the library")
            } else if !power.is_rail(net) {
                f.parts(pins.iter().map(|p| b.pin(*p).part))
                    .message(format!(
                        "power pins {} are on {name}, which is a signal, not a rail",
                        pin_list(cx, &pins)
                    ))
                    .fix("wire the pins to their rail, or rename the net if it is one")
            } else if !fed.contains_key(&net) {
                f.message(format!(
                    "{name} feeds power pins of {parts}, but no source reaches it"
                ))
                .fix("add the missing link (a ferrite, 0 Ω or regulator) from a sourced rail")
            } else {
                continue;
            };
            out.push(found.build());
        }
        out
    }
}

inventory::submit!(Registered(&Rail1));

#[cfg(test)]
mod tests {
    #[test]
    fn unpowered_pins() {
        let found = crate::testing::fingerprints(&super::Rail1, "rail-1_unpowered_pins");
        assert_eq!(
            found,
            [
                "RAIL-1:net +3V3_OPA|part U4",
                "RAIL-1:net +5V_ISO",
                "RAIL-1:net /VDD_EN|part U3",
                "RAIL-1:net unconnected-(U2-VCC-Pad1)|part U2",
            ],
            "{found:?}"
        );
    }
}
