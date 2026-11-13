//! STEP reader for better_erc: a STEP file (AP203, AP214, AP242 B-rep) into a [`Model`] of parts, each with its
//! transform and colour, its faces with triangles and their exact surface kind, and its edges with a polyline and
//! their curve kind, all in mm in the model frame. [`measure`] has the measurements on those kinds.
//!
//! Parsing is ruststep's, the B-rep conversion and the tessellation truck's (pure Rust, no OpenCascade). Our own
//! walk over the entities reads what truck does not: the length unit, the assembly tree with its transforms, and
//! the colours. Everything that goes wrong on the way is a [`Warning`] with its STEP entity id, never a lost model:
//! a face truck cannot convert or tessellate is reported and left out, and the rest of the model stays.

mod entities;
mod kinds;
pub mod math;
pub mod measure;
mod mesh;
mod structure;
mod text;
pub mod write;

pub use math::{Transform, Vec3};

use std::path::Path;
use std::time::{Duration, Instant};

/// How to read a file.
#[derive(Clone, Debug)]
pub struct Options {
    /// The chord tolerance of the tessellation in mm: how far a triangle may be from the true surface.
    pub tolerance_mm: f64,
    /// The time the tessellation may take. A face still being tessellated after it is not meshed (a warning).
    /// [`read_file`] and [`read_bytes`] give up and return [`ReadError::Timeout`] after twice this time plus a
    /// second, for the rare file that hangs in truck's conversion rather than in the tessellation; the worker
    /// thread is then left behind. A batch over many files runs each in a child process instead.
    pub budget: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            tolerance_mm: 0.01,
            budget: Duration::from_secs(10),
        }
    }
}

/// An RGB colour, each channel 0 to 1.
pub type Colour = [f32; 3];

/// A STEP model in mm.
#[derive(Clone, Debug, Default)]
pub struct Model {
    /// The length unit the file is written in (of its top-level shape).
    pub unit: LengthUnit,
    /// The solids, each placed in the model frame. A part used several times in an assembly is here once per use.
    pub parts: Vec<Part>,
    /// Everything that was not read as it should be.
    pub warnings: Vec<Warning>,
}

/// A length unit and its size in mm.
#[derive(Clone, Debug, PartialEq)]
pub struct LengthUnit {
    /// `mm`, `m`, `inch`, ... as the file names it.
    pub name: String,
    /// One unit in mm.
    pub mm: f64,
}

impl Default for LengthUnit {
    fn default() -> Self {
        LengthUnit {
            name: "mm".into(),
            mm: 1.0,
        }
    }
}

/// One solid (or surface model) of the file, placed in the model frame.
#[derive(Clone, Debug)]
pub struct Part {
    /// The product's name, or the solid's own name when the file has no product for it.
    pub name: String,
    /// The STEP entity of the solid (`MANIFOLD_SOLID_BREP`, `BREP_WITH_VOIDS`, `SHELL_BASED_SURFACE_MODEL`).
    pub step_id: u64,
    /// Where the assembly tree puts the solid: its own coordinates (scaled to mm) into the model frame.
    pub transform: Transform,
    /// The solid's colour, when the file gives one.
    pub colour: Option<Colour>,
    /// The faces, in the model frame.
    pub faces: Vec<Face>,
    /// The edges, in the model frame.
    pub edges: Vec<Edge>,
}

/// A face of a part.
#[derive(Clone, Debug)]
pub struct Face {
    /// The STEP entity (`ADVANCED_FACE`).
    pub step_id: u64,
    /// The exact surface the face lies on.
    pub surface: SurfaceKind,
    /// The face's colour: its own, else its shell's or its solid's.
    pub colour: Option<Colour>,
    /// The triangles; `None` when truck could not tessellate the face (there is a warning for it).
    pub mesh: Option<Mesh>,
}

/// Triangles in mm in the model frame, wound counter-clockwise seen from outside the solid.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    /// One unit normal per position, pointing out of the solid.
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

/// The exact surface of a face, from the STEP B-rep (not fitted to triangles), in mm in the model frame.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceKind {
    /// `normal` points out of the solid.
    Plane { origin: Vec3, normal: Vec3 },
    /// `outward` is false for a hole (the solid is outside the cylinder).
    Cylinder {
        axis_origin: Vec3,
        axis: Vec3,
        radius: f64,
        outward: bool,
    },
    /// `radius` at `origin` on the axis; the half angle in radians, the cone opening towards `axis`.
    Cone {
        origin: Vec3,
        axis: Vec3,
        radius: f64,
        half_angle: f64,
    },
    Sphere {
        centre: Vec3,
        radius: f64,
        outward: bool,
    },
    Torus {
        centre: Vec3,
        axis: Vec3,
        major_radius: f64,
        minor_radius: f64,
    },
    /// B-spline, NURBS, extrusion, revolution, offset and other surfaces.
    Freeform,
}

/// An edge of a part.
#[derive(Clone, Debug)]
pub struct Edge {
    /// The STEP entity (`EDGE_CURVE`).
    pub step_id: u64,
    pub curve: CurveKind,
    /// Points along the edge in mm in the model frame; empty when it could not be tessellated.
    pub polyline: Vec<Vec3>,
    /// The faces of the part the edge bounds (indices into [`Part::faces`]), usually two.
    pub faces: Vec<usize>,
}

