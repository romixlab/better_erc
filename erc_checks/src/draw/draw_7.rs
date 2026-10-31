//! `DRAW-7` Wire running through a symbol body, a field or a label

use super::overlap::{bare, finding};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Draw7;

static META: CheckMeta = CheckMeta {
    id: "DRAW-7",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Hard,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[],
};

impl Check for Draw7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A wire through a body, a power symbol or a pin it does not connect to; a wire over text is `DRAW-9`
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for (a, b, _) in bare(scene, true) {
                let fix = "route the wire around it";
                out.push(finding(cx, &META, scene, a, b, "runs through".into(), fix));
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw7));
