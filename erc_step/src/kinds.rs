//! The exact kinds of faces and edges, read from the STEP entities and placed in the model frame in mm.

use crate::entities::{Entities, entity, logical, real};
use crate::math::{Transform, normalize, scale};
use crate::{CurveKind, SurfaceKind};

/// Where a part's local coordinates go: scale to mm, then the part's transform.
#[derive(Clone, Copy)]
pub struct Frame {
    pub transform: Transform,
    pub unit_mm: f64,
    pub angle_rad: f64,
}

impl Frame {
    pub fn point(&self, p: crate::Vec3) -> crate::Vec3 {
        self.transform.apply_point(scale(p, self.unit_mm))
    }

    pub fn direction(&self, d: crate::Vec3) -> crate::Vec3 {
        normalize(self.transform.apply_vector(d)).unwrap_or(d)
    }
}

/// The surface of a face (`ADVANCED_FACE` or `FACE_SURFACE`). `orientation` is false when an oriented face or
/// shell turns it over.
pub fn surface(ents: &Entities<'_>, face: u64, orientation: bool, f: &Frame) -> SurfaceKind {
    let p = ents
        .simple(face, "ADVANCED_FACE")
        .or_else(|| ents.simple(face, "FACE_SURFACE"));
    let Some(p) = p else {
        return SurfaceKind::Freeform;
    };
    let same_sense = p.get(3).and_then(logical).unwrap_or(true);
    let outward = same_sense == orientation;
    let sign = if outward { 1.0 } else { -1.0 };
    let Some(geometry) = p.get(2).and_then(entity) else {
        return SurfaceKind::Freeform;
    };
    let Some(keyword) = ents.keyword(geometry) else {
        return SurfaceKind::Freeform;
    };
    let Some(q) = ents.simple(geometry, keyword) else {
        return SurfaceKind::Freeform;
    };
    let Some(ax) = q.get(1).and_then(entity).and_then(|a| ents.axis2(a)) else {
        return SurfaceKind::Freeform;
    };
    let origin = f.point(ax.translation);
    let z = f.direction(ax.apply_vector([0.0, 0.0, 1.0]));
    let len = |i: usize| q.get(i).and_then(real).map(|r| r * f.unit_mm);
    match keyword {
        "PLANE" => SurfaceKind::Plane {
            origin,
            normal: scale(z, sign),
        },
        "CYLINDRICAL_SURFACE" => match len(2) {
            Some(radius) => SurfaceKind::Cylinder {
                axis_origin: origin,
                axis: z,
                radius,
                outward,
            },
            None => SurfaceKind::Freeform,
        },
        "CONICAL_SURFACE" => match (len(2), q.get(3).and_then(real)) {
            (Some(radius), Some(a)) => SurfaceKind::Cone {
                origin,
                axis: z,
                radius,
                half_angle: a * f.angle_rad,
            },
            _ => SurfaceKind::Freeform,
        },
        "SPHERICAL_SURFACE" => match len(2) {
            Some(radius) => SurfaceKind::Sphere {
                centre: origin,
                radius,
                outward,
            },
            None => SurfaceKind::Freeform,
        },
        "TOROIDAL_SURFACE" | "DEGENERATE_TOROIDAL_SURFACE" => match (len(2), len(3)) {
            (Some(major_radius), Some(minor_radius)) => SurfaceKind::Torus {
                centre: origin,
                axis: z,
                major_radius,
                minor_radius,
            },
            _ => SurfaceKind::Freeform,
        },
        _ => SurfaceKind::Freeform,
    }
}

/// The 3D curve under an edge's geometry: a surface or seam curve's own curve.
pub fn curve_3d(ents: &Entities<'_>, mut geometry: u64) -> u64 {
    for _ in 0..4 {
        let inner = [
            "SURFACE_CURVE",
            "SEAM_CURVE",
            "INTERSECTION_CURVE",
            "BOUNDED_SURFACE_CURVE",
        ]
        .iter()
        .find_map(|k| ents.simple(geometry, k))
        .and_then(|p| p.get(1))
        .and_then(entity);
        match inner {
            Some(g) => geometry = g,
            None => break,
        }
    }
    geometry
}

/// The curve of an edge (`EDGE_CURVE`).
pub fn curve(ents: &Entities<'_>, edge: u64, f: &Frame) -> CurveKind {
    let Some(p) = ents.simple(edge, "EDGE_CURVE") else {
        return CurveKind::Other;
    };
    let vertex = |i: usize| {
        let v = p.get(i).and_then(entity)?;
        let pt = ents.simple(v, "VERTEX_POINT")?.get(1).and_then(entity)?;
        ents.point(pt).map(|x| f.point(x))
    };
    let Some(geometry) = p.get(3).and_then(entity) else {
        return CurveKind::Other;
    };
    let geometry = curve_3d(ents, geometry);
    if ents.simple(geometry, "LINE").is_some() {
        return match (vertex(1), vertex(2)) {
            (Some(start), Some(end)) => CurveKind::Line { start, end },
            _ => CurveKind::Other,
        };
    }
    if let Some(q) = ents.simple(geometry, "CIRCLE") {
        let ax = q.get(1).and_then(entity).and_then(|a| ents.axis2(a));
        if let (Some(ax), Some(r)) = (ax, q.get(2).and_then(real)) {
            return CurveKind::Circle {
                centre: f.point(ax.translation),
                axis: f.direction(ax.apply_vector([0.0, 0.0, 1.0])),
                radius: r * f.unit_mm,
            };
        }
    }
    CurveKind::Other
}
