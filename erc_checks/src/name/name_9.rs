//! `NAME-9` IO numbering out of order against the part's pins

use super::named_nets;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::{gpio, numbered};
use erc_model::{NetId, PartClass, PartId};
use std::collections::BTreeMap;

pub struct Name9;

/// A numbered family on one part's port: (family, part, port)
type FamilyKey = (String, PartId, String);

/// Its members: net number -> (net, GPIO number)
type Members = BTreeMap<u32, (NetId, u32)>;

static META: CheckMeta = CheckMeta {
    id: "NAME-9",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[Param {
        name: "min_family",
        default: 3.0,
        unit: "",
        doc: "the fewest nets of one numbered family on one part's GPIOs that are compared",
    }],
};

impl Check for Name9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Numbered net families (`IO0`..`IO7`) on the GPIOs of one IC (pin names `GPIO17`, `PA5`, `P0.12`, one
    /// port): the GPIO numbers taken in the order of the net numbers rise (or fall) except for a few out of place,
    /// which are the finding. A family in no order at all is left alone (a choice, not a slip).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let min = cx.param(&META, "min_family") as usize;
        let power = cx.power();
        let mut families: BTreeMap<FamilyKey, Members> = BTreeMap::new();
        for net in named_nets(cx) {
            if power.is_power_net(net) {
                continue;
            }
            let name = &b.net(net).name;
            let Some(num) = numbered(name) else { continue };
            for q in net_pins(b, net) {
                let pin = b.pin(q);
                let class = classes.get(pin.part);
                if !(class.is_ic() || class == PartClass::Unknown) {
                    continue;
                }
                if let Some(g) = gpio(&pin.name) {
                    families
                        .entry((num.family(name), pin.part, g.port))
                        .or_default()
                        .insert(num.index, (net, g.number));
                }
            }
        }
        let mut out = Vec::new();
        for ((_, part, _), members) in families {
            if members.len() < min {
                continue;
            }
            let order: Vec<(NetId, u32)> = members.values().copied().collect();
            let mut rising: Vec<u32> = order.iter().map(|(_, g)| *g).collect();
            rising.sort_unstable();
            let falling: Vec<u32> = rising.iter().rev().copied().collect();
            let off = |sorted: &[u32]| -> Vec<NetId> {
                order
                    .iter()
                    .zip(sorted)
                    .filter(|((_, g), s)| g != *s)
                    .map(|((n, _), _)| *n)
                    .collect()
            };
            let (up, down) = (off(&rising), off(&falling));
            let wrong = if up.len() <= down.len() { up } else { down };
            if wrong.is_empty() || wrong.len() * 2 > order.len() {
                continue;
            }
            let shown: Vec<String> = wrong
                .iter()
                .map(|n| {
                    let g = order.iter().find(|(m, _)| m == n).map_or(0, |(_, g)| *g);
                    format!("{} on GPIO {g}", b.net(*n).name)
                })
                .collect();
            out.push(
                cx.finding(&META)
                    .part(part)
                    .nets(wrong.iter().copied())
                    .message(format!(
                        "{}: {} out of order against the GPIO numbers of the rest of the family",
                        b.part(part).designator,
                        shown.join(", ")
                    ))
                    .fix("swap the nets at the part (or rename them) so a block of pins stays consecutive")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Name9));

#[cfg(test)]
mod tests {
    #[test]
    fn swapped_ios() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name9, "name-9_io_order"),
            ["NAME-9:net /IO4|net /IO5|part U1"]
        );
    }
}
