//! `DRAW-40` Power or tie wiring much longer than it needs, with a cheaper drawing at hand

use super::{MM, location, nm, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{PinRef, Scene, Tie, mm};
use erc_model::Pt;
use erc_model::geometry::{Dir, DrawnPin, Rect};
use std::collections::BTreeMap;

pub struct Draw40;

const fn weight(name: &'static str, default: f64, doc: &'static str) -> Param {
    Param {
        name,
        default,
        unit: "",
        doc,
    }
}

const W_LEN: Param = weight("drawing.route.w_len", 1.0, "cost per step of wire");
const W_BEND: Param = weight("drawing.route.w_bend", 2.0, "cost per bend");
const W_CROSS: Param = weight(
    "drawing.route.w_cross",
    4.0,
    "cost per crossing with another net",
);
const W_CROSS_NEAR: Param = weight(
    "drawing.route.w_cross_near",
    8.0,
    "cost of a crossing within one step of a pin end or a junction",
);
const W_SYMBOL: Param = weight(
    "drawing.route.w_symbol",
    6.0,
    "cost per power or ground symbol beyond the first of that rail",
);
const W_DETOUR: Param = weight(
    "drawing.route.w_detour",
    4.0,
    "cost per unit of drawn length over Manhattan length, less one",
);
const DETOUR_MAX: Param = weight(
    "drawing.route.detour_max",
    1.5,
    "drawn length over Manhattan length above which a cheaper drawing is looked for",
);
const STEP: Param = Param {
    name: "drawing.route.step",
    default: 2.54,
    unit: "mm",
    doc: "the grid step wire is counted in",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-40",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Hard,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[
        W_LEN,
        W_BEND,
        W_CROSS,
        W_CROSS_NEAR,
        W_SYMBOL,
        W_DETOUR,
        DETOUR_MAX,
        STEP,
    ],
};

/// The routing cost's terms, read once per run
struct Weights {
    len: f64,
    bend: f64,
    cross: f64,
    near: f64,
    symbol: f64,
    detour: f64,
    max: f64,
    step: i64,
}

impl Weights {
    fn of(cx: &Ctx) -> Weights {
        let w = |p: &Param| cx.param(&META, p.name);
        Weights {
            len: w(&W_LEN),
            bend: w(&W_BEND),
            cross: w(&W_CROSS),
            near: w(&W_CROSS_NEAR),
            symbol: w(&W_SYMBOL),
            detour: w(&W_DETOUR),
            max: w(&DETOUR_MAX),
            step: nm(cx, &META, &STEP),
        }
    }
}

/// A piece of a drawing: straight wire between two points
type Seg = (Pt, Pt);

fn length(segs: &[Seg]) -> i64 {
    let one = |(a, b): &Seg| (a.x - b.x).abs() + (a.y - b.y).abs();
    segs.iter().map(one).sum()
}

/// What crossing the wires of other ties costs a drawing that belongs to tie `own`
fn crossings(scene: &Scene, w: &Weights, own: usize, segs: &[Seg]) -> f64 {
    let g = scene.geometry;
    let tied = |i: usize| scene.ties[own].wires.contains(&i);
    let others = g.wires.iter().enumerate().filter(|(i, _)| !tied(*i));
    let mut cost = 0.0;
    for (_, o) in others {
        for s in segs {
            let Some(p) = crossing(*s, (o.a, o.b)) else {
                continue;
            };
            let close = |q: Pt| (q.x - p.x).abs() + (q.y - p.y).abs() <= w.step;
            let pins = g.symbols.iter().flat_map(|s| &s.pins).map(|p| p.at);
            let near = pins.chain(g.junctions.iter().copied()).any(close);
            cost += if near { w.near } else { w.cross };
        }
    }
    cost
}

/// Where a horizontal and a vertical piece cross, each past the other's ends
fn crossing(a: Seg, b: Seg) -> Option<Pt> {
    let flat = |s: Seg| s.0.y == s.1.y && s.0.x != s.1.x;
    let upright = |s: Seg| s.0.x == s.1.x && s.0.y != s.1.y;
    let (h, v) = if flat(a) && upright(b) {
        (a, b)
    } else if flat(b) && upright(a) {
        (b, a)
    } else {
        return None;
    };
    let (x, y) = (v.0.x, h.0.y);
    let inside = |p: i64, q: i64, r: i64| p.min(q) < r && r < p.max(q);
    (inside(h.0.x, h.1.x, x) && inside(v.0.y, v.1.y, y)).then_some(Pt::new(x, y))
}

/// Bends of a drawn tie: points where exactly two wires end, across each other
fn bends(scene: &Scene, tie: &Tie) -> usize {
    let mut ends: BTreeMap<Pt, Vec<bool>> = BTreeMap::new();
    for w in tie.wires.iter().map(|w| &scene.geometry.wires[*w]) {
        for p in [w.a, w.b] {
            ends.entry(p).or_default().push(w.a.x == w.b.x);
        }
    }
    ends.values()
        .filter(|e| e.len() == 2 && e[0] != e[1])
        .count()
}

