//! The pairs of items that matter to `DRAW-7`, `DRAW-9` and `DRAW-34`, with what each check exempts

use super::{
    CLEARANCE_PART, CLEARANCE_POWER, CLEARANCE_TEXT, at, location, near_point, nm, sheet_subject,
};
use crate::meta::CheckMeta;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene, mm};
use erc_model::geometry::{Rect, Shape};
use std::borrow::Cow;
use std::collections::BTreeMap;

/// Whether a pair can never be a finding: two wires, two pins, two things of one symbol as the library drew them,
/// a wire on the thing it connects to, a label along its own wire or on its own pin, two things grouped on purpose (a
/// note heading and its box)
pub(super) fn exempt(scene: &Scene, a: &Item, b: &Item) -> bool {
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

/// Two text boxes side by side with a shared edge: one unit in two columns
fn one_unit(a: &Item, b: &Item) -> bool {
    a.kind == K::TextBox && b.kind == K::TextBox && a.rect.touches(b.rect)
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
    pairs(scene, |a, b| bare_hit(a, b, wire_through))
}

/// What of `a` counts against `b`: a text box wholly around `b` only its text, a body its drawn shapes, anything
/// else its box; `None` when nothing does
fn outline<'a>(a: &'a Item, b: &Item) -> Option<Cow<'a, [Shape]>> {
    if a.kind == K::TextBox && a.rect.encloses(b.rect) {
        return a.text.map(|t| Cow::Owned(vec![Shape::Box(t)]));
    }
    if a.shapes.is_empty() {
        Some(Cow::Owned(vec![Shape::Box(a.rect)]))
    } else {
        Some(Cow::Borrowed(&a.shapes))
    }
}

/// How far apart the two items are, as [`Shape::separation`] measures it, when it is under `limit`: negative when
/// they share area. The boxes go first: they are never farther apart than what is drawn in them.
fn separation(a: &Item, b: &Item, limit: i64) -> Option<i64> {
    if a.rect.separation(b.rect) >= limit {
        return None;
    }
    let (sa, sb) = (outline(a, b)?, outline(b, a)?);
    let mut best = i64::MAX;
    for x in sa.iter() {
        let bx = x.bbox();
        for y in sb.iter() {
            if bx.separation(y.bbox()) < best.min(limit) {
                best = best.min(x.separation(y));
            }
        }
    }
    (best < limit).then_some(best)
}

/// The shared area of a pair that is not exempt, for `DRAW-7` when `wire_through`, else for `DRAW-9`
pub(super) fn bare_hit(a: &Item, b: &Item, wire_through: bool) -> Option<i64> {
    let by_wire =
        (a.kind.is_wire() && !b.kind.is_text()) || (b.kind.is_wire() && !a.kind.is_text());
    if by_wire != wire_through || in_free_room(a, b) {
        return None;
    }
    separation(a, b, 0)?;
    let reach = |x: &Item, y: &Item| outline(x, y)?.iter().map(Shape::bbox).reduce(Rect::union);
    let shared = reach(a, b)?.intersection(reach(b, a)?);
    Some(shared.map_or(1, |i| i.width().max(1) * i.height().max(1)))
}

/// The clearance margins of `DRAW-34`, in nanometres
#[derive(Clone, Copy, Debug)]
pub(super) struct Margins {
    pub part: i64,
    pub power: i64,
    pub text: i64,
}

impl Margins {
    pub(super) fn of(cx: &Ctx, meta: &CheckMeta) -> Margins {
        Margins {
            part: nm(cx, meta, &CLEARANCE_PART),
            power: nm(cx, meta, &CLEARANCE_POWER),
            text: nm(cx, meta, &CLEARANCE_TEXT),
        }
    }

    fn around(self, i: &Item) -> i64 {
        match i.kind {
            K::Body | K::Pin | K::Sheet => self.part,
            K::Power => self.power,
            K::Wire | K::Bus => 0,
            _ => self.text,
        }
    }
}

/// `DRAW-34`: boxes that overlap only with their margins; the score is how far they fall short, closest first
pub(super) fn close<'s>(
    cx: &Ctx,
    meta: &CheckMeta,
    scene: &'s Scene,
) -> Vec<(&'s Item, &'s Item, i64)> {
    close_with(scene, Margins::of(cx, meta))
}

/// [`close`] with the margins given
pub(super) fn close_with<'s>(scene: &'s Scene, m: Margins) -> Vec<(&'s Item, &'s Item, i64)> {
    let mut found = pairs(scene, |a, b| close_hit(scene, m, a, b));
    found.sort_by_key(|p| std::cmp::Reverse(p.2));
    found
}

