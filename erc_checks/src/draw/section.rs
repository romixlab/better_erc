//! The sections of a sheet: each heading with its parameter box and the parts nearest below and right of it, and
//! lines of things that should sit on one row or column

use super::{HEADING_SIZE, SECTION_REACH, nm};
use crate::Ctx;
use crate::meta::CheckMeta;
use erc_analysis::scene::{Item, ItemKind as K, Scene};
use erc_model::Pt;
use erc_model::geometry::Rect;
use std::collections::BTreeMap;

/// A heading with what it heads
pub(super) struct Section<'s> {
    /// The heading's text
    pub title: String,
    pub heading: &'s Item,
    /// Parts, power symbols and labels, wires left out: they run between sections
    pub items: Vec<&'s Item>,
    /// The box around the heading, its parameter box and the items
    pub rect: Rect,
}

/// The heading of the sections that may hold a thing at `p`: above it, and left of it within the reach; the nearest
fn heading_of(heads: &[&Item], p: Pt, reach: i64) -> Option<usize> {
    let above = |h: &&&Item| h.rect.min.y <= p.y && h.rect.min.x - reach <= p.x;
    let by = |h: &&&Item| (p.x - h.rect.min.x).abs() + (p.y - h.rect.min.y).abs();
    let near = heads.iter().filter(above).min_by_key(by)?;
    heads.iter().position(|h| std::ptr::eq(*h, *near))
}

/// The sheet's sections, in file order; a symbol's things go with its body
pub(super) fn sections<'s>(cx: &Ctx, meta: &CheckMeta, scene: &'s Scene) -> Vec<Section<'s>> {
    let size = nm(cx, meta, &HEADING_SIZE);
    let reach = nm(cx, meta, &SECTION_REACH);
    let texts = scene.items.iter().filter(|i| i.kind == K::Text);
    let heads: Vec<(&Item, &str)> = texts
        .zip(&scene.geometry.texts)
        .filter(|(_, t)| t.text.bold && t.text.size.0 == size)
        .map(|(i, t)| (i, t.text.text.as_str()))
        .collect();
    let items: Vec<&Item> = heads.iter().map(|(i, _)| *i).collect();
    let mut out: Vec<Section> = heads
        .iter()
        .map(|(item, title)| {
            let boxes = scene.items.iter().filter(|o| o.kind == K::TextBox);
            let own = boxes.filter(|o| scene.grouped(item, o));
            Section {
                title: title.lines().next().unwrap_or("").to_string(),
                heading: item,
                items: Vec::new(),
                rect: own.fold(item.rect, |r, o| r.union(o.rect)),
            }
        })
        .collect();
    let mut of_symbol: BTreeMap<usize, Option<usize>> = BTreeMap::new();
    for item in &scene.items {
        if matches!(item.kind, K::Body | K::Power) {
            let home = heading_of(&items, item.rect.center(), reach);
            of_symbol.extend(item.owner.map(|o| (o, home)));
        }
    }
    for item in &scene.items {
        let home = match item.kind {
            K::Body | K::Pin | K::Power | K::Field => item.owner.and_then(|o| of_symbol.get(&o)),
            K::Label => None,
            _ => continue,
        };
        let home = match (item.kind, home) {
            (K::Label, _) => heading_of(&items, item.rect.center(), reach),
            (_, h) => h.copied().flatten(),
        };
        if let Some(i) = home {
            out[i].rect = out[i].rect.union(item.rect);
            out[i].items.push(item);
        }
    }
    out
}

/// A row or column with the things that fall just off it
pub(super) struct Line<'s> {
    /// Where the line is
    pub at: i64,
    pub off: Vec<(i64, &'s Item)>,
}

/// The lines in a set of positions: values closer than `snap`, chained, form one group, whose line is the value most
/// of them share, on a tie the lowest with `low_wins`, else the highest. A group all on one line has no finding.
pub(super) fn line_up<'s>(members: &[(i64, &'s Item)], snap: i64, low_wins: bool) -> Vec<Line<'s>> {
    let mut sorted = members.to_vec();
    sorted.sort_by_key(|(v, _)| *v);
    let mut groups: Vec<Vec<(i64, &Item)>> = Vec::new();
    for m in sorted {
        match groups.last_mut() {
            Some(g) if m.0 - g[g.len() - 1].0 < snap => g.push(m),
            _ => groups.push(vec![m]),
        }
    }
    let mut out = Vec::new();
    for group in groups {
        let mut count: BTreeMap<i64, usize> = BTreeMap::new();
        for (v, _) in &group {
            *count.entry(*v).or_default() += 1;
        }
        if count.len() < 2 {
            continue;
        }
        let most = *count.values().max().unwrap_or(&0);
        let tied = count.iter().filter(|(_, c)| **c == most).map(|(v, _)| *v);
        let at = if low_wins { tied.min() } else { tied.max() };
        let Some(at) = at else { continue };
        let off = group.into_iter().filter(|(v, _)| *v != at).collect();
        out.push(Line { at, off });
    }
    out
}
