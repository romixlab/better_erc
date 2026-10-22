//! `DEC` - decoupling and datasheet passives, on the rails of [`erc_analysis::power`] and the supply pins and
//! capacitors of [`erc_analysis::supply`]

mod dec_1;
mod dec_10;
mod dec_11;
mod dec_12;
mod dec_4;
mod dec_7;
mod dec_8;

use crate::Ctx;
use erc_analysis::supply::caps_to_ground;
use erc_model::{NetId, PartId};

pub(crate) use crate::text::{designators, pin_list};

/// The fitted capacitors from a net to a ground in the variant checked
fn caps(cx: &Ctx, net: NetId) -> Vec<PartId> {
    caps_to_ground(cx.view(), cx.classes(), cx.power(), net)
}
