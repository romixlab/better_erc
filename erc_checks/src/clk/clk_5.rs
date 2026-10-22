//! `CLK-5` No pull-up on a reset_n input

use super::{active_low_reset_nets, push_pull_outputs};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::pull::pulls;

pub struct Clk5;

static META: CheckMeta = CheckMeta {
    id: "CLK-5",
    category: "CLK",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Clk5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An active-low reset net (reset inputs by name, [`super::is_active_low`]) with no fitted pull-up and no
    /// push-pull output holding it. Internal pull-ups (STM32 NRST, about 40 kΩ) are part data (NET-7, CFG-5) not
    /// read yet: the message names them, the severity stays.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, pins) in active_low_reset_nets(cx) {
            let p = pulls(cx.view(), cx.classes(), cx.values(), cx.power(), net);
            if !p.up.is_empty() || !push_pull_outputs(cx, net).is_empty() {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {}: an active-low reset with no pull-up; only an internal pull (if the part has one) holds it out of reset",
                        pin_list(cx, &pins),
                        b.net(net).name
                    ))
                    .fix("10 kΩ to the part's IO rail, with a capacitor to ground (CLK-7)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Clk5));

#[cfg(test)]
mod tests {
    #[test]
    fn reset_without_pull_up() {
        let found = crate::testing::fingerprints(&super::Clk5, "clk-5_reset_pull_ups");
        assert_eq!(
            found,
            ["CLK-5:net /ESP_EN|pin U3.3", "CLK-5:net /NRST|pin U1.3"],
            "{found:?}"
        );
    }
}
