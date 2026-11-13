//! A footprint's 3D models against the footprint, the library side of `berc fp model`: each `(model ...)` entry's
//! file found ([`erc_kicad::model3d`]), read ([`erc_step`]) and placed as KiCad places it, then measured: its extents
//! and height above the board, its centre against the pads', how far it sticks out of the courtyard, and whether
//! its leads land on the pads.
//!
//! **The frame** is KiCad's 3D frame of the footprint: x right, y up, z up out of the board, the origin at the
//! footprint's origin on the board's surface. It is the frame the model's `offset` is written in. A footprint's own
//! 2D frame has y down, so a pad at `(x, y)` there is at `(x, -y)` here.
//!
//! **KiCad's transform** from the model's frame into the footprint's ([`placement`]): the `scale`, then the
//! rotation about x, y and z, each by the negated `rotate` angle, then the `offset` in mm. On a board
//! ([`board_placement`]) the footprint frame is then flipped about the x axis for a footprint on the back, turned by
//! the footprint's rotation about z, and moved to its position (`(x, -y)` of the board's coordinates) on the
//! board's top or bottom surface. `kicad-cli pcb export step` pins this down in the tests.

use super::outline::{self, Item, Kind, P};
use crate::Severity;
use erc_kicad::model3d::{self, ModelPath};
use erc_model::LocalFootprint;
use erc_model::layout::{Footprint, ModelEntry, Pad, Side};
use erc_step::{Transform, Vec3};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// How thick a slice of the model's bottom counts as its leads, mm
pub const SLICE_MM: f64 = 0.2;

/// The rotation about one axis by `deg` degrees, exact at multiples of 90°
fn sin_cos(deg: f64) -> (f64, f64) {
    let d = deg.rem_euclid(360.0);
    if d.fract() == 0.0 && (d as i64) % 90 == 0 {
        return match d as i64 {
            0 => (0.0, 1.0),
            90 => (1.0, 0.0),
            180 => (0.0, -1.0),
            _ => (-1.0, 0.0),
        };
    }
    d.to_radians().sin_cos()
}

fn linear(rot: [[f64; 3]; 3]) -> Transform {
    Transform {
        rot,
        translation: [0.0; 3],
    }
}

fn rot_x(deg: f64) -> Transform {
    let (s, c) = sin_cos(deg);
    linear([[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]])
}

fn rot_y(deg: f64) -> Transform {
    let (s, c) = sin_cos(deg);
    linear([[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]])
}

fn rot_z(deg: f64) -> Transform {
    let (s, c) = sin_cos(deg);
    linear([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]])
}

/// KiCad's transform of a model entry: the model's own frame (mm) into the footprint's 3D frame. Not rigid when
/// the scale is other than 1.
pub fn placement(e: &ModelEntry) -> Transform {
    let scale = linear([
        [e.scale.x, 0.0, 0.0],
        [0.0, e.scale.y, 0.0],
        [0.0, 0.0, e.scale.z],
    ]);
    let r = rot_z(-e.rotate.z)
        .compose(&rot_y(-e.rotate.y))
        .compose(&rot_x(-e.rotate.x))
        .compose(&scale);
    Transform {
        translation: [e.offset.x, e.offset.y, e.offset.z],
        ..r
    }
}

/// The footprint's 3D frame into the board's 3D frame of `kicad-cli pcb export step` with the board's origin as
/// the output's (`--user-origin 0x0mm`): x right, y up (the board's y negated), z up. `surface_z` is the board
/// surface the footprint sits on: the top for a footprint on the front, the bottom for one on the back.
pub fn board_placement(f: &Footprint, surface_z: f64) -> Transform {
    let flip = match f.side {
        Side::Front => Transform::IDENTITY,
        Side::Back => rot_x(180.0),
    };
    let r = rot_z(f.rotation).compose(&flip);
    Transform {
        translation: [f.at.x as f64 / 1e6, -(f.at.y as f64) / 1e6, surface_z],
        ..r
    }
}

/// Settings of an analysis
#[derive(Clone, Debug)]
pub struct ModelOptions {
    pub read: erc_step::Options,
    /// KiCad's configuration directory for `kicad_common.json` ([`super::kicad_config_dir`]); `None`: the process
    /// environment, `KIPRJMOD` and the stock directories only
    pub config: Option<PathBuf>,
    /// The bottom slice taken as the leads, mm ([`SLICE_MM`])
    pub slice_mm: f64,
}

impl Default for ModelOptions {
    fn default() -> Self {
        ModelOptions {
            read: erc_step::Options {
                // A tenth of what a land pattern is drawn to is plenty for extents and the leads' footprint
                tolerance_mm: 0.02,
                ..erc_step::Options::default()
            },
            config: super::kicad_config_dir(),
            slice_mm: SLICE_MM,
        }
    }
}

/// Where a model entry's path led
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Resolved {
    /// The file, and the `${VAR}`s substituted to get there
    Found { path: PathBuf, vars: Vec<String> },
    /// A `${VAR}` nothing sets
    UnknownVar { var: String },
    /// No file at the expanded path (nor a `.step` or `.stp` beside a `.wrl`)
    NotFound { path: PathBuf },
}

impl From<ModelPath> for Resolved {
    fn from(p: ModelPath) -> Resolved {
        match p {
            ModelPath::Found { path, vars } => Resolved::Found { path, vars },
            ModelPath::UnknownVar(var) => Resolved::UnknownVar { var },
            ModelPath::NotFound(path) => Resolved::NotFound { path },
        }
    }
}

