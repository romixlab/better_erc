//! `DRV-14` Inductive load switched without a freewheel path

use super::{designators, unclamped};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::PartClass;

pub struct Drv14;

static META: CheckMeta = CheckMeta {
    id: "DRV-14",
    category: "DRV",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Drv14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted motor, solenoid or transformer with a pin switched by a transistor and no freewheel diode, TVS or
    /// RC across the load or the switch. Loads behind connectors need the project file to say they are inductive
    /// (CFG-2); relays are `DRV-9`.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for load in cx.view().fitted_components() {
            let class = cx.classes().get(load);
            if !matches!(
                class,
                PartClass::Motor | PartClass::Solenoid | PartClass::Transformer
            ) {
                continue;
            }
            for (net, switches) in unclamped(cx, load, false) {
                out.push(
                    cx.finding(&META)
                        .part(load)
                        .net(net)
                        .message(format!(
                            "{} {} is switched on {} by {} with no freewheel diode, TVS or snubber: interrupting its current drives the switch into avalanche",
                            class.name(),
                            b.part(load).designator,
                            b.net(net).name,
                            designators(cx, switches.iter().copied())
                        ))
                        .fix("a freewheel diode (fast or Schottky) across the load, or a TVS across the switch where a fast decay is needed")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Drv14));

#[cfg(test)]
mod tests {
    #[test]
    fn loads_without_freewheel() {
        let found = crate::testing::fingerprints(&super::Drv14, "drv-14_inductive_loads");
        assert_eq!(found, ["DRV-14:net /VALVE_SW|part SOL1"], "{found:?}");
    }
}
