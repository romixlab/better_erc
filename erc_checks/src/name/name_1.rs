//! `NAME-1` Inconsistent naming of inverted signals

use super::named_nets;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::names::{LowStyle, active_low, pair_halves};
use erc_model::NetId;
use std::collections::{BTreeMap, BTreeSet};

pub struct Name1;

static META: CheckMeta = CheckMeta {
    id: "NAME-1",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

/// The active-low nets of a board by marker style: named signal nets, not the `_N` half of a differential pair
pub(crate) fn low_nets(cx: &Ctx) -> BTreeMap<LowStyle, Vec<NetId>> {
    let b = cx.board();
    let power = cx.power();
    let nets: Vec<NetId> = named_nets(cx).filter(|n| !power.is_power_net(*n)).collect();
    let names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
    let (pairs, _) = pair_halves(&names);
    let in_pair: BTreeSet<usize> = pairs.iter().flat_map(|p| [p.pos, p.neg]).collect();
    let mut by_style: BTreeMap<LowStyle, Vec<NetId>> = BTreeMap::new();
    for (i, n) in nets.iter().enumerate() {
        if in_pair.contains(&i) {
            continue;
        }
        if let Some(a) = active_low(names[i]) {
            by_style.entry(a.style).or_default().push(*n);
        }
    }
    by_style
}

impl Check for Name1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// More than one active-low marker style among the net names: one finding on the nets outside the most used
    /// style, listing every style with examples
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let by_style = low_nets(cx);
        if by_style.len() < 2 {
            return Vec::new();
        }
        let (main, _) = by_style
            .iter()
            .max_by_key(|(s, nets)| (nets.len(), std::cmp::Reverse(**s)))
            .expect("two styles");
        let listed: Vec<String> = by_style
            .iter()
            .map(|(s, nets)| {
                let ex: Vec<&str> = nets
                    .iter()
                    .take(3)
                    .map(|n| b.net(*n).name.as_str())
                    .collect();
                format!("{} ({}: {})", s.example(), nets.len(), ex.join(", "))
            })
            .collect();
        let others = by_style
            .iter()
            .filter(|(s, _)| *s != main)
            .flat_map(|(_, nets)| nets.iter().copied());
        vec![
            cx.finding(&META)
                .nets(others)
                .message(format!(
                    "active-low signals are marked in {} styles: {}",
                    by_style.len(),
                    listed.join("; ")
                ))
                .fix(format!(
                    "one style for the project, {} as most nets use it (`~{{...}}` renders as an overbar, `_N` survives every tool)",
                    main.example()
                ))
                .build(),
        ]
    }
}

inventory::submit!(Registered(&Name1));

#[cfg(test)]
mod tests {
    #[test]
    fn mixed_styles() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name1, "name-1_inverted_signal_styles"),
            ["NAME-1:net /nPWR_FAULT|net /~{HUB_RESET}"]
        );
    }
}
