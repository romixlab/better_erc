//! The 3D model's orthographic views over the datasheet page (VIEW-24): top, bottom, front, back, left and right,
//! each a filled silhouette with the model's visible edges, at the page's mm scale, to hold against the package
//! drawing.
//!
//! A view is worked out on the CPU, once per model, on the worker thread that reads it ([`project`]):
//!
//! - **the fill**: the model's triangles, projected; the blend pass draws them in every blend mode, normal too,
//!   since it mixes each pixel from the page's own and so overlapping triangles mix in only once;
//! - **the silhouette**: every projected triangle turned counter-clockwise and unioned by `i_overlay` (non-zero
//!   fill), its contours the outline drawn and the corners the measuring ruler snaps to;
//! - **the edges**: the B-rep edges projected, cut where a nearer face hides them. Hiding is decided on a depth
//!   raster of the triangles (the nearest surface per cell); an edge point is seen when no surface in its cell or
//!   the cells around it is nearer, so edges on a face's border and on the outline stay.
//!
//! **The view frame** is the page's: x right, y down, in mm, the origin where the footprint's origin projects
//! (on the board's top surface for the side views). The top view lies over the footprint as placed on the page, so
//! with the footprint's placement it sits on its pads; the side views look at the model from the front (from -y),
//! back, left (from -x) and right, the board's surface their horizontal line through the origin. The bottom view is
//! seen from below: x mirrored, as a package drawing's bottom view is.

use super::grab::Grab;
use super::*;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::simplify::SimplifyShape;
use ve_theme::Tone;
use ve_theme::props::*;
use ve_view3d::Mesh;
use ve_view3d::glam::Vec3;

/// What the model onto the footprint does, for its button's tooltip
pub(super) const ON_FOOTPRINT_HINT: &str = "the model onto the footprint as KiCad places it: at the footprint's origin \
     with its turn, seen from its side (the bottom for the bottom view), the model entry's offset and rotation applied";

/// Which way the model is looked at
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    #[default]
    Top,
    Bottom,
    Front,
    Back,
    Left,
    Right,
}

impl Side {
    pub const ALL: [Side; 6] = [
        Side::Top,
        Side::Bottom,
        Side::Front,
        Side::Back,
        Side::Left,
        Side::Right,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Side::Top => "Top",
            Side::Bottom => "Bottom",
            Side::Front => "Front",
            Side::Back => "Back",
            Side::Left => "Left",
            Side::Right => "Right",
        }
    }

    pub fn index(self) -> usize {
        Side::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// A point of the footprint's 3D frame (x right, y up, z up out of the board) in this view: where it lands
    /// (x right, y down, mm) and how near the viewer it is (larger: nearer)
    pub fn project(self, p: Vec3) -> ([f64; 2], f64) {
        let (x, y, z) = (p.x as f64, p.y as f64, p.z as f64);
        match self {
            Side::Top => ([x, -y], z),
            Side::Bottom => ([-x, -y], -z),
            Side::Front => ([x, -z], -y),
            Side::Back => ([-x, -z], y),
            Side::Left => ([-y, -z], -x),
            Side::Right => ([y, -z], x),
        }
    }
}

/// One view of the model, in the view frame (x right, y down, mm)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectedView {
    pub side: Side,
    /// The triangles, for the fill
    pub fill: Vec<[[f32; 2]; 3]>,
    /// The silhouette: its contours, each closed (the last point not repeated); outer ones and holes
    pub outline: Vec<Vec<[f64; 2]>>,
    /// The edges seen, as polylines
    pub edges: Vec<Vec<[f64; 2]>>,
    /// Where a measuring ruler snaps: the silhouette's corners and the seen edges' ends
    pub snaps: Vec<[f64; 2]>,
    /// The extent: lowest and highest corner; none for a view with nothing in it
    pub extent: Option<([f64; 2], [f64; 2])>,
}

/// How many raster cells the longer side of a view gets for hiding edges
const RASTER: usize = 768;
/// Below this a triangle's projected area (mm²) is edge-on and left out
const FLAT_AREA: f64 = 1e-9;
/// Nearer than this (mm) another surface must be to hide an edge
const HIDE_MM: f64 = 0.005;
/// Corners closer than this (mm) to one already kept are one snap target
const SNAP_MERGE_MM: f64 = 1e-4;

fn area2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// The nearest surface per cell over a view's extent
struct Depth {
    lo: [f64; 2],
    cell: f64,
    w: usize,
    h: usize,
    near: Vec<f64>,
}

impl Depth {
    fn new(lo: [f64; 2], hi: [f64; 2]) -> Depth {
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-6);
        let cell = span / RASTER as f64;
        let w = (((hi[0] - lo[0]) / cell).ceil() as usize).clamp(1, RASTER + 1);
        let h = (((hi[1] - lo[1]) / cell).ceil() as usize).clamp(1, RASTER + 1);
        Depth {
            lo,
            cell,
            w,
            h,
            near: vec![f64::NEG_INFINITY; w * h],
        }
    }

    fn cell_of(&self, p: [f64; 2]) -> (isize, isize) {
        (
            ((p[0] - self.lo[0]) / self.cell).floor() as isize,
            ((p[1] - self.lo[1]) / self.cell).floor() as isize,
        )
    }

    fn at(&self, i: isize, j: isize) -> f64 {
        if i < 0 || j < 0 || i as usize >= self.w || j as usize >= self.h {
            return f64::NEG_INFINITY;
        }
        self.near[j as usize * self.w + i as usize]
    }

    /// Draws a triangle: each cell whose centre it covers keeps the nearer of its depth there and what it had
    fn draw(&mut self, p: [[f64; 2]; 3], d: [f64; 3]) {
        let a = area2(p[0], p[1], p[2]);
        if a.abs() < FLAT_AREA {
            return;
        }
        let min = [
            p[0][0].min(p[1][0]).min(p[2][0]),
            p[0][1].min(p[1][1]).min(p[2][1]),
        ];
        let max = [
            p[0][0].max(p[1][0]).max(p[2][0]),
            p[0][1].max(p[1][1]).max(p[2][1]),
        ];
        let (i0, j0) = self.cell_of(min);
        let (i1, j1) = self.cell_of(max);
        for j in j0.max(0)..=j1.min(self.h as isize - 1) {
            for i in i0.max(0)..=i1.min(self.w as isize - 1) {
                let c = [
                    self.lo[0] + (i as f64 + 0.5) * self.cell,
                    self.lo[1] + (j as f64 + 0.5) * self.cell,
                ];
                let w0 = area2(p[1], p[2], c) / a;
                let w1 = area2(p[2], p[0], c) / a;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-9 || w1 < -1e-9 || w2 < -1e-9 {
                    continue;
                }
                let z = w0 * d[0] + w1 * d[1] + w2 * d[2];
                let k = j as usize * self.w + i as usize;
                if z > self.near[k] {
                    self.near[k] = z;
                }
            }
        }
    }

    /// Whether a point at depth `d` is seen: no surface in its cell or around it is nearer
    fn seen(&self, p: [f64; 2], d: f64) -> bool {
        let (i, j) = self.cell_of(p);
        let mut far = f64::INFINITY;
        for dj in -1..=1 {
            for di in -1..=1 {
                far = far.min(self.at(i + di, j + dj));
            }
        }
        d >= far - HIDE_MM
    }
}

