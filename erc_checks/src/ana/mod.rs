//! `ANA` - analog: shunts and their sense amplifiers, op-amp feedback, unused channels, comparator hysteresis,
//! resistor ranges, DC return paths and bias dividers; amplifier pins by name ([`erc_analysis::analog`])

mod ana_10;
mod ana_11;
mod ana_12;
mod ana_16;
mod ana_21;
mod ana_22;
mod ana_3;
mod ana_4;

use crate::Ctx;
use erc_analysis::analog::{AmpUnit, amp_units, sense_pin};
use erc_analysis::graph::{net_degree, net_pins};
use erc_model::{NetId, PartClass, PartId, PinId};
use std::collections::{BTreeMap, BTreeSet};

/// The channels of the fitted parts of `class` (op-amps, comparators)
pub(crate) fn units(cx: &Ctx, class: PartClass) -> Vec<AmpUnit> {
    cx.view()
        .fitted_components()
        .filter(|p| cx.classes().get(*p) == class)
        .flat_map(|p| amp_units(cx.board(), p))
        .collect()
}

/// A pin's net when the pin is connected to something: not flagged no-connect, on a net with another component pin
pub(crate) fn connected(cx: &Ctx, pin: Option<PinId>) -> Option<NetId> {
    let b = cx.board();
    let p = b.pin(pin?);
    if p.no_connect {
        return None;
    }
    p.net.filter(|n| net_degree(b, *n) >= 2)
}

/// The nets reached from `start` through fitted parts passing `through` (each joins all its pins' nets); rails and
/// grounds are reached but not walked through
pub(crate) fn walk(cx: &Ctx, start: NetId, through: &dyn Fn(PartId) -> bool) -> BTreeSet<NetId> {
    let b = cx.board();
    let power = cx.power();
    let mut seen = BTreeSet::from([start]);
    let mut todo = vec![start];
    while let Some(net) = todo.pop() {
        if power.is_power_net(net) {
            continue;
        }
        for pin in net_pins(b, net) {
            let part = b.pin(pin).part;
            if !cx.fitted(part) || !through(part) {
                continue;
            }
            for q in &b.part(part).pins {
                if let Some(n) = b.pin(*q).net
                    && seen.insert(n)
                {
                    todo.push(n);
                }
            }
        }
    }
    seen
}

/// The fitted two-pin parts of a class `keep` accepts that join `a` and `c`
pub(crate) fn between(
    cx: &Ctx,
    a: NetId,
    c: NetId,
    keep: &dyn Fn(PartClass) -> bool,
) -> Vec<PartId> {
    let b = cx.board();
    let mut out: Vec<PartId> = net_pins(b, a)
        .map(|p| b.pin(p).part)
        .filter(|p| {
            cx.fitted(*p)
                && keep(cx.classes().get(*p))
                && b.part(*p).pins.len() == 2
                && b.part(*p).pins.iter().any(|q| b.pin(*q).net == Some(c))
                && b.part(*p).pins.iter().any(|q| b.pin(*q).net == Some(a))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// A shunt: a fitted two-pin resistor below `max` and above 0 Ω, with its two nets
pub(crate) fn shunts(cx: &Ctx, max: f64) -> Vec<(PartId, NetId, NetId)> {
    let b = cx.board();
    cx.view()
        .fitted_components()
        .filter(|p| cx.classes().get(*p).is_resistor() && b.part(*p).pins.len() == 2)
        .filter(|p| {
            cx.values()
                .get(*p)
                .value
                .is_some_and(|v| v > 0.0 && v < max)
        })
        .filter_map(|p| {
            let nets: Vec<NetId> = b
                .part(p)
                .pins
                .iter()
                .filter_map(|q| b.pin(*q).net)
                .collect();
            match nets.as_slice() {
                [x, y] if x != y => Some((p, *x, *y)),
                _ => None,
            }
        })
        .collect()
}

/// A current-sense input pair of a fitted IC (no op-amp or comparator: their inputs are `ANA-10`'s): the part,
/// the channel, the positive and negative sense pins
pub(crate) struct SensePair {
    pub part: PartId,
    pub pos: PinId,
    pub neg: PinId,
}

/// The sense input pairs of the fitted ICs, by pin name ([`sense_pin`])
pub(crate) fn sense_pairs(cx: &Ctx) -> Vec<SensePair> {
    let b = cx.board();
    let mut out = Vec::new();
    for part in cx.view().fitted_components() {
        let class = cx.classes().get(part);
        if !class.is_ic() || matches!(class, PartClass::Opamp | PartClass::Comparator) {
            continue;
        }
        let mut chans: BTreeMap<(u32, String), (Option<PinId>, Option<PinId>)> = BTreeMap::new();
        for pin in &b.part(part).pins {
            let p = b.pin(*pin);
            if let Some((positive, channel)) = sense_pin(&p.name, p.etype.get().copied()) {
                let e = chans.entry((p.unit, channel)).or_default();
                if positive {
                    e.0.get_or_insert(*pin);
                } else {
                    e.1.get_or_insert(*pin);
                }
            }
        }
        out.extend(chans.into_values().filter_map(|(pos, neg)| {
            Some(SensePair {
                part,
                pos: pos?,
                neg: neg?,
            })
        }));
    }
    out
}

/// A net and the nets one fitted two-pin resistor away (rails and grounds included), leaving out the resistor
/// `except`
pub(crate) fn with_series(cx: &Ctx, net: NetId, except: PartId) -> BTreeSet<NetId> {
    let b = cx.board();
    let mut out = BTreeSet::from([net]);
    for pin in net_pins(b, net) {
        let r = b.pin(pin).part;
        if r == except
            || !cx.fitted(r)
            || !cx.classes().get(r).is_resistor()
            || b.part(r).pins.len() != 2
        {
            continue;
        }
        out.extend(
            b.part(r)
                .pins
                .iter()
                .filter(|q| **q != pin)
                .filter_map(|q| b.pin(*q).net),
        );
    }
    out
}
