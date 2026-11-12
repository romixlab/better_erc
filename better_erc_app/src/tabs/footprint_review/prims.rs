//! A datasheet page's drawing as the pieces the PDF draws, for the Select mode: each subpath of a path (a line, a
//! polyline, an arc or curve, a closed shape) is one piece, kept with its curves as they are, in page points, y
//! down. A click picks the piece nearest the pointer (inside a filled shape counts when no outline is that near),
//! a box the pieces wholly inside it.

use hayro::kurbo::{BezPath, ParamCurveNearest, PathEl, Point, Rect, Shape};

/// A piece the page draws as one: a subpath, its curves kept
#[derive(Clone, Debug, PartialEq)]
pub struct Prim {
    /// Starts with its MoveTo; a closed one ends with ClosePath
    pub path: BezPath,
    /// The PDF fills it (a filled shape is picked from inside too)
    pub filled: bool,
    /// The stroke's width, points; 0 for a filled shape's outline
    pub width: f32,
}

/// How closely a curve is followed when one is drawn or tested, points
const FLAT: f64 = 0.05;

impl Prim {
    /// Its box, page points
    pub fn bbox(&self) -> Rect {
        self.path.bounding_box()
    }

    /// How far `p` is from its centre line (its outline for a filled shape), points
    pub fn distance(&self, p: [f32; 2]) -> f64 {
        let p = Point::new(p[0] as f64, p[1] as f64);
        self.path
            .segments()
            .map(|s| s.nearest(p, 1e-3).distance_sq.sqrt())
            .fold(f64::INFINITY, f64::min)
    }

    /// `p` inside the filled shape
    pub fn inside(&self, p: [f32; 2]) -> bool {
        self.filled && self.path.contains(Point::new(p[0] as f64, p[1] as f64))
    }

    /// Its points as straight pieces within `tol` points, to draw it; a closed one ends where it starts
    pub fn polyline(&self, tol: f64) -> Vec<[f32; 2]> {
        let mut out = Vec::new();
        let mut start = None;
        hayro::kurbo::flatten(self.path.iter(), tol.max(1e-4), |el| match el {
            PathEl::MoveTo(p) => {
                start = Some(p);
                out.push([p.x as f32, p.y as f32]);
            }
            PathEl::LineTo(p) => out.push([p.x as f32, p.y as f32]),
            PathEl::ClosePath => {
                if let Some(s) = start {
                    out.push([s.x as f32, s.y as f32]);
                }
            }
            _ => {}
        });
        out
    }
}

/// The piece a click at `p` picks, within `reach` points: the nearest by its line; else the smallest filled shape
/// `p` is inside
pub fn pick(prims: &[Prim], p: [f32; 2], reach: f32) -> Option<usize> {
    let reach = reach as f64;
    let near = prims
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let b = s
                .bbox()
                .inflate(reach + s.width as f64, reach + s.width as f64);
            b.contains(Point::new(p[0] as f64, p[1] as f64))
        })
        .map(|(i, s)| (i, (s.distance(p) - s.width as f64 / 2.0).max(0.0)))
        .filter(|(_, d)| *d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i);
    near.or_else(|| {
        prims
            .iter()
            .enumerate()
            .filter(|(_, s)| s.inside(p))
            .min_by(|a, b| a.1.bbox().area().total_cmp(&b.1.bbox().area()))
            .map(|(i, _)| i)
    })
}

/// The pieces wholly inside the box from `a` to `b` (any corners), page points
pub fn in_box(prims: &[Prim], a: [f32; 2], b: [f32; 2]) -> Vec<usize> {
    let r = Rect::new(a[0] as f64, a[1] as f64, b[0] as f64, b[1] as f64).abs();
    prims
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let bb = s.bbox();
            r.contains(bb.origin()) && r.contains(Point::new(bb.x1, bb.y1))
        })
        .map(|(i, _)| i)
        .collect()
}

/// A piece drawn on screen: its points within a quarter of a screen point
pub fn screen_tolerance(zoom: f32) -> f64 {
    (0.25 / zoom.max(1e-3) as f64).min(FLAT * 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: [f64; 2], b: [f64; 2]) -> Prim {
        let mut path = BezPath::new();
        path.move_to((a[0], a[1]));
        path.line_to((b[0], b[1]));
        Prim {
            path,
            filled: false,
            width: 0.5,
        }
    }

    fn square(x: f64, y: f64, s: f64) -> Prim {
        let mut path = BezPath::new();
        path.move_to((x, y));
        path.line_to((x + s, y));
        path.line_to((x + s, y + s));
        path.line_to((x, y + s));
        path.close_path();
        Prim {
            path,
            filled: true,
            width: 0.0,
        }
    }

    #[test]
    fn a_click_picks_the_nearest_line_else_the_shape_it_is_in() {
        let prims = vec![
            line([0.0, 0.0], [100.0, 0.0]),
            line([0.0, 10.0], [100.0, 10.0]),
            square(200.0, 200.0, 50.0),
            square(210.0, 210.0, 10.0),
        ];
        assert_eq!(pick(&prims, [50.0, 3.0], 5.0), Some(0));
        assert_eq!(pick(&prims, [50.0, 8.0], 5.0), Some(1));
        assert_eq!(pick(&prims, [50.0, 30.0], 5.0), None);
        // Inside both squares, far from their edges: the smaller
        assert_eq!(pick(&prims, [215.0, 215.0], 1.0), Some(3));
        assert_eq!(pick(&prims, [240.0, 240.0], 1.0), Some(2));
    }

    #[test]
    fn a_box_takes_what_is_wholly_inside() {
        let prims = vec![
            line([0.0, 0.0], [100.0, 0.0]),
            line([0.0, 10.0], [40.0, 10.0]),
            square(20.0, 20.0, 10.0),
        ];
        assert_eq!(in_box(&prims, [50.0, 40.0], [-1.0, 5.0]), [1, 2]);
        assert_eq!(prims[2].polyline(0.1).len(), 5);
    }
}
