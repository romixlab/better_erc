//! `ANA-22` Mid-rail bias divider without a capacitor to ground

use super::units;
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::access::series_neighbours;
use erc_analysis::graph::net_pins;
use erc_analysis::pull::pulls;
use erc_analysis::supply::caps_to_ground;
use erc_model::{NetId, PartClass, PinId};
use std::collections::BTreeMap;

pub struct Ana22;

static META: CheckMeta = CheckMeta {
    id: "ANA-22",
    category: "ANA",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Ana22 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net with a fitted resistor to a rail and another to a ground that feeds an op-amp input directly or
    /// through one series resistor, with no fitted capacitor to a ground on it (or on the net behind that series
    /// resistor)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let inputs: BTreeMap<NetId, Vec<PinId>> = units(cx, PartClass::Opamp)
            .into_iter()
            .flat_map(|u| [u.inp, u.inn])
            .flatten()
            .filter_map(|p| b.pin(p).net.map(|n| (n, p)))
            .fold(BTreeMap::new(), |mut m, (n, p)| {
                m.entry(n).or_insert_with(Vec::new).push(p);
                m
            });
        let mut out = Vec::new();
        for net in (0..b.nets.len()).map(NetId::from_index) {
            if power.is_power_net(net) || net_pins(b, net).count() < 3 {
                continue;
            }
            let p = pulls(cx.view(), cx.classes(), cx.values(), power, net);
            if p.up.is_empty() || p.down.is_empty() {
                continue;
            }
            let mut fed = vec![net];
            fed.extend(
                series_neighbours(cx.view(), cx.classes(), power, net)
                    .into_iter()
                    .map(|(_, n)| n),
            );
            let fed: Vec<NetId> = fed.into_iter().filter(|n| inputs.contains_key(n)).collect();
            if fed.is_empty() {
                continue;
            }
            let filtered = std::iter::once(net)
                .chain(fed.iter().copied())
                .any(|n| !caps_to_ground(cx.view(), cx.classes(), power, n).is_empty());
            if filtered {
                continue;
            }
            let amps: Vec<_> = fed
                .iter()
                .flat_map(|n| &inputs[n])
                .map(|p| b.pin(*p).part)
                .collect();
            out.push(
                cx.finding(&META)
                    .net(net)
                    .parts(p.parts())
                    .message(format!(
                        "bias divider {} on {} feeds {} with no capacitor to ground: the supply's noise rides on the bias",
                        designators(cx, p.parts()),
                        b.net(net).name,
                        designators(cx, amps)
                    ))
                    .fix("a capacitor from the midpoint to ground (corner well below the signal band), or a buffered reference")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Ana22));

#[cfg(test)]
mod tests {
    #[test]
    fn unfiltered_bias() {
        let found = crate::testing::fingerprints(&super::Ana22, "ana-22_bias_divider_caps");
        assert_eq!(found, ["ANA-22:net /BIAS|part R1|part R2"], "{found:?}");
    }
}
