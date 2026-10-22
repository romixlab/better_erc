//! `DBG-4` No power rail LEDs

use super::{led_ends, leds};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Dbg4;

static META: CheckMeta = CheckMeta {
    id: "DBG-4",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every rail with no fitted LED between it and a ground, directly or through a resistor at either end
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let ends: Vec<_> = leds(cx).into_iter().map(|l| led_ends(cx, l)).collect();
        let mut out = Vec::new();
        for rail in power.rails.keys() {
            let shown = ends.iter().any(|e| {
                e.len() == 2
                    && ((e[0].contains(rail) && e[1].iter().any(|n| power.is_ground(*n)))
                        || (e[1].contains(rail) && e[0].iter().any(|n| power.is_ground(*n))))
            });
            if shown {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(*rail)
                    .message(format!("no LED shows {}", b.net(*rail).name))
                    .fix("an LED with its resistor from the rail to ground (DNP on battery boards); the input rail and switched rails first")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dbg4));

#[cfg(test)]
mod tests {
    #[test]
    fn rails_without_leds() {
        let found = crate::testing::fingerprints(&super::Dbg4, "dbg-4_power_leds");
        assert_eq!(found, ["DBG-4:net +5V"], "{found:?}");
    }
}
