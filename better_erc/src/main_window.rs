mod default_state;
mod menu_bar;
mod side_panel;

use crate::prelude::*;
use crate::tabs::{Tab, TreeBehavior};
use egui::{SidePanel, Window};
use egui_tiles::Tile;

pub struct BetterErcApp {
    cx: Context,
    state: State,
}

#[derive(Serialize, Deserialize)]
// #[serde(default)] // if we add new fields, give them default values when deserializing old state
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
        let mut state = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_else(|| {
                info!("Default state created, because deserializing failed");
                Self::default_state(&cx)
            })
        } else {
            info!("Default state created, because persistence is disabled");
            Self::default_state(&cx)
        };

        // Restore context for tabs
        state.tabs_behavior.feed_cx(cx.clone());

        // Init tabs
        for (_id, tile) in state.tabs.tiles.iter_mut() {
            if let Tile::Pane(tab) = tile {
                tab.init(&cx);
            }
        }

        // Init fonts
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
        cc.egui_ctx.set_fonts(fonts);

        Self { cx, state }
    }
}

impl eframe::App for BetterErcApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.menu_bar(ctx, ui);
            });
        });

        SidePanel::left("side_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.side_panel(ui);
                });
            });

        if self.state.debug_window_shown {
            Window::new("Debug")
                .open(&mut self.state.debug_window_shown)
                .collapsible(true)
                .scroll([true, true])
                .show(ctx, |ui| {
                    crate::windows::debug::DebugWindow {}.ui(
                        &mut self.state.tabs,
                        &mut self.state.tabs_behavior,
                        ui,
                    );
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.state.tabs.ui(&mut self.state.tabs_behavior, ui);
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }
}
