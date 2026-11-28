//! `DRAW-34` Items closer than their clearance margins (symbol bodies, power symbols, fields, labels, note boxes)

use super::overlap::{close, finding};
use super::{
    CLEARANCE_PACKED, CLEARANCE_PART, CLEARANCE_POWER, CLEARANCE_SPACING, CLEARANCE_TEXT,
    FIELDS_BODY_GAP, FIELDS_ENDS_RAISE, FIELDS_GAP_MAX, FIELDS_HIDE_DEFAULTS, FIELDS_PITCH,
    FIELDS_STEP,
};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Draw34;

static META: CheckMeta = CheckMeta {
    id: "DRAW-34",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Hard,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &["DRAW-9"],
    // The field layout of its fix, `berc place-fields` (TOOL-13)
    params: &[
        CLEARANCE_PART,
        CLEARANCE_POWER,
        CLEARANCE_TEXT,
        CLEARANCE_SPACING,
        CLEARANCE_PACKED,
        FIELDS_PITCH,
        FIELDS_BODY_GAP,
        FIELDS_STEP,
        FIELDS_GAP_MAX,
        FIELDS_ENDS_RAISE,
        FIELDS_HIDE_DEFAULTS,
    ],
};

impl Check for Draw34 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Boxes that stay apart bare and overlap when each grows by its own margin, closest pairs first per sheet
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for (a, b, short) in close(cx, &META, scene) {
                let what = format!("is {} short of its margin to", cx.units().len(short));
                let fix = "give them room: move one, or set the drawing.clearance margins";
                out.push(finding(cx, &META, scene, a, b, what, fix));
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw34));

/// The margins this check holds items to, as the board's settings give them
pub(super) fn margins(cx: &Ctx) -> super::overlap::Margins {
    super::overlap::Margins::of(cx, &META)
}

/// A `drawing.fields.*` setting of the board, in millimetres
pub(super) fn field_setting(cx: &Ctx, p: &crate::meta::Param) -> f64 {
    cx.param(&META, p.name)
}

/// A `drawing.fields.*` word setting of the board
pub(super) fn field_choice(cx: &Ctx, p: &crate::meta::Param) -> &'static str {
    cx.choice(&META, p.name)
}

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw34, "draw-34_too_close"),
            ["DRAW-34:item R1 body|item text box \"Near\"|sheet /"]
        );
    }

    #[test]
    fn writes_positions_and_lengths_in_the_run_units() {
        let message = |units| {
            let mut opts = crate::RunOptions::default();
            opts.config.units = units;
            let report = crate::testing::report_with(&super::Draw34, "draw-34_too_close", &opts);
            report.findings[0].message.clone()
        };
        let mm = message(None);
        let mil = message(Some(crate::Units::Mil));
        assert!(mm.contains(" mm short") && !mm.contains("mil"), "{mm}");
        assert!(mil.contains(" mil short") && !mil.contains("mm"), "{mil}");
        assert_eq!(
            mil.matches(" mil").count(),
            3,
            "two positions and one length: {mil}"
        );
        let fingerprints = |units| {
            let mut opts = crate::RunOptions::default();
            opts.config.units = units;
            crate::testing::fingerprints_with(&super::Draw34, "draw-34_too_close", &opts)
        };
        assert_eq!(fingerprints(None), fingerprints(Some(crate::Units::Inch)));
    }
}
