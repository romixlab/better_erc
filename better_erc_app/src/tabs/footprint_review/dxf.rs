//! The pieces selected on a datasheet page as a DXF in the footprint's millimetres (VIEW-22): the page's points go
//! through the drawing's scale and the footprint's placement (origin, rotation, bottom view) back into the
//! footprint's own frame, so KiCad's footprint editor (File → Import → Graphics, at 0, 0) drops the drawing in
//! place over the pads. DXF's Y points up, KiCad's down: the import turns it over.
//!
//! The file is R12 ASCII, written here: LINE, ARC, CIRCLE and POLYLINE. Curves the PDF draws as Béziers become ARCs
//! where they follow a circle within the tolerance (a PDF draws circles and arcs that way), else polylines within it.
//! It is plain text with a few comment lines on where it came from, so an agent reads it too.

use super::outline::{self, P};
use super::prims::Prim;
use hayro::kurbo::{ParamCurve, PathEl, PathSeg, Point};
use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

/// From page points to the footprint's millimetres in DXF's frame (Y up): the inverse of the footprint's placement
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToFootprint {
    /// The footprint's origin on the page, points
    pub origin: [f64; 2],
    /// Degrees, counter-clockwise on screen
    pub rotation: f64,
    /// Seen from below
    pub mirror: bool,
    pub mm_per_point: f64,
}

impl ToFootprint {
    /// A page point in footprint mm, KiCad's frame (Y down)
    pub fn footprint(&self, p: Point) -> P {
        let k = self.mm_per_point;
        let v = [(p.x - self.origin[0]) * k, (p.y - self.origin[1]) * k];
        let r = outline::rotate(v, -self.rotation);
        if self.mirror { [-r[0], r[1]] } else { r }
    }

    /// A page point in the DXF: footprint mm, Y up
    pub fn dxf(&self, p: Point) -> P {
        let [x, y] = self.footprint(p);
        [x, -y]
    }
}

/// A DXF entity, mm, Y up
#[derive(Clone, Debug, PartialEq)]
pub enum Entity {
    Line {
        a: P,
        b: P,
    },
    /// Counter-clockwise from `start` to `end`, degrees
    Arc {
        c: P,
        r: f64,
        start: f64,
        end: f64,
    },
    Circle {
        c: P,
        r: f64,
    },
    Polyline {
        points: Vec<P>,
        closed: bool,
    },
}

/// An arc of a circle while pieces are joined: centre, radius, the angle it starts at and its signed sweep, radians
#[derive(Clone, Copy, Debug)]
struct ArcPiece {
    c: P,
    r: f64,
    from: f64,
    sweep: f64,
}

impl ArcPiece {
    fn end(&self) -> P {
        let a = self.from + self.sweep;
        [self.c[0] + self.r * a.cos(), self.c[1] + self.r * a.sin()]
    }

    /// The arc as an entity: a circle when it goes all round, its ends no more than `tol` apart
    fn entity(&self, tol: f64) -> Entity {
        if self.sweep.abs() * self.r >= TAU * self.r - tol {
            return Entity::Circle {
                c: self.c,
                r: self.r,
            };
        }
        let (a, b) = if self.sweep >= 0.0 {
            (self.from, self.from + self.sweep)
        } else {
            (self.from + self.sweep, self.from)
        };
        Entity::Arc {
            c: self.c,
            r: self.r,
            start: degrees(a),
            end: degrees(b),
        }
    }
}

/// Radians as degrees in 0..360
fn degrees(a: f64) -> f64 {
    let d = a.to_degrees().rem_euclid(360.0);
    if d >= 360.0 - 1e-9 { 0.0 } else { d }
}

/// An angle into -π..=π
fn wrap(a: f64) -> f64 {
    let a = a.rem_euclid(TAU);
    if a > PI { a - TAU } else { a }
}