/// A box in the footprint's 3D frame, mm
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Extents {
    pub min: Vec3,
    pub max: Vec3,
    pub size: Vec3,
    /// How far the model rises above the board surface (its top, 0 when it is all below)
    pub height: f64,
    /// How far it reaches below the board surface (THT leads), 0 when it does not
    pub depth: f64,
    /// The middle of the box in x and y
    pub centre: [f64; 2],
}

/// The model against the courtyard
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CourtyardFit {
    /// How far the model reaches out of the courtyard, mm; 0 when it is all inside
    pub overhang: f64,
    /// The model's point furthest out, in x and y, when it reaches out
    pub worst: Option<[f64; 2]>,
}

/// The model's bottom slice (its leads) against the pads' copper
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Leads {
    /// The slice, z from and to (mm in the footprint frame)
    pub slice: [f64; 2],
    /// The slice seen from below, mm²
    pub area: f64,
    /// Of that, not over any pad, mm²
    pub off_pads: f64,
    /// Each copper pad on the front, in the footprint's order
    pub pads: Vec<PadLeads>,
    /// The size of the grid the areas are counted on, mm
    pub resolution: f64,
}

/// One pad's copper and how much of it the leads cover
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PadLeads {
    pub number: String,
    /// The pad's copper, mm²
    pub area: f64,
    /// Of it, under the slice, mm²
    pub covered: f64,
}

impl PadLeads {
    /// The share of the pad under a lead, 0 to 1
    pub fn fraction(&self) -> f64 {
        if self.area > 0.0 {
            self.covered / self.area
        } else {
            0.0
        }
    }
}

/// What a model file holds, placed on its footprint
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Analysis {
    /// The file's length unit and its size in mm
    pub unit: String,
    pub unit_mm: f64,
    pub parts: usize,
    pub faces: usize,
    /// Faces with triangles; the others are in `warnings`
    pub faces_meshed: usize,
    pub triangles: usize,
    /// What the reader could not read as it should, each with its STEP entity
    pub warnings: Vec<String>,
    /// `None` when nothing was meshed
    pub extents: Option<Extents>,
    /// `None` without a courtyard or without a mesh
    pub courtyard: Option<CourtyardFit>,
    /// `None` without a mesh
    pub leads: Option<Leads>,
}

/// One `(model ...)` entry of a footprint
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelReport {
    pub entry: ModelEntry,
    pub resolved: Resolved,
    /// The analysis, when the file was found and read
    pub analysis: Option<Analysis>,
    /// Why a file found could not be read
    pub read_error: Option<String>,
}

/// A finding on a footprint's models, by its catalogue id (`docs/checks.md`)
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelFinding {
    pub check: &'static str,
    pub severity: Severity,
    /// The model entry it is about (index into [`FootprintModels::models`]); `None` for the footprint
    pub model: Option<usize>,
    pub message: String,
}

/// A footprint and its models
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FootprintModels {
    /// The footprint's file, or the board and its reference
    pub source: String,
    pub reference: Option<String>,
    pub lib_id: String,
    /// The middle of the pads' copper, x and y in the footprint's 3D frame; `None` without copper pads
    pub pads_centre: Option<[f64; 2]>,
    pub models: Vec<ModelReport>,
    pub findings: Vec<ModelFinding>,
}

/// Model files read once per run: footprints of a library or a board share them
#[derive(Default)]
pub struct ModelCache {
    read: HashMap<PathBuf, Arc<Result<erc_step::Model, String>>>,
}

impl ModelCache {
    fn get(
        &mut self,
        path: &Path,
        opts: &erc_step::Options,
    ) -> Arc<Result<erc_step::Model, String>> {
        self.read
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(erc_step::read_file(path, opts).map_err(|e| e.to_string())))
            .clone()
    }
}

/// A point of the footprint's 2D frame (y down) in its 3D frame (y up)
fn up(p: P) -> [f64; 2] {
    [p[0], -p[1]]
}

/// Whether `pad` puts copper on the front: an SMD or plated pad on `F.Cu` or every copper layer, not a hole without
/// copper, not one of the package's vias
fn front_copper(pad: &Pad, pads: &[Pad]) -> bool {
    pad.kind != "np_thru_hole"
        && !outline::is_via(pad, pads)
        && pad
            .layers
            .iter()
            .any(|l| l == "F.Cu" || l == "*.Cu" || l == "F&B.Cu")
}

/// Analyses every model entry of `f`, its paths resolved against `project_dir` with `var` answering each `${VAR}`
pub fn analyze(
    source: String,
    reference: Option<String>,
    f: &LocalFootprint,
    project_dir: &Path,
    var: &dyn Fn(&str) -> Option<String>,
    cache: &mut ModelCache,
    opts: &ModelOptions,
) -> FootprintModels {
    let pads: Vec<(&Pad, Vec<Vec<[f64; 2]>>)> = f
        .pads
        .iter()
        .filter(|p| front_copper(p, &f.pads))
        .map(|p| {
            let polys = outline::pad_copper(p)
                .into_iter()
                .map(|poly| poly.into_iter().map(up).collect())
                .collect();
            (p, polys)
        })
        .collect();
    let pads_centre = bbox2(pads.iter().flat_map(|(_, polys)| polys.iter().flatten()))
        .map(|(lo, hi)| [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]);
    let courtyard = courtyard_segments(f);
    let mut models = Vec::new();
    for entry in &f.footprint.models {
        let resolved: Resolved = model3d::resolve(&entry.path, project_dir, var).into();
        let (analysis, read_error) = match &resolved {
            Resolved::Found { path, .. } => match &*cache.get(path, &opts.read) {
                Ok(m) => (Some(analyse_one(m, entry, &pads, &courtyard, opts)), None),
                Err(e) => (None, Some(e.clone())),
            },
            _ => (None, None),
        };
        models.push(ModelReport {
            entry: entry.clone(),
            resolved,
            analysis,
            read_error,
        });
    }
    // A logo or a mounting hole needs no model: only a footprint with copper pads that is not board-only
    let wants_model = !pads.is_empty() && !f.footprint.attrs.board_only;
    let findings = findings(&models, wants_model);
    FootprintModels {
        source,
        reference,
        lib_id: f.footprint.lib_id.clone(),
        pads_centre,
        models,
        findings,
    }
}

