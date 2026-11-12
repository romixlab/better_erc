//! A footprint as shapes to draw over a datasheet: every pad's outline as a polygon, its drill, and the graphics of
//! the layers a review compares (courtyard, fab, silk, copper, mask, paste, board edge), in millimetres in the
//! footprint's own frame (x right, y down, as KiCad draws it). Each pad's mask opening and paste aperture are drawn
//! as outlines of the mask and paste layers; vias in a package (KiCad draws them as round plated pads, thermal vias
//! in an exposed pad) as rings of their own.

use better_erc::model::LocalFootprint;
use better_erc::model::layout::{Geom, Pad, Pt, Shape};
use std::f64::consts::PI;

/// What a shape belongs to, each shown or hidden on its own
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Pad,
    /// A pad on the back copper only
    BackPad,
    /// A via in the package: a round plated pad without a number of its own (see [`is_via`])
    Via,
    Courtyard,
    Fab,
    Silk,
    Copper,
    Mask,
    Paste,
    Edge,
}

impl Kind {
    pub const ALL: [Kind; 10] = [
        Kind::Pad,
        Kind::BackPad,
        Kind::Via,
        Kind::Courtyard,
        Kind::Fab,
        Kind::Silk,
        Kind::Copper,
        Kind::Mask,
        Kind::Paste,
        Kind::Edge,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Pad => "Pads",
            Kind::BackPad => "Back pads",
            Kind::Via => "Vias",
            Kind::Courtyard => "Courtyard",
            Kind::Fab => "Fab",
            Kind::Silk => "Silk",
            Kind::Copper => "Copper graphics",
            Kind::Mask => "Mask",
            Kind::Paste => "Paste",
            Kind::Edge => "Board edge",
        }
    }

    /// Shown when a footprint is opened
    pub fn shown_at_first(self) -> bool {
        matches!(
            self,
            Kind::Pad | Kind::BackPad | Kind::Via | Kind::Courtyard | Kind::Fab | Kind::Copper
        )
    }

    fn of_layer(layer: &str) -> Option<Kind> {
        let side = layer.split_once('.').map_or(layer, |(_, l)| l);
        Some(match side {
            "CrtYd" | "Courtyard" => Kind::Courtyard,
            "Fab" => Kind::Fab,
            "SilkS" | "Silkscreen" => Kind::Silk,
            "Cu" => Kind::Copper,
            "Mask" => Kind::Mask,
            "Paste" => Kind::Paste,
            "Cuts" => Kind::Edge,
            _ => return None,
        })
    }
}

/// A point in millimetres
pub type P = [f64; 2];

/// One shape to draw
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A filled outline; `convex` when a plain fill draws it right
    Area {
        kind: Kind,
        points: Vec<P>,
        convex: bool,
    },
    /// A line along `points`, `width` mm wide (0: the thinnest line)
    Line {
        kind: Kind,
        points: Vec<P>,
        closed: bool,
        width: f64,
    },
    /// A drill hole of a pad or a via
    Hole {
        kind: Kind,
        center: P,
        size: P,
        rotation: f64,
    },
    /// A pad's number at its middle
    Number { at: P, text: String, size: f64 },
}

impl Item {
    pub fn kind(&self) -> Kind {
        match self {
            Item::Area { kind, .. } | Item::Line { kind, .. } | Item::Hole { kind, .. } => *kind,
            Item::Number { .. } => Kind::Pad,
        }
    }

    /// Every point of the shape, for its extent
    pub fn points(&self) -> Vec<P> {
        match self {
            Item::Area { points, .. } | Item::Line { points, .. } => points.clone(),
            Item::Hole { center, size, .. } => vec![
                [center[0] - size[0] / 2.0, center[1] - size[1] / 2.0],
                [center[0] + size[0] / 2.0, center[1] + size[1] / 2.0],
            ],
            Item::Number { at, .. } => vec![*at],
        }
    }
}

fn mm(p: Pt) -> P {
    [p.x as f64 / 1e6, p.y as f64 / 1e6]
}

fn len(v: i64) -> f64 {
    v as f64 / 1e6
}

/// `p` turned by `deg` counter-clockwise on screen (y down), as [`better_erc::model::footprint::rotate`]
pub fn rotate(p: P, deg: f64) -> P {
    let (s, c) = deg.to_radians().sin_cos();
    [p[0] * c + p[1] * s, -p[0] * s + p[1] * c]
}

