//! `DRAW-36` Near-miss alignment: label anchors, power symbols, tie verticals or part edges on one side of a part
//! or within a window, off a shared column or row by more than zero and less than the snap distance

use super::{ALIGN_SNAP, ALIGN_WINDOW, location, nm, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene, mm};
use erc_model::Pt;
use erc_model::geometry::Rect;
use std::collections::BTreeMap;

pub struct Draw36;

static META: CheckMeta = CheckMeta {
    id: "DRAW-36",
    category: "DRAW",
    severity: Severity::Warning,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &["DRAW-10"],
    params: &[ALIGN_WINDOW, ALIGN_SNAP],
};

/// An axis things line up on
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Axis {
    /// A column: x is compared
    X,
    /// A row: y is compared
    Y,
}

impl Axis {
    fn of(self, p: Pt) -> i64 {
        match self {
            Axis::X => p.x,
            Axis::Y => p.y,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
        }
    }

    /// How far apart the boxes are across the axis: negative when they run alongside each other
    fn apart(self, a: Rect, b: Rect) -> i64 {
        match self {
            Axis::X => (a.min.y - b.max.y).max(b.min.y - a.max.y),
            Axis::Y => (a.min.x - b.max.x).max(b.min.x - a.max.x),
        }
    }
}

/// What may line up with what: labels with labels, power symbols with power symbols, tie wires with tie wires,
/// parts with parts
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Label,
    Power,
    Wire,
    Part,
}

/// A thing that can line up, with the lines it offers on an axis
struct Liner<'s> {
    item: &'s Item,
    class: Class,
    /// Where the lines come from: an anchor has one, a body its two edges, its centre and its pins, compared like
    /// with like
    lines: Vec<(u8, Pt)>,
}

/// The kind of line a pin of a part gives. Parts of different sizes with pins on one line are lined up, so a shared
/// pin line clears the pair; pins off each other say nothing, a pin pitch is no near miss.
const PIN_LINE: u8 = 3;

/// The kind of line a body's centre gives. Bodies of different sizes cannot have both edges on one line, so an edge
/// agreeing clears the pair but only their centres can miss.
const CENTRE_LINE: u8 = 1;

fn liners<'s>(scene: &'s Scene, axis: Axis) -> Vec<Liner<'s>> {
    let g = scene.geometry;
    let mut out = Vec::new();
    let mut labels = g.labels.iter();
    for item in &scene.items {
        let lines = match item.kind {
            K::Label => {
                let l = labels.next().expect("one item per label");
                (Class::Label, vec![(0, l.at)])
            }
            K::Power => {
                let s = item.owner.map(|o| &g.symbols[o]);
                let at = s.and_then(|s| s.pins.first()).map(|p| p.at);
                (Class::Power, vec![(0, at.unwrap_or(item.rect.center()))])
            }
            K::Wire => {
                let across = match axis {
                    Axis::X => item.rect.width(),
                    Axis::Y => item.rect.height(),
                };
                if across != 0 {
                    continue;
                }
                (Class::Wire, vec![(0, item.rect.min)])
            }
            K::Body => {
                let r = item.rect;
                let mut lines = vec![(0, r.min), (CENTRE_LINE, r.center()), (2, r.max)];
                let pins = item.owner.iter().flat_map(|o| &g.symbols[*o].pins);
                lines.extend(pins.map(|p| (PIN_LINE, p.at)));
                (Class::Part, lines)
            }
            _ => continue,
        };
        out.push(Liner {
            item,
            class: lines.0,
            lines: lines.1,
        });
    }
    out
}

/// The smallest non-zero offset between two liners' lines, when no pair of their lines agrees exactly
fn miss(axis: Axis, a: &Liner, b: &Liner, snap: i64) -> Option<(i64, i64, i64)> {
    let span = |l: &Liner| match axis {
        Axis::X => l.item.rect.width(),
        Axis::Y => l.item.rect.height(),
    };
    let like = a.class != Class::Part || span(a) == span(b);
    let mut best = None;
    for (ka, pa) in &a.lines {
        for (_, pb) in b.lines.iter().filter(|(kb, _)| kb == ka) {
            let (va, vb) = (axis.of(*pa), axis.of(*pb));
            let off = (va - vb).abs();
            if off == 0 {
                return None;
            }
            let counts = *ka == CENTRE_LINE || (*ka != PIN_LINE && like);
            if counts && off < snap && best.is_none_or(|(o, _, _)| off < o) {
                best = Some((off, va, vb));
            }
        }
    }
    best
}

