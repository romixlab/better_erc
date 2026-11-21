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

/// Where a symbol's fields go
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// A column on the right of the body
    Right,
    Left,
    Above,
    Below,
    /// A two-pin part packed against its neighbours: Reference before the body and Value after it along the
    /// part's axis, the other fields hidden
    Along,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Right => "right",
            Side::Left => "left",
            Side::Above => "above",
            Side::Below => "below",
            Side::Along => "along the part",
        }
    }
}

/// One field moved
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMove {
    /// The field's name (`Value`)
    pub field: String,
    /// The anchor: the text is centred across its reading direction on it
    pub at: Pt,
    /// Reads upwards, not left to right
    pub vertical: bool,
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
    /// Where its fields go; `None` when no place is better than where they are
    pub side: Option<Side>,
    /// Empty when the fields stay
    pub moves: Vec<FieldMove>,
    /// The shown fields to hide, by name
    pub hidden: Vec<String>,
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
    /// Items no longer drawn: fields hidden
    gone: Vec<bool>,
    /// By symbol: its fields are in the way of something and the placer has not been to it yet
    pending: Vec<bool>,
    margins: Margins,
    /// Inside the drawing frame; `None` when the paper's size is not known
    frame: Option<Rect>,
}

/// A layout to try: where it puts each field it shows (by index into the shown fields), and which it hides
struct Layout {
    side: Side,
    lines: Vec<(usize, Pt, bool, HAlign, Rect)>,
    hidden: Vec<usize>,
}