/// How many straight pieces a quarter circle is drawn with
const QUARTER: usize = 8;

fn arc_points(out: &mut Vec<P>, c: P, r: f64, from_deg: f64, to_deg: f64) {
    let n = (((to_deg - from_deg).abs() / 90.0) * QUARTER as f64)
        .ceil()
        .max(1.0) as usize;
    for i in 0..=n {
        let a = (from_deg + (to_deg - from_deg) * i as f64 / n as f64).to_radians();
        out.push([c[0] + r * a.cos(), c[1] + r * a.sin()]);
    }
}

fn circle_points(c: P, r: f64) -> Vec<P> {
    let mut v = Vec::new();
    arc_points(&mut v, c, r, 0.0, 360.0);
    v.pop();
    v
}

/// A rectangle `w` by `h` about the origin with each corner (top left, top right, bottom right, bottom left; y
/// down) rounded by `radius` or cut by `chamfer`
fn rect_points(w: f64, h: f64, radius: f64, chamfer: [f64; 4]) -> Vec<P> {
    let (x, y) = (w / 2.0, h / 2.0);
    // Corner, its centre direction and the angles its arc runs through (y down: angles turn clockwise on screen)
    let corners = [
        ([-x, -y], [1.0, 1.0], 180.0),
        ([x, -y], [-1.0, 1.0], 270.0),
        ([x, y], [-1.0, -1.0], 0.0),
        ([-x, y], [1.0, -1.0], 90.0),
    ];
    let mut out = Vec::new();
    for (i, (c, d, a0)) in corners.into_iter().enumerate() {
        if chamfer[i] > 0.0 {
            let k = chamfer[i];
            // The two points the cut leaves, in the order the outline runs (clockwise on screen)
            let (p1, p2) = match i {
                0 => ([c[0], c[1] + k], [c[0] + k, c[1]]),
                1 => ([c[0] - k, c[1]], [c[0], c[1] + k]),
                2 => ([c[0], c[1] - k], [c[0] - k, c[1]]),
                _ => ([c[0] + k, c[1]], [c[0], c[1] - k]),
            };
            out.push(p1);
            out.push(p2);
        } else if radius > 0.0 {
            let centre = [c[0] + d[0] * radius, c[1] + d[1] * radius];
            arc_points(&mut out, centre, radius, a0, a0 + 90.0);
        } else {
            out.push(c);
        }
    }
    out
}

/// A pad's copper outline in the pad's own frame (unrotated, about its drill), grown by `grow` mm on every side
/// (shrunk when negative: a mask or paste margin), and whether it is convex
fn pad_shape(pad: &Pad, grow: f64) -> (Vec<P>, bool) {
    let small0 = len(pad.size.x.min(pad.size.y));
    // Never shrunk past nothing
    let g = grow.max(-small0 / 2.0 + 1e-3);
    let (w, h) = (len(pad.size.x) + 2.0 * g, len(pad.size.y) + 2.0 * g);
    let small = w.min(h);
    let d = &pad.details;
    // Corners grow round by the margin and shrink sharp, as KiCad draws a margin
    let rounded = |r: f64| (r + g).max(0.0);
    let mut points = match pad.shape.as_str() {
        "circle" => circle_points([0.0, 0.0], w / 2.0),
        "oval" => rect_points(w, h, small / 2.0, [0.0; 4]),
        "roundrect" => rect_points(
            w,
            h,
            rounded(d.roundrect_rratio.unwrap_or(0.25) * small0),
            [0.0; 4],
        ),
        "chamfered_rect" => {
            let k = d.chamfer_ratio.unwrap_or(0.0) * small0;
            let corner = |name: &str| {
                if d.chamfer.iter().any(|c| c == name) {
                    k
                } else {
                    0.0
                }
            };
            let chamfer = [
                corner("top_left"),
                corner("top_right"),
                corner("bottom_right"),
                corner("bottom_left"),
            ];
            rect_points(
                w,
                h,
                rounded(d.roundrect_rratio.unwrap_or(0.0) * small0),
                chamfer,
            )
        }
        "trapezoid" => {
            // KiCad: delta x narrows the right side's height, delta y the bottom side's width
            let dl = d
                .rect_delta
                .map_or([0.0, 0.0], |p| [len(p.x) / 2.0, len(p.y) / 2.0]);
            let (x, y) = (w / 2.0, h / 2.0);
            vec![
                [-x - dl[1], -y - dl[0]],
                [x + dl[1], -y + dl[0]],
                [x - dl[1], y - dl[0]],
                [-x + dl[1], y + dl[0]],
            ]
        }
        "custom" => match d.anchor.as_deref() {
            Some("circle") => circle_points([0.0, 0.0], w / 2.0),
            _ => rect_points(w, h, rounded(0.0), [0.0; 4]),
        },
        _ => rect_points(w, h, rounded(0.0), [0.0; 4]),
    };
    if let Some(o) = d.drill_offset {
        for p in &mut points {
            p[0] += len(o.x);
            p[1] += len(o.y);
        }
    }
    (points, pad.shape != "custom")
}

