//! `LIB-6` Footprint does not match the MPN's package

use crate::component::{self, Judged};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::package::Package;
use erc_model::Subject;

pub struct Lib6;

static META: CheckMeta = CheckMeta {
    id: "LIB-6",
    category: "LIB",
    severity: Severity::Error,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Lib6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each fitted part's `Package` (the board's `DEFAULT_R_PACKAGE` or `DEFAULT_C_PACKAGE` for a resistor or
    /// capacitor without the field) against the chip case its footprint names, and each MPN's case from the parts
    /// snapshot against the footprint's case, else against the `Package` field
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let board = cx.board();
        let mut out = Vec::new();
        for p in component::fitted_sorted(cx) {
            let reference = &board.part(p).designator;
            let footprint = Package::of(board, p);
            let size = footprint.as_ref().and_then(|f| f.size);
            let stated = component::stated(
                cx,
                p,
                "Package",
                component::default_var(reference, "Package"),
            );
            if let (Some(s), Some(size), Some(fp)) = (&stated, size, &footprint)
                && component::case_code(&s.text) != size
            {
                out.push(
                    cx.finding(&META)
                        .part(p)
                        .item(Subject::Part(cx.board_id, p), format!("{reference} Package"))
                        .message(format!(
                            "{reference}: {} says {}, the footprint {} is {size}: write Package {size}",
                            s.from, s.text, fp.name
                        ))
                        .fact(&s.from, &s.text, s.source.clone())
                        .fact("footprint", &fp.name, footprint_source(cx, p))
                        .input(format!("{reference}.{}", s.from), &s.text)
                        .input(format!("{reference}.Footprint"), &fp.name)
                        .fix("set the Package field to the footprint's case, or place the footprint of the stated package")
                        .build(),
                );
            }
            let target = match (size, &stated) {
                (Some(size), _) => Some((size.to_owned(), "the footprint")),
                (None, Some(s)) => Some((component::case_code(&s.text), "Package")),
                (None, None) => None,
            };
            let Some((want, against)) = target else {
                continue;
            };
            let judged = component::candidates(cx, p)
                .iter()
                .map(|c| {
                    let mut j = Judged::new(c);
                    match c.text("case_eia") {
                        None => j.lacks("case_eia"),
                        Some(case) => {
                            j.input(format!("{}.case_eia", c.field), case, c.source("case_eia"));
                            j.inputs(&[(format!("{reference}.case"), want.clone())]);
                            if !component::case_code(case).eq_ignore_ascii_case(&want) {
                                j.miss(format!("case {case}, {against} is {want}"));
                            }
                        }
                    }
                    j
                })
                .collect();
            out.extend(component::report(
                cx,
                &META,
                p,
                judged,
                "pick the footprint from the MPN's package drawing, or an MPN in the footprint's package",
            ));
        }
        out
    }
}

/// Where a part's footprint name comes from: its Footprint field
fn footprint_source(cx: &Ctx, p: erc_model::PartId) -> erc_model::Provenance {
    component::field(cx, p, "Footprint").map_or_else(
        || erc_model::Provenance::Derived {
            rule: "the board file's footprint".to_owned(),
        },
        |f| f.source,
    )
}

inventory::submit!(Registered(&Lib6));
