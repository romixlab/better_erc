//! `TEST-2` Several test points on one net (incl. through 0R), unless top and bottom

use super::test_points;
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::access::zero_ohm_groups;
use erc_model::layout::Side;
use erc_model::{NetId, PartId, Provenance};
use std::collections::BTreeMap;

pub struct Test2;

/// Test points by (net group, board side), with the nets they sit on
type BySide = BTreeMap<(NetId, Option<Side>), (Vec<PartId>, Vec<NetId>)>;

static META: CheckMeta = CheckMeta {
    id: "TEST-2",
    category: "TEST",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Test2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Test points grouped by net, nets joined by 0 Ω resistors, net ties and jumpers counted as one, and by
    /// board side from the layout; two or more in one group on one side are a finding. Without a layout the
    /// sides are not known and all count as one (an estimate).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let groups = zero_ohm_groups(cx.view(), cx.classes(), cx.values());
        let side = |tp: PartId| -> Option<Side> {
            b.layout
                .as_ref()?
                .footprints
                .iter()
                .find(|f| f.part == Some(tp))
                .map(|f| f.side)
        };
        let mut by = BySide::new();
        for tp in test_points(cx) {
            let nets: Vec<NetId> = b
                .part(tp)
                .pins
                .iter()
                .filter_map(|p| b.pin(*p).net)
                .collect();
            let Some(net) = nets.first().copied() else {
                continue;
            };
            let root = groups.get(&net).copied().unwrap_or(net);
            let e = by.entry((root, side(tp))).or_default();
            e.0.push(tp);
            if !e.1.contains(&net) {
                e.1.push(net);
            }
        }
        let mut out = Vec::new();
        for ((_, side), (tps, nets)) in by {
            if tps.len() < 2 {
                continue;
            }
            let names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
            let where_ = match side {
                Some(Side::Front) => " on the top",
                Some(Side::Back) => " on the bottom",
                None => "",
            };
            let mut f = cx
                .finding(&META)
                .parts(tps.iter().copied())
                .nets(nets.iter().copied())
                .message(format!(
                    "{} test points {}{where_} on {}",
                    tps.len(),
                    designators(cx, tps.iter().copied()),
                    crate::text::and_list(&names)
                ));
            if side.is_none() {
                f = f.fact(
                    "side",
                    "not known",
                    Provenance::Assumed {
                        rule: "no layout: the test points counted as on one side".into(),
                    },
                );
            }
            out.push(
                f.fix("keep one per side; move the spare to a net that has none (TEST-4)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Test2));

#[cfg(test)]
mod tests {
    #[test]
    fn duplicate_test_points() {
        let found = crate::testing::fingerprints(&super::Test2, "test-2_duplicate_test_points");
        assert_eq!(
            found,
            [
                "TEST-2:net +1V8|part TP1|part TP2",
                "TEST-2:net +3V3|net +3V3_S|part TP4|part TP5"
            ],
            "{found:?}"
        );
    }
}