/// A drawn shape's points: a polygon, or a line with its width
fn shape_points(geom: &Geom) -> (Vec<P>, bool) {
    match geom {
        Geom::Line(a, b) => (vec![mm(*a), mm(*b)], false),
        Geom::Rect(a, b) => {
            let (a, b) = (mm(*a), mm(*b));
            (vec![a, [b[0], a[1]], b, [a[0], b[1]]], true)
        }
        Geom::Circle { center, radius } => (circle_points(mm(*center), len(*radius)), true),
        Geom::Arc { start, mid, end } => (arc_through(mm(*start), mm(*mid), mm(*end)), false),
        Geom::Poly(pts) => (pts.iter().map(|p| mm(*p)).collect(), true),
        Geom::Bezier(c) => {
            let c = c.map(mm);
            let pts = (0..=16)
                .map(|i| {
                    let t = i as f64 / 16.0;
                    let u = 1.0 - t;
                    let k = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    [
                        (0..4).map(|j| k[j] * c[j][0]).sum(),
                        (0..4).map(|j| k[j] * c[j][1]).sum(),
                    ]
                })
                .collect();
            (pts, false)
        }
    }
}

/// The arc from `a` through `m` to `b`, as points
fn arc_through(a: P, m: P, b: P) -> Vec<P> {
    let d = 2.0 * (a[0] * (m[1] - b[1]) + m[0] * (b[1] - a[1]) + b[0] * (a[1] - m[1]));
    if d.abs() < 1e-12 {
        return vec![a, b];
    }
    let sq = |p: P| p[0] * p[0] + p[1] * p[1];
    let c = [
        (sq(a) * (m[1] - b[1]) + sq(m) * (b[1] - a[1]) + sq(b) * (a[1] - m[1])) / d,
        (sq(a) * (b[0] - m[0]) + sq(m) * (a[0] - b[0]) + sq(b) * (m[0] - a[0])) / d,
    ];
    let r = (sq([a[0] - c[0], a[1] - c[1]])).sqrt();
    let ang = |p: P| (p[1] - c[1]).atan2(p[0] - c[0]);
    let (a0, am, mut a1) = (ang(a), ang(m), ang(b));
    // Go the way that passes the middle point
    let norm = |x: f64| x.rem_euclid(2.0 * PI);
    if norm(am - a0) > norm(a1 - a0) {
        a1 -= 2.0 * PI;
        while a1 > a0 {
            a1 -= 2.0 * PI;
        }
    } else {
        while a1 < a0 {
            a1 += 2.0 * PI;
        }
    }
    let mut out = Vec::new();
    arc_points(&mut out, c, r, a0.to_degrees(), a1.to_degrees());
    out
}

fn shape_item(kind: Kind, s: &Shape, place: impl Fn(P) -> P) -> Item {
    let (points, closed) = shape_points(&s.geom);
    let points: Vec<P> = points.into_iter().map(place).collect();
    if s.filled && closed {
        Item::Area {
            kind,
            convex: !matches!(s.geom, Geom::Poly(_)),
            points,
        }
    } else {
        Item::Line {
            kind,
            points,
            closed,
            width: len(s.width),
        }
    }
}

