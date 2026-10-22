//! `BOM-9` Part without a value

use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Bom9;

static META: CheckMeta = CheckMeta {
    id: "BOM-9",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Bom9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter(|p| {
                erc_analysis::value::value_text(cx.view(), *p)
                    .is_none_or(|t| t.fact.value.trim().is_empty())
            })
            .map(|p| {
                cx.finding(&META)
                    .part(p)
                    .message(format!("{} has no value", b.part(p).designator))
                    .fix("fill in the value (and the MPN)")
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&Bom9));

#[cfg(test)]
mod tests {
    #[test]
    fn empty_value() {
        assert_eq!(
            crate::testing::fingerprints(&super::Bom9, "bom-10_values"),
            ["BOM-9:part R2"]
        );
    }
}
