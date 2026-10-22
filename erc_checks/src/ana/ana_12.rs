//! `ANA-12` Comparator on a slow or noisy signal without hysteresis

use super::{between, connected, units};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::PartClass;

pub struct Ana12;

static META: CheckMeta = CheckMeta {
    id: "ANA-12",
    category: "ANA",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Ana12 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each comparator channel with its output and non-inverting input connected and no fitted resistor between
    /// them, on a part without a hysteresis pin (`HYST`). Which signals are slow or noisy is not in the netlist:
    /// every such comparator is listed.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for u in units(cx, PartClass::Comparator) {
            let (Some(o), Some(p)) = (connected(cx, u.out), connected(cx, u.inp)) else {
                continue;
            };
            let hyst_pin = b.part(u.part).pins.iter().any(|q| {
                b.pin(*q)
                    .name
                    .to_uppercase()
                    .trim_start_matches("~{")
                    .starts_with("HYST")
            });
            if hyst_pin || !between(cx, o, p, &|c| c.is_resistor()).is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(u.part)
                    .pins(u.out.into_iter().chain(u.inp))
                    .message(format!(
                        "comparator {}: no resistor from the output to the non-inverting input; on a slow or noisy input it chatters at each crossing",
                        u.label(b)
                    ))
                    .fix("a positive feedback resistor for a few mV of hysteresis (SBOA313), or a comparator with built-in hysteresis")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Ana12));

#[cfg(test)]
mod tests {
    #[test]
    fn no_hysteresis() {
        let found = crate::testing::fingerprints(&super::Ana12, "ana-12_comparator_hysteresis");
        assert_eq!(found, ["ANA-12:part U1|pin U1.1|pin U1.3"], "{found:?}");
    }
}
