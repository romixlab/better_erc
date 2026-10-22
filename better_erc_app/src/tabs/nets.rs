use crate::prelude::*;
use crate::tabs::TabUi;
use better_erc::analysis::graph::net_degree;
use better_erc::analysis::value::format_si;
use better_erc::model::Unit;

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
            for (board, a) in s.boards() {
                let name = |n: &better_erc::model::NetId| board.net(*n).name.as_str();
                ui.heading("Power rails:");
                for (net, voltage) in &a.power.rails {
                    match voltage.value {
                        Some(v) => {
                            ui.label(format!("{} ({})", name(net), format_si(v, Unit::Volt)))
                        }
                        None => ui.label(name(net)),
                    };
                }
                ui.heading("Ground nets:");
                for net in &a.power.grounds {
                    ui.label(name(net));
                }
                ui.heading("Switching nodes");
                for net in &a.power.switching_nodes {
                    ui.label(name(net));
                }
                ui.heading("All nets");
                for (i, net) in board.nets.iter().enumerate() {
                    let id = better_erc::model::NetId::from_index(i);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(net.name.as_str())
                                .monospace()
                                // .size(18.0)
                                .strong(),
                        );
                        ui.label(format!("{}", net_degree(board, id)));
                    });
                }
            }
        });
    }
}
