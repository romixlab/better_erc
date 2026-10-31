//! `DRAW-45` Section headings just off a shared row or column

use super::section::{line_up, sections};
use super::{HEADING_SIZE, SECTION_REACH, location, nm, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::mm;
use erc_model::Pt;

pub struct Draw45;

const SECTION_SNAP: Param = Param {
    name: "drawing.section.snap",
    default: 10.16,
    unit: "mm",
    doc: "offsets of section tops or left edges below this (and above zero) are near misses",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-45",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &["DRAW-36"],
    params: &[SECTION_SNAP, HEADING_SIZE, SECTION_REACH],
};

impl Check for Draw45 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Tops on one row and left edges on one column, by the headings: each line that has outliers is one finding
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let snap = nm(cx, &META, &SECTION_SNAP);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let sheet = sheet_subject(cx, scene);
            let all = sections(cx, &META, scene);
            let axes = [("y", "row"), ("x", "column")];
            for (n, what) in axes {
                let of = |p: Pt| if n == "y" { p.y } else { p.x };
                let tops: Vec<_> = all
                    .iter()
                    .map(|s| (of(s.heading.rect.min), s.heading))
                    .collect();
                for line in line_up(&tops, snap, true) {
                    let list: Vec<String> = line
                        .off
                        .iter()
                        .map(|(v, i)| format!("{} at {n} {}", i.name, mm(*v)))
                        .collect();
                    let mut f = cx.finding(&META).subject(sheet);
                    for (_, i) in &line.off {
                        f = f
                            .item(sheet, i.name.clone())
                            .location(location(scene, i, i.rect.min));
                    }
                    out.push(
                        f.message(format!(
                            "section {what} at {n} {}, off it: {}",
                            mm(line.at),
                            list.join(", ")
                        ))
                        .fix("put the sections on one row or column")
                        .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw45));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw45, "draw-45_section_row"),
            [
                "DRAW-45:item text \"Just below\"|sheet /",
                "DRAW-45:item text \"Just right\"|sheet /"
            ]
        );
    }
}