/// The exact curve of an edge, in mm in the model frame.
#[derive(Clone, Debug, PartialEq)]
pub enum CurveKind {
    Line {
        start: Vec3,
        end: Vec3,
    },
    /// A full circle or an arc of it.
    Circle {
        centre: Vec3,
        axis: Vec3,
        radius: f64,
    },
    Other,
}

/// Something not read as it should be.
#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    /// The STEP entity it is about, when there is one.
    pub entity: Option<u64>,
    pub kind: WarningKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WarningKind {
    /// An entity the B-rep needs could not be read (a malformed or unsupported record).
    EntityNotRead,
    /// A face left out: its surface or an edge could not be converted.
    FaceDropped,
    /// A face converted but not tessellated (a panic in truck, or the time budget ran out).
    FaceNotMeshed,
    /// A curve drawn simpler than it is (a hyperbola as a straight edge).
    Approximated,
    /// The assembly tree could not place something; it is shown where its own coordinates put it.
    Placement,
    /// A unit the reader does not know; millimetres assumed.
    Unit,
    /// Geometry of a kind the reader does not read (faceted B-reps, tessellated shells).
    Unsupported,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.entity {
            Some(id) => write!(f, "#{id}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// Why a file could not be read at all.
#[derive(Debug)]
pub enum ReadError {
    Io(std::io::Error),
    /// Not a STEP file, or one ruststep cannot parse.
    Parse(String),
    /// Reading took longer than the budget allows (see [`Options::budget`]).
    Timeout,
    /// Reading panicked outside the parts that catch panics.
    Panic(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Io(e) => write!(f, "cannot read the file: {e}"),
            ReadError::Parse(e) => write!(f, "not a STEP file ruststep can parse: {e}"),
            ReadError::Timeout => f.write_str("reading took longer than the time budget"),
            ReadError::Panic(e) => write!(f, "the reader panicked: {e}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads a STEP file. See [`Options`] for the time limits.
pub fn read_file(path: impl AsRef<Path>, options: &Options) -> Result<Model, ReadError> {
    let bytes = std::fs::read(path).map_err(ReadError::Io)?;
    read_bytes(bytes, options)
}

/// Reads a STEP file's contents on a worker thread, within the time limits of [`Options`].
pub fn read_bytes(bytes: Vec<u8>, options: &Options) -> Result<Model, ReadError> {
    let (tx, rx) = std::sync::mpsc::channel();
    let opts = options.clone();
    std::thread::Builder::new()
        .name("erc_step read".into())
        .spawn(move || {
            let text = String::from_utf8_lossy(&bytes);
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_str(&text, &opts)));
            let _ = tx.send(result.unwrap_or_else(|e| Err(ReadError::Panic(mesh::panic_text(&e)))));
        })
        .map_err(ReadError::Io)?;
    rx.recv_timeout(options.budget * 2 + Duration::from_secs(1))
        .unwrap_or(Err(ReadError::Timeout))
}

/// Reads a STEP file's text on this thread. The tessellation stops at the budget; truck's conversion of a
/// curve may still hang (very rare), which only [`read_bytes`] guards against.
pub fn read_str(text: &str, options: &Options) -> Result<Model, ReadError> {
    let deadline = Instant::now() + options.budget;
    let text = text::fix_unnamed(text);
    let exchange =
        ruststep::parser::parse(&text).map_err(|e| ReadError::Parse(short(&e.to_string())))?;
    let Some(data) = exchange.data.first() else {
        return Err(ReadError::Parse("no DATA section".into()));
    };
    let ents = entities::Entities::new(&data.entities);
    let mut warnings = Vec::new();
    let structure = structure::Structure::read(&ents, &mut warnings);
    let table = mesh::table(&data.entities, structure.angle_rad, &mut warnings);
    let mut mesher = mesh::Mesher::new(&table, &ents, options.tolerance_mm, deadline);
    let mut parts = Vec::new();
    for placed in &structure.placed {
        parts.push(mesher.part(placed, &structure));
    }
    warnings.extend(mesher.take_warnings());
    Ok(Model {
        unit: structure.unit.clone(),
        parts,
        warnings,
    })
}

fn short(s: &str) -> String {
    let line = s.lines().next().unwrap_or_default();
    line.chars().take(300).collect()
}

/// An axis-aligned box in mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn size(&self) -> Vec3 {
        math::sub(self.max, self.min)
    }

    fn grow(b: &mut Option<Bounds>, p: Vec3) {
        match b {
            Some(b) => {
                for (i, x) in p.into_iter().enumerate() {
                    b.min[i] = b.min[i].min(x);
                    b.max[i] = b.max[i].max(x);
                }
            }
            None => *b = Some(Bounds { min: p, max: p }),
        }
    }
}

impl Model {
    /// The box around every meshed triangle; `None` when nothing was meshed.
    pub fn bounds(&self) -> Option<Bounds> {
        let mut b = None;
        for part in &self.parts {
            for p in part
                .faces
                .iter()
                .filter_map(|f| f.mesh.as_ref())
                .flat_map(|m| &m.positions)
            {
                Bounds::grow(&mut b, *p);
            }
        }
        b
    }

    pub fn face_count(&self) -> usize {
        self.parts.iter().map(|p| p.faces.len()).sum()
    }

    pub fn triangle_count(&self) -> usize {
        self.parts
            .iter()
            .flat_map(|p| &p.faces)
            .filter_map(|f| f.mesh.as_ref())
            .map(|m| m.triangles.len())
            .sum()
    }
}
