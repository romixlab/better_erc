//! `BOM-22` Maker field (`MfgN`) without the MPN field it belongs beside

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_parts::mpn::lone_makers;

pub struct Bom22;

static META: CheckMeta = CheckMeta {
    id: "BOM-22",
    category: "BOM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Bom22 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every filled maker field of a fitted part in the variant checked whose MPN field (`Mfg2` beside `Mpn2`) is
    /// absent or empty: the maker names nothing, and the MPN it was meant for may have been typed into another
    /// field or dropped
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let view = cx.view();
        let mut out = Vec::new();
        for p in view.fitted_components() {
            for (field, maker) in lone_makers(view, p) {
                out.push(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} names maker `{maker}` in {field} but has no MPN beside it",
                            b.part(p).designator
                        ))
                        .fix(format!(
                            "fill in the MPN field `{field}` belongs beside (`Mpn2` for `Mfg2`), or drop `{field}` when \
                             the MPN is gone"
                        ))
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Bom22));