fn bbox2<'a>(points: impl Iterator<Item = &'a [f64; 2]>) -> Option<([f64; 2], [f64; 2])> {
    let mut b: Option<([f64; 2], [f64; 2])> = None;
    for p in points {
        let (lo, hi) = b.get_or_insert((*p, *p));
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    b
}

/// The courtyard's outline as segments in the footprint's 3D frame: every courtyard shape, each closed one closed
fn courtyard_segments(f: &LocalFootprint) -> Vec<[[f64; 2]; 2]> {
    let mut out = Vec::new();
    for item in outline::items(f, outline::Margins::default()) {
        let (points, closed) = match item {
            Item::Area {
                kind: Kind::Courtyard,
                points,
                ..
            } => (points, true),
            Item::Line {
                kind: Kind::Courtyard,
                points,
                closed,
                ..
            } => (points, closed),
            _ => continue,
        };
        let pts: Vec<[f64; 2]> = points.into_iter().map(up).collect();
        for w in pts.windows(2) {
            out.push([w[0], w[1]]);
        }
        if closed && pts.len() > 2 {
            out.push([pts[pts.len() - 1], pts[0]]);
        }
    }
    out
}

/// The triangles of `m` placed by `entry`, in the footprint's 3D frame
fn placed_triangles(m: &erc_step::Model, entry: &ModelEntry) -> Vec<[Vec3; 3]> {
    let t = placement(entry);
    let mut out = Vec::new();
    for mesh in m
        .parts
        .iter()
        .flat_map(|p| &p.faces)
        .filter_map(|f| f.mesh.as_ref())
    {
        let pts: Vec<Vec3> = mesh.positions.iter().map(|p| t.apply_point(*p)).collect();
        for tri in &mesh.triangles {
            out.push(tri.map(|i| pts[i as usize]));
        }
    }
    out
}

fn analyse_one(
    m: &erc_step::Model,
    entry: &ModelEntry,
    pads: &[(&Pad, Vec<Vec<[f64; 2]>>)],
    courtyard: &[[[f64; 2]; 2]],
    opts: &ModelOptions,
) -> Analysis {
    let tris = placed_triangles(m, entry);
    let extents = extents(&tris);
    let courtyard =
        (!courtyard.is_empty() && !tris.is_empty()).then(|| courtyard_fit(&tris, courtyard));
    let leads = extents.map(|e| leads(&tris, e.min[2], opts.slice_mm, pads));
    Analysis {
        unit: m.unit.name.clone(),
        unit_mm: m.unit.mm,
        parts: m.parts.len(),
        faces: m.face_count(),
        faces_meshed: m
            .parts
            .iter()
            .flat_map(|p| &p.faces)
            .filter(|f| f.mesh.is_some())
            .count(),
        triangles: m.triangle_count(),
        warnings: m.warnings.iter().map(|w| w.to_string()).collect(),
        extents,
        courtyard,
        leads,
    }
}

fn extents(tris: &[[Vec3; 3]]) -> Option<Extents> {
    let first = tris.first()?[0];
    let (mut min, mut max) = (first, first);
    for p in tris.iter().flatten() {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    Some(Extents {
        min,
        max,
        size: [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
        height: max[2].max(0.0),
        depth: (-min[2]).max(0.0),
        centre: [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0],
    })
}

/// Even-odd: whether `p` is inside the loops the segments form, whatever order they come in
fn inside(p: [f64; 2], segments: &[[[f64; 2]; 2]]) -> bool {
    let mut odd = false;
    for [a, b] in segments {
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p[0] < x {
                odd = !odd;
            }
        }
    }
    odd
}

fn segment_distance(p: [f64; 2], [a, b]: &[[f64; 2]; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (qx * qx + qy * qy).sqrt()
}

/// How far the model's outline seen from above reaches out of the courtyard: the furthest vertex outside it
fn courtyard_fit(tris: &[[Vec3; 3]], courtyard: &[[[f64; 2]; 2]]) -> CourtyardFit {
    let mut worst: Option<([f64; 2], f64)> = None;
    let mut seen = std::collections::HashSet::new();
    for p in tris.iter().flatten() {
        let q = [p[0], p[1]];
        if !seen.insert([q[0].to_bits(), q[1].to_bits()]) || inside(q, courtyard) {
            continue;
        }
        let d = courtyard
            .iter()
            .map(|s| segment_distance(q, s))
            .fold(f64::INFINITY, f64::min);
        // Points on the line itself are in
        if d > 1e-6 && worst.is_none_or(|(_, w)| d > w) {
            worst = Some((q, d));
        }
    }
    CourtyardFit {
        overhang: worst.map_or(0.0, |(_, d)| d),
        worst: worst.map(|(q, _)| q),
    }
}

/// The parts of a triangle with `lo <= z <= hi`, seen from below: a convex polygon in x and y, empty when none
fn clip_slab(tri: &[Vec3; 3], lo: f64, hi: f64) -> Vec<[f64; 2]> {
    fn clip(poly: Vec<Vec3>, keep: impl Fn(&Vec3) -> f64) -> Vec<Vec3> {
        let mut out = Vec::new();
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let (da, db) = (keep(&a), keep(&b));
            if da >= 0.0 {
                out.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                out.push([0, 1, 2].map(|k| a[k] + t * (b[k] - a[k])));
            }
        }
        out
    }
    let poly = clip(tri.to_vec(), |p| hi - p[2]);
    let poly = clip(poly, |p| p[2] - lo);
    poly.into_iter().map(|p| [p[0], p[1]]).collect()
}

/// Whether `p` is inside the polygon (even-odd)
fn in_polygon(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut odd = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
            if p[0] < x {
                odd = !odd;
            }
        }
    }
    odd
}

