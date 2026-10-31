//! `DRAW-9` Text overlapping other text, a symbol body or a wire (fields, labels, notes)

use super::overlap::{bare, finding};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Draw9;

static META: CheckMeta = CheckMeta {
    id: "DRAW-9",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Hard,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[],
};

impl Check for Draw9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Boxes of two items that share area, with no margin; a wire against a body or a pin is `DRAW-7`
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for (a, b, _) in bare(scene, false) {
                let fix = "move one of them; a part's fields go beside its body, clear of wires";
                out.push(finding(cx, &META, scene, a, b, "overlaps".into(), fix));
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw9));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw9, "draw-9_text_overlap"),
            ["DRAW-9:item R1 Value|item text \"Over the value\"|sheet /"]
        );
    }
}