/// Pins of one side of one symbol, in order along that side
struct Group<'s> {
    reference: &'s str,
    side: Dir,
    pins: Vec<&'s DrawnPin>,
}

fn groups<'s>(scene: &Scene<'s>, tie: &Tie) -> Vec<Group<'s>> {
    let g = scene.geometry;
    let mut by: BTreeMap<(usize, Dir), Vec<&'s DrawnPin>> = BTreeMap::new();
    let parts = tie.pins.iter().filter(|p| !g.symbols[p.symbol].power);
    for PinRef { symbol, pin } in parts {
        let p = &g.symbols[*symbol].pins[*pin];
        by.entry((*symbol, p.outward)).or_default().push(p);
    }
    by.into_iter()
        .map(|((symbol, side), mut pins)| {
            pins.sort_by_key(|p| if side.is_horizontal() { p.at.y } else { p.at.x });
            Group {
                reference: &g.symbols[symbol].reference,
                side,
                pins,
            }
        })
        .collect()
}

impl Group<'_> {
    fn name(&self) -> String {
        let numbers: Vec<&str> = self.pins.iter().map(|p| p.number.as_str()).collect();
        format!("{}.{}", self.reference, numbers.join(","))
    }

    /// One step out of each pin, and the wire that collects them; the collector's two ends
    fn collect(&self, step: i64) -> (Vec<Seg>, Pt, Pt) {
        let (dx, dy) = self.side.step();
        let out = |p: &DrawnPin| Pt::new(p.at.x + dx * step, p.at.y + dy * step);
        let segs: Vec<Seg> = self.pins.iter().map(|p| (p.at, out(p))).collect();
        let (a, b) = (out(self.pins[0]), out(self.pins[self.pins.len() - 1]));
        let collector = (a != b).then_some((a, b));
        (segs.into_iter().chain(collector).collect(), a, b)
    }

    /// Dropped straight to a symbol of its own in `to`; `None` when the pins face away from it
    fn drop_to(&self, to: Dir, step: i64) -> Option<(Vec<Seg>, usize)> {
        if self.side == to.opposite() {
            return None;
        }
        let (mut segs, a, b) = self.collect(step);
        let (dx, dy) = to.step();
        let further = |p: Pt| p.x * dx + p.y * dy;
        let from = if further(a) >= further(b) { a } else { b };
        segs.push((from, Pt::new(from.x + dx * step, from.y + dy * step)));
        let straight = self.side == to && self.pins.len() == 1;
        Some((segs, if straight { 0 } else { 1 }))
    }
}

/// The two L paths between two points
fn ells(a: Pt, b: Pt) -> [Vec<Seg>; 2] {
    let bent = |c: Pt| vec![(a, c), (c, b)];
    [bent(Pt::new(a.x, b.y)), bent(Pt::new(b.x, a.y))]
}

/// A candidate drawing, what it would cost and how to say it
struct Redraw {
    cost: f64,
    what: String,
}

/// Each pin group straight down its own way to a symbol of its own
fn straight_drops(
    scene: &Scene,
    w: &Weights,
    tie: usize,
    gs: &[Group],
    to: Dir,
    rail: &str,
) -> Option<Redraw> {
    let mut cost = w.symbol * (gs.len() - 1) as f64;
    for g in gs {
        let (segs, bent) = g.drop_to(to, w.step)?;
        let steps = length(&segs) as f64 / w.step as f64;
        cost += w.len * steps + w.bend * bent as f64 + crossings(scene, w, tie, &segs);
    }
    let names: Vec<String> = gs.iter().map(Group::name).collect();
    Some(Redraw {
        cost,
        what: format!("straight drop from {} to {rail}", names.join(" and ")),
    })
}

/// The groups joined by the shortest L each to the symbol, or to the first group where the tie has none
fn joined(
    scene: &Scene,
    w: &Weights,
    tie: usize,
    gs: &[Group],
    target: Pt,
    rail: &str,
    span: i64,
) -> Redraw {
    let mut cost = 0.0;
    let mut wire = 0;
    for g in gs {
        let (mut segs, a, b) = g.collect(w.step);
        let dist = |p: Pt| (p.x - target.x).abs() + (p.y - target.y).abs();
        let from = if dist(a) <= dist(b) { a } else { b };
        let score = |path: &Vec<Seg>| crossings(scene, w, tie, path);
        let [p, q] = ells(from, target);
        let best = if score(&p) <= score(&q) { p } else { q };
        let bent = usize::from(from.x != target.x && from.y != target.y);
        cost += w.bend * bent as f64 + score(&best);
        segs.extend(best);
        cost += crossings(scene, w, tie, &segs[..segs.len() - 2]);
        wire += length(&segs);
    }
    cost += w.len * wire as f64 / w.step as f64;
    cost += w.detour * (wire as f64 / span.max(1) as f64 - 1.0).max(0.0);
    let at = format!("x {} y {}", target.x as f64 / MM, target.y as f64 / MM);
    Redraw {
        cost,
        what: format!("a direct path to {rail} at {at}"),
    }
}

