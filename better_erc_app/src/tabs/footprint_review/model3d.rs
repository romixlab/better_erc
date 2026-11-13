//! The footprint's 3D model in the review (VIEW-23): the STEP models its `(model ...)` entries name, read by
//! [`erc_step`] on a worker thread, placed as KiCad places them ([`better_erc::fp::model`]) over a board with the
//! pads' copper on it, and shown in a [`ve_view3d::View3d`] in place of the page (the 3D key swaps them).
//!
//! **Measure model** is a mode of its own: a click picks a face or an edge, a second click another. What lies
//! between them is measured on the exact B-rep kinds the STEP file writes, not on the triangles:
//!
//! - two parallel planes: their distance; two planes at an angle: the angle;
//! - two cylinders, circles, cones or tori: the distance of their axes (a pin pitch);
//! - a plane and an axis parallel to it: the axis' height over the plane; a plane and anything else: the point's;
//! - one face or edge picked twice: a cylinder's or circle's diameter, an edge's length;
//! - otherwise the straight distance between the two points.
//!
//! The result shows on the view and goes into the review note as `M3 = 1.270 mm (pitch, axis to axis)`, numbered
//! with the rulers' measurements.
//!
//! **The frame** is KiCad's 3D frame of the footprint (x right, y up, z up out of the board, the origin at the
//! footprint's origin on the board's top surface), in mm.

use super::*;
use better_erc::fp::model::{self as fpm, FootprintModels, ViewModels};
use eframe::egui_wgpu;
use erc_step::math::{dot, normalize};
use erc_step::measure::{self as m, Axis, Plane};
use erc_step::{CurveKind, SurfaceKind, Transform, Vec3 as V3};
use ve_view3d::glam::Vec3;
use ve_view3d::{Item, LineSet, Mark, Mesh, Pick, Scene, View3d, ViewPreset};

/// The board under the model, mm thick
pub const BOARD_MM: f64 = 1.6;
/// The pads' copper over the board's top surface, mm
pub const COPPER_MM: f64 = 0.035;
/// The board reaches this far beyond the model and the pads, mm
const MARGIN_MM: f64 = 1.0;

const BOARD_COLOUR: Color32 = Color32::from_rgb(28, 82, 52);
const COPPER_COLOUR: Color32 = Color32::from_rgb(200, 150, 70);
const MODEL_COLOUR: Color32 = Color32::from_gray(170);
const EDGE_COLOUR: Color32 = Color32::from_gray(25);

/// The mesh of the pads in [`ModelScene::scene`], after the board's (0); the model's parts follow
pub const PADS_MESH: usize = 1;

/// The app's wgpu render state, for views made later than the app (the review tab makes its view on first use)
fn render_state_id() -> Id {
    Id::new("footprint review: wgpu render state")
}

/// Keeps the app's wgpu render state in `ctx` for the 3D view
pub fn register(ctx: &egui::Context, rs: &egui_wgpu::RenderState) {
    ctx.data_mut(|d| d.insert_temp(render_state_id(), rs.clone()));
}

pub(super) fn render_state(ctx: &egui::Context) -> Option<egui_wgpu::RenderState> {
    ctx.data(|d| d.get_temp::<egui_wgpu::RenderState>(render_state_id()))
}

/// What a face or an edge of the scene is, exactly
#[derive(Clone, Debug, PartialEq)]
pub enum Exact {
    Surface(SurfaceKind),
    Curve { curve: CurveKind, polyline: Vec<V3> },
}

/// One mesh of the scene: what it is, and each face's surface (in the footprint frame)
#[derive(Clone, Debug, Default)]
pub struct MeshInfo {
    pub name: String,
    pub faces: Vec<SurfaceKind>,
}

/// The footprint's models as a scene, with the exact kinds of its faces and edges
pub struct ModelScene {
    /// The footprint's lib id and hash it was made for
    pub key: String,
    /// Board, pads, then each part of each model; `None` once handed to the view
    pub scene: Option<Scene>,
    /// One per mesh of the scene
    pub meshes: Vec<MeshInfo>,
    /// The model's edges, one per polyline of line set 0: the part's name, the curve and its points
    pub edges: Vec<(String, CurveKind, Vec<V3>)>,
    /// The analysis of each model entry (`berc fp model`'s)
    pub report: FootprintModels,
    /// Why entries are not shown or not measured exactly, one line each
    pub notes: Vec<String>,
    /// How many models are in the scene
    pub shown: usize,
    /// The model's six orthographic views for the page, by [`modelviews::Side::index`] (VIEW-24)
    pub views: Vec<modelviews::ProjectedView>,
    /// The model's parts and edges alone (no board, no pads), for the shaded model over the page (VIEW-27);
    /// `None` once handed to its renderer
    pub parts: Option<Scene>,
}

/// Colour of a STEP colour, 0 to 1 per channel
fn colour(c: Option<erc_step::Colour>) -> Option<Color32> {
    let c = c?;
    let u = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(Color32::from_rgb(u(c[0]), u(c[1]), u(c[2])))
}

fn v(p: V3) -> Vec3 {
    Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
}

/// The scale of an entry when it is the same along every axis
fn uniform_scale(e: &better_erc::model::layout::ModelEntry) -> Option<f64> {
    let s = e.scale;
    ((s.x - s.y).abs() < 1e-9 && (s.x - s.z).abs() < 1e-9 && s.x > 0.0).then_some(s.x)
}

fn direction(t: &Transform, d: V3) -> V3 {
    normalize(t.apply_vector(d)).unwrap_or(d)
}

/// A surface placed by `t` (uniform scale `s`); a scale that is not uniform keeps only the planes' points, so every
/// kind but a plane turns freeform and a plane's normal is worked out from two directions in it
fn place_surface(t: &Transform, s: Option<f64>, k: &SurfaceKind) -> SurfaceKind {
    use SurfaceKind::*;
    let Some(s) = s else {
        return match k {
            Plane { origin, normal } => {
                let a = erc_step::math::perpendicular(*normal);
                let b = erc_step::math::cross(*normal, a);
                let n = erc_step::math::cross(t.apply_vector(a), t.apply_vector(b));
                match normalize(n) {
                    Some(normal) => Plane {
                        origin: t.apply_point(*origin),
                        normal,
                    },
                    None => Freeform,
                }
            }
            _ => Freeform,
        };
    };
    match k.clone() {
        Plane { origin, normal } => Plane {
            origin: t.apply_point(origin),
            normal: direction(t, normal),
        },
        Cylinder {
            axis_origin,
            axis,
            radius,
            outward,
        } => Cylinder {
            axis_origin: t.apply_point(axis_origin),
            axis: direction(t, axis),
            radius: radius * s,
            outward,
        },
        Cone {
            origin,
            axis,
            radius,
            half_angle,
        } => Cone {
            origin: t.apply_point(origin),
            axis: direction(t, axis),
            radius: radius * s,
            half_angle,
        },
        Sphere {
            centre,
            radius,
            outward,
        } => Sphere {
            centre: t.apply_point(centre),
            radius: radius * s,
            outward,
        },
        Torus {
            centre,
            axis,
            major_radius,
            minor_radius,
        } => Torus {
            centre: t.apply_point(centre),
            axis: direction(t, axis),
            major_radius: major_radius * s,
            minor_radius: minor_radius * s,
        },
        Freeform => Freeform,
    }
}

