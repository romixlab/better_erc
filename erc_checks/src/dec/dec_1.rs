//! `DEC-1` No decoupling capacitors on power nets

use super::{caps, designators};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::supply::power_in_by_part;

pub struct Dec1;

static META: CheckMeta = CheckMeta {
    id: "DEC-1",
    category: "DEC",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Dec1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A rail with power-input pins of fitted parts and no fitted capacitor from it to a ground
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for net in cx.power().rails.keys() {
            let loads = power_in_by_part(cx.view(), *net);
            if loads.is_empty() || !caps(cx, *net).is_empty() {
                continue;
            }
            let pins: usize = loads.values().map(Vec::len).sum();
            out.push(
                cx.finding(&META)
                    .net(*net)
                    .message(format!(
                        "{} feeds {pins} power pin{} of {} and has no capacitor to a ground",
                        b.net(*net).name,
                        if pins == 1 { "" } else { "s" },
                        designators(cx, loads.keys().copied())
                    ))
                    .fix("a 100 nF ceramic from the rail to ground at each power pin, plus bulk on the rail")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dec1));

#[cfg(test)]
mod tests {
    #[test]
    fn rail_without_capacitors() {
        let found = crate::testing::fingerprints(&super::Dec1, "dec-1_no_decoupling");
        assert_eq!(found, ["DEC-1:net +3V3_IO"], "{found:?}");
    }
}