/// The view of `meshes` (the model's parts, in the footprint's 3D frame) and their `edges` from `side`
pub fn project(meshes: &[&Mesh], edges: &[Vec<Vec3>], side: Side) -> ProjectedView {
    let mut tris: Vec<([[f64; 2]; 3], [f64; 3])> = Vec::new();
    for m in meshes {
        for t in &m.triangles {
            let Some(v) = t
                .iter()
                .map(|i| m.positions.get(*i as usize).map(|p| side.project(*p)))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let p = [v[0].0, v[1].0, v[2].0];
            if area2(p[0], p[1], p[2]).abs() < FLAT_AREA {
                continue;
            }
            tris.push((p, [v[0].1, v[1].1, v[2].1]));
        }
    }
    let mut out = ProjectedView {
        side,
        ..ProjectedView::default()
    };
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for (p, _) in &tris {
        for q in p {
            lo = [lo[0].min(q[0]), lo[1].min(q[1])];
            hi = [hi[0].max(q[0]), hi[1].max(q[1])];
        }
    }
    if tris.is_empty() {
        return out;
    }
    out.extent = Some((lo, hi));

    let mut depth = Depth::new(lo, hi);
    for (p, d) in &tris {
        depth.draw(*p, *d);
    }

    // The fill: every triangle; the blend pass mixes the page in once where they overlap
    out.fill = tris
        .iter()
        .map(|(p, _)| p.map(|q| [q[0] as f32, q[1] as f32]))
        .collect();

    // The silhouette: every triangle counter-clockwise (in a y-up sense), unioned
    let contours: Vec<Vec<[f64; 2]>> = tris
        .iter()
        .map(|(p, _)| {
            if area2(p[0], p[1], p[2]) > 0.0 {
                p.to_vec()
            } else {
                vec![p[0], p[2], p[1]]
            }
        })
        .collect();
    let shapes = contours.simplify_shape(FillRule::NonZero);
    for shape in shapes {
        for contour in shape {
            if contour.len() >= 3 {
                out.outline.push(contour);
            }
        }
    }

    // The edges seen: each polyline sampled finer than a cell, the runs of seen points kept
    let step = depth.cell / 2.0;
    for poly in edges {
        let pts: Vec<([f64; 2], f64)> = poly.iter().map(|p| side.project(*p)).collect();
        let mut run: Vec<[f64; 2]> = Vec::new();
        let flush = |run: &mut Vec<[f64; 2]>, out: &mut ProjectedView| {
            // An edge along the line of sight is a point: nothing to draw
            let long = run
                .windows(2)
                .any(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]) > 1e-9);
            if run.len() >= 2 && long {
                out.edges.push(straightened(std::mem::take(run)));
            } else {
                run.clear();
            }
        };
        for (k, w) in pts.windows(2).enumerate() {
            let ((a, da), (b, db)) = (w[0], w[1]);
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            let n = ((len / step).ceil() as usize).max(1);
            for s in 0..=n {
                if s == 0 && k > 0 {
                    continue;
                }
                let t = s as f64 / n as f64;
                let q = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
                if depth.seen(q, da + (db - da) * t) {
                    run.push(q);
                } else {
                    flush(&mut run, &mut out);
                }
            }
        }
        flush(&mut run, &mut out);
    }

    // Snap targets: the silhouette's corners, the seen edges' ends
    let mut snaps: Vec<[f64; 2]> = Vec::new();
    let mut add = |p: [f64; 2]| {
        if !snaps
            .iter()
            .any(|s| (s[0] - p[0]).hypot(s[1] - p[1]) < SNAP_MERGE_MM)
        {
            snaps.push(p);
        }
    };
    for c in &out.outline {
        for p in c {
            add(*p);
        }
    }
    for e in &out.edges {
        if let (Some(a), Some(b)) = (e.first(), e.last()) {
            add(*a);
            add(*b);
        }
    }
    out.snaps = snaps;
    out
}

/// A run of samples without the points between two others on a straight line
fn straightened(run: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(run.len());
    for p in run {
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let len = (p[0] - a[0]).hypot(p[1] - a[1]);
            let ahead = (b[0] - a[0]) * (p[0] - a[0]) + (b[1] - a[1]) * (p[1] - a[1]) > 0.0;
            if len > 0.0 && ahead && area2(a, b, p).abs() / len < 1e-9 {
                *out.last_mut().unwrap() = p;
                continue;
            }
        }
        out.push(p);
    }
    out
}

/// All six views
pub fn project_all(meshes: &[&Mesh], edges: &[Vec<Vec3>]) -> Vec<ProjectedView> {
    Side::ALL
        .iter()
        .map(|s| project(meshes, edges, *s))
        .collect()
}

/// How the model is shown over the page
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelLook {
    /// The six flat views of VIEW-24, each shown on its own
    #[default]
    Silhouettes,
    /// The shaded model, turned by the SpaceMouse or Ctrl+drag, snapping to a side (VIEW-27)
    Solid,
}

impl ModelLook {
    pub fn name(self) -> &'static str {
        match self {
            ModelLook::Silhouettes => "Silhouettes",
            ModelLook::Solid => "3D",
        }
    }
}

/// What the keys, the puck and the placement controls act on, as one of two: the footprint or the model
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moves {
    Footprint,
    /// The model: a silhouette view in Silhouettes, the shaded model in 3D
    Model,
}

/// How many projections the page holds at most
pub(super) const VIEWS_MAX: usize = 24;

/// One projection of the model over the page: which side it shows and where it lies
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelView {
    pub side: Side,
    pub place: Placement,
}

