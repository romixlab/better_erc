//! `BOM-14` Mounted part without an MPN

use super::mpns;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{PartClass, PartId, Subject};

pub struct Bom14;

static META: CheckMeta = CheckMeta {
    id: "BOM-14",
    category: "BOM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Variant, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

/// Parts nobody buys by MPN: test points, fiducials, mounting holes, net ties
fn exempt(class: PartClass) -> bool {
    class.is_mechanical() || matches!(class, PartClass::TestPoint | PartClass::NetTie)
}

impl Check for Bom14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Fitted parts in the BOM without an MPN; when no part of the board has one, one finding for the board
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let view = cx.view();
        let classes = cx.classes();
        let bought: Vec<PartId> = view
            .fitted_components()
            .filter(|p| view.in_bom(*p) && !exempt(classes.get(*p)))
            .collect();
        let missing: Vec<PartId> = bought
            .iter()
            .copied()
            .filter(|p| mpns(view, *p).is_empty())
            .collect();
        if missing.is_empty() {
            return Vec::new();
        }
        if missing.len() == bought.len() && missing.len() > 1 {
            let mut names: Vec<&str> = missing
                .iter()
                .map(|p| b.part(*p).designator.as_str())
                .collect();
            names.sort_by(|a, b| erc_model::natural_cmp(a, b));
            return vec![
                cx.finding(&META)
                    .subject(Subject::Board(cx.board_id))
                    .message(format!(
                        "no part has an MPN: {} parts in the BOM ({}) leave the choice to the assembler",
                        missing.len(),
                        names.join(", ")
                    ))
                    .fix("add an MPN field (`MPN`, `Mpn1`) to the parts, with alternates for passives")
                    .build(),
            ];
        }
        missing
            .into_iter()
            .map(|p| {
                let class = classes.fact(p);
                cx.finding(&META)
                    .part(p)
                    .message(format!(
                        "{} ({} {}) is fitted without an MPN",
                        b.part(p).designator,
                        class.value.describe(),
                        view.value_text(p).unwrap_or("").trim()
                    ))
                    .fix("fill in the MPN, with alternates for passives")
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&Bom14));

#[cfg(test)]
mod tests {
    #[test]
    fn fitted_without_mpn() {
        // R2 has an MPN, R3 is DNP, TP1 is a test point, R4 an empty Mpn1 and an MPN in Mpn2
        assert_eq!(
            crate::testing::fingerprints(&super::Bom14, "bom-14_mpn"),
            ["BOM-14:part C1", "BOM-14:part R1"]
        );
    }
}
