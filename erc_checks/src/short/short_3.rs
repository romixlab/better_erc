//! `SHORT-3` Non-DNM part shunting another mounted part

use super::is_link;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::part_nets;
use erc_model::{NetId, PartId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Short3;

static META: CheckMeta = CheckMeta {
    id: "SHORT-3",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Variant],
    overlaps: &[],
    params: &[],
};

impl Check for Short3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// In the variant checked: a fitted 0 Ω, jumper, net tie or switch in parallel with another fitted part (the
    /// option collapses), and two fitted links from one net to two different rails (both selector options fitted)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let values = cx.values();
        let link = |p: PartId| is_link(classes.get(p), values.get(p).value);
        let mut by_pair: BTreeMap<(NetId, NetId), Vec<PartId>> = BTreeMap::new();
        let mut links_on: BTreeMap<NetId, Vec<(PartId, NetId)>> = BTreeMap::new();
        for p in cx.view().fitted_components() {
            let nets: Vec<NetId> = part_nets(b, p).into_iter().collect();
            let [a, c] = nets[..] else { continue };
            by_pair.entry((a, c)).or_default().push(p);
            if link(p) {
                links_on.entry(a).or_default().push((p, c));
                links_on.entry(c).or_default().push((p, a));
            }
        }
        let mut out = Vec::new();
        for ((a, c), parts) in &by_pair {
            if parts.len() < 2 {
                continue;
            }
            let Some(l) = parts.iter().copied().find(|p| link(*p)) else {
                continue;
            };
            let others: Vec<&str> = parts
                .iter()
                .filter(|p| **p != l)
                .map(|p| b.part(*p).designator.as_str())
                .collect();
            out.push(
                cx.finding(&META)
                    .parts(parts.iter().copied())
                    .nets([*a, *c])
                    .message(format!(
                        "{} ({}) is fitted across {}, which it shorts out ({} to {})",
                        b.part(l).designator,
                        link_name(cx, l),
                        others.join(", "),
                        b.net(*a).name,
                        b.net(*c).name
                    ))
                    .fix("mark the option DNP in every variant that does not use it, and make BOMs from the variants")
                    .build(),
            );
        }
        let power = cx.power();
        let mut seen = BTreeSet::new();
        for (common, links) in &links_on {
            let to_rails: Vec<&(PartId, NetId)> =
                links.iter().filter(|(_, n)| power.is_rail(*n)).collect();
            for (i, (p, r)) in to_rails.iter().enumerate() {
                for (q, s) in &to_rails[i + 1..] {
                    if r == s || !seen.insert((*p, *q)) {
                        continue;
                    }
                    out.push(
                        cx.finding(&META)
                            .parts([*p, *q])
                            .nets([*common, *r, *s])
                            .message(format!(
                                "{} and {} are both fitted and join {} and {} through {}",
                                b.part(*p).designator,
                                b.part(*q).designator,
                                b.net(*r).name,
                                b.net(*s).name,
                                b.net(*common).name
                            ))
                            .fix("fit one of the two selector options per variant, mark the other DNP")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

fn link_name(cx: &Ctx, p: PartId) -> String {
    let c = cx.classes().get(p);
    if c.is_resistor() {
        "0 Ω".into()
    } else {
        c.name().into()
    }
}

inventory::submit!(Registered(&Short3));

#[cfg(test)]
mod tests {
    #[test]
    fn parallel_options() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short3, "short-3_fitted_options"),
            [
                "SHORT-3:net +3V3|net +5V|net /SEL|part R5|part R6",
                "SHORT-3:net /FILT_IN|net /FILT_OUT|part R1|part R2",
            ]
        );
    }
}
