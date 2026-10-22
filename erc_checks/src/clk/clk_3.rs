//! `CLK-3` Crystal NC pins may need to go to ground

use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_degree;
use erc_model::{PartClass, PinId, Subject};

pub struct Clk3;

static META: CheckMeta = CheckMeta {
    id: "CLK-3",
    category: "CLK",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Clk3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted crystal with four or more pins, two of them connected (the crystal) and the others connected to
    /// nothing (no net, a net of their own, a no-connect flag): the case pads float. With the board file, also a
    /// crystal footprint whose pads beyond its symbol's pins have no net (a 2-pin symbol on a 4-pad package).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            if cx.classes().get(part) != PartClass::Crystal {
                continue;
            }
            let open = |p: &PinId| {
                let pin = b.pin(*p);
                pin.no_connect || pin.net.is_none_or(|n| net_degree(b, n) < 2)
            };
            let pins = &b.part(part).pins;
            let floating: Vec<PinId> = pins.iter().copied().filter(|p| open(p)).collect();
            if pins.len() >= 4 && !floating.is_empty() && pins.len() - floating.len() >= 2 {
                out.push(
                    floating
                        .iter()
                        .fold(cx.finding(&META).part(part), |f, p| f.pin(*p))
                        .message(format!(
                            "crystal {}: {} connect to nothing; on a 4-pad package these are the case pads, and a floating lid couples noise into the oscillator",
                            b.part(part).designator,
                            pin_list(cx, &floating)
                        ))
                        .fix("connect the case pads to the oscillator's ground, as the datasheet shows")
                        .build(),
                );
            }
            let Some(layout) = &b.layout else { continue };
            for fp in layout.footprints.iter().filter(|f| f.part == Some(part)) {
                let numbers: Vec<&str> = pins.iter().map(|p| b.pin(*p).number.as_str()).collect();
                let pads: Vec<_> = fp
                    .pads
                    .iter()
                    .copied()
                    .filter(|id| {
                        let pad = layout.pad(*id);
                        pad.kind != "np_thru_hole"
                            && pad.net.is_none()
                            && !numbers.contains(&pad.number.as_str())
                            && !pad.number.is_empty()
                    })
                    .collect();
                if pads.is_empty() {
                    continue;
                }
                let names: Vec<String> = pads
                    .iter()
                    .map(|id| format!("pad {}", layout.pad(*id).number))
                    .collect();
                out.push(
                    pads.iter()
                        .fold(cx.finding(&META).part(part), |f, id| {
                            f.subject(Subject::Pad(cx.board_id, *id))
                        })
                        .message(format!(
                            "crystal {}: footprint {} has {} with no pin in the symbol and no net: the case pads float",
                            b.part(part).designator,
                            fp.lib_id,
                            crate::text::and_list(&names)
                        ))
                        .fix("a symbol with the case pins, connected to the oscillator's ground")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Clk3));

#[cfg(test)]
mod tests {
    #[test]
    fn floating_case_pads() {
        let found = crate::testing::fingerprints(&super::Clk3, "clk-3_crystal_case_pads");
        assert_eq!(found, ["CLK-3:part Y1|pin Y1.2|pin Y1.4"], "{found:?}");
    }
}
