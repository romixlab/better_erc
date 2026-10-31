//! `DRAW-41` Wire bending right at a pin or a power symbol: its first or last segment shorter than one major grid
//! step, or across the pin's axis

use super::{PIN_LEAD, at, nm, one};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::mm;
use erc_model::geometry::Segment;

pub struct Draw41;

static META: CheckMeta = CheckMeta {
    id: "DRAW-41",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[PIN_LEAD],
};

impl Check for Draw41 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Follows the wire from each pin along the pin's axis. No wire that way, with one ending on the pin, leaves
    /// across the axis (a pin on the middle of a wire is a tee, no bend); a straight run that bends or branches before the lead is up bends too soon. A run that ends on a
    /// pin or a label has no bend, however short: `DRAW-29` looks at the short ones between two pins.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let lead = nm(cx, &META, &PIN_LEAD);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let g = scene.geometry;
            for (r, p) in scene.pins() {
                let Some(item) = scene.pin_item(r) else {
                    continue;
                };
                let ends = |w: &&Segment| !w.is_empty() && (w.a == p.at || w.b == p.at);
                let on_pin = g.wires.iter().any(|w| ends(&w));
                let through = g.wires.iter().any(|w| w.contains(p.at) && !ends(&w));
                let end = scene.run_end(p.at, p.outward);
                let run = (end.x - p.at.x).abs() + (end.y - p.at.y).abs();
                let what = if !on_pin || through {
                    continue;
                } else if run == 0 {
                    format!(
                        "{} leaves across its axis ({})",
                        at(item, p.at),
                        p.outward.name()
                    )
                } else if run < lead && scene.bends_at(end, p.outward) {
                    format!("{} bends {} mm out", at(item, p.at), mm(run))
                } else {
                    continue;
                };
                out.push(
                    one(cx, &META, scene, item, p.at)
                        .message(what)
                        .fix("run the wire straight out of the pin before the first bend")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw41));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw41, "draw-41_pin_lead"),
            [
                "DRAW-41:item R1.1 pin|sheet /",
                "DRAW-41:item R2.1 pin|sheet /"
            ]
        );
    }
}