/// The model views' state in the tab, saved with it
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelViews {
    /// The model's projections over the page, each placed on its own; a side may be shown more than once. They are
    /// drawn in both looks, under the 3D model in 3D
    pub views: Vec<ModelView>,
    /// The side the model view key showed last, and cycles on from; the view the model stands for in Silhouettes
    /// before the pointer was over one
    pub view: Side,
    /// Silhouettes or the shaded model
    pub look: ModelLook,
    /// The shaded model's placement and turn (VIEW-27)
    pub solid: super::solid::Solid,
    /// How opaque the views' fill and the 3D model are; none: the settings' model opacity
    pub opacity: Option<f32>,
    /// How the views blend with the page; none: the settings' blend mode
    pub blend: Option<BlendMode>,
    pub blink: bool,
}

impl ModelViews {
    pub fn any_shown(&self) -> bool {
        !self.views.is_empty()
    }

    /// The first projection of `side`
    pub fn of(&self, side: Side) -> Option<usize> {
        self.views.iter().position(|v| v.side == side)
    }

    /// Whether `side` is shown at least once
    pub fn shows(&self, side: Side) -> bool {
        self.of(side).is_some()
    }
}

impl FootprintReview {
    /// The projection the keys and the puck act on, when it is one ([`FootprintReview::aimed`])
    pub(super) fn moving_view(&self) -> Option<usize> {
        match self.aimed() {
            Grab::View(i) => Some(i),
            _ => None,
        }
    }

    /// The keys and the puck act on the shaded model
    pub(super) fn moving_solid(&self) -> bool {
        self.aimed() == Grab::Solid
    }

    /// Whether the keys and the puck act on the footprint or the model
    pub(super) fn moves_now(&self) -> Moves {
        match self.aimed() {
            Grab::Footprint | Grab::Page => Moves::Footprint,
            Grab::View(_) | Grab::Solid => Moves::Model,
        }
    }

    /// The placement the mouse, the arrow keys, turn and mirror move: the projection's or the shaded model's when
    /// the model moves, else the footprint's
    pub(super) fn moving_mut(&mut self) -> &mut Placement {
        if self.moving_solid() {
            let first = self.place.clone();
            return self.model_views.solid.place.get_or_insert(first);
        }
        match self.moving_view() {
            Some(i) if i < self.model_views.views.len() => &mut self.model_views.views[i].place,
            _ => &mut self.place,
        }
    }

    /// Where a projection goes when first shown: the top view on the footprint (so it lies on its pads), the
    /// others unturned, into the middle of the view once the model is read
    fn first_place(&self, side: Side) -> Placement {
        match side {
            Side::Top => self.place.clone(),
            _ => Placement {
                origin: self.place.origin,
                rotation: 0.0,
                mirror: false,
            },
        }
    }

    /// What the placement controls move, in words
    pub(super) fn moving_name(&self) -> String {
        if self.moving_solid() {
            return "the 3D model".into();
        }
        match self
            .moving_view()
            .and_then(|i| self.model_views.views.get(i))
        {
            Some(v) => format!("the model's {} view", v.side.name().to_lowercase()),
            None => "the footprint".into(),
        }
    }

    /// A projection of `side` added; its middle goes to page point `at` once the model is read, the canvas'
    /// middle without one. None when the page holds [`VIEWS_MAX`] already
    pub(super) fn add_projection(&mut self, side: Side, at: Option<[f32; 2]>) -> Option<usize> {
        if self.model_views.views.len() >= VIEWS_MAX {
            return None;
        }
        let place = self.first_place(side);
        self.model_views.views.push(ModelView { side, place });
        let i = self.model_views.views.len() - 1;
        let target = at.unwrap_or_else(|| self.to_page(self.rt.canvas, self.rt.canvas.center()));
        self.rt.center_model_view = Some((i, target));
        self.usage(&format!("model views add {}", side.name().to_lowercase()));
        Some(i)
    }

    /// The first projection of `side`, added when there is none: the top view on the footprint, the others into
    /// the middle of the view
    pub(super) fn show_model_view(&mut self, side: Side) -> Option<usize> {
        if let Some(i) = self.model_views.of(side) {
            return Some(i);
        }
        if self.model_views.views.len() >= VIEWS_MAX {
            return None;
        }
        if side == Side::Top {
            let place = self.first_place(side);
            self.model_views.views.push(ModelView { side, place });
            return Some(self.model_views.views.len() - 1);
        }
        self.add_projection(side, None)
    }

    /// Projection `i` taken off the page; what pointed at the ones after it follows them down
    pub(super) fn remove_projection(&mut self, i: usize) {
        if i >= self.model_views.views.len() {
            return;
        }
        let side = self.model_views.views.remove(i).side;
        self.rt.model_menu = Default::default();
        let shift = |g: Grab| match g {
            Grab::View(j) if j == i => None,
            Grab::View(j) if j > i => Some(Grab::View(j - 1)),
            g => Some(g),
        };
        self.rt.aim = shift(self.rt.aim).unwrap_or(Grab::Footprint);
        self.rt.grab = self.rt.grab.and_then(shift);
        self.rt.hover_grab = self.rt.hover_grab.and_then(shift);
        self.rt.center_model_view = match self.rt.center_model_view {
            Some((j, _)) if j == i => None,
            Some((j, t)) if j > i => Some((j - 1, t)),
            c => c,
        };
        self.usage(&format!(
            "model views remove {}",
            side.name().to_lowercase()
        ));
    }

    /// Every projection of `side` taken off the page
    fn remove_side(&mut self, side: Side) {
        while let Some(i) = self.model_views.of(side) {
            self.remove_projection(i);
        }
    }

    /// The model view key. Silhouettes: the next side shown (the first press shows the side last shown, when the
    /// keys act on the footprint) and aimed at, so the keys act on it until the pointer is over something else. 3D:
    /// the shaded model turns to the next side, eased
    pub(super) fn cycle_model_view(&mut self, now: f64) {
        let next = Side::ALL[(self.model_views.view.index() + 1) % Side::ALL.len()];
        match self.model_views.look {
            ModelLook::Silhouettes => {
                if self.moves_now() == Moves::Model {
                    self.model_views.view = next;
                }
                self.aim_view(self.model_views.view);
            }
            ModelLook::Solid => {
                let side = match self.solid_side() {
                    Some((s, _)) => Side::ALL[(s.index() + 1) % Side::ALL.len()],
                    None => self.model_views.view,
                };
                self.model_views.view = side;
                self.turn_solid_to(side, now);
                self.usage(&format!("model 3d side {}", side.name().to_lowercase()));
            }
        }
    }

