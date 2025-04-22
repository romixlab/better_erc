use crate::prelude::*;
use egui_tiles::{SimplificationOptions, Tile, TileId, Tiles, UiResponse};
use std::fmt::Debug;

pub mod nets;
pub mod pcb_data_import;

#[derive(Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter, AsRefStr))]
#[strum_discriminants(name(TabKind))]
pub enum Tab {
    PcbDataImport(pcb_data_import::PcbDataImport),
    Nets(nets::Nets),
}

pub trait TabUi {
    fn init(&mut self, cx: &Context);
    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, id: Id);
}

impl Tab {
    pub fn ui(&mut self, cx: &mut Context, ui: &mut Ui, tile_id: TileId) -> UiResponse {
        let id = Id::new("Tab").with(tile_id);
        egui::Frame::NONE
            .inner_margin(4.0)
            .show(ui, |ui| match self {
                Tab::PcbDataImport(t) => t.ui(ui, cx, id),
                Tab::Nets(t) => t.ui(ui, cx, id),
            });
        // if dragged {
        //     UiResponse::DragStarted
        // } else {
        //     UiResponse::None
        // }
        UiResponse::None
    }

    pub fn init(&mut self, cx: &Context) {
        match self {
            Tab::PcbDataImport(t) => t.init(cx),
            Tab::Nets(t) => t.init(cx),
        }
    }

    pub fn title(&self) -> WidgetText {
        match self {
            Tab::PcbDataImport(_t) => "PCB Data Import",
            Tab::Nets(_t) => "Nets",
        }
        .into()
    }

    fn is_closeable(&self) -> bool {
        true
    }
}

pub struct TreeBehavior {
    simplification_options: SimplificationOptions,
    tab_bar_height: f32,
    gap_width: f32,
    // pub(crate) add_child_to: Option<TileId>,
    cx: Option<Context>,
}

impl Default for TreeBehavior {
    fn default() -> Self {
        Self {
            simplification_options: SimplificationOptions {
                prune_empty_tabs: true,
                prune_empty_containers: true,
                prune_single_child_tabs: false,
                prune_single_child_containers: true,
                all_panes_must_have_tabs: true,
                join_nested_linear_containers: true,
            },
            tab_bar_height: 20.0,
            gap_width: 2.0,
            // add_child_to: None,
            cx: None,
        }
    }
}

impl TreeBehavior {
    pub(crate) fn ui(&mut self, ui: &mut Ui) {
        let Self {
            simplification_options,
            tab_bar_height,
            gap_width,
            // add_child_to: _,
            cx: _,
        } = self;

        Grid::new("behavior_ui").num_columns(2).show(ui, |ui| {
            ui.label("All panes must have tabs:");
            ui.checkbox(&mut simplification_options.all_panes_must_have_tabs, "");
            ui.end_row();

            ui.label("Join nested containers:");
            ui.checkbox(
                &mut simplification_options.join_nested_linear_containers,
                "",
            );
            ui.end_row();

            ui.label("Tab bar height:");
            ui.add(DragValue::new(tab_bar_height).range(0.0..=100.0).speed(1.0));
            ui.end_row();

            ui.label("Gap width:");
            ui.add(DragValue::new(gap_width).range(0.0..=20.0).speed(1.0));
            ui.end_row();
        });
    }

    pub fn feed_cx(&mut self, cx: Context) {
        self.cx = Some(cx);
    }
}

impl egui_tiles::Behavior<Tab> for TreeBehavior {
    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, view: &mut Tab) -> UiResponse {
        if let Some(cx) = &mut self.cx {
            view.ui(cx, ui, tile_id)
        } else {
            UiResponse::None
        }
    }

    fn tab_title_for_pane(&mut self, view: &Tab) -> WidgetText {
        view.title()
    }

    fn is_tab_closable(&self, tiles: &Tiles<Tab>, tile_id: TileId) -> bool {
        if let Some(tile) = tiles.get(tile_id) {
            match tile {
                Tile::Pane(tab) => tab.is_closeable(),
                Tile::Container(_) => false,
            }
        } else {
            true
        }
    }

    fn on_tab_close(&mut self, tiles: &mut Tiles<Tab>, tile_id: TileId) -> bool {
        if let Some(tile) = tiles.get(tile_id) {
            match tile {
                Tile::Pane(pane) => {
                    // Single pane removal
                    let tab_title = self.tab_title_for_pane(pane);
                    debug!("Closing tab: {}, tile ID: {tile_id:?}", tab_title.text());
                }
                Tile::Container(container) => {
                    // Container removal
                    debug!("Closing container: {:?}", container.kind());
                    let children_ids = container.children();
                    for child_id in children_ids {
                        if let Some(Tile::Pane(pane)) = tiles.get(*child_id) {
                            let tab_title = self.tab_title_for_pane(pane);
                            debug!("Closing tab: {}, tile ID: {tile_id:?}", tab_title.text());
                        }
                    }
                }
            }
        }

        // Proceed to removing the tab
        true
    }

    // fn top_bar_right_ui(
    //     &mut self,
    //     _tiles: &Tiles<Tab>,
    //     ui: &mut Ui,
    //     tile_id: TileId,
    //     _tabs: &egui_tiles::Tabs,
    //     _scroll_offset: &mut f32,
    // ) {
    //     ui.add_space(4.0);
    //     if ui.button("➕").clicked() {
    //         self.add_child_to = Some(tile_id);
    //     }
    // }

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        self.tab_bar_height
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        self.gap_width
    }

    fn simplification_options(&self) -> SimplificationOptions {
        self.simplification_options
    }
}
