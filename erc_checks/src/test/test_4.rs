//! `TEST-4` Power rail or ground without a test point

use super::test_points;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::access::zero_ohm_groups;
use erc_model::NetId;
use std::collections::BTreeSet;

pub struct Test4;

static META: CheckMeta = CheckMeta {
    id: "TEST-4",
    category: "TEST",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Test4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every rail and ground with no test point on it or on a net joined to it by a 0 Ω resistor, net tie or
    /// jumper
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let groups = zero_ohm_groups(cx.view(), cx.classes(), cx.values());
        let root = |n: NetId| groups.get(&n).copied().unwrap_or(n);
        let probed: BTreeSet<NetId> = test_points(cx)
            .into_iter()
            .flat_map(|tp| b.part(tp).pins.clone())
            .filter_map(|p| b.pin(p).net)
            .map(root)
            .collect();
        let mut out = Vec::new();
        for net in power.rails.keys().chain(power.grounds.keys()) {
            if probed.contains(&root(*net)) {
                continue;
            }
            let what = if power.is_ground(*net) {
                "ground"
            } else {
                "rail"
            };
            out.push(
                cx.finding(&META)
                    .net(*net)
                    .message(format!("{what} {} has no test point", b.net(*net).name))
                    .fix("a test point per rail and at least one ground pad (or a ground loop) per board area")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Test4));

#[cfg(test)]
mod tests {
    #[test]
    fn rails_without_test_points() {
        let found = crate::testing::fingerprints(&super::Test4, "test-4_rail_test_points");
        assert_eq!(found, ["TEST-4:net +5V", "TEST-4:net GNDA"], "{found:?}");
    }
}
