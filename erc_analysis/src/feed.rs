//! Power feeds, the source layer of the power structure (NET-5, architecture §1.4): which pins bring power onto a
//! net ([`Source`]: a regulator or load-switch output, a power-output pin, a connector or battery pin) and which
//! two-terminal parts carry it from net to net ([`Link`]: a ferrite, inductor, fuse, resistor, diode, switch,
//! jumper, net tie, or a transistor's drain-source or collector-emitter path). A rail no source reaches through
//! the links a check accepts is unfed (RAIL-5); a connector pin on a rail no regulator reaches brings power in
//! (PIN-4, PIN-6), one on a regulated rail hands it out (PIN-3).
//!
//! Grounds are never part of a feed: a link to a ground is a load or a filter, not a path. A diode passes power
//! only from its anode to its cathode; every other link both ways. The power tree with enables, states and the
//! voltage it sets on each rail comes later (NET-5 full, NET-8) on top of these.

use crate::class::PartClasses;
use crate::graph::net_pins;
use crate::power::Power;
use crate::value::Values;
use erc_model::class::{DiodeKind, PartClass};
use erc_model::{EcadPinType, NetId, NetKind, PartId, PinId, View};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The largest resistance the rail checks accept as a feed by default: an RC filter's series resistor (33 Ω in
/// front of an analog supply), a current-sense shunt, a 0 Ω option
pub const MAX_FEED_OHMS: f64 = 100.0;

/// What brings power onto a net
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceKind {
    /// A regulator's output (a power-output pin or one named `OUT`, `VOUT`), or its switching node
    Regulator,
    /// A load switch's or eFuse's output
    LoadSwitch,
    /// A power-output pin of another part
    PowerOut,
    /// A connector pin: power from outside the board (or handed out: PIN-3 decides)
    Connector,
    /// A battery terminal
    Battery,
}

/// A pin that brings power onto a net
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub pin: PinId,
    pub part: PartId,
    pub kind: SourceKind,
}

/// What a link is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LinkKind {
    Ferrite,
    Inductor,
    Resistor,
    Fuse,
    Diode,
    Switch,
    Jumper,
    NetTie,
    /// A MOSFET's drain-source or a BJT's collector-emitter path: a discrete switch
    Transistor,
}

/// A fitted two-terminal part between two nets that are not grounds
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub part: PartId,
    pub kind: LinkKind,
    pub a: NetId,
    pub b: NetId,
    /// A resistor's value, ohms, when it reads
    pub ohms: Option<f64>,
    /// A diode's cathode net: power passes only towards it
    pub forward: Option<NetId>,
}

impl Link {
    /// The net on the other side of `from`, when power passes that way
    pub fn other(&self, from: NetId) -> Option<NetId> {
        let to = if from == self.a {
            self.b
        } else if from == self.b {
            self.a
        } else {
            return None;
        };
        match self.forward {
            Some(cathode) if cathode != to => None,
            _ => Some(to),
        }
    }

    /// Passes DC with no filtering to speak of: a 0 Ω resistor, a jumper, a net tie
    pub fn transparent(&self) -> bool {
        match self.kind {
            LinkKind::Jumper | LinkKind::NetTie => true,
            LinkKind::Resistor => self.ohms == Some(0.0),
            _ => false,
        }
    }
}

/// The feeds of a board as one variant mounts it
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Feeds {
    /// The source pins on each net (grounds left out)
    pub sources: BTreeMap<NetId, Vec<Source>>,
    pub links: Vec<Link>,
}

/// A pin name that states a regulator's output: `OUT`, `VOUT`, `VOUT2`, `VO`, `OUTPUT`
pub fn is_output_name(name: &str) -> bool {
    let n = name.to_uppercase();
    let base = n.trim_end_matches(|c: char| c.is_ascii_digit());
    matches!(base, "OUT" | "VOUT" | "VO" | "OUTPUT" | "VOUT_" | "OUT_")
}

impl Feeds {
    /// The sources and links of the fitted parts of `view`
    pub fn of(view: View, classes: &PartClasses, values: &Values, power: &Power) -> Feeds {
        let b = view.board;
        let mut feeds = Feeds::default();
        for part in view.fitted_components() {
            let class = classes.get(part);
            for pin_id in &b.part(part).pins {
                let pin = b.pin(*pin_id);
                let Some(net) = pin.net else { continue };
                if power.is_ground(net) || b.net(net).kind == NetKind::Unconnected {
                    continue;
                }
                let power_out = pin.etype.get() == Some(&EcadPinType::PowerOut);
                let kind = match class {
                    PartClass::Regulator { .. }
                        if power_out
                            || is_output_name(&pin.name)
                            || power.switching_nodes.contains(&net) =>
                    {
                        Some(SourceKind::Regulator)
                    }
                    PartClass::LoadSwitch if power_out || is_output_name(&pin.name) => {
                        Some(SourceKind::LoadSwitch)
                    }
                    PartClass::Connector { .. } => Some(SourceKind::Connector),
                    PartClass::Battery => Some(SourceKind::Battery),
                    _ if power_out => Some(SourceKind::PowerOut),
                    _ => None,
                };
                if let Some(kind) = kind {
                    feeds.sources.entry(net).or_default().push(Source {
                        pin: *pin_id,
                        part,
                        kind,
                    });
                }
            }
            if let Some(link) = link(view, classes, values, power, part) {
                feeds.links.push(link);
            }
        }
        feeds
    }

    /// The links with an end on `net`
    pub fn links_at(&self, net: NetId) -> impl Iterator<Item = &Link> {
        self.links.iter().filter(move |l| l.a == net || l.b == net)
    }

    /// The sources on `net`
    pub fn sources_on(&self, net: NetId) -> &[Source] {
        self.sources.get(&net).map_or(&[], Vec::as_slice)
    }

