//! `TEST` - testability: unconnected pins, test points per net, rails and bring-up buses without test points,
//! on the probe access of [`erc_analysis::access`]

mod test_1;
mod test_2;
mod test_4;
mod test_5;

use crate::Ctx;
use erc_model::{PartClass, PartId};

/// The test points of the board, fitted or not (a test point is a footprint, often left out of the BOM)
pub(crate) fn test_points(cx: &Ctx) -> Vec<PartId> {
    cx.components()
        .filter(|p| cx.classes().get(*p) == PartClass::TestPoint)
        .collect()
}
