//! The shaded 3D model over the datasheet page (VIEW-27): ve_view3d renders the footprint's model (its parts, no
//! board or pads) with an orthographic camera at the page's mm scale into a picture of its own, which the blend pass
//! mixes into the page with the model views' blend mode, opacity and blink.
//!
//! **Placement** is the model views': the origin where the footprint's origin lands, a turn and a mirror (T and X),
//! applied to the picture as to a silhouette. **The turn** is the camera's rotation in the model's frame
//! ([`Solid::turn`], ve_view3d's convention): the SpaceMouse's tilt, roll and twist and Ctrl+drag turn it, and when
//! they let go it finishes onto the nearest of the 24 axis-aligned orientations, eased (the settings' snap; Alt held:
//! the free angle), so it lands exactly on a drawing's view. Sitting on a side, it is one of the silhouettes turned
//! in quarter turns ([`side_of`]): Measure footprint snaps to that silhouette's corners.
//!
//! **The picture** is a square around the model's origin, as wide as the model reaches from it both ways, so the
//! origin is its middle at every turn; it is rendered again only when the turn, the zoom or the size change.

use super::modelviews::{ModelLook, Side};
use super::*;
use egui::Vec2;
use std::f32::consts::FRAC_PI_2;
use ve_theme::Tone;
use ve_theme::props::*;
use ve_view3d::glam::Quat;
use ve_view3d::{Release, SnapAnim, View3d, ViewPreset, angle_to_nearest};

/// The most pixels a side of the picture gets; zoomed in further, it is stretched
const MAX_PICTURE_PX: f32 = 3072.0;

/// What perspective over the page does, for its switch
pub(super) const PERSPECTIVE_HINT: &str = "Perspective, for looking at depth: farther parts smaller, nearer ones larger; the \
     page's mm scale holds at the board plane only (through the footprint's origin)";

/// Radians the model turns per point of Ctrl+drag
const DRAG_TURN: f32 = 0.01;

/// The shaded model's state in the tab, saved with it
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Solid {
    /// Where it sits on the page; none until 3D is first picked (then the footprint's)
    pub place: Option<Placement>,
    /// How it is turned: the camera's rotation in the model's frame; from the top at first
    pub turn: Quat,
    /// Seen in perspective, for looking at depth: the page's mm scale then holds only on the plane through the
    /// footprint's origin facing the view (the board's surface seen from the top or bottom) (VIEW-32)
    pub perspective: bool,
}

impl Default for Solid {
    fn default() -> Self {
        Self {
            place: None,
            turn: side_rotation(Side::Top),
            perspective: false,
        }
    }
}

/// The shaded model's state while the tab is open: never saved
#[derive(Default)]
pub(super) struct SolidRt {
    /// The renderer, made on first use with the app's wgpu render state; its scene the model's parts
    pub view: Option<View3d>,
    /// The scene handed to the view: for the model read with this key
    key: Option<String>,
    /// How far the model reaches from its origin, mm
    pub radius: f32,
    /// Watches the puck's and the drag's turn let go
    release: Release,
    /// The finishing turn under way
    pub anim: Option<SnapAnim>,
    /// Something turned the model this frame: the puck or a Ctrl+drag
    pub turned: bool,
    /// Into the middle of the canvas at the next frame
    pub centre: bool,
    /// Where its name was drawn last, screen points: its bottom left (VIEW-36)
    pub label_at: Option<Pos2>,
}

/// The turn that shows `side` as its silhouette does (the bottom seen from below, x mirrored by looking from there)
pub fn side_rotation(side: Side) -> Quat {
    match side {
        Side::Top => ViewPreset::Top.rotation(),
        Side::Bottom => ViewPreset::Bottom.rotation() * Quat::from_rotation_z(2.0 * FRAC_PI_2),
        Side::Front => ViewPreset::Front.rotation(),
        Side::Back => ViewPreset::Back.rotation(),
        Side::Left => ViewPreset::Left.rotation(),
        Side::Right => ViewPreset::Right.rotation(),
    }
}

/// The side `turn` sits on and its quarter turns: `turn` is `side_rotation(side)` turned by `quarters` × 90° about
/// the view axis, which shows the side's silhouette turned that many quarter turns clockwise on screen; `None`
/// between sides
pub fn side_of(turn: Quat) -> Option<(Side, u8)> {
    if angle_to_nearest(turn) > 1e-3 {
        return None;
    }
    Side::ALL.into_iter().find_map(|s| {
        (0..4u8).find_map(|k| {
            let q = side_rotation(s) * Quat::from_rotation_z(k as f32 * FRAC_PI_2);
            (q.angle_between(turn) < 1e-3).then_some((s, k))
        })
    })
}

/// A point of a side's silhouette (x right, y down, mm) where it lands with the model turned `quarters` quarter
/// turns on that side
pub fn quartered(p: [f64; 2], quarters: u8) -> [f64; 2] {
    outline::rotate(p, -90.0 * quarters as f64)
}

/// A turn in the screen's frame (x right, y up, z toward the viewer) as the same turn in the picture's frame, the
/// picture turned `rotation` degrees counter-clockwise on screen and mirrored first when `mirror`
fn into_picture(on_screen: Quat, rotation: f32, mirror: bool) -> Quat {
    let r = Quat::from_rotation_z(rotation.to_radians());
    let q = r.inverse() * on_screen * r;
    // Mirrored left to right: a turn's axis mirrors and its sense reverses
    if mirror {
        Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
    } else {
        q
    }
}

impl FootprintReview {
    /// The side the shaded model sits on, if it sits on one
    pub(super) fn solid_side(&self) -> Option<(Side, u8)> {
        side_of(self.model_views.solid.turn)
    }

    /// The side in words: "top", "front, a quarter turn", "between sides"
    pub(super) fn solid_side_name(&self) -> String {
        match self.solid_side() {
            Some((s, 0)) => s.name().to_lowercase(),
            Some((s, k)) => format!("{} turned {}°", s.name().to_lowercase(), 90 * k as u32),
            None => "between sides".into(),
        }
    }

    /// Turns the shaded model onto `side` (as its silhouette shows it), eased by the snap's time
    pub(super) fn turn_solid_to(&mut self, side: Side, now: f64) {
        let time = self.rt.settings.model_snap.duration;
        self.rt.solid.anim =
            SnapAnim::towards(self.model_views.solid.turn, side_rotation(side), now, time);
        if self.rt.solid.anim.is_none() {
            self.model_views.solid.turn = side_rotation(side);
        }
    }

