//! `DRV-7` No snubber on a half-bridge output

use super::{DRAIN, SOURCE, designators, pin_named};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::{net_pins, series_paths};
use erc_model::{NetId, PartClass, PartId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Drv7;

static META: CheckMeta = CheckMeta {
    id: "DRV-7",
    category: "DRV",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Drv7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Switching nodes: a converter's (an IC and an inductor, NET-6) and a discrete half bridge's midpoint (an
    /// N-channel low side with its source on a ground, a second MOSFET from the node to a rail); each must have a
    /// resistor and a capacitor in series from it to a ground or a rail, fitted or as a DNP footprint
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut nodes: BTreeMap<NetId, BTreeSet<PartId>> = BTreeMap::new();
        for n in &power.switching_nodes {
            let parts = net_pins(b, *n)
                .map(|p| b.pin(p).part)
                .filter(|p| cx.classes().get(*p).is_ic());
            nodes.entry(*n).or_default().extend(parts);
        }
        let fets: Vec<(PartId, NetId, NetId)> = cx
            .view()
            .fitted_components()
            .filter(|q| cx.classes().get(*q).is_mosfet())
            .filter_map(|q| Some((q, pin_named(cx, q, DRAIN)?.1, pin_named(cx, q, SOURCE)?.1)))
            .collect();
        for &(low, d, s) in &fets {
            let n_channel = matches!(
                cx.classes().get(low),
                PartClass::Mosfet {
                    channel: Some(erc_model::class::Channel::N)
                }
            );
            if !n_channel || !power.is_ground(s) || power.is_power_net(d) {
                continue;
            }
            for &(high, hd, hs) in &fets {
                let other = if hs == d {
                    hd
                } else if hd == d {
                    hs
                } else {
                    continue;
                };
                if high != low && power.is_rail(other) {
                    nodes.entry(d).or_default().extend([low, high]);
                }
            }
        }
        let rc = |p: PartId| {
            let c = cx.classes().get(p);
            c.is_resistor() || c.is_capacitor()
        };
        let mut out = Vec::new();
        for (node, parts) in nodes {
            let snubbed = series_paths(b, node, &rc, &|n| !power.is_power_net(n), 2)
                .into_iter()
                .any(|path| {
                    power.is_power_net(path.end)
                        && path.parts.len() == 2
                        && path
                            .parts
                            .iter()
                            .any(|p| cx.classes().get(*p).is_resistor())
                        && path
                            .parts
                            .iter()
                            .any(|p| cx.classes().get(*p).is_capacitor())
                });
            if snubbed {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(node)
                    .parts(parts.iter().copied())
                    .message(format!(
                        "switching node {} of {} has no RC snubber to ground or a rail, not even a footprint",
                        b.net(node).name,
                        designators(cx, parts.iter().copied())
                    ))
                    .fix("place a series RC footprint from the node to ground (across the low side); size it from the measured ring frequency (AN11160)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Drv7));

#[cfg(test)]
mod tests {
    #[test]
    fn switching_nodes_without_snubber() {
        let found = crate::testing::fingerprints(&super::Drv7, "drv-7_half_bridge_snubber");
        assert_eq!(
            found,
            [
                "DRV-7:net /PHASE_A|part Q1|part Q2",
                "DRV-7:net /SW1|part U1"
            ],
            "{found:?}"
        );
    }
}
