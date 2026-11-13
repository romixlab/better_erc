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
}

impl Default for Solid {
    fn default() -> Self {
        Self {
            place: None,
            turn: side_rotation(Side::Top),
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
        let r = self.rt.solid.radius as f64;
        [[-r, -r], [r, -r], [r, r], [-r, r]]
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
        let r = self.rt.solid.radius;
        let mm_per_point = self.mm_per_point();
        if r <= 0.0 || mm_per_point <= 0.0 {
            return;
        }
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
        view.camera.set_ortho_scale(side / (2.0 * r), side);
        let Some((texture, picture)) = view.render_offscreen(&ctx, vec2(side, side)) else {
            return;
        };
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
        let top_left = corners
            .iter()
            .copied()
            .min_by(|p, q| (p.x + p.y).total_cmp(&(q.x + q.y)))
            .unwrap_or(corners[0]);
        let text = format!(
            "3D model · {}{}",
            self.solid_side_name(),
            if self.moving_solid() { " · moves" } else { "" }
        );
        painter.text(
            top_left - vec2(0.0, 2.0),
            Align2::LEFT_BOTTOM,
            text,
            FontId::proportional(12.0),
            line,
        );
    }

    /// Where Measure footprint snaps on the shaded model: while it sits on a side, the corners of that side's
    /// silhouette turned with it, page points
    pub(super) fn solid_snaps(&self) -> impl Iterator<Item = [f32; 2]> + '_ {
        let on = (self.model_views.look == ModelLook::Solid)
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
        Group::new("Snap")
            .tooltip("What a turn does when the puck or the drag lets go; Settings → Save keeps it")
            .show(ui, |ui| {
                self.rt.settings.model_snap.ui(ui);
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
}
