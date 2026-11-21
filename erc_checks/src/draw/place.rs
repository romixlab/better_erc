//! `TOOL-13`: the fix side of `DRAW-9` and `DRAW-34` for symbol fields. Each symbol whose shown fields overlap
//! something, or come closer than its margin, gets them stacked in one column beside its body, on the side where
//! the column hits nothing, judged by the same pair tests the two checks use.

use super::draw_34;
use super::overlap::{Margins, bare_hit, close_hit, exempt};
use crate::Ctx;
use erc_analysis::scene::{Item, ItemKind as K, Scene};
use erc_model::Pt;
use erc_model::geometry::{DrawnField, DrawnSymbol, HAlign, Rect, VAlign, text_rect};

/// The step fields are placed on: half of the 50 mil grid
const STEP: i64 = 1_270_000;
/// Space between two lines of a column, past the text's height, before rounding up to [`STEP`]
const LINE_GAP: i64 = 500_000;
/// Least space between a column and the symbol's body and pins
const BODY_GAP: i64 = 635_000;
/// Margin of the drawing frame inside the paper edge, as KiCad's default page layout draws it
const FRAME: i64 = 10_000_000;
/// How many steps a column may slide along its side when it does not fit centred
const SLIDE_STEPS_MAX: i64 = 4;
/// How many steps farther out from the body a column may stand when it does not fit next to it
const OUT_STEPS_MAX: i64 = 2;

/// The side of the body a column of fields stands on
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
    Above,
    Below,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Right => "right",
            Side::Left => "left",
            Side::Above => "above",
            Side::Below => "below",
        }
    }
}

/// One field moved
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMove {
    /// The field's name (`Value`)
    pub field: String,
    /// The anchor: the text is drawn upright and vertically centred on it
    pub at: Pt,
    pub halign: HAlign,
    /// The text's box at its new place
    pub rect: Rect,
}

/// A symbol whose fields overlap something: where they go, or why they stay
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolPlan {
    /// The sheet file, relative to the project directory
    pub file: String,
    pub reference: String,
    /// The UUID of the placed symbol in its file
    pub uuid: String,
    /// The side its fields move to; `None` when no side is better than where they are
    pub side: Option<Side>,
    /// Empty when the fields stay
    pub moves: Vec<FieldMove>,
    /// What the fields still overlap or come too close to, where they end up: `R1 Value` and `C3 body`
    pub left: Vec<(String, String)>,
}

/// The fields of a design to move
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldPlan {
    pub symbols: Vec<SymbolPlan>,
}

/// The `DRAW-9` and `DRAW-34` pairs of a design, all and those with a field in them
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OverlapCount {
    pub draw_9: usize,
    pub draw_34: usize,
    pub draw_9_fields: usize,
    pub draw_34_fields: usize,
}

/// Whether the scene is the sheet named `sheet`: by its name in the parent or by its file, with or without the
/// extension; every scene without a name
fn on_sheet(cx: &Ctx, scene: &Scene, sheet: Option<&str>) -> bool {
    let Some(want) = sheet else { return true };
    let file = &scene.geometry.file;
    let stem = file.strip_suffix(".kicad_sch").unwrap_or(file);
    let name = scene
        .sheet
        .and_then(|s| cx.board().sheets.get(s.0 as usize))
        .map(|s| s.name.as_str());
    file == want || stem == want || name == Some(want)
}

/// Counts what `DRAW-9` and `DRAW-34` find on the board of `cx`, on one sheet or all
pub fn count_overlaps(cx: &Ctx, sheet: Option<&str>) -> OverlapCount {
    let m = draw_34::margins(cx);
    let mut out = OverlapCount::default();
    for scene in cx.scenes().iter().filter(|s| on_sheet(cx, s, sheet)) {
        let field = |a: &Item, b: &Item| a.kind == K::Field || b.kind == K::Field;
        for (a, b, _) in super::overlap::bare(scene, false) {
            out.draw_9 += 1;
            out.draw_9_fields += usize::from(field(a, b));
        }
        for (a, b, _) in super::overlap::close_with(scene, m) {
            out.draw_34 += 1;
            out.draw_34_fields += usize::from(field(a, b));
        }
    }
    out
}

/// Plans the fields of the board of `cx`, on one sheet or all. Symbols whose fields overlap nothing are left out;
/// power symbols, free text and everything that is not a field stay where they are.
pub fn plan_fields(cx: &Ctx, sheet: Option<&str>) -> FieldPlan {
    let m = draw_34::margins(cx);
    let mut plan = FieldPlan::default();
    for scene in cx.scenes().iter().filter(|s| on_sheet(cx, s, sheet)) {
        plan_scene(scene, m, &mut plan.symbols);
    }
    plan
}

/// The conflicts of a set of boxes: how many pairs, and their summed overlap, so fewer pairs win first
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Cost {
    pairs: usize,
    amount: i64,
}

