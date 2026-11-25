//! `DRAW-49` Placeholder left in a note, a text or a field

use super::one;
use crate::meta::*;
use crate::placeholder::{placeholder, placeholders_in};
use crate::{Ctx, Finding};
use erc_analysis::scene::ItemKind as K;
use erc_model::geometry::Text;

pub struct Draw49;

static META: CheckMeta = CheckMeta {
    id: "DRAW-49",
    category: "DRAW",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[],
};

/// Fields another check reads for placeholders: the value (`BOM-5`), the reference (`NAME-12`)
const SKIPPED_FIELDS: [&str; 2] = ["Reference", "Value"];

/// The placeholders of a sheet text; a bold heading's first word names its note's kind (`DRAW-32`), not a placeholder
fn of_text(t: &Text) -> Vec<String> {
    let text = t.text.trim();
    let first = text.split_whitespace().next().unwrap_or("");
    let rest = if t.bold && first.trim_end_matches(':').eq_ignore_ascii_case("TODO") {
        &text[first.len()..]
    } else {
        text
    };
    placeholders_in(rest)
}

/// `` `x V`, `y V` ``
fn quoted(spots: &[String]) -> String {
    let q: Vec<String> = spots.iter().map(|s| format!("`{s}`")).collect();
    q.join(", ")
}

impl Check for Draw49 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every text, text box and table cell of each sheet, then every field of each part but its reference and value
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let g = scene.geometry;
            let texts = scene.items.iter().filter(|i| i.kind == K::Text);
            let boxes = scene.items.iter().filter(|i| i.kind == K::TextBox);
            let tables = scene.items.iter().filter(|i| i.kind == K::Table);
            let free = texts.zip(&g.texts).map(|(i, t)| (i, of_text(&t.text)));
            let boxed = boxes
                .zip(&g.text_boxes)
                .map(|(i, b)| (i, placeholders_in(&b.text.text)));
            let cells = tables.zip(&g.tables).map(|(i, t)| {
                let spots = t.cells.iter().flat_map(|c| placeholders_in(&c.text.text));
                (i, spots.collect())
            });
            for (item, spots) in free.chain(boxed).chain(cells) {
                if spots.is_empty() {
                    continue;
                }
                out.push(
                    one(cx, &META, scene, item, item.rect.min)
                        .message(format!(
                            "{} holds the placeholder {}",
                            item.name,
                            quoted(&spots)
                        ))
                        .fix("fill in the value, or delete the line if it does not apply")
                        .build(),
                );
            }
        }
        let b = cx.board();
        let view = cx.view();
        for p in cx.components() {
            let mut spots = Vec::new();
            for f in &b.part(p).fields {
                if SKIPPED_FIELDS.contains(&f.name.as_str()) {
                    continue;
                }
                let Some(v) = view.field(p, &f.name) else {
                    continue;
                };
                let mut found = placeholders_in(v);
                if found.is_empty() && placeholder(v) {
                    found.push(v.trim().to_string());
                }
                spots.extend(found.into_iter().map(|s| format!("{} `{s}`", f.name)));
            }
            if !spots.is_empty() {
                out.push(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "{} has a placeholder in {}",
                            b.part(p).designator,
                            spots.join(", ")
                        ))
                        .fix("fill in the field, or clear it")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw49));
