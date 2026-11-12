use crate::prelude::*;
use crate::tabs::TabUi;

/// Schematic vs layout footprint review: planned, nothing to show yet (VIEW-19)
#[derive(Default, Serialize, Deserialize)]
pub struct FootprintReview {}

impl TabUi for FootprintReview {
    fn init(&mut self, _cx: &Context) {}

    fn ui(&mut self, ui: &mut Ui, _cx: &mut Context, _id: Id) {
        ui.label("Footprint review: nothing to show yet (VIEW-19)");
    }
}
