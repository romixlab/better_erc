//! What a press in Move drags (VIEW-32): whatever is under the pointer. A press on the footprint drags the footprint,
//! on a projection that projection, on the 3D model the model, on the empty page it pans the page. Where two overlap,
//! the one the pointer was last over wins, else the topmost (the footprint is drawn over the model). Hidden ones are
//! never picked. Hovering highlights what a press would pick and the mode bar names it.
//!
//! The keys (turn, mirror, the arrows, blink), the SpaceMouse and the placement controls act on the same: what the
//! pointer is over, else what it was last over ([`FootprintReview::aimed`]), marked while the pointer is off the
//! canvas. There is no selection to make first.
//!
//! A double click brings the footprint's origin under the pointer, a double right click the 3D model's (the shaded
//! model in 3D, the projection aimed at else); the view does not pan.
//!
//! A hit is decided in each one's own frame: the page point taken back through its placement into footprint or
//! view mm, then held against its extent (a few screen points of slack around it).

use super::modelviews::ModelLook;
use super::*;

/// What a press in Move drags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    Footprint,
    /// A projection of the model, by its index in [`super::modelviews::ModelViews::views`]
    View(usize),
    /// The shaded 3D model (3D)
    Solid,
    /// Nothing under the pointer: the page pans
    Page,
}

/// Screen points of slack around what can be picked
const SLACK_PT: f32 = 4.0;

/// The highlight of what a press would pick
const HOVER_COLOUR: Color32 = Color32::from_rgb(255, 200, 0);

fn inside((lo, hi): ([f64; 2], [f64; 2]), p: [f64; 2], slack: f64) -> bool {
    p[0] >= lo[0] - slack && p[0] <= hi[0] + slack && p[1] >= lo[1] - slack && p[1] <= hi[1] + slack
}

impl FootprintReview {
    /// A page point back in the mm of what `place` placed (the inverse of [`FootprintReview::placed`])
    fn unplaced(&self, place: &Placement, p: [f32; 2]) -> [f64; 2] {
        let k = self.mm_per_point();
        let d = [
            (p[0] - place.origin[0]) as f64 * k,
            (p[1] - place.origin[1]) as f64 * k,
        ];
        let r = outline::rotate(d, -place.rotation as f64);
        if place.mirror { [-r[0], r[1]] } else { r }
    }

    /// The slack around a target, mm
    fn slack_mm(&self) -> f64 {
        (SLACK_PT / self.zoom.max(1e-6)) as f64 * self.mm_per_point()
    }

    /// Whether `g` is under page point `p`
    fn grab_hits(&self, g: Grab, p: [f32; 2]) -> bool {
        let slack = self.slack_mm();
        match g {
            Grab::Footprint => {
                self.footprint_shown()
                    && outline::extent(&self.rt.items)
                        .is_some_and(|e| inside(e, self.unplaced(&self.place, p), slack))
            }
            Grab::View(i) => {
                self.model_shown()
                    && self.model_views.views.get(i).is_some_and(|view| {
                        self.projected(view.side)
                            .and_then(|v| v.extent)
                            .is_some_and(|e| inside(e, self.unplaced(&view.place, p), slack))
                    })
            }
            Grab::Solid => {
                let Some(place) =
                    self.model_views.solid.place.as_ref().filter(|_| {
                        self.model_shown() && self.model_views.look == ModelLook::Solid
                    })
                else {
                    return false;
                };
                let q = self.unplaced(place, p);
                // On a side: that side's silhouette, turned with it; between sides: the middle of the picture
                match self
                    .solid_side()
                    .and_then(|(side, k)| Some((self.projected(side)?.extent?, k)))
                {
                    Some((e, k)) => inside(e, outline::rotate(q, 90.0 * k as f64), slack),
                    None => {
                        let r = 0.6 * self.rt.solid.radius as f64;
                        r > 0.0 && inside(([-r, -r], [r, r]), q, slack)
                    }
                }
            }
            Grab::Page => true,
        }
    }

    /// What a press at page point `p` drags: the one last over first, then the topmost; the page when nothing
    /// shown is there
    pub(super) fn grab_at(&self, p: [f32; 2]) -> Grab {
        // The projections are drawn over the 3D model, so one just added on it can be dragged off; the later
        // over the earlier
        let solid = (self.model_views.look == ModelLook::Solid).then_some(Grab::Solid);
        let model = (0..self.model_views.views.len())
            .rev()
            .map(Grab::View)
            .chain(solid);
        // The one last over first, so a pointer moving on from it onto an overlap keeps it; then the topmost
        std::iter::once(self.rt.aim)
            .chain([Grab::Footprint])
            .chain(model)
            .find(|g| self.grab_hits(*g, p))
            .unwrap_or(Grab::Page)
    }

    /// What the keys, the puck and the placement controls act on: what the pointer was last over on the canvas,
    /// the footprint before anything was. One hidden gives way to the other shown; the 3D model, else the view of
    /// the side picked, stands for the model when the pointer was last over the footprint
    pub(super) fn aimed(&self) -> Grab {
        let views = self.model_views.views.len();
        let model = match (self.rt.aim, self.model_views.look) {
            (Grab::View(i), _) if i < views => Some(Grab::View(i)),
            (_, ModelLook::Solid) => Some(Grab::Solid),
            (_, ModelLook::Silhouettes) => {
                self.model_views.of(self.model_views.view).map(Grab::View)
            }
        };
        let on_model = matches!(self.rt.aim, Grab::View(_) | Grab::Solid);
        let (model_hidden, fp_hidden) = (self.rt.model_hidden, self.rt.fp_hidden);
        let to_model = if on_model {
            !(model_hidden && !fp_hidden)
        } else {
            fp_hidden && !model_hidden
        };
        match model {
            Some(m) if to_model => m,
            _ => Grab::Footprint,
        }
    }

