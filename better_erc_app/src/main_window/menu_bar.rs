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
                ui.close();
            }
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            egui::widgets::global_theme_preference_buttons(ui);
            self.colors_menu(ui);
        });
    }
}

impl BetterErcApp {
    /// Where the colours come from (the desktop theme, the brand, a fixed accent), saved in the settings file at once
    fn colors_menu(&mut self, ui: &mut Ui) {
        let Some(look) = &self.look else {
            ui.add_enabled(false, egui::Button::new("Colours"))
                .on_disabled_hover_text("Not in this run: the brand colours are fixed (a test)");
            return;
        };
        ui.menu_button("Colours", |ui| {
            if let Some(source) = look.ui(ui) {
                let file = crate::settings::Settings::file();
                self.colors_note = Some(match &file {
                    None => "not saved: neither BERC_CONFIG, XDG_CONFIG_HOME nor HOME is set".into(),
                    Some(f) => {
                        let (mut all, _) = crate::settings::Settings::load_from(f);
                        all.colors = source;
                        match all.save_to(f) {
                            Ok(()) => format!("saved to {}", f.display()),
                            Err(e) => e,
                        }
                    }
                });
            }
            if let Some(note) = &self.colors_note {
                ui.weak(note);
            }
        })
        .response
        .on_hover_text("Where the colours come from: the desktop theme, the vhrd brand, or an accent of your own");
    }
}