    /// Turns the shaded model by a turn seen on screen (x right, y up, z toward the viewer)
    pub(super) fn turn_solid(&mut self, on_screen: Quat) {
        let place = self.model_views.solid.place.clone().unwrap_or_default();
        let q = into_picture(on_screen, place.rotation, place.mirror);
        let s = &mut self.model_views.solid;
        s.turn = (s.turn * q.inverse()).normalize();
        self.rt.solid.anim = None;
        self.rt.solid.turned = true;
    }

    /// The puck's tilt, roll and twist turn the shaded model while 3D is shown (at the mapping's rotate speed)
    pub(super) fn solid_puck(&mut self, nav: &ve_spacemouse::Nav, dt: f32) {
        if self.model_views.look != ModelLook::Solid || nav.rotate.iter().all(|r| *r == 0.0) {
            return;
        }
        let [tilt, roll, turn] = nav.rotate.map(|r| r * dt);
        // As ve_view3d's camera takes the puck: the content tilts forward, rolls right, turns clockwise
        self.turn_solid(
            Quat::from_rotation_x(-tilt)
                * Quat::from_rotation_y(roll)
                * Quat::from_rotation_z(-turn),
        );
    }

    /// A Ctrl+drag by `delta` points turns the shaded model: sideways about the screen's up, up and down about its
    /// right, the near side following the pointer
    pub(super) fn solid_drag(&mut self, delta: Vec2) {
        self.turn_solid(
            Quat::from_rotation_y(delta.x * DRAG_TURN) * Quat::from_rotation_x(delta.y * DRAG_TURN),
        );
    }

    /// Each frame in 3D: the scene handed to the renderer, a turn that let go finished onto the nearest side, the
    /// finishing turn moved on, the model centred when asked
    pub(super) fn solid_frame(&mut self, ctx: &egui::Context, canvas: Rect) {
        let (now, alt) = ctx.input(|i| (i.time, i.modifiers.alt));
        let turned = std::mem::take(&mut self.rt.solid.turned);
        let snap = self.rt.settings.model_snap;
        if self.rt.solid.release.update(turned) && snap.enabled && !alt {
            self.rt.solid.anim = SnapAnim::new(self.model_views.solid.turn, now, snap.duration);
            self.usage("model 3d snap");
        }
        if let Some(anim) = self.rt.solid.anim {
            let (q, done) = anim.at(now);
            self.model_views.solid.turn = q;
            if done {
                self.rt.solid.anim = None;
            } else {
                ctx.request_repaint();
            }
        }
        if self.model_views.look != ModelLook::Solid {
            return;
        }
        if self.model_views.solid.place.is_none() {
            self.model_views.solid.place = Some(self.place.clone());
        }
        // The scene: the model's parts, handed over once per model read
        let key = self.rt.model.key.clone();
        if self.rt.solid.key != key {
            self.rt.solid.key = None;
            if let Some(v) = &mut self.rt.solid.view {
                v.set_scene(ve_view3d::Scene::default());
            }
        }
        if self.rt.solid.key.is_none()
            && let Some(scene) = self.rt.model.loaded.as_mut().and_then(|l| l.parts.take())
        {
            let view =
                self.rt
                    .solid
                    .view
                    .get_or_insert_with(|| match super::model3d::render_state(ctx) {
                        Some(rs) => View3d::new(&rs),
                        None => View3d::without_gpu(),
                    });
            let b = scene.bounds();
            self.rt.solid.radius = if b.is_empty() {
                0.0
            } else {
                // The farthest corner of the box from the origin: the picture holds the model at every turn
                (0..8)
                    .map(|i| {
                        let pick =
                            |bit: usize, lo: f32, hi: f32| if i & bit == 0 { lo } else { hi };
                        ve_view3d::glam::Vec3::new(
                            pick(1, b.min.x, b.max.x),
                            pick(2, b.min.y, b.max.y),
                            pick(4, b.min.z, b.max.z),
                        )
                        .length()
                    })
                    .fold(0.0, f32::max)
                    * 1.02
            };
            view.set_scene(scene);
            self.rt.solid.key = key;
        }
        if self.rt.solid.centre && canvas.width() > 1.0 && self.rt.solid.key.is_some() {
            let target = self.to_page(canvas, canvas.center());
            if let Some(p) = &mut self.model_views.solid.place {
                p.origin = target;
            }
            self.rt.solid.centre = false;
        }
    }

