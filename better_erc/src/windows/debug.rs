use crate::tabs::{Tab, TreeBehavior};
use egui::Ui;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct DebugWindow {}

impl DebugWindow {
    pub fn ui(
        &mut self,
        tabs: &mut egui_tiles::Tree<Tab>,
        tabs_behavior: &mut TreeBehavior,
        ui: &mut Ui,
    ) {
        tabs_behavior.ui(ui);

        ui.collapsing("Tree", |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let tree_debug = format!("{:#?}", tabs);
            ui.monospace(&tree_debug);
        });

        ui.separator();

        ui.collapsing("Active tiles", |ui| {
            let active = tabs.active_tiles();
            for tile_id in active {
                use egui_tiles::Behavior as _;
                let name = tabs_behavior.tab_title_for_tile(&tabs.tiles, tile_id);
                ui.label(format!("{} - {tile_id:?}", name.text()));
            }
        });

        ui.separator();

        if let Some(root) = tabs.root() {
            tree_ui(ui, tabs_behavior, &mut tabs.tiles, root);
        }
    }
}

pub(crate) fn tree_ui(
    ui: &mut Ui,
    behavior: &mut dyn egui_tiles::Behavior<Tab>,
    tiles: &mut egui_tiles::Tiles<Tab>,
    tile_id: egui_tiles::TileId,
) {
    // Get the name BEFORE we remove the tile below!
    let text = format!(
        "{} - {tile_id:?}",
        behavior.tab_title_for_tile(tiles, tile_id).text()
    );

    // Temporarily remove the tile to circumvent the borrowchecker
    let Some(mut tile) = tiles.remove(tile_id) else {
        log::debug!("Missing tile {tile_id:?}");
        return;
    };

    let default_open = true;
    egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        ui.id().with((tile_id, "tree")),
        default_open,
    )
        .show_header(ui, |ui| {
            ui.label(text);
            let mut visible = tiles.is_visible(tile_id);
            ui.checkbox(&mut visible, "Visible");
            tiles.set_visible(tile_id, visible);
        })
        .body(|ui| match &mut tile {
            egui_tiles::Tile::Pane(_) => {}
            egui_tiles::Tile::Container(container) => {
                let mut kind = container.kind();
                egui::ComboBox::from_label("Kind")
                    .selected_text(format!("{kind:?}"))
                    .show_ui(ui, |ui| {
                        for typ in egui_tiles::ContainerKind::ALL {
                            ui.selectable_value(&mut kind, typ, format!("{typ:?}"))
                                .clicked();
                        }
                    });
                if kind != container.kind() {
                    container.set_kind(kind);
                }

                for &child in container.children() {
                    tree_ui(ui, behavior, tiles, child);
                }
            }
        });

    // Put the tile back
    tiles.insert(tile_id, tile);
}
