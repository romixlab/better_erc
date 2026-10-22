//! `RAIL-18` Two regulator outputs tied together

use super::designators;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::{LinkKind, SourceKind};
use erc_model::class::RegulatorKind;
use erc_model::{NetId, PartClass, PartId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Rail18;

static META: CheckMeta = CheckMeta {
    id: "RAIL-18",
    category: "RAIL",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Rail18 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A rail driven by the outputs of two or more fitted regulators: their output pins on it, or (a buck or a
    /// converter of unknown kind) the far side of the inductor on their switching node. No part data says which
    /// parts are made for paralleling yet, so every such rail is reported.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let feeds = cx.feeds();
        let mut by_rail: BTreeMap<NetId, BTreeSet<PartId>> = BTreeMap::new();
        for (net, sources) in &feeds.sources {
            for s in sources.iter().filter(|s| s.kind == SourceKind::Regulator) {
                if power.is_rail(*net) {
                    by_rail.entry(*net).or_default().insert(s.part);
                } else if power.switching_nodes.contains(net)
                    && !matches!(
                        cx.classes().get(s.part),
                        PartClass::Regulator {
                            kind: Some(RegulatorKind::Boost | RegulatorKind::BuckBoost)
                        }
                    )
                {
                    for l in feeds
                        .links_at(*net)
                        .filter(|l| l.kind == LinkKind::Inductor)
                    {
                        let far = if l.a == *net { l.b } else { l.a };
                        if power.is_rail(far) {
                            by_rail.entry(far).or_default().insert(s.part);
                        }
                    }
                }
            }
        }
        by_rail
            .into_iter()
            .filter(|(_, parts)| parts.len() > 1)
            .map(|(rail, parts)| {
                cx.finding(&META)
                    .net(rail)
                    .parts(parts.iter().copied())
                    .message(format!(
                        "the outputs of {} all drive {}: parallel regulators fight, the higher one takes the load",
                        designators(cx, parts.iter().copied()),
                        b.net(rail).name
                    ))
                    .fix("one regulator for the current needed, a part made for paralleling with ballast, or ORing diodes when two sources are meant (PIN-12)")
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&Rail18));

#[cfg(test)]
mod tests {
    #[test]
    fn parallel_regulators() {
        let found = crate::testing::fingerprints(&super::Rail18, "rail-18_parallel_regulators");
        assert_eq!(
            found,
            [
                "RAIL-18:net +1V8|part U3|part U4",
                "RAIL-18:net +3V3|part U1|part U2"
            ],
            "{found:?}"
        );
    }
}
