use crate::prelude::*;
use crate::tabs::TabUi;

#[derive(Default, Serialize, Deserialize)]
pub struct Style {
    #[serde(skip)]
    transient: Option<Transient>,
}

struct Transient {}

impl TabUi for Style {
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
                for d in &board.diagnostics.style {
                    ui.colored_label(Color32::ORANGE, format!("{:?}", d));
                }
                if board.diagnostics.style.is_empty() {
                    ui.label("No style warnings found");
                }
            }
        });
    }
}
