//! `IO-6` High impedance nets acting as touch buttons

use super::undriven_inputs;
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::access::reach;
use erc_analysis::graph::net_pins;

pub struct Io6;

static META: CheckMeta = CheckMeta {
    id: "IO-6",
    category: "IO",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &["IO-1"],
    params: &[],
};

impl Check for Io6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An IO-1 net (inputs nothing drives) that a finger, cable or probe reaches: a test point or connector pin on
    /// it or one series part away, or a switch on it
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, pins) in undriven_inputs(cx) {
            let mut touch: Vec<_> = reach(cx.view(), cx.classes(), cx.power(), net)
                .into_iter()
                .map(|r| b.pin(r.pin).part)
                .collect();
            touch.extend(
                net_pins(b, net)
                    .map(|p| b.pin(p).part)
                    .filter(|p| cx.fitted(*p) && cx.classes().get(*p).is_switch()),
            );
            if touch.is_empty() {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {} float and {} can touch the net: a touch sensor, it flips with a finger or ESD",
                        pin_list(cx, &pins),
                        b.net(net).name,
                        designators(cx, touch)
                    ))
                    .fix("a pull (10-100 kΩ) on the board side, and ESD protection where it leaves the enclosure")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Io6));

#[cfg(test)]
mod tests {
    #[test]
    fn touchable_inputs() {
        let found = crate::testing::fingerprints(&super::Io6, "io-6_touch_inputs");
        assert_eq!(
            found,
            [
                "IO-6:net /TOUCH_FAR|pin U1.5",
                "IO-6:net /TOUCH_TP|pin U1.3"
            ],
            "{found:?}"
        );
    }
}