/// A grid of cells over a box: the leads and the pads are counted on it, each cell by its middle
struct Grid {
    lo: [f64; 2],
    cell: f64,
    w: usize,
    h: usize,
}

impl Grid {
    /// The cells whose middle is inside `poly`
    fn cells(&self, poly: &[[f64; 2]], mut each: impl FnMut(usize)) {
        let Some((lo, hi)) = bbox2(poly.iter()) else {
            return;
        };
        let x0 = (((lo[0] - self.lo[0]) / self.cell) - 0.5).floor().max(0.0) as usize;
        let y0 = (((lo[1] - self.lo[1]) / self.cell) - 0.5).floor().max(0.0) as usize;
        let x1 =
            ((((hi[0] - self.lo[0]) / self.cell) - 0.5).ceil().max(0.0) as usize).min(self.w - 1);
        let y1 =
            ((((hi[1] - self.lo[1]) / self.cell) - 0.5).ceil().max(0.0) as usize).min(self.h - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let c = [
                    self.lo[0] + (x as f64 + 0.5) * self.cell,
                    self.lo[1] + (y as f64 + 0.5) * self.cell,
                ];
                if in_polygon(c, poly) {
                    each(y * self.w + x);
                }
            }
        }
    }
}

/// The model's bottom `slice_mm` from `z0`, seen from below, against each pad's copper: counted on a grid of at
/// most 2000 cells a side (0.005 mm cells for a package up to 10 mm)
fn leads(tris: &[[Vec3; 3]], z0: f64, slice_mm: f64, pads: &[(&Pad, Vec<Vec<[f64; 2]>>)]) -> Leads {
    let slice = [z0, z0 + slice_mm];
    let shadows: Vec<Vec<[f64; 2]>> = tris
        .iter()
        .map(|t| clip_slab(t, slice[0], slice[1]))
        .filter(|p| p.len() > 2)
        .collect();
    let all = shadows
        .iter()
        .flatten()
        .chain(pads.iter().flat_map(|(_, polys)| polys.iter().flatten()));
    let Some((lo, hi)) = bbox2(all) else {
        return Leads {
            slice,
            area: 0.0,
            off_pads: 0.0,
            pads: Vec::new(),
            resolution: 0.0,
        };
    };
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]);
    let cell = (span / 2000.0).max(0.005);
    let grid = Grid {
        lo,
        cell,
        w: ((hi[0] - lo[0]) / cell).ceil() as usize + 1,
        h: ((hi[1] - lo[1]) / cell).ceil() as usize + 1,
    };
    let mut lead = vec![false; grid.w * grid.h];
    for s in &shadows {
        grid.cells(s, |i| lead[i] = true);
    }
    let mut on_pad = vec![false; grid.w * grid.h];
    let a = cell * cell;
    let mut out = Vec::new();
    for (pad, polys) in pads {
        let mut mine = std::collections::HashSet::new();
        for poly in polys {
            grid.cells(poly, |i| {
                mine.insert(i);
            });
        }
        let covered = mine.iter().filter(|i| lead[**i]).count();
        for i in &mine {
            on_pad[*i] = true;
        }
        out.push(PadLeads {
            number: pad.number.clone(),
            area: mine.len() as f64 * a,
            covered: covered as f64 * a,
        });
    }
    let total = lead.iter().filter(|l| **l).count();
    let off = lead
        .iter()
        .zip(&on_pad)
        .filter(|(l, p)| **l && !**p)
        .count();
    Leads {
        slice,
        area: total as f64 * a,
        off_pads: off as f64 * a,
        pads: out,
        resolution: cell,
    }
}

/// Whether a value is a whole multiple of 90° (to a millionth of a degree)
fn right_angle(deg: f64) -> bool {
    let d = erc_model::footprint::deg360(deg);
    (d / 90.0).fract().abs() < 1e-9
}