fn place_curve(t: &Transform, s: Option<f64>, k: &CurveKind) -> CurveKind {
    match (k, s) {
        (CurveKind::Line { start, end }, _) => CurveKind::Line {
            start: t.apply_point(*start),
            end: t.apply_point(*end),
        },
        (
            CurveKind::Circle {
                centre,
                axis,
                radius,
            },
            Some(s),
        ) => CurveKind::Circle {
            centre: t.apply_point(*centre),
            axis: direction(t, *axis),
            radius: radius * s,
        },
        _ => CurveKind::Other,
    }
}

/// Triangles of a simple polygon (ear clipping), as indices into it; either winding
pub fn triangulate(poly: &[[f64; 2]]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    let ccw = area >= 0.0;
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let k = idx.len();
        let mut clipped = false;
        for i in 0..k {
            let (a, b, c) = (idx[(i + k - 1) % k], idx[i], idx[(i + 1) % k]);
            let turn = cross(poly[a], poly[b], poly[c]);
            if (turn > 0.0) != ccw || turn == 0.0 {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                if j == a || j == b || j == c {
                    return false;
                }
                let p = poly[j];
                let (d1, d2, d3) = (
                    cross(poly[a], poly[b], p),
                    cross(poly[b], poly[c], p),
                    cross(poly[c], poly[a], p),
                );
                if ccw {
                    d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
                } else {
                    d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0
                }
            });
            if inside {
                continue;
            }
            out.push([a, b, c]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            // Degenerate (collinear or self-touching): a fan over what is left
            for i in 1..idx.len() - 1 {
                out.push([idx[0], idx[i], idx[i + 1]]);
            }
            return out;
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// The flat plane of a mesh face built here: its first triangle's normal through its first corner
fn flat_kind(mesh: &Mesh, face: u32) -> SurfaceKind {
    let Some(t) = (0..mesh.triangles.len()).find(|&t| mesh.face(t) == face) else {
        return SurfaceKind::Freeform;
    };
    let Some([a, b, c]) = mesh.corners(t) else {
        return SurfaceKind::Freeform;
    };
    match (b - a).cross(c - a).try_normalize() {
        Some(n) => SurfaceKind::Plane {
            origin: [a.x as f64, a.y as f64, a.z as f64],
            normal: [n.x as f64, n.y as f64, n.z as f64],
        },
        None => SurfaceKind::Freeform,
    }
}

/// The scene of a footprint's models: the board (its top at z 0), the front pads' copper on it, and every part of
/// every model found and read, placed by its entry; the kinds of their faces and edges placed alike
pub fn build(key: String, vm: ViewModels, pads: &[(String, Vec<Vec<[f64; 2]>>)]) -> ModelScene {
    let mut notes = Vec::new();
    let mut parts: Vec<(Mesh, MeshInfo)> = Vec::new();
    let mut edges = Vec::new();
    let mut polylines = Vec::new();
    let mut shown = 0;
    for (report, model) in vm.report.models.iter().zip(&vm.models) {
        let path = &report.entry.path;
        let model = match (&report.resolved, model) {
            (_, Some(m)) => match &**m {
                Ok(m) => m,
                Err(e) => {
                    notes.push(format!("{path}: not read: {e}"));
                    continue;
                }
            },
            (fpm::Resolved::UnknownVar { var }, None) => {
                notes.push(format!("{path}: nothing sets ${{{var}}}"));
                continue;
            }
            (fpm::Resolved::NotFound { path: p }, None) => {
                notes.push(format!("{path}: no file at {}", p.display()));
                continue;
            }
            (fpm::Resolved::Found { .. }, None) => continue,
        };
        if report.entry.hide {
            notes.push(format!("{path}: hidden in the footprint, shown here"));
        }
        let t = fpm::placement(&report.entry);
        let s = uniform_scale(&report.entry);
        if s.is_none() {
            notes.push(format!(
                "{path}: scaled unevenly, so only planes measure exactly"
            ));
        }
        shown += 1;
        for part in &model.parts {
            let mut mesh = Mesh::default();
            let mut info = MeshInfo {
                name: part.name.clone(),
                faces: Vec::new(),
            };
            let part_colour = colour(part.colour);
            for (fi, face) in part.faces.iter().enumerate() {
                mesh.face_colors
                    .push(colour(face.colour).or(part_colour).unwrap_or(MODEL_COLOUR));
                info.faces.push(place_surface(&t, s, &face.surface));
                let Some(fm) = &face.mesh else { continue };
                let base = mesh.positions.len() as u32;
                mesh.positions
                    .extend(fm.positions.iter().map(|p| v(t.apply_point(*p))));
                // A scale that is not uniform bends normals: the flat shading of the triangles then
                mesh.normals.extend(
                    fm.normals
                        .iter()
                        .map(|n| v(if s.is_some() { direction(&t, *n) } else { *n })),
                );
                mesh.triangles
                    .extend(fm.triangles.iter().map(|tr| tr.map(|i| i + base)));
                mesh.face_of_triangle
                    .extend(std::iter::repeat_n(fi as u32, fm.triangles.len()));
            }
            if s.is_none() {
                mesh.normals.clear();
            }
            for e in &part.edges {
                let poly: Vec<V3> = e.polyline.iter().map(|p| t.apply_point(*p)).collect();
                polylines.push(poly.iter().map(|p| v(*p)).collect());
                edges.push((part.name.clone(), place_curve(&t, s, &e.curve), poly));
            }
            parts.push((mesh, info));
        }
    }

    // The pads' copper, one face each, just over the board
    let mut pad_mesh = Mesh::default();
    let mut pad_info = MeshInfo {
        name: "pads".into(),
        faces: Vec::new(),
    };
    let z = COPPER_MM as f32;
    for (_, polys) in pads {
        let face = pad_mesh.face_colors.len() as u32;
        pad_mesh.face_colors.push(COPPER_COLOUR);
        pad_info.faces.push(SurfaceKind::Plane {
            origin: [0.0, 0.0, COPPER_MM],
            normal: [0.0, 0.0, 1.0],
        });
        for poly in polys {
            let base = pad_mesh.positions.len() as u32;
            pad_mesh
                .positions
                .extend(poly.iter().map(|p| Vec3::new(p[0] as f32, p[1] as f32, z)));
            pad_mesh
                .normals
                .extend(std::iter::repeat_n(Vec3::Z, poly.len()));
            for [a, b, c] in triangulate(poly) {
                // Counter-clockwise seen from above
                let (pa, pb, pc) = (poly[a], poly[b], poly[c]);
                let up =
                    (pb[0] - pa[0]) * (pc[1] - pa[1]) - (pb[1] - pa[1]) * (pc[0] - pa[0]) >= 0.0;
                let tri = if up { [a, b, c] } else { [a, c, b] };
                pad_mesh.triangles.push(tri.map(|i| base + i as u32));
                pad_mesh.face_of_triangle.push(face);
            }
        }
    }

    // The board: under everything, a margin around it
    let mut b = ve_view3d::Bounds::EMPTY;
    for (m, _) in &parts {
        b.union(&m.bounds());
    }
    b.union(&pad_mesh.bounds());
    let (lo, hi) = if b.is_empty() {
        (Vec3::new(-2.0, -2.0, 0.0), Vec3::new(2.0, 2.0, 0.0))
    } else {
        (b.min, b.max)
    };
    let mg = MARGIN_MM as f32;
    let board = ve_view3d::shapes::cuboid(
        Vec3::new(lo.x - mg, lo.y - mg, -BOARD_MM as f32),
        Vec3::new(hi.x + mg, hi.y + mg, 0.0),
        BOARD_COLOUR,
    );
    let board_info = MeshInfo {
        name: "board".into(),
        faces: (0..board.face_count() as u32)
            .map(|f| flat_kind(&board, f))
            .collect(),
    };

    let part_meshes: Vec<&Mesh> = parts.iter().map(|(m, _)| m).collect();
    let views = modelviews::project_all(&part_meshes, &polylines);
    let parts_scene = Scene {
        meshes: part_meshes.iter().map(|m| (*m).clone()).collect(),
        lines: vec![LineSet {
            polylines: polylines.clone(),
            color: EDGE_COLOUR,
            width: 1.2,
        }],
    };
    let mut meshes = vec![board, pad_mesh];
    let mut infos = vec![board_info, pad_info];
    for (m, i) in parts {
        meshes.push(m);
        infos.push(i);
    }
    ModelScene {
        key,
        scene: Some(Scene {
            meshes,
            lines: vec![LineSet {
                polylines,
                color: EDGE_COLOUR,
                width: 1.2,
            }],
        }),
        meshes: infos,
        edges,
        report: vm.report,
        notes,
        shown,
        views,
        parts: Some(parts_scene),
    }
}

/// A face or an edge picked on the model, with what it is
#[derive(Clone, Debug, PartialEq)]
pub struct Picked {
    pub item: Item,
    pub exact: Exact,
    /// Where the pointer hit it, footprint frame mm
    pub point: V3,
    /// The part or the board it belongs to
    pub owner: String,
    /// The pad's number, for a pad
    pub pad: Option<String>,
}

impl ModelScene {
    /// What a pick of the view is
    pub fn picked(&self, p: &Pick, pads: &[(String, Vec<Vec<[f64; 2]>>)]) -> Option<Picked> {
        let point = [p.point.x as f64, p.point.y as f64, p.point.z as f64];
        match p.item()? {
            item @ Item::Edge { edge, .. } => {
                let (owner, curve, polyline) = self.edges.get(edge)?;
                Some(Picked {
                    item,
                    exact: Exact::Curve {
                        curve: curve.clone(),
                        polyline: polyline.clone(),
                    },
                    point,
                    owner: owner.clone(),
                    pad: None,
                })
            }
            item @ Item::Face { mesh, face } => {
                let info = self.meshes.get(mesh)?;
                Some(Picked {
                    item,
                    exact: Exact::Surface(info.faces.get(face as usize)?.clone()),
                    point,
                    owner: info.name.clone(),
                    pad: (mesh == PADS_MESH)
                        .then(|| pads.get(face as usize).map(|(n, _)| n.clone()))
                        .flatten(),
                })
            }
        }
    }
}

fn axis_name(d: V3) -> String {
    for (i, n) in ["X", "Y", "Z"].iter().enumerate() {
        if d[i].abs() > 1.0 - 1e-6 {
            return format!("{}{n}", if d[i] > 0.0 { "+" } else { "−" });
        }
    }
    format!("({:.2}, {:.2}, {:.2})", d[0], d[1], d[2])
}

/// One line on what a pick is: `cylinder Ø 0.600 mm`, `plane facing +Z at z 1.200 mm`, `line 2.540 mm`
pub fn describe(p: &Picked) -> String {
    let what = match &p.exact {
        Exact::Surface(SurfaceKind::Plane { origin, normal }) => {
            let at = (0..3)
                .find(|&i| normal[i].abs() > 1.0 - 1e-6)
                .map(|i| format!(" at {} {:.3} mm", ["x", "y", "z"][i], origin[i]))
                .unwrap_or_default();
            format!("plane facing {}{at}", axis_name(*normal))
        }
        Exact::Surface(SurfaceKind::Cylinder {
            radius, outward, ..
        }) => format!(
            "{} Ø {:.3} mm",
            if *outward { "cylinder" } else { "hole" },
            2.0 * radius
        ),
        Exact::Surface(SurfaceKind::Cone {
            radius, half_angle, ..
        }) => format!(
            "cone Ø {:.3} mm, {:.1}° half angle",
            2.0 * radius,
            half_angle.to_degrees()
        ),
        Exact::Surface(SurfaceKind::Sphere { radius, .. }) => {
            format!("sphere Ø {:.3} mm", 2.0 * radius)
        }
        Exact::Surface(SurfaceKind::Torus {
            major_radius,
            minor_radius,
            ..
        }) => format!("torus R {:.3} mm, r {:.3} mm", major_radius, minor_radius),
        Exact::Surface(SurfaceKind::Freeform) => "freeform surface".into(),
        Exact::Curve {
            curve: CurveKind::Line { start, end },
            ..
        } => format!("line {:.3} mm", erc_step::math::distance(*start, *end)),
        Exact::Curve {
            curve: CurveKind::Circle { radius, .. },
            ..
        } => format!("circle Ø {:.3} mm", 2.0 * radius),
        Exact::Curve { polyline, .. } => {
            let len: f64 = polyline
                .windows(2)
                .map(|w| erc_step::math::distance(w[0], w[1]))
                .sum();
            format!("curve {len:.3} mm long")
        }
    };
    match &p.pad {
        Some(n) => format!("pad {n}: {what}"),
        None => format!("{}: {what}", p.owner),
    }
}

/// A measurement between two picks
#[derive(Clone, Debug, PartialEq)]
pub struct ModelMeasure {
    /// mm, or radians for an angle
    pub value: f64,
    pub angle: bool,
    /// What was measured: `plane to plane`, `pitch, axis to axis`, `diameter`, ...
    pub what: &'static str,
    pub from: V3,
    pub to: V3,
}

impl ModelMeasure {
    /// `1.270 mm` or `12.50°`
    pub fn value_text(&self) -> String {
        if self.angle {
            format!("{:.2}°", self.value.to_degrees())
        } else {
            format!("{:.3} mm", self.value)
        }
    }

    /// As the note writes it: `M3 = 1.270 mm (pitch, axis to axis)`
    pub fn note(&self, n: usize) -> String {
        format!("M{n} = {} ({})", self.value_text(), self.what)
    }

    fn from(mm: m::Measurement, what: &'static str) -> ModelMeasure {
        ModelMeasure {
            value: mm.value,
            angle: false,
            what,
            from: mm.from,
            to: mm.to,
        }
    }
}

impl Exact {
    fn plane(&self) -> Option<Plane> {
        match self {
            Exact::Surface(s) => s.plane(),
            _ => None,
        }
    }

    /// The axis of a cylinder, cone, torus or circle
    fn axis(&self) -> Option<Axis> {
        match self {
            Exact::Surface(s) => s.axis(),
            Exact::Curve { curve, .. } => curve.axis(),
        }
    }
}

/// What lies between two picks, measured on their exact kinds (see the module's list)
pub fn measure(a: &Picked, b: &Picked) -> ModelMeasure {
    if a.item == b.item {
        match &a.exact {
            Exact::Surface(s @ SurfaceKind::Cylinder { .. }) => {
                if let Some(d) = m::cylinder_diameter(s, a.point) {
                    return ModelMeasure::from(d, "diameter");
                }
            }
            Exact::Curve { curve, polyline } => {
                if let Some(d) = m::circle_diameter(curve, a.point) {
                    return ModelMeasure::from(d, "diameter");
                }
                let edge = erc_step::Edge {
                    step_id: 0,
                    curve: curve.clone(),
                    polyline: polyline.clone(),
                    faces: Vec::new(),
                };
                if let Some(l) = m::edge_length(&edge) {
                    return ModelMeasure::from(l, "edge length");
                }
            }
            _ => {}
        }
    }
    if let (Some(pa), Some(pb)) = (a.exact.plane(), b.exact.plane()) {
        return match m::plane_to_plane(pa, pb, a.point) {
            Some(d) => ModelMeasure::from(d, "plane to plane"),
            None => {
                let g = m::angle_between_planes(pa, pb, a.point);
                ModelMeasure {
                    angle: true,
                    ..ModelMeasure::from(g, "angle between planes")
                }
            }
        };
    }
    if let (Some(xa), Some(xb)) = (a.exact.axis(), b.exact.axis()) {
        return ModelMeasure::from(m::axis_to_axis(xa, xb), "pitch, axis to axis");
    }
    // A plane and something else, either way round
    let (plane, other) = match (a.exact.plane(), b.exact.plane()) {
        (Some(p), None) => (Some(p), b),
        (None, Some(p)) => (Some(p), a),
        _ => (None, a),
    };
    if let Some(plane) = plane {
        if let Some(axis) = other.exact.axis()
            && dot(axis.direction, plane.normal).abs() < m::PARALLEL
        {
            return ModelMeasure::from(
                m::plane_to_point(plane, axis.project(other.point)),
                "plane to axis",
            );
        }
        if let Exact::Curve {
            curve: CurveKind::Line { start, end },
            ..
        } = &other.exact
            && (plane.signed_distance(*start) - plane.signed_distance(*end)).abs() < 1e-6
        {
            return ModelMeasure::from(m::plane_to_point(plane, other.point), "plane to edge");
        }
        return ModelMeasure::from(m::plane_to_point(plane, other.point), "plane to point");
    }
    ModelMeasure::from(m::point_to_point(a.point, b.point), "point to point")
}

/// The 3D model's state in the tab: never saved
#[derive(Default)]
pub(super) struct Model3d {
    /// The footprint the scene was made or is being made for
    pub(super) key: Option<String>,
    loading: Option<Receiver<ModelScene>>,
    pub(super) loaded: Option<ModelScene>,
    /// The front pads' copper of the footprint loaded (numbers and polygons), for naming a pad picked
    pads: Vec<(String, Vec<Vec<[f64; 2]>>)>,
    pub(super) view: Option<View3d>,
    /// Measure model's first pick, waiting for the second
    pub(super) first: Option<Picked>,
    /// The last measurement and its number in the note
    pub(super) last: Option<(usize, ModelMeasure)>,
}

impl Model3d {
    /// Reading the models
    pub(super) fn loading(&self) -> bool {
        self.loading.is_some()
    }
}

impl FootprintReview {
    /// The key of the footprint shown: its scene is made again when it changes
    fn model_key(&self) -> Option<String> {
        let s = self.rt.sourced.as_ref()?;
        Some(format!(
            "{}\n{}\n{:?}",
            s.footprint.footprint.lib_id, s.hash, s.source
        ))
    }

    /// Whether the footprint shown names a 3D model
    pub(super) fn has_model_entries(&self) -> bool {
        self.rt
            .sourced
            .as_ref()
            .is_some_and(|s| !s.footprint.footprint.models.is_empty())
    }

    /// Why Measure model and the 3D view have nothing to show, when they don't
    pub(super) fn model_blocker(&self) -> Option<&'static str> {
        if self.rt.sourced.is_none() {
            Some("no footprint shown: pick a symbol")
        } else if !self.has_model_entries() {
            Some("the footprint names no 3D model")
        } else {
            None
        }
    }

    /// The 3D key: the canvas between the page and the model; leaving the model leaves Measure model
    pub(super) fn toggle_3d(&mut self) {
        if self.view_3d {
            if self.tool == Tool::MeasureModel {
                self.set_tool(Tool::Move);
            }
            self.view_3d = false;
            self.usage("model view page");
        } else {
            self.view_3d = true;
            self.usage("model view 3d");
        }
    }

    /// Starts reading the footprint's models when the 3D view wants them and they are not read for it yet
    pub(super) fn model_wanted(&mut self, ctx: &egui::Context) {
        let key = self.model_key();
        if key == self.rt.model.key {
            return;
        }
        self.rt.model.key = key.clone();
        self.rt.model.loaded = None;
        self.rt.model.loading = None;
        self.rt.model.first = None;
        self.rt.model.last = None;
        if let Some(view) = &mut self.rt.model.view {
            view.set_scene(Scene::default());
            view.marks.clear();
        }
        let (Some(key), Some(sourced)) = (key, self.rt.sourced.clone()) else {
            return;
        };
        let pads = fpm::front_pad_copper(&sourced.footprint);
        self.rt.model.pads = pads.clone();
        let opts = fpm::ModelOptions {
            config: self.rt.config.clone(),
            ..fpm::ModelOptions::default()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = self.threads_wake().then(|| ctx.clone());
        std::thread::Builder::new()
            .name("fp review model".into())
            .spawn(move || {
                let vm = fpm::for_view(&sourced, &opts);
                let _ = tx.send(build(key, vm, &pads));
                if let Some(ctx) = wake {
                    ctx.request_repaint();
                }
            })
            .ok();
        self.rt.model.loading = Some(rx);
    }

    /// Takes the scene in when the worker is done (tests: waits for it)
    pub(super) fn model_poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.rt.model.loading else {
            return;
        };
        #[cfg(test)]
        let got = if self.rt.sync_pdf {
            rx.recv().ok()
        } else {
            rx.try_recv().ok()
        };
        #[cfg(not(test))]
        let got = rx.try_recv().ok();
        let Some(scene) = got else {
            if !self.threads_wake() {
                ctx.request_repaint();
            }
            return;
        };
        self.rt.model.loading = None;
        if Some(&scene.key) != self.rt.model.key.as_ref() {
            return;
        }
        self.usage("model loaded");
        self.rt.model.loaded = Some(scene);
        ctx.request_repaint();
    }

    /// The view, made on first use with the app's wgpu render state; the scene loaded handed to it, framed
    fn model_view(&mut self, ctx: &egui::Context) -> &mut View3d {
        let m = &mut self.rt.model;
        let view = m.view.get_or_insert_with(|| match render_state(ctx) {
            Some(rs) => View3d::new(&rs),
            None => View3d::without_gpu(),
        });
        if let Some(scene) = m.loaded.as_mut().and_then(|l| l.scene.take()) {
            view.set_scene(scene);
            view.view(ViewPreset::Iso);
        }
        view
    }

    /// A click on the model in Measure model: the first pick waits, the second measures and writes the note
    fn model_click(&mut self, pick: Option<Pick>) {
        let m = &self.rt.model;
        let picked = m
            .loaded
            .as_ref()
            .zip(pick)
            .and_then(|(l, p)| l.picked(&p, &m.pads));
        let Some(picked) = picked else {
            self.rt.model.first = None;
            return;
        };
        let Some(first) = self.rt.model.first.take() else {
            self.rt.model.first = Some(picked);
            return;
        };
        let mm = measure(&first, &picked);
        self.rt.measure_count += 1;
        let n = self.rt.measure_count;
        let text = mm.note(n);
        let note = self.rt.note.trim_end().to_string();
        self.rt.note = if note.is_empty() {
            format!("{text} ")
        } else {
            format!("{note} {text} ")
        };
        self.rt.model.last = Some((n, mm));
        self.usage("model measure");
    }

    /// The marks on the view: the first pick waiting, the last measurement
    fn model_marks(&self) -> Vec<Mark> {
        let mut marks = Vec::new();
        if let Some((n, mm)) = &self.rt.model.last {
            marks.push(Mark::segment(
                v(mm.from),
                v(mm.to),
                format!("M{n} = {}", mm.value_text()),
            ));
        }
        if let Some(p) = &self.rt.model.first {
            marks.push(Mark::point(v(p.point), format!("1: {}", describe(p))));
        }
        marks
    }

    /// The canvas showing the 3D model: the mode and note bars, the view, what is under the pointer, the keys
    pub(super) fn canvas_3d(&mut self, ui: &mut Ui) {
        self.mode_bar(ui);
        self.note_bar(ui);
        let ctx = ui.ctx().clone();
        self.model_wanted(&ctx);
        self.model_poll(&ctx);
        let marks = self.model_marks();
        let measuring = self.tool == Tool::MeasureModel;
        let first = self.rt.model.first.as_ref().map(|p| p.item);
        let has_scene = self.rt.model.loaded.as_ref().is_some_and(|l| l.shown > 0);
        let nav = (!self.rt.puck.settings.calibrating())
            .then(|| self.rt.puck.settings.mapping().apply(&self.rt.puck.axes));
        let (resp, hovered, clicked) = if has_scene {
            let view = self.model_view(&ctx);
            view.marks = marks;
            if let Some(nav) = nav {
                view.navigate(nav);
            }
            let r = view.show(ui);
            if measuring {
                // The selection shows the first pick, not what View3d picked on its own
                view.selection = first.into_iter().collect();
            }
            let clicked = r.response.clicked().then_some(r.clicked);
            (r.response, r.hovered, clicked)
        } else {
            let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 0.0, CANVAS_BACKGROUND);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.model_status(),
                FontId::proportional(15.0),
                Color32::LIGHT_GRAY,
            );
            (resp, None, None)
        };
        let canvas = resp.rect;
        self.rt.canvas = canvas;
        if measuring && let Some(pick) = clicked {
            self.model_click(pick);
        }
        let painter = ui.painter_at(canvas);
        // What is under the pointer, and the measurement, at the top
        let mut lines = Vec::new();
        if let Some(l) = &self.rt.model.loaded
            && let Some(p) = hovered.and_then(|h| l.picked(&h, &self.rt.model.pads))
        {
            lines.push(describe(&p));
        }
        if let Some(p) = &self.rt.model.first {
            lines.push(format!("first: {} · click the second", describe(p)));
        } else if let Some((n, mm)) = &self.rt.model.last {
            lines.push(mm.note(*n));
        }
        let mut y = canvas.min.y + 4.0;
        for text in lines {
            let galley = painter.layout_no_wrap(text, FontId::monospace(13.0), Color32::WHITE);
            let r =
                Rect::from_min_size(pos2(canvas.min.x + 4.0, y), galley.size() + vec2(8.0, 4.0));
            painter.rect_filled(r, 3.0, Color32::from_black_alpha(170));
            painter.galley(r.min + vec2(4.0, 2.0), galley, Color32::WHITE);
            y = r.max.y + 2.0;
        }
        // The keys, under the view
        let galley = painter.layout_no_wrap(
            self.help_line_3d(),
            FontId::proportional(12.0),
            Color32::WHITE,
        );
        let r = Rect::from_min_size(
            pos2(canvas.min.x + 4.0, canvas.max.y - galley.size().y - 8.0),
            galley.size() + vec2(8.0, 4.0),
        );
        painter.rect_filled(r, 3.0, Color32::from_black_alpha(170));
        painter.galley(r.min + vec2(4.0, 2.0), galley, Color32::WHITE);
        // Keys while the pointer is over the view: those of the modes, the view and the review
        if resp.hovered() && !ctx.egui_wants_keyboard_input() {
            let km = self.keys();
            let (p, shift) = Pressed::read(ui, &km);
            for (on, action) in [
                (p.escape, Action::EndRuler),
                (p.move_tool, Action::Mode(Tool::Move)),
                (p.measure_pdf, Action::Mode(Tool::MeasurePdf)),
                (p.measure_footprint, Action::Mode(Tool::MeasureFootprint)),
                (p.select, Action::Mode(Tool::Select)),
                (p.measure_model, Action::Mode(Tool::MeasureModel)),
                (p.view_3d, Action::View3d),
                (p.puck, Action::Puck),
                (p.accept, Action::Accept),
                (p.next, Action::NextToReview),
                (p.note, Action::Note),
            ] {
                if on {
                    self.act(&ctx, action, None, shift, true);
                }
            }
        }
    }

    /// What the 3D view says while it has no model to show
    fn model_status(&self) -> String {
        if let Some(why) = self.model_blocker() {
            return format!("No 3D model: {why}");
        }
        if self.rt.model.loading() {
            return "Reading the 3D model…".into();
        }
        match &self.rt.model.loaded {
            Some(l) if l.notes.is_empty() => "The 3D model has nothing to show".into(),
            Some(l) => format!("No 3D model shown:\n{}", l.notes.join("\n")),
            None => "Reading the 3D model…".into(),
        }
    }

    /// One line on the 3D view's mouse and keys
    fn help_line_3d(&self) -> String {
        let k = self.keys();
        let n = key_name;
        format!(
            "3D model · drag turns · right or middle drag pans · wheel zooms at the pointer · double-click centres · \
             right-click: views\n{} measure the model: click a face or an edge, then another · {} back to the page · \
             {} or Esc: Move · {} write the note",
            n(k.measure_model),
            n(k.view_3d),
            n(k.move_tool),
            n(k.note),
        )
    }

    /// The side panel's section on the model: the file, what it holds, how it sits, what failed
    pub(super) fn model_section(&mut self, ui: &mut Ui) {
        use ve_theme::props::*;
        let k = self.keys();
        let summary = match (&self.rt.model.loaded, self.model_blocker()) {
            (_, Some(_)) => "none".to_string(),
            (Some(l), None) => format!("{} of {}", l.shown, l.report.models.len()),
            (None, None) if self.view_3d => "reading".into(),
            (None, None) => format!("{} shows it", key_name(k.view_3d)),
        };
        SubPane::new("3D model")
            .id_salt("fp model")
            .summary(summary)
            .tooltip("The footprint's STEP model placed as KiCad places it, measured on its exact faces and edges")
            .show(ui, |ui| {
                let why = self.model_blocker();
                let mut toggle = false;
                let mut mode = false;
                ui.horizontal_wrapped(|ui| {
                    let r = ui.add_enabled(
                        why.is_none() || self.view_3d,
                        Button::selectable(self.view_3d, "3D view"),
                    );
                    let r = match why {
                        Some(w) => r.on_disabled_hover_text(w),
                        None => r.on_hover_text(format!(
                            "The canvas shows the 3D model in place of the page ({})",
                            key_name(k.view_3d)
                        )),
                    };
                    toggle = r.clicked();
                    let on = self.tool == Tool::MeasureModel;
                    let r = ui.add_enabled(
                        why.is_none() || on,
                        Button::selectable(on, Tool::MeasureModel.name()),
                    );
                    let r = match why {
                        Some(w) => r.on_disabled_hover_text(w),
                        None => r.on_hover_text(format!(
                            "{}; {} (again: back to Move)",
                            Tool::MeasureModel.what(),
                            key_name(k.measure_model)
                        )),
                    };
                    mode = r.clicked();
                });
                if let Some(l) = &self.rt.model.loaded {
                    for (i, r) in l.report.models.iter().enumerate() {
                        Facts::new(("model", i)).show(ui, |f| {
                            f.fact("File", &r.entry.path, "the footprint's (model ...) entry");
                            if let Some(a) = &r.analysis {
                                f.fact(
                                    "Holds",
                                    format!(
                                        "{} parts, {} of {} faces, {} triangles, in {}",
                                        a.parts, a.faces_meshed, a.faces, a.triangles, a.unit
                                    ),
                                    "what the reader made of the file",
                                );
                                if let Some(e) = &a.extents {
                                    f.fact(
                                        "Size",
                                        format!(
                                            "{:.3} × {:.3} × {:.3} mm",
                                            e.size[0], e.size[1], e.size[2]
                                        ),
                                        "x, y and z in the footprint's frame",
                                    );
                                    f.fact(
                                        "Height",
                                        format!("{:.3} mm, {:.3} mm below", e.height, e.depth),
                                        "above the board's surface, and below it (THT leads)",
                                    );
                                }
                            }
                        });
                    }
                    for line in l.notes.iter().chain(
                        l.report
                            .models
                            .iter()
                            .filter_map(|r| r.analysis.as_ref())
                            .flat_map(|a| a.warnings.iter().take(5)),
                    ) {
                        ui.colored_label(Color32::from_rgb(230, 140, 0), line);
                    }
                    for f in &l.report.findings {
                        ui.label(format!("{}: {}", f.check, f.message));
                    }
                } else if self.rt.model.loading() {
                    ui.label("reading…");
                }
                hint(
                    ui,
                    format!(
                        "{} swaps the page and the model; {} measures it into the review note",
                        key_name(k.view_3d),
                        key_name(k.measure_model)
                    ),
                );
                if toggle {
                    self.toggle_3d();
                }
                if mode {
                    self.tool_key(Tool::MeasureModel);
                }
            });
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::tests::harness;
    use super::*;
    use egui_kittest::Harness;

    /// A two-pin part in `dir`: pads at x ±1.27, and its STEP model, a body 4 × 2 × 1.1 mm standing 0.1 mm over the
    /// board on two leads under the pads
    pub(in super::super) fn two_pin(dir: &Path) -> PathBuf {
        use erc_step::write::{Cuboid, cuboids};
        let step = cuboids(
            "TWO_PIN",
            &[
                Cuboid::new([-2.0, -1.0, 0.1], [2.0, 1.0, 1.2]),
                Cuboid::new([-1.47, -0.4, 0.0], [-1.07, 0.4, 0.1]),
                Cuboid::new([1.07, -0.4, 0.0], [1.47, 0.4, 0.1]),
            ],
        );
        let model = dir.join("two_pin.step");
        std::fs::write(&model, step).unwrap();
        let fp = dir.join("Two_Pin.kicad_mod");
        std::fs::write(
            &fp,
            format!(
                r#"(footprint "Two_Pin" (version 20240108) (generator "pcbnew") (layer "F.Cu")
  (property "Reference" "REF**" (at 0 -3 0) (layer "F.SilkS"))
  (attr smd)
  (fp_line (start -2.5 -1.5) (end 2.5 -1.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
  (fp_line (start 2.5 -1.5) (end 2.5 1.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
  (fp_line (start 2.5 1.5) (end -2.5 1.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
  (fp_line (start -2.5 1.5) (end -2.5 -1.5) (stroke (width 0.05) (type solid)) (layer "F.CrtYd"))
  (pad "1" smd rect (at -1.27 0) (size 0.6 1) (layers "F.Cu" "F.Paste" "F.Mask"))
  (pad "2" smd rect (at 1.27 0) (size 0.6 1) (layers "F.Cu" "F.Paste" "F.Mask"))
  (model "{}" (offset (xyz 0 0 0)) (scale (xyz 1 1 1)) (rotate (xyz 0 0 0)))
)"#,
                model.display()
            ),
        )
        .unwrap();
        fp
    }

    /// Where `p` is on the screen, when the view shows it there as a face of `mesh` with the normal `normal`
    fn on_screen(
        h: &Harness<'static, FootprintReview>,
        p: Vec3,
        mesh: usize,
        normal: Vec3,
    ) -> Option<egui::Pos2> {
        let view = h.state().rt.model.view.as_ref()?;
        let rect = view.rect()?;
        let proj = view.projector(rect);
        let (s, _) = proj.project(p)?;
        let pick = view.picker().pick(view.scene(), &proj, s, 0.0)?;
        let f = pick.face?;
        (f.mesh == mesh && pick.edge.is_none() && f.normal.dot(normal) > 0.99).then_some(s)
    }

    fn click(h: &mut Harness<'static, FootprintReview>, at: egui::Pos2) {
        h.hover_at(at);
        h.run();
        h.drag_at(at);
        h.run();
        h.drop_at(at);
        h.run();
    }

    /// The 3D key shows the model over the board and its pads; Measure model's two clicks, on the body's top and on
    /// the board, put the body's height into the note; the 3D key again goes back to the page and to Move
    #[test]
    fn the_model_in_3d_measured_into_the_note() {
        let dir = std::env::temp_dir().join(format!("berc-fp-model3d-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let mut h = harness("fp model 3d");
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, &fp);
        h.run();
        assert!(h.state().model_blocker().is_none());
        let key = |h: &mut Harness<'static, FootprintReview>, k: egui::Key| {
            h.hover_at(h.state().rt.canvas.center());
            h.run();
            h.key_press(k);
            h.run();
            h.run();
        };
        key(&mut h, egui::Key::Num3);
        assert!(h.state().view_3d);
        h.run();
        let st = h.state();
        let l = st.rt.model.loaded.as_ref().expect("the model read");
        assert_eq!(l.shown, 1, "{:?}", l.notes);
        assert_eq!(l.meshes.len(), 5, "board, pads, body and two leads");
        let e = l.report.models[0]
            .analysis
            .as_ref()
            .unwrap()
            .extents
            .unwrap();
        assert!((e.height - 1.2).abs() < 1e-6, "{e:?}");
        assert_eq!(st.rt.model.view.as_ref().unwrap().scene().meshes.len(), 5);

        key(&mut h, egui::Key::O);
        assert_eq!(h.state().tool, Tool::MeasureModel);
        let top =
            on_screen(&h, Vec3::new(0.3, 0.2, 1.2), 2, Vec3::Z).expect("the body's top in view");
        // A point of the board's top no part hides
        let board = [
            (-2.6, -1.8),
            (2.6, -1.8),
            (-2.6, 1.8),
            (2.6, 1.8),
            (0.0, -1.8),
            (0.0, 1.8),
        ]
        .into_iter()
        .find_map(|(x, y)| on_screen(&h, Vec3::new(x, y, 0.0), 0, Vec3::Z))
        .expect("some of the board in view");
        click(&mut h, top);
        let first = h
            .state()
            .rt
            .model
            .first
            .clone()
            .expect("the first pick waits");
        assert!(
            describe(&first).contains("plane facing +Z at z 1.200 mm"),
            "{}",
            describe(&first)
        );
        click(&mut h, board);
        let st = h.state();
        assert!(st.rt.model.first.is_none());
        assert!(
            st.rt.note.contains("M1 = 1.200 mm (plane to plane)"),
            "{}",
            st.rt.note
        );
        assert_eq!(st.rt.model.view.as_ref().unwrap().marks.len(), 1);

        key(&mut h, egui::Key::Num3);
        assert!(!h.state().view_3d);
        assert_eq!(h.state().tool, Tool::Move);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// On the GPU: the 3D view draws the model where picking finds it, in the model's grey over the board's green.
    /// `BERC_MODEL3D_PNG` names a file to save the picture to.
    #[test]
    fn the_model_renders_on_the_gpu() {
        let dir = std::env::temp_dir().join(format!("berc-fp-model3d-gpu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let rs = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let settings = FootprintReviewSettings {
            font_dirs: Vec::new(),
            library_search: Vec::new(),
            library_name: Vec::new(),
            ..Default::default()
        };
        let mut cx = Context::new();
        let mut h = Harness::builder()
            .with_size(vec2(1200.0, 900.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(
                rs.clone(),
            ))
            .build_ui_state(
                move |ui, state: &mut FootprintReview| {
                    state.ui(ui, &mut cx, Id::new("fp model gpu"))
                },
                FootprintReview::with(settings, Arc::new(|_: &str| {})),
            );
        register(&h.ctx, &rs);
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, &fp);
        h.state_mut().toggle_3d();
        h.run();
        h.run();
        assert!(h.state().rt.model.view.as_ref().unwrap().has_gpu());
        let top =
            on_screen(&h, Vec3::new(0.3, 0.2, 1.2), 2, Vec3::Z).expect("the body's top in view");
        let board = [(-2.6, -1.8), (2.6, -1.8), (-2.6, 1.8), (2.6, 1.8)]
            .into_iter()
            .find_map(|(x, y)| on_screen(&h, Vec3::new(x, y, 0.0), 0, Vec3::Z))
            .expect("some of the board in view");
        let image = h.render().unwrap();
        if let Some(p) = std::env::var_os("BERC_MODEL3D_PNG") {
            image.save(p).unwrap();
        }
        let ppp = h.ctx.pixels_per_point();
        let at = |p: egui::Pos2| {
            let px = image.get_pixel((p.x * ppp) as u32, (p.y * ppp) as u32).0;
            [px[0] as i32, px[1] as i32, px[2] as i32]
        };
        let (t, b) = (at(top), at(board));
        // The body is grey, lit: the channels close together and not dark
        assert!(
            (t[0] - t[1]).abs() < 30 && (t[1] - t[2]).abs() < 30 && t[1] > 60,
            "{t:?}"
        );
        // The board is green
        assert!(b[1] > b[0] + 15 && b[1] > b[2] + 5, "{b:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A real STEP file through the whole build, its model at the footprint's origin: `BERC_MODEL3D_STEP` names
    /// it (skips without). Prints what it holds and the pitch of each two vertical cylinders, nearest first.
    #[test]
    fn a_real_model_builds() {
        let Some(step) = std::env::var_os("BERC_MODEL3D_STEP") else {
            eprintln!("BERC_MODEL3D_STEP not set: no real model read");
            return;
        };
        let dir = std::env::temp_dir().join(format!("berc-fp-model3d-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = dir.join("Real.kicad_mod");
        std::fs::write(
            &fp,
            format!(
                "(footprint \"Real\" (layer \"F.Cu\") (attr through_hole)\n  (model \"{}\"))\n",
                Path::new(&step).display()
            ),
        )
        .unwrap();
        let sourced = better_erc::fp::FootprintSource::File(fp).read().unwrap();
        let opts = fpm::ModelOptions {
            config: None,
            ..fpm::ModelOptions::default()
        };
        let l = build("real".into(), fpm::for_view(&sourced, &opts), &[]);
        assert_eq!(l.shown, 1, "{:?}", l.notes);
        let faces: Vec<&SurfaceKind> = l.meshes.iter().skip(2).flat_map(|m| &m.faces).collect();
        let count = |f: fn(&SurfaceKind) -> bool| faces.iter().filter(|k| f(k)).count();
        eprintln!(
            "{} parts, {} faces: {} planes, {} cylinders, {} freeform; {} edges; {} triangles",
            l.meshes.len() - 2,
            faces.len(),
            count(|k| matches!(k, SurfaceKind::Plane { .. })),
            count(|k| matches!(k, SurfaceKind::Cylinder { .. })),
            count(|k| matches!(k, SurfaceKind::Freeform)),
            l.edges.len(),
            l.scene.as_ref().unwrap().triangle_count()
        );
        let pins: Vec<(usize, &SurfaceKind)> = l
            .meshes
            .iter()
            .enumerate()
            .skip(2)
            .flat_map(|(i, m)| m.faces.iter().map(move |f| (i, f)))
            .filter(|(_, k)| {
                matches!(k, SurfaceKind::Cylinder { axis, outward: true, .. } if axis[2].abs() > 0.999)
            })
            .collect();
        let mut pitches = Vec::new();
        for (i, (ma, a)) in pins.iter().enumerate() {
            for (mb, b) in &pins[i + 1..] {
                let pa = Picked {
                    item: Item::Face { mesh: *ma, face: 0 },
                    exact: Exact::Surface((*a).clone()),
                    point: [0.0; 3],
                    owner: String::new(),
                    pad: None,
                };
                let pb = Picked {
                    item: Item::Face { mesh: *mb, face: 1 },
                    exact: Exact::Surface((*b).clone()),
                    ..pa.clone()
                };
                let mm = measure(&pa, &pb);
                if mm.value > 0.01 {
                    pitches.push(mm.note(1));
                }
            }
        }
        pitches.sort();
        pitches.dedup();
        eprintln!("pitches: {:?}", &pitches[..pitches.len().min(6)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_footprint_without_a_model_says_so() {
        let mut h = harness("fp model none");
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut()
            .open_path(&ctx, &super::super::tests::footprint());
        h.run();
        assert_eq!(
            h.state().model_blocker(),
            Some("the footprint names no 3D model")
        );
        h.state_mut().toggle_3d();
        h.run();
        assert!(
            h.state()
                .rt
                .model
                .loaded
                .as_ref()
                .is_some_and(|l| l.shown == 0)
        );
        assert!(h.state().model_status().contains("names no 3D model"));
    }

    fn picked(item: Item, exact: Exact, point: V3) -> Picked {
        Picked {
            item,
            exact,
            point,
            owner: "part".into(),
            pad: None,
        }
    }

    fn face(mesh: usize, face: u32) -> Item {
        Item::Face { mesh, face }
    }

    fn plane(z: f64, up: bool) -> Exact {
        Exact::Surface(SurfaceKind::Plane {
            origin: [3.0, 1.0, z],
            normal: [0.0, 0.0, if up { 1.0 } else { -1.0 }],
        })
    }

    fn pin(x: f64) -> Exact {
        Exact::Surface(SurfaceKind::Cylinder {
            axis_origin: [x, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            radius: 0.3,
            outward: true,
        })
    }

    #[test]
    fn measures_on_the_exact_kinds() {
        let top = picked(face(2, 0), plane(1.2, true), [0.0, 0.0, 1.2]);
        let board = picked(face(0, 5), plane(0.0, true), [5.0, 5.0, 0.0]);
        let mm = measure(&top, &board);
        assert_eq!(mm.what, "plane to plane");
        assert!((mm.value - 1.2).abs() < 1e-12);
        assert_eq!(mm.note(3), "M3 = 1.200 mm (plane to plane)");

        let a = picked(face(3, 1), pin(0.0), [0.3, 0.0, 0.5]);
        let b = picked(face(4, 1), pin(1.27), [1.57, 0.0, 0.1]);
        let mm = measure(&a, &b);
        assert_eq!(mm.note(3), "M3 = 1.270 mm (pitch, axis to axis)");
        let mm = measure(&a, &a);
        assert_eq!(mm.what, "diameter");
        assert!((mm.value - 0.6).abs() < 1e-12);

        let side = picked(
            face(2, 3),
            Exact::Surface(SurfaceKind::Plane {
                origin: [1.0, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            }),
            [1.0, 0.0, 0.5],
        );
        let mm = measure(&top, &side);
        assert!(mm.angle);
        assert_eq!(mm.value_text(), "90.00°");

        // A pin lying flat: its axis' height over the board
        let lying = picked(
            face(5, 0),
            Exact::Surface(SurfaceKind::Cylinder {
                axis_origin: [0.0, 0.0, 0.25],
                axis: [1.0, 0.0, 0.0],
                radius: 0.25,
                outward: true,
            }),
            [0.0, 0.0, 0.5],
        );
        let mm = measure(&board, &lying);
        assert_eq!(mm.what, "plane to axis");
        assert!((mm.value - 0.25).abs() < 1e-12);

        let line = Exact::Curve {
            curve: CurveKind::Line {
                start: [0.0, 0.0, 0.0],
                end: [2.54, 0.0, 0.0],
            },
            polyline: vec![[0.0, 0.0, 0.0], [2.54, 0.0, 0.0]],
        };
        let e = picked(Item::Edge { set: 0, edge: 7 }, line, [1.0, 0.0, 0.0]);
        let mm = measure(&e, &e);
        assert_eq!(mm.note(1), "M1 = 2.540 mm (edge length)");
        assert_eq!(measure(&top, &e).what, "plane to edge");
    }

    /// The exact kinds follow the entry's transform: turned 90° about z, moved, scaled by 2
    #[test]
    fn kinds_are_placed_as_the_triangles() {
        use better_erc::model::layout::{ModelEntry, Xyz};
        let e = ModelEntry {
            offset: Xyz::new(1.0, 2.0, 0.5),
            scale: Xyz::new(2.0, 2.0, 2.0),
            rotate: Xyz::new(0.0, 0.0, 90.0),
            ..ModelEntry::default()
        };
        let t = fpm::placement(&e);
        let s = uniform_scale(&e);
        assert_eq!(s, Some(2.0));
        let close = |a: V3, b: V3| (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9);
        let SurfaceKind::Plane { origin, normal } = place_surface(
            &t,
            s,
            &SurfaceKind::Plane {
                origin: [1.0, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            },
        ) else {
            panic!()
        };
        // KiCad turns by the negated angle: +x goes to -y
        assert!(close(normal, [0.0, -1.0, 0.0]), "{normal:?}");
        assert!(close(origin, t.apply_point([1.0, 0.0, 0.0])));
        let SurfaceKind::Cylinder { axis, radius, .. } = place_surface(
            &t,
            s,
            &SurfaceKind::Cylinder {
                axis_origin: [0.0; 3],
                axis: [0.0, 1.0, 0.0],
                radius: 0.3,
                outward: true,
            },
        ) else {
            panic!()
        };
        assert!(close(axis, [1.0, 0.0, 0.0]), "{axis:?}");
        assert!((radius - 0.6).abs() < 1e-12);
        // Scaled unevenly: a plane keeps a true normal, a cylinder is no longer one
        let uneven = ModelEntry {
            scale: Xyz::new(2.0, 1.0, 1.0),
            ..ModelEntry::default()
        };
        let t = fpm::placement(&uneven);
        assert_eq!(uniform_scale(&uneven), None);
        let SurfaceKind::Plane { normal, .. } = place_surface(
            &t,
            None,
            &SurfaceKind::Plane {
                origin: [0.0; 3],
                normal: [0.6, 0.8, 0.0],
            },
        ) else {
            panic!()
        };
        // x stretched by 2: the normal leans toward y (0.3, 0.8 normalised)
        let n = 0.3f64.hypot(0.8);
        assert!(close(normal, [0.3 / n, 0.8 / n, 0.0]), "{normal:?}");
        assert_eq!(
            place_surface(
                &t,
                None,
                &SurfaceKind::Sphere {
                    centre: [0.0; 3],
                    radius: 1.0,
                    outward: true
                }
            ),
            SurfaceKind::Freeform
        );
    }

    #[test]
    fn ear_clipping_covers_the_polygon() {
        // An L, clockwise: 3 squares' area
        let l = [
            [0.0, 0.0],
            [0.0, 2.0],
            [1.0, 2.0],
            [1.0, 1.0],
            [2.0, 1.0],
            [2.0, 0.0],
        ];
        let tris = triangulate(&l);
        assert_eq!(tris.len(), 4);
        let area: f64 = tris
            .iter()
            .map(|[a, b, c]| {
                let (a, b, c) = (l[*a], l[*b], l[*c]);
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
            })
            .sum();
        assert!((area - 3.0).abs() < 1e-12, "{area}");
    }
}
