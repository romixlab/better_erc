//! `CLK-14` Reset input driven by a push-pull output: fights the debugger, supervisor or button that also drives it

use super::{push_pull_outputs, reset_nets};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::EcadPinType;

pub struct Clk14;

static META: CheckMeta = CheckMeta {
    id: "CLK-14",
    category: "CLK",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Clk14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A reset net (reset inputs by name) with a push-pull output (pin type `output`) and any other driver: a
    /// second output, a bidirectional or open-drain pin (an MCU's own NRST, a supervisor), a connector pin (a
    /// debugger) or a switch
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, _) in reset_nets(cx) {
            let push_pull = push_pull_outputs(cx, net);
            if push_pull.is_empty() {
                continue;
            }
            let others: Vec<_> = net_pins(b, net)
                .filter(|p| !push_pull.contains(p))
                .filter(|p| {
                    let pin = b.pin(*p);
                    let class = cx.classes().get(pin.part);
                    let fitted = cx.fitted(pin.part);
                    (fitted
                        && matches!(
                            pin.etype.get(),
                            Some(
                                EcadPinType::Bidirectional
                                    | EcadPinType::TriState
                                    | EcadPinType::OpenCollector
                                    | EcadPinType::OpenEmitter
                            )
                        ))
                        || class.is_connector()
                        || (fitted && class.is_switch())
                })
                .collect();
            if others.is_empty() && push_pull.len() < 2 {
                continue;
            }
            let with = if others.is_empty() {
                "each other".to_string()
            } else {
                pin_list(cx, &others)
            };
            out.push(
                push_pull
                    .iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} drives the reset net {} push-pull, against {with}: a reset line is wired-OR, open-drain sources and one pull-up",
                        pin_list(cx, &push_pull),
                        b.net(net).name
                    ))
                    .fix("an open-drain source (a supervisor with open-drain output, a GPIO in open-drain mode), or a series resistor where push-pull cannot be avoided")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Clk14));

#[cfg(test)]
mod tests {
    #[test]
    fn push_pull_on_reset() {
        let found = crate::testing::fingerprints(&super::Clk14, "clk-14_push_pull_reset");
        assert_eq!(found, ["CLK-14:net /NRST|pin U2.1"], "{found:?}");
    }
}