/// The tie's scored drawing and its cheapest candidate, when the tie is one this check reads: wires between pins
/// and a power symbol, or pins of one part tied together. A tie with a label goes on elsewhere and has no known end.
fn judge(scene: &Scene, w: &Weights, t: usize) -> Option<(f64, Redraw, f64)> {
    let g = scene.geometry;
    let tie = &scene.ties[t];
    let powers: Vec<usize> = tie
        .pins
        .iter()
        .filter(|p| g.symbols[p.symbol].power)
        .map(|p| p.symbol)
        .collect();
    let gs = groups(scene, tie);
    let same_part = gs.iter().all(|x| x.reference == gs[0].reference);
    let readable = !gs.is_empty()
        && tie.pins.len() >= 2
        && tie.wires.len() >= 2
        && (!powers.is_empty() || (same_part && tie.labels.is_empty()));
    if !readable {
        return None;
    }
    let ends = tie.pins.iter().map(|p| g.symbols[p.symbol].pins[p.pin].at);
    let bounds = Rect::around(ends)?;
    let span = bounds.width() + bounds.height();
    let segs: Vec<Seg> = tie
        .wires
        .iter()
        .map(|i| (g.wires[*i].a, g.wires[*i].b))
        .collect();
    let drawn = length(&segs);
    let ratio = drawn as f64 / span.max(1) as f64;
    if span == 0 || ratio <= w.max {
        return None;
    }
    let cost = w.len * drawn as f64 / w.step as f64
        + w.bend * bends(scene, tie) as f64
        + crossings(scene, w, t, &segs)
        + w.symbol * powers.len().saturating_sub(1) as f64
        + w.detour * (ratio - 1.0);
    let power = powers.first().map(|s| &g.symbols[*s]);
    let rail = power.map_or("the first pin", |s| s.value.as_str());
    let target = power
        .and_then(|s| s.pins.first())
        .map_or(gs[0].pins[0].at, |p| p.at);
    let mut best = joined(scene, w, t, &gs, target, rail, span);
    let to = power
        .and_then(|s| s.pins.first())
        .map(|p| p.outward.opposite());
    let drop = to.and_then(|d| straight_drops(scene, w, t, &gs, d, rail));
    if let Some(o) = drop.filter(|o| o.cost < best.cost) {
        best = o;
    }
    (best.cost < cost).then_some((cost, best, ratio))
}

impl Check for Draw40 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Scores a tie that runs longer than the detour limit, and the two ways to draw it again: pin groups dropped
    /// to symbols of their own, or joined by L paths to the existing symbol. A suggestion only: the candidates
    /// ignore parts that would have to move.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let w = Weights::of(cx);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let sheet = sheet_subject(cx, scene);
            for t in 0..scene.ties.len() {
                let Some((cost, best, ratio)) = judge(scene, &w, t) else {
                    continue;
                };
                let wire = scene.ties[t].wires[0];
                let item = scene
                    .items
                    .iter()
                    .find(|i| i.uuid == scene.geometry.wires[wire].uuid);
                let Some(item) = item else { continue };
                let pins = groups(scene, &scene.ties[t]);
                let names: Vec<String> = pins.iter().map(Group::name).collect();
                let at = scene.geometry.wires[wire].a;
                out.push(
                    cx.finding(&META)
                        .subject(sheet)
                        .item(sheet, format!("{} ({})", item.name, names.join(", ")))
                        .location(location(scene, item, at))
                        .message(format!(
                            "{} at x {} y {} runs {ratio:.1} times its Manhattan length: {} costs {:.0} instead of {:.0}",
                            item.name,
                            mm(at.x),
                            mm(at.y),
                            best.what,
                            best.cost,
                            cost
                        ))
                        .fix("redraw the tie the cheaper way")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw40));

#[cfg(test)]
mod tests {
    use super::{Pt, crossing};

    #[test]
    fn a_crossing_is_past_both_ends() {
        let h = (Pt::new(0, 5), Pt::new(10, 5));
        assert_eq!(
            crossing(h, (Pt::new(5, 0), Pt::new(5, 10))),
            Some(Pt::new(5, 5))
        );
        assert_eq!(crossing(h, (Pt::new(5, 5), Pt::new(5, 10))), None);
        assert_eq!(crossing(h, (Pt::new(0, 0), Pt::new(0, 10))), None);
    }

    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw40, "draw-40_tie_detour"),
            ["DRAW-40:item wire of GND (U1.5,6,7)|sheet /"]
        );
    }
}
