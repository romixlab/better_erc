//! `DRAW-35` Two- to four-pin part tied to a supply or ground drawn horizontal

use super::{at, one};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{ItemKind as K, Scene};
use erc_model::geometry::{DrawnPin, DrawnSymbol};
use erc_model::{NetId, PartClass};

pub struct Draw35;

const MAX_PINS: Param = Param {
    name: "max_pins",
    default: 4.0,
    unit: "",
    doc: "pins of the largest part that stands on the voltage axis",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-35",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry, Need::Rails, Need::PartClasses],
    overlaps: &[],
    params: &[MAX_PINS],
};

/// Whether a part of this class stands on the voltage axis: discrete parts, whatever their pin count
pub(super) fn small_part(cx: &Ctx, s: &DrawnSymbol) -> bool {
    let part = cx.board().part_by_designator(&s.reference);
    part.is_some_and(|p| {
        matches!(
            cx.classes().get(p),
            PartClass::Resistor
                | PartClass::Capacitor { .. }
                | PartClass::Inductor
                | PartClass::FerriteBead
                | PartClass::Diode { .. }
                | PartClass::Fuse { .. }
        )
    })
}

/// The net of a drawn pin, by the part's designator and the pin's number
fn net_of(cx: &Ctx, s: &DrawnSymbol, p: &DrawnPin) -> Option<NetId> {
    let b = cx.board();
    let part = b.part_by_designator(&s.reference)?;
    let pins = b.part(part).pins.iter().map(|q| b.pin(*q));
    pins.filter(|q| q.number == p.number).find_map(|q| q.net)
}

/// The part's body item, or the first thing of it
fn part_item<'s>(scene: &'s Scene, symbol: usize) -> Option<&'s erc_analysis::scene::Item> {
    let mut own = scene.items.iter().filter(|i| i.owner == Some(symbol));
    own.find(|i| i.kind == K::Body)
}

impl Check for Draw35 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Small discrete parts with a rail or ground pin that points left or right. A part between two rails, ground not among them, sits in the power path
    /// (a ferrite, a shunt), where lying down is right.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let power = cx.power();
        let max = cx.param(&META, MAX_PINS.name) as usize;
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for (i, s) in scene.geometry.symbols.iter().enumerate() {
                let pins: Vec<&DrawnPin> = s.pins.iter().filter(|p| !p.hidden).collect();
                let small = (2..=max).contains(&pins.len());
                if s.power || !small || !small_part(cx, s) {
                    continue;
                }
                let nets: Vec<Option<NetId>> = pins.iter().map(|p| net_of(cx, s, p)).collect();
                let rails = nets.iter().flatten().filter(|n| power.is_rail(**n)).count();
                let grounds = nets
                    .iter()
                    .flatten()
                    .filter(|n| power.is_ground(**n))
                    .count();
                if rails >= 2 && grounds == 0 {
                    continue;
                }
                let on_power =
                    |n: &Option<NetId>| n.is_some_and(|n| power.is_rail(n) || power.is_ground(n));
                let lying = pins
                    .iter()
                    .zip(&nets)
                    .find(|(p, n)| on_power(n) && p.outward.is_horizontal());
                let Some(((p, _), item)) = lying.zip(part_item(scene, i)) else {
                    continue;
                };
                out.push(
                    one(cx, &META, scene, item, s.at)
                        .message(format!(
                            "{} lies horizontal, pin {} to power",
                            at(item, s.at),
                            p.number
                        ))
                        .fix("stand it on the voltage axis: rail end up, ground end down")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw35));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw35, "draw-35_small_part_horizontal"),
            ["DRAW-35:item R1 body|sheet /"]
        );
    }
}