/// A sheet as the placer changes it: the scene's items, the fields of moved symbols at their new places
struct Board<'s, 'a> {
    scene: &'s Scene<'a>,
    items: Vec<Item>,
    margins: Margins,
    /// Inside the drawing frame; `None` when the paper's size is not known
    frame: Option<Rect>,
}

impl Board<'_, '_> {
    /// The conflicts of fields `own` (indices into the items) with their boxes `rects`, against every other item,
    /// each other and the frame; with the names of what they hit when `names`. For a new column a wire through
    /// the gap between two lines counts too: a person never stacks fields across a wire.
    fn cost(
        &self,
        own: &[usize],
        rects: &[Rect],
        column: bool,
        names: Option<&mut Vec<(String, String)>>,
    ) -> Cost {
        let mut cost = Cost::default();
        let mut hits = Vec::new();
        let placed: Vec<Item> = own
            .iter()
            .zip(rects)
            .map(|(i, r)| Item {
                rect: *r,
                ..self.items[*i].clone()
            })
            .collect();
        let mut add = |a: &Item, b: &Item, amount: i64| {
            cost.pairs += 1;
            cost.amount += amount;
            hits.push((a.name.clone(), b.name.clone()));
        };
        // Nothing farther than both margins can be in the way
        let m = self.margins;
        let reach = m.part.max(m.power).max(m.text) + m.text;
        for (k, a) in placed.iter().enumerate() {
            let near = a.rect.grown(reach);
            for (j, b) in self.items.iter().enumerate() {
                if !near.touches(b.rect) || own.contains(&j) || exempt(self.scene, a, b) {
                    continue;
                }
                let hit =
                    bare_hit(a, b, false).or_else(|| close_hit(self.scene, self.margins, a, b));
                if let Some(amount) = hit {
                    add(a, b, amount);
                }
            }
            for b in &placed[k + 1..] {
                if let Some(amount) = bare_hit(a, b, false) {
                    add(a, b, amount);
                }
            }
            if let Some(frame) = self.frame
                && !(frame.contains(a.rect.min) && frame.contains(a.rect.max))
            {
                let edge = Item {
                    name: "the sheet edge".to_owned(),
                    ..a.clone()
                };
                add(a, &edge, a.rect.width() * a.rect.height());
            }
        }
        let hull = rects.iter().copied().reduce(Rect::union);
        if let (true, Some(hull), Some(first)) = (column && rects.len() > 1, hull, placed.first()) {
            let across = |w: &Item| {
                w.kind.is_wire()
                    && hull.overlaps(w.rect)
                    && !placed.iter().any(|f| f.rect.overlaps(w.rect))
            };
            for w in self.items.iter().filter(|w| across(w)) {
                let i = hull
                    .intersection(w.rect)
                    .expect("the hull overlaps the wire");
                add(first, w, i.width().max(1) * i.height().max(1));
            }
        }
        if let Some(names) = names {
            *names = hits;
        }
        cost
    }
}

/// How many times the placer goes over a sheet: a symbol whose fields found no room may find it once its
/// neighbours' fields have moved
const PASSES_MAX: usize = 3;

