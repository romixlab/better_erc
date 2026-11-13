//! Measurements on the exact kinds of faces and edges: distances in mm, angles in radians. Each returns its value
//! and the two points to draw it between.

use crate::math::{Vec3, add, cross, distance, dot, norm, normalize, scale, sub};
use crate::{CurveKind, Edge, SurfaceKind};

/// Planes closer to parallel than this (in radians) count as parallel.
pub const PARALLEL: f64 = 1e-3;

/// A measured value and where to draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measurement {
    /// mm for a distance, radians for an angle.
    pub value: f64,
    pub from: Vec3,
    pub to: Vec3,
}

/// A plane through `origin` with the unit `normal`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub origin: Vec3,
    pub normal: Vec3,
}

/// A line through `origin` along the unit `direction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl SurfaceKind {
    pub fn plane(&self) -> Option<Plane> {
        match *self {
            SurfaceKind::Plane { origin, normal } => Some(Plane { origin, normal }),
            _ => None,
        }
    }

    /// The axis of a cylinder, cone or torus.
    pub fn axis(&self) -> Option<Axis> {
        match *self {
            SurfaceKind::Cylinder {
                axis_origin: origin,
                axis,
                ..
            }
            | SurfaceKind::Cone { origin, axis, .. }
            | SurfaceKind::Torus {
                centre: origin,
                axis,
                ..
            } => Some(Axis {
                origin,
                direction: axis,
            }),
            _ => None,
        }
    }
}

impl CurveKind {
    /// The axis of a circle.
    pub fn axis(&self) -> Option<Axis> {
        match *self {
            CurveKind::Circle { centre, axis, .. } => Some(Axis {
                origin: centre,
                direction: axis,
            }),
            _ => None,
        }
    }
}

impl Plane {
    /// The signed distance of `p` from the plane, positive on the normal's side.
    pub fn signed_distance(&self, p: Vec3) -> f64 {
        dot(sub(p, self.origin), self.normal)
    }

    /// The foot of `p` on the plane.
    pub fn project(&self, p: Vec3) -> Vec3 {
        sub(p, scale(self.normal, self.signed_distance(p)))
    }
}

impl Axis {
    /// The foot of `p` on the line.
    pub fn project(&self, p: Vec3) -> Vec3 {
        add(
            self.origin,
            scale(self.direction, dot(sub(p, self.origin), self.direction)),
        )
    }
}

/// The angle between two directions, 0 to π.
fn angle(a: Vec3, b: Vec3) -> f64 {
    norm(cross(a, b)).atan2(dot(a, b))
}

/// The distance between two parallel planes, drawn from `near`'s foot on `a` to the foot of that on `b`. `None`
/// when they are not parallel.
pub fn plane_to_plane(a: Plane, b: Plane, near: Vec3) -> Option<Measurement> {
    let t = angle(a.normal, b.normal);
    if t > PARALLEL && t < std::f64::consts::PI - PARALLEL {
        return None;
    }
    let from = a.project(near);
    let to = b.project(from);
    Some(Measurement {
        value: distance(from, to),
        from,
        to,
    })
}

/// The distance from a point to a plane, drawn from the point to its foot.
pub fn plane_to_point(a: Plane, p: Vec3) -> Measurement {
    let to = a.project(p);
    Measurement {
        value: distance(p, to),
        from: p,
        to,
    }
}

/// A cylinder's (or circle's) diameter, drawn across it through the point of the axis nearest to `near`, towards
/// `near`.
pub fn diameter(axis: Axis, radius: f64, near: Vec3) -> Measurement {
    let centre = axis.project(near);
    let towards =
        normalize(sub(near, centre)).unwrap_or_else(|| crate::math::perpendicular(axis.direction));
    Measurement {
        value: 2.0 * radius,
        from: add(centre, scale(towards, radius)),
        to: sub(centre, scale(towards, radius)),
    }
}

/// The diameter of a cylinder face; `None` for another kind.
pub fn cylinder_diameter(surface: &SurfaceKind, near: Vec3) -> Option<Measurement> {
    match *surface {
        SurfaceKind::Cylinder {
            axis_origin,
            axis,
            radius,
            ..
        } => Some(diameter(
            Axis {
                origin: axis_origin,
                direction: axis,
            },
            radius,
            near,
        )),
        _ => None,
    }
}

