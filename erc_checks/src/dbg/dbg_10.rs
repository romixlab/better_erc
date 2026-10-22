//! `DBG-10` Many things on one power net: suggest splitting with a 0R before each power input

use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::supply::power_in_by_part;

pub struct Dbg10;

static META: CheckMeta = CheckMeta {
    id: "DBG-10",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[Param {
        name: "max_parts",
        default: 5.0,
        unit: "",
        doc: "the most parts a rail feeds directly before a branch footprint is suggested",
    }],
};

impl Check for Dbg10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A rail with power-input pins of more than `max_parts` fitted parts on it directly: no 0 Ω, ferrite or
    /// jumper splits it into branches
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let max = cx.param(&META, "max_parts");
        let mut out = Vec::new();
        for rail in cx.power().rails.keys() {
            let loads = power_in_by_part(cx.view(), *rail);
            if (loads.len() as f64) <= max {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(*rail)
                    .message(format!(
                        "{} feeds {} parts directly ({}): no branch can be isolated or measured",
                        b.net(*rail).name,
                        loads.len(),
                        designators(cx, loads.keys().copied())
                    ))
                    .fact("max_parts", max, cx.param_source(&META, "max_parts"))
                    .fix("a 0 Ω (or ferrite, or solder jumper) at the start of each block's branch")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dbg10));

#[cfg(test)]
mod tests {
    #[test]
    fn crowded_rail() {
        let found = crate::testing::fingerprints(&super::Dbg10, "dbg-10_crowded_rail");
        assert_eq!(found, ["DBG-10:net +3V3"], "{found:?}");
    }
}