/// Everything of `f` to draw, pads last so they are on top
pub fn items(f: &LocalFootprint) -> Vec<Item> {
    let mut out = Vec::new();
    for g in &f.footprint.graphics {
        // Only the front and the board edge: a library footprint draws on the front
        if g.layer.starts_with("B.") {
            continue;
        }
        if let Some(kind) = Kind::of_layer(&g.layer) {
            out.push(shape_item(kind, g, |p| p));
        }
    }
    // Zones and keepouts as their outlines
    for z in &f.footprint.zones {
        if z.outline.len() > 1 {
            out.push(Item::Line {
                kind: Kind::Copper,
                points: z.outline.iter().map(|p| mm(*p)).collect(),
                closed: true,
                width: 0.0,
            });
        }
    }
    let st = &f.footprint.settings;
    for pad in &f.pads {
        let at = mm(pad.at);
        let place = |p: P| {
            let r = rotate(p, pad.rotation);
            [r[0] + at[0], r[1] + at[1]]
        };
        let on = |layer: &str| {
            let any = format!("*.{}", layer.split_once('.').map_or(layer, |(_, l)| l));
            pad.layers.iter().any(|l| l == layer || *l == any)
        };
        // The mask opening and the paste aperture, by the pad's margins, else the footprint's
        let small = len(pad.size.x.min(pad.size.y));
        let mask = len(pad
            .solder_mask_margin
            .or(st.solder_mask_margin)
            .unwrap_or(0));
        let paste = len(pad
            .solder_paste_margin
            .or(st.solder_paste_margin)
            .unwrap_or(0))
            + pad
                .solder_paste_ratio
                .or(st.solder_paste_ratio)
                .unwrap_or(0.0)
                * small;
        for (layer, kind, grow) in [
            ("F.Mask", Kind::Mask, mask),
            ("F.Paste", Kind::Paste, paste),
        ] {
            if on(layer) {
                let (shape, _) = pad_shape(pad, grow);
                out.push(Item::Line {
                    kind,
                    points: shape.into_iter().map(place).collect(),
                    closed: true,
                    width: 0.0,
                });
            }
        }
        let front = on("F.Cu") || pad.layers.iter().any(|l| l == "F&B.Cu");
        let back = pad.layers.iter().any(|l| l == "B.Cu");
        if is_via(pad, &f.pads) {
            let (ring, _) = pad_shape(pad, 0.0);
            out.push(Item::Line {
                kind: Kind::Via,
                points: ring.into_iter().map(place).collect(),
                closed: true,
                width: 0.0,
            });
            if let Some(d) = pad.drill {
                out.push(Item::Hole {
                    kind: Kind::Via,
                    center: at,
                    size: [len(d.x), len(d.x)],
                    rotation: 0.0,
                });
            }
            continue;
        }
        if front || back {
            let kind = if front { Kind::Pad } else { Kind::BackPad };
            let (shape, convex) = pad_shape(pad, 0.0);
            out.push(Item::Area {
                kind,
                points: shape.into_iter().map(place).collect(),
                convex,
            });
            for prim in &pad.primitives {
                out.push(shape_item(kind, prim, place));
            }
        }
        if let Some(d) = pad.drill {
            out.push(Item::Hole {
                kind: Kind::Pad,
                center: at,
                size: [len(d.x), len(if d.y > 0 { d.y } else { d.x })],
                rotation: pad.rotation,
            });
        }
        if !pad.number.is_empty() && (front || back) {
            out.push(Item::Number {
                at: place(pad.details.drill_offset.map_or([0.0, 0.0], mm)),
                text: pad.number.clone(),
                size: len(pad.size.x.min(pad.size.y)),
            });
        }
    }
    out
}

/// Whether `pad` is a via in the package: a round plated hole through all copper, either without a number, marked
/// as a heatsink, or sharing its number with a surface pad (a thermal via of an exposed pad). KiCad 10 has no vias
/// in footprints; a package's vias are such pads, and they are in the review hash as pads.
pub fn is_via(pad: &Pad, pads: &[Pad]) -> bool {
    pad.kind == "thru_hole"
        && pad.shape == "circle"
        && pad.drill.is_some()
        && pad.layers.iter().any(|l| l == "*.Cu" || l == "F&B.Cu")
        && (pad.number.is_empty()
            || pad.property.as_deref() == Some("pad_prop_heatsink")
            || pads
                .iter()
                .any(|o| o.kind == "smd" && o.number == pad.number))
}

