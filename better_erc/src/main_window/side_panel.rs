use crate::BetterErcApp;
use crate::prelude::*;
use crate::tabs::{Tab, TabKind, TabKindDiscriminants};
use egui_tiles::{Container, Tile};

impl BetterErcApp {
    pub(super) fn side_panel(&mut self, ui: &mut Ui) {
        ui.add_space(8.0);
        for tab_kind in TabKindDiscriminants::iter() {
            let id = self
                .state
                .tabs
                .tiles
                .iter()
                .find_map(|(id, tile)| match tile {
                    Tile::Pane(tab) => {
                        if TabKindDiscriminants::from(&tab.kind) == tab_kind {
                            Some(id.clone())
                        } else {
                            None
                        }
                    }
                    Tile::Container(_) => None,
                });
            let text = match tab_kind {
                TabKindDiscriminants::PcbDataImport => "PCB Data Import",
                TabKindDiscriminants::Nets => "Nets",
            };
            let mut is_open = id
                .map(|id| self.state.tabs.tiles.is_visible(id))
                .unwrap_or(false);
            if ui.toggle_value(&mut is_open, text).changed() {
                if let Some(id) = id {
                    let is_visible = self.state.tabs.is_visible(id);
                    self.state.tabs.set_visible(id, !is_visible);
                    if let Some(r) = self.state.tabs.root {
                        if let Some(Tile::Container(Container::Tabs(tabs))) =
                            self.state.tabs.tiles.get_mut(r)
                        {
                            tabs.set_active(id);
                        }
                    }
                    debug!("Changed visible");
                } else {
                    let kind = match tab_kind {
                        TabKindDiscriminants::PcbDataImport => {
                            TabKind::PcbDataImport(Default::default())
                        }
                        TabKindDiscriminants::Nets => TabKind::Nets(Default::default()),
                    };
                    let nr = self
                        .state
                        .tabs
                        .tiles
                        .iter()
                        .filter_map(|(_, t)| match t {
                            Tile::Pane(tab) => Some(tab.nr),
                            Tile::Container(_) => None,
                        })
                        .max()
                        .unwrap_or(0)
                        + 1;
                    let tab = Tab { kind, nr };
                    if let Some(r) = self.state.tabs.root {
                        let new_child = self.state.tabs.tiles.insert_pane(tab);
                        // self.state
                        //     .tabs
                        //     .move_tile_to_container(id, r, usize::MAX, true);
                        if let Some(Tile::Container(Container::Tabs(tabs))) =
                            self.state.tabs.tiles.get_mut(r)
                        {
                            tabs.add_child(new_child);
                            tabs.set_active(new_child);
                        }
                    } else {
                        debug!("No root");
                        let mut tiles = egui_tiles::Tiles::default();
                        let tabs = vec![tiles.insert_pane(tab)];
                        let root = tiles.insert_tab_tile(tabs);
                        self.state.tabs =
                            egui_tiles::Tree::new("main_window_tile_tree", root, tiles);
                    }
                    debug!("Created new");
                }
            }
        }
    }
}
