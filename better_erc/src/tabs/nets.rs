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

    fn ui(&mut self, _ui: &mut Ui, _cx: &mut Context, _id: Id) {
        let Some(_t) = &mut self.transient else {
            return;
        };
    }
}