impl Board<'_, '_> {
    /// The conflicts of a symbol's fields at new boxes `placed` (item index and box), against every other item but
    /// the symbol's fields `own`, against each other and the frame; with the names of what they hit when `names`.
    /// For a new column a wire through the gap between two lines counts too: a person never stacks fields across
    /// a wire.
    fn cost(
        &self,
        own: &[usize],
        placed: &[(usize, Rect)],
        column: bool,
        names: Option<&mut Vec<(String, String)>>,
    ) -> Cost {
        let mut cost = Cost::default();
        let mut hits = Vec::new();
        let placed: Vec<Item> = placed
            .iter()
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
                if self.gone[j]
                    || !near.touches(b.rect)
                    || own.contains(&j)
                    || exempt(self.scene, a, b)
                {
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
        let hull = placed.iter().map(|p| p.rect).reduce(Rect::union);
        if let (true, Some(hull), Some(first)) = (column && placed.len() > 1, hull, placed.first())
        {
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

    /// Whether any of the boxes overlaps, or comes within the margins of, the body of another symbol or a field
    /// of one whose fields are settled
    fn hits_neighbour(&self, s: usize, field: &Item, rects: &[Rect]) -> bool {
        let neighbour = |j: usize, b: &Item| {
            let other = b.owner.filter(|o| *o != s);
            !self.gone[j]
                && other.is_some_and(|o| match b.kind {
                    K::Body => true,
                    K::Field => !self.pending[o],
                    _ => false,
                })
        };
        rects.iter().any(|r| {
            let a = Item {
                rect: *r,
                ..field.clone()
            };
            self.items.iter().enumerate().any(|(j, b)| {
                neighbour(j, b)
                    && !exempt(self.scene, &a, b)
                    && (bare_hit(&a, b, false).is_some()
                        || close_hit(self.scene, self.margins, &a, b).is_some())
            })
        })
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
        gone: vec![false; scene.items.len()],
        pending: vec![false; g.symbols.len()],
        margins,
        frame,
    };
    for s in 0..g.symbols.len() {
        let own: Vec<usize> = (0..board.items.len())
            .filter(|i| board.items[*i].owner == Some(s) && board.items[*i].kind == K::Field)
            .collect();
        let shown: Vec<(usize, Rect)> = own.iter().map(|i| (*i, board.items[*i].rect)).collect();
        board.pending[s] = board.cost(&own, &shown, false, None).pairs > 0;
    }
    let mut plans: Vec<Option<SymbolPlan>> = vec![None; g.symbols.len()];
    for _ in 0..PASSES_MAX {
        let mut changed = false;
        for (s, sym) in g.symbols.iter().enumerate() {
            if !sym.power && place_symbol(&mut board, s, sym, &mut plans[s]) {
                changed = true;
            }
            board.pending[s] = false;
        }
        if !changed {
            break;
        }
    }
    out.extend(plans.into_iter().flatten());
}

/// Moves the fields of symbol `s` when they conflict with something and another layout conflicts less: along a
/// packed two-pin part, else a column beside the body. `plan` keeps the last move and what is still in the way.
/// Whether the fields moved.
fn place_symbol(
    board: &mut Board,
    s: usize,
    sym: &DrawnSymbol,
    plan: &mut Option<SymbolPlan>,
) -> bool {
    let own: Vec<usize> = (0..board.items.len())
        .filter(|i| board.items[*i].owner == Some(s) && board.items[*i].kind == K::Field)
        .collect();
    let shown_now: Vec<(usize, Rect)> = own
        .iter()
        .filter(|i| !board.gone[**i])
        .map(|i| (*i, board.items[*i].rect))
        .collect();
    // A column this placer made is held to its own rule about wires through it
    // A part laid along stays so: its hidden fields are gone, and a column would have to show them again
    if plan.as_ref().is_some_and(|p| p.side == Some(Side::Along)) {
        return false;
    }
    let column = plan.as_ref().is_some_and(|p| p.side.is_some());
    let now = board.cost(&own, &shown_now, column, None);
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
    let in_order: Vec<&DrawnField> = order.iter().map(|k| shown[*k]).collect();
    let columns: Vec<Layout> = candidates(sym, &in_order)
        .into_iter()
        .map(|(side, places)| Layout {
            side,
            lines: order
                .iter()
                .zip(places)
                .map(|(k, (at, halign, rect))| (*k, at, false, halign, rect))
                .collect(),
            hidden: Vec::new(),
        })
        .collect();
    let rects = |l: &Layout| -> Vec<(usize, Rect)> {
        l.lines
            .iter()
            .map(|(k, _, _, _, r)| (own[*k], *r))
            .collect()
    };
    let scored = |l: Layout| {
        let column = l.side != Side::Along;
        (board.cost(&own, &rects(&l), column, None), l)
    };
    let packed = packed_layout(board, s, sym, &shown, &own, &columns);
    let columns: Vec<Layout> = columns;
    // A column that fits wins; else a packed part takes the layout along it or none, a column beside it would
    // only meet its neighbours
    let column_best = columns.into_iter().map(scored).min_by_key(|c| c.0);
    let best = match (column_best, packed) {
        (Some(c), _) if c.0.pairs == 0 => Some(c),
        (_, Some(l)) => Some(scored(l)),
        (c, None) => c,
    };
    let p = plan.get_or_insert_with(|| SymbolPlan {
        file: board.scene.geometry.file.clone(),
        reference: sym.reference.clone(),
        uuid: sym.uuid.clone(),
        side: None,
        moves: Vec::new(),
        hidden: Vec::new(),
        left: Vec::new(),
    });
    match best {
        Some((cost, l)) if cost < now => {
            let placed = rects(&l);
            board.cost(&own, &placed, l.side != Side::Along, Some(&mut p.left));
            for (i, r) in &placed {
                board.items[*i].rect = *r;
                board.gone[*i] = false;
            }
            for k in &l.hidden {
                board.gone[own[*k]] = true;
            }
            p.side = Some(l.side);
            p.hidden = l.hidden.iter().map(|k| shown[*k].name.clone()).collect();
            p.moves = l
                .lines
                .iter()
                .map(|(k, at, vertical, halign, rect)| FieldMove {
                    field: shown[*k].name.clone(),
                    at: *at,
                    vertical: *vertical,
                    halign: *halign,
                    rect: *rect,
                })
                .collect();
            true
        }
        _ => {
            board.cost(&own, &shown_now, column, Some(&mut p.left));
            false
        }
    }
}

/// Reference prefixes of the two-pin parts that are laid out along their axis when packed: resistors,
/// capacitors, diodes and LEDs, inductors, ferrite beads
const TWO_PIN_PREFIXES: &[&str] = &["R", "C", "D", "LED", "LD", "L", "FB"];

/// Whether the symbol is a two-pin part, and lying horizontally
fn two_pin(sym: &DrawnSymbol) -> Option<bool> {
    let prefix: String = sym
        .reference
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    if !TWO_PIN_PREFIXES.contains(&prefix.as_str()) {
        return None;
    }
    let pins: Vec<_> = sym.pins.iter().filter(|p| !p.hidden).collect();
    let [a, b] = pins.as_slice() else { return None };
    let (ha, hb) = (a.outward.is_horizontal(), b.outward.is_horizontal());
    (ha == hb && a.outward != b.outward).then_some(ha)
}

/// For a two-pin part packed against its neighbours (a column on either side across its axis would hit another
/// part's body or settled fields; used when no column fits cleanly): Reference before the body and Value after it, reading along the part on the line beside its axis,
/// every other field hidden. `None` for any other symbol, or one without Reference and Value shown.
fn packed_layout(
    board: &Board,
    s: usize,
    sym: &DrawnSymbol,
    shown: &[&DrawnField],
    own: &[usize],
    columns: &[Layout],
) -> Option<Layout> {
    let horizontal = two_pin(sym)?;
    let reference = shown.iter().position(|f| f.name == "Reference")?;
    let value = shown.iter().position(|f| f.name == "Value")?;
    let across = if horizontal {
        [Side::Above, Side::Below]
    } else {
        [Side::Right, Side::Left]
    };
    let field = &board.items[own[reference]];
    let outline = sym.outline();
    let blocked = across.iter().any(|side| {
        // The first candidate of each side is the column centred on the body; its lines next to the body tell
        columns.iter().find(|l| l.side == *side).is_some_and(|l| {
            let rects: Vec<Rect> = l.lines.iter().map(|line| line.4).collect();
            let next: Vec<Rect> = match side {
                Side::Above => rects.last().copied().into_iter().collect(),
                Side::Below => rects.first().copied().into_iter().collect(),
                _ => rects
                    .into_iter()
                    .filter(|r| r.min.y < outline.max.y && r.max.y > outline.min.y)
                    .collect(),
            };
            board.hits_neighbour(s, field, &next)
        })
    });
    if !blocked {
        return None;
    }
    let body = sym.body.unwrap_or_else(|| sym.outline());
    let middle = body.center();
    let line = |k: usize, at: Pt, halign: HAlign| {
        let rect = text_rect(
            at,
            !horizontal,
            shown[k].text.extent(),
            halign,
            VAlign::Center,
        );
        (k, at, !horizontal, halign, rect)
    };
    // One step off the axis, so the text sits beside the part's own wire, not on it
    let lines = if horizontal {
        let y = nearest(middle.y) - STEP;
        vec![
            line(
                reference,
                Pt::new(down(body.min.x - BODY_GAP), y),
                HAlign::Right,
            ),
            line(value, Pt::new(up(body.max.x + BODY_GAP), y), HAlign::Left),
        ]
    } else {
        // A vertical text reads upwards: Left starts at the anchor and runs up
        let x = nearest(middle.x) - STEP;
        vec![
            line(
                reference,
                Pt::new(x, down(body.min.y - BODY_GAP)),
                HAlign::Left,
            ),
            line(value, Pt::new(x, up(body.max.y + BODY_GAP)), HAlign::Right),
        ]
    };
    let hidden = (0..shown.len())
        .filter(|k| *k != reference && *k != value)
        .collect();
    Some(Layout {
        side: Side::Along,
        lines,
        hidden,
    })
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
            Side::Along => unreachable!("a column stands on one of the four sides"),
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
