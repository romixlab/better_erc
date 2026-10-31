//! `DRAW-28` Label not lying on its wire: text hanging off the stub's end, or justified against its side

use super::{at, nm, one};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{ItemKind as K, mm};
use erc_model::LabelKind;

pub struct Draw28;

const STUB_EXTRA: Param = Param {
    name: "drawing.label.stub_extra",
    default: 1.27,
    unit: "mm",
    doc: "wire a label's stub has beyond its text",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-28",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[STUB_EXTRA],
};

impl Check for Draw28 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Horizontal local labels: the wire must run from the anchor the way the text does, as long as the text and
    /// the setting. A global or hierarchical label's flag starts at the wire's end whatever it says, a vertical
    /// label is `DRAW-11`'s, a label on a bus names the bus.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let extra = nm(cx, &META, &STUB_EXTRA);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let g = scene.geometry;
            let items = scene.items.iter().filter(|i| i.kind == K::Label);
            for (l, item) in g.labels.iter().zip(items) {
                if l.kind != LabelKind::Local
                    || !l.runs.is_horizontal()
                    || g.buses.iter().any(|b| b.contains(l.at))
                {
                    continue;
                }
                let end = scene.run_end(l.at, l.runs);
                let stub = (end.x - l.at.x).abs();
                let text = l.text.bbox.width();
                let on_wire = g.wires.iter().any(|w| !w.is_empty() && w.contains(l.at));
                let what = if stub >= text + extra {
                    continue;
                } else if stub > 0 {
                    format!(
                        "{}: stub {} mm under text {} mm",
                        at(item, l.at),
                        mm(stub),
                        mm(text)
                    )
                } else if on_wire {
                    format!(
                        "{}: text runs {} off its wire",
                        at(item, l.at),
                        l.runs.name()
                    )
                } else {
                    format!("{}: no stub of its own", at(item, l.at))
                };
                out.push(
                    one(cx, &META, scene, item, l.at)
                        .message(what)
                        .fix("justify the label so its text lies along the wire, on a stub longer than the text")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw28));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw28, "draw-28_label_off_wire"),
            [
                "DRAW-28:item label LONGNAME|sheet /",
                "DRAW-28:item label OFF|sheet /",
                "DRAW-28:item label ONPIN|sheet /"
            ]
        );
    }
}
