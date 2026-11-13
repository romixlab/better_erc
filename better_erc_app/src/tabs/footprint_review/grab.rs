//! What a press in Move drags (VIEW-32): whatever is under the pointer. A press on the footprint drags the footprint,
//! on a silhouette view that view, on the 3D model the model, on the empty page it pans the page. Where two overlap,
//! the one the moves switch (`Q`) picks wins, else the topmost (the footprint is drawn over the model). Hidden ones
//! are never picked. Hovering highlights what a press would pick and the mode bar names it. The moves switch stays
//! the choice for the keys, the arrows, turn, mirror and the SpaceMouse.
//!
//! A hit is decided in each one's own frame: the page point taken back through its placement into footprint or
//! view mm, then held against its extent (a few screen points of slack around it).

use super::modelviews::{ModelLook, Moves, Side};
use super::*;

/// What a press in Move drags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    Footprint,
    /// A silhouette view (Silhouettes)
    View(Side),
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
            Grab::View(side) => {
                self.model_shown()
                    && self.model_views.look == ModelLook::Silhouettes
                    && self.model_views.shown[side.index()]
                    && match (&self.model_views.place[side.index()], self.projected(side)) {
                        (Some(place), Some(v)) => v
                            .extent
                            .is_some_and(|e| inside(e, self.unplaced(place, p), slack)),
                        _ => false,
                    }
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

    /// What a press at page point `p` drags: the moves switch's pick first, then the topmost; the page when nothing
    /// shown is there
    pub(super) fn grab_at(&self, p: [f32; 2]) -> Grab {
        let mut model: Vec<Grab> = Vec::new();
        match self.model_views.look {
            ModelLook::Solid => model.push(Grab::Solid),
            ModelLook::Silhouettes => {
                // The view picked first, then the others in their order
                model.push(Grab::View(self.model_views.view));
                model.extend(
                    Side::ALL
                        .into_iter()
                        .filter(|s| *s != self.model_views.view)
                        .map(Grab::View),
                );
            }
        }
        let order: Vec<Grab> = match self.model_views.moves {
            Moves::Model => model.into_iter().chain([Grab::Footprint]).collect(),
            Moves::Footprint => std::iter::once(Grab::Footprint).chain(model).collect(),
        };
        order
            .into_iter()
            .find(|g| self.grab_hits(*g, p))
            .unwrap_or(Grab::Page)
    }

    /// The placement a drag of `g` moves; none for the page
    pub(super) fn grab_place_mut(&mut self, g: Grab) -> Option<&mut Placement> {
        match g {
            Grab::Footprint => Some(&mut self.place),
            Grab::View(side) => self.model_views.place[side.index()].as_mut(),
            Grab::Solid => self.model_views.solid.place.as_mut(),
            Grab::Page => None,
        }
    }

    /// What a press drags, in words, for the mode bar
    pub(super) fn grab_name(g: Grab) -> String {
        match g {
            Grab::Footprint => "the footprint".into(),
            Grab::View(s) => format!("the {} view", s.name().to_lowercase()),
            Grab::Solid => "the 3D model".into(),
            Grab::Page => "the page (pans)".into(),
        }
    }

    /// The outline of what a press would pick, thin, in the highlight colour
    pub(super) fn draw_grab_hover(&self, painter: &egui::Painter, canvas: Rect, g: Grab) {
        let s = Stroke::new(1.5, HOVER_COLOUR);
        let closed = |pts: Vec<Pos2>| {
            painter.add(Shape::closed_line(pts, s));
        };
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
            Grab::View(side) => {
                if let (Some(place), Some(v)) =
                    (&self.model_views.place[side.index()], self.projected(side))
                {
                    for c in &v.outline {
                        closed(
                            c.iter()
                                .map(|p| self.to_screen(canvas, self.placed(place, *p)))
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
