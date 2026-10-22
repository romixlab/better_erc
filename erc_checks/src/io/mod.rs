//! `IO` - default states and pulls: inputs nothing drives, buttons and their pulls, gates and open-drain outputs
//! without a pull, pull-ups and pull-downs on one net; on the pin types of the symbols, [`erc_analysis::pull`]
//! and the probe access of [`erc_analysis::access`]

mod io_1;
mod io_10;
mod io_11;
mod io_15;
mod io_16;
mod io_6;
mod io_7;
mod io_8;

use crate::Ctx;
use erc_analysis::graph::net_pins;
use erc_analysis::pull::{Pulls, pulls};
use erc_model::{EcadPinType, NetId, PartId, PinId};
use std::collections::{BTreeMap, BTreeSet};

/// The pulls of a net in the variant checked
pub(crate) fn net_pulls(cx: &Ctx, net: NetId) -> Pulls {
    pulls(cx.view(), cx.classes(), cx.values(), cx.power(), net)
}

/// A pin that can set its net's level: an output, bidirectional, tri-state, power or open-drain pin of a fitted
/// part, or a pin of a fitted IC whose type says nothing (passive, unspecified, free: its role is not known)
pub(crate) fn drives(cx: &Ctx, pin: PinId) -> bool {
    let b = cx.board();
    let p = b.pin(pin);
    if !cx.fitted(p.part) || p.no_connect {
        return false;
    }
    let class = cx.classes().get(p.part);
    match p.etype.get() {
        Some(
            EcadPinType::Output
            | EcadPinType::Bidirectional
            | EcadPinType::TriState
            | EcadPinType::PowerOut
            | EcadPinType::OpenCollector
            | EcadPinType::OpenEmitter,
        ) => true,
        Some(EcadPinType::Passive | EcadPinType::Unspecified | EcadPinType::Free) | None => {
            class.is_ic() || class.is_programmable()
        }
        _ => false,
    }
}

/// The nets something can set the level of: rails and grounds, nets with a [`drives`] pin, and, until nothing
/// changes, nets a fitted resistor, inductor, ferrite, jumper, net tie or crystal joins to a driven net, or a
/// transistor's channel or a switch connects to one (a FET's drain and source, a BJT's collector and emitter)
pub(crate) fn driven_nets(cx: &Ctx) -> BTreeSet<NetId> {
    let b = cx.board();
    let power = cx.power();
    let mut driven: BTreeSet<NetId> = (0..b.nets.len())
        .map(NetId::from_index)
        .filter(|n| power.is_power_net(*n) || net_pins(b, *n).any(|p| drives(cx, p)))
        .collect();
    // Each conducting part with the groups of its nets that it joins
    let mut joins: Vec<Vec<NetId>> = Vec::new();
    for part in cx.view().fitted_components() {
        let class = cx.classes().get(part);
        let nets_of = |names: &[&str]| -> Vec<NetId> {
            b.part(part)
                .pins
                .iter()
                .map(|p| b.pin(*p))
                .filter(|p| names.iter().any(|n| p.name.eq_ignore_ascii_case(n)))
                .filter_map(|p| p.net)
                .collect()
        };
        let all: Vec<NetId> = b
            .part(part)
            .pins
            .iter()
            .filter_map(|p| b.pin(*p).net)
            .collect();
        if (class.is_resistor()
            || class.is_dc_tie()
            || class == erc_model::PartClass::Inductor
            || class == erc_model::PartClass::Crystal)
            && b.part(part).pins.len() == 2
        {
            joins.push(all);
        } else if class.is_mosfet() {
            joins.push(nets_of(&["S", "SOURCE", "D", "DRAIN"]));
        } else if class.is_bjt() {
            joins.push(nets_of(&["C", "COLLECTOR", "E", "EMITTER"]));
        } else if class.is_switch() {
            joins.push(all);
        }
    }
    loop {
        let before = driven.len();
        for group in &joins {
            if group.iter().any(|n| driven.contains(n)) {
                driven.extend(group.iter().copied());
            }
        }
        if driven.len() == before {
            return driven;
        }
    }
}

/// The input pins (pin type `input`) of fitted parts on nets nothing drives, by net; a pin with a no-connect flag
/// is left out (the designer says so on purpose)
pub(crate) fn undriven_inputs(cx: &Ctx) -> BTreeMap<NetId, Vec<PinId>> {
    let b = cx.board();
    let driven = driven_nets(cx);
    let mut out: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
    for part in cx.view().fitted_components() {
        for pin in &b.part(part).pins {
            let p = b.pin(*pin);
            if p.etype.get() != Some(&EcadPinType::Input) || p.no_connect {
                continue;
            }
            if let Some(net) = p.net
                && !driven.contains(&net)
            {
                out.entry(net).or_default().push(*pin);
            }
        }
    }
    out
}

/// A switch's sides: the one rail or ground it connects to and the signal nets it pulls there. A switch with no
/// power net, or with two (a changeover between rails), has no such sides.
pub(crate) fn button_sides(cx: &Ctx, switch: PartId) -> Option<(NetId, Vec<NetId>)> {
    let b = cx.board();
    let power = cx.power();
    let nets: BTreeSet<NetId> = b
        .part(switch)
        .pins
        .iter()
        .filter_map(|p| b.pin(*p).net)
        .collect();
    let rails: Vec<NetId> = nets
        .iter()
        .copied()
        .filter(|n| power.is_power_net(*n))
        .collect();
    let signals: Vec<NetId> = nets
        .iter()
        .copied()
        .filter(|n| !power.is_power_net(*n) && net_pins(b, *n).count() > 1)
        .collect();
    match rails.as_slice() {
        [far] if !signals.is_empty() => Some((*far, signals)),
        _ => None,
    }
}

/// The fitted switches
pub(crate) fn switches(cx: &Ctx) -> Vec<PartId> {
    cx.view()
        .fitted_components()
        .filter(|p| cx.classes().get(*p).is_switch())
        .collect()
}

/// Whether two power nets sit at one level: both grounds, the same rail, or two rails of the same voltage
pub(crate) fn same_level(cx: &Ctx, a: NetId, c: NetId) -> bool {
    let power = cx.power();
    if a == c || (power.is_ground(a) && power.is_ground(c)) {
        return true;
    }
    match (power.volts(a), power.volts(c)) {
        (Some(x), Some(y)) => power.is_rail(a) && power.is_rail(c) && (x - y).abs() < 1e-6,
        _ => false,
    }
}
