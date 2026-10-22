//! `BOM-6` "Calculate later" part mounted

use super::placeholder;
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Bom6;

static META: CheckMeta = CheckMeta {
    id: "BOM-6",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Values],
    overlaps: &["BOM-5"],
    params: &[],
};

impl Check for Bom6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter(|p| cx.fitted(*p))
            .filter_map(|p| {
                let fact = erc_analysis::value::value_field(b, p, cx.variant)?;
                placeholder(&fact.value).then(|| {
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} is mounted with the placeholder value `{}`",
                            b.part(p).designator,
                            fact.value
                        ))
                        .fact("Value", &fact.value, fact.source.clone())
                        .fix("decide the value, or make the part DNP")
                        .build()
                })
            })
            .collect()
    }
}

inventory::submit!(Registered(&Bom6));

#[cfg(test)]
mod tests {
    #[test]
    fn fitted_placeholders_only() {
        // R9 has `?` too, and is DNP
        assert_eq!(
            crate::testing::fingerprints(&super::Bom6, "bom-10_values"),
            ["BOM-6:part R10", "BOM-6:part R3"]
        );
    }
}
