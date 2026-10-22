//! `NAME-6` Suggest per page designators; highlight missing ones

use super::{Numbering, numbered_parts, numbering};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{BoardId, Subject};
use std::collections::BTreeSet;

pub struct Name6;

static META: CheckMeta = CheckMeta {
    id: "NAME-6",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Name6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// On a board with parts on two pages or more: numbered per sheet (page × 100 or × 1000), the parts whose
    /// number is not on their page; numbered across the project, one suggestion to number per sheet
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let parts = numbered_parts(cx);
        let pages: BTreeSet<u32> = parts.iter().filter_map(|(_, _, _, pg)| *pg).collect();
        if pages.len() < 2 {
            return Vec::new();
        }
        match numbering(cx) {
            Numbering::Flat => vec![
                cx.finding(&META)
                    .subject(Subject::Board(BoardId(cx.board_id.0)))
                    .message(format!(
                        "designators run across the project's {} pages: numbered per sheet, R607 tells where it is",
                        pages.len()
                    ))
                    .fix("annotate from the sheet number × 100 (KiCad: Annotate, \"Use first free number after sheet number × 100\")")
                    .build(),
            ],
            Numbering::PerSheet { base } => parts
                .iter()
                .filter_map(|(p, _, n, pg)| {
                    let pg = (*pg)?;
                    if n / base == pg {
                        return None;
                    }
                    let part = b.part(*p);
                    Some(
                        cx.finding(&META)
                            .part(*p)
                            .subject(Subject::Sheet(cx.board_id, part.sheet))
                            .message(format!(
                                "{} is on page {pg} ({}), outside its series {}xx",
                                part.designator,
                                b.sheet_path(part.sheet),
                                pg * base / 100
                            ))
                            .fix("re-annotate the part with its sheet's series, or move it back")
                            .build(),
                    )
                })
                .collect(),
        }
    }
}

inventory::submit!(Registered(&Name6));

#[cfg(test)]
mod tests {
    #[test]
    fn off_sheet_numbers() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name6, "name-6_per_sheet_designators"),
            ["NAME-6:part R214|sheet /Power/"]
        );
    }
}