fn plan_scene(scene: &Scene, margins: Margins, out: &mut Vec<SymbolPlan>) {
    let g = scene.geometry;
    let (w, h) = g.paper_size;
    let frame = (w > 2 * FRAME && h > 2 * FRAME)
        .then(|| Rect::of(Pt::new(FRAME, FRAME), Pt::new(w - FRAME, h - FRAME)));
    let mut board = Board {
        scene,
        items: scene.items.clone(),
        margins,
        frame,
    };
    let mut plans: Vec<Option<SymbolPlan>> = vec![None; g.symbols.len()];
    for _ in 0..PASSES_MAX {
        let mut changed = false;
        for (s, sym) in g.symbols.iter().enumerate() {
            if !sym.power && place_symbol(&mut board, s, sym, &mut plans[s]) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    out.extend(plans.into_iter().flatten());
}

/// Moves the fields of symbol `s` when they conflict with something and a column beside the body conflicts
/// less; `plan` keeps the last move and what is still in the way. Whether the fields moved.
fn place_symbol(
    board: &mut Board,
    s: usize,
    sym: &DrawnSymbol,
    plan: &mut Option<SymbolPlan>,
) -> bool {
    let own: Vec<usize> = (0..board.items.len())
        .filter(|i| board.items[*i].owner == Some(s) && board.items[*i].kind == K::Field)
        .collect();
    let rects: Vec<Rect> = own.iter().map(|i| board.items[*i].rect).collect();
    let moved = plan.as_ref().is_some_and(|p| p.side.is_some());
    let now = board.cost(&own, &rects, moved, None);
    if now.pairs == 0 {
        return false;
    }
    let shown: Vec<&DrawnField> = sym
        .fields
        .iter()
        .filter(|f| !f.hidden && !f.text.text.is_empty())
        .collect();
    debug_assert_eq!(shown.len(), own.len(), "one field item per shown field");
    let order = column_order(&shown);
    let best = candidates(sym, &order.iter().map(|k| shown[*k]).collect::<Vec<_>>())
        .into_iter()
        .map(|(side, places)| {
            let mut rects = vec![Rect::default(); own.len()];
            for (n, k) in order.iter().enumerate() {
                rects[*k] = places[n].2;
            }
            (board.cost(&own, &rects, true, None), side, places, rects)
        })
        .min_by_key(|c| c.0);
    let p = plan.get_or_insert_with(|| SymbolPlan {
        file: board.scene.geometry.file.clone(),
        reference: sym.reference.clone(),
        uuid: sym.uuid.clone(),
        side: None,
        moves: Vec::new(),
        left: Vec::new(),
    });
    match best {
        Some((cost, side, places, rects)) if cost < now => {
            board.cost(&own, &rects, true, Some(&mut p.left));
            for (k, i) in own.iter().enumerate() {
                board.items[*i].rect = rects[k];
            }
            p.side = Some(side);
            p.moves = order
                .iter()
                .zip(places)
                .map(|(k, (at, halign, rect))| FieldMove {
                    field: shown[*k].name.clone(),
                    at,
                    halign,
                    rect,
                })
                .collect();
            true
        }
        _ => {
            board.cost(&own, &rects, moved, Some(&mut p.left));
            false
        }
    }
}

/// The fields in column order, by index into `shown`: Reference, Value, then the rest top to bottom as they
/// stand now
fn column_order(shown: &[&DrawnField]) -> Vec<usize> {
    let rank = |f: &DrawnField| match f.name.as_str() {
        "Reference" => 0,
        "Value" => 1,
        _ => 2,
    };
    let mut order: Vec<usize> = (0..shown.len()).collect();
    order.sort_by_key(|k| {
        let at = shown[*k].text.bbox.min;
        (rank(shown[*k]), at.y, at.x)
    });
    order
}

/// A column's anchors, alignments and boxes, line by line
type Column = Vec<(Pt, HAlign, Rect)>;

/// Every column to try, best first: centred right, left, above, below the body, then slid along each side, then
/// the same a step or two farther out
fn candidates(sym: &DrawnSymbol, fields: &[&DrawnField]) -> Vec<(Side, Column)> {
    let body = sym.outline();
    let extents: Vec<(i64, i64)> = fields.iter().map(|f| f.text.extent()).collect();
    let tallest = extents.iter().map(|e| e.1).max().unwrap_or(STEP);
    let pitch = up(tallest + LINE_GAP);
    let n = fields.len() as i64;
    let middle = body.center();
    let top_centred = nearest(middle.y - (n - 1) * pitch / 2);
    let column = |side: Side, slide: i64, out: i64| -> Column {
        let (x0, y0, halign) = match side {
            Side::Right => (
                up(body.max.x + BODY_GAP) + out,
                top_centred + slide,
                HAlign::Left,
            ),
            Side::Left => (
                down(body.min.x - BODY_GAP) - out,
                top_centred + slide,
                HAlign::Right,
            ),
            Side::Above => {
                let last = down(body.min.y - BODY_GAP - tallest / 2) - out;
                (
                    nearest(middle.x) + slide,
                    last - (n - 1) * pitch,
                    HAlign::Center,
                )
            }
            Side::Below => {
                let first = up(body.max.y + BODY_GAP + tallest / 2) + out;
                (nearest(middle.x) + slide, first, HAlign::Center)
            }
        };
        extents
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let at = Pt::new(x0, y0 + i as i64 * pitch);
                (at, halign, text_rect(at, false, *e, halign, VAlign::Center))
            })
            .collect()
    };
    let sides = [Side::Right, Side::Left, Side::Above, Side::Below];
    let mut all = Vec::new();
    for o in 0..=OUT_STEPS_MAX {
        for s in sides {
            all.push((s, column(s, 0, o * STEP)));
        }
        for k in 1..=SLIDE_STEPS_MAX {
            for s in sides {
                for slide in [k * STEP, -k * STEP] {
                    all.push((s, column(s, slide, o * STEP)));
                }
            }
        }
    }
    all
}

/// `v` rounded up to the step
fn up(v: i64) -> i64 {
    v.div_euclid(STEP) * STEP + if v.rem_euclid(STEP) == 0 { 0 } else { STEP }
}

/// `v` rounded down to the step
fn down(v: i64) -> i64 {
    v.div_euclid(STEP) * STEP
}

/// `v` rounded to the nearest step
fn nearest(v: i64) -> i64 {
    down(v + STEP / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding() {
        assert_eq!(up(1_270_001), 2_540_000);
        assert_eq!(up(-1_000_000), 0);
        assert_eq!(down(-1), -1_270_000);
        assert_eq!(nearest(1_900_000), 1_270_000);
        assert_eq!(nearest(1_905_000), 2_540_000);
    }
}