/// The parts each label, power symbol or wire is tied to, by symbol and the side its pin leaves the body
fn sides(scene: &Scene, item: &Item) -> Vec<(usize, erc_model::geometry::Dir)> {
    let Some(tie) = item.tie else {
        return Vec::new();
    };
    let g = scene.geometry;
    let parts = scene.ties[tie].pins.iter();
    parts
        .filter(|p| !g.symbols[p.symbol].power)
        .map(|p| (p.symbol, g.symbols[p.symbol].pins[p.pin].outward))
        .collect()
}

/// Whether two wires both end on pins of one symbol: they sit at its pin pitch, nothing to line up
fn one_symbol(scene: &Scene, a: &Item, b: &Item) -> bool {
    let symbols = |i: &Item| {
        let g = scene.geometry;
        let tie = i.tie.map(|t| &scene.ties[t]);
        let pins = tie.into_iter().flat_map(|t| &t.pins);
        pins.filter(|p| !g.symbols[p.symbol].power)
            .map(|p| p.symbol)
            .collect::<Vec<_>>()
    };
    let (sa, sb) = (symbols(a), symbols(b));
    sa.iter().any(|s| sb.contains(s))
}

impl Check for Draw36 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Two liners of one class in line with each other, not alongside: neighbours by the window or by being on
    /// the same side of one part. Parallel wires and parts side by side are spacing, not a missed column. Of the
    /// wires of one tie, the pair with the smallest offset speaks for the rest.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let window = nm(cx, &META, &ALIGN_WINDOW);
        let snap = nm(cx, &META, &ALIGN_SNAP);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for axis in [Axis::X, Axis::Y] {
                let liners = liners(scene, axis);
                let mut found: BTreeMap<(&str, &str), (&Liner, &Liner, i64, i64, i64)> =
                    BTreeMap::new();
                for (i, a) in liners.iter().enumerate() {
                    for b in &liners[i + 1..] {
                        if a.class != b.class || a.item.name == b.item.name {
                            continue;
                        }
                        let apart = axis.apart(a.item.rect, b.item.rect);
                        if apart < 0 {
                            continue;
                        }
                        if a.class == Class::Wire && one_symbol(scene, a.item, b.item) {
                            continue;
                        }
                        let same_side = {
                            let sb = sides(scene, b.item);
                            sides(scene, a.item).iter().any(|s| sb.contains(s))
                        };
                        if apart > window && !same_side {
                            continue;
                        }
                        let Some((off, va, vb)) = miss(axis, a, b, snap) else {
                            continue;
                        };
                        let key = (a.item.name.as_str(), b.item.name.as_str());
                        let key = (key.0.min(key.1), key.0.max(key.1));
                        let e = found.entry(key).or_insert((a, b, off, va, vb));
                        if off < e.2 {
                            *e = (a, b, off, va, vb);
                        }
                    }
                }
                let sheet = sheet_subject(cx, scene);
                for (a, b, off, va, vb) in found.into_values() {
                    let (pa, pb) = (a.lines[0].1, b.lines[0].1);
                    let n = axis.name();
                    out.push(
                        cx.finding(&META)
                            .subject(sheet)
                            .item(sheet, a.item.name.clone())
                            .item(sheet, b.item.name.clone())
                            .location(location(scene, a.item, pa))
                            .location(location(scene, b.item, pb))
                            .message(format!(
                                "{} at {n} {}, {} at {n} {}: {} mm",
                                a.item.name,
                                mm(va),
                                b.item.name,
                                mm(vb),
                                mm(off)
                            ))
                            .fix("line them up, or move one clearly away")
                            .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw36));