/// The diameter of a circle edge; `None` for another kind.
pub fn circle_diameter(curve: &CurveKind, near: Vec3) -> Option<Measurement> {
    match *curve {
        CurveKind::Circle {
            centre,
            axis,
            radius,
        } => Some(diameter(
            Axis {
                origin: centre,
                direction: axis,
            },
            radius,
            near,
        )),
        _ => None,
    }
}

/// The shortest distance between two lines, drawn between their nearest points: the pitch of two pins. For
/// parallel lines, between `a`'s origin's foot on `b` and the origin.
pub fn axis_to_axis(a: Axis, b: Axis) -> Measurement {
    let w = sub(a.origin, b.origin);
    let n = cross(a.direction, b.direction);
    let from = if norm(n) < PARALLEL {
        a.origin
    } else {
        // the point of a nearest to b
        let (aa, ab, bb) = (
            dot(a.direction, a.direction),
            dot(a.direction, b.direction),
            dot(b.direction, b.direction),
        );
        let (d, e) = (dot(a.direction, w), dot(b.direction, w));
        let s = (ab * e - bb * d) / (aa * bb - ab * ab);
        add(a.origin, scale(a.direction, s))
    };
    let to = b.project(from);
    Measurement {
        value: distance(from, to),
        from,
        to,
    }
}

/// The length of an edge: exact for a line, along its polyline otherwise.
pub fn edge_length(edge: &Edge) -> Option<Measurement> {
    if let CurveKind::Line { start, end } = edge.curve {
        return Some(point_to_point(start, end));
    }
    let (first, last) = (*edge.polyline.first()?, *edge.polyline.last()?);
    let value = edge.polyline.windows(2).map(|w| distance(w[0], w[1])).sum();
    Some(Measurement {
        value,
        from: first,
        to: last,
    })
}

pub fn point_to_point(a: Vec3, b: Vec3) -> Measurement {
    Measurement {
        value: distance(a, b),
        from: a,
        to: b,
    }
}

/// The angle between two planes, 0 to π/2, drawn between `near`'s feet on them.
pub fn angle_between_planes(a: Plane, b: Plane, near: Vec3) -> Measurement {
    let t = angle(a.normal, b.normal);
    Measurement {
        value: t.min(std::f64::consts::PI - t),
        from: a.project(near),
        to: b.project(near),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn planes() {
        let a = Plane {
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        };
        let b = Plane {
            origin: [5.0, 5.0, 1.27],
            normal: [0.0, 0.0, -1.0],
        };
        let m = plane_to_plane(a, b, [1.0, 2.0, 3.0]).unwrap();
        assert!(close(m.value, 1.27));
        assert_eq!(m.from, [1.0, 2.0, 0.0]);
        assert!(close(m.to[2], 1.27));
        let tilted = Plane {
            origin: [0.0; 3],
            normal: normalize([0.0, 1.0, 1.0]).unwrap(),
        };
        assert!(plane_to_plane(a, tilted, [0.0; 3]).is_none());
        assert!(close(
            angle_between_planes(a, tilted, [0.0; 3]).value,
            std::f64::consts::FRAC_PI_4
        ));
        assert!(close(plane_to_point(a, [3.0, 4.0, -2.0]).value, 2.0));
    }

    #[test]
    fn axes_and_diameters() {
        // two pins 2.54 apart along x, both along z
        let a = Axis {
            origin: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
        };
        let b = Axis {
            origin: [2.54, 0.0, 7.0],
            direction: [0.0, 0.0, -1.0],
        };
        let m = axis_to_axis(a, b);
        assert!(close(m.value, 2.54));
        // skew lines: x axis and a line along y at z = 3
        let c = Axis {
            origin: [0.0, 0.0, 0.0],
            direction: [1.0, 0.0, 0.0],
        };
        let d = Axis {
            origin: [4.0, -1.0, 3.0],
            direction: [0.0, 1.0, 0.0],
        };
        let m = axis_to_axis(c, d);
        assert!(close(m.value, 3.0));
        assert!(close(m.from[0], 4.0) && close(m.to[2], 3.0));
        let cyl = SurfaceKind::Cylinder {
            axis_origin: [0.0; 3],
            axis: [0.0, 0.0, 1.0],
            radius: 0.4,
            outward: true,
        };
        let m = cylinder_diameter(&cyl, [1.0, 0.0, 2.0]).unwrap();
        assert!(close(m.value, 0.8));
        assert_eq!(m.from, [0.4, 0.0, 2.0]);
        assert_eq!(m.to, [-0.4, 0.0, 2.0]);
    }
}