    /// The renderer has no GPU (no wgpu renderer): why there is no 3D picture
    pub(super) fn solid_blocker(&self) -> Option<&'static str> {
        match &self.rt.solid.view {
            Some(v) if !v.has_gpu() => Some(
                "the 3D picture needs the app's wgpu renderer, which this window does not have",
            ),
            _ => None,
        }
    }

    /// The picture's corners on the page, in footprint mm around the origin before the placement (x right, y down):
    /// top left, top right, bottom right, bottom left
    fn solid_corners(&self) -> [[f64; 2]; 4] {
        let r = self.solid_half() as f64;
        [[-r, -r], [r, -r], [r, r], [-r, r]]
    }

    /// Half the picture's side at the origin's depth, mm. Orthographic: the model's reach. In perspective the eye
    /// sits at `half / tan(fov / 2)`, and a part nearer the eye looks larger; the picture holds it at every turn
    /// when it is `1 + tan(fov / 2)` times the reach
    fn solid_half(&self) -> f32 {
        let r = self.rt.solid.radius;
        if self.model_views.solid.perspective {
            r * (1.0 + (super::model3d::fov_radians(self.rt.settings.fov_deg) * 0.5).tan())
        } else {
            r
        }
    }

    /// Draws the shaded model: rendered at the page's scale, mixed into the page by the blend pass in every blend
    /// mode, else by egui's painter at the opacity; its origin and name by the painter on top
    pub(super) fn draw_solid(
        &mut self,
        painter: &egui::Painter,
        canvas: Rect,
        page: Option<(Rect, blend::PageImage)>,
        id: Id,
    ) {
        if self.model_views.look != ModelLook::Solid {
            return;
        }
        let Some(place) = self.model_views.solid.place.clone() else {
            return;
        };
        let r = self.solid_half();
        let mm_per_point = self.mm_per_point();
        if r <= 0.0 || mm_per_point <= 0.0 {
            return;
        }
        let perspective = self.model_views.solid.perspective;
        let fov = super::model3d::fov_radians(self.rt.settings.fov_deg);
        let ctx = painter.ctx().clone();
        // Screen points per mm, and the picture's side in points (capped in pixels)
        let ppmm = self.zoom / mm_per_point as f32;
        let side = (2.0 * r * ppmm)
            .min(MAX_PICTURE_PX / ctx.pixels_per_point())
            .max(8.0);
        let turn = self.model_views.solid.turn;
        let Some(view) = &mut self.rt.solid.view else {
            return;
        };
        view.camera.target = ve_view3d::glam::Vec3::ZERO;
        view.camera.rotation = turn;
        view.xray = self.xray;
        view.style.hidden_edges = self.rt.settings.hidden_edges;
        // The page's scale at the origin's depth; perspective keeps it only there (VIEW-32)
        if perspective {
            view.camera.projection = ve_view3d::Projection::Perspective;
            view.camera.fov_y = fov;
            view.camera.set_scale(side / (2.0 * r), side);
        } else {
            view.camera.fov_y = fov;
            view.camera.set_ortho_scale(side / (2.0 * r), side);
        }
        let Some((texture, picture)) = view.render_offscreen(&ctx, vec2(side, side)) else {
            return;
        };
        // The model's box corners in the picture, points from its top left (VIEW-36)
        let bounds = view.bounds();
        let projector = view
            .camera
            .projector(Rect::from_min_size(Pos2::ZERO, vec2(side, side)), &bounds);
        let (lo, hi) = (bounds.min, bounds.max);
        let in_picture: Vec<Pos2> = (0..8)
            .filter(|_| !bounds.is_empty())
            .filter_map(|i| {
                let pick = |bit: usize, a: f32, b: f32| if i & bit == 0 { a } else { b };
                let p = ve_view3d::glam::Vec3::new(
                    pick(1, lo.x, hi.x),
                    pick(2, lo.y, hi.y),
                    pick(4, lo.z, hi.z),
                );
                projector.project(p).map(|(q, _)| q)
            })
            .collect();
        let a = self.model_views_opacity();
        let mode = self.model_views_blend();
        let gpu = blend::available(&ctx) && page.is_some();
        let corners = self
            .solid_corners()
            .map(|c| self.to_screen(canvas, self.placed(&place, c)));
        let uv = [
            pos2(0.0, 0.0),
            pos2(1.0, 0.0),
            pos2(1.0, 1.0),
            pos2(0.0, 1.0),
        ];
        let tint = if gpu {
            Color32::WHITE
        } else {
            Color32::WHITE.gamma_multiply(a)
        };
        let mut mesh = egui::Mesh::with_texture(texture);
        for (p, uv) in corners.iter().zip(uv) {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: *p,
                uv,
                color: tint,
            });
        }
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        match (gpu, page) {
            (true, Some((page_rect, image))) => {
                painter.add(eframe::egui_wgpu::Callback::new_paint_callback(
                    canvas,
                    blend::BlendPass {
                        id: id.with("model 3d"),
                        mesh,
                        page: image,
                        page_rect,
                        around: CANVAS_BACKGROUND,
                        opacity: a,
                        mode,
                        viewport: canvas,
                        layer: Some(picture),
                    },
                ));
            }
            _ => {
                painter.add(Shape::mesh(mesh));
            }
        }
        // Its origin and name
        let colors = self.layer_colors();
        let [cr, cg, cb] = colors.model_edges.0;
        let line = Color32::from_rgb(cr, cg, cb);
        let o = self.to_screen(canvas, place.origin);
        let s = Stroke::new(1.0, line);
        painter.line_segment([o - vec2(6.0, 0.0), o + vec2(6.0, 0.0)], s);
        painter.line_segment([o - vec2(0.0, 6.0), o + vec2(0.0, 6.0)], s);
        // Its name over the top left corner of the model's box on screen, the picture's when it has no box
        let to_mm = |q: Pos2| {
            let k = |t: f32| ((t / side) * 2.0 - 1.0) as f64 * r as f64;
            [k(q.x), k(q.y)]
        };
        let on_screen: Vec<Pos2> = in_picture
            .iter()
            .map(|q| self.to_screen(canvas, self.placed(&place, to_mm(*q))))
            .collect();
        let top_left = super::modelviews::label_corner(if on_screen.is_empty() {
            &corners
        } else {
            &on_screen
        });
        self.rt.solid.label_at = Some(top_left);
        let text = format!(
            "3D model · {}{}{}",
            self.solid_side_name(),
            if perspective {
                " · perspective: mm at the board plane only"
            } else {
                ""
            },
            if self.moving_solid() { " · moves" } else { "" }
        );
        painter.text(
            top_left,
            Align2::LEFT_BOTTOM,
            text,
            FontId::proportional(12.0),
            line,
        );
    }

    /// Where Measure footprint snaps on the shaded model: while it sits on a side, the corners of that side's
    /// silhouette turned with it, page points
    pub(super) fn solid_snaps(&self) -> impl Iterator<Item = [f32; 2]> + '_ {
        // In perspective the outline is not the silhouette's: nothing to snap to
        let on = (self.model_views.look == ModelLook::Solid && !self.model_views.solid.perspective)
            .then(|| self.solid_side())
            .flatten()
            .filter(|_| self.rt.solid.anim.is_none());
        let place = self.model_views.solid.place.clone().unwrap_or_default();
        on.into_iter().flat_map(move |(side, k)| {
            let place = place.clone();
            self.projected(side)
                .map(|v| v.snaps.clone())
                .unwrap_or_default()
                .into_iter()
                .map(move |p| self.placed(&place, quartered(p, k)))
        })
    }

    /// The 3D part of the Model views section: the side it sits on, turn to a side, the snap
    pub(super) fn solid_controls(&mut self, ui: &mut Ui) {
        let k = self.keys();
        if let Some(why) = self.solid_blocker() {
            notice(ui, Tone::Warn, format!("No 3D picture: {why}"));
        }
        let now = ui.input(|i| i.time);
        Group::new("Turn to")
            .tooltip(format!(
                "Turns the model onto a side, as its silhouette shows it; {} the next side. The SpaceMouse's tilt, \
                 roll and twist and Ctrl+drag in Move turn it freely",
                key_name(k.model_view)
            ))
            .show(ui, |ui| {
                let on = self.solid_side();
                let mut pick = None;
                ui.horizontal_wrapped(|ui| {
                    for side in Side::ALL {
                        if ui
                            .selectable_label(on == Some((side, 0)), side.name())
                            .on_hover_text(format!(
                                "The model seen from the {}",
                                side.name().to_lowercase()
                            ))
                            .clicked()
                        {
                            pick = Some(side);
                        }
                    }
                });
                if let Some(side) = pick {
                    self.model_views.view = side;
                    self.turn_solid_to(side, now);
                    self.usage(&format!("model 3d side {}", side.name().to_lowercase()));
                }
                ui.label(format!("Now: {}", self.solid_side_name()));
            });
        self.solid_camera_controls(ui);
        Group::new("Snap")
            .tooltip("What a turn does when the puck or the drag lets go; Settings → Save keeps it")
            .show(ui, |ui| {
                self.rt.settings.model_snap.ui(ui);
            });
        self.xray_controls(ui);
    }

    /// The model over the page in perspective (`on`) or orthographic
    pub(super) fn set_perspective(&mut self, on: bool) {
        self.model_views.solid.perspective = on;
        self.usage(if on {
            "model 3d perspective"
        } else {
            "model 3d orthographic"
        });
    }

    /// Orthographic or perspective for the model over the page, and the field of view (shared with the 3D view's)
    /// (VIEW-32)
    fn solid_camera_controls(&mut self, ui: &mut Ui) {
        Group::new("Camera")
            .tooltip(PERSPECTIVE_HINT)
            .show(ui, |ui| {
                let mut on = self.model_views.solid.perspective;
                Segmented::new(&mut on)
                    .id_salt("model 3d perspective")
                    .item(
                        false,
                        "Orthographic",
                        "sizes don't change with depth: the page's mm scale everywhere, for measuring",
                    )
                    .item(true, "Perspective", PERSPECTIVE_HINT)
                    .fill_width()
                    .show(ui);
                if on != self.model_views.solid.perspective {
                    self.set_perspective(on);
                }
                PropertyRow::new("Field of view")
                    .tooltip("vertical, degrees, the 3D view's too; Settings → Save keeps it")
                    .show(ui, |ui| {
                        ui.add_enabled(
                            on,
                            ScalarField::new(&mut self.rt.settings.fov_deg)
                                .range(
                                    ve_view3d::Camera::FOV_MIN.to_degrees() as f64
                                        ..=ve_view3d::Camera::FOV_MAX.to_degrees() as f64,
                                )
                                .speed(0.5)
                                .decimals(0)
                                .unit("°")
                                .tooltip("wider shows depth more strongly"),
                        )
                        .on_disabled_hover_text(
                            "orthographic has no field of view: pick Perspective to set it",
                        );
                    });
                if on {
                    notice(
                        ui,
                        Tone::Warn,
                        "Perspective is for looking at depth: the page's mm scale holds at the board plane only, \
                         nearer parts look larger, and Measure footprint has no corners of the model to snap to",
                    );
                }
            });
    }

    /// X-ray and how it draws the hidden edges, for the Model views section (VIEW-30)
    pub(super) fn xray_controls(&mut self, ui: &mut Ui) {
        let k = key_name(self.keys().xray);
        Group::new("X-ray")
            .tooltip(format!(
                "The 3D model see-through, in the 3D view and over the page; {k} switches"
            ))
            .show(ui, |ui| {
                let mut on = self.xray;
                ui.add(ToggleChip::new(&mut on, format!("See through · {k}")).tooltip(
                    "the faces faded, the edges behind them drawn too, from the model's own edges",
                ));
                if on != self.xray {
                    self.toggle_xray();
                }
                PropertyRow::new("Hidden edges")
                    .tooltip("how the edges behind the faces look; Settings → Save keeps it")
                    .show(ui, |ui| {
                        Segmented::new(&mut self.rt.settings.hidden_edges)
                            .id_salt("hidden edges")
                            .item(ve_view3d::HiddenEdges::Thin, "Thin", "thinner solid lines")
                            .item(
                                ve_view3d::HiddenEdges::Dashed,
                                "Dashed",
                                "thinner dashed lines, as a drawing shows hidden lines",
                            )
                            .fill_width()
                            .show(ui);
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ve_view3d::glam::Vec3;

    #[test]
    fn each_side_and_quarter_is_found_again() {
        for s in Side::ALL {
            for k in 0..4u8 {
                let q = side_rotation(s) * Quat::from_rotation_z(k as f32 * FRAC_PI_2);
                assert_eq!(side_of(q), Some((s, k)), "{s:?} {k}");
            }
        }
        let between = side_rotation(Side::Front) * Quat::from_rotation_x(0.3);
        assert_eq!(side_of(between), None);
    }

    /// The renderer's camera at a side shows each model point where the silhouette (`Side::project`), turned by the
    /// quarter turns, puts it: the picture and the silhouettes agree
    #[test]
    fn the_picture_agrees_with_the_silhouettes() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0));
        let points = [
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(-2.5, 0.5, 1.0),
            Vec3::new(0.3, -1.7, -0.4),
        ];
        for s in Side::ALL {
            for k in 0..4u8 {
                let mut cam = ve_view3d::Camera {
                    rotation: side_rotation(s) * Quat::from_rotation_z(k as f32 * FRAC_PI_2),
                    ..Default::default()
                };
                // 10 points per mm, the origin in the middle
                cam.set_ortho_scale(10.0, rect.height());
                let bounds = ve_view3d::Bounds {
                    min: Vec3::splat(-5.0),
                    max: Vec3::splat(5.0),
                };
                let proj = cam.projector(rect, &bounds);
                for p in points {
                    let (screen, _) = proj.project(p).expect("in view");
                    let (flat, _) = s.project(p);
                    let q = quartered(flat, k);
                    let want = rect.center() + vec2(q[0] as f32, q[1] as f32) * 10.0;
                    assert!(
                        (screen - want).length() < 1e-2,
                        "{s:?} {k}: {p} at {screen:?}, the silhouette says {want:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_turn_on_screen_is_the_same_turn_in_a_turned_or_mirrored_picture() {
        // Twisting the content clockwise on screen twists it clockwise in the picture, however it is turned
        let twist = Quat::from_rotation_z(-0.2);
        assert!(into_picture(twist, 90.0, false).angle_between(twist) < 1e-5);
        // Mirrored, the twist goes the other way in the picture
        let m = into_picture(twist, 0.0, true);
        assert!(m.angle_between(twist.inverse()) < 1e-5);
        // Tilting about the screen's x in a picture turned a quarter turn is a tilt about the picture's y
        let tilt = Quat::from_rotation_x(0.3);
        let t = into_picture(tilt, 90.0, false);
        // (acos near 1 loses digits: compare to a thousandth of a radian)
        assert!(
            t.angle_between(Quat::from_rotation_y(-0.3)) < 1e-3,
            "{t} {:?}",
            t.to_axis_angle()
        );
    }

    use super::super::model3d::tests::two_pin;
    use super::super::modelviews::Moves;
    use super::super::tests::datasheet;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    type H = Harness<'static, FootprintReview>;

    /// The review tab on a real GPU: the blend pass and the 3D renderer there
    fn gpu_harness(name: &'static str) -> H {
        let rs = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            eframe::egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let settings = FootprintReviewSettings {
            font_dirs: Vec::new(),
            library_search: Vec::new(),
            library_name: Vec::new(),
            // The centre cross as short as before VIEW-35, so the pixels sampled beside the origin see the model
            centre_cross_px: 6.0,
            ..Default::default()
        };
        let mut cx = Context::new();
        let h = Harness::builder()
            .with_size(vec2(1200.0, 1000.0))
            .with_step_dt(0.05)
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(
                rs.clone(),
            ))
            .build_ui_state(
                move |ui, state: &mut FootprintReview| state.ui(ui, &mut cx, Id::new(name)),
                FootprintReview::with(settings, Arc::new(|_: &str| {})),
            );
        blend::register(&h.ctx, &rs, 1, None);
        super::super::model3d::register(&h.ctx, &rs);
        h
    }

    /// A key over the canvas; steps, not runs: an eased turn keeps repainting
    fn key(h: &mut H, k: egui::Key) {
        h.hover_at(h.state().rt.canvas.center());
        h.step();
        h.key_press(k);
        h.step();
        h.step();
    }

    fn steps(h: &mut H, n: usize) {
        for _ in 0..n {
            h.step();
        }
    }

    fn screen(h: &H, p: [f32; 2]) -> egui::Pos2 {
        let st = h.state();
        st.to_screen(st.rt.canvas, p)
    }

    fn pixel(img: &image::RgbaImage, p: egui::Pos2) -> [i32; 3] {
        let c = img.get_pixel(p.x.round() as u32, p.y.round() as u32).0;
        [c[0] as i32, c[1] as i32, c[2] as i32]
    }

    fn diff(a: [i32; 3], b: [i32; 3]) -> i32 {
        (0..3).map(|i| (a[i] - b[i]).abs()).sum()
    }

    /// The 3D model over the page: L shows it, Q moves it; at a known scale its body's edge lands at the page mm
    /// the model says; the puck turns it and letting go lands it on the side; K turns it to the next side; Ctrl+drag
    /// turns it and lands it again, Alt keeps the free angle; Measure footprint snaps to its outline's corner
    #[test]
    fn the_3d_model_over_the_page() {
        let dir = std::env::temp_dir().join(format!("berc-fp-solid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let pdf = datasheet(&dir);
        let mut h = gpu_harness("fp solid");
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, &fp);
        h.state_mut().open_datasheet(&ctx, &pdf);
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        let st = h.state_mut();
        st.scale = Some(Scale {
            mm_per_point: 0.04,
            page: st.page,
            mm: 2.0,
            note: None,
        });
        // 1 mm: 25 page points, 25 screen points
        st.zoom = 1.0;
        st.pan = [0.0, 0.0];
        st.rt.fit_page = false;
        st.place.origin = [300.0, 400.0];
        st.model_views.opacity = Some(1.0);
        h.run();
        // Nothing of the model over the page yet: the page as it is
        let plain = h.render().unwrap();

        // L: 3D, on the footprint, seen from the top, the footprint still moving; Q: the model moves
        key(&mut h, egui::Key::L);
        assert_eq!(h.state().model_views.look, ModelLook::Solid);
        assert_eq!(h.state().model_views.moves, Moves::Footprint);
        key(&mut h, egui::Key::Q);
        assert!(h.state().moving_solid());
        assert_eq!(h.state().moving_name(), "the 3D model");
        steps(&mut h, 4);
        let st = h.state();
        assert_eq!(st.model_views.solid.place.as_ref(), Some(&st.place));
        assert_eq!(st.solid_side(), Some((Side::Top, 0)));
        assert!(st.rt.solid.radius > 2.0, "{}", st.rt.solid.radius);
        assert!(st.help_line().contains("the 3D model"));

        // The body is 4 x 2 mm about the origin: its right edge 2 mm (50 points) right of it, its top 1 mm up
        let img = h.render().unwrap();
        if let Some(png) = std::env::var_os("BERC_SOLID_PNG") {
            img.save(png).unwrap();
        }
        let o = screen(&h, [300.0, 400.0]);
        // Its name over the model's top left corner, not the larger picture's (VIEW-36)
        let near = |h: &H, want: egui::Vec2| {
            let got = h.state().rt.solid.label_at.expect("named") - o;
            assert!(
                (got - want).length() < 1.5,
                "the name at {got:?}, want {want:?}"
            );
        };
        near(&h, vec2(-50.0, -27.0));
        // Turned a quarter (T): the body 2 x 4 mm, its name over its new corner; four turns: back
        key(&mut h, egui::Key::T);
        steps(&mut h, 2);
        near(&h, vec2(-25.0, -52.0));
        for _ in 0..3 {
            key(&mut h, egui::Key::T);
        }
        steps(&mut h, 2);
        near(&h, vec2(-50.0, -27.0));
        for (at, inside) in [
            (o + vec2(46.0, 0.0), true),
            (o + vec2(54.0, 0.0), false),
            (o + vec2(0.0, -21.0), true),
            (o + vec2(0.0, -29.0), false),
        ] {
            let d = diff(pixel(&img, at), pixel(&plain, at));
            if inside {
                assert!(d > 40, "the model at {at:?}: {d}");
            } else {
                assert!(d < 12, "nothing of the model at {at:?}: {d}");
            }
        }

        // U: X-ray (VIEW-30). The body fades, and its leads under it show: their inner edges at x ±1.07 mm (26.75
        // points), hidden from the top, are drawn (thin here, so solid)
        h.state_mut().rt.settings.hidden_edges = ve_view3d::HiddenEdges::Thin;
        key(&mut h, egui::Key::U);
        assert!(h.state().xray);
        let xray = h.render().unwrap();
        if let Some(png) = std::env::var_os("BERC_SOLID_XRAY_PNG") {
            xray.save(png).unwrap();
        }
        let inside = o + vec2(10.0, -12.0);
        let d = diff(pixel(&xray, inside), pixel(&img, inside));
        assert!(d > 20, "the body fades: {d}");
        // The most an edge changes the picture across a few points about x
        let edge_at = |a: &image::RgbaImage, b: &image::RgbaImage, x: f32| {
            (-3..=3)
                .map(|dx| o + vec2(x + dx as f32, 5.0))
                .map(|p| diff(pixel(a, p), pixel(b, p)))
                .max()
                .unwrap()
        };
        let beside = |x: f32| {
            (-3..=3)
                .map(|dx| o + vec2(x + dx as f32, 5.0))
                .map(|p| diff(pixel(&xray, p), pixel(&xray, o + vec2(18.0, 5.0))))
                .max()
                .unwrap()
        };
        for x in [-26.75, 26.75] {
            assert!(
                beside(x) > 30,
                "the lead's hidden edge at {x}: {}",
                beside(x)
            );
        }
        // Without X-ray no edge there: the body's top alike all across
        assert!(edge_at(&img, &img, 26.75) == 0);
        key(&mut h, egui::Key::U);
        assert!(!h.state().xray);

        // Measure footprint snaps to the outline's corner (2, 1) mm: 50, 25 points from the origin
        key(&mut h, egui::Key::G);
        let corner = [350.0, 375.0];
        h.hover_at(screen(&h, [corner[0] - 1.5, corner[1] + 1.0]));
        h.run();
        let snap = h.state().rt.snap.clone().expect("a snap at the corner");
        assert_eq!(snap.kind, SnapKind::ModelCorner);
        assert!(
            (snap.at[0] - corner[0]).abs() < 0.01 && (snap.at[1] - corner[1]).abs() < 0.01,
            "{:?}",
            snap.at
        );
        key(&mut h, egui::Key::Escape);

        // The puck twists it 20°, then lets go: it lands on the top again
        let nav = ve_spacemouse::Nav {
            rotate: [0.0, 0.0, 0.35],
            ..Default::default()
        };
        for _ in 0..20 {
            h.state_mut().solid_puck(&nav, 0.05);
            h.step();
        }
        assert_eq!(h.state().solid_side(), None, "turned off the side");
        steps(&mut h, 10);
        assert_eq!(h.state().solid_side(), Some((Side::Top, 0)));

        // K: the next side, eased
        key(&mut h, egui::Key::K);
        steps(&mut h, 10);
        assert_eq!(h.state().solid_side(), Some((Side::Bottom, 0)));

        // Ctrl+drag 40 points sideways turns it about 23°, letting go lands it back; with Alt the angle stays
        let drag = |h: &mut H, alt: bool| {
            let m = egui::Modifiers {
                command: true,
                ctrl: true,
                alt,
                ..Default::default()
            };
            let a = screen(h, [300.0, 400.0]) + vec2(80.0, 80.0);
            h.event(egui::Event::ModifiersChanged(m));
            h.hover_at(a);
            h.run();
            h.drag_at(a);
            h.step();
            for i in 1..=4 {
                h.hover_at(a + vec2(10.0 * i as f32, 0.0));
                h.step();
            }
            assert_eq!(h.state().solid_side(), None, "turning");
            h.drop_at(a + vec2(40.0, 0.0));
            h.step();
            h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
            steps(h, 10);
        };
        let origin = h.state().model_views.solid.place.clone();
        drag(&mut h, false);
        assert_eq!(h.state().solid_side(), Some((Side::Bottom, 0)));
        assert_eq!(
            h.state().model_views.solid.place,
            origin,
            "Ctrl+drag turns, not moves"
        );
        drag(&mut h, true);
        assert_eq!(h.state().solid_side(), None, "Alt: the free angle");

        // Back to the silhouettes: the 3D model is gone, the views as they were
        key(&mut h, egui::Key::L);
        assert_eq!(h.state().model_views.look, ModelLook::Silhouettes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Round 8 (VIEW-32): Home puts the model on the footprint as KiCad places it, at the footprint's origin with its
    /// turn, seen from the bottom for a footprint in bottom view, in Silhouettes and in 3D. In Move a press drags what
    /// is under it (Q's pick first where they overlap), the empty page pans; a hidden one is never dragged, the keys
    /// move the one shown until it is shown again
    #[test]
    fn the_model_on_the_footprint_and_drags_under_the_pointer() {
        let dir = std::env::temp_dir().join(format!("berc-fp-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let pdf = datasheet(&dir);
        let mut h = gpu_harness("fp home");
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, &fp);
        h.state_mut().open_datasheet(&ctx, &pdf);
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        let st = h.state_mut();
        st.scale = Some(Scale {
            mm_per_point: 0.04,
            page: st.page,
            mm: 2.0,
            note: None,
        });
        st.zoom = 1.0;
        st.pan = [0.0, 0.0];
        st.rt.fit_page = false;
        st.place = Placement {
            origin: [300.0, 400.0],
            rotation: 90.0,
            mirror: true,
        };
        st.model_views.opacity = Some(1.0);
        h.run();
        let on = Placement {
            origin: [300.0, 400.0],
            rotation: 90.0,
            mirror: false,
        };
        // Silhouettes: the bottom view onto the footprint, shown and picked
        key(&mut h, egui::Key::Home);
        let st = h.state();
        assert_eq!(
            st.model_views.place[Side::Bottom.index()].as_ref(),
            Some(&on)
        );
        assert!(st.model_views.shown[Side::Bottom.index()]);
        assert_eq!(st.model_views.view, Side::Bottom);
        h.run();
        h.run();
        let st = h.state();
        assert!(st.projected(Side::Bottom).is_some(), "the model read");
        // A model point lands where the footprint puts the same point: KiCad's footprint frame is y down
        for p in [Vec3::new(2.0, 1.0, 0.0), Vec3::new(-1.27, -0.4, 0.3)] {
            let by_model = st.placed(&on, Side::Bottom.project(p).0);
            let by_fp = st.fp_to_page([p.x as f64, -p.y as f64]);
            assert!(
                (by_model[0] - by_fp[0]).abs() < 1e-3 && (by_model[1] - by_fp[1]).abs() < 1e-3,
                "{p}: {by_model:?} {by_fp:?}"
            );
        }
        // 3D, the model moved away and turned: Home puts it back, turned onto the bottom
        key(&mut h, egui::Key::L);
        let st = h.state_mut();
        st.model_views.solid.place = Some(Placement {
            origin: [500.0, 200.0],
            rotation: 0.0,
            mirror: true,
        });
        st.model_views.solid.turn = side_rotation(Side::Front);
        key(&mut h, egui::Key::Home);
        steps(&mut h, 10);
        assert_eq!(h.state().model_views.solid.place.as_ref(), Some(&on));
        assert_eq!(h.state().solid_side(), Some((Side::Bottom, 0)));
        // The footprint on the top: the model from the top
        h.state_mut().place.mirror = false;
        key(&mut h, egui::Key::Home);
        steps(&mut h, 10);
        assert_eq!(h.state().solid_side(), Some((Side::Top, 0)));

        // A drag goes to what is under the press. The model sits on the footprint: Q's pick (the footprint) wins
        let drag = |h: &mut H, a: egui::Pos2, d: egui::Vec2| {
            h.hover_at(a);
            h.step();
            h.drag_at(a);
            h.step();
            for i in 1..=4 {
                h.hover_at(a + d * (i as f32 / 4.0));
                h.step();
            }
            h.drop_at(a + d);
            h.step();
            h.hover_at(h.state().rt.canvas.center());
            h.step();
        };
        let solid_at = |h: &H| h.state().model_views.solid.place.as_ref().unwrap().origin;
        let fp_at = |h: &H| h.state().place.origin;
        let near =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 0.5 && (a[1] - b[1]).abs() < 0.5;
        let on = screen(&h, [300.0, 400.0]) + vec2(5.0, 5.0);
        h.hover_at(on);
        h.step();
        assert_eq!(
            h.state().rt.hover_grab,
            Some(grab::Grab::Footprint),
            "hover names the pick"
        );
        drag(&mut h, on, vec2(40.0, 0.0));
        assert!(near(fp_at(&h), [340.0, 400.0]), "{:?}", fp_at(&h));
        assert!(near(solid_at(&h), [300.0, 400.0]), "the model stays");
        // Q: the model is picked first where they overlap
        key(&mut h, egui::Key::Q);
        drag(&mut h, on, vec2(0.0, 30.0));
        assert!(near(solid_at(&h), [300.0, 430.0]), "{:?}", solid_at(&h));
        assert!(near(fp_at(&h), [340.0, 400.0]));
        // The footprint alone under the press (its pad 2, right of the model now): the footprint, whatever Q says
        let pad2 = screen(&h, h.state().fp_to_page([1.27, 0.0]));
        drag(&mut h, pad2, vec2(-40.0, 0.0));
        assert!(near(fp_at(&h), [300.0, 400.0]), "{:?}", fp_at(&h));
        // The empty page pans
        let pan = h.state().pan;
        let empty = screen(&h, [700.0, 150.0]);
        drag(&mut h, empty, vec2(25.0, 10.0));
        assert_eq!(h.state().rt.grab, None);
        let p = h.state().pan;
        assert!(
            (p[0] - pan[0] - 25.0).abs() < 0.5 && (p[1] - pan[1] - 10.0).abs() < 0.5,
            "{p:?}"
        );
        assert!(near(fp_at(&h), [300.0, 400.0]) && near(solid_at(&h), [300.0, 430.0]));

        // The model hidden (J), Q on Model: never picked, the footprint under it moves; the keys move the footprint
        // too, and the mode bar says so; J again: Q's choice back
        key(&mut h, egui::Key::J);
        assert!(!h.state().moving_solid() && h.state().moves_overridden().is_some());
        let at = screen(&h, [300.0, 430.0]);
        drag(&mut h, at, vec2(0.0, 40.0));
        assert!(
            near(solid_at(&h), [300.0, 430.0]),
            "a hidden model is never dragged"
        );
        let fp = fp_at(&h);
        h.key_press(egui::Key::ArrowRight);
        h.step();
        assert_eq!(
            fp_at(&h)[0],
            fp[0] + 1.0,
            "the arrows move the footprint shown"
        );
        key(&mut h, egui::Key::J);
        assert!(h.state().moving_solid());
        // The footprint hidden (H), Q on Footprint: the model shown moves by the keys, a press on the footprint pans
        key(&mut h, egui::Key::Q);
        key(&mut h, egui::Key::H);
        assert!(h.state().moving_solid());
        let m = solid_at(&h);
        h.key_press(egui::Key::ArrowDown);
        h.step();
        assert_eq!(solid_at(&h)[1], m[1] + 1.0);
        key(&mut h, egui::Key::H);
        assert!(!h.state().moving_solid(), "Q's choice back");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Perspective over the page (VIEW-32): the body's top, 1.2 mm over the board, looks larger by the eye's
    /// distance over its distance to the top, the origin stays where it is, the label says the scale holds at the
    /// board plane only, and Measure footprint has no model corner to snap to; orthographic again as before
    #[test]
    fn the_3d_model_in_perspective() {
        let dir = std::env::temp_dir().join(format!("berc-fp-persp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let pdf = datasheet(&dir);
        let mut h = gpu_harness("fp persp");
        h.run();
        let ctx = h.ctx.clone();
        h.state_mut().open_path(&ctx, &fp);
        h.state_mut().open_datasheet(&ctx, &pdf);
        h.state_mut().rt.pdf.as_mut().unwrap().wait_idle(&ctx);
        h.run();
        let st = h.state_mut();
        st.scale = Some(Scale {
            mm_per_point: 0.04,
            page: st.page,
            mm: 2.0,
            note: None,
        });
        st.zoom = 1.0;
        st.pan = [0.0, 0.0];
        st.rt.fit_page = false;
        st.place.origin = [300.0, 400.0];
        st.model_views.opacity = Some(1.0);
        st.rt.settings.fov_deg = 60.0;
        h.run();
        let plain = h.render().unwrap();
        key(&mut h, egui::Key::L);
        steps(&mut h, 4);
        let o = screen(&h, [300.0, 400.0]);
        let model_at = |h: &mut H, x: f32| {
            let img = h.render().unwrap();
            diff(
                pixel(&img, o + vec2(x, 0.0)),
                pixel(&plain, o + vec2(x, 0.0)),
            ) > 40
        };
        // Orthographic: the edge at 2 mm, 50 points
        assert!(model_at(&mut h, 46.0) && !model_at(&mut h, 54.0));
        // The mode bar's switch, right of the side panel's
        let mode_bar = |h: &H, label: &str| {
            use egui_kittest::kittest::NodeT;
            let canvas = h.state().rt.canvas;
            h.query_all_by_label(label)
                .find(|n| {
                    n.accesskit_node().bounding_box().is_some_and(|b| {
                        b.x0 as f32 >= canvas.min.x && (b.y1 as f32) <= canvas.min.y
                    })
                })
                .expect("in the mode bar")
                .click();
        };
        mode_bar(&h, "Perspective");
        steps(&mut h, 3);
        assert!(h.state().model_views.solid.perspective);
        // The eye at half / tan(30°) from the origin, half the picture (1 + tan 30°) times the model's reach
        let t = 30f32.to_radians().tan();
        let d = h.state().rt.solid.radius * (1.0 + t) / t;
        let edge = 50.0 * d / (d - 1.2);
        assert!(edge > 56.0, "{edge}");
        assert!(
            model_at(&mut h, edge - 3.0),
            "the top's edge nearer: {edge}"
        );
        assert!(!model_at(&mut h, edge + 4.0), "nothing beyond it: {edge}");
        // Either side of the origin (its cross drawn on both pictures): the model about it, where it was
        assert!(
            model_at(&mut h, -10.0) && model_at(&mut h, 10.0),
            "the origin where it was"
        );
        // Measure footprint: no corner of the model to snap to
        assert_eq!(h.state().solid_snaps().count(), 0);
        mode_bar(&h, "Perspective");
        steps(&mut h, 3);
        assert!(!h.state().model_views.solid.perspective);
        assert!(model_at(&mut h, 46.0) && !model_at(&mut h, 54.0));
        assert!(h.state().solid_snaps().count() > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Pictures of the review's steps for a person to look at: `BERC_FP_SHOTS` names a folder (skips without). A
    /// two-pin part over a drawing at 3 screen points per page point: a measuring ruler with its + note button,
    /// then added; the 3D model over the page, then in X-ray; the footprint hidden; the page alone; Measure across
    /// from a pad to the drawing; the 3D view, then in X-ray; round 8: the model off the footprint and put back by
    /// Home, in perspective, the model picked under the pointer, hidden with the footprint moving, the 3D view
    /// orthographic.
    #[test]
    fn pictures_of_the_review() {
        let Some(out) = std::env::var_os("BERC_FP_SHOTS").map(PathBuf::from) else {
            eprintln!("BERC_FP_SHOTS not set: no pictures");
            return;
        };
        std::fs::create_dir_all(&out).unwrap();
        let dir = std::env::temp_dir().join(format!("berc-fp-shots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let pdf = datasheet(&dir);
        let mut h = gpu_harness("fp shots");
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
        st.zoom = 3.0;
        st.pan = [-150.0, -330.0];
        st.rt.fit_page = false;
        st.place.origin = [250.0, 230.0];
        st.model_views.opacity = Some(0.8);
        steps(&mut h, 3);
        let shot = |h: &mut H, name: &str| {
            h.render().unwrap().save(out.join(name)).unwrap();
        };
        let drag = |h: &mut H, a: [f32; 2], b: [f32; 2]| {
            let (a, b) = (screen(h, a), screen(h, b));
            h.hover_at(a);
            h.step();
            h.drag_at(a);
            h.step();
            for i in 1..=5 {
                h.hover_at(a + (b - a) * (i as f32 / 5.0));
                h.step();
            }
            h.drop_at(b);
            h.step();
            h.hover_at(b + vec2(40.0, 60.0));
            h.step();
        };
        // Measure PDF along the drawing's dimension line: its + note button beside the label, then added
        key(&mut h, egui::Key::E);
        drag(&mut h, [101.0, 201.0], [199.0, 199.0]);
        shot(&mut h, "1-measure-with-note-button.png");
        h.get_by_label("+ note").click();
        steps(&mut h, 2);
        shot(&mut h, "2-measure-added-to-note.png");
        key(&mut h, egui::Key::Escape);
        // The 3D model over the page, then in X-ray (hidden edges dashed)
        key(&mut h, egui::Key::L);
        steps(&mut h, 4);
        shot(&mut h, "3-model-over-page.png");
        key(&mut h, egui::Key::U);
        shot(&mut h, "4-model-over-page-xray.png");
        // The footprint hidden, the model kept; Space held: the page alone
        key(&mut h, egui::Key::H);
        shot(&mut h, "5-footprint-hidden.png");
        key(&mut h, egui::Key::H);
        let space = |pressed| egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Default::default(),
        };
        h.event(space(true));
        steps(&mut h, 2);
        shot(&mut h, "6-page-alone-space-held.png");
        h.event(space(false));
        steps(&mut h, 2);
        key(&mut h, egui::Key::U);
        key(&mut h, egui::Key::J);
        // Measure across: pad 2's centre to the dimension line's end
        key(&mut h, egui::Key::Y);
        let p2 = h.state().fp_to_page([1.27, 0.0]);
        drag(&mut h, [p2[0] + 1.0, p2[1] + 1.0], [199.0, 201.0]);
        shot(&mut h, "7-measure-across.png");
        h.get_by_label("+ note").click();
        steps(&mut h, 2);
        shot(&mut h, "7a-measure-across-in-note.png");
        key(&mut h, egui::Key::Escape);
        key(&mut h, egui::Key::J);
        // The 3D view, then in X-ray
        key(&mut h, egui::Key::Num3);
        steps(&mut h, 4);
        shot(&mut h, "8-3d-view.png");
        key(&mut h, egui::Key::U);
        steps(&mut h, 2);
        shot(&mut h, "9-3d-view-xray.png");
        key(&mut h, egui::Key::U);
        // Round 8 (VIEW-32), back on the page: the 3D model moved off, then Home puts it onto the footprint
        key(&mut h, egui::Key::Num3);
        steps(&mut h, 2);
        let fp_place = h.state().place.clone();
        h.state_mut().model_views.solid.place = Some(Placement {
            origin: [fp_place.origin[0] + 40.0, fp_place.origin[1] + 25.0],
            rotation: 0.0,
            mirror: false,
        });
        h.state_mut().model_views.solid.turn = side_rotation(Side::Front);
        steps(&mut h, 2);
        shot(&mut h, "10-model-off-the-footprint.png");
        key(&mut h, egui::Key::Home);
        steps(&mut h, 10);
        shot(&mut h, "11-model-on-the-footprint-home.png");
        // Perspective over the page, turned a little so the depth shows
        h.state_mut().rt.settings.fov_deg = 60.0;
        h.state_mut().set_perspective(true);
        h.state_mut().model_views.solid.turn =
            side_rotation(Side::Top) * Quat::from_rotation_x(0.5) * Quat::from_rotation_y(0.3);
        h.state_mut().rt.settings.model_snap.enabled = false;
        steps(&mut h, 3);
        shot(&mut h, "12-model-in-perspective.png");
        h.state_mut().set_perspective(false);
        key(&mut h, egui::Key::Home);
        steps(&mut h, 10);
        // Hovering the model with Q on Model: its outline, and the mode bar names it
        key(&mut h, egui::Key::Q);
        h.hover_at(screen(&h, fp_place.origin) + vec2(4.0, 4.0));
        steps(&mut h, 2);
        shot(&mut h, "13-hover-picks-the-model.png");
        // The model hidden: the switch shows the footprint moving, Model disabled
        key(&mut h, egui::Key::J);
        h.hover_at(screen(&h, fp_place.origin) + vec2(4.0, 4.0));
        steps(&mut h, 2);
        shot(&mut h, "13a-model-hidden-footprint-moves.png");
        key(&mut h, egui::Key::J);
        key(&mut h, egui::Key::Q);
        // The 3D view orthographic
        h.state_mut().rt.settings.view_3d_projection = ve_view3d::Projection::Orthographic;
        key(&mut h, egui::Key::Num3);
        steps(&mut h, 4);
        shot(&mut h, "14-3d-view-orthographic.png");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
