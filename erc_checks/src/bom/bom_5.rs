//! `BOM-5` No TODOs or "calculate later" values left

use super::placeholder;
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Bom5;

static META: CheckMeta = CheckMeta {
    id: "BOM-5",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Bom5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter_map(|p| {
                let fact = erc_analysis::value::value_field(b, p, cx.variant)?;
                placeholder(&fact.value).then(|| {
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} has the placeholder value `{}`",
                            b.part(p).designator,
                            fact.value
                        ))
                        .fact("Value", &fact.value, fact.source.clone())
                        .fix("decide the value; if it stays open, mark the part DNP and say so on the sheet")
                        .build()
                })
            })
            .collect()
    }
}

inventory::submit!(Registered(&Bom5));

#[cfg(test)]
mod tests {
    #[test]
    fn placeholders_fitted_or_not() {
        assert_eq!(
            crate::testing::fingerprints(&super::Bom5, "bom-10_values"),
            ["BOM-5:part R10", "BOM-5:part R3", "BOM-5:part R9"]
        );
    }
}
