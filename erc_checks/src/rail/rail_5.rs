//! `RAIL-5` Power net with loads but no source

use super::{MAX_FEED, designators, fed, ground_named, one_part};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::feed::power_in_on;
use erc_analysis::supply::caps_to_ground;

pub struct Rail5;

static META: CheckMeta = CheckMeta {
    id: "RAIL-5",
    category: "RAIL",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &["RAIL-1"],
    params: &[MAX_FEED],
};

impl Check for Rail5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A rail with power-input pins of two or more parts (or of one part and a capacitor) that no source reaches:
    /// no regulator or load-switch output, power-output pin, connector or battery on it, nor behind a ferrite,
    /// inductor, fuse, diode, switch, jumper, net tie, transistor or a resistor of at most `max_feed`
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let fed = fed(cx, &META);
        let mut out = Vec::new();
        for net in cx.power().rails.keys() {
            if fed.contains_key(net) {
                continue;
            }
            let loads: Vec<_> = power_in_on(cx.view(), *net)
                .into_iter()
                .filter(|p| !ground_named(cx, *p))
                .collect();
            if loads.is_empty() || one_part(cx, *net).is_some() {
                continue;
            }
            let caps = caps_to_ground(cx.view(), cx.classes(), cx.power(), *net);
            let with_caps = if caps.is_empty() {
                String::new()
            } else {
                format!(" and {}", designators(cx, caps))
            };
            out.push(
                cx.finding(&META)
                    .net(*net)
                    .message(format!(
                        "{} has loads ({}{with_caps}) but no source: no regulator, connector, switch or series part brings power to it",
                        b.net(*net).name,
                        designators(cx, loads.iter().map(|p| b.pin(*p).part)),
                    ))
                    .fix("add the missing link (a ferrite or 0 Ω from the rail meant to feed it); put a test point on every rail")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Rail5));

#[cfg(test)]
mod tests {
    #[test]
    fn rail_without_source() {
        let found = crate::testing::fingerprints(&super::Rail5, "rail-5_unsourced_rail");
        assert_eq!(
            found,
            ["RAIL-5:net +3V3_WEAK", "RAIL-5:net +5V0_FLT"],
            "{found:?}"
        );
    }
}
