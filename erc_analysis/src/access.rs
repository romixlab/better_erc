//! Where a probe, a cable or a bodge wire can touch a net: test points and connector pins on it, or one series
//! part (a resistor, ferrite, net tie or jumper) away; and the groups of nets joined by 0 Ω links. What the DBG
//! and TEST checks ask of every debug, bus and power net.

use crate::class::PartClasses;
use crate::graph::net_pins;
use crate::power::Power;
use crate::value::Values;
use erc_model::{NetId, PartClass, PartId, PinId, View};
use std::collections::{BTreeMap, BTreeSet};

/// A part whose pads a probe or a cable reaches: a test point or a connector. Fitted or not: a DNP header's
/// footprint is still there to solder to.
pub fn is_access(classes: &PartClasses, part: PartId) -> bool {
    let c = classes.get(part);
    c == PartClass::TestPoint || c.is_connector()
}

/// The test point and connector pins on a net
pub fn access_pins(view: View, classes: &PartClasses, net: NetId) -> Vec<PinId> {
    let b = view.board;
    net_pins(b, net)
        .filter(|p| is_access(classes, b.pin(*p).part))
        .collect()
}

/// A fitted two-pin part a signal passes through in series: a resistor, ferrite, net tie or jumper
pub fn is_series(view: View, classes: &PartClasses, part: PartId) -> bool {
    let c = classes.get(part);
    view.fitted(part) && view.board.part(part).pins.len() == 2 && (c.is_resistor() || c.is_dc_tie())
}

/// The nets one series part away from `net`, with the part: the far side of every series resistor, ferrite,
/// net tie or jumper on it. A far side on a rail or ground is a pull, not a series part, and is left out.
pub fn series_neighbours(
    view: View,
    classes: &PartClasses,
    power: &Power,
    net: NetId,
) -> Vec<(PartId, NetId)> {
    let b = view.board;
    let mut out = Vec::new();
    for pin in net_pins(b, net) {
        let part = b.pin(pin).part;
        if !is_series(view, classes, part) {
            continue;
        }
        let far = b
            .part(part)
            .pins
            .iter()
            .filter(|p| **p != pin)
            .find_map(|p| b.pin(*p).net);
        if let Some(far) = far
            && far != net
            && !power.is_power_net(far)
        {
            out.push((part, far));
        }
    }
    out
}

/// Where a probe reaches a net: a test point or connector pin, on the net itself (`via` none) or behind one
/// series part
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reach {
    pub pin: PinId,
    pub via: Option<PartId>,
}

/// The test point and connector pins on `net` or one series part away
pub fn reach(view: View, classes: &PartClasses, power: &Power, net: NetId) -> Vec<Reach> {
    let mut out: Vec<Reach> = access_pins(view, classes, net)
        .into_iter()
        .map(|pin| Reach { pin, via: None })
        .collect();
    for (part, far) in series_neighbours(view, classes, power, net) {
        out.extend(
            access_pins(view, classes, far)
                .into_iter()
                .map(|pin| Reach {
                    pin,
                    via: Some(part),
                }),
        );
    }
    out.sort();
    out.dedup();
    out
}

/// A fitted part that joins its two nets with no resistance worth the name: a 0 Ω resistor, a net tie, a
/// jumper
pub fn is_zero_ohm(view: View, classes: &PartClasses, values: &Values, part: PartId) -> bool {
    let c = classes.get(part);
    view.fitted(part)
        && view.board.part(part).pins.len() == 2
        && (matches!(c, PartClass::NetTie | PartClass::Jumper)
            || (c.is_resistor() && values.get(part).value == Some(0.0)))
}

/// Nets joined by 0 Ω links ([`is_zero_ohm`]): every net with a link maps to the smallest net id of its group;
/// a net without one is its own group and not in the map
pub fn zero_ohm_groups(
    view: View,
    classes: &PartClasses,
    values: &Values,
) -> BTreeMap<NetId, NetId> {
    let b = view.board;
    let mut parent: BTreeMap<NetId, NetId> = BTreeMap::new();
    fn root(parent: &mut BTreeMap<NetId, NetId>, n: NetId) -> NetId {
        let mut r = n;
        while let Some(&p) = parent.get(&r) {
            if p == r {
                break;
            }
            r = p;
        }
        parent.insert(n, r);
        r
    }
    for part in view.fitted_components() {
        if !is_zero_ohm(view, classes, values, part) {
            continue;
        }
        let nets: BTreeSet<NetId> = b
            .part(part)
            .pins
            .iter()
            .filter_map(|p| b.pin(*p).net)
            .collect();
        let mut it = nets.into_iter();
        let (Some(a), Some(c)) = (it.next(), it.next()) else {
            continue;
        };
        parent.entry(a).or_insert(a);
        parent.entry(c).or_insert(c);
        let (ra, rc) = (root(&mut parent, a), root(&mut parent, c));
        parent.insert(ra.max(rc), ra.min(rc));
    }
    let nets: Vec<NetId> = parent.keys().copied().collect();
    nets.into_iter()
        .map(|n| (n, root(&mut parent, n)))
        .collect()
}
