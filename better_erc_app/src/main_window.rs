mod default_state;
#[cfg(test)]
mod look_tests;
mod menu_bar;
mod side_panel;

use crate::prelude::*;
use crate::tabs::{Tab, TreeBehavior};
use egui::{Panel, Window};
use egui_tiles::Tile;

pub struct BetterErcApp {
    cx: Context,
    state: State,
}

#[derive(Serialize, Deserialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
struct State {
    tabs: egui_tiles::Tree<Tab>,
    #[serde(skip)]
    tabs_behavior: TreeBehavior,
    debug_window_shown: bool,
}

impl BetterErcApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create context, message bus and launch UI re-painter
        let cx = Context::new();
        // let egui_cx = cc.egui_ctx.clone();
        // tokio::spawn(async move {
        //     re_painter(egui_cx, bus_rx).await;
        // });

        // Load previous app state (if any).
        let mut state: State = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_else(|| {
                info!("Default state created, because deserializing failed");
                Default::default()
            })
        } else {
            info!("Default state created, because persistence is disabled");
            Default::default()
        };

        // Restore context for tabs
        state.tabs_behavior.feed_cx(cx.clone());

        // Init tabs
        for (_id, tile) in state.tabs.tiles.iter_mut() {
            if let Tile::Pane(tab) = tile {
                tab.init(&cx);
            }
        }

        // The footprint review's blend pass, for the renderer eframe made (main.rs: no multisampling, no depth buffer)
        if let Some(rs) = cc.wgpu_render_state.as_ref() {
            crate::tabs::footprint_review::blend::register(&cc.egui_ctx, rs, 1, None);
            crate::tabs::footprint_review::model3d::register(&cc.egui_ctx, rs);
        }

        setup_look(&cc.egui_ctx);

        Self { cx, state }
    }

    /// The app's frame: the menu bar, the side panel, the debug window and the tabs
    fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        Panel::top("top_panel").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                self.menu_bar(&ctx, ui);
            });
        });

        Panel::left("side_panel").resizable(true).show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                self.side_panel(ui);
            });
        });

        if self.state.debug_window_shown {
            Window::new("Debug")
                .open(&mut self.state.debug_window_shown)
                .collapsible(true)
                .scroll([true, true])
                .show(&ctx, |ui| {
                    crate::windows::debug::DebugWindow {}.ui(
                        &mut self.state.tabs,
                        &mut self.state.tabs_behavior,
                        ui,
                    );
                });
        }

        egui::CentralPanel::default().show(ui, |ui| {
            self.state.tabs.ui(&mut self.state.tabs_behavior, ui);
        });
    }
}

/// The app's fonts and style: the vhrd look (ve_theme: IBM Plex, the brand's dark and light styles, egui following
/// the system's choice) with the phosphor icon fonts behind Plex
pub fn setup_look(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
    ctx.set_fonts(fonts);
    ve_theme::setup(ctx);
}

impl eframe::App for BetterErcApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }
}
