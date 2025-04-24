use crate::prelude::*;
use crate::tabs::TabUi;

#[derive(Default, Serialize, Deserialize)]
pub struct Inputs {
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

impl TabUi for Inputs {
    fn init(&mut self, _cx: &Context) {
        self.transient = Some(Transient {});
    }

    fn ui(&mut self, ui: &mut Ui, cx: &mut Context, _id: Id) {
        let Some(_t) = &mut self.transient else {
            return;
        };
        let s = cx.blocking_read();
        ScrollArea::vertical().show(ui, |ui| {
            for board in &s.boards {
                for (part_key, part) in &board.netlist.lib_parts {
                    for (pin_id, pin) in &part.pins {
                        if pin.default_mode.ty.is_input() {
                            ui.label(format!(
                                "{}: {:?} {:?} {:?}",
                                part_key.1.0, pin.default_mode.ty, pin_id, pin.name
                            ));
                            for (d, _c) in board
                                .netlist
                                .components
                                .iter()
                                .filter(|(_d, c)| c.lib_source == *part_key)
                            {
                                ui.label(format!("{d:?}"));
                            }
                            ui.separator();
                        }
                    }
                }
            }
        });
    }
}