/// The findings on a footprint's models (`LIB-22` to `LIB-28`)
fn findings(models: &[ModelReport], wants_model: bool) -> Vec<ModelFinding> {
    let mut out = Vec::new();
    let mut push = |check, severity, model, message: String| {
        out.push(ModelFinding {
            check,
            severity,
            model,
            message,
        })
    };
    if wants_model && models.iter().all(|m| m.entry.hide) {
        let message = if models.is_empty() {
            "no 3D model".to_string()
        } else {
            "every 3D model is hidden".to_string()
        };
        push("LIB-28", Severity::Suggestion, None, message);
    }
    for (i, m) in models.iter().enumerate() {
        let e = &m.entry;
        match &m.resolved {
            Resolved::UnknownVar { var } => push(
                "LIB-22",
                Severity::Warning,
                Some(i),
                format!("model file not found: ${{{var}}} is not set"),
            ),
            Resolved::NotFound { path } => push(
                "LIB-22",
                Severity::Warning,
                Some(i),
                format!("model file not found: {}", path.display()),
            ),
            Resolved::Found { .. } => {}
        }
        if let Some(err) = &m.read_error {
            push(
                "LIB-22",
                Severity::Warning,
                Some(i),
                format!("model file not read: {err}"),
            );
        }
        if [e.scale.x, e.scale.y, e.scale.z]
            .iter()
            .any(|s| (s - 1.0).abs() > 1e-9)
        {
            push(
                "LIB-25",
                Severity::Warning,
                Some(i),
                format!(
                    "scaled by {} {} {}: the model is not the package's size",
                    e.scale.x, e.scale.y, e.scale.z
                ),
            );
        }
        if ![e.rotate.x, e.rotate.y, e.rotate.z]
            .iter()
            .all(|d| right_angle(*d))
        {
            push(
                "LIB-27",
                Severity::Warning,
                Some(i),
                format!(
                    "turned by {}° {}° {}°: not a multiple of 90°",
                    e.rotate.x, e.rotate.y, e.rotate.z
                ),
            );
        }
        let Some(a) = &m.analysis else {
            continue;
        };
        if (a.unit_mm - 1.0).abs() > 1e-12 {
            push(
                "LIB-26",
                Severity::Suggestion,
                Some(i),
                format!("written in {} (read as {} mm each)", a.unit, a.unit_mm),
            );
        }
        if let Some(c) = &a.courtyard
            && c.overhang > 0.005
        {
            push(
                "LIB-23",
                Severity::Warning,
                Some(i),
                format!("reaches {:.3} mm out of the courtyard", c.overhang),
            );
        }
        if let Some(l) = &a.leads {
            let bare: Vec<&str> = l
                .pads
                .iter()
                .filter(|p| p.area > 0.0 && p.covered == 0.0)
                .map(|p| p.number.as_str())
                .collect();
            if !bare.is_empty() {
                push(
                    "LIB-24",
                    Severity::Warning,
                    Some(i),
                    format!(
                        "no lead on pad{} {}",
                        if bare.len() == 1 { "" } else { "s" },
                        bare.join(", ")
                    ),
                );
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------------------------
// berc fp model

/// `berc fp model` over a board, a footprint library or one footprint file
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelRun {
    /// What was given: the board, the `.pretty` folder or the `.kicad_mod` file
    pub target: PathBuf,
    /// The directory model paths and `KIPRJMOD` are resolved against
    pub project_dir: PathBuf,
    /// By reference on a board, by name in a library
    pub footprints: Vec<FootprintModels>,
}

/// The board of a KiCad project: the `.kicad_pcb` itself, the one beside a `.kicad_pro` or `.kicad_sch`, or the
/// one in a directory (the project's own when there are several)
fn board_of(path: &Path) -> Option<PathBuf> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if path.is_file() {
        return match ext {
            "kicad_pcb" => Some(path.to_path_buf()),
            "kicad_pro" | "kicad_sch" => {
                Some(path.with_extension("kicad_pcb")).filter(|p| p.is_file())
            }
            _ => None,
        };
    }
    let mut boards: Vec<PathBuf> = std::fs::read_dir(path)
        .ok()?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "kicad_pcb"))
        .collect();
    boards.sort();
    if boards.len() > 1
        && let Some(own) = boards
            .iter()
            .find(|b| b.with_extension("kicad_pro").is_file())
    {
        return Some(own.clone());
    }
    boards.into_iter().next()
}

/// Analyses the models of the footprints at `target`: a KiCad project (a directory, its `.kicad_pro`,
/// `.kicad_sch` or `.kicad_pcb`) with `names` the references to look at, a footprint library (`.pretty`) with
/// `names` the footprints, or one `.kicad_mod`; every footprint when `names` is empty. Model paths are resolved
/// against `project` when given, else the board's directory or the library's parent. `progress` hears each
/// footprint as it starts.
pub fn run(
    target: &Path,
    names: &[String],
    project: Option<&Path>,
    opts: &ModelOptions,
    progress: &mut dyn FnMut(&str),
) -> Result<ModelRun, super::FpError> {
    use super::FpError;
    let parent = |p: &Path| {
        p.parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
    };
    let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut cache = ModelCache::default();
    // (source, reference, footprint), and the default project directory
    let (items, dir): (Vec<(String, Option<String>, LocalFootprint)>, PathBuf) =
        if ext == "kicad_mod" {
            if !names.is_empty() {
                return Err(FpError::ReferenceOnLibrary(target.to_path_buf()));
            }
            let f = super::read_library(target)?;
            let lib = parent(target);
            let dir = if lib.extension().is_some_and(|e| e == "pretty") {
                parent(&lib)
            } else {
                lib
            };
            (vec![(target.display().to_string(), None, f)], dir)
        } else if ext == "pretty" && target.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(target)
                .map_err(|e| FpError::Read(target.to_path_buf(), e))?
                .filter_map(|e| Some(e.ok()?.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "kicad_mod"))
                .collect();
            files.sort();
            let stem = |p: &Path| {
                p.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };
            for n in names {
                if !files.iter().any(|f| &stem(f) == n) {
                    return Err(FpError::NoFootprint {
                        board: target.to_path_buf(),
                        reference: n.clone(),
                    });
                }
            }
            let mut items = Vec::new();
            for file in files
                .iter()
                .filter(|f| names.is_empty() || names.contains(&stem(f)))
            {
                items.push((file.display().to_string(), None, super::read_library(file)?));
            }
            (items, parent(target))
        } else {
            let board = board_of(target).ok_or_else(|| FpError::NoBoard(target.to_path_buf()))?;
            let lay = super::read_board(&board)?;
            for n in names {
                if !lay.footprints.iter().any(|f| &f.reference == n) {
                    return Err(FpError::NoFootprint {
                        board: board.clone(),
                        reference: n.clone(),
                    });
                }
            }
            let mut order: Vec<usize> = (0..lay.footprints.len())
                .filter(|i| names.is_empty() || names.contains(&lay.footprints[*i].reference))
                .collect();
            order.sort_by(|a, b| {
                erc_model::natural_cmp(&lay.footprints[*a].reference, &lay.footprints[*b].reference)
            });
            let items = order
                .into_iter()
                .map(|i| {
                    let r = lay.footprints[i].reference.clone();
                    (
                        format!("{r} on {}", board.display()),
                        Some(r),
                        lay.local_footprint(erc_model::layout::FootprintId::from_index(i)),
                    )
                })
                .collect();
            (items, parent(&board))
        };
    let project_dir = project.map_or(dir, Path::to_path_buf);
    let vars = erc_kicad::libtable::KicadVars::new(&project_dir, opts.config.as_deref());
    let var = |n: &str| vars.get(n);
    let mut footprints = Vec::new();
    for (source, reference, f) in items {
        progress(reference.as_deref().unwrap_or(&f.footprint.lib_id));
        footprints.push(analyze(
            source,
            reference,
            &f,
            &project_dir,
            &var,
            &mut cache,
            opts,
        ));
    }
    Ok(ModelRun {
        target: target.to_path_buf(),
        project_dir,
        footprints,
    })
}

