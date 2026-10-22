//! `DEC-10` Bypass capacitor on the IC's power pin but not on the rail

use super::{designators, pin_list};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::{EcadPinType, NetId, PartId, PinId};

pub struct Dec10;

static META: CheckMeta = CheckMeta {
    id: "DEC-10",
    category: "DEC",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Dec10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net whose fitted parts are power-input pins and capacitors to ground only: nothing brings power in
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            if power.is_ground(net) {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, net)
                .filter(|p| cx.fitted(b.pin(*p).part))
                .collect();
            let mut loads: Vec<PinId> = Vec::new();
            let mut caps: Vec<PartId> = Vec::new();
            let only = pins.iter().all(|p| {
                let pin = b.pin(*p);
                if pin.etype.get() == Some(&EcadPinType::PowerIn) {
                    loads.push(*p);
                    true
                } else if cx.classes().get(pin.part).is_capacitor() && to_ground(cx, pin.part, net)
                {
                    caps.push(pin.part);
                    true
                } else {
                    false
                }
            });
            if !only || loads.is_empty() || caps.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(net)
                    .message(format!(
                        "{} holds only power pins ({}) and capacitors to ground ({}): nothing brings power in",
                        b.net(net).name,
                        pin_list(cx, &loads),
                        designators(cx, caps)
                    ))
                    .fix("connect the net to its rail (a label, a ferrite or a 0 Ω as intended)")
                    .build(),
            );
        }
        out
    }
}

/// Whether a capacitor's pins other than the one on `net` are all on a ground
fn to_ground(cx: &Ctx, cap: PartId, net: NetId) -> bool {
    let b = cx.board();
    let others: Vec<Option<NetId>> = b
        .part(cap)
        .pins
        .iter()
        .map(|p| b.pin(*p).net)
        .filter(|n| *n != Some(net))
        .collect();
    !others.is_empty()
        && others
            .iter()
            .all(|n| n.is_some_and(|n| cx.power().is_ground(n)))
}

inventory::submit!(Registered(&Dec10));

#[cfg(test)]
mod tests {
    #[test]
    fn island_of_pins_and_caps() {
        let found = crate::testing::fingerprints(&super::Dec10, "dec-10_unsourced_bypass");
        assert_eq!(found, ["DEC-10:net +5V0_FLT"], "{found:?}");
    }
}
