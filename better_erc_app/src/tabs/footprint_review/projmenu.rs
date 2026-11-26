//! The model's right-click menu (VIEW-38): a right click on the 3D model or on one of its projections offers Add
//! projection (top, bottom, front, back, left, right), each added under the pointer as it picks the side, so a drag
//! from there lays it beside the datasheet's drawings; on a projection it also offers to take that one off.
//!
//! A double right click brings the model to the pointer (VIEW-32), so the menu waits for egui's double click time
//! after the first right click: a second one in that time is the double right click, and no menu opens.

use super::grab::Grab;
use super::modelviews::{Side, VIEWS_MAX};
use super::*;

/// Where the menu was asked for
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MenuAt {
    /// The screen point of the right click: the menu's top left corner
    screen: Pos2,
    /// What the right click was on
    on: Grab,
}

/// The menu's state in the tab, never saved
#[derive(Clone, Debug, Default)]
pub(super) struct MenuRt {
    /// A right click on the model and its time, waiting out the double click time
    pending: Option<(f64, MenuAt)>,
    /// The menu shown
    open: Option<MenuAt>,
}

impl FootprintReview {
    /// Each frame on the canvas: a right click on the model at page point `p` asks for the menu, which opens
    /// once the double click time is over without a second one; the menu shown while it is open
    pub(super) fn model_menu(
        &mut self,
        ctx: &egui::Context,
        resp: &egui::Response,
        p: Option<[f32; 2]>,
        id: Id,
    ) {
        let now = ctx.input(|i| i.time);
        let delay = ctx.options(|o| o.input_options.max_double_click_delay);
        let menu = &mut self.rt.model_menu;
        if resp.double_clicked_by(egui::PointerButton::Secondary) {
            menu.pending = None;
            menu.open = None;
        } else if resp.clicked_by(egui::PointerButton::Secondary)
            && let (Some(page), Some(screen)) = (p, resp.interact_pointer_pos())
        {
            let on = self.grab_at(page);
            let menu = &mut self.rt.model_menu;
            menu.open = None;
            if matches!(on, Grab::View(_) | Grab::Solid) {
                menu.pending = Some((now, MenuAt { screen, on }));
            }
        }
        let menu = &mut self.rt.model_menu;
        if let Some((t, at)) = menu.pending {
            if now - t > delay {
                menu.open = Some(at);
                menu.pending = None;
            } else {
                ctx.request_repaint_after_secs((t + delay - now) as f32 + 0.01);
            }
        }
        if let Some(at) = menu.open {
            self.show_model_menu(ctx, at, id);
        }
    }