impl ModelRun {
    /// Exit code 1: a warning or an error on any footprint; suggestions (no model, not in mm) do not fail the run
    pub fn is_ok(&self) -> bool {
        self.footprints
            .iter()
            .flat_map(|f| &f.findings)
            .all(|f| !matches!(f.severity, Severity::Error | Severity::Warning))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialises")
    }

    /// A block per footprint: each model's file, what it holds, its size, height and centre, how it sits against
    /// the courtyard and the pads, then the findings; `details`: every pad's coverage and every reader warning
    pub fn to_text_color(&self, details: bool) -> String {
        use anstyle::{AnsiColor, Style};
        use std::fmt::Write as _;
        let dim = Style::new().dimmed();
        let bold = Style::new().bold();
        let sev = |s: Severity| match s {
            Severity::Error => AnsiColor::Red.on_default().bold(),
            Severity::Warning => AnsiColor::Yellow.on_default().bold(),
            Severity::Suggestion => AnsiColor::Cyan.on_default(),
            Severity::Info => Style::new().dimmed(),
        };
        let mut out = String::new();
        let mut counts: HashMap<&'static str, usize> = HashMap::new();
        for f in &self.footprints {
            let name = match &f.reference {
                Some(r) => format!("{r}  {}", f.lib_id),
                None => f.lib_id.clone(),
            };
            let _ = writeln!(out, "{bold}{name}{bold:#}  {dim}{}{dim:#}", f.source);
            if f.models.is_empty() {
                let _ = writeln!(out, "  {dim}no model{dim:#}");
            }
            for (i, m) in f.models.iter().enumerate() {
                let hidden = if m.entry.hide { " (hidden)" } else { "" };
                let _ = writeln!(out, "  model {}{hidden}", m.entry.path);
                let e = &m.entry;
                if e.offset != Default::default()
                    || e.rotate != Default::default()
                    || e.scale != erc_model::layout::Xyz::new(1.0, 1.0, 1.0)
                {
                    let xyz = |v: &erc_model::layout::Xyz| format!("{} {} {}", v.x, v.y, v.z);
                    let _ = writeln!(
                        out,
                        "    {dim}offset{dim:#} {}  {dim}rotate{dim:#} {}  {dim}scale{dim:#} {}",
                        xyz(&e.offset),
                        xyz(&e.rotate),
                        xyz(&e.scale)
                    );
                }
                match &m.resolved {
                    Resolved::Found { path, vars } => {
                        let via = if vars.is_empty() {
                            String::new()
                        } else {
                            format!("  {dim}through {}{dim:#}", vars.join(", "))
                        };
                        let _ = writeln!(out, "    {dim}file{dim:#}      {}{via}", path.display());
                    }
                    Resolved::UnknownVar { var } => {
                        let _ = writeln!(out, "    {dim}file{dim:#}      ${{{var}}} is not set");
                    }
                    Resolved::NotFound { path } => {
                        let _ =
                            writeln!(out, "    {dim}file{dim:#}      none at {}", path.display());
                    }
                }
                if let Some(err) = &m.read_error {
                    let _ = writeln!(out, "    {dim}read{dim:#}      {err}");
                }
                if let Some(a) = &m.analysis {
                    write_analysis(&mut out, a, f.pads_centre, details);
                }
                for x in f.findings.iter().filter(|x| x.model == Some(i)) {
                    let st = sev(x.severity);
                    let _ = writeln!(out, "    {st}{}{st:#} {}", x.check, x.message);
                    *counts.entry(x.check).or_default() += 1;
                }
            }
            for x in f.findings.iter().filter(|x| x.model.is_none()) {
                let st = sev(x.severity);
                let _ = writeln!(out, "  {st}{}{st:#} {}", x.check, x.message);
                *counts.entry(x.check).or_default() += 1;
            }
        }
        let models: usize = self.footprints.iter().map(|f| f.models.len()).sum();
        let _ = write!(
            out,
            "{} footprint{}, {models} model{}",
            self.footprints.len(),
            if self.footprints.len() == 1 { "" } else { "s" },
            if models == 1 { "" } else { "s" }
        );
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort();
        if counts.is_empty() {
            let _ = writeln!(out, ", no findings");
        } else {
            let list: Vec<String> = counts.iter().map(|(c, n)| format!("{c} {n}")).collect();
            let _ = writeln!(out, ", findings: {}", list.join(", "));
        }
        out
    }
}

