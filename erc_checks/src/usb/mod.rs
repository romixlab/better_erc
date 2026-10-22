//! `USB` - USB and Type-C: CC terminations, shield terminations, CC pins tied, Rd on a plug's VCONN, USB 2.0 rows,
//! VBUS and ground contacts; Type-C contacts by number or name ([`erc_analysis::connector`])

mod usb_1;
mod usb_10;
mod usb_11;
mod usb_16;
mod usb_4;
mod usb_9;

use crate::Ctx;
use erc_analysis::connector::{TypeCPin, is_plug, type_c_pins};
use erc_analysis::graph::net_degree;
use erc_model::class::ConnectorKind;
use erc_model::{NetId, PartClass, PartId, PinId};

/// The fitted Type-C connectors: receptacles (`plugs` false) or plugs
pub(crate) fn type_c(cx: &Ctx, plugs: bool) -> Vec<PartId> {
    cx.view()
        .fitted_components()
        .filter(|p| {
            cx.classes().get(*p)
                == PartClass::Connector {
                    kind: Some(ConnectorKind::UsbC),
                }
        })
        .filter(|p| is_plug(cx.board(), *p) == plugs)
        .collect()
}

/// The one pin of a Type-C connector with `role` (the first, where a symbol has it twice)
pub(crate) fn pin(cx: &Ctx, part: PartId, role: TypeCPin) -> Option<PinId> {
    type_c_pins(cx.board(), part, role).into_iter().next()
}

/// A pin's net when the pin is connected to something: no no-connect flag, on a net with another component pin
pub(crate) fn connected(cx: &Ctx, pin: PinId) -> Option<NetId> {
    let b = cx.board();
    let p = b.pin(pin);
    if p.no_connect {
        return None;
    }
    p.net.filter(|n| net_degree(b, *n) >= 2)
}
