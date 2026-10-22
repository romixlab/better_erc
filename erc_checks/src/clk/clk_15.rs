//! `CLK-15` One reset net shared by the MCU and its peripherals

use super::reset_nets;
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};

pub struct Clk15;

static META: CheckMeta = CheckMeta {
    id: "CLK-15",
    category: "CLK",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Clk15 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net with the reset input of an MCU, SoC or FPGA and the reset inputs of other parts
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, pins) in reset_nets(cx) {
            let parts = |controller: bool| -> Vec<_> {
                pins.iter()
                    .map(|p| b.pin(*p).part)
                    .filter(|p| cx.classes().get(*p).is_programmable() == controller)
                    .collect()
            };
            let (controllers, peripherals) = (parts(true), parts(false));
            if controllers.is_empty() || (peripherals.is_empty() && controllers.len() < 2) {
                continue;
            }
            let peripherals = if peripherals.is_empty() {
                controllers[1..].to_vec()
            } else {
                peripherals
            };
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} shares its reset {} with {}: every debugger, watchdog or software reset resets them too",
                        designators(cx, controllers.iter().take(1).copied()),
                        b.net(net).name,
                        designators(cx, peripherals)
                    ))
                    .fix("a reset of their own for the peripherals, from a GPIO with a pull (IO-13) or the supervisor; NRST for the MCU and the debugger")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Clk15));

#[cfg(test)]
mod tests {
    #[test]
    fn shared_reset() {
        let found = crate::testing::fingerprints(&super::Clk15, "clk-15_shared_reset");
        assert_eq!(found, ["CLK-15:net /NRST|pin U1.3|pin U2.3"], "{found:?}");
    }
}
