use crate::BetterErcApp;
use crate::context::Context;
use crate::main_window::State;
use crate::tabs::pcb_data_import::PcbDataImport;
use crate::tabs::{Tab, TabKind, TabUi};

impl BetterErcApp {
    pub(super) fn default_state(cx: &Context) -> State {
        let mut next_view_nr = 0;
        let mut gen_view = |kind: TabKind| {
            let view = Tab {
                kind,
                nr: next_view_nr,
            };
            next_view_nr += 1;
            view
        };

        let mut tiles = egui_tiles::Tiles::default();
        let tabs =
            vec![tiles.insert_pane(gen_view(TabKind::PcbDataImport(PcbDataImport::new(cx))))];
        let root = tiles.insert_tab_tile(tabs);
        let open_tabs = egui_tiles::Tree::new("main_window_tile_tree", root, tiles);

        State {
            tabs: open_tabs,
            tabs_behavior: Default::default(),
            debug_window_shown: false,
        }
    }
}
