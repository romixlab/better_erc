//! A footprint in its own frame, as its library file (`.kicad_mod`) draws it: at the origin, unrotated, on the
//! front. A library footprint is read this way; [`Layout::local_footprint`] brings a footprint placed on a board
//! back to it, so a board copy and its library file compare as equal.
//!
//! KiCad writes a placed footprint's pads and graphics relative to the footprint, but pad angles and the
//! footprint's zones absolute; the board reader maps them all to absolute board coordinates. A footprint on the
//! back is the front one mirrored about its own x axis (y negated, in KiCad's flip), its layers swapped (`F.Cu`
//! and `B.Cu`, inner copper layers in reverse) and its pad angles negated. Undoing that here gives back the
//! library geometry: exactly at right angles, to within KiCad's own 1 nm rounding at other angles.

use crate::layout::*;
use serde::{Deserialize, Serialize};

/// A footprint and its pads in the footprint's own frame
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalFootprint {
    /// At the origin, rotation 0, on the front; [`Footprint::pads`] indexes [`pads`](Self::pads)
    pub footprint: Footprint,
    /// Positions relative to the footprint, rotations relative to it; [`Pad::footprint`] is 0
    pub pads: Vec<Pad>,
}

/// `p` turned by `deg` counter-clockwise on screen (y down), exact at multiples of 90°
pub fn rotate(p: Pt, deg: f64) -> Pt {
    let d = deg.rem_euclid(360.0);
    if d.fract() == 0.0 && (d as i64) % 90 == 0 {
        return match d as i64 {
            0 => p,
            90 => Pt::new(p.y, -p.x),
            180 => Pt::new(-p.x, -p.y),
            _ => Pt::new(-p.y, p.x),
        };
    }
    let (s, c) = d.to_radians().sin_cos();
    let (x, y) = (p.x as f64, p.y as f64);
    Pt::new(
        (x * c + y * s).round() as i64,
        (-x * s + y * c).round() as i64,
    )
}

/// An angle in degrees as `[0, 360)`, rounded to a millionth of a degree so `90.00000000001` is `90`
pub fn deg360(deg: f64) -> f64 {
    let d = ((deg * 1e6).round() / 1e6).rem_euclid(360.0);
    if d == 360.0 { 0.0 } else { d + 0.0 }
}

/// An arc in a form that does not depend on how it was drawn: its ends in order (the smaller point first) and the
/// point halfway along it. KiCad turns an arc round when it flips a footprint and moves the middle point to the
/// true middle; three points in a line stay as they are.
pub fn canonical_arc(start: Pt, mid: Pt, end: Pt) -> (Pt, Pt, Pt) {
    let (a, b) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let f = |p: Pt| (p.x as f64, p.y as f64);
    let ((ax, ay), (mx, my), (bx, by)) = (f(a), f(mid), f(b));
    let d = 2.0 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my));
    if d.abs() < 1.0 {
        return (a, mid, b);
    }
    let sq = |x: f64, y: f64| x * x + y * y;
    let cx = (sq(ax, ay) * (my - by) + sq(mx, my) * (by - ay) + sq(bx, by) * (ay - my)) / d;
    let cy = (sq(ax, ay) * (bx - mx) + sq(mx, my) * (ax - bx) + sq(bx, by) * (mx - ax)) / d;
    let r = (ax - cx).hypot(ay - cy);
    let ang = |x: f64, y: f64| (y - cy).atan2(x - cx);
    let (sa, ma, ea) = (ang(ax, ay), ang(mx, my), ang(bx, by));
    let tau = std::f64::consts::TAU;
    // The sweep from a to b, counter-clockwise in these coordinates, and whether the middle point is on it
    let ccw = (ea - sa).rem_euclid(tau);
    let sweep = if (ma - sa).rem_euclid(tau) < ccw {
        ccw
    } else {
        ccw - tau
    };
    let h = sa + sweep / 2.0;
    let m = Pt::new(
        (cx + r * h.cos()).round() as i64,
        (cy + r * h.sin()).round() as i64,
    );
    (a, m, b)
}

/// The layer on the other side: `F.Cu` and `B.Cu`, `F.SilkS` and `B.SilkS`, ..., `In1.Cu` and `In(n-2).Cu` of a
/// board with `copper` copper layers; layers on both sides (`*.Cu`, `F&B.Cu`) and user layers stay
pub fn flip_layer(name: &str, copper: usize) -> String {
    if let Some(rest) = name.strip_prefix("F.") {
        return format!("B.{rest}");
    }
    if let Some(rest) = name.strip_prefix("B.") {
        return format!("F.{rest}");
    }
    if let Some(n) = name
        .strip_prefix("In")
        .and_then(|r| r.strip_suffix(".Cu"))
        .and_then(|n| n.parse::<usize>().ok())
        && copper > 2
        && (1..=copper - 2).contains(&n)
    {
        return format!("In{}.Cu", copper - 1 - n);
    }
    name.to_string()
}

