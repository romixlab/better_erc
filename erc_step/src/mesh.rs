//! Shells to triangles: truck converts the geometry and tessellates, our code keeps the topology with its entity
//! ids and contains what goes wrong.
//!
//! - The topology (shell, faces, loops, edges, vertices) is walked here, so every face and edge keeps its STEP id,
//!   and a face whose surface or edge truck cannot convert is dropped with a warning instead of silently.
//! - Surfaces and curves are wrapped: a sphere's tolerance stays under half its radius (truck asserts otherwise),
//!   and the wrappers' `subs` panic once the deadline has passed.
//! - A shell is tessellated inside `catch_unwind`; when it panics, face by face, so one bad face costs only itself.
//! - The tolerance is in the file's unit.

use crate::entities::{Entities, entity, logical, refs};
use crate::kinds::{self, Frame};
use crate::math::{Vec3, normalize};
use crate::structure::{Placed, Structure};
use crate::{Edge, Face, Mesh, Part, Warning, WarningKind};
use ruststep::ast::{EntityInstance, Name};
use ruststep::tables::IntoOwned;
use ruststep::tables::PlaceHolder;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::result::Result;
use std::time::Instant;
use truck_geotrait::*;
use truck_meshalgo::tessellation::RobustMeshableShape;
use truck_stepio::r#in::{Table, step_geometry::*};
use truck_topology::compress::{
    CompressedEdge, CompressedEdgeIndex, CompressedFace, CompressedShell,
};

const BUDGET: &str = "erc_step: time budget exceeded";

/// truck's table of the entities it reads; an entity it knows but cannot read is a warning. Two fixes first:
/// the records of a complex entity are sorted by name, as the standard asks and truck relies on (some exporters
/// do not sort them), and a `DEGENERATE_TOROIDAL_SURFACE` becomes the `TOROIDAL_SURFACE` it is a kind of (truck
/// does not read it; the trimming edges pick the half that is used). And a cone's angle is turned into radians,
/// which truck takes it in whatever the file's angle unit (`angle_rad`, the size of that unit in radians).
pub fn table(instances: &[EntityInstance], angle_rad: f64, warnings: &mut Vec<Warning>) -> Table {
    let mut table = Table::default();
    for e in instances {
        let fixed = match e {
            EntityInstance::Complex { id, subsuper }
                if !subsuper.0.is_sorted_by(|a, b| a.name <= b.name) =>
            {
                let mut records = subsuper.0.clone();
                records.sort_by(|a, b| a.name.cmp(&b.name));
                Some(EntityInstance::Complex {
                    id: *id,
                    subsuper: ruststep::ast::SubSuperRecord(records),
                })
            }
            EntityInstance::Simple { id, record }
                if record.name == "DEGENERATE_TOROIDAL_SURFACE" =>
            {
                let p = crate::entities::params(record);
                Some(EntityInstance::Simple {
                    id: *id,
                    record: ruststep::ast::Record {
                        name: "TOROIDAL_SURFACE".into(),
                        parameter: ruststep::ast::Parameter::List(
                            p.iter().take(4).cloned().collect(),
                        ),
                    },
                })
            }
            EntityInstance::Simple { id, record }
                if record.name == "CONICAL_SURFACE" && angle_rad != 1.0 =>
            {
                let mut p = crate::entities::params(record).to_vec();
                if let Some(a) = p.get(3).and_then(crate::entities::real) {
                    p[3] = ruststep::ast::Parameter::Real(a * angle_rad);
                }
                Some(EntityInstance::Simple {
                    id: *id,
                    record: ruststep::ast::Record {
                        name: record.name.clone(),
                        parameter: ruststep::ast::Parameter::List(p),
                    },
                })
            }
            _ => None,
        };
        if let Err(err) = table.push_instance(fixed.as_ref().unwrap_or(e)) {
            let id = match e {
                EntityInstance::Simple { id, .. } | EntityInstance::Complex { id, .. } => *id,
            };
            warnings.push(Warning {
                entity: Some(id),
                kind: WarningKind::EntityNotRead,
                message: crate::short(&err.to_string()),
            });
        }
    }
    table
}

