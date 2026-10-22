use crate::prelude::*;
use crate::tabs::TabUi;

#[derive(Default, Serialize, Deserialize)]
pub struct I2C {
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

impl TabUi for I2C {
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
                for d in &board.diagnostics.i2c {
                    ui.colored_label(Color32::ORANGE, format!("{:?}", d));
                }
                ui.label(format!("{:#?}", board.i2c_buses));
            }
        });
    }
}
