//! `DBG-4` No power rail LEDs

use super::{led_ends, leds};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::power::PowerSignal;
use erc_model::{NetId, PartClass};
use std::collections::BTreeSet;

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

    /// Every rail with no fitted LED between it and a ground, directly or through a resistor at either end; an IC's
    /// regulator filter pin (a rail by its name alone, with only that IC and capacitors on it) is left out
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
            if shown || filter_pin(cx, *rail) {
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

/// A rail by its name alone whose fitted parts are one IC and capacitors: the IC's internal regulator brought out
/// for its filter capacitor (`CRFILT`, `PLLFILT`, `VCAP`), which an LED would load
fn filter_pin(cx: &Ctx, rail: NetId) -> bool {
    let b = cx.board();
    let view = cx.view();
    let classes = cx.classes();
    let by_name_only = cx.power().rails[&rail]
        .signals
        .iter()
        .all(|s| s.value == PowerSignal::Name);
    let others: BTreeSet<_> = net_pins(b, rail)
        .map(|p| b.pin(p).part)
        .filter(|p| view.fitted(*p) && !matches!(classes.get(*p), PartClass::Capacitor { .. }))
        .collect();
    by_name_only && others.len() == 1 && others.iter().all(|p| classes.get(*p).is_ic())
}

inventory::submit!(Registered(&Dbg4));

#[cfg(test)]
mod tests {
    #[test]
    fn rails_without_leds() {
        let found = crate::testing::fingerprints(&super::Dbg4, "dbg-4_power_leds");
        assert_eq!(found, ["DBG-4:net +5V"], "{found:?}");
    }

    #[test]
    fn filter_pins_and_sense_nets_need_no_led() {
        let found = crate::testing::fingerprints(&super::Dbg4, "short-6_sense_nets");
        assert_eq!(
            found,
            ["DBG-4:net +3V3", "DBG-4:net /+1V8", "DBG-4:net /+5V_S"],
            "{found:?}"
        );
    }
}
