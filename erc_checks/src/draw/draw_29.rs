//! `DRAW-29` Pin joined directly to another pin, with no wire between them

use super::{PIN_LEAD, at, location, nm, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{PinRef, Scene};
use erc_model::Pt;
use erc_model::geometry::DrawnPin;
use std::collections::BTreeSet;

pub struct Draw29;

static META: CheckMeta = CheckMeta {
    id: "DRAW-29",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[PIN_LEAD],
};

/// A part's pins, power symbols left out: a power symbol sits on a pin by design
fn part_pins<'s>(scene: &'s Scene) -> Vec<(PinRef, &'s DrawnPin)> {
    let g = scene.geometry;
    scene
        .pins()
        .filter(|(r, _)| !g.symbols[r.symbol].power)
        .collect()
}

impl Check for Draw29 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Two pins of different symbols on one point, or facing each other over less wire than the lead
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let lead = nm(cx, &META, &PIN_LEAD);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let sheet = sheet_subject(cx, scene);
            let pins = part_pins(scene);
            let mut seen: BTreeSet<(PinRef, PinRef)> = BTreeSet::new();
            for (ra, a) in &pins {
                let end: Pt = scene.run_end(a.at, a.outward);
                for (rb, b) in &pins {
                    let gap = (end.x - a.at.x).abs() + (end.y - a.at.y).abs();
                    let touching = a.at == b.at;
                    let short = gap > 0 && gap < lead && end == b.at;
                    if ra.symbol == rb.symbol || !(touching || short) {
                        continue;
                    }
                    if !seen.insert((*ra.min(rb), *ra.max(rb))) {
                        continue;
                    }
                    let (Some(ia), Some(ib)) = (scene.pin_item(*ra), scene.pin_item(*rb)) else {
                        continue;
                    };
                    let how = if touching {
                        "sits on"
                    } else {
                        "is too close to"
                    };
                    out.push(
                        cx.finding(&META)
                            .subject(sheet)
                            .item(sheet, ia.name.clone())
                            .item(sheet, ib.name.clone())
                            .location(location(scene, ia, a.at))
                            .location(location(scene, ib, b.at))
                            .message(format!("{} {how} {}", at(ia, a.at), at(ib, b.at)))
                            .fix("move the parts apart: at least one major grid step of wire between two pins")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw29));
