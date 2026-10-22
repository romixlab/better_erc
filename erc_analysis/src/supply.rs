//! Supply pins and their capacitors: the power-input pins of the fitted parts on a net and the fitted capacitors
//! from a net to a ground. What the DEC checks and the rail checks of DBG count.

use crate::class::PartClasses;
use crate::graph::net_pins;
use crate::power::Power;
use erc_model::{EcadPinType, NetId, PartId, PinId, View};
use std::collections::BTreeMap;

/// The power-input pins of fitted components on a net
pub fn power_in_pins(view: View, net: NetId) -> Vec<PinId> {
    let b = view.board;
    net_pins(b, net)
        .filter(|p| {
            let pin = b.pin(*p);
            view.fitted(pin.part) && pin.etype.get() == Some(&EcadPinType::PowerIn)
        })
        .collect()
}

/// The power-input pins on a net by the part they belong to
pub fn power_in_by_part(view: View, net: NetId) -> BTreeMap<PartId, Vec<PinId>> {
    let b = view.board;
    let mut out: BTreeMap<PartId, Vec<PinId>> = BTreeMap::new();
    for p in power_in_pins(view, net) {
        out.entry(b.pin(p).part).or_default().push(p);
    }
    out
}

/// The fitted capacitors with one pin on `net` and another on a ground
pub fn caps_to_ground(view: View, classes: &PartClasses, power: &Power, net: NetId) -> Vec<PartId> {
    let b = view.board;
    let mut out: Vec<PartId> = net_pins(b, net)
        .map(|p| b.pin(p).part)
        .filter(|part| view.fitted(*part) && classes.get(*part).is_capacitor())
        .filter(|part| {
            b.part(*part)
                .pins
                .iter()
                .filter_map(|p| b.pin(*p).net)
                .any(|n| n != net && power.is_ground(n))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}