/// How one placed footprint maps back to its own frame
struct Undo {
    at: Pt,
    rot: f64,
    back: bool,
    copper: usize,
}

impl Undo {
    /// A board point to the footprint's frame
    fn pt(&self, p: Pt) -> Pt {
        let l = rotate(Pt::new(p.x - self.at.x, p.y - self.at.y), -self.rot);
        self.mirror(l)
    }

    /// The flip about the footprint's x axis (pad-relative points too)
    fn mirror(&self, p: Pt) -> Pt {
        if self.back { Pt::new(p.x, -p.y) } else { p }
    }

    fn square(&self) -> bool {
        (self.rot / 90.0).fract() == 0.0
    }

    fn layer(&self, l: &str) -> String {
        if self.back {
            flip_layer(l, self.copper)
        } else {
            l.to_string()
        }
    }

    /// A pad's or zone's layers; KiCad writes a library pad's `*.Cu` as `F&B.Cu` on a two-layer board, and on
    /// any board for a pad without plating (`npth`)
    fn layers(&self, ls: &[String], npth: bool) -> Vec<String> {
        ls.iter()
            .map(|l| match l.as_str() {
                "F&B.Cu" if self.copper == 2 || npth => "*.Cu".to_string(),
                _ => self.layer(l),
            })
            .collect()
    }

    fn geom(&self, g: &Geom, map: &dyn Fn(Pt) -> Pt) -> Geom {
        let rect = |a: Pt, b: Pt| {
            Geom::Rect(
                Pt::new(a.x.min(b.x), a.y.min(b.y)),
                Pt::new(a.x.max(b.x), a.y.max(b.y)),
            )
        };
        match g {
            Geom::Line(a, b) => Geom::Line(map(*a), map(*b)),
            Geom::Rect(a, b) => rect(map(*a), map(*b)),
            Geom::Circle { center, radius } => Geom::Circle {
                center: map(*center),
                radius: *radius,
            },
            Geom::Arc { start, mid, end } => Geom::Arc {
                start: map(*start),
                mid: map(*mid),
                end: map(*end),
            },
            Geom::Poly(ps) => {
                let ps: Vec<Pt> = ps.iter().map(|p| map(*p)).collect();
                // The board reader turns a rectangle of a footprint at an odd angle into a polygon; back at 0° it is
                // the rectangle again
                match ps.as_slice() {
                    [a, b, c, d] if !self.square() && axis_rect(a, b, c, d) => rect(*a, *c),
                    _ => Geom::Poly(ps),
                }
            }
            Geom::Bezier(ps) => Geom::Bezier(ps.map(map)),
        }
    }

    fn shape(&self, s: &Shape, map: &dyn Fn(Pt) -> Pt) -> Shape {
        Shape {
            layer: self.layer(&s.layer),
            geom: self.geom(&s.geom, map),
            width: s.width,
            filled: s.filled,
        }
    }

    fn pad(&self, p: &Pad) -> Pad {
        let rel = p.rotation - self.rot;
        let mirror = |q: Pt| self.mirror(q);
        let mut d = p.details.clone();
        if self.back {
            d.rect_delta = d.rect_delta.map(mirror);
            d.drill_offset = d.drill_offset.map(mirror);
            for c in &mut d.chamfer {
                *c = match c.as_str() {
                    "top_left" => "bottom_left",
                    "top_right" => "bottom_right",
                    "bottom_left" => "top_left",
                    "bottom_right" => "top_right",
                    o => o,
                }
                .to_string();
            }
            d.padstack = d.padstack.map(|t| flip_quoted_layers(&t, self.copper));
        }
        Pad {
            footprint: FootprintId(0),
            at: self.pt(p.at),
            rotation: deg360(if self.back { -rel } else { rel }),
            layers: self.layers(&p.layers, p.kind == "np_thru_hole"),
            primitives: p
                .primitives
                .iter()
                .map(|s| self.shape(s, &mirror))
                .collect(),
            details: d,
            ..p.clone()
        }
    }
}

/// Four corners in order of a rectangle with sides along the axes, give or take the 2 nm of rounding a turn there
/// and back leaves
fn axis_rect(a: &Pt, b: &Pt, c: &Pt, d: &Pt) -> bool {
    let near = |u: i64, v: i64| (u - v).abs() <= 2;
    (near(a.y, b.y) && near(b.x, c.x) && near(c.y, d.y) && near(d.x, a.x))
        || (near(a.x, b.x) && near(b.y, c.y) && near(c.x, d.x) && near(d.y, a.y))
}