    /// The nets reached from `from` through the links `pass` accepts (diodes forward only), each with the link
    /// parts on the way there in order (empty for the start nets)
    pub fn reach(
        &self,
        from: impl IntoIterator<Item = NetId>,
        pass: &dyn Fn(&Link) -> bool,
    ) -> BTreeMap<NetId, Vec<PartId>> {
        self.walk(from, pass, true)
    }

    /// [`Feeds::reach`] through diodes both ways: the nets joined to `from` whichever way power flows (what a
    /// short on one of them pulls down)
    pub fn region(
        &self,
        from: impl IntoIterator<Item = NetId>,
        pass: &dyn Fn(&Link) -> bool,
    ) -> BTreeMap<NetId, Vec<PartId>> {
        self.walk(from, pass, false)
    }

    fn walk(
        &self,
        from: impl IntoIterator<Item = NetId>,
        pass: &dyn Fn(&Link) -> bool,
        forward_only: bool,
    ) -> BTreeMap<NetId, Vec<PartId>> {
        let mut seen: BTreeMap<NetId, Vec<PartId>> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for n in from {
            if seen.insert(n, Vec::new()).is_none() {
                queue.push_back(n);
            }
        }
        while let Some(n) = queue.pop_front() {
            for l in self.links_at(n) {
                if !pass(l) {
                    continue;
                }
                let next = if forward_only {
                    l.other(n)
                } else if l.a == n {
                    Some(l.b)
                } else {
                    Some(l.a)
                };
                let Some(next) = next else { continue };
                if seen.contains_key(&next) {
                    continue;
                }
                let mut path = seen[&n].clone();
                path.push(l.part);
                seen.insert(next, path);
                queue.push_back(next);
            }
        }
        seen
    }

    /// The nets some source of a kind in `kinds` reaches through the links `pass` accepts
    pub fn fed_by(
        &self,
        kinds: &[SourceKind],
        pass: &dyn Fn(&Link) -> bool,
    ) -> BTreeMap<NetId, Vec<PartId>> {
        let starts: Vec<NetId> = self
            .sources
            .iter()
            .filter(|(_, s)| s.iter().any(|s| kinds.contains(&s.kind)))
            .map(|(n, _)| *n)
            .collect();
        self.reach(starts, pass)
    }
}

/// The link a fitted part makes, if it is a two-terminal part (a transistor by its main terminals) between two
/// nets that are not grounds
fn link(
    view: View,
    classes: &PartClasses,
    values: &Values,
    power: &Power,
    part: PartId,
) -> Option<Link> {
    let b = view.board;
    let class = classes.get(part);
    let kind = match class {
        PartClass::FerriteBead => LinkKind::Ferrite,
        PartClass::Inductor => LinkKind::Inductor,
        PartClass::Resistor => LinkKind::Resistor,
        PartClass::Fuse { .. } => LinkKind::Fuse,
        PartClass::Diode {
            kind: None | Some(DiodeKind::Rectifier | DiodeKind::Schottky),
        } => LinkKind::Diode,
        PartClass::Switch { .. } => LinkKind::Switch,
        PartClass::Jumper => LinkKind::Jumper,
        PartClass::NetTie => LinkKind::NetTie,
        c if c.is_transistor() => LinkKind::Transistor,
        _ => return None,
    };
    let pins: Vec<PinId> = b.part(part).pins.clone();
    let main = |names: &[&str]| -> Option<NetId> {
        pins.iter()
            .map(|p| b.pin(*p))
            .find(|p| names.contains(&p.name.to_uppercase().as_str()))
            .and_then(|p| p.net)
    };
    let (a, b_net, forward) = match kind {
        LinkKind::Transistor => {
            let a = main(&["D", "DRAIN", "C", "COLLECTOR"])?;
            let s = main(&["S", "SOURCE", "E", "EMITTER"])?;
            (a, s, None)
        }
        LinkKind::Diode => {
            let k = main(&["K", "CATHODE"])?;
            let an = main(&["A", "ANODE"])?;
            (an, k, Some(k))
        }
        _ => {
            let nets: BTreeSet<NetId> = pins.iter().filter_map(|p| b.pin(*p).net).collect();
            if nets.len() != 2 {
                return None;
            }
            let mut it = nets.into_iter();
            (it.next()?, it.next()?, None)
        }
    };
    if a == b_net || power.is_ground(a) || power.is_ground(b_net) {
        return None;
    }
    let ohms = (kind == LinkKind::Resistor)
        .then(|| values.get(part).value)
        .flatten();
    Some(Link {
        part,
        kind,
        a,
        b: b_net,
        ohms,
        forward,
    })
}

/// The pins on a net of fitted parts typed power input, whatever their names
pub fn power_in_on(view: View, net: NetId) -> Vec<PinId> {
    let b = view.board;
    net_pins(b, net)
        .filter(|p| {
            let pin = b.pin(*p);
            view.fitted(pin.part) && pin.etype.get() == Some(&EcadPinType::PowerIn)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_names() {
        for n in ["OUT", "VOUT", "Vout", "VO", "OUT2", "OUTPUT"] {
            assert!(is_output_name(n), "{n}");
        }
        for n in ["VIN", "IN", "SW", "FB", "EN", "OUTA_EN"] {
            assert!(!is_output_name(n), "{n}");
        }
    }

    #[test]
    fn diodes_pass_forward_only() {
        let (x, y) = (NetId::from_index(0), NetId::from_index(1));
        let d = Link {
            part: PartId::from_index(0),
            kind: LinkKind::Diode,
            a: x,
            b: y,
            ohms: None,
            forward: Some(y),
        };
        assert_eq!(d.other(x), Some(y));
        assert_eq!(d.other(y), None);
    }
}
