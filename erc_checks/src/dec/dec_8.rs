//! `DEC-8` Decoupling only on the source side of a ferrite, filter or series resistor

use super::{caps, designators};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::supply::power_in_pins;
use erc_model::{NetId, PartClass};

pub struct Dec8;

static META: CheckMeta = CheckMeta {
    id: "DEC-8",
    category: "DEC",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Dec8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted two-pin ferrite, inductor or resistor between two rails whose side with power-input pins has no
    /// capacitor to ground while the other side has some
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            if !matches!(
                class,
                PartClass::FerriteBead | PartClass::Inductor | PartClass::Resistor
            ) || b.part(part).pins.len() != 2
            {
                continue;
            }
            let nets: Vec<NetId> = b
                .part(part)
                .pins
                .iter()
                .filter_map(|p| b.pin(*p).net)
                .collect();
            let [x, y] = nets[..] else { continue };
            if x == y || !power.is_rail(x) || !power.is_rail(y) {
                continue;
            }
            for (pin_side, source) in [(x, y), (y, x)] {
                let pins = power_in_pins(cx.view(), pin_side);
                let source_caps = caps(cx, source);
                if pins.is_empty() || !caps(cx, pin_side).is_empty() || source_caps.is_empty() {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .net(pin_side)
                        .message(format!(
                            "{} behind {} {} feeds {} power pin{} and has no capacitor; {} sit on the source side {}",
                            b.net(pin_side).name,
                            class.describe(),
                            b.part(part).designator,
                            pins.len(),
                            if pins.len() == 1 { "" } else { "s" },
                            designators(cx, source_caps),
                            b.net(source).name
                        ))
                        .fix("put the capacitors on the load side, at the pins")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Dec8));

#[cfg(test)]
mod tests {
    #[test]
    fn capacitors_on_the_wrong_side() {
        let found = crate::testing::fingerprints(&super::Dec8, "dec-8_source_side_caps");
        assert_eq!(found, ["DEC-8:net +3V3_VDDA|part R1"], "{found:?}");
    }
}
