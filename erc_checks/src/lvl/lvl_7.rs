//! `LVL-7` Several power rails on a part and no banks defined

use crate::meta::*;
use crate::rail::ground_named;
use crate::{Ctx, Finding};
use erc_analysis::feed::power_in_on;
use erc_model::{NetId, PartClass};
use std::collections::BTreeSet;

pub struct Lvl7;

static META: CheckMeta = CheckMeta {
    id: "LVL-7",
    category: "LVL",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Lvl7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted part (not a regulator or load switch, whose rails are input and output) with power-input pins on
    /// two or more rails. No library or part data gives pin banks yet (NET-7), so every such part is reported:
    /// the level checks cannot tell which supply its signal pins follow.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            if class.is_regulator() || class == PartClass::LoadSwitch || class.is_connector() {
                continue;
            }
            let rails: BTreeSet<NetId> = b
                .part(part)
                .pins
                .iter()
                .filter(|p| !ground_named(cx, **p))
                .filter_map(|p| {
                    let net = b.pin(*p).net?;
                    (power.is_rail(net) && power_in_on(cx.view(), net).contains(p)).then_some(net)
                })
                .collect();
            if rails.len() < 2 {
                continue;
            }
            let names: Vec<&str> = rails.iter().map(|n| b.net(*n).name.as_str()).collect();
            out.push(
                cx.finding(&META)
                    .part(part)
                    .nets(rails.iter().copied())
                    .message(format!(
                        "{} is supplied from {} rails ({}) and nothing says which supply its signal pins follow",
                        b.part(part).designator,
                        rails.len(),
                        names.join(", ")
                    ))
                    .fix("add bank data to the library part (a pin property, a unit per bank, or a pin file via IMP-10)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Lvl7));

#[cfg(test)]
mod tests {
    #[test]
    fn parts_on_several_rails() {
        let found = crate::testing::fingerprints(&super::Lvl7, "lvl-7_several_rails");
        assert_eq!(
            found,
            [
                "LVL-7:net +1V8|net +3V3|part U2",
                "LVL-7:net +3V3|net +3V3_A|part U1"
            ],
            "{found:?}"
        );
    }
}
