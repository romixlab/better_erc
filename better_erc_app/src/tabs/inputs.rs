use crate::prelude::*;
use crate::tabs::TabUi;
use better_erc::model::EcadPinType;

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
            for (board, _) in s.boards() {
                for (i, symbol) in board.symbols.iter().enumerate() {
                    for pin in &symbol.pins {
                        if !matches!(pin.etype, EcadPinType::Input | EcadPinType::PowerIn) {
                            continue;
                        }
                        ui.label(format!(
                            "{}: {} {} {}",
                            symbol.name,
                            pin.etype.name(),
                            pin.number,
                            pin.name
                        ));
                        for part in board
                            .parts
                            .iter()
                            .filter(|p| p.symbol.is_some_and(|s| s.index() == i))
                        {
                            ui.label(part.designator.as_str());
                        }
                        ui.separator();
                    }
                }
            }
        });
    }
}
