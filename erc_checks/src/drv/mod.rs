//! `DRV` - drivers and switches: transistors by their pin names (G, D, S; B, C, E), relays, inductive loads and
//! LEDs by class, on the graph and the rails of [`erc_analysis::power`]

mod drv_12;
mod drv_14;
mod drv_17;
mod drv_5;
mod drv_6;
mod drv_7;
mod drv_9;

use crate::Ctx;
use erc_analysis::graph::{net_pins, series_paths};
use erc_model::{EcadPinType, NetId, PartClass, PartId, PinId};
use std::collections::BTreeSet;

pub(crate) use crate::text::designators;

pub(crate) const GATE: &[&str] = &["G", "GATE"];
pub(crate) const DRAIN: &[&str] = &["D", "DRAIN"];
pub(crate) const SOURCE: &[&str] = &["S", "SOURCE"];
pub(crate) const BASE: &[&str] = &["B", "BASE"];
pub(crate) const COLLECTOR: &[&str] = &["C", "COLLECTOR"];
pub(crate) const EMITTER: &[&str] = &["E", "EMITTER"];

/// The first pin of a part named one of `names` (upper case), with its net
pub(crate) fn pin_named(cx: &Ctx, part: PartId, names: &[&str]) -> Option<(PinId, NetId)> {
    let b = cx.board();
    b.part(part).pins.iter().find_map(|p| {
        let pin = b.pin(*p);
        names
            .contains(&pin.name.to_uppercase().as_str())
            .then_some(pin.net.map(|n| (*p, n)))
            .flatten()
    })
}

/// A transistor's two main terminals (drain and source, collector and emitter) by pin name
pub(crate) fn main_terminals(cx: &Ctx, part: PartId) -> Option<(NetId, NetId)> {
    if let (Some((_, d)), Some((_, s))) = (pin_named(cx, part, DRAIN), pin_named(cx, part, SOURCE))
    {
        return Some((d, s));
    }
    let (_, c) = pin_named(cx, part, COLLECTOR)?;
    let (_, e) = pin_named(cx, part, EMITTER)?;
    Some((c, e))
}

/// The pins on a net of fitted ICs that drive it: typed output, bidirectional, tri-state or open collector/emitter
pub(crate) fn logic_drivers(cx: &Ctx, net: NetId) -> Vec<PinId> {
    let b = cx.board();
    net_pins(b, net)
        .filter(|p| {
            let pin = b.pin(*p);
            cx.fitted(pin.part)
                && cx.classes().get(pin.part).is_ic()
                && matches!(
                    pin.etype.get(),
                    Some(
                        EcadPinType::Output
                            | EcadPinType::Bidirectional
                            | EcadPinType::TriState
                            | EcadPinType::OpenCollector
                            | EcadPinType::OpenEmitter
                    )
                )
        })
        .collect()
}

/// The fitted transistors with a main terminal on `net`, with their main terminals (this net first)
pub(crate) fn switches_on(cx: &Ctx, net: NetId) -> Vec<(PartId, NetId, NetId)> {
    let b = cx.board();
    let parts: BTreeSet<PartId> = net_pins(b, net).map(|p| b.pin(p).part).collect();
    parts
        .into_iter()
        .filter(|p| cx.fitted(*p) && cx.classes().get(*p).is_transistor())
        .filter_map(|p| {
            let (x, y) = main_terminals(cx, p)?;
            if x == net {
                Some((p, x, y))
            } else if y == net {
                Some((p, y, x))
            } else {
                None
            }
        })
        .collect()
}

/// A fitted part that suppresses an inductive kick when it sits across a coil or a switch: a diode (not an LED),
/// a TVS or Zener, a resistor or a capacitor (an RC)
fn suppressor_part(cx: &Ctx, part: PartId) -> bool {
    let c = cx.classes().get(part);
    cx.fitted(part) && ((c.is_diode() && !c.is_led()) || c.is_resistor() || c.is_capacitor())
}

/// The suppressors between two nets: chains of at most two parts through a node nothing else uses, with a diode
/// (a diode, diode and Zener, diode and resistor) or a resistor and a capacitor (an RC snubber)
pub(crate) fn suppressors(cx: &Ctx, a: NetId, b: NetId) -> Vec<Vec<PartId>> {
    let board = cx.board();
    let power = cx.power();
    let through = |p: PartId| suppressor_part(cx, p);
    let node = |n: NetId| !power.is_power_net(n) && net_pins(board, n).count() == 2;
    series_paths(board, a, &through, &node, 2)
        .into_iter()
        .filter(|path| path.end == b)
        .map(|path| path.parts)
        .filter(|parts| {
            let classes: Vec<PartClass> = parts.iter().map(|p| cx.classes().get(*p)).collect();
            classes.iter().any(|c| c.is_diode())
                || (classes.iter().any(|c| c.is_resistor())
                    && classes.iter().any(|c| c.is_capacitor()))
        })
        .collect()
}

/// The nets of an inductive load's pins that a switch drives with nothing to take the kick: each with the
/// transistors (and, with `ics`, the IC outputs' parts) switching it. A switched pin is clamped by a suppressor
/// ([`suppressors`]) to another pin of the load, across the transistor to its other main terminal, or to a rail or
/// ground; by an IC switch whose pin named `COM` sits on another pin of the load (a ULN2003's internal diodes).
pub(crate) fn unclamped(cx: &Ctx, load: PartId, ics: bool) -> Vec<(NetId, BTreeSet<PartId>)> {
    let b = cx.board();
    let power = cx.power();
    let load_nets: BTreeSet<NetId> = b
        .part(load)
        .pins
        .iter()
        .filter_map(|p| b.pin(*p).net)
        .collect();
    let mut out = Vec::new();
    for &n in &load_nets {
        if power.is_power_net(n) {
            continue;
        }
        let transistors = switches_on(cx, n);
        let ic_parts: BTreeSet<PartId> = if ics {
            logic_drivers(cx, n)
                .into_iter()
                .map(|p| b.pin(p).part)
                .collect()
        } else {
            BTreeSet::new()
        };
        if transistors.is_empty() && ic_parts.is_empty() {
            continue;
        }
        let mut targets: BTreeSet<NetId> = load_nets.iter().copied().filter(|m| *m != n).collect();
        targets.extend(transistors.iter().map(|(_, _, other)| *other));
        targets.extend(power.rails.keys().copied());
        targets.extend(power.grounds.keys().copied());
        let clamped = targets.iter().any(|t| !suppressors(cx, n, *t).is_empty())
            || ic_parts.iter().any(|ic| {
                pin_named(cx, *ic, &["COM"]).is_some_and(|(_, net)| load_nets.contains(&net))
            });
        if !clamped {
            let mut switches: BTreeSet<PartId> = transistors.iter().map(|(p, _, _)| *p).collect();
            switches.extend(ic_parts);
            out.push((n, switches));
        }
    }
    out
}
