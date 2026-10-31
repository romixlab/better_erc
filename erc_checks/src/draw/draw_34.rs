//! `DRAW-34` Items closer than their clearance margins (symbol bodies, power symbols, fields, labels, note boxes)

use super::overlap::{close, finding, gap_text};
use super::{CLEARANCE_PART, CLEARANCE_POWER, CLEARANCE_TEXT};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Draw34;

static META: CheckMeta = CheckMeta {
    id: "DRAW-34",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Hard,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &["DRAW-9"],
    params: &[CLEARANCE_PART, CLEARANCE_POWER, CLEARANCE_TEXT],
};

impl Check for Draw34 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Boxes that stay apart bare and overlap when each grows by its own margin, closest pairs first per sheet
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for (a, b, short) in close(cx, &META, scene) {
                let what = format!("is {} short of its margin to", gap_text(short));
                let fix = "give them room: move one, or set the drawing.clearance margins";
                out.push(finding(cx, &META, scene, a, b, what, fix));
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw34));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw34, "draw-34_too_close"),
            ["DRAW-34:item R1 body|item text box \"Near\"|sheet /"]
        );
    }
}
