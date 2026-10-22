//! `DRV-5` MOSFET inverter: gate resistor against Cgd coupling

use super::{DRAIN, GATE, SOURCE, logic_drivers, pin_named};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::PartClass;
use erc_model::class::Channel;

pub struct Drv5;

static META: CheckMeta = CheckMeta {
    id: "DRV-5",
    category: "DRV",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Drv5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted N-channel MOSFET with its source on a ground and a resistor from its drain to a rail (an
    /// inverter with a pull-up) whose gate is on the same net as an IC's output, bidirectional or tri-state pin:
    /// no series resistor slows the gate edge
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for q in cx.view().fitted_components() {
            if cx.classes().get(q)
                != (PartClass::Mosfet {
                    channel: Some(Channel::N),
                })
            {
                continue;
            }
            let (Some((_, g)), Some((_, d)), Some((_, s))) = (
                pin_named(cx, q, GATE),
                pin_named(cx, q, DRAIN),
                pin_named(cx, q, SOURCE),
            ) else {
                continue;
            };
            if !power.is_ground(s) || power.is_power_net(d) || power.is_power_net(g) {
                continue;
            }
            let pulled_up = net_pins(b, d).any(|p| {
                let r = b.pin(p).part;
                cx.fitted(r)
                    && cx.classes().get(r).is_resistor()
                    && b.part(r)
                        .pins
                        .iter()
                        .filter_map(|x| b.pin(*x).net)
                        .any(|n| power.is_rail(n))
            });
            let drivers = logic_drivers(cx, g);
            if !pulled_up || drivers.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(q)
                    .net(g)
                    .message(format!(
                        "{} inverts with a pull-up on {}, and its gate {} is driven straight by {}: the gate edge couples through Cgd and drives the drain below ground",
                        b.part(q).designator,
                        b.net(d).name,
                        b.net(g).name,
                        pin_list(cx, &drivers)
                    ))
                    .fix("a gate series resistor (100 Ω to 1 kΩ), a stronger pull-up, or a logic inverter")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Drv5));

#[cfg(test)]
mod tests {
    #[test]
    fn inverter_without_gate_resistor() {
        let found = crate::testing::fingerprints(&super::Drv5, "drv-5_inverter_gate_resistor");
        assert_eq!(found, ["DRV-5:net /RST_MCU|part Q1"], "{found:?}");
    }
}
