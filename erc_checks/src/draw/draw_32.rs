//! `DRAW-32` Free-standing text, or a note box that is not one of the six kinds (TODO, Information, Caution, Debug,
//! Critical, Layout)

use super::{HEADING_SIZE, nm, one};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene, mm};
use erc_model::geometry::{Colour, DrawnTextBox, Text};

pub struct Draw32;

const GROUP_HEADING_SIZE: Param = Param {
    name: "drawing.group_heading_size",
    default: 1.905,
    unit: "mm",
    doc: "height of a group heading inside a section",
};

const COLOUR_SLACK: Param = Param {
    name: "drawing.note.colour_slack",
    default: 8.0,
    unit: "counts",
    doc: "difference per colour channel a note box's stroke may have from its kind",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-32",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[HEADING_SIZE, GROUP_HEADING_SIZE, COLOUR_SLACK],
};

/// The note kinds of the drawing guide with their stroke colours. Information and the parameter box share a grey;
/// the first is dashed, the second solid and without a heading.
const KINDS: [(&str, (u8, u8, u8)); 6] = [
    ("TODO", (0, 131, 143)),
    ("Information", (110, 110, 110)),
    ("Caution", (167, 118, 1)),
    ("Debug", (232, 113, 10)),
    ("Critical", (210, 40, 70)),
    ("Layout", (21, 101, 192)),
];

/// The grey of a parameter box's solid stroke
const PARAMETER_BOX: (u8, u8, u8) = (110, 110, 110);

/// The words a simulator directive starts with, as in ngspice and KiCad's simulator
const DIRECTIVES: &[&str] = &[
    ".ac", ".control", ".csparam", ".dc", ".disto", ".end", ".endc", ".ends", ".four", ".func",
    ".global", ".ic", ".include", ".lib", ".meas", ".measure", ".model", ".nodeset", ".noise",
    ".op", ".options", ".option", ".param", ".plot", ".print", ".probe", ".pz", ".save", ".sens",
    ".sp", ".step", ".subckt", ".temp", ".tf", ".title", ".tran",
];

fn near(c: Colour, (r, g, b): (u8, u8, u8), slack: f64) -> bool {
    let d = |x: u8, y: u8| (x as f64 - y as f64).abs() <= slack;
    d(c.r, r) && d(c.g, g) && d(c.b, b)
}

fn is_directive(text: &str) -> bool {
    let word = text.split_whitespace().next().unwrap_or("");
    DIRECTIVES.contains(&word.to_lowercase().as_str())
}

/// What is wrong with a free text item, if anything
fn free_text(scene: &Scene, item: &Item, t: &Text, heading: (i64, i64)) -> Option<String> {
    if t.text.trim_start().starts_with('.') {
        return (!is_directive(t.text.trim_start())).then(|| {
            format!(
                "{} starts with a dot and is no simulator directive",
                item.name
            )
        });
    }
    let in_box = scene
        .items
        .iter()
        .any(|o| o.kind == K::TextBox && scene.grouped(item, o));
    let mut sheets = scene.items.iter().filter(|o| o.kind == K::Sheet);
    let caption = sheets.any(|o| o.rect.contains(t.bbox.min) && o.rect.contains(t.bbox.max));
    if caption || (t.bold && (t.size.0 >= heading.0 || t.size.0 == heading.1 || in_box)) {
        return None;
    }
    let what = if t.bold {
        format!(
            "is bold at {} mm, neither a section heading nor a note heading",
            mm(t.size.0)
        )
    } else {
        "stands free: put it in a note box".to_string()
    };
    Some(format!("{} {what}", item.name))
}

/// What is wrong with a text box, if anything
fn note_box(scene: &Scene, item: &Item, b: &DrawnTextBox, slack: f64) -> Option<String> {
    let colour = b.stroke.colour;
    let solid = b.stroke.style == "solid";
    if solid && colour.is_some_and(|c| near(c, PARAMETER_BOX, slack)) {
        return None;
    }
    let kind = KINDS
        .iter()
        .find(|(_, k)| colour.is_some_and(|c| near(c, *k, slack)));
    let Some((kind, _)) = kind else {
        return Some(format!("{} has no note kind's stroke colour", item.name));
    };
    let heading = scene.geometry.texts.iter().find(|t| {
        let member = |u: &String| *u == t.uuid;
        let group = scene.geometry.groups.iter();
        t.text.bold
            && group
                .filter(|g| g.members.contains(&item.uuid))
                .any(|g| g.members.iter().any(member))
    });
    let word = heading.map(|h| {
        let w = h.text.text.split([':', ' ']).next().unwrap_or("");
        w.to_string()
    });
    match word {
        Some(w) if w == *kind => None,
        Some(w) => Some(format!(
            "{} has a {kind} stroke under the heading {w}",
            item.name
        )),
        None => Some(format!(
            "{} has a {kind} stroke and no heading grouped with it",
            item.name
        )),
    }
}

impl Check for Draw32 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Free text that is neither a section or group heading, nor a note heading in a group with its box, nor a
    /// sheet box's caption, nor a simulator directive; text boxes by the colour and the heading of the six kinds, or a parameter box's solid grey.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let heading = (
            nm(cx, &META, &HEADING_SIZE),
            nm(cx, &META, &GROUP_HEADING_SIZE),
        );
        let slack = cx.param(&META, COLOUR_SLACK.name);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let g = scene.geometry;
            let texts = scene.items.iter().filter(|i| i.kind == K::Text);
            for (item, t) in texts.zip(&g.texts) {
                if let Some(m) = free_text(scene, item, &t.text, heading) {
                    out.push(
                        one(cx, &META, scene, item, t.text.at)
                            .message(m)
                            .fix("a note box of one of the six kinds, or a bold 2.54 mm section heading")
                            .build(),
                    );
                }
            }
            let boxes = scene.items.iter().filter(|i| i.kind == K::TextBox);
            for (item, b) in boxes.zip(&g.text_boxes) {
                if let Some(m) = note_box(scene, item, b, slack) {
                    out.push(
                        one(cx, &META, scene, item, b.rect.min)
                            .message(m)
                            .fix("the stroke and the heading of one of the six kinds, grouped with the box")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw32));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw32, "draw-32_free_text"),
            [
                "DRAW-32:item text \".notes for later\"|sheet /",
                "DRAW-32:item text \"A loose remark\"|sheet /",
                "DRAW-32:item text box \"Keep this short\"|sheet /",
                "DRAW-32:item text box \"No kind at all\"|sheet /"
            ]
        );
    }
}
