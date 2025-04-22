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
        for board in &s.boards {
            ScrollArea::vertical().show(ui, |ui| {
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
            });
        }
    }
}
