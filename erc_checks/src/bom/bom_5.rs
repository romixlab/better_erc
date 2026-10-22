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
                let t = erc_analysis::value::value_text(cx.view(), p)?;
                let fact = t.fact;
                placeholder(&fact.value).then(|| {
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} has the placeholder value `{}`",
                            b.part(p).designator,
                            fact.value
                        ))
                        .fact(t.label(), &fact.value, fact.source.clone())
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

    /// The part's value, not a field by its name: a value an importer read from `Comment` (Altium's convention) is
    /// checked, and the finding says where it came from
    #[test]
    fn value_from_another_field() {
        use erc_model::{DirFiles, ImportOptions};
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
        let mut imported = erc_kicad::project::read(
            &DirFiles(dir),
            "bom-10_values.kicad_sch",
            &ImportOptions::default(),
        )
        .unwrap();
        let b = &mut imported.design.boards[0];
        for p in &mut b.parts {
            for f in &mut p.fields {
                if f.name == "Value" {
                    f.name = "Comment".into();
                }
            }
        }
        let report = crate::run_with(
            &[&super::Bom5],
            &imported.design,
            &crate::RunOptions::default(),
        )
        .unwrap();
        let r3 = report
            .findings
            .iter()
            .find(|f| f.fingerprint == "BOM-5:part R3")
            .expect("still found");
        assert_eq!(r3.facts[0].name, "Value (from Comment)");
    }
}
