//! `HS-4` Differential pair asymmetry

use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::graph::{net_pins, part_on_net};
use erc_analysis::interface::usb_data;
use erc_analysis::names::{PairStyle, pair_halves};
use erc_model::{NetId, PartId};
use std::collections::BTreeMap;

pub struct Hs4;

static META: CheckMeta = CheckMeta {
    id: "HS-4",
    category: "HS",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Hs4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The two lines of a pair by net name (`_P`/`_N`, `+`/`-`, `DP`/`DM`, `_H`/`_L`, glued `XP`/`XN`; rails and
    /// grounds left out): the fitted parts on one line and not on the other, compared by class, value and footprint.
    /// A resistor from a USB pair's D+ to a rail is the full-speed pull-up (`HS-6`) and left out.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let nets: Vec<NetId> = (0..b.nets.len())
            .map(NetId::from_index)
            .filter(|n| !power.is_power_net(*n) && !b.net(*n).name.is_empty())
            .collect();
        let names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
        let (pairs, _) = pair_halves(&names);
        let key = |part: PartId| {
            (
                cx.classes().get(part).name(),
                cx.values().get(part).text.clone(),
                cx.view().field(part, "Footprint").unwrap_or("").to_string(),
            )
        };
        let mut out = Vec::new();
        for pair in pairs {
            let (pos, neg) = (nets[pair.pos], nets[pair.neg]);
            let usb = pair.style == PairStyle::DpDm
                || pair.base.to_uppercase().contains("USB")
                || net_pins(b, pos).any(|p| usb_data(&b.pin(p).name).is_some());
            let only = |net: NetId, other: NetId| -> Vec<PartId> {
                let mut parts: Vec<PartId> = net_pins(b, net)
                    .map(|p| b.pin(p).part)
                    .filter(|p| cx.fitted(*p) && !part_on_net(b, *p, other))
                    .collect();
                parts.sort();
                parts.dedup();
                parts
            };
            let tally = |parts: &[PartId]| {
                let mut m: BTreeMap<_, Vec<PartId>> = BTreeMap::new();
                for p in parts {
                    m.entry(key(*p)).or_default().push(*p);
                }
                m
            };
            let mut on_pos = only(pos, neg);
            if usb {
                // The full-speed D+ pull-up
                on_pos.retain(|p| {
                    !(cx.classes().get(*p).is_resistor()
                        && b.part(*p)
                            .pins
                            .iter()
                            .any(|q| b.pin(*q).net.is_some_and(|n| power.is_rail(n))))
                });
            }
            let (tp, tn) = (tally(&on_pos), tally(&only(neg, pos)));
            let mut extra_pos = Vec::new();
            let mut extra_neg = Vec::new();
            for k in tp.keys().chain(tn.keys()) {
                let (a, c) = (
                    tp.get(k).map_or(&[][..], |v| v),
                    tn.get(k).map_or(&[][..], |v| v),
                );
                if a.len() > c.len() {
                    extra_pos.extend(&a[c.len()..]);
                } else if c.len() > a.len() {
                    extra_neg.extend(&c[a.len()..]);
                }
            }
            extra_pos.sort();
            extra_pos.dedup();
            extra_neg.sort();
            extra_neg.dedup();
            if extra_pos.is_empty() && extra_neg.is_empty() {
                continue;
            }
            let mut what = Vec::new();
            for (net, extra) in [(pos, &extra_pos), (neg, &extra_neg)] {
                if !extra.is_empty() {
                    what.push(format!(
                        "{} only on {}",
                        designators(cx, extra.iter().copied()),
                        b.net(net).name
                    ));
                }
            }
            out.push(
                cx.finding(&META)
                    .nets([pos, neg])
                    .parts(extra_pos.iter().chain(&extra_neg).copied())
                    .message(format!(
                        "differential pair {} / {} is not symmetric: {}: part of the differential signal turns into common mode",
                        b.net(pos).name,
                        b.net(neg).name,
                        what.join("; ")
                    ))
                    .fix("mirror the part on the other line, or a two-channel part (array, dual resistor) that treats both lines alike; test points as a pair or off the pair")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Hs4));

#[cfg(test)]
mod tests {
    #[test]
    fn asymmetric_pairs() {
        let found = crate::testing::fingerprints(&super::Hs4, "hs-4_pair_asymmetry");
        assert_eq!(
            found,
            [
                "HS-4:net /LVDS_N|net /LVDS_P|part TP1",
                "HS-4:net /USB_DM|net /USB_DP|part D1",
            ],
            "{found:?}"
        );
    }
}