/// How far a pair that is not exempt falls short of its margins, when the boxes stay apart bare
pub(super) fn close_hit(scene: &Scene, m: Margins, a: &Item, b: &Item) -> Option<i64> {
    if own(a, b) || tied(scene, a, b) || one_unit(a, b) {
        return None;
    }
    let (ma, mb) = (m.around(a), m.around(b));
    let apart = separation(a, b, ma + mb)?;
    (apart >= 0).then_some(ma + mb - apart)
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

#[cfg(test)]
mod tests {
    use crate::draw::draw_9::Draw9;
    use crate::testing::fingerprints_of;

    /// A sheet with these library symbols and items
    fn sheet(items: &str) -> String {
        let field = |name: &str| {
            format!(
                "(property \"{name}\" \"\" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))"
            )
        };
        let fields = ["Reference", "Value", "Footprint", "Datasheet"]
            .map(field)
            .join("");
        let pin = |at: &str| {
            format!(
                "(pin passive line (at {at}) (length 2.54) (name \"~\" (effects (font (size 1.27 1.27)))) \
                 (number \"1\" (effects (font (size 1.27 1.27)))))"
            )
        };
        format!(
            "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\") \
             (uuid \"e9000000-0000-4000-8000-000000000000\") (paper \"A4\") (lib_symbols \
             (symbol \"Test:R\" (pin_numbers (hide yes)) (pin_names (hide yes)) {fields} \
             (symbol \"R_0_1\" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none)))) \
             (symbol \"R_1_1\" {} {})) \
             (symbol \"Test:Probe\" (pin_numbers (hide yes)) (pin_names (hide yes)) {fields} \
             (symbol \"Probe_0_1\" (polyline (pts (xy 0 0) (xy 0.5 0) (xy 3 2.5) (xy 3 3) (xy 2.5 3) (xy 0 0.5) (xy 0 0)) \
             (stroke (width 0) (type default)) (fill (type background)))) \
             (symbol \"Probe_1_1\" (pin passive line (at 0 0 90) (length 0) (name \"1\" (effects (font (size 1.27 1.27)))) \
             (number \"1\" (effects (font (size 1.27 1.27)))))))) \
             {items} (sheet_instances (path \"/\" (page \"1\"))))",
            pin("0 5.08 270"),
            pin("0 -5.08 90"),
        )
    }

    /// A placed symbol of `lib` with reference `reference` at `x`, `y`
    fn symbol(lib: &str, reference: &str, x: f64, y: f64, n: u32) -> String {
        format!(
            "(symbol (lib_id \"Test:{lib}\") (at {x} {y} 0) (unit 1) (in_bom yes) (on_board yes) \
             (uuid \"e9000000-0000-4000-8000-0000000001{n:02}\") \
             (property \"Reference\" \"{reference}\" (at {x} {y} 0) (effects (font (size 1.27 1.27)) (hide yes))) \
             (property \"Value\" \"x\" (at {x} {y} 0) (effects (font (size 1.27 1.27)) (hide yes))) \
             (instances (project \"t\" (path \"/e9000000-0000-4000-8000-000000000000\" (reference \"{reference}\") (unit 1)))))"
        )
    }

    /// A text box `Note` with its corner at `x`, `y`
    fn text_box(x: f64, y: f64, w: f64, h: f64) -> String {
        format!(
            "(text_box \"Note\" (exclude_from_sim no) (at {x} {y} 0) (size {w} {h}) \
             (stroke (width 0.1524) (type dot)) (fill (type none)) \
             (effects (font (size 1.27 1.27)) (justify left top)) (uuid \"e9000000-0000-4000-8000-000000000200\"))"
        )
    }

    #[test]
    fn bodies_count_by_what_they_draw() {
        // Diagonal bars 2.54 mm apart: their boxes overlap, the bars do not
        let apart = sheet(
            &(symbol("Probe", "TP1", 50.0, 50.0, 1) + &symbol("Probe", "TP2", 52.54, 50.0, 2)),
        );
        assert_eq!(fingerprints_of(&Draw9, &apart), Vec::<String>::new());
        let onto = sheet(
            &(symbol("Probe", "TP1", 50.0, 50.0, 1) + &symbol("Probe", "TP2", 50.3, 50.0, 2)),
        );
        assert_eq!(
            fingerprints_of(&Draw9, &onto),
            [
                "DRAW-9:item TP1 body|item TP2 body|sheet /",
                "DRAW-9:item TP1 body|item TP2.1 pin|sheet /",
            ]
        );
    }

    #[test]
    fn a_text_box_around_parts_counts_by_its_text() {
        let parts = symbol("R", "R1", 60.0, 60.0, 1) + &symbol("R", "R2", 70.0, 60.0, 2);
        let around = sheet(&(parts.clone() + &text_box(55.0, 50.0, 20.0, 20.0)));
        assert_eq!(fingerprints_of(&Draw9, &around), Vec::<String>::new());
        // The border through the middle of both bodies
        let across = sheet(&(parts + &text_box(55.0, 60.0, 20.0, 10.0)));
        assert_eq!(
            fingerprints_of(&Draw9, &across),
            [
                "DRAW-9:item R1 body|item text box \"Note\"|sheet /",
                "DRAW-9:item R2 body|item text box \"Note\"|sheet /",
            ]
        );
    }
}