fn write_analysis(out: &mut String, a: &Analysis, pads_centre: Option<[f64; 2]>, details: bool) {
    use anstyle::Style;
    use std::fmt::Write as _;
    let dim = Style::new().dimmed();
    let unit = if (a.unit_mm - 1.0).abs() < 1e-12 {
        a.unit.clone()
    } else {
        format!("{} ({} mm)", a.unit, a.unit_mm)
    };
    let _ = writeln!(
        out,
        "    {dim}read{dim:#}      {unit}, {} part{}, {} faces ({} meshed), {} triangles",
        a.parts,
        if a.parts == 1 { "" } else { "s" },
        a.faces,
        a.faces_meshed,
        a.triangles
    );
    if !a.warnings.is_empty() {
        if details {
            for w in &a.warnings {
                let _ = writeln!(out, "    {dim}warning{dim:#}   {w}");
            }
        } else {
            let _ = writeln!(
                out,
                "    {dim}warnings{dim:#}  {} (--details lists them)",
                a.warnings.len()
            );
        }
    }
    let Some(e) = &a.extents else {
        let _ = writeln!(out, "    {dim}nothing meshed{dim:#}");
        return;
    };
    let _ = writeln!(
        out,
        "    {dim}size{dim:#}      {} x {} x {} mm  {dim}x {}..{}  y {}..{}  z {}..{} (y up){dim:#}",
        Mm(e.size[0]),
        Mm(e.size[1]),
        Mm(e.size[2]),
        Mm(e.min[0]),
        Mm(e.max[0]),
        Mm(e.min[1]),
        Mm(e.max[1]),
        Mm(e.min[2]),
        Mm(e.max[2])
    );
    let depth = if e.depth >= 0.0005 {
        format!(", {} mm below it", Mm(e.depth))
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "    {dim}height{dim:#}    {} mm above the board{depth}",
        Mm(e.height)
    );
    match pads_centre {
        Some(p) => {
            let (dx, dy) = (e.centre[0] - p[0], e.centre[1] - p[1]);
            let _ = writeln!(
                out,
                "    {dim}centre{dim:#}    {}, {}  {dim}the pads' {}, {}: off by {}, {}{dim:#}",
                Mm(e.centre[0]),
                Mm(e.centre[1]),
                Mm(p[0]),
                Mm(p[1]),
                Mm(dx),
                Mm(dy)
            );
        }
        None => {
            let _ = writeln!(
                out,
                "    {dim}centre{dim:#}    {}, {}",
                Mm(e.centre[0]),
                Mm(e.centre[1])
            );
        }
    }
    match &a.courtyard {
        Some(c) if c.overhang > 0.0 => {
            let at = c
                .worst
                .map_or(String::new(), |w| format!(" at {}, {}", Mm(w[0]), Mm(w[1])));
            let _ = writeln!(
                out,
                "    {dim}courtyard{dim:#} {} mm out{at}",
                Mm(c.overhang)
            );
        }
        Some(_) => {
            let _ = writeln!(out, "    {dim}courtyard{dim:#} inside");
        }
        None => {
            let _ = writeln!(out, "    {dim}courtyard{dim:#} none to hold it against");
        }
    }
    if let Some(l) = &a.leads {
        let under = l.pads.iter().filter(|p| p.covered > 0.0).count();
        let _ = writeln!(
            out,
            "    {dim}leads{dim:#}     {under} of {} pads under a lead, {} of {} mm² off the pads  {dim}(the bottom {:.2} mm, z {}..{}){dim:#}",
            l.pads.len(),
            Mm(l.off_pads),
            Mm(l.area),
            l.slice[1] - l.slice[0],
            Mm(l.slice[0]),
            Mm(l.slice[1])
        );
        if details {
            for p in &l.pads {
                let _ = writeln!(
                    out,
                    "      {dim}pad{dim:#} {:<4} {:5.1} % of {} mm² under a lead",
                    p.number,
                    p.fraction() * 100.0,
                    Mm(p.area)
                );
            }
        }
    }
}

/// A length (or an area) to the micrometre, never `-0.000`
struct Mm(f64);

impl std::fmt::Display for Mm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = (self.0 * 1000.0).round() / 1000.0;
        write!(f, "{:.3}", if v == 0.0 { 0.0 } else { v })
    }
}