/// The smallest and largest corner of everything in `items`, mm
pub fn extent(items: &[Item]) -> Option<(P, P)> {
    let pts: Vec<P> = items.iter().flat_map(Item::points).collect();
    let first = *pts.first()?;
    Some(pts.iter().fold((first, first), |(lo, hi), p| {
        (
            [lo[0].min(p[0]), lo[1].min(p[1])],
            [hi[0].max(p[0]), hi[1].max(p[1])],
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> LocalFootprint {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../erc_kicad/tests/fixtures/footprints/Test.pretty")
            .join(name);
        better_erc::fp::read_library(&p).unwrap()
    }

    /// Every pad of the test footprints becomes an outline the size of the pad, and a turned rectangle stays one
    #[test]
    fn pads_become_outlines() {
        for name in ["Mixed_Pads.kicad_mod", "Asym_SOT.kicad_mod"] {
            let f = fixture(name);
            let items = items(&f);
            // One number per copper pad (a custom pad's primitives are areas of their own)
            let pads = items
                .iter()
                .filter(|i| matches!(i, Item::Number { .. }))
                .count();
            let copper_pads = f
                .pads
                .iter()
                .filter(|p| !p.number.is_empty() && p.layers.iter().any(|l| l.ends_with(".Cu")))
                .count();
            assert_eq!(pads, copper_pads, "{name}");
            assert!(extent(&items).is_some());
        }
        let r = rect_points(2.0, 1.0, 0.0, [0.0; 4]);
        assert_eq!(r, [[-1.0, -0.5], [1.0, -0.5], [1.0, 0.5], [-1.0, 0.5]]);
        let t = rotate([1.0, 0.0], 90.0);
        assert!((t[0]).abs() < 1e-12 && (t[1] + 1.0).abs() < 1e-12, "{t:?}");
        let a = arc_through([1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]);
        assert!(
            a.iter().all(|p| p[1] >= -1e-9),
            "through the middle point: {a:?}"
        );
    }

    /// Thermal vias in an exposed pad (round plated pads numbered as the pad) are drawn as vias: a ring and a hole
    /// of their own kind, no number; a plated pin with a number of its own stays a pad
    #[test]
    fn package_vias_are_vias() {
        let mut f = fixture("Asym_SOT.kicad_mod");
        let mut via = f.pads[0].clone();
        via.kind = "thru_hole".into();
        via.shape = "circle".into();
        via.size = Pt::new(600_000, 600_000);
        via.drill = Some(Pt::new(300_000, 300_000));
        via.layers = vec!["*.Cu".into()];
        let mut pin = via.clone();
        pin.number = "99".into();
        f.pads.push(via.clone());
        f.pads.push(pin.clone());
        assert!(is_via(&via, &f.pads) && !is_via(&pin, &f.pads));
        let items = items(&f);
        let of = |k: Kind| items.iter().filter(|i| i.kind() == k).count();
        assert_eq!(of(Kind::Via), 2, "a ring and a hole");
        assert!(
            items
                .iter()
                .any(|i| matches!(i, Item::Number { text, .. } if text == "99"))
        );
        let numbers = items
            .iter()
            .filter(|i| matches!(i, Item::Number { text, .. } if *text == via.number))
            .count();
        assert_eq!(
            numbers, 1,
            "the via has no number: only the pad it shares it with"
        );
    }

    /// The mask opening grows by the margin, the paste aperture shrinks by a negative one
    #[test]
    fn mask_and_paste_by_their_margins() {
        let mut f = fixture("Asym_SOT.kicad_mod");
        f.pads.truncate(1);
        let p = &mut f.pads[0];
        p.shape = "rect".into();
        p.rotation = 0.0;
        p.at = Pt::default();
        p.size = Pt::new(1_000_000, 1_000_000);
        p.layers = vec!["F.Cu".into(), "F.Mask".into(), "F.Paste".into()];
        p.solder_mask_margin = Some(100_000);
        p.solder_paste_margin = Some(-100_000);
        let items = items(&f);
        let span = |k: Kind| {
            let pts: Vec<P> = items
                .iter()
                .filter(|i| i.kind() == k)
                .flat_map(Item::points)
                .collect();
            pts.iter().map(|p| p[0]).fold(f64::MIN, f64::max)
                - pts.iter().map(|p| p[0]).fold(f64::MAX, f64::min)
        };
        assert!(
            (span(Kind::Mask) - 1.2).abs() < 1e-9,
            "{}",
            span(Kind::Mask)
        );
        assert!(
            (span(Kind::Paste) - 0.8).abs() < 1e-9,
            "{}",
            span(Kind::Paste)
        );
    }
}
