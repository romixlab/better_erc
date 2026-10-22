//! `ANA-3` Shunt lines to the amplifier without a filter

use super::{between, sense_pairs, shunts, with_series};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::analog::{AmpPin, amp_pin};
use erc_analysis::supply::caps_to_ground;
use erc_analysis::value::format_si;
use erc_model::Unit;

pub struct Ana3;

static META: CheckMeta = CheckMeta {
    id: "ANA-3",
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

impl Check for Ana3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A shunt (a fitted resistor below `max_shunt`) whose two ends reach the two sense inputs of a current-sense
    /// amplifier (pin names `IN+`/`IN-`, `VIN+`/`VIN-`, `SENSE+`/`-`, `CSP`/`CSN`), directly or through one
    /// series resistor each, with no capacitor across the inputs and none to ground at the amplifier's output or
    /// one series resistor after it
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let max = cx.param(&META, "max_shunt");
        let mut out = Vec::new();
        for (shunt, x, y) in shunts(cx, max) {
            for pair in sense_pairs(cx) {
                let (Some(p), Some(n)) = (b.pin(pair.pos).net, b.pin(pair.neg).net) else {
                    continue;
                };
                let (ps, ns) = (with_series(cx, p, shunt), with_series(cx, n, shunt));
                let on_shunt =
                    (ps.contains(&x) && ns.contains(&y)) || (ps.contains(&y) && ns.contains(&x));
                if !on_shunt || p == n {
                    continue;
                }
                if !between(cx, p, n, &|c| c.is_capacitor()).is_empty() {
                    continue;
                }
                let output_rc = b.part(pair.part).pins.iter().any(|q| {
                    let pin = b.pin(*q);
                    amp_pin(&pin.name, pin.etype.get().copied())
                        .is_some_and(|(r, _)| r == AmpPin::Out)
                        && pin.net.is_some_and(|o| {
                            with_series(cx, o, shunt).into_iter().any(|m| {
                                !cx.power().is_power_net(m)
                                    && !caps_to_ground(cx.view(), cx.classes(), cx.power(), m)
                                        .is_empty()
                            })
                        })
                });
                if output_rc {
                    continue;
                }
                let value = cx
                    .values()
                    .get(shunt)
                    .value
                    .map(|v| format_si(v, Unit::Ohm))
                    .unwrap_or_default();
                out.push(
                    cx.finding(&META)
                        .part(shunt)
                        .part(pair.part)
                        .message(format!(
                            "shunt {} ({value}) into {} with no filter: no capacitor across the sense inputs and no RC at the output; ripple and edges reach the ADC",
                            b.part(shunt).designator,
                            b.part(pair.part).designator
                        ))
                        .fact("max_shunt", max, cx.param_source(&META, "max_shunt"))
                        .fix("an RC on the output into the ADC; or ≤ 10 Ω in each sense line and a capacitor across the inputs")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Ana3));

#[cfg(test)]
mod tests {
    #[test]
    fn unfiltered_shunt() {
        let found = crate::testing::fingerprints(&super::Ana3, "ana-3_shunt_filters");
        assert_eq!(found, ["ANA-3:part R1|part U1"], "{found:?}");
    }
}
