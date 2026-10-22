//! `DBG-8` Suggest DNM pull-ups on open drain pins; test point detector should highlight those nets

use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::{net_degree, net_pins};
use erc_analysis::pin_name::{PinFunction, functions};
use erc_model::{EcadPinType, NetId, PinId};
use std::collections::BTreeMap;

pub struct Dbg8;

static META: CheckMeta = CheckMeta {
    id: "DBG-8",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net driven by an open-drain output (pin type open collector, or an IC's `INT`, `ALERT`, `PG`,
    /// `FAULT` by name) with no resistor, fitted or not, to a rail; I2C lines are I2C-1's
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut nets: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            for pin in &b.part(part).pins {
                let p = b.pin(*pin);
                let fns = functions(&p.name);
                if fns
                    .iter()
                    .any(|f| matches!(f.function, PinFunction::I2cScl | PinFunction::I2cSda))
                {
                    continue;
                }
                let open = p.etype.get() == Some(&EcadPinType::OpenCollector)
                    || (class.is_ic()
                        && !class.is_programmable()
                        && fns.iter().any(|f| f.function.is_flag()));
                let Some(net) = p.net else { continue };
                if open && !power.is_power_net(net) && net_degree(b, net) >= 2 {
                    nets.entry(net).or_default().push(*pin);
                }
            }
        }
        let mut out = Vec::new();
        for (net, pins) in nets {
            let pulled = net_pins(b, net).any(|pin| {
                let r = b.pin(pin).part;
                cx.classes().get(r).is_resistor()
                    && b.part(r)
                        .pins
                        .iter()
                        .filter(|p| **p != pin)
                        .filter_map(|p| b.pin(*p).net)
                        .any(|n| power.is_rail(n))
            });
            if pulled {
                continue;
            }
            let f = pins
                .iter()
                .fold(cx.finding(&META).net(net), |f, p| f.pin(*p));
            out.push(
                f.message(format!(
                    "{} on {} is open-drain with no pull-up footprint: it relies on the reader's internal pull",
                    pin_list(cx, &pins),
                    b.net(net).name
                ))
                .fix("a DNM (or fitted) 10 kΩ to the reader's IO rail, and a test point")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dbg8));

#[cfg(test)]
mod tests {
    #[test]
    fn open_drain_without_pull_up() {
        let found = crate::testing::fingerprints(&super::Dbg8, "dbg-8_open_drain_pull_ups");
        assert_eq!(found, ["DBG-8:net /PMIC_IRQ|pin U2.3"], "{found:?}");
    }
}