/// The directory a footprint's model paths and `KIPRJMOD` resolve against, as `berc fp model` takes it: a board's
/// directory, a library footprint's project, a `.kicad_mod`'s library's parent (the folder beside a `.pretty`)
pub fn project_dir(source: &super::FootprintSource) -> PathBuf {
    use super::FootprintSource;
    let parent = |p: &Path| {
        p.parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
    };
    match source {
        FootprintSource::File(file) => {
            let lib = parent(file);
            if lib.extension().is_some_and(|e| e == "pretty") {
                parent(&lib)
            } else {
                lib
            }
        }
        FootprintSource::Board { board, .. } => parent(board),
        FootprintSource::Library { project, .. } if project.is_file() => parent(project),
        FootprintSource::Library { project, .. } => project.clone(),
    }
}

/// The copper of each pad on the front, in the footprint's 3D frame (y up): its number and its outline polygons
pub fn front_pad_copper(f: &LocalFootprint) -> Vec<(String, Vec<Vec<[f64; 2]>>)> {
    f.pads
        .iter()
        .filter(|p| front_copper(p, &f.pads))
        .map(|p| {
            let polys = outline::pad_copper(p)
                .into_iter()
                .map(|poly| poly.into_iter().map(up).collect())
                .collect();
            (p.number.clone(), polys)
        })
        .collect()
}

/// A footprint's models for a 3D view: the analysis of every entry, and the model each entry read, in the order of
/// [`FootprintModels::models`] (`None` where the file was not found; the error where it did not read). Place a
/// model with [`placement`] of its entry.
pub struct ViewModels {
    pub report: FootprintModels,
    pub models: Vec<Option<Arc<Result<erc_step::Model, String>>>>,
}

/// Reads and analyses the models of `sourced` for a view, paths resolved as [`project_dir`] and `opts.config` say
pub fn for_view(sourced: &super::Sourced, opts: &ModelOptions) -> ViewModels {
    let dir = project_dir(&sourced.source);
    let vars = erc_kicad::libtable::KicadVars::new(&dir, opts.config.as_deref());
    let var = |n: &str| vars.get(n);
    let mut cache = ModelCache::default();
    let report = analyze(
        sourced.footprint.footprint.lib_id.clone(),
        None,
        &sourced.footprint,
        &dir,
        &var,
        &mut cache,
        opts,
    );
    let models = report
        .models
        .iter()
        .map(|m| match &m.resolved {
            Resolved::Found { path, .. } => Some(cache.get(path, &opts.read)),
            _ => None,
        })
        .collect();
    ViewModels { report, models }
}

#[cfg(test)]
mod tests {
    use super::*;
    use erc_model::layout::Xyz;

    fn close(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
    }

    fn entry(offset: [f64; 3], scale: [f64; 3], rotate: [f64; 3]) -> ModelEntry {
        let x = |v: [f64; 3]| Xyz::new(v[0], v[1], v[2]);
        ModelEntry {
            offset: x(offset),
            scale: x(scale),
            rotate: x(rotate),
            ..ModelEntry::default()
        }
    }

    /// What kicad-cli 10 made of a box from (0, 0, 0) to (3, 2, 1) under each single setting
    #[test]
    fn placement_matches_kicad_per_axis() {
        let corner = [3.0, 2.0, 1.0];
        let t = |e: ModelEntry| placement(&e).apply_point(corner);
        assert!(close(
            t(entry([1.0, 2.0, 3.0], [1.0; 3], [0.0; 3])),
            [4.0, 4.0, 4.0]
        ));
        // rotate z 90: the box lands at x 0..2, y -3..0
        assert!(close(
            t(entry([0.0; 3], [1.0; 3], [0.0, 0.0, 90.0])),
            [2.0, -3.0, 1.0]
        ));
        // rotate x 90: y 0..1, z -2..0
        assert!(close(
            t(entry([0.0; 3], [1.0; 3], [90.0, 0.0, 0.0])),
            [3.0, 1.0, -2.0]
        ));
        // rotate y 90: x -1..0, z 0..3
        assert!(close(
            t(entry([0.0; 3], [1.0; 3], [0.0, 90.0, 0.0])),
            [-1.0, 2.0, 3.0]
        ));
        assert!(close(
            t(entry([0.0; 3], [2.0, 1.0, 1.0], [0.0; 3])),
            [6.0, 2.0, 1.0]
        ));
    }

    #[test]
    fn courtyard_inside_and_distance() {
        let sq = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let segs: Vec<[[f64; 2]; 2]> = (0..4).map(|i| [sq[i], sq[(i + 1) % 4]]).collect();
        assert!(inside([1.0, 1.0], &segs));
        assert!(!inside([3.0, 1.0], &segs));
        let tris = [[[1.0, 1.0, 0.0], [2.5, 1.0, 0.0], [1.0, 1.5, 0.0]]];
        let fit = courtyard_fit(&tris, &segs);
        assert!((fit.overhang - 0.5).abs() < 1e-12);
        assert_eq!(fit.worst, Some([2.5, 1.0]));
    }

    #[test]
    fn slab_clipping() {
        let tri = [[0.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]];
        assert_eq!(clip_slab(&tri, 2.0, 3.0).len(), 0);
        assert_eq!(clip_slab(&tri, -1.0, 2.0).len(), 3);
        assert_eq!(clip_slab(&tri, 0.0, 0.5).len(), 3);
        assert_eq!(clip_slab(&tri, 0.25, 0.5).len(), 4);
    }

    #[test]
    fn right_angles() {
        assert!(right_angle(-90.0) && right_angle(270.0) && right_angle(0.0));
        assert!(!right_angle(45.0) && !right_angle(90.5));
    }
}
