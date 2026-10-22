//! `DRV-9` Relays without flyback diode

use super::{designators, unclamped};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{PartClass, PartId};

pub struct Drv9;

static META: CheckMeta = CheckMeta {
    id: "DRV-9",
    category: "DRV",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A solid-state or photo-MOS relay: an LED input, no coil
fn solid_state(cx: &Ctx, part: PartId) -> bool {
    let b = cx.board();
    let p = b.part(part);
    let desc = p
        .symbol
        .map(|s| b.symbols[s.index()].description.to_lowercase())
        .unwrap_or_default();
    p.lib_id.starts_with("Relay_SolidState:")
        || [
            "solid state",
            "solid-state",
            "photomos",
            "photo-mos",
            "optomos",
            "photorelay",
            "photovoltaic",
            "mosfet output",
        ]
        .iter()
        .any(|w| desc.contains(w))
}

impl Check for Drv9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted electromechanical relay with a pin switched by a transistor or an IC output and no diode, TVS or
    /// RC across the coil or the switch. The coil is found by its switched pin (relay pins are mostly unnamed);
    /// a relay no switch drives is not judged.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for k in cx.view().fitted_components() {
            if cx.classes().get(k) != PartClass::Relay || solid_state(cx, k) {
                continue;
            }
            for (net, switches) in unclamped(cx, k, true) {
                out.push(
                    cx.finding(&META)
                        .part(k)
                        .net(net)
                        .message(format!(
                            "relay {} is switched on {} by {} with no flyback diode, TVS or RC: the coil's kick takes the switch into avalanche",
                            b.part(k).designator,
                            b.net(net).name,
                            designators(cx, switches.iter().copied())
                        ))
                        .fix("a diode across the coil (cathode to the supply side), with a Zener in series for fast release; or a TVS where the driver has the headroom")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Drv9));

#[cfg(test)]
mod tests {
    #[test]
    fn relay_without_flyback() {
        let found = crate::testing::fingerprints(&super::Drv9, "drv-9_relay_flyback");
        assert_eq!(found, ["DRV-9:net /K1_COIL|part K1"], "{found:?}");
    }
}