/// The layer names in the quoted strings of a padstack's text, flipped
fn flip_quoted_layers(text: &str, copper: usize) -> String {
    text.split('"')
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 1 {
                flip_layer(part, copper)
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\"")
}

impl Layout {
    /// The board's copper layers by its layer table; 2 when the table is empty
    pub fn copper_layers(&self) -> usize {
        match self
            .layers
            .iter()
            .filter(|l| l.name.ends_with(".Cu"))
            .count()
        {
            0 => 2,
            n => n,
        }
    }

    /// Footprint `id` in its own frame: its position and rotation undone, a footprint on the back flipped to the
    /// front, as its library file draws it. Texts move too (and lose or gain their mirroring); fields, nets and
    /// attributes stay as they are.
    pub fn local_footprint(&self, id: FootprintId) -> LocalFootprint {
        let f = self.footprint(id);
        let u = Undo {
            at: f.at,
            rot: f.rotation,
            back: f.side == Side::Back,
            copper: self.copper_layers(),
        };
        let pt = |p: Pt| u.pt(p);
        let pads: Vec<Pad> = f.pads.iter().map(|p| u.pad(self.pad(*p))).collect();
        let footprint = Footprint {
            side: Side::Front,
            at: Pt::default(),
            rotation: 0.0,
            pads: (0..pads.len()).map(PadId::from_index).collect(),
            graphics: f.graphics.iter().map(|s| u.shape(s, &pt)).collect(),
            texts: f
                .texts
                .iter()
                .map(|t| BoardText {
                    layer: u.layer(&t.layer),
                    at: u.pt(t.at),
                    mirrored: t.mirrored != u.back,
                    ..t.clone()
                })
                .collect(),
            zones: f
                .zones
                .iter()
                .map(|z| Zone {
                    layers: u.layers(&z.layers, false),
                    outline: z.outline.iter().map(|p| u.pt(*p)).collect(),
                    ..z.clone()
                })
                .collect(),
            ..f.clone()
        };
        LocalFootprint { footprint, pads }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turns_are_exact_and_match_the_board_reader() {
        // The board reader's frame: 90° takes (1, 0) to (0, -1) on screen
        assert_eq!(rotate(Pt::new(1, 0), 90.0), Pt::new(0, -1));
        assert_eq!(rotate(Pt::new(0, 1), 90.0), Pt::new(1, 0));
        assert_eq!(rotate(Pt::new(3, 4), -270.0), rotate(Pt::new(3, 4), 90.0));
        let p = Pt::new(1_234_567, -7_654_321);
        assert_eq!(rotate(rotate(p, 30.0), -30.0), p);
    }

    #[test]
    fn layers_flip() {
        assert_eq!(flip_layer("F.Cu", 4), "B.Cu");
        assert_eq!(flip_layer("B.CrtYd", 4), "F.CrtYd");
        assert_eq!(flip_layer("In1.Cu", 4), "In2.Cu");
        assert_eq!(flip_layer("In1.Cu", 6), "In4.Cu");
        assert_eq!(flip_layer("*.Cu", 4), "*.Cu");
        assert_eq!(flip_layer("User.1", 4), "User.1");
        assert_eq!(
            flip_quoted_layers(r#"(layer "F.Cu" (shape rect))"#, 2),
            r#"(layer "B.Cu" (shape rect))"#
        );
    }

    #[test]
    fn arcs_compare_however_drawn() {
        let (s, e) = (
            Pt::new(-3_000_000, -2_500_000),
            Pt::new(-1_200_000, -3_000_000),
        );
        let drawn = canonical_arc(s, Pt::new(-2_200_000, -3_200_000), e);
        let flipped = canonical_arc(e, Pt::new(-2_223_300, -3_193_800), s);
        assert_eq!(drawn.0, flipped.0);
        assert!((drawn.1.x - flipped.1.x).abs() <= 100 && (drawn.1.y - flipped.1.y).abs() <= 100);
        // A half circle: the middle point decides the side
        let up = canonical_arc(Pt::new(-10, 0), Pt::new(0, -10), Pt::new(10, 0));
        let down = canonical_arc(Pt::new(10, 0), Pt::new(0, 10), Pt::new(-10, 0));
        assert_eq!((up.1, down.1), (Pt::new(0, -10), Pt::new(0, 10)));
    }

    #[test]
    fn angles() {
        assert_eq!(deg360(-90.0), 270.0);
        assert_eq!(deg360(360.0), 0.0);
        assert_eq!(deg360(-0.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(deg360(89.999_999_999_9), 90.0);
    }
}