fn dist(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn pt(p: Point) -> P {
    [p.x, p.y]
}

/// The circle through three points, when they are not on one line
fn circle(a: P, b: P, c: P) -> Option<(P, f64)> {
    let d = 2.0 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    let scale = dist(a, b).max(dist(b, c)).max(dist(a, c));
    if d.abs() <= 1e-12 * scale * scale {
        return None;
    }
    let sq = |p: P| p[0] * p[0] + p[1] * p[1];
    let x = (sq(a) * (b[1] - c[1]) + sq(b) * (c[1] - a[1]) + sq(c) * (a[1] - b[1])) / d;
    let y = (sq(a) * (c[0] - b[0]) + sq(b) * (a[0] - c[0]) + sq(c) * (b[0] - a[0])) / d;
    let centre = [x, y];
    Some((centre, dist(centre, a)))
}

/// A curve that follows a circle within `tol` (mm): its arc. A curve nearly straight (a radius over 100 times the
/// chord) is no arc.
fn arc_of(seg: &PathSeg, tol: f64) -> Option<ArcPiece> {
    let (p0, pm, p1) = (pt(seg.eval(0.0)), pt(seg.eval(0.5)), pt(seg.eval(1.0)));
    let (c, r) = circle(p0, pm, p1)?;
    if r > 100.0 * dist(p0, p1).max(dist(p0, pm)) {
        return None;
    }
    let on = (1..16).all(|i| {
        let q = pt(seg.eval(i as f64 / 16.0));
        (dist(q, c) - r).abs() <= tol
    });
    if !on {
        return None;
    }
    let angle = |p: P| (p[1] - c[1]).atan2(p[0] - c[0]);
    let (a0, am, a1) = (angle(p0), angle(pm), angle(p1));
    Some(ArcPiece {
        c,
        r,
        from: a0,
        sweep: wrap(am - a0) + wrap(a1 - am),
    })
}

/// What one piece becomes while it is read: straight runs and arcs, in order
enum Part {
    Run(Vec<P>),
    Arc(ArcPiece),
}

/// The DXF's entities for `prims` (page points), through `to`; curves within `tol` mm
pub fn entities(prims: &[&Prim], to: &ToFootprint, tol: f64) -> Vec<Entity> {
    let tol = tol.max(1e-6);
    let mut out = Vec::new();
    for prim in prims {
        // The piece in DXF's frame: the map is affine, so curves stay curves
        let path: Vec<PathEl> = prim
            .path
            .elements()
            .iter()
            .map(|el| {
                let f = |p: Point| {
                    let [x, y] = to.dxf(p);
                    Point::new(x, y)
                };
                match *el {
                    PathEl::MoveTo(p) => PathEl::MoveTo(f(p)),
                    PathEl::LineTo(p) => PathEl::LineTo(f(p)),
                    PathEl::QuadTo(a, p) => PathEl::QuadTo(f(a), f(p)),
                    PathEl::CurveTo(a, b, p) => PathEl::CurveTo(f(a), f(b), f(p)),
                    PathEl::ClosePath => PathEl::ClosePath,
                }
            })
            .collect();
        let closed = matches!(path.last(), Some(PathEl::ClosePath));
        let mut parts: Vec<Part> = Vec::new();
        for seg in hayro::kurbo::segments(path.iter().copied()) {
            let (a, b) = (pt(seg.eval(0.0)), pt(seg.eval(1.0)));
            let arc = match seg {
                // A path closed where it already is draws no closing line
                PathSeg::Line(_) if dist(a, b) <= 1e-9 => continue,
                PathSeg::Line(_) => None,
                _ => arc_of(&seg, tol),
            };
            match arc {
                Some(arc) => {
                    // An arc that goes on round the same circle: one arc
                    if let Some(Part::Arc(last)) = parts.last_mut()
                        && dist(last.c, arc.c) <= tol
                        && (last.r - arc.r).abs() <= tol
                        && last.sweep.signum() == arc.sweep.signum()
                        && dist(last.end(), a) <= tol
                    {
                        last.sweep += arc.sweep;
                    } else {
                        parts.push(Part::Arc(arc));
                    }
                }
                None => {
                    let goes_on = matches!(parts.last(),
                        Some(Part::Run(run)) if run.last().is_some_and(|l| dist(*l, a) <= 1e-9));
                    if !goes_on {
                        parts.push(Part::Run(vec![a]));
                    }
                    let Some(Part::Run(run)) = parts.last_mut() else {
                        unreachable!("a run was just pushed")
                    };
                    match seg {
                        PathSeg::Line(_) => run.push(b),
                        _ => {
                            let el = match seg {
                                PathSeg::Quad(q) => PathEl::QuadTo(q.p1, q.p2),
                                PathSeg::Cubic(c) => PathEl::CurveTo(c.p1, c.p2, c.p3),
                                PathSeg::Line(l) => PathEl::LineTo(l.p1),
                            };
                            let start = PathEl::MoveTo(seg.eval(0.0));
                            hayro::kurbo::flatten([start, el], tol, |e| {
                                if let PathEl::LineTo(p) = e {
                                    run.push(pt(p));
                                }
                            });
                        }
                    }
                }
            }
        }
        // A closed piece of one straight run is a closed polyline; of one arc all round, a circle
        let whole = parts.len() == 1;
        for part in parts {
            match part {
                Part::Arc(arc) => out.push(arc.entity(tol)),
                Part::Run(mut points) => {
                    let round = closed
                        && whole
                        && points.len() > 3
                        && dist(points[0], points[points.len() - 1]) <= 1e-9;
                    if round {
                        points.pop();
                    }
                    match points.as_slice() {
                        [a, b] if !round => out.push(Entity::Line { a: *a, b: *b }),
                        [_] | [] => {}
                        _ => out.push(Entity::Polyline {
                            points,
                            closed: round,
                        }),
                    }
                }
            }
        }
    }
    out
}

/// A number as the DXF gives it: micrometres and finer, no needless zeros
fn num(v: f64) -> String {
    let s = format!("{:.6}", if v.abs() < 5e-7 { 0.0 } else { v });
    let s = s.trim_end_matches('0');
    match s.strip_suffix('.') {
        Some(s) => format!("{s}.0"),
        None => s.to_string(),
    }
}

/// The DXF file (R12 ASCII, mm) of `entities`, `comments` first as comment lines
pub fn write(entities: &[Entity], comments: &[String]) -> String {
    let mut s = String::new();
    let mut g = |code: i32, v: &str| {
        let _ = writeln!(s, "{code:>3}\n{v}");
    };
    for c in comments {
        g(999, &c.replace(['\n', '\r'], " "));
    }
    g(0, "SECTION");
    g(2, "HEADER");
    g(9, "$ACADVER");
    g(1, "AC1009");
    // Millimetres (the units' code is from later versions; readers that know it take mm from it)
    g(9, "$INSUNITS");
    g(70, "4");
    g(9, "$MEASUREMENT");
    g(70, "1");
    g(0, "ENDSEC");
    g(0, "SECTION");
    g(2, "ENTITIES");
    for e in entities {
        match e {
            Entity::Line { a, b } => {
                g(0, "LINE");
                g(8, "0");
                for (code, v) in [
                    (10, a[0]),
                    (20, a[1]),
                    (30, 0.0),
                    (11, b[0]),
                    (21, b[1]),
                    (31, 0.0),
                ] {
                    g(code, &num(v));
                }
            }
            Entity::Arc { c, r, start, end } => {
                g(0, "ARC");
                g(8, "0");
                for (code, v) in [
                    (10, c[0]),
                    (20, c[1]),
                    (30, 0.0),
                    (40, *r),
                    (50, *start),
                    (51, *end),
                ] {
                    g(code, &num(v));
                }
            }
            Entity::Circle { c, r } => {
                g(0, "CIRCLE");
                g(8, "0");
                for (code, v) in [(10, c[0]), (20, c[1]), (30, 0.0), (40, *r)] {
                    g(code, &num(v));
                }
            }
            Entity::Polyline { points, closed } => {
                g(0, "POLYLINE");
                g(8, "0");
                g(66, "1");
                for (code, v) in [(10, 0.0), (20, 0.0), (30, 0.0)] {
                    g(code, &num(v));
                }
                g(70, if *closed { "1" } else { "0" });
                for p in points {
                    g(0, "VERTEX");
                    g(8, "0");
                    for (code, v) in [(10, p[0]), (20, p[1]), (30, 0.0)] {
                        g(code, &num(v));
                    }
                }
                g(0, "SEQEND");
                g(8, "0");
            }
        }
    }
    g(0, "ENDSEC");
    g(0, "EOF");
    s
}

/// A DXF read back: its version and the entities this module writes, in order; what it does not know is skipped
#[cfg(test)]
pub fn read(text: &str) -> Result<(String, Vec<Entity>), String> {
    let lines: Vec<&str> = text.lines().collect();
    if !lines.len().is_multiple_of(2) {
        return Err("an odd number of lines: group codes and values come in pairs".into());
    }
    let pairs: Vec<(i32, &str)> = lines
        .chunks(2)
        .map(|c| {
            c[0].trim()
                .parse::<i32>()
                .map(|code| (code, c[1].trim()))
                .map_err(|_| format!("`{}` is no group code", c[0]))
        })
        .collect::<Result<_, _>>()?;
    let version = pairs
        .windows(2)
        .find(|w| w[0] == (9, "$ACADVER"))
        .map(|w| w[1].1.to_string())
        .ok_or("no $ACADVER")?;
    // The entities, each its group codes up to the next 0
    let start = pairs
        .windows(2)
        .position(|w| w[0] == (0, "SECTION") && w[1] == (2, "ENTITIES"))
        .ok_or("no ENTITIES section")?
        + 2;
    let mut records: Vec<(&str, Vec<(i32, f64)>)> = Vec::new();
    for &(code, v) in &pairs[start..] {
        if code == 0 {
            if v == "ENDSEC" {
                break;
            }
            records.push((v, Vec::new()));
        } else if let Some((_, values)) = records.last_mut()
            && let Ok(x) = v.parse::<f64>()
        {
            values.push((code, x));
        }
    }
    let get = |values: &[(i32, f64)], code: i32| -> Result<f64, String> {
        values
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, v)| *v)
            .ok_or(format!("no group {code}"))
    };
    let mut out = Vec::new();
    let mut poly: Option<(Vec<P>, bool)> = None;
    for (kind, v) in records {
        match kind {
            "LINE" => out.push(Entity::Line {
                a: [get(&v, 10)?, get(&v, 20)?],
                b: [get(&v, 11)?, get(&v, 21)?],
            }),
            "ARC" => out.push(Entity::Arc {
                c: [get(&v, 10)?, get(&v, 20)?],
                r: get(&v, 40)?,
                start: get(&v, 50)?,
                end: get(&v, 51)?,
            }),
            "CIRCLE" => out.push(Entity::Circle {
                c: [get(&v, 10)?, get(&v, 20)?],
                r: get(&v, 40)?,
            }),
            "POLYLINE" => poly = Some((Vec::new(), get(&v, 70).unwrap_or(0.0) as i64 & 1 == 1)),
            "VERTEX" => {
                poly.as_mut()
                    .ok_or("a VERTEX outside a POLYLINE")?
                    .0
                    .push([get(&v, 10)?, get(&v, 20)?]);
            }
            "SEQEND" => {
                let (points, closed) = poly.take().ok_or("a SEQEND outside a POLYLINE")?;
                out.push(Entity::Polyline { points, closed });
            }
            _ => {}
        }
    }
    Ok((version, out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hayro::kurbo::BezPath;

    /// A circle as PDFs draw one: four Béziers, each a quarter, counter-clockwise on the page
    pub(crate) fn bezier_circle(c: [f64; 2], r: f64) -> BezPath {
        let k = 0.552_284_75 * r;
        let [x, y] = c;
        let mut p = BezPath::new();
        p.move_to((x + r, y));
        p.curve_to((x + r, y - k), (x + k, y - r), (x, y - r));
        p.curve_to((x - k, y - r), (x - r, y - k), (x - r, y));
        p.curve_to((x - r, y + k), (x - k, y + r), (x, y + r));
        p.curve_to((x + k, y + r), (x + r, y + k), (x + r, y));
        p.close_path();
        p
    }

    fn prim(path: BezPath) -> Prim {
        Prim {
            path,
            filled: false,
            width: 0.5,
        }
    }

    fn close(a: P, b: P) -> bool {
        dist(a, b) < 1e-4
    }

    /// The placement's inverse: the footprint's origin is 0, 0; a point to its right on screen is +X, below it -Y
    /// in the DXF; turned 90° counter-clockwise on screen, the footprint's +X points up the page; bottom view
    /// mirrors X
    #[test]
    fn page_points_into_footprint_mm() {
        let mut to = ToFootprint {
            origin: [100.0, 200.0],
            rotation: 0.0,
            mirror: false,
            mm_per_point: 0.02,
        };
        assert!(close(to.dxf(Point::new(100.0, 200.0)), [0.0, 0.0]));
        assert!(close(to.dxf(Point::new(150.0, 250.0)), [1.0, -1.0]));
        to.rotation = 90.0;
        // The footprint's (1, 0) mm is drawn 50 points up the page
        assert!(close(to.footprint(Point::new(100.0, 150.0)), [1.0, 0.0]));
        to.rotation = 0.0;
        to.mirror = true;
        assert!(close(to.footprint(Point::new(150.0, 250.0)), [-1.0, 1.0]));
    }

    /// A circle drawn as four Béziers is one CIRCLE; a quarter of it an ARC, counter-clockwise in the DXF; a wavy
    /// curve a polyline within the tolerance; a square a closed polyline; a line a LINE. All read back the same.
    #[test]
    fn pieces_into_entities_and_back() {
        let to = ToFootprint {
            origin: [0.0, 0.0],
            rotation: 0.0,
            mirror: false,
            mm_per_point: 0.1,
        };
        // As a PDF draws it: four Béziers
        let circle_path = bezier_circle([100.0, 100.0], 20.0);
        // A quarter: from (120, 100) down the page to (100, 120), clockwise on the page (Y down)
        let mut quarter = BezPath::new();
        let k = 0.552_284_75 * 20.0;
        quarter.move_to((120.0, 100.0));
        quarter.curve_to((120.0, 100.0 + k), (100.0 + k, 120.0), (100.0, 120.0));
        let mut wave = BezPath::new();
        wave.move_to((0.0, 0.0));
        wave.curve_to((10.0, 30.0), (20.0, -30.0), (30.0, 0.0));
        let mut square = BezPath::new();
        square.move_to((0.0, 0.0));
        square.line_to((10.0, 0.0));
        square.line_to((10.0, 10.0));
        square.line_to((0.0, 10.0));
        square.close_path();
        let mut line = BezPath::new();
        line.move_to((0.0, 0.0));
        line.line_to((50.0, 0.0));
        let prims: Vec<Prim> = [circle_path, quarter, wave, square, line]
            .into_iter()
            .map(prim)
            .collect();
        let refs: Vec<&Prim> = prims.iter().collect();
        let es = entities(&refs, &to, 0.005);
        assert_eq!(es.len(), 5, "{es:#?}");
        match &es[0] {
            Entity::Circle { c, r } => assert!(
                close(*c, [10.0, -10.0]) && (r - 2.0).abs() < 1e-3,
                "{:?}",
                es[0]
            ),
            e => panic!("{e:?}"),
        }
        match &es[1] {
            // In the DXF (Y up) the quarter goes from (12, -10) to (10, -12): counter-clockwise from 270° to 360°
            Entity::Arc { c, r, start, end } => {
                assert!(
                    close(*c, [10.0, -10.0]) && (r - 2.0).abs() < 1e-3,
                    "{:?}",
                    es[1]
                );
                assert!(
                    (start - 270.0).abs() < 0.01 && end.abs() < 0.01,
                    "{:?}",
                    es[1]
                );
            }
            e => panic!("{e:?}"),
        }
        match &es[2] {
            Entity::Polyline {
                points,
                closed: false,
            } => {
                assert!(points.len() > 5, "{points:?}");
                assert!(close(points[0], [0.0, 0.0]) && close(*points.last().unwrap(), [3.0, 0.0]));
            }
            e => panic!("{e:?}"),
        }
        assert_eq!(
            es[3],
            Entity::Polyline {
                points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, -1.0], [0.0, -1.0]],
                closed: true
            }
        );
        assert_eq!(
            es[4],
            Entity::Line {
                a: [0.0, 0.0],
                b: [5.0, 0.0]
            }
        );
        let text = write(&es, &["from a test".into()]);
        assert!(
            text.starts_with("999\nfrom a test\n  0\nSECTION\n"),
            "{text}"
        );
        let (version, back) = read(&text).unwrap();
        assert_eq!(version, "AC1009");
        assert_eq!(back.len(), es.len());
        for (a, b) in back.iter().zip(&es) {
            match (a, b) {
                (
                    Entity::Polyline {
                        points: p,
                        closed: c,
                    },
                    Entity::Polyline {
                        points: q,
                        closed: d,
                    },
                ) => {
                    assert_eq!(c, d);
                    assert!(p.iter().zip(q).all(|(a, b)| close(*a, *b)));
                }
                _ => assert_eq!(format!("{a:.4?}"), format!("{b:.4?}")),
            }
        }
    }
}