    /// The first projection of `side` shown and aimed at: the keys and the puck act on it until the pointer is
    /// over something else
    pub(super) fn aim_view(&mut self, side: Side) {
        self.model_views.view = side;
        if let Some(i) = self.show_model_view(side) {
            self.rt.aim = Grab::View(i);
        }
        self.usage(&format!("model view {}", side.name().to_lowercase()));
    }

    /// Silhouettes or 3D; the 3D model's first placement is the footprint's, seen from the top, so it lies on its
    /// pads
    pub(super) fn set_look(&mut self, look: ModelLook) {
        self.model_views.look = look;
        if look == ModelLook::Solid && self.model_views.solid.place.is_none() {
            self.model_views.solid.place = Some(self.place.clone());
        }
        // The keys on the 3D model go on to the view of the side picked
        if look == ModelLook::Silhouettes
            && self.rt.aim == Grab::Solid
            && let Some(i) = self.show_model_view(self.model_views.view)
        {
            self.rt.aim = Grab::View(i);
        }
        self.usage(&format!("model look {}", look.name().to_lowercase()));
    }

    /// The look key
    pub(super) fn toggle_look(&mut self) {
        self.set_look(match self.model_views.look {
            ModelLook::Silhouettes => ModelLook::Solid,
            ModelLook::Solid => ModelLook::Silhouettes,
        });
    }

    /// The model view `side` as worked out from the model read, when it has something in it
    pub(super) fn projected(&self, side: Side) -> Option<&ProjectedView> {
        let v = self.rt.model.loaded.as_ref()?.views.get(side.index())?;
        v.extent.is_some().then_some(v)
    }

    /// The projections, each with its index, placement and what it draws
    fn shown_views(&self) -> impl Iterator<Item = (usize, &ModelView, &ProjectedView)> {
        self.model_views
            .views
            .iter()
            .enumerate()
            .filter_map(|(i, v)| Some((i, v, self.projected(v.side)?)))
    }

    /// Each frame on the page: the model read while a view is wanted, a projection just added moved to where it
    /// was asked for once it can be
    pub(super) fn model_views_frame(&mut self, ctx: &egui::Context, canvas: Rect) {
        let wanted = match self.model_views.look {
            ModelLook::Silhouettes => self.model_views.any_shown(),
            ModelLook::Solid => true,
        };
        if !wanted {
            return;
        }
        if self.model_blocker().is_some() {
            return;
        }
        self.model_wanted(ctx);
        self.model_poll(ctx);
        if let Some((i, target)) = self.rt.center_model_view
            && canvas.width() > 1.0
            && let Some(v) = self.model_views.views.get(i)
            && let Some((lo, hi)) = self.projected(v.side).and_then(|p| p.extent)
        {
            let mid = self.placed(&v.place, [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]);
            let p = &mut self.model_views.views[i].place;
            p.origin = [
                p.origin[0] + target[0] - mid[0],
                p.origin[1] + target[1] - mid[1],
            ];
            self.rt.center_model_view = None;
        }
    }

