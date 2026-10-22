//! `NAME-2` Inconsistent naming of differential pairs

use super::named_nets;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::names::{Half, PairStyle, pair_halves};
use erc_model::NetId;
use std::collections::BTreeMap;

pub struct Name2;

static META: CheckMeta = CheckMeta {
    id: "NAME-2",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Name2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Differential pairs among the signal nets in more than one style (one finding on the pairs outside the
    /// most used style), and two halves without a partner that belong together but are written in two styles
    /// (`USB_DP` with `USB_D-`)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let nets: Vec<NetId> = named_nets(cx).filter(|n| !power.is_power_net(*n)).collect();
        let names: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
        let (pairs, lone) = pair_halves(&names);
        let mut out = Vec::new();
        let mut by_style: BTreeMap<PairStyle, Vec<(usize, usize)>> = BTreeMap::new();
        for p in &pairs {
            by_style.entry(p.style).or_default().push((p.pos, p.neg));
        }
        if by_style.len() > 1 {
            let (main, _) = by_style
                .iter()
                .max_by_key(|(s, v)| (v.len(), std::cmp::Reverse(**s)))
                .expect("two styles");
            let listed: Vec<String> = by_style
                .iter()
                .map(|(s, v)| {
                    let ex: Vec<String> = v
                        .iter()
                        .take(2)
                        .map(|(a, c)| format!("{}/{}", names[*a], names[*c]))
                        .collect();
                    format!("{} ({}: {})", s.example(), v.len(), ex.join(", "))
                })
                .collect();
            let others = by_style
                .iter()
                .filter(|(s, _)| *s != main)
                .flat_map(|(_, v)| v.iter().flat_map(|(a, c)| [nets[*a], nets[*c]]));
            out.push(
                cx.finding(&META)
                    .nets(others)
                    .message(format!(
                        "differential pairs are named in {} styles: {}",
                        by_style.len(),
                        listed.join("; ")
                    ))
                    .fix(format!(
                        "one style the layout tool pairs by (`_P`/`_N` or `+`/`-` in KiCad), {} as most pairs use it",
                        main.example()
                    ))
                    .build(),
            );
        }
        for (i, (a, ha)) in lone.iter().enumerate() {
            for (c, hc) in &lone[i + 1..] {
                if ha.half == hc.half || ha.style == hc.style || ha.key() != hc.key() {
                    continue;
                }
                let (pos, neg) = if ha.half == Half::Pos { (a, c) } else { (c, a) };
                out.push(
                    cx.finding(&META)
                        .nets([nets[*pos], nets[*neg]])
                        .message(format!(
                            "{} and {} look like the two halves of one pair in two styles: a layout tool does not pair them",
                            names[*pos], names[*neg]
                        ))
                        .fix("name both halves in one style")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Name2));

#[cfg(test)]
mod tests {
    #[test]
    fn pair_styles() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name2, "name-2_diff_pair_styles"),
            [
                "NAME-2:net /CAN+|net /CAN-",
                "NAME-2:net /USB_D-|net /USB_DP",
            ]
        );
    }
}