/// The text of a caught panic.
pub fn panic_text(e: &Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "panic".into())
}

/// A face of a shell in the shell's own coordinates and unit.
struct LocalFace {
    /// the `ADVANCED_FACE`
    id: u64,
    /// the entity the shell lists (an `ORIENTED_FACE` or the face itself)
    listed: u64,
    orientation: bool,
    mesh: Option<Mesh>,
    /// indices into `LocalShell::edges`
    edges: Vec<usize>,
}

struct LocalEdge {
    id: u64,
    polyline: Vec<Vec3>,
}

#[derive(Default)]
struct LocalShell {
    faces: Vec<LocalFace>,
    edges: Vec<LocalEdge>,
}

pub struct Mesher<'t, 'e, 'a> {
    table: &'t Table,
    ents: &'e Entities<'a>,
    tolerance_mm: f64,
    deadline: Instant,
    cache: HashMap<(u64, u64), Rc<LocalShell>>,
    warnings: Vec<Warning>,
}

/// A face converted by truck, with its edges as STEP ids.
struct Converted {
    id: u64,
    listed: u64,
    orientation: bool,
    surface: Surface,
    /// boundaries: (edge id, orientation along the boundary)
    boundaries: Vec<Vec<(u64, bool)>>,
}

impl<'t, 'e, 'a> Mesher<'t, 'e, 'a> {
    pub fn new(
        table: &'t Table,
        ents: &'e Entities<'a>,
        tolerance_mm: f64,
        deadline: Instant,
    ) -> Self {
        Mesher {
            table,
            ents,
            tolerance_mm,
            deadline,
            cache: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn take_warnings(&mut self) -> Vec<Warning> {
        std::mem::take(&mut self.warnings)
    }

    fn warn(&mut self, entity: u64, kind: WarningKind, message: impl Into<String>) {
        self.warnings.push(Warning {
            entity: Some(entity),
            kind,
            message: message.into(),
        });
    }

    /// A placed solid as a part in the model frame.
    pub fn part(&mut self, placed: &Placed, structure: &Structure) -> Part {
        let frame = Frame {
            transform: placed.transform,
            unit_mm: placed.unit_mm,
            angle_rad: placed.angle_rad,
        };
        let colours = &structure.colours;
        let solid_colour = colours.get(&placed.solid).copied();
        let mut part = Part {
            name: placed.name.clone(),
            step_id: placed.solid,
            transform: placed.transform,
            colour: solid_colour
                .or_else(|| {
                    placed
                        .shells
                        .iter()
                        .find_map(|(s, _)| colours.get(s).copied())
                })
                .or(placed.inherited_colour),
            faces: Vec::new(),
            edges: Vec::new(),
        };
        for &(shell_id, shell_orientation) in &placed.shells {
            let tol = self.tolerance_mm / placed.unit_mm;
            let key = (shell_id, tol.to_bits());
            let local = match self.cache.get(&key) {
                Some(l) => l.clone(),
                None => {
                    let l = Rc::new(self.shell(shell_id, tol));
                    self.cache.insert(key, l.clone());
                    l
                }
            };
            let shell_colour = colours
                .get(&shell_id)
                .copied()
                .or(solid_colour)
                .or(placed.inherited_colour);
            let face_base = part.faces.len();
            let mut edge_faces: Vec<Vec<usize>> = vec![Vec::new(); local.edges.len()];
            for (i, f) in local.faces.iter().enumerate() {
                for &e in &f.edges {
                    if !edge_faces[e].contains(&(face_base + i)) {
                        edge_faces[e].push(face_base + i);
                    }
                }
                let orientation = f.orientation == shell_orientation;
                part.faces.push(Face {
                    step_id: f.id,
                    surface: kinds::surface(self.ents, f.id, orientation, &frame),
                    colour: colours
                        .get(&f.id)
                        .or_else(|| colours.get(&f.listed))
                        .copied()
                        .or(shell_colour),
                    mesh: f
                        .mesh
                        .as_ref()
                        .map(|m| place_mesh(m, &frame, !shell_orientation)),
                });
            }
            for (e, faces) in local.edges.iter().zip(edge_faces) {
                part.edges.push(Edge {
                    step_id: e.id,
                    curve: kinds::curve(self.ents, e.id, &frame),
                    polyline: e.polyline.iter().map(|p| frame.point(*p)).collect(),
                    faces,
                });
            }
        }
        part
    }

    /// Converts and tessellates one shell in its own coordinates, `tol` in the file's unit.
    fn shell(&mut self, shell_id: u64, tol: f64) -> LocalShell {
        let ents = self.ents;
        let Some(p) = ents
            .simple(shell_id, "CLOSED_SHELL")
            .or_else(|| ents.simple(shell_id, "OPEN_SHELL"))
        else {
            self.warn(
                shell_id,
                WarningKind::Unsupported,
                "not a CLOSED_SHELL or OPEN_SHELL, not read",
            );
            return LocalShell::default();
        };
        let listed = p.get(1).map(refs).unwrap_or_default();
        let mut edge_curves: HashMap<u64, Option<Curve3D>> = HashMap::new();
        let mut converted = Vec::new();
        let mut straightened = Vec::new();
        for face in listed {
            match catch_unwind(AssertUnwindSafe(|| {
                self.convert_face(face, &mut edge_curves, &mut straightened)
            })) {
                Ok(Ok(c)) => converted.push(c),
                Ok(Err((id, msg))) => self.warn(id, WarningKind::FaceDropped, msg),
                Err(e) => self.warn(
                    face,
                    WarningKind::FaceDropped,
                    format!("panic in conversion: {}", panic_text(&e)),
                ),
            }
        }
        for (e, why) in straightened {
            self.warn(
                e,
                WarningKind::Approximated,
                format!("drawn as a straight line ({why})"),
            );
        }
        // the compressed shell: vertices and edges of the converted faces only
        let mut vertices = Vec::new();
        let mut vertex_index: HashMap<u64, usize> = HashMap::new();
        let mut edges = Vec::new();
        let mut edge_ids = Vec::new();
        let mut edge_index: HashMap<u64, usize> = HashMap::new();
        let mut faces = Vec::new();
        let mut face_edges = Vec::new();
        for c in &converted {
            let mut boundaries = Vec::new();
            let mut used = Vec::new();
            for b in &c.boundaries {
                let mut wire = Vec::new();
                for &(eid, orientation) in b {
                    let index = match edge_index.get(&eid) {
                        Some(i) => *i,
                        None => {
                            let (v0, v1) = edge_vertices(ents, eid).unwrap_or((0, 0));
                            let mut vi = |v: u64| {
                                *vertex_index.entry(v).or_insert_with(|| {
                                    vertices.push(
                                        vertex_point(ents, v).unwrap_or(Point3::new(0.0, 0.0, 0.0)),
                                    );
                                    vertices.len() - 1
                                })
                            };
                            let vertices = (vi(v0), vi(v1));
                            let curve = edge_curves[&eid]
                                .clone()
                                .expect("converted faces have converted edges");
                            edges.push(CompressedEdge {
                                vertices,
                                curve: Crv {
                                    c: curve,
                                    deadline: self.deadline,
                                },
                            });
                            edge_ids.push(eid);
                            edge_index.insert(eid, edges.len() - 1);
                            edges.len() - 1
                        }
                    };
                    used.push(index);
                    wire.push(CompressedEdgeIndex { index, orientation });
                }
                boundaries.push(wire);
            }
            used.sort_unstable();
            used.dedup();
            face_edges.push(used);
            faces.push(CompressedFace {
                boundaries,
                orientation: c.orientation,
                surface: Surf {
                    s: c.surface.clone(),
                    deadline: self.deadline,
                },
            });
        }
        let cs = CompressedShell {
            vertices,
            edges,
            faces,
        };
        let mut meshes: Vec<Option<Mesh>> = vec![None; cs.faces.len()];
        let mut polylines: Vec<Option<Vec<Vec3>>> = vec![None; cs.edges.len()];
        let mut failed: Vec<Option<String>> = vec![None; cs.faces.len()];
        if Instant::now() > self.deadline {
            failed.iter_mut().for_each(|f| *f = Some(BUDGET.into()));
        } else {
            match catch_unwind(AssertUnwindSafe(|| cs.robust_triangulation(tol))) {
                Ok(m) => {
                    for (i, f) in m.faces.into_iter().enumerate() {
                        meshes[i] = f.surface.map(|pm| local_mesh(&pm, f.orientation));
                    }
                    for (i, e) in m.edges.into_iter().enumerate() {
                        polylines[i] = Some(e.curve.0.iter().map(|p| [p.x, p.y, p.z]).collect());
                    }
                }
                Err(_) => {
                    // face by face, each with only its own edges
                    for (i, face) in cs.faces.iter().enumerate() {
                        if Instant::now() > self.deadline {
                            failed[i] = Some(BUDGET.into());
                            continue;
                        }
                        let own = &face_edges[i];
                        let local_index: HashMap<usize, usize> =
                            own.iter().enumerate().map(|(k, e)| (*e, k)).collect();
                        let single = CompressedShell {
                            vertices: cs.vertices.clone(),
                            edges: own.iter().map(|e| cs.edges[*e].clone()).collect(),
                            faces: vec![CompressedFace {
                                boundaries: face
                                    .boundaries
                                    .iter()
                                    .map(|w| {
                                        w.iter()
                                            .map(|x| CompressedEdgeIndex {
                                                index: local_index[&x.index],
                                                orientation: x.orientation,
                                            })
                                            .collect()
                                    })
                                    .collect(),
                                orientation: face.orientation,
                                surface: face.surface.clone(),
                            }],
                        };
                        match catch_unwind(AssertUnwindSafe(|| single.robust_triangulation(tol))) {
                            Ok(m) => {
                                let f = &m.faces[0];
                                meshes[i] =
                                    f.surface.as_ref().map(|pm| local_mesh(pm, f.orientation));
                                for (k, e) in m.edges.iter().enumerate() {
                                    polylines[own[k]].get_or_insert_with(|| {
                                        e.curve.0.iter().map(|p| [p.x, p.y, p.z]).collect()
                                    });
                                }
                            }
                            Err(e) => failed[i] = Some(panic_text(&e)),
                        }
                    }
                }
            }
        }
        // edges no meshed face gave a polyline: on their own
        for (i, pl) in polylines.iter_mut().enumerate() {
            if pl.is_none() && Instant::now() <= self.deadline {
                let c = &cs.edges[i].curve;
                *pl = catch_unwind(AssertUnwindSafe(|| {
                    let poly = PolylineCurve::from_curve(c, c.range_tuple(), tol);
                    poly.0
                        .iter()
                        .map(|p| [p.x, p.y, p.z])
                        .collect::<Vec<Vec3>>()
                }))
                .ok();
            }
        }
        // truck meshes the whole parameter range of a surface it fails to trim (a cylinder out to its axis
        // origin): a plane, cylinder or cone face never leaves the box of its own edges, nor any other face that box
        // ten times over
        let mut stray: Vec<Option<(f64, f64)>> = vec![None; converted.len()];
        for (i, c) in converted.iter().enumerate() {
            let ruled = ents
                .simple(c.id, "ADVANCED_FACE")
                .and_then(|p| p.get(2))
                .and_then(entity)
                .and_then(|g| ents.keyword(g))
                .is_some_and(|k| matches!(k, "PLANE" | "CYLINDRICAL_SURFACE" | "CONICAL_SURFACE"));
            let Some(mesh) = meshes[i].as_ref() else {
                continue;
            };
            let edge_points: Vec<&Vec3> = face_edges[i]
                .iter()
                .filter_map(|e| polylines[*e].as_ref())
                .flatten()
                .collect();
            if edge_points.is_empty() || face_edges[i].iter().any(|e| polylines[*e].is_none()) {
                continue;
            }
            let mut lo = [f64::MAX; 3];
            let mut hi = [f64::MIN; 3];
            for q in edge_points {
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
            let out = mesh
                .positions
                .iter()
                .flat_map(|q| (0..3).map(move |k| (lo[k] - q[k]).max(q[k] - hi[k])))
                .fold(0.0, f64::max);
            // Any other surface may bulge past its edges (a dome, a spline face bounded by a short seam), but not by
            // ten times their box's size: that far out is truck's parameter search gone wrong (a vertex 1000 units
            // off on a 0.3 unit extrusion of a spline)
            let diagonal = (0..3).map(|k| (hi[k] - lo[k]).powi(2)).sum::<f64>().sqrt();
            let limit = if ruled {
                10.0 * tol
            } else {
                10.0 * tol.max(diagonal)
            };
            if out > limit {
                meshes[i] = None;
                stray[i] = Some((out, limit));
            }
        }
        let mut out = LocalShell::default();
        for (i, c) in converted.iter().enumerate() {
            if let Some((out, limit)) = stray[i] {
                let msg = format!(
                    "not meshed: truck's trimming failed (triangles {out:.3} units outside the face's edges, \
                     {limit:.3} allowed)"
                );
                self.warn(c.id, WarningKind::FaceNotMeshed, msg);
            } else if meshes[i].is_none() {
                let why = match &failed[i] {
                    Some(t) if t == BUDGET => "not meshed: the time budget ran out".to_string(),
                    Some(t) => format!("not meshed: panic in tessellation: {}", crate::short(t)),
                    None => "not meshed: truck could not trim the face".to_string(),
                };
                self.warn(c.id, WarningKind::FaceNotMeshed, why);
            }
            out.faces.push(LocalFace {
                id: c.id,
                listed: c.listed,
                orientation: c.orientation,
                mesh: meshes[i].take(),
                edges: face_edges[i].clone(),
            });
        }
        for (i, id) in edge_ids.into_iter().enumerate() {
            out.edges.push(LocalEdge {
                id,
                polyline: polylines[i].take().unwrap_or_default(),
            });
        }
        out
    }

    /// One face of a shell through truck: its surface and its edges' curves. Err names the entity at fault.
    fn convert_face(
        &self,
        listed: u64,
        edge_curves: &mut HashMap<u64, Option<Curve3D>>,
        straightened: &mut Vec<(u64, String)>,
    ) -> Result<Converted, (u64, String)> {
        let ents = self.ents;
        let (id, orientation) = match ents.simple(listed, "ORIENTED_FACE") {
            // ORIENTED_FACE(name, *, face, orientation)
            Some(p) => (
                p.get(2)
                    .and_then(entity)
                    .ok_or((listed, "oriented face without its face".to_string()))?,
                p.get(3).and_then(logical).unwrap_or(true),
            ),
            None => (listed, true),
        };
        let holder = self.table.face_surface.get(&id).ok_or_else(|| {
            let what = ents.keyword(id).unwrap_or("a complex entity");
            (id, format!("face not read ({what})"))
        })?;
        let step_surface = holder
            .face_geometry
            .clone()
            .into_owned(self.table)
            .map_err(|e| {
                (
                    id,
                    format!("surface not read: {}", crate::short(&e.to_string())),
                )
            })?;
        let mut surface = Surface::try_from(&step_surface).map_err(|e| {
            (
                id,
                format!("surface not converted: {}", crate::short(&e.to_string())),
            )
        })?;
        if !holder.same_sense {
            surface.invert();
        }
        let face = ents
            .simple(id, "ADVANCED_FACE")
            .or_else(|| ents.simple(id, "FACE_SURFACE"));
        let bounds = face.and_then(|p| p.get(1)).map(refs).unwrap_or_default();
        let mut boundaries = Vec::new();
        for b in bounds {
            let Some(bp) = ents
                .simple(b, "FACE_OUTER_BOUND")
                .or_else(|| ents.simple(b, "FACE_BOUND"))
            else {
                continue;
            };
            let bound_orientation = bp.get(2).and_then(logical).unwrap_or(true);
            let Some(lp) = bp
                .get(1)
                .and_then(entity)
                .and_then(|l| ents.simple(l, "EDGE_LOOP"))
            else {
                // a VERTEX_LOOP (the apex of a cone) has no edges to trim by
                continue;
            };
            let mut wire = Vec::new();
            for oe in lp.get(1).map(refs).unwrap_or_default() {
                let (edge, o) = match ents.simple(oe, "ORIENTED_EDGE") {
                    Some(q) => (
                        q.get(3)
                            .and_then(entity)
                            .ok_or((oe, "oriented edge without its edge".to_string()))?,
                        q.get(4).and_then(logical).unwrap_or(true),
                    ),
                    None => (oe, true),
                };
                if let std::collections::hash_map::Entry::Vacant(slot) = edge_curves.entry(edge) {
                    let c = self.convert_edge(edge, straightened);
                    slot.insert(c.as_ref().ok().cloned());
                    if let Err(msg) = c {
                        return Err((id, format!("edge #{edge}: {msg}")));
                    }
                } else if edge_curves[&edge].is_none() {
                    return Err((id, format!("edge #{edge} not converted")));
                }
                wire.push((edge, o == bound_orientation));
            }
            if !bound_orientation {
                wire.reverse();
            }
            boundaries.push(wire);
        }
        Ok(Converted {
            id,
            listed,
            orientation,
            surface,
            boundaries,
        })
    }

    fn convert_edge(
        &self,
        edge: u64,
        straightened: &mut Vec<(u64, String)>,
    ) -> Result<Curve3D, String> {
        let ents = self.ents;
        let geometry = ents
            .simple(edge, "EDGE_CURVE")
            .and_then(|p| p.get(3))
            .and_then(entity);
        if let Some(g) = geometry
            && ents.keyword(kinds::curve_3d(ents, g)) == Some("HYPERBOLA")
        {
            // truck's hyperbola conversion can spin forever in its quartic solver: drawn straight
            let (v0, v1) = edge_vertices(ents, edge).ok_or("edge without vertices")?;
            let (p, q) = (
                vertex_point(ents, v0).ok_or("vertex")?,
                vertex_point(ents, v1).ok_or("vertex")?,
            );
            straightened.push((edge, "a hyperbola".into()));
            return Ok(Curve3D::Line(Line(p, q)));
        }
        let holder = self.table.edge_curve.get(&edge).ok_or("edge not read")?;
        let first = holder
            .clone()
            .into_owned(self.table)
            .map_err(|e| crate::short(&e.to_string()))
            .and_then(|owned| {
                owned
                    .parse_curve3d()
                    .map_err(|e| crate::short(&e.to_string()))
            });
        let Err(why) = first else { return first };
        // a surface curve whose pcurve truck cannot place: its 3D curve alone
        if let Some(g) = geometry {
            let c3d = kinds::curve_3d(ents, g);
            if c3d != g {
                let mut plain = holder.clone();
                plain.edge_geometry = PlaceHolder::Ref(Name::Entity(c3d));
                if let Ok(c) = plain
                    .into_owned(self.table)
                    .map_err(|e| e.to_string())
                    .and_then(|o| o.parse_curve3d().map_err(|e| e.to_string()))
                {
                    return Ok(c);
                }
            }
        }
        // else a straight edge between its vertices, so the face stays
        let (v0, v1) = edge_vertices(ents, edge).ok_or(why.clone())?;
        let (Some(p), Some(q)) = (vertex_point(ents, v0), vertex_point(ents, v1)) else {
            return Err(why);
        };
        if v0 == v1 {
            return Err(why);
        }
        straightened.push((edge, why));
        Ok(Curve3D::Line(Line(p, q)))
    }
}

fn edge_vertices(ents: &Entities<'_>, edge: u64) -> Option<(u64, u64)> {
    let p = ents.simple(edge, "EDGE_CURVE")?;
    Some((p.get(1).and_then(entity)?, p.get(2).and_then(entity)?))
}

fn vertex_point(ents: &Entities<'_>, v: u64) -> Option<Point3> {
    let pt = ents.simple(v, "VERTEX_POINT")?.get(1).and_then(entity)?;
    let p = ents.point(pt)?;
    Some(Point3::new(p[0], p[1], p[2]))
}

/// truck's polygon mesh as our mesh, still local; turned over when the face's orientation is false.
fn local_mesh(pm: &PolygonMesh, orientation: bool) -> Mesh {
    let mut m = Mesh::default();
    let mut index: HashMap<(usize, Option<usize>), u32> = HashMap::new();
    let positions = pm.positions();
    let normals = pm.normals();
    for tri in pm.faces().triangle_iter() {
        let p = |k: usize| {
            let v = positions[tri[k].pos];
            [v.x, v.y, v.z]
        };
        let flat = normalize(crate::math::cross(
            crate::math::sub(p(1), p(0)),
            crate::math::sub(p(2), p(0)),
        ))
        .unwrap_or([0.0, 0.0, 1.0]);
        let mut ids = [0u32; 3];
        for (k, id) in ids.iter_mut().enumerate() {
            let key = (tri[k].pos, tri[k].nor);
            *id = *index.entry(key).or_insert_with(|| {
                m.positions.push(p(k));
                let n = tri[k]
                    .nor
                    .and_then(|i| normals.get(i))
                    .and_then(|n| normalize([n.x, n.y, n.z]))
                    .unwrap_or(flat);
                m.normals.push(n);
                (m.positions.len() - 1) as u32
            });
        }
        m.triangles.push(ids);
    }
    if !orientation {
        turn_over(&mut m);
    }
    m
}

fn turn_over(m: &mut Mesh) {
    for t in &mut m.triangles {
        t.swap(1, 2);
    }
    for n in &mut m.normals {
        *n = crate::math::scale(*n, -1.0);
    }
}

fn place_mesh(m: &Mesh, frame: &Frame, turn: bool) -> Mesh {
    let mut out = Mesh {
        positions: m.positions.iter().map(|p| frame.point(*p)).collect(),
        normals: m.normals.iter().map(|n| frame.direction(*n)).collect(),
        triangles: m.triangles.clone(),
    };
    if turn {
        turn_over(&mut out);
    }
    out
}

/// A surface for the tessellation: stops at the deadline, keeps a sphere's tolerance under half its radius.
#[derive(Clone, Debug)]
struct Surf {
    s: Surface,
    deadline: Instant,
}

impl Surf {
    fn check(&self) {
        if Instant::now() > self.deadline {
            panic!("{BUDGET}");
        }
    }
}

impl ParametricSurface for Surf {
    type Point = Point3;
    type Vector = Vector3;
    fn subs(&self, u: f64, v: f64) -> Point3 {
        self.check();
        self.s.subs(u, v)
    }
    fn uder(&self, u: f64, v: f64) -> Vector3 {
        self.s.uder(u, v)
    }
    fn vder(&self, u: f64, v: f64) -> Vector3 {
        self.s.vder(u, v)
    }
    fn uuder(&self, u: f64, v: f64) -> Vector3 {
        self.s.uuder(u, v)
    }
    fn uvder(&self, u: f64, v: f64) -> Vector3 {
        self.s.uvder(u, v)
    }
    fn vvder(&self, u: f64, v: f64) -> Vector3 {
        self.s.vvder(u, v)
    }
    fn der_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
        self.s.der_mn(m, n, u, v)
    }
    fn parameter_range(&self) -> (ParameterRange, ParameterRange) {
        self.s.parameter_range()
    }
    fn u_period(&self) -> Option<f64> {
        self.s.u_period()
    }
    fn v_period(&self) -> Option<f64> {
        self.s.v_period()
    }
}

impl ParametricSurface3D for Surf {
    fn normal(&self, u: f64, v: f64) -> Vector3 {
        self.s.normal(u, v)
    }
}

impl SearchParameter<D2> for Surf {
    type Point = Point3;
    fn search_parameter<H: Into<SPHint2D>>(
        &self,
        p: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.check();
        self.s.search_parameter(p, hint, trials)
    }
}

impl SearchNearestParameter<D2> for Surf {
    type Point = Point3;
    /// truck's search, and when it finds nothing (its Newton steps can run off a torus or a B-spline from a poor
    /// start), again from the nearest point of a 16 by 16 grid over a bounded surface.
    fn search_nearest_parameter<H: Into<SPHint2D>>(
        &self,
        p: Point3,
        hint: H,
        trials: usize,
    ) -> Option<(f64, f64)> {
        self.check();
        self.s
            .search_nearest_parameter(p, hint, trials)
            .or_else(|| {
                let (Some(u), Some(v)) = self.s.try_range_tuple() else {
                    return None;
                };
                let start = algo::surface::presearch(&self.s, p, (u, v), 16);
                self.s
                    .search_nearest_parameter(p, start, trials)
                    .or_else(|| Some(grid_nearest(&self.s, p, (u, v))))
            })
    }
}

/// The parameter of the surface point nearest to `p` by a grid search refined around the best point, four levels
/// of 16 by 16: never fails, accurate to the range / 16^4 (for when Newton's method does not converge).
fn grid_nearest(s: &Surface, p: Point3, (mut ur, mut vr): ((f64, f64), (f64, f64))) -> (f64, f64) {
    const N: usize = 16;
    let mut best = ((ur.0 + ur.1) / 2.0, (vr.0 + vr.1) / 2.0);
    for _ in 0..4 {
        let (du, dv) = ((ur.1 - ur.0) / N as f64, (vr.1 - vr.0) / N as f64);
        let mut min = f64::INFINITY;
        for i in 0..=N {
            for j in 0..=N {
                let (u, v) = (ur.0 + du * i as f64, vr.0 + dv * j as f64);
                let d = s.subs(u, v).distance2(p);
                if d < min {
                    min = d;
                    best = (u, v);
                }
            }
        }
        ur = ((best.0 - du).max(ur.0), (best.0 + du).min(ur.1));
        vr = ((best.1 - dv).max(vr.0), (best.1 + dv).min(vr.1));
    }
    best
}

impl ParameterDivision2D for Surf {
    fn parameter_division(
        &self,
        range: ((f64, f64), (f64, f64)),
        tol: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        if let Surface::ElementarySurface(e) = &self.s
            && let ElementarySurface::Sphere(sphere) = e
        {
            return self
                .s
                .parameter_division(range, tol.min(sphere.entity().0.radius() * 0.5));
        }
        self.s.parameter_division(range, tol)
    }
}

/// An edge curve for the tessellation: stops at the deadline. Its division is truck-geotrait's, capped by our
/// patch (depth 16, stops on NaN).
#[derive(Clone, Debug)]
struct Crv {
    c: Curve3D,
    deadline: Instant,
}

impl ParametricCurve for Crv {
    type Point = Point3;
    type Vector = Vector3;
    fn subs(&self, t: f64) -> Point3 {
        if Instant::now() > self.deadline {
            panic!("{BUDGET}");
        }
        self.c.subs(t)
    }
    fn der(&self, t: f64) -> Vector3 {
        self.c.der(t)
    }
    fn der2(&self, t: f64) -> Vector3 {
        self.c.der2(t)
    }
    fn der_n(&self, n: usize, t: f64) -> Vector3 {
        self.c.der_n(n, t)
    }
    fn parameter_range(&self) -> ParameterRange {
        self.c.parameter_range()
    }
    fn period(&self) -> Option<f64> {
        self.c.period()
    }
}

impl BoundedCurve for Crv {}

impl ParameterDivision1D for Crv {
    type Point = Point3;
    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Point3>) {
        if Instant::now() > self.deadline {
            panic!("{BUDGET}");
        }
        self.c.parameter_division(range, tol)
    }
}