    /// Where a measuring ruler snaps on the model views shown: their outlines' corners and their edges' ends, page
    /// points
    /// and the shaded model's silhouette corners while it sits on a side
    pub(super) fn model_view_snaps(&self) -> impl Iterator<Item = ([f32; 2], SnapKind)> + '_ {
        self.shown_views()
            .flat_map(move |(_, view, v)| {
                v.snaps
                    .iter()
                    .map(move |p| (self.placed(&view.place, *p), SnapKind::ModelCorner))
            })
            .chain(self.solid_snaps().map(|p| (p, SnapKind::ModelCorner)))
    }

    pub(super) fn model_views_opacity(&self) -> f32 {
        self.model_views
            .opacity
            .unwrap_or(self.rt.settings.model_opacity)
            .clamp(0.0, 1.0)
    }

    /// The model's opacity, apart from the footprint's: in the View section and the Model views section (one value)
    pub(super) fn model_opacity_row(&mut self, ui: &mut Ui) {
        let mut opacity = self.model_views_opacity();
        PropertyRow::new("Model opacity")
            .tooltip(
                "The 3D model and its silhouettes, apart from the footprint: 0 only their outline and edges, 1 the \
                 fill hides the page under it",
            )
            .show(ui, |ui| {
                if ui
                    .add(
                        ScalarField::new(&mut opacity)
                            .range(0.0..=1.0)
                            .speed(0.005)
                            .decimals(2)
                            .tooltip("How much of the page the model covers"),
                    )
                    .changed()
                {
                    self.model_views.opacity = Some(opacity);
                }
            });
    }

    pub(super) fn model_views_blend(&self) -> BlendMode {
        self.model_views.blend.unwrap_or(self.rt.settings.blend)
    }

    /// Draws the model views shown: the fill by the blend pass over `page` in every blend mode (overlapping
    /// triangles mix in once), by egui's painter without the pass; the outlines, edges, names and origins by the
    /// painter on top. The footprint is drawn after them, so where they overlap it is on top.
    pub(super) fn draw_model_views(
        &self,
        painter: &egui::Painter,
        canvas: Rect,
        page: Option<(Rect, blend::PageImage)>,
        id: Id,
    ) {
        let a = self.model_views_opacity();
        let colors = self.layer_colors();
        let [r, g, b] = colors.model.0;
        let fill = Color32::from_rgb(r, g, b);
        let [r, g, b] = colors.model_edges.0;
        let line = Color32::from_rgb(r, g, b);
        let mode = self.model_views_blend();
        let gpu = blend::available(painter.ctx()) && page.is_some();
        let mut mesh = egui::Mesh::default();
        let mut lines = Vec::new();
        let mut labels = Vec::new();
        for (i, view, v) in self.shown_views() {
            let (side, place) = (view.side, &view.place);
            let screen = |p: [f64; 2]| self.to_screen(canvas, self.placed(place, p));
            let c = if gpu { fill } else { fill.gamma_multiply(a) };
            for t in &v.fill {
                let base = mesh.vertices.len() as u32;
                for q in t {
                    mesh.colored_vertex(screen([q[0] as f64, q[1] as f64]), c);
                }
                mesh.add_triangle(base, base + 1, base + 2);
            }
            let current = self.moving_view() == Some(i);
            let stroke = Stroke::new(
                if current { 2.5 } else { 1.5 },
                line.gamma_multiply(a.max(0.6)),
            );
            for contour in &v.outline {
                lines.push(Shape::closed_line(
                    contour.iter().map(|p| screen(*p)).collect(),
                    stroke,
                ));
            }
            for e in &v.edges {
                lines.push(Shape::line(
                    e.iter().map(|p| screen(*p)).collect(),
                    Stroke::new(1.0, line.gamma_multiply(a.max(0.6))),
                ));
            }
            // The view's name over the top left corner of its box on screen, and its origin
            if let Some((lo, hi)) = v.extent {
                let corners = [
                    [lo[0], lo[1]],
                    [hi[0], lo[1]],
                    [lo[0], hi[1]],
                    [hi[0], hi[1]],
                ]
                .map(screen);
                let top_left = label_corner(&corners);
                let text = if current {
                    format!("{} view · moves", side.name())
                } else {
                    format!("{} view", side.name())
                };
                labels.push((top_left, text));
            }
            let o = self.to_screen(canvas, place.origin);
            let s = Stroke::new(1.0, line);
            lines.push(Shape::line_segment(
                [o - vec2(6.0, 0.0), o + vec2(6.0, 0.0)],
                s,
            ));
            lines.push(Shape::line_segment(
                [o - vec2(0.0, 6.0), o + vec2(0.0, 6.0)],
                s,
            ));
        }
        if !mesh.is_empty() {
            match (gpu, page) {
                (true, Some(page)) => {
                    blend_pass(painter, canvas, page, id.with("model views"), mesh, a, mode)
                }
                _ => {
                    painter.add(Shape::mesh(mesh));
                }
            }
        }
        painter.extend(lines);
        for (at, text) in labels {
            painter.text(
                at,
                Align2::LEFT_BOTTOM,
                text,
                FontId::proportional(12.0),
                line,
            );
        }
    }

    /// The side panel's section on the model's views over the page
    pub(super) fn model_views_section(&mut self, ui: &mut Ui) {
        let summary = match self.model_views.look {
            ModelLook::Silhouettes => {
                if self.model_views.any_shown() {
                    format!("Silhouettes {}", self.views_named())
                } else {
                    "Silhouettes, none shown".to_string()
                }
            }
            ModelLook::Solid if self.model_views.any_shown() => {
                format!("3D, {}; {}", self.solid_side_name(), self.views_named())
            }
            ModelLook::Solid => format!("3D, {}", self.solid_side_name()),
        };
        let summary = format!("{summary} · keys on {}", self.moving_name());
        SubPane::new("Model views")
            .id_salt("fp model views")
            .summary(summary)
            .tooltip(
                "The 3D model over the page at its scale: its six flat views (Silhouettes) or the shaded model \
                 turned by the SpaceMouse (3D)",
            )
            .default_open(false)
            .show(ui, |ui| self.model_views_controls(ui));
    }

    /// The switch Silhouettes or 3D, with its key; the mode bar shows the same
    pub(super) fn model_switches(&mut self, ui: &mut Ui, fill: bool) {
        let k = self.keys();
        let mut look = self.model_views.look;
        let mut seg = Segmented::new(&mut look).id_salt("model look");
        if fill {
            seg = seg.fill_width();
        }
        seg.item(
            ModelLook::Silhouettes,
            "Silhouettes",
            format!(
                "The model's six flat views, each placed on its own; {} switches",
                key_name(k.model_look)
            ),
        )
        .item(
            ModelLook::Solid,
            "3D",
            format!(
                "The shaded model at the page's scale, turned by the SpaceMouse or Ctrl+drag, landing on the nearest \
                 side when let go; {} switches",
                key_name(k.model_look)
            ),
        )
        .show(ui);
        if look != self.model_views.look {
            self.set_look(look);
        }
    }

    fn model_views_controls(&mut self, ui: &mut Ui) {
        let k = self.keys();
        if let Some(why) = self.model_blocker() {
            notice(ui, Tone::Warn, format!("No model views: {why}"));
            return;
        }
        if self.rt.model.loading() {
            notice(ui, Tone::Neutral, "Reading the 3D model…");
        } else if let Some(l) = &self.rt.model.loaded
            && l.shown == 0
        {
            notice(
                ui,
                Tone::Warn,
                format!("No 3D model shown: {}", l.notes.join("; ")),
            );
        }
        Group::new("Shown as")
            .tooltip(format!("Silhouettes or 3D ({})", key_name(k.model_look)))
            .show(ui, |ui| {
                ui.vertical(|ui| self.model_switches(ui, true));
            });
        self.projection_controls(ui);
        if self.model_views.look == ModelLook::Solid {
            self.solid_controls(ui);
        }
        let gpu = blend::available(ui.ctx());
        let mut mode = self.model_views_blend();
        Group::new("Blend")
            .tooltip("How the views' fill mixes with the page under it")
            .show(ui, |ui| {
                let mut seg = Segmented::new(&mut mode)
                    .id_salt("model views blend")
                    .fill_width();
                for m in BlendMode::ALL {
                    let why = (m != BlendMode::Normal && !gpu).then_some(
                        "needs the app's wgpu renderer, which this window does not have",
                    );
                    seg = seg.item_if(m, m.name(), blend_hint(m), why);
                }
                seg.show(ui);
            });
        if mode != self.model_views_blend() {
            self.model_views.blend = Some(mode);
            self.usage(&format!("model views blend {}", mode.name().to_lowercase()));
        }
        self.model_opacity_row(ui);
        ui.horizontal_wrapped(|ui| {
            ui.add(
                ToggleChip::new(&mut self.model_views.blink, "Blink").tooltip(format!(
                    "The views on and off; {} blinks what moves",
                    key_name(k.blink)
                )),
            );
            let centre = match self.model_views.look {
                ModelLook::Silhouettes => "The view picked into the middle of the canvas",
                ModelLook::Solid => "The 3D model into the middle of the canvas",
            };
            if ui.button("Centre in view").on_hover_text(centre).clicked() {
                self.centre_model();
            }
            if ui
                .button(format!(
                    "On the footprint · {}",
                    key_name(k.model_on_footprint)
                ))
                .on_hover_text(ON_FOOTPRINT_HINT)
                .clicked()
            {
                self.model_on_footprint(ui.input(|i| i.time));
            }
        });
        hint(
            ui,
            format!(
                "At the page's scale; a view's corners and edge ends, and the 3D model's outline corners while it \
                 sits on a side, are where Measure footprint ({}) snaps",
                key_name(k.measure_footprint)
            ),
        );
    }

    /// The model onto the footprint as KiCad places it: at the footprint's origin with its turn, seen from the side
    /// it is on (the top; the bottom for a footprint in bottom view, as KiCad puts a back-side footprint's model
    /// under the board). The model's parts already carry the model entry's offset, scale and rotation, so the
    /// footprint's placement is all that is left. Silhouettes: that side's view, shown and picked; 3D: the shaded
    /// model, turned onto that side
    pub(super) fn model_on_footprint(&mut self, now: f64) {
        let side = if self.place.mirror {
            Side::Bottom
        } else {
            Side::Top
        };
        let on = Placement {
            origin: self.place.origin,
            rotation: self.place.rotation,
            mirror: false,
        };
        self.model_views.view = side;
        match self.model_views.look {
            ModelLook::Silhouettes => {
                if let Some(i) = self.show_model_view(side) {
                    self.model_views.views[i].place = on;
                    if self.rt.center_model_view.is_some_and(|(j, _)| j == i) {
                        self.rt.center_model_view = None;
                    }
                }
            }
            ModelLook::Solid => {
                self.model_views.solid.place = Some(on);
                self.turn_solid_to(side, now);
            }
        }
        self.usage("model on footprint");
    }

    /// The model picked into the middle of the canvas: the view picked, or the 3D model
    pub(super) fn centre_model(&mut self) {
        match self.model_views.look {
            ModelLook::Silhouettes => {
                let i = match self.moving_view() {
                    Some(i) => Some(i),
                    None => self.show_model_view(self.model_views.view),
                };
                if let Some(i) = i {
                    let mid = self.to_page(self.rt.canvas, self.rt.canvas.center());
                    self.rt.center_model_view = Some((i, mid));
                }
            }
            ModelLook::Solid => self.rt.solid.centre = true,
        }
    }

    /// The projections in words: each side, with how often it is shown when more than once
    fn views_named(&self) -> String {
        Side::ALL
            .iter()
            .filter_map(|s| {
                let n = self
                    .model_views
                    .views
                    .iter()
                    .filter(|v| v.side == *s)
                    .count();
                match n {
                    0 => None,
                    1 => Some(s.name().to_owned()),
                    n => Some(format!("{} x{n}", s.name())),
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Which sides are projected: a chip per side adds its view or takes every one of it off; a right click on
    /// the model adds more of one
    fn projection_controls(&mut self, ui: &mut Ui) {
        let full = self.model_views.views.len() >= VIEWS_MAX;
        Group::new("Projections")
            .tooltip(
                "The model's flat views over the page, each placed on its own; a right click on the model adds \
                 one at the pointer, on a view takes it off",
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for side in Side::ALL {
                        let was = self.model_views.shows(side);
                        let mut on = was;
                        let tip = format!("The model seen from the {}", side.name().to_lowercase());
                        let why = (full && !was).then(|| {
                            format!("the page holds {VIEWS_MAX} views, the most it takes: take one off first")
                        });
                        let chip = ToggleChip::new(&mut on, side.name())
                            .tooltip(tip)
                            .disabled_because(why);
                        if !ui.add(chip).changed() || on == was {
                            continue;
                        }
                        if on {
                            self.show_model_view(side);
                        } else {
                            self.remove_side(side);
                        }
                    }
                });
            });
    }
}

