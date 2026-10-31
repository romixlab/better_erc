//! `DRAW-40` Power or tie wiring much longer than it needs, with a cheaper drawing at hand

use super::{MM, location, nm, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::power::{NameRole, read_name};
use erc_analysis::scene::{PinRef, Scene, Tie, mm};
use erc_model::Pt;
use erc_model::geometry::{Dir, DrawnPin, DrawnSymbol, Rect};
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
const K_POWER: Param = weight(
    "drawing.route.k_power",
    2.0,
    "bends and detours of a power tie count this many times, and a bent one is looked at whatever its detour",
);
const K_GROUND: Param = weight(
    "drawing.route.k_ground",
    2.0,
    "bends and detours of a ground tie count this many times, and a bent one is looked at whatever its detour",
);
const K_SIGNAL: Param = weight(
    "drawing.route.k_signal",
    1.0,
    "bends and detours of a signal tie count this many times",
);
const MIN_SAVING: Param = weight(
    "drawing.route.min_saving",
    2.0,
    "cost a redraw must save to be worth a finding",
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
        K_POWER,
        K_GROUND,
        K_SIGNAL,
        MIN_SAVING,
        STEP,
    ],
};

/// The routing cost's terms, read once per run
#[derive(Clone)]
struct Weights {
    len: f64,
    bend: f64,
    cross: f64,
    near: f64,
    symbol: f64,
    detour: f64,
    max: f64,
    k_power: f64,
    k_ground: f64,
    k_signal: f64,
    min_saving: f64,
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
            k_power: w(&K_POWER),
            k_ground: w(&K_GROUND),
            k_signal: w(&K_SIGNAL),
            min_saving: w(&MIN_SAVING),
            step: nm(cx, &META, &STEP),
        }
    }

    /// The weights for a tie of this rail, `None` for a signal: its class's bends and detours count more
    fn of_class(&self, rail: Option<&str>) -> Weights {
        let k = match rail.map(|r| read_name(r, false).role) {
            None => self.k_signal,
            Some(NameRole::Ground(_)) => self.k_ground,
            Some(_) => self.k_power,
        };
        Weights {
            bend: self.bend * k,
            detour: self.detour * k,
            ..self.clone()
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
fn bends(scene: &Scene, wires: &[usize]) -> usize {
    let mut ends: BTreeMap<Pt, Vec<bool>> = BTreeMap::new();
    for w in wires.iter().map(|w| &scene.geometry.wires[*w]) {
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

    /// Dropped straight to a symbol of its own in `to`, level with `row`, so the symbol keeps the section's row when
    /// the pins are short of it; `None` when the pins face away from it
    fn drop_to(&self, to: Dir, row: Pt, step: i64) -> Option<(Vec<Seg>, usize)> {
        if self.side == to.opposite() {
            return None;
        }
        let (mut segs, a, b) = self.collect(step);
        let (dx, dy) = to.step();
        let further = |p: Pt| p.x * dx + p.y * dy;
        let from = if further(a) >= further(b) { a } else { b };
        let run = (further(row) - further(from)).max(step);
        segs.push((from, Pt::new(from.x + dx * run, from.y + dy * run)));
        let straight = self.side == to && self.pins.len() == 1;
        Some((segs, if straight { 0 } else { 1 }))
    }
}

/// The way a piece of wire runs
fn heading((a, b): Seg) -> Option<Dir> {
    match ((b.x - a.x).signum(), (b.y - a.y).signum()) {
        (1, 0) => Some(Dir::Right),
        (-1, 0) => Some(Dir::Left),
        (0, 1) => Some(Dir::Down),
        (0, -1) => Some(Dir::Up),
        _ => None,
    }
}

/// The two L paths between two points, without their empty legs
fn ells(a: Pt, b: Pt) -> [Vec<Seg>; 2] {
    let bent = |c: Pt| {
        let legs = [(a, c), (c, b)];
        legs.into_iter().filter(|(p, q)| p != q).collect()
    };
    [bent(Pt::new(a.x, b.y)), bent(Pt::new(b.x, a.y))]
}

/// The bends of a path out of a pin stub running `side`, when the path keeps `DRAW-41`: it does not turn back over
/// the stub, and it reaches the symbol, when `into` says which way that takes, along the symbol's axis with a lead
fn lead_kept(path: &[Seg], side: Dir, into: Option<Dir>, step: i64) -> Option<usize> {
    let first = heading(*path.first()?)?;
    if first == side.opposite() {
        return None;
    }
    let last = *path.last()?;
    let enters = into.is_none_or(|d| heading(last) == Some(d) && length(&[last]) >= step);
    enters.then_some(path.len() - 1 + usize::from(first != side))
}

/// A candidate drawing, what it would cost and how to say it
struct Redraw {
    cost: f64,
    what: String,
}

/// What a candidate is scored against
struct Site<'r, 's> {
    scene: &'r Scene<'s>,
    w: &'r Weights,
    tie: usize,
    /// The Manhattan length of the tie
    span: i64,
}

impl Site<'_, '_> {
    /// Whether a piece would land on another net's wire: a crossing is fine, a shared point joins the nets
    fn blocked(&self, segs: &[Seg]) -> bool {
        let g = self.scene.geometry;
        let tied = &self.scene.ties[self.tie].wires;
        let others = g
            .wires
            .iter()
            .enumerate()
            .filter(|(i, _)| !tied.contains(i));
        let on = |(a, b): Seg, p: Pt| {
            let r = Rect::of(a, b);
            r.contains(p) && (r.width() == 0 || r.height() == 0)
        };
        others.into_iter().any(|(_, o)| {
            segs.iter()
                .any(|s| on((o.a, o.b), s.0) || on((o.a, o.b), s.1) || on(*s, o.a) || on(*s, o.b))
        })
    }

    /// Length and detour of a redraw with this much wire
    fn wire_cost(&self, wire: i64) -> f64 {
        let w = self.w;
        w.len * wire as f64 / w.step as f64
            + w.detour * (wire as f64 / self.span.max(1) as f64 - 1.0).max(0.0)
    }
}

/// Each pin group straight down its own way to a symbol of its own, on the row of the tie's symbol
fn straight_drops(site: &Site, gs: &[Group], to: Dir, row: Pt, rail: &str) -> Option<Redraw> {
    let w = site.w;
    let mut cost = w.symbol * (gs.len() - 1) as f64;
    let mut wire = 0;
    for g in gs {
        let (segs, bent) = g.drop_to(to, row, w.step)?;
        if site.blocked(&segs) {
            return None;
        }
        wire += length(&segs);
        cost += w.bend * bent as f64 + crossings(site.scene, w, site.tie, &segs);
    }
    let names: Vec<String> = gs.iter().map(Group::name).collect();
    Some(Redraw {
        cost: cost + site.wire_cost(wire),
        what: format!("straight drop from {} to {rail}", names.join(" and ")),
    })
}

/// The groups joined by the shortest L each to the symbol, or to the first group where the tie has none; `None` when
/// no L keeps the pin lead at both ends
fn joined(site: &Site, gs: &[Group], target: Pt, into: Option<Dir>, rail: &str) -> Option<Redraw> {
    let (scene, w, tie) = (site.scene, site.w, site.tie);
    let mut cost = 0.0;
    let mut wire = 0;
    for g in gs {
        let (mut segs, a, b) = g.collect(w.step);
        let dist = |p: Pt| (p.x - target.x).abs() + (p.y - target.y).abs();
        let from = if dist(a) <= dist(b) { a } else { b };
        let scored = |path: Vec<Seg>| {
            let bent = lead_kept(&path, g.side, into, w.step)?;
            let blocked = site.blocked(&path) || site.blocked(&segs);
            let cost = w.bend * bent as f64 + crossings(scene, w, tie, &path);
            (!blocked).then_some((cost, path))
        };
        let (path_cost, path) = ells(from, target)
            .into_iter()
            .filter_map(scored)
            .min_by(|p, q| p.0.total_cmp(&q.0))?;
        cost += path_cost + crossings(scene, w, tie, &segs);
        // The collector's far end is a corner
        cost += w.bend * usize::from(g.pins.len() > 1) as f64;
        segs.extend(path);
        wire += length(&segs);
    }
    let at = format!("x {} y {}", target.x as f64 / MM, target.y as f64 / MM);
    Some(Redraw {
        cost: cost + site.wire_cost(wire),
        what: format!("a direct path to {rail} at {at}"),
    })
}

/// Whether a symbol only marks its net for KiCad's ERC: it stands at no end of a route
fn is_flag(s: &DrawnSymbol) -> bool {
    s.value == "PWR_FLAG"
}

/// The tie's wires without the stubs that lead only to a `PWR_FLAG`: from each flag along wires that nothing else
/// joins, up to the first joint or pin
fn routed_wires(scene: &Scene, tie: &Tie) -> Vec<usize> {
    let g = scene.geometry;
    let mut wires = tie.wires.clone();
    let pin_at = |p: &PinRef| g.symbols[p.symbol].pins[p.pin].at;
    let others: Vec<Pt> = tie
        .pins
        .iter()
        .filter(|p| !is_flag(&g.symbols[p.symbol]))
        .map(pin_at)
        .collect();
    let flags = tie.pins.iter().filter(|p| is_flag(&g.symbols[p.symbol]));
    for start in flags.map(pin_at) {
        let mut at = start;
        loop {
            let on: Vec<usize> = wires
                .iter()
                .copied()
                .filter(|w| g.wires[*w].contains(at))
                .collect();
            let [only] = on[..] else { break };
            let w = &g.wires[only];
            if at != w.a && at != w.b {
                break;
            }
            wires.retain(|x| *x != only);
            at = if at == w.a { w.b } else { w.a };
            if others.contains(&at) {
                break;
            }
        }
    }
    wires
}

/// The tie's scored drawing and its cheapest candidate, when the tie is one this check reads: wires between pins
/// and a power symbol, or pins of one part tied together. A tie with a label goes on elsewhere and has no known end.
/// Power and ground ties are looked at with any bend, a signal only past the detour limit: signals may bend more.
fn judge(scene: &Scene, w: &Weights, t: usize) -> Option<(f64, Redraw, f64)> {
    let g = scene.geometry;
    let tie = &scene.ties[t];
    let powers: Vec<usize> = tie
        .pins
        .iter()
        .filter(|p| g.symbols[p.symbol].power && !is_flag(&g.symbols[p.symbol]))
        .map(|p| p.symbol)
        .collect();
    let gs = groups(scene, tie);
    let wires = routed_wires(scene, tie);
    let same_part = gs.iter().all(|x| x.reference == gs[0].reference);
    let readable = !gs.is_empty()
        && gs.iter().map(|x| x.pins.len()).sum::<usize>() + powers.len() >= 2
        && wires.len() >= 2
        && (!powers.is_empty() || (same_part && tie.labels.is_empty()));
    if !readable {
        return None;
    }
    let ends = tie
        .pins
        .iter()
        .filter(|p| !is_flag(&g.symbols[p.symbol]))
        .map(|p| g.symbols[p.symbol].pins[p.pin].at);
    let bounds = Rect::around(ends)?;
    let span = bounds.width() + bounds.height();
    let segs: Vec<Seg> = wires
        .iter()
        .map(|i| (g.wires[*i].a, g.wires[*i].b))
        .collect();
    let drawn = length(&segs);
    let ratio = drawn as f64 / span.max(1) as f64;
    let power = powers.first().map(|s| &g.symbols[*s]);
    let w = &w.of_class(power.map(|s| s.value.as_str()));
    let bent = bends(scene, &wires);
    if span == 0 || (ratio <= w.max && (power.is_none() || bent == 0)) {
        return None;
    }
    let cost = w.len * drawn as f64 / w.step as f64
        + w.bend * bent as f64
        + crossings(scene, w, t, &segs)
        + w.symbol * powers.len().saturating_sub(1) as f64
        + w.detour * (ratio - 1.0);
    let rail = power.map_or("the first pin", |s| s.value.as_str());
    let pin = power.and_then(|s| s.pins.first());
    let target = pin.map_or(gs[0].pins[0].at, |p| p.at);
    // A wire reaches a symbol against the way it leaves the symbol's pin
    let to = pin.map(|p| p.outward.opposite());
    let site = Site {
        scene,
        w,
        tie: t,
        span,
    };
    let direct = joined(&site, &gs, target, to, rail);
    let drop = to.and_then(|d| straight_drops(&site, &gs, d, target, rail));
    let best = [direct, drop]
        .into_iter()
        .flatten()
        .min_by(|a, b| a.cost.total_cmp(&b.cost))?;
    (best.cost + w.min_saving <= cost).then_some((cost, best, ratio))
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

    #[test]
    fn power_tie_bent_sideways() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw40, "draw-40_power_sideways"),
            ["DRAW-40:item wire of +5V (R1.1)|sheet /"]
        );
    }
}
