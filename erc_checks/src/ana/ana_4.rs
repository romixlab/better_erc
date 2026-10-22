//! `ANA-4` Current sense resistor straight to ground or power: suggest net ties

use super::{sense_pairs, shunts};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};

pub struct Ana4;

static META: CheckMeta = CheckMeta {
    id: "ANA-4",
    category: "ANA",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[Param {
        name: "max_shunt",
        default: 1.0,
        unit: "ohm",
        doc: "a resistor below this value is a current shunt",
    }],
};

impl Check for Ana4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A shunt (a fitted resistor below `max_shunt`) with an end on a rail or ground, where a current-sense
    /// amplifier's sense input sits on that same net: the netlist cannot keep the Kelvin connection
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let max = cx.param(&META, "max_shunt");
        let pairs = sense_pairs(cx);
        let mut out = Vec::new();
        for (shunt, x, y) in shunts(cx, max) {
            for end in [x, y] {
                if !power.is_power_net(end) {
                    continue;
                }
                let sense: Vec<_> = pairs
                    .iter()
                    .flat_map(|p| [p.pos, p.neg])
                    .filter(|p| b.pin(*p).net == Some(end))
                    .collect();
                if sense.is_empty() {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .part(shunt)
                        .net(end)
                        .message(format!(
                            "shunt {} ends on {} and {} senses on {} too: any copper of that net is in the measurement",
                            b.part(shunt).designator,
                            b.net(end).name,
                            pin_list(cx, &sense),
                            b.net(end).name
                        ))
                        .fact("max_shunt", max, cx.param_source(&META, "max_shunt"))
                        .fix("a sense net of its own (ISENSE_N) joined to the power net by a net tie at the shunt pad, routed as a pair")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Ana4));

#[cfg(test)]
mod tests {
    #[test]
    fn shunt_on_ground() {
        let found = crate::testing::fingerprints(&super::Ana4, "ana-4_shunt_kelvin");
        assert_eq!(found, ["ANA-4:net GND|part R1"], "{found:?}");
    }
}
