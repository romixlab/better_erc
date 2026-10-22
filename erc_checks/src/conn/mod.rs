//! `CONN` - connectors: IC pins straight on connector pins, connectors without ground or with few grounds, micro
//! HDMI wired as type A, floating shells, look-alike connectors with incompatible pinouts; shell pins by name
//! ([`erc_analysis::connector`])

mod conn_10;
mod conn_16;
mod conn_17;
mod conn_2;
mod conn_4;
mod conn_8;

use crate::Ctx;
use crate::usb::connected;
use erc_analysis::connector::is_shell_pin;
use erc_model::{NetId, PartId, PinId};

/// The fitted connectors
pub(crate) fn connectors(cx: &Ctx) -> Vec<PartId> {
    cx.view()
        .fitted_components()
        .filter(|p| cx.classes().get(*p).is_connector())
        .collect()
}

/// A connector's contact pins (no shell or mounting pins) that are connected, with their nets
pub(crate) fn contacts(cx: &Ctx, part: PartId) -> Vec<(PinId, NetId)> {
    let b = cx.board();
    b.part(part)
        .pins
        .iter()
        .filter(|p| !is_shell_pin(b.pin(**p)))
        .filter_map(|p| Some((*p, connected(cx, *p)?)))
        .collect()
}

/// A pin number as a position along the connector (`12`), when it is one
pub(crate) fn position(cx: &Ctx, pin: PinId) -> Option<u32> {
    cx.board().pin(pin).number.parse().ok()
}
