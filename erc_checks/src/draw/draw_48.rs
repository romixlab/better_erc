//! `DRAW-48` Functional block (parts sharing a `Block` field) with no note beside it

use super::block::{Block, blocks, finding, is_cost};
use super::{HEADING_SIZE, nm};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene};
use std::collections::BTreeSet;

pub struct Draw48;

pub(super) const BLOCK_REACH: Param = Param {
    name: "drawing.block.reach",
    default: 25.4,
    unit: "mm",
    doc: "how far from a block's nearest part or heading a note may stand and still belong to it",
    choices: &[],
};

const BLOCK_TIE: Param = Param {
    name: "drawing.block.tie",
    default: 2.54,
    unit: "mm",
    doc: "blocks this much farther from a note than the nearest one get the note too",
    choices: &[],
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-48",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[BLOCK_REACH, BLOCK_TIE, HEADING_SIZE],
};

/// The notes of a sheet with their text: text boxes and table cells with text, and texts that are neither a heading
/// (bold, or at the heading size or larger) nor a cost line
fn notes<'s>(scene: &'s Scene, heading: i64) -> Vec<(&'s Item, &'s str)> {
    let g = scene.geometry;
    let texts = scene.items.iter().filter(|i| i.kind == K::Text);
    let free = texts.zip(&g.texts).filter_map(|(item, t)| {
        let text = t.text.text.trim();
        let title = t.text.bold || t.text.size.0 >= heading;
        (!text.is_empty() && !title && !is_cost(text)).then_some((item, text))
    });
    let boxes = scene.items.iter().filter(|i| i.kind == K::TextBox);
    let filled = boxes.zip(&g.text_boxes).filter_map(|(item, b)| {
        let text = b.text.text.trim();
        (!text.is_empty() && !is_cost(text)).then_some((item, text))
    });
    let tables = scene.items.iter().filter(|i| i.kind == K::Table);
    let cells = tables.zip(&g.tables).flat_map(|(item, t)| {
        let text = t.cells.iter().map(|c| c.text.text.trim());
        text.filter(|t| !t.is_empty()).map(move |t| (item, t))
    });
    free.chain(filled).chain(cells).collect()
}

/// Whether a note names one of the block's parts by its reference, as a word (`R901`, not `R9010`)
fn names_part(text: &str, block: &Block) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| block.references.iter().any(|r| r == w))
}

/// The blocks the notes belong to: each block a note names a part of, else the nearest by the gap to its parts and
/// heading, within `reach`, and every block within `tie` of that gap (a note between two blocks serves both)
fn documented(all: &[Block], notes: &[(&Item, &str)], reach: i64, tie: i64) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for (item, text) in notes {
        let named: Vec<usize> = (0..all.len())
            .filter(|i| names_part(text, &all[*i]))
            .collect();
        if !named.is_empty() {
            out.extend(named);
            continue;
        }
        let gaps: Vec<i64> = all.iter().map(|b| b.gap(item.rect)).collect();
        let Some(min) = gaps.iter().copied().min().filter(|g| *g <= reach) else {
            continue;
        };
        out.extend((0..all.len()).filter(|i| gaps[*i] <= min + tie));
    }
    out
}

/// The names (lower case) of the blocks whose heading is on some sheet
fn headed(cx: &Ctx) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for scene in cx.scenes() {
        for b in blocks(scene) {
            if b.heading.is_some() {
                out.insert(b.name.to_lowercase());
            }
        }
    }
    out
}

impl Check for Draw48 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Per sheet, each note goes to the block nearest to it; a block that gets none is reported on the sheet of its
    /// heading. Parts of a block drawn on another sheet than its heading are left to that sheet.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let reach = nm(cx, &META, &BLOCK_REACH);
        let tie = nm(cx, &META, &BLOCK_TIE);
        let heading = nm(cx, &META, &HEADING_SIZE);
        let headed = headed(cx);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let all = blocks(scene);
            let notes = notes(scene, heading);
            let done = documented(&all, &notes, reach, tie);
            for (i, b) in all.iter().enumerate() {
                let away = b.heading.is_none() && headed.contains(&b.name.to_lowercase());
                if done.contains(&i) || away {
                    continue;
                }
                let parts = b.references.join(", ");
                let what = if b.heading.is_some() {
                    ""
                } else {
                    " and no heading"
                };
                let message = format!(
                    "block \"{}\" ({parts}) has no note{what}: nothing says what it does or why",
                    b.name
                );
                out.extend(finding(cx, &META, scene, b).map(|f| {
                    f.message(message)
                        .fix("a note box beside the block (Information, or See N.. for a note in the board's notes)")
                        .build()
                }));
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw48));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw48, "draw-48_block_notes"),
            [
                "DRAW-48:item block Charge pump|sheet /",
                "DRAW-48:item block Filter|sheet /",
            ]
        );
    }
}
