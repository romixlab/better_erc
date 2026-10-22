use crate::BetterErcApp;
use egui::{Align, Layout, Ui};

impl BetterErcApp {
    pub(super) fn menu_bar(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        // NOTE: no File->Quit on web pages!
        let is_web = cfg!(target_arch = "wasm32");
        if !is_web {
            ui.menu_button("File", |ui| {
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.add_space(16.0);
        }

        ui.menu_button("Window", |ui| {
            if ui
                .toggle_value(&mut self.state.debug_window_shown, "Debug")
                .changed()
            {
                ui.close_menu();
            }
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            egui::widgets::global_theme_preference_buttons(ui);
        });
    }
}