    /// The mark on what the keys and the puck act on, while the pointer is off the canvas: its outline, dashed
    pub(super) fn draw_aim(&self, painter: &egui::Painter, canvas: Rect) {
        let mut shapes = Vec::new();
        self.grab_outline(canvas, self.aimed(), |pts| {
            let mut closed = pts.clone();
            closed.extend(pts.first());
            shapes.extend(Shape::dashed_line(
                &closed,
                Stroke::new(1.5, HOVER_COLOUR),
                6.0,
                4.0,
            ));
        });
        painter.extend(shapes);
    }

    /// The 3D model's placement as it shows over the page: the shaded model in 3D; in Silhouettes the projection
    /// aimed at, else the first of the side picked; none while the model is hidden or not placed
    pub(super) fn model_place_mut(&mut self) -> Option<&mut Placement> {
        if !self.model_shown() {
            return None;
        }
        let g = match (self.model_views.look, self.aimed()) {
            (ModelLook::Solid, _) => Grab::Solid,
            (ModelLook::Silhouettes, Grab::View(i)) => Grab::View(i),
            (ModelLook::Silhouettes, _) => Grab::View(self.model_views.of(self.model_views.view)?),
        };
        self.grab_place_mut(g)
    }

    /// A double click at page point `p` puts the footprint's origin there (Move only: the other modes take clicks),
    /// a double right click the 3D model's; turn, mirror and the view stay
    pub(super) fn bring_to_pointer(&mut self, resp: &egui::Response, p: [f32; 2]) {
        if self.tool == Tool::Move && resp.double_clicked() && self.footprint_shown() {
            self.place.origin = p;
            self.centre_restart();
        }
        if resp.double_clicked_by(egui::PointerButton::Secondary)
            && let Some(place) = self.model_place_mut()
        {
            place.origin = p;
        }
    }

    /// The placement a drag of `g` moves; none for the page
    pub(super) fn grab_place_mut(&mut self, g: Grab) -> Option<&mut Placement> {
        match g {
            Grab::Footprint => Some(&mut self.place),
            Grab::View(i) => self.model_views.views.get_mut(i).map(|v| &mut v.place),
            Grab::Solid => self.model_views.solid.place.as_mut(),
            Grab::Page => None,
        }
    }

    /// What a press drags, in words, for the mode bar
    pub(super) fn grab_name(&self, g: Grab) -> String {
        match g {
            Grab::Footprint => "the footprint".into(),
            Grab::View(i) => match self.model_views.views.get(i) {
                Some(v) => format!("the {} view", v.side.name().to_lowercase()),
                None => "the page (pans)".into(),
            },
            Grab::Solid => "the 3D model".into(),
            Grab::Page => "the page (pans)".into(),
        }
    }

    /// The outline of what a press would pick, thin, in the highlight colour
    pub(super) fn draw_grab_hover(&self, painter: &egui::Painter, canvas: Rect, g: Grab) {
        let s = Stroke::new(1.5, HOVER_COLOUR);
        self.grab_outline(canvas, g, |pts| {
            painter.add(Shape::closed_line(pts, s));
        });
    }

    /// The outline of `g` on screen, each closed contour given to `closed`
    fn grab_outline(&self, canvas: Rect, g: Grab, mut closed: impl FnMut(Vec<Pos2>)) {
        let corners = |(lo, hi): ([f64; 2], [f64; 2])| [lo, [hi[0], lo[1]], hi, [lo[0], hi[1]]];
        match g {
            Grab::Footprint => {
                if let Some(e) = outline::extent(&self.rt.items) {
                    closed(
                        corners(e)
                            .map(|p| self.to_screen(canvas, self.fp_to_page(p)))
                            .to_vec(),
                    );
                }
            }
            Grab::View(i) => {
                if let Some(view) = self.model_views.views.get(i)
                    && let Some(v) = self.projected(view.side)
                {
                    for c in &v.outline {
                        closed(
                            c.iter()
                                .map(|p| self.to_screen(canvas, self.placed(&view.place, *p)))
                                .collect(),
                        );
                    }
                }
            }
            Grab::Solid => {
                let Some(place) = &self.model_views.solid.place else {
                    return;
                };
                let screen = |p: [f64; 2]| self.to_screen(canvas, self.placed(place, p));
                match self
                    .solid_side()
                    .and_then(|(side, k)| Some((self.projected(side)?, k)))
                {
                    Some((v, k)) => {
                        for c in &v.outline {
                            closed(
                                c.iter()
                                    .map(|p| screen(super::solid::quartered(*p, k)))
                                    .collect(),
                            );
                        }
                    }
                    None => {
                        let r = 0.6 * self.rt.solid.radius as f64;
                        closed(corners(([-r, -r], [r, r])).map(screen).to_vec());
                    }
                }
            }
            Grab::Page => {}
        }
    }
}
