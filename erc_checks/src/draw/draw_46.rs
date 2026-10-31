//! `DRAW-46` Leader into a small part off its centre, not diagonal, or clipped by the outline

use super::draw_35::small_part;
use super::{COLOUR_SLACK, near_colour, nm, one};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K, Scene, mm};
use erc_model::Pt;
use erc_model::geometry::{DrawnGraphic, Rect};

pub struct Draw46;

const CLEAR: Param = Param {
    name: "drawing.leader.clear",
    default: 0.635,
    unit: "mm",
    doc: "gap from a leader's tip to the part's outline",
};

const AIM: Param = Param {
    name: "drawing.leader.aim",
    default: 2.54,
    unit: "mm",
    doc: "how far from the body's centre the line of a leader's last segment may pass",
};

const ANGLE_SLACK: Param = Param {
    name: "drawing.leader.angle_slack",
    default: 3.0,
    unit: "degrees",
    doc: "how far a leader's last segment may be off 45 degrees",
};

const REACH: Param = Param {
    name: "drawing.leader.reach",
    default: 5.08,
    unit: "mm",
    doc: "how far from a leader's tip a part counts as what it points at",
};

const MAX_PINS: Param = Param {
    name: "max_pins",
    default: 4.0,
    unit: "",
    doc: "pins of the largest part a leader goes into on a diagonal",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-46",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry, Need::PartClasses],
    overlaps: &[],
    params: &[CLEAR, AIM, ANGLE_SLACK, REACH, MAX_PINS, COLOUR_SLACK],
};

/// The leader purple of the drawing guide
const LEADER: (u8, u8, u8) = (142, 92, 205);

/// Whether a graphic is drawn in the leader colour
fn purple(g: &DrawnGraphic, slack: f64) -> bool {
    g.stroke
        .colour
        .is_some_and(|c| near_colour(c, LEADER, slack))
}

/// The nanometres from a point to a box's edge, zero inside it
fn distance(p: Pt, r: Rect) -> f64 {
    let dx = (r.min.x - p.x).max(p.x - r.max.x).max(0) as f64;
    let dy = (r.min.y - p.y).max(p.y - r.max.y).max(0) as f64;
    dx.hypot(dy)
}

/// The nanometres from a point to the nearest side of a rectangle, inside or out
fn to_edge(p: Pt, r: Rect) -> f64 {
    let inside = [p.x - r.min.x, r.max.x - p.x, p.y - r.min.y, r.max.y - p.y];
    let least = inside.into_iter().min().unwrap_or(0);
    if least >= 0 {
        least as f64
    } else {
        distance(p, r)
    }
}

/// A leader's last segment, from the point before its tip to the tip
#[derive(Clone, Copy, Debug, PartialEq)]
struct Last {
    from: Pt,
    tip: Pt,
}

impl Last {
    /// Degrees the segment is off a diagonal
    fn off_diagonal(self) -> f64 {
        let (dx, dy) = (self.tip.x - self.from.x, self.tip.y - self.from.y);
        ((dy.abs() as f64).atan2(dx.abs() as f64).to_degrees() - 45.0).abs()
    }

    /// The nanometres its line passes from `c`, past the tip; `None` when it points away from `c`
    fn miss(self, c: Pt) -> Option<f64> {
        let (dx, dy) = (
            (self.tip.x - self.from.x) as f64,
            (self.tip.y - self.from.y) as f64,
        );
        let (cx, cy) = ((c.x - self.from.x) as f64, (c.y - self.from.y) as f64);
        let len = dx.hypot(dy);
        (dx * cx + dy * cy > 0.0).then(|| (dx * cy - dy * cx).abs() / len)
    }
}

impl Last {
    /// The nanometres left from the tip to the outline along its line, the way the head would run into it; straight
    /// to the box when the line misses it, zero on or inside it
    fn gap(self, r: Rect) -> f64 {
        if r.contains(self.tip) {
            return 0.0;
        }
        let len = ((self.tip.x - self.from.x) as f64).hypot((self.tip.y - self.from.y) as f64);
        // How far along the line it enters and leaves the box's span on one axis
        let span = |p: i64, from: i64, min: i64, max: i64| {
            let d = (p - from) as f64 / len;
            let (lo, hi) = ((min - p) as f64 / d, (max - p) as f64 / d);
            match d == 0.0 {
                true if (min..=max).contains(&p) => (f64::MIN, f64::MAX),
                true => (f64::MAX, f64::MIN),
                false => (lo.min(hi), lo.max(hi)),
            }
        };
        let (x0, x1) = span(self.tip.x, self.from.x, r.min.x, r.max.x);
        let (y0, y1) = span(self.tip.y, self.from.y, r.min.y, r.max.y);
        let (enter, leave) = (x0.max(y0), x1.min(y1));
        if enter >= 0.0 && enter <= leave {
            enter
        } else {
            distance(self.tip, r)
        }
    }
}

/// The last segment of a dotted polyline in the leader colour. The tip is the end its arrowhead sits on, else the
/// line's last point.
fn last_segment(g: &DrawnGraphic, heads: &[&DrawnGraphic]) -> Option<Last> {
    let mut points = g.points.clone();
    points.dedup();
    let (first, end) = (*points.first()?, *points.last()?);
    let on_first = heads.iter().any(|h| h.points.contains(&first));
    if on_first && !heads.iter().any(|h| h.points.contains(&end)) {
        points.reverse();
    }
    match points[..] {
        [.., from, tip] => Some(Last { from, tip }),
        _ => None,
    }
}

