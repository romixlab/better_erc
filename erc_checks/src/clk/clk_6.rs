//! `CLK-6` Suggest a cap on buttons to reset inputs; note hw debounce in the programmer report

use super::reset_nets;
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::access::series_neighbours;
use erc_analysis::graph::net_pins;
use erc_analysis::supply::caps_to_ground;

pub struct Clk6;

static META: CheckMeta = CheckMeta {
    id: "CLK-6",
    category: "CLK",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Clk6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A reset net (reset inputs by name) with a fitted switch on it or one series resistor away, and no fitted
    /// capacitor to a ground: no hardware debounce (the programmer report, RPT-13, will say so)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, pins) in reset_nets(cx) {
            let mut nets = vec![net];
            nets.extend(
                series_neighbours(cx.view(), cx.classes(), cx.power(), net)
                    .into_iter()
                    .map(|(_, n)| n),
            );
            let buttons: Vec<_> = nets
                .iter()
                .flat_map(|n| net_pins(b, *n))
                .map(|p| b.pin(p).part)
                .filter(|p| cx.fitted(*p) && cx.classes().get(*p).is_switch())
                .collect();
            if buttons.is_empty()
                || !caps_to_ground(cx.view(), cx.classes(), cx.power(), net).is_empty()
            {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {}: reset button {} with no capacitor; each bounce is a reset (hw debounce: none)",
                        pin_list(cx, &pins),
                        b.net(net).name,
                        designators(cx, buttons)
                    ))
                    .fix("100 nF to ground on the reset net (with the pull-up, a time constant in the ms range)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Clk6));

#[cfg(test)]
mod tests {
    #[test]
    fn reset_button_without_capacitor() {
        let found = crate::testing::fingerprints(&super::Clk6, "clk-6_reset_buttons");
        assert_eq!(
            found,
            ["CLK-6:net /NRST|pin U1.3", "CLK-6:net /PHY_RESET|pin U3.3"],
            "{found:?}"
        );
    }
}