    /// The menu at `at`; closed by a pick, Escape or a press outside it
    fn show_model_menu(&mut self, ctx: &egui::Context, at: MenuAt, id: Id) {
        let full = self.model_views.views.len() >= VIEWS_MAX;
        let area = egui::Area::new(id.with("model menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(at.screen)
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style())
                    .show(ui, |ui| self.model_menu_items(ui, at, full))
                    .inner
            });
        let outside = ctx.input(|i| {
            i.pointer.any_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area.response.rect.contains(p))
        });
        if area.inner || outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.rt.model_menu.open = None;
        }
    }

    /// The menu's items; true once one is picked
    fn model_menu_items(&mut self, ui: &mut Ui, at: MenuAt, full: bool) -> bool {
        let mut picked = false;
        ui.label(RichText::new("Add projection").strong());
        for side in Side::ALL {
            let lower = side.name().to_lowercase();
            let b = ui
                .add_enabled(!full, egui::Button::new(format!("{} view", side.name())))
                .on_hover_text(format!(
                    "The model seen from the {lower}, added under the pointer at the page's scale; a drag places it"
                ))
                .on_disabled_hover_text(format!(
                    "The page holds {VIEWS_MAX} views, the most it takes: take one off first"
                ));
            if b.clicked() {
                let under = ui.input(|i| i.pointer.interact_pos());
                let p = under.map(|s| self.to_page(self.rt.canvas, s));
                if let Some(i) = self.add_projection(side, p) {
                    self.rt.aim = Grab::View(i);
                }
                picked = true;
            }
        }
        if let Grab::View(i) = at.on
            && let Some(v) = self.model_views.views.get(i)
        {
            ui.separator();
            let name = format!("Remove this {} view", v.side.name().to_lowercase());
            if ui
                .button(name)
                .on_hover_text("This projection off the page; the others stay")
                .clicked()
            {
                self.remove_projection(i);
                picked = true;
            }
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::super::model3d::tests::two_pin;
    use super::super::modelviews::ModelLook;
    use super::super::tests::{datasheet, harness};
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    type H = Harness<'static, FootprintReview>;

    fn screen(h: &H, p: [f32; 2]) -> Pos2 {
        let st = h.state();
        st.to_screen(st.rt.canvas, p)
    }

    fn page(h: &H, p: Pos2) -> [f32; 2] {
        let st = h.state();
        st.to_page(st.rt.canvas, p)
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
    }

    /// `clicks` right clicks at `a`, 50 ms apart from time `t0` (s), then the clock past the double click time:
    /// the harness's clock stands still, so the times are its own
    fn right_clicks(h: &mut H, a: Pos2, clicks: usize, t0: f64) {
        h.hover_at(a);
        h.run();
        let mut t = t0;
        for _ in 0..clicks {
            for pressed in [true, false] {
                h.input_mut().time = Some(t);
                h.event(egui::Event::PointerButton {
                    pos: a,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
                h.step();
                t += 0.05;
            }
        }
        h.input_mut().time = Some(t + 1.0);
        h.step();
        h.run();
    }

    fn drag(h: &mut H, a: Pos2, b: Pos2) {
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
    }

    /// A right click on the 3D model opens Add projection once the double click time is over; each side added
    /// lies where the click was, a drag places it, the 3D model staying; a right click on a projection takes it
    /// off. A double right click brings the model to the pointer and opens no menu
    #[test]
    fn right_click_the_model_to_add_and_remove_projections() {
        let dir = std::env::temp_dir().join(format!("berc-fp-projmenu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let fp = two_pin(&dir);
        let pdf = datasheet(&dir);
        let mut h = harness("fp projection menu");
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
        st.rt.center_footprint = false;
        st.rt.fp_hidden = true;
        st.set_look(ModelLook::Solid);
        h.run();
        h.run();
        assert!(h.state().projected(Side::Top).is_some(), "the model read");
        let solid = h.state().model_views.solid.place.clone().unwrap();

        // A single right click on the 3D model: no menu until the double click time is over, then Add projection
        let on_model = screen(&h, [solid.origin[0] + 10.0, solid.origin[1]]);
        right_clicks(&mut h, on_model, 1, 100.0);
        assert!(h.state().rt.model_menu.open.is_some(), "the menu opens");
        assert!(h.query_by_label("Remove this front view").is_none());
        let pick = h.get_by_label("Front view").rect().center();
        h.get_by_label("Front view").click();
        h.run();
        h.run();
        let st = h.state();
        assert!(st.rt.model_menu.open.is_none(), "a pick closes it");
        assert_eq!(st.model_views.views.len(), 1);
        assert_eq!(st.model_views.views[0].side, Side::Front);
        assert_eq!(st.aimed(), Grab::View(0), "the keys act on the view added");
        // Its middle under the pointer as it picked the side
        let (lo, hi) = st.projected(Side::Front).unwrap().extent.unwrap();
        let place = st.model_views.views[0].place.clone();
        let mid = st.placed(&place, [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]);
        assert!(near(mid, page(&h, pick)), "{mid:?}");

        // Dragged beside the model: the view moves, the 3D model stays
        let to = pick + vec2(80.0, 60.0);
        drag(&mut h, pick, to);
        let st = h.state();
        let moved = st.model_views.views[0].place.clone();
        assert!(
            near(
                moved.origin,
                [place.origin[0] + 40.0, place.origin[1] + 30.0]
            ),
            "{:?}",
            moved.origin
        );
        assert_eq!(st.model_views.solid.place.as_ref(), Some(&solid));

        // A right click on the front view: Add projection there too, and Remove this front view; a top view added
        right_clicks(&mut h, to, 1, 200.0);
        assert!(h.query_by_label("Remove this front view").is_some());
        h.get_by_label("Top view").click();
        h.run();
        assert_eq!(h.state().model_views.views.len(), 2);
        assert_eq!(h.state().model_views.views[1].side, Side::Top);
        // The top view dragged off the front one, then the front one taken off
        drag(&mut h, to, to + vec2(0.0, 120.0));
        right_clicks(&mut h, to + vec2(-30.0, 0.0), 1, 300.0);
        h.get_by_label("Remove this front view").click();
        h.run();
        let st = h.state();
        assert_eq!(
            st.model_views
                .views
                .iter()
                .map(|v| v.side)
                .collect::<Vec<_>>(),
            [Side::Top]
        );
        assert!(st.rt.model_menu.open.is_none());

        // Escape closes the menu with nothing added
        right_clicks(&mut h, on_model, 1, 400.0);
        assert!(h.state().rt.model_menu.open.is_some());
        h.key_press(egui::Key::Escape);
        h.run();
        assert!(h.state().rt.model_menu.open.is_none());
        assert_eq!(h.state().model_views.views.len(), 1);

        // A double right click: the 3D model to the pointer, no menu
        let there = on_model + vec2(-60.0, -40.0);
        right_clicks(&mut h, there, 2, 500.0);
        let st = h.state();
        assert!(
            st.rt.model_menu.open.is_none(),
            "no menu on a double right click"
        );
        assert!(
            near(
                st.model_views.solid.place.as_ref().unwrap().origin,
                page(&h, there)
            ),
            "the model brought to the pointer"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