/// Where a model's label goes (VIEW-36): just over the top left corner of the box on screen around `points` (what of
/// the model is drawn), so it stays beside the model however it is placed, turned and zoomed
pub(super) fn label_corner(points: &[Pos2]) -> Pos2 {
    Rect::from_points(points).left_top() - vec2(0.0, 2.0)
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::model3d::tests::two_pin;
    use super::super::tests::{datasheet, harness};
    use super::*;
    use egui_kittest::Harness;
    use ve_view3d::shapes::cuboid;

    type H = Harness<'static, FootprintReview>;

    /// The side of the projection the keys act on
    pub(in super::super) fn moving_side(st: &FootprintReview) -> Option<Side> {
        st.moving_view().map(|i| st.model_views.views[i].side)
    }

    /// The first projection of `side`'s placement
    pub(in super::super) fn place_of(st: &FootprintReview, side: Side) -> Option<Placement> {
        st.model_views
            .of(side)
            .map(|i| st.model_views.views[i].place.clone())
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// The box's 12 edges
    fn box_edges(lo: Vec3, hi: Vec3) -> Vec<Vec<Vec3>> {
        let c = |i: usize| {
            Vec3::new(
                if i & 1 == 0 { lo.x } else { hi.x },
                if i & 2 == 0 { lo.y } else { hi.y },
                if i & 4 == 0 { lo.z } else { hi.z },
            )
        };
        let mut e = Vec::new();
        for a in 0..8usize {
            for bit in [1, 2, 4] {
                if a & bit == 0 {
                    e.push(vec![c(a), c(a | bit)]);
                }
            }
        }
        e
    }

    fn extent_of(v: &ProjectedView) -> ([f64; 2], [f64; 2]) {
        v.extent.expect("something in the view")
    }

    #[test]
    fn a_box_from_each_side() {
        // 4 wide (x), 2 deep (y), 1 high (z), standing on the board off the origin
        let (lo, hi) = (Vec3::new(1.0, 0.0, 0.0), Vec3::new(5.0, 2.0, 1.0));
        let m = cuboid(lo, hi, egui::Color32::WHITE);
        let views = project_all(&[&m], &box_edges(lo, hi));
        let want: [(Side, [f64; 4]); 6] = [
            (Side::Top, [1.0, -2.0, 5.0, 0.0]),
            (Side::Bottom, [-5.0, -2.0, -1.0, 0.0]),
            (Side::Front, [1.0, -1.0, 5.0, 0.0]),
            (Side::Back, [-5.0, -1.0, -1.0, 0.0]),
            (Side::Left, [-2.0, -1.0, 0.0, 0.0]),
            (Side::Right, [0.0, -1.0, 2.0, 0.0]),
        ];
        for (v, (side, [x0, y0, x1, y1])) in views.iter().zip(want) {
            assert_eq!(v.side, side);
            let (l, h) = extent_of(v);
            assert!(
                close(l[0], x0) && close(l[1], y0) && close(h[0], x1) && close(h[1], y1),
                "{side:?}: {l:?} {h:?}"
            );
            // The silhouette: one rectangle, its four corners the snap targets with the edges' ends
            assert_eq!(v.outline.len(), 1, "{side:?}: {:?}", v.outline);
            let mut corners = v.outline[0].clone();
            corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
            assert_eq!(
                corners,
                vec![[x0, y0], [x0, y1], [x1, y0], [x1, y1]],
                "{side:?}"
            );
            for c in &corners {
                assert!(
                    v.snaps
                        .iter()
                        .any(|s| close(s[0], c[0]) && close(s[1], c[1]))
                );
            }
            // Of a box, the four edges around the face seen; the others are on the outline or hidden behind it
            assert_eq!(v.edges.len(), 4 + 4, "{side:?}: {:?}", v.edges);
            // The fill: the face toward the viewer, two triangles
            assert!(!v.fill.is_empty(), "{side:?}");
        }
    }

    #[test]
    fn hidden_edges_are_cut() {
        // A small box on top of a big one: seen from below, the small one's edges are behind the big one
        let (blo, bhi) = (Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 4.0, 1.0));
        let (slo, shi) = (Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, 3.0, 2.0));
        let (big, top) = (
            cuboid(blo, bhi, egui::Color32::WHITE),
            cuboid(slo, shi, egui::Color32::WHITE),
        );
        let meshes = [&big, &top];
        let small = box_edges(slo, shi);
        let below = project(&meshes, &small, Side::Bottom);
        assert!(below.edges.is_empty(), "{:?}", below.edges);
        // From above all of the small box's top edges and its upright ones (as points, dropped) are seen
        let above = project(&meshes, &small, Side::Top);
        let mut len = 0.0;
        for e in &above.edges {
            for w in e.windows(2) {
                len += (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
            }
        }
        // The top square's 8 mm, and the bottom square's 8 mm right under it (on the same lines)
        assert!((len - 16.0).abs() < 0.05, "{len}");
        // From the front: the small box's front face edges seen (6 mm), of its back face's the three on its outline
        // (4 mm) but not the one behind the front face, over the big box
        let front = project(&meshes, &small, Side::Front);
        let front_len: f64 = front
            .edges
            .iter()
            .flat_map(|e| {
                e.windows(2)
                    .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
            })
            .sum();
        assert!((front_len - 10.0).abs() < 0.05, "{front_len}");
        // The two boxes' silhouette from the front: one contour of eight corners
        let both = project(&meshes, &[], Side::Front);
        assert_eq!(both.outline.len(), 1);
        assert_eq!(both.outline[0].len(), 8, "{:?}", both.outline);
    }

    #[test]
    fn nothing_to_project() {
        let v = project(&[], &[], Side::Left);
        assert_eq!(v.extent, None);
        assert!(v.outline.is_empty() && v.fill.is_empty() && v.edges.is_empty());
    }

    fn key(h: &mut H, k: egui::Key) {
        h.hover_at(h.state().rt.canvas.center());
        h.run();
        h.key_press(k);
        h.run();
        h.run();
    }

    fn screen(h: &H, p: [f32; 2]) -> egui::Pos2 {
        let st = h.state();
        st.to_screen(st.rt.canvas, p)
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
    }

    /// The model view key shows the top view on the footprint and moves it, then each view in turn, wrapping; the
    /// moves key the footprint again; a drag and the turn key move only what moves; Measure footprint snaps to a view's corner
    #[test]
    fn model_views_over_the_page() {
        let dir = std::env::temp_dir().join(format!("berc-fp-modelviews-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let pdf = datasheet(&dir);
        let mut h = harness("fp model views");
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, &fp);
        h.state_mut().open_datasheet(&ctx, &pdf);
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        let st = h.state_mut();
        st.scale = Some(Scale {
            mm_per_point: 0.02,
            page: st.page,
            mm: 2.0,
            note: None,
        });
        st.zoom = 2.0;
        st.pan = [0.0, 0.0];
        st.place.origin = [150.0, 300.0];
        h.run();
        assert!(!h.state().model_views.any_shown());

        // Top: on the footprint, read and projected; the body 4 x 2 mm
        key(&mut h, egui::Key::K);
        let st = h.state();
        assert_eq!(moving_side(st), Some(Side::Top));
        assert!(st.model_views.shows(Side::Top));
        assert_eq!(place_of(st, Side::Top).as_ref(), Some(&st.place));
        let top = st
            .projected(Side::Top)
            .expect("the model read and projected");
        let (lo, hi) = top.extent.unwrap();
        assert!(
            (lo[0] + 2.0).abs() < 1e-6 && (hi[0] - 2.0).abs() < 1e-6,
            "{lo:?} {hi:?}"
        );
        assert!(
            (lo[1] + 1.0).abs() < 1e-6 && (hi[1] - 1.0).abs() < 1e-6,
            "{lo:?} {hi:?}"
        );
        assert!(st.help_line().contains("the model's top view"));

        // Measure footprint snaps to the top view's corner (2, 1) mm: 100, 50 points from the origin
        key(&mut h, egui::Key::G);
        assert_eq!(h.state().tool, Tool::MeasureFootprint);
        let corner = [250.0, 350.0];
        h.hover_at(screen(&h, [corner[0] - 1.5, corner[1] + 1.0]));
        h.run();
        let snap = h.state().rt.snap.clone().expect("a snap at the corner");
        assert_eq!(snap.kind, SnapKind::ModelCorner);
        assert!(near(snap.at, corner), "{:?}", snap.at);
        key(&mut h, egui::Key::Escape);
        assert_eq!(h.state().tool, Tool::Move);

        // Bottom: shown beside, moved into the middle of the view; a drag and the turn key move it alone
        key(&mut h, egui::Key::K);
        let st = h.state();
        assert_eq!(moving_side(st), Some(Side::Bottom));
        assert!(st.model_views.shows(Side::Top) && st.model_views.shows(Side::Bottom));
        assert_eq!(st.rt.center_model_view, None);
        let mid = st.to_page(st.rt.canvas, st.rt.canvas.center());
        let bottom = place_of(st, Side::Bottom).unwrap();
        assert!(near(bottom.origin, mid), "{:?} {mid:?}", bottom.origin);
        let fp_before = st.place.clone();
        let a = screen(&h, bottom.origin);
        let b = a + vec2(40.0, 20.0);
        h.hover_at(a);
        h.run();
        h.drag_at(a);
        h.run();
        for i in 1..=4 {
            h.hover_at(a + (b - a) * (i as f32 / 4.0));
            h.run();
        }
        h.drop_at(b);
        h.run();
        let moved = place_of(h.state(), Side::Bottom).unwrap();
        assert!(
            near(
                moved.origin,
                [bottom.origin[0] + 20.0, bottom.origin[1] + 10.0]
            ),
            "{:?}",
            moved.origin
        );
        key(&mut h, egui::Key::T);
        let st = h.state();
        assert_eq!(place_of(st, Side::Bottom).unwrap().rotation, 90.0);
        assert_eq!(st.place, fp_before);

        // Front: x across, the height up from the board's line
        key(&mut h, egui::Key::K);
        let (lo, hi) = h.state().projected(Side::Front).unwrap().extent.unwrap();
        assert!(
            (lo[1] + 1.2).abs() < 1e-6 && hi[1].abs() < 1e-6,
            "{lo:?} {hi:?}"
        );

        // Back, left, right; the model view key then wraps to the top view, which the keys then act on; the
        // model hidden (J), they act on the footprint again: a drag moves it
        for _ in 0..3 {
            key(&mut h, egui::Key::K);
        }
        assert_eq!(moving_side(h.state()), Some(Side::Right));
        key(&mut h, egui::Key::K);
        assert_eq!(moving_side(h.state()), Some(Side::Top));
        assert_eq!(h.state().model_views.views.len(), 6);
        key(&mut h, egui::Key::J);
        assert_eq!(moving_side(h.state()), None);
        assert_eq!(h.state().moves_now(), Moves::Footprint);
        let o = h.state().place.origin;
        let a = screen(&h, o);
        h.hover_at(a);
        h.run();
        h.drag_at(a);
        h.run();
        h.hover_at(a + vec2(10.0, 0.0));
        h.run();
        h.drop_at(a + vec2(10.0, 0.0));
        h.run();
        assert!(
            near(h.state().place.origin, [o[0] + 5.0, o[1]]),
            "{:?}",
            h.state().place.origin
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A real footprint and datasheet drawn on the GPU with the model's views placed over the package drawing:
    /// `BERC_MODEL_VIEWS` holds `FOOTPRINT;PDF;PAGE;MM_PER_POINT;ZOOM;X,Y` (the page point at the canvas' top left)
    /// and then `fp:X,Y,DEG` and `SIDE:X,Y,DEG[,m]` for the footprint and each view (page points, `m`: mirrored),
    /// `3d:X,Y,DEG[,m]` for the shaded model (VIEW-27), `turn:SIDE` for the side it shows and `blend:MODE,OPACITY`
    /// (multiply at 0.45 without it);
    /// `BERC_MODEL_VIEWS_PNG` names the picture to write. Skips without them.
    #[test]
    fn a_real_drawing_with_the_model_views() {
        let (Some(spec), Some(png)) = (
            std::env::var("BERC_MODEL_VIEWS").ok(),
            std::env::var_os("BERC_MODEL_VIEWS_PNG"),
        ) else {
            eprintln!("BERC_MODEL_VIEWS or BERC_MODEL_VIEWS_PNG not set: no real drawing");
            return;
        };
        let parts: Vec<&str> = spec.split(';').collect();
        let num = |s: &str| s.trim().parse::<f32>().unwrap();
        let pair = |s: &str| {
            let v: Vec<f32> = s.split(',').map(num).collect();
            [v[0], v[1]]
        };
        let rs = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            eframe::egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let settings = FootprintReviewSettings {
            library_search: Vec::new(),
            library_name: Vec::new(),
            ..Default::default()
        };
        let mut cx = Context::new();
        let mut h = Harness::builder()
            .with_size(vec2(1700.0, 1000.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(
                rs.clone(),
            ))
            .build_ui_state(
                move |ui, state: &mut FootprintReview| {
                    state.ui(ui, &mut cx, Id::new("fp model views real"))
                },
                FootprintReview::with(settings, Arc::new(|_: &str| {})),
            );
        blend::register(&h.ctx, &rs, 1, None);
        super::super::model3d::register(&h.ctx, &rs);
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, Path::new(parts[0]));
        h.state_mut().open_datasheet(&ctx, Path::new(parts[1]));
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        let st = h.state_mut();
        st.page = parts[2].trim().parse::<usize>().unwrap() - 1;
        st.rt.auto_page = false;
        st.scale = Some(Scale {
            mm_per_point: parts[3].trim().parse().unwrap(),
            page: st.page,
            mm: 0.0,
            note: None,
        });
        st.zoom = num(parts[4]);
        let at = pair(parts[5]);
        st.pan = [-at[0] * st.zoom, -at[1] * st.zoom];
        st.rt.fit_page = false;
        for p in &parts[6..] {
            let (what, v) = p.split_once(':').unwrap();
            // `blend:MODE,OPACITY` (normal, multiply, screen, difference)
            if what.trim() == "blend" {
                let (mode, opacity) = v.split_once(',').unwrap();
                st.model_views.blend = BlendMode::ALL
                    .into_iter()
                    .find(|m| m.name().eq_ignore_ascii_case(mode.trim()));
                st.model_views.opacity = Some(num(opacity));
                continue;
            }
            if what.trim() == "turn" {
                let side = Side::ALL
                    .into_iter()
                    .find(|s| s.name().eq_ignore_ascii_case(v.trim()))
                    .unwrap();
                st.model_views.solid.turn = super::super::solid::side_rotation(side);
                continue;
            }
            let v: Vec<&str> = v.split(',').collect();
            let place = Placement {
                origin: [num(v[0]), num(v[1])],
                rotation: num(v[2]),
                mirror: v.get(3).is_some_and(|m| m.trim() == "m"),
            };
            if what == "fp" {
                st.place = place;
                st.rt.center_footprint = false;
                continue;
            }
            // The shaded model (VIEW-27): `3d:x,y,rotation[,m]` places it, `turn:side` turns it onto a side
            if what.trim() == "3d" {
                st.model_views.look = ModelLook::Solid;
                st.model_views.solid.place = Some(place);
                continue;
            }
            let side = Side::ALL
                .into_iter()
                .find(|s| s.name().eq_ignore_ascii_case(what.trim()))
                .unwrap();
            st.model_views.views.push(ModelView { side, place });
        }
        if st.model_views.blend.is_none() {
            st.model_views.blend = Some(BlendMode::Multiply);
            st.model_views.opacity = Some(0.45);
        }
        h.run();
        h.run();
        let st = h.state();
        let l = st.rt.model.loaded.as_ref().expect("the model read");
        assert!(l.shown > 0, "{:?}", l.notes);
        for side in Side::ALL {
            if let Some(v) = st.projected(side) {
                eprintln!(
                    "{}: extent {:?}, {} fill triangles, {} contours, {} edges, {} snaps",
                    side.name(),
                    v.extent,
                    v.fill.len(),
                    v.outline.len(),
                    v.edges.len(),
                    v.snaps.len()
                );
            }
        }
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        for _ in 0..4 {
            h.step();
        }
        let image = h.render().unwrap();
        image.save(png).unwrap();
    }
}
