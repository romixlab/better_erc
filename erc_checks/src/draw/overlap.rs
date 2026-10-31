//! The pairs of items that matter to `DRAW-7`, `DRAW-9` and `DRAW-34`, with what each check exempts

use super::{
    CLEARANCE_PART, CLEARANCE_POWER, CLEARANCE_TEXT, at, location, near_point, nm, sheet_subject,
};
use crate::meta::CheckMeta;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene, mm};
use std::collections::BTreeMap;

/// Whether a pair can never be a finding: two wires, two pins, two things of one symbol as the library drew them,
/// a wire on the thing it connects to, a label along its own wire or on its own pin, two things grouped on purpose (a
/// note heading and its box)
fn exempt(scene: &Scene, a: &Item, b: &Item) -> bool {
    if (a.kind.is_wire() && b.kind.is_wire()) || (a.kind == K::Pin && b.kind == K::Pin) {
        return true;
    }
    if own(a, b) && a.kind != K::Field && b.kind != K::Field {
        return true;
    }
    if scene.grouped(a, b) {
        return true;
    }
    let joined = a.tie.is_some() && a.tie == b.tie;
    let (first, second) = if a.kind.is_wire() { (a, b) } else { (b, a) };
    match (first.kind, second.kind) {
        (K::Wire, K::Pin | K::Power)
        | (K::Label, K::Pin | K::Power)
        | (K::Pin | K::Power, K::Label) => joined,
        (K::Wire, K::Body) => on_part(scene, first, second),
        (K::Wire, K::Label) => joined && along(scene, first, second),
        _ => false,
    }
}

/// Whether the wire `w` is tied to a pin of the part `body`
fn on_part(scene: &Scene, w: &Item, body: &Item) -> bool {
    w.tie
        .zip(body.owner)
        .is_some_and(|(tie, part)| scene.ties[tie].pins.iter().any(|p| p.symbol == part))
}

/// Whether one of the two is wired to the symbol the other belongs to; its fields do not count, they are placed by
/// hand and can be in the way of the part's own wires
fn tied(scene: &Scene, a: &Item, b: &Item) -> bool {
    let on = |t: &Item, o: &Item| {
        let symbol_thing = !matches!(o.kind, K::Field | K::Sheet | K::SheetPin);
        symbol_thing
            && t.tie
                .zip(o.owner)
                .is_some_and(|(tie, part)| scene.ties[tie].pins.iter().any(|p| p.symbol == part))
    };
    on(a, b) || on(b, a)
}

/// Whether the wire `w` runs the way the label's text does
fn along(scene: &Scene, w: &Item, label: &Item) -> bool {
    let labels = &scene.geometry.labels;
    labels
        .iter()
        .find(|l| l.uuid == label.uuid)
        .is_some_and(|l| l.runs.is_horizontal() == (w.rect.height() == 0))
}

/// Two things of one symbol, or of one sheet symbol
fn own(a: &Item, b: &Item) -> bool {
    let sheet = |i: &Item| matches!(i.kind, K::Sheet | K::SheetPin);
    a.owner.is_some() && a.owner == b.owner && sheet(a) == sheet(b)
}

/// A field wholly inside its own body, or a caption wholly inside a sheet box, sits in free room; on the edge it
/// collides with the outline
fn in_free_room(a: &Item, b: &Item) -> bool {
    let inside = |t: &Item, r: &Item| {
        let room = (t.kind == K::Field && r.kind == K::Body && own(t, r))
            || (t.kind == K::Text && r.kind == K::Sheet);
        room && r.rect.contains(t.rect.min) && r.rect.contains(t.rect.max)
    };
    inside(a, b) || inside(b, a)
}

/// Pairs of items by a test, each pair of things once; the wires of one tie count as one wire, keeping the piece
/// with the largest overlap
fn pairs<'s>(
    scene: &'s Scene<'_>,
    mut hit: impl FnMut(&Item, &Item) -> Option<i64>,
) -> Vec<(&'s Item, &'s Item, i64)> {
    let mut found: BTreeMap<(usize, usize), (&Item, &Item, i64)> = BTreeMap::new();
    let key = |i: usize, item: &Item| {
        if item.kind == K::Wire {
            // Past every item index, one key per tie
            scene.items.len() + item.tie.unwrap_or(0)
        } else {
            i
        }
    };
    for (i, a) in scene.items.iter().enumerate() {
        for (j, b) in scene.items.iter().enumerate().skip(i + 1) {
            if exempt(scene, a, b) {
                continue;
            }
            let Some(score) = hit(a, b) else { continue };
            let k = (key(i, a), key(j, b));
            let k = (k.0.min(k.1), k.0.max(k.1));
            let e = found.entry(k).or_insert((a, b, score));
            if score > e.2 {
                *e = (a, b, score);
            }
        }
    }
    found.into_values().collect()
}

/// `DRAW-7`: a wire through a body, a power symbol or a pin stub it is not on. `DRAW-9`: every other bare overlap
pub(super) fn bare<'s>(scene: &'s Scene, wire_through: bool) -> Vec<(&'s Item, &'s Item, i64)> {
    pairs(scene, |a, b| {
        let by_wire =
            (a.kind.is_wire() && !b.kind.is_text()) || (b.kind.is_wire() && !a.kind.is_text());
        if by_wire != wire_through || in_free_room(a, b) {
            return None;
        }
        let i = a.rect.intersection(b.rect)?;
        Some(i.width().max(1) * i.height().max(1))
    })
}

fn margin(cx: &Ctx, meta: &CheckMeta, i: &Item) -> i64 {
    match i.kind {
        K::Body | K::Pin | K::Sheet => nm(cx, meta, &CLEARANCE_PART),
        K::Power => nm(cx, meta, &CLEARANCE_POWER),
        K::Wire | K::Bus => 0,
        _ => nm(cx, meta, &CLEARANCE_TEXT),
    }
}

/// `DRAW-34`: boxes that overlap only with their margins; the score is how far they fall short, closest first
pub(super) fn close<'s>(
    cx: &Ctx,
    meta: &CheckMeta,
    scene: &'s Scene,
) -> Vec<(&'s Item, &'s Item, i64)> {
    let mut found = pairs(scene, |a, b| {
        if a.rect.overlaps(b.rect) || own(a, b) || tied(scene, a, b) {
            return None;
        }
        let (ma, mb) = (margin(cx, meta, a), margin(cx, meta, b));
        if !a.rect.grown(ma).overlaps(b.rect.grown(mb)) {
            return None;
        }
        Some(ma + mb - a.rect.gap(b.rect))
    });
    found.sort_by_key(|p| std::cmp::Reverse(p.2));
    found
}

/// A finding on the pair, naming both with their positions
pub(super) fn finding(
    cx: &Ctx,
    meta: &'static CheckMeta,
    scene: &Scene,
    a: &Item,
    b: &Item,
    what: String,
    fix: &str,
) -> Finding {
    let sheet = sheet_subject(cx, scene);
    let (pa, pb) = (near_point(a, b.rect), near_point(b, a.rect));
    cx.finding(meta)
        .subject(sheet)
        .item(sheet, a.name.clone())
        .item(sheet, b.name.clone())
        .location(location(scene, a, pa))
        .location(location(scene, b, pb))
        .message(format!("{} {what} {}", at(a, pa), at(b, pb)))
        .fix(fix)
        .build()
}

/// `0.42 mm`
pub(super) fn gap_text(nm: i64) -> String {
    format!("{} mm", mm(nm))
}