/// What a leader's tip points at: the body its line aims at within reach, else the nearest; `None` when a block's
/// rectangle is nearer than any body
fn target<'s>(scene: &'s Scene, last: Last, reach: f64, aim: f64, slack: f64) -> Option<&'s Item> {
    let aimed = |i: &Item| last.miss(i.rect.center()).is_some_and(|m| m <= aim);
    let body = scene
        .items
        .iter()
        .filter(|i| i.kind == K::Body && distance(last.tip, i.rect) <= reach)
        .min_by(|a, b| {
            let key = |i: &Item| (!aimed(i), distance(last.tip, i.rect));
            let ((a_off, a_d), (b_off, b_d)) = (key(a), key(b));
            a_off.cmp(&b_off).then(a_d.total_cmp(&b_d))
        })?;
    let d = distance(last.tip, body.rect);
    let rects = scene
        .geometry
        .graphics
        .iter()
        .filter(|g| g.kind == "rectangle" && g.stroke.style == "dot" && purple(g, slack));
    let mut rects = rects;
    (!rects.any(|g| to_edge(last.tip, g.bbox) < d)).then_some(body)
}

impl Check for Draw46 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A leader is a dotted polyline in the leader purple. Its target is a body within reach of the tip; a leader to
    /// a block's rectangle or to a larger part is out of scope. The outline is the body's box.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let clear = nm(cx, &META, &CLEAR);
        let aim = nm(cx, &META, &AIM) as f64;
        let reach = nm(cx, &META, &REACH) as f64;
        let angle_slack = cx.param(&META, ANGLE_SLACK.name);
        let max_pins = cx.param(&META, MAX_PINS.name) as usize;
        let slack = cx.param(&META, COLOUR_SLACK.name);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let g = scene.geometry;
            let polylines = || g.graphics.iter().filter(|p| p.kind == "polyline");
            let lines = polylines().filter(|p| p.stroke.style == "dot" && purple(p, slack));
            let heads: Vec<_> = polylines().filter(|p| p.fill.is_some()).collect();
            for leader in lines {
                let Some(last) = last_segment(leader, &heads) else {
                    continue;
                };
                let Some(body) = target(scene, last, reach, aim, slack) else {
                    continue;
                };
                let symbol = body.owner.map(|s| &g.symbols[s]);
                let small = symbol.is_some_and(|s| s.pins.len() <= max_pins && small_part(cx, s));
                if !small {
                    continue;
                }
                let mut wrong = Vec::new();
                let off = last.off_diagonal();
                if off > angle_slack {
                    wrong.push(format!(
                        "its last segment is {off:.0} degrees off a diagonal"
                    ));
                }
                match last.miss(body.rect.center()) {
                    Some(m) if m > aim => wrong.push(format!(
                        "its line passes {} mm from the body's centre",
                        mm(m as i64)
                    )),
                    None => wrong.push("it points away from the body".to_string()),
                    _ => {}
                }
                // To the micron: a tip drawn at 0.449 on both axes is 0.635 along a diagonal
                let gap = (last.gap(body.rect) / 1e3).round() as i64 * 1000;
                if gap == 0 {
                    wrong.push("its tip is on or inside the outline".to_string());
                } else if gap < clear {
                    wrong.push(format!("its tip stops {} mm from the outline", mm(gap)));
                }
                if wrong.is_empty() {
                    continue;
                }
                let start = leader.points.first().copied().unwrap_or(last.from);
                let message = format!(
                    "Leader from x {} y {} into {}: {}",
                    mm(start.x),
                    mm(start.y),
                    body.name,
                    wrong.join(", ")
                );
                out.push(
                    one(cx, &META, scene, body, last.tip)
                        .message(message)
                        .fix("end the leader on a 45 degree segment aimed at the body's centre, clear of the outline")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw46));

#[cfg(test)]
mod tests {
    use super::*;

    fn last(from: (i64, i64), tip: (i64, i64)) -> Last {
        Last {
            from: Pt::new(from.0, from.1),
            tip: Pt::new(tip.0, tip.1),
        }
    }

    #[test]
    fn the_last_segment_is_measured() {
        assert!(last((0, 0), (5, 5)).off_diagonal() < 1e-9);
        assert!(last((0, 0), (-5, 5)).off_diagonal() < 1e-9);
        assert!((last((0, 0), (5, 0)).off_diagonal() - 45.0).abs() < 1e-9);
        let l = last((0, 0), (4, 4));
        assert_eq!(l.miss(Pt::new(8, 8)), Some(0.0));
        assert_eq!(l.miss(Pt::new(-3, -3)), None);
        assert!((l.miss(Pt::new(8, 4)).unwrap() - 8f64.sqrt()).abs() < 1e-9);
        let r = Rect::of(Pt::new(10, 10), Pt::new(20, 20));
        assert!((last((0, 7), (3, 10)).gap(r) - 7f64.hypot(7.0)).abs() < 1e-9);
        assert_eq!(last((0, 15), (8, 15)).gap(r), 2.0);
        assert_eq!(last((0, 15), (12, 15)).gap(r), 0.0);
        assert_eq!(last((0, 0), (9, 0)).gap(r), 1f64.hypot(10.0));
    }

    #[test]
    fn distances_to_a_box() {
        let r = Rect::of(Pt::new(0, 0), Pt::new(10, 10));
        assert_eq!(distance(Pt::new(5, 5), r), 0.0);
        assert_eq!(distance(Pt::new(13, 14), r), 5.0);
        assert_eq!(to_edge(Pt::new(2, 5), r), 2.0);
        assert_eq!(to_edge(Pt::new(-3, 5), r), 3.0);
    }

    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&Draw46, "draw-46_leader_aim"),
            [
                "DRAW-46:item R2 body|sheet /",
                "DRAW-46:item R3 body|sheet /",
                "DRAW-46:item R4 body|sheet /"
            ]
        );
    }
}
