//! `DRAW-44` Two sections closer than the section gap

use super::section::sections;
use super::{HEADING_SIZE, SECTION_REACH, location, nm, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};

use erc_model::geometry::Rect;

pub struct Draw44;

const SECTION_GAP: Param = Param {
    name: "drawing.section_gap",
    default: 10.0,
    unit: "mm",
    doc: "gap that separates two sections",
    choices: &[],
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-44",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Hard,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[SECTION_GAP, HEADING_SIZE, SECTION_REACH],
};

/// `x 10..20 y 30..40`, in the run's units
fn span(cx: &Ctx, r: Rect) -> String {
    cx.units().span((r.min.x, r.max.x), (r.min.y, r.max.y))
}

impl Check for Draw44 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The boxes of each pair of sections, heading, parameter box, parts and labels; boxes that run into each other
    /// have no gap at all
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let gap = nm(cx, &META, &SECTION_GAP);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let sheet = sheet_subject(cx, scene);
            let all = sections(cx, &META, scene);
            for (i, a) in all.iter().enumerate() {
                for b in &all[i + 1..] {
                    let apart = a.rect.gap(b.rect);
                    if apart >= gap || a.items.is_empty() || b.items.is_empty() {
                        continue;
                    }
                    let (ha, hb) = (a.heading, b.heading);
                    out.push(
                        cx.finding(&META)
                            .subject(sheet)
                            .scene_item(sheet, ha)
                            .scene_item(sheet, hb)
                            .location(location(scene, ha, a.rect.min))
                            .location(location(scene, hb, b.rect.min))
                            .message(format!(
                                "sections {} ({}) and {} ({}) are {} apart, less than {}",
                                a.title,
                                span(cx, a.rect),
                                b.title,
                                span(cx, b.rect),
                                cx.units().len(apart),
                                cx.units().num(gap)
                            ))
                            .fix("move the sections apart")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw44));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw44, "draw-44_section_gap"),
            ["DRAW-44:item text \"First\"|item text \"Second\"|sheet /"]
        );
    }
}
