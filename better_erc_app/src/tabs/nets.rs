use crate::prelude::*;
use crate::tabs::TabUi;

#[derive(Default, Serialize, Deserialize)]
pub struct Nets {
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

impl TabUi for Nets {
    fn init(&mut self, _cx: &Context) {
        self.transient = Some(Transient {});
    }

    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, _id: Id) {
        let Some(_t) = &mut self.transient else {
            return;
        };
        let s = cx.blocking_read();
        // TODO: switch to TableView for nets
        ScrollArea::vertical().show(ui, |ui| {
            for board in &s.boards {
                ui.heading("Power rails:");
                for net_name in board.power.power_rails.keys() {
                    ui.label(net_name.0.as_str());
                }
                ui.heading("Ground nets:");
                for net_name in &board.power.ground_nets {
                    ui.label(net_name.0.as_str());
                }
                ui.heading("Switching nodes");
                for net_name in &board.switching_nodes {
                    ui.label(net_name.0.as_str());
                }
                ui.heading("All nets");
                for (net_name, net) in &board.netlist.nets {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(net_name.0.as_str())
                                .monospace()
                                // .size(18.0)
                                .strong(),
                        );
                        ui.label(format!("{}", net.nodes.len()));
                    });
                }
            }
        });
    }
}
