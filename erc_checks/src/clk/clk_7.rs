//! `CLK-7` No capacitor on a reset_n input: noise and brown-outs reach it

use super::active_low_reset_nets;
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::supply::caps_to_ground;

pub struct Clk7;

static META: CheckMeta = CheckMeta {
    id: "CLK-7",
    category: "CLK",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &["CLK-6"],
    params: &[],
};

impl Check for Clk7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An active-low reset net (reset inputs by name, [`super::is_active_low`]) with no fitted capacitor to a
    /// ground
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, pins) in active_low_reset_nets(cx) {
            if !caps_to_ground(cx.view(), cx.classes(), cx.power(), net).is_empty() {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {}: an active-low reset with no capacitor to ground; noise, ESD and slow supply ramps reach it unfiltered",
                        pin_list(cx, &pins),
                        b.net(net).name
                    ))
                    .fix("10-100 nF to ground (1 µF where a power-up delay is wanted), within what the drivers can pull low")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Clk7));

#[cfg(test)]
mod tests {
    #[test]
    fn reset_without_capacitor() {
        let found = crate::testing::fingerprints(&super::Clk7, "clk-7_reset_capacitors");
        assert_eq!(
            found,
            ["CLK-7:net /NRST|pin U1.3", "CLK-7:net /PHY_RST|pin U2.3"],
            "{found:?}"
        );
    }
}
