//! The functional blocks of a sheet: the symbols that share a `Block` field value, the heading text that names them,
//! and the notes that stand nearest to them

use super::{location, sheet_subject};
use crate::Ctx;
use crate::finding::FindingBuilder;
use crate::meta::CheckMeta;
use erc_analysis::scene::{Item, ItemKind as K, Scene};
use erc_model::geometry::Rect;
use std::collections::BTreeMap;

/// The symbol field that names the block a part belongs to
pub(super) const BLOCK_FIELD: &str = "Block";

/// A block as drawn on one sheet
pub(super) struct Block<'s> {
    /// The `Block` field value
    pub name: String,
    /// The bodies and pins of its symbols
    pub parts: Vec<&'s Item>,
    /// The references of its symbols, in file order, each once
    pub references: Vec<String>,
    /// The text on this sheet that reads as the block's name
    pub heading: Option<&'s Item>,
}

impl Block<'_> {
    /// How far a box is from the block: the gap to its nearest part or to its heading
    pub fn gap(&self, r: Rect) -> i64 {
        let parts = self.parts.iter().map(|p| p.rect.gap(r));
        let heading = self.heading.map(|h| h.rect.gap(r));
        parts.chain(heading).min().unwrap_or(i64::MAX)
    }

    /// Where a finding on the block points: its heading, else its first part
    pub fn anchor(&self) -> Option<&Item> {
        self.heading.or_else(|| self.parts.first().copied())
    }
}

/// A finding on a block of a sheet, by its name, located at its heading or first part; `None` for a block with
/// neither on the sheet
pub(super) fn finding<'c, 'a>(
    cx: &'c Ctx<'a>,
    meta: &'static CheckMeta,
    scene: &Scene,
    block: &Block,
) -> Option<FindingBuilder<'c, 'a>> {
    let anchor = block.anchor()?;
    let sheet = sheet_subject(cx, scene);
    Some(
        cx.finding(meta)
            .subject(sheet)
            .item(sheet, format!("block {}", block.name))
            .location(location(scene, anchor, anchor.rect.center())),
    )
}

/// Whether two names are the same block name: case and surrounding space ignored
pub(super) fn same_name(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// A text's first line
pub(super) fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("").trim()
}

/// The blocks of a sheet, in the order their first symbol is drawn; a heading counts when its first line is the block
/// name
pub(super) fn blocks<'s>(scene: &'s Scene) -> Vec<Block<'s>> {
    let g = scene.geometry;
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut out: Vec<Block> = Vec::new();
    let mut of_symbol: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, s) in g.symbols.iter().enumerate() {
        if s.power {
            continue;
        }
        let field = s.fields.iter().find(|f| f.name == BLOCK_FIELD);
        let Some(name) = field.map(|f| f.text.text.trim()).filter(|n| !n.is_empty()) else {
            continue;
        };
        let key = name.to_lowercase();
        let b = *index.entry(key).or_insert_with(|| {
            out.push(Block {
                name: name.to_string(),
                parts: Vec::new(),
                references: Vec::new(),
                heading: None,
            });
            out.len() - 1
        });
        of_symbol.insert(i, b);
        if !out[b].references.contains(&s.reference) {
            out[b].references.push(s.reference.clone());
        }
    }
    for item in &scene.items {
        if !matches!(item.kind, K::Body | K::Pin) {
            continue;
        }
        if let Some(b) = item.owner.and_then(|o| of_symbol.get(&o)) {
            out[*b].parts.push(item);
        }
    }
    let texts = scene.items.iter().filter(|i| i.kind == K::Text);
    for (item, t) in texts.zip(&g.texts) {
        let line = first_line(&t.text.text);
        if let Some(b) = out
            .iter_mut()
            .find(|b| b.heading.is_none() && same_name(&b.name, line))
        {
            b.heading = Some(item);
        }
    }
    out
}

/// Whether a text is a cost line: `$0.19 @25 · $0.12 @1k`, also after a label (`Sheet: $1.29 @25`)
pub(super) fn is_cost(text: &str) -> bool {
    let t = text.trim();
    let t = match t.split_once(':') {
        Some((label, rest)) if !label.contains('$') => rest.trim_start(),
        _ => t,
    };
    t.starts_with('$') && t.contains('@')
}

#[cfg(test)]
mod tests {
    use super::is_cost;

    #[test]
    fn cost_lines() {
        for t in [
            "$0.19 @25 · $0.12 @1k",
            "Sheet: $1.29 @25 · $0.829 @1k",
            "Board: $27.30 @25",
        ] {
            assert!(is_cost(t), "{t}");
        }
        for t in ["Costs $5 at most", "$VAR", "Information:\n--\nGain = 2"] {
            assert!(!is_cost(t), "{t}");
        }
    }
}
