//! `DRAW-49` Placeholder left in a note, a text or a field

use super::one;
use crate::meta::*;
use crate::placeholder::{heading_word, placeholder, placeholders_in};
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene};
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

/// A text, text box or table of a sheet, as part of a note
struct Note<'s> {
    item: &'s Item,
    /// A bold free text that opens with an open word: the heading grouped with a note's box (`DRAW-32`)
    bold_heading: bool,
    /// The open word that heads it
    heading: Option<String>,
    /// The placeholders it holds besides that word
    spots: Vec<String>,
}

impl<'s> Note<'s> {
    fn free(item: &'s Item, t: &Text) -> Self {
        let text = t.text.trim();
        let heading = heading_word(&t.text, t.bold);
        let body = match heading {
            // The word of a bold heading ends at its colon or at a space
            Some(_) if t.bold => text
                .split_once(char::is_whitespace)
                .map_or("", |(_, rest)| rest),
            _ => text,
        };
        Note {
            item,
            bold_heading: t.bold && heading.is_some(),
            heading: heading.map(str::to_string),
            spots: placeholders_in(body),
        }
    }

    fn boxed(item: &'s Item, t: &Text) -> Self {
        Note {
            item,
            bold_heading: false,
            heading: heading_word(&t.text, false).map(str::to_string),
            spots: placeholders_in(&t.text),
        }
    }

    /// What to report on it: its placeholders; else its heading's open word, when nothing grouped with it holds a
    /// placeholder and no bold heading grouped with it is reported in its place
    fn report(&self, scene: &Scene, all: &[Note]) -> Vec<String> {
        if !self.spots.is_empty() {
            return self.spots.clone();
        }
        let silent = all
            .iter()
            .filter(|o| scene.grouped(self.item, o.item))
            .any(|o| !o.spots.is_empty() || (o.bold_heading && !self.bold_heading));
        self.heading.iter().filter(|_| !silent).cloned().collect()
    }
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
            let free = texts.zip(&g.texts).map(|(i, t)| Note::free(i, &t.text));
            let boxed = boxes
                .zip(&g.text_boxes)
                .map(|(i, b)| Note::boxed(i, &b.text));
            let cells = tables.zip(&g.tables).map(|(i, t)| {
                let spots = t.cells.iter().flat_map(|c| placeholders_in(&c.text.text));
                Note {
                    item: i,
                    bold_heading: false,
                    heading: None,
                    spots: spots.collect(),
                }
            });
            let notes: Vec<Note> = free.chain(boxed).chain(cells).collect();
            for note in &notes {
                let spots = note.report(scene, &notes);
                if spots.is_empty() {
                    continue;
                }
                let item = note.item;
                let fix = if note.spots.is_empty() {
                    "do the open work, then delete the note"
                } else {
                    "fill in the value, or delete the line if it does not apply"
                };
                out.push(
                    one(cx, &META, scene, item, item.rect.min)
                        .message(format!(
                            "{} holds the placeholder {}",
                            item.name,
                            quoted(&spots)
                        ))
                        .fix(fix)
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
