//! Connectivity queries over a board's part-pin-net graph. Only real components count: virtual `#` parts (power
//! symbols, flags) name nets but are not on them as far as these queries go.

use erc_model::{Board, EcadPinType, NetId, PartId, PinId};
use std::collections::BTreeSet;

/// The component pins on a net
pub fn net_pins(board: &Board, net: NetId) -> impl Iterator<Item = PinId> + '_ {
    board
        .net(net)
        .pins
        .iter()
        .copied()
        .filter(|p| board.is_component(board.pin(*p).part))
}

/// The number of component pins on a net
pub fn net_degree(board: &Board, net: NetId) -> usize {
    net_pins(board, net).count()
}

/// The components with a pin on any of `nets`
pub fn net_parts(board: &Board, nets: &[NetId]) -> BTreeSet<PartId> {
    nets.iter()
        .flat_map(|n| net_pins(board, *n))
        .map(|p| board.pin(p).part)
        .collect()
}

/// The nets a part's pins are on
pub fn part_nets(board: &Board, part: PartId) -> BTreeSet<NetId> {
    board
        .part(part)
        .pins
        .iter()
        .filter_map(|p| board.pin(*p).net)
        .collect()
}

/// The nets of a part's pins, leaving out the pins named in `exclude` (a FET's gate)
pub fn part_nets_except_pins(board: &Board, part: PartId, exclude: &[&str]) -> BTreeSet<NetId> {
    board
        .part(part)
        .pins
        .iter()
        .map(|p| board.pin(*p))
        .filter(|p| !exclude.contains(&p.name.as_str()))
        .filter_map(|p| p.net)
        .collect()
}

/// Whether two parts share a net
pub fn parts_connected(board: &Board, a: PartId, b: PartId) -> bool {
    let a = part_nets(board, a);
    part_nets(board, b).iter().any(|n| a.contains(n))
}

/// Whether a part has a pin on `net`
pub fn part_on_net(board: &Board, part: PartId, net: NetId) -> bool {
    board
        .part(part)
        .pins
        .iter()
        .any(|p| board.pin(*p).net == Some(net))
}

/// Chains of parts from net `start` to net `end`: the first part has a pin on `start`, each next part shares a net
/// with the previous one (not through the pin the previous one entered by), the last has a pin on `end`, and each
/// part passes its filter in `through`. Each chain is the entry pin and part of every link, in order; chains come
/// sorted, so the first is the same on every run.
pub fn net_chains(
    board: &Board,
    start: NetId,
    through: &[&dyn Fn(PartId) -> bool],
    end: NetId,
) -> Vec<Vec<(PinId, PartId)>> {
    let Some((first, rest)) = through.split_first() else {
        return Vec::new();
    };
    let mut layers: Vec<BTreeSet<(PinId, PartId)>> = vec![
        net_pins(board, start)
            .map(|p| (p, board.pin(p).part))
            .filter(|(_, part)| first(*part))
            .collect(),
    ];
    for filter in rest {
        let mut next = BTreeSet::new();
        for (entry, part) in layers.last().into_iter().flatten() {
            next.extend(reachable_pins(board, *part, *entry, *filter));
        }
        layers.push(next);
    }
    if let Some(last) = layers.last_mut() {
        last.retain(|(_, part)| part_on_net(board, *part, end));
    }
    let mut chains: Vec<Vec<(PinId, PartId)>> = vec![Vec::new()];
    for layer in &layers {
        let mut longer = Vec::new();
        for chain in &chains {
            for link in layer {
                let joined = chain
                    .last()
                    .is_none_or(|(_, prev)| parts_connected(board, *prev, link.1));
                if joined {
                    let mut c = chain.clone();
                    c.push(*link);
                    longer.push(c);
                }
            }
        }
        chains = longer;
    }
    chains
}

/// The pins of parts passing `filter` reachable from `part` through its pins other than `entry` (on nets that
/// `entry` is not on)
fn reachable_pins(
    board: &Board,
    part: PartId,
    entry: PinId,
    filter: &dyn Fn(PartId) -> bool,
) -> BTreeSet<(PinId, PartId)> {
    let entry_net = board.pin(entry).net;
    let mut out = BTreeSet::new();
    for net in part_nets(board, part) {
        if Some(net) == entry_net {
            continue;
        }
        for pin in net_pins(board, net) {
            let other = board.pin(pin).part;
            if other != part && filter(other) {
                out.insert((pin, other));
            }
        }
    }
    out
}

/// The component pins typed `input` that are on a net, by net name and then pin: the inputs IO-1 will judge
/// (driven or floating) and the Inputs view lists
pub fn input_pins(board: &Board) -> Vec<PinId> {
    let mut pins: Vec<PinId> = (0..board.pins.len())
        .map(PinId::from_index)
        .filter(|p| {
            let pin = board.pin(*p);
            pin.net.is_some()
                && board.is_component(pin.part)
                && pin.etype.get() == Some(&EcadPinType::Input)
        })
        .collect();
    pins.sort_by(|a, b| {
        let key = |p: &PinId| board.pin(*p).net.map(|n| board.net(n).name.as_str());
        key(a).cmp(&key(b)).then_with(|| a.cmp(b))
    });
    pins
}

/// Chains of parts, each part passing its filter in `through` and sharing a net with the previous one; nets for
/// which `skip` is true do not join parts (rails and grounds, which connect nearly everything). The first chain
/// link is every part passing the first filter. `erc_core`'s `PcbAssembly::find_part_chains`: an IC then an
/// inductor finds converters' switching nodes, IC, resistor, inductor one with a sense resistor.
pub fn part_chains(
    board: &Board,
    through: &[&dyn Fn(PartId) -> bool],
    skip: &dyn Fn(NetId) -> bool,
) -> Vec<Vec<PartId>> {
    if through.len() < 2 {
        return Vec::new();
    }
    let parts: Vec<PartId> = (0..board.parts.len())
        .map(PartId::from_index)
        .filter(|p| board.is_component(*p))
        .collect();
    let mut chains: Vec<Vec<PartId>> = parts
        .iter()
        .filter(|p| through[0](**p))
        .map(|p| vec![*p])
        .collect();
    for filter in &through[1..] {
        let mut longer = Vec::new();
        for chain in &chains {
            let last = *chain.last().expect("never empty");
            let nets: BTreeSet<NetId> = part_nets(board, last)
                .into_iter()
                .filter(|n| !skip(*n))
                .collect();
            for &p in &parts {
                if p != last && filter(p) && part_nets(board, p).iter().any(|n| nets.contains(n)) {
                    let mut c = chain.clone();
                    c.push(p);
                    longer.push(c);
                }
            }
        }
        chains = longer;
    }
    chains
}

/// A path of two-terminal parts in series from a net: the parts in order and the net it ends on
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeriesPath {
    pub parts: Vec<PartId>,
    pub end: NetId,
}

/// The paths from `start` through parts with exactly two nets that pass `through`, at most `max_len` parts long;
/// a path goes on from a net only when `pass` lets it (a node of the chain: no rail, no ground), and never through
/// a net twice. Every prefix is a path of its own, so a caller picks the ends it wants (a ground, `start` again).
pub fn series_paths(
    board: &Board,
    start: NetId,
    through: &dyn Fn(PartId) -> bool,
    pass: &dyn Fn(NetId) -> bool,
    max_len: usize,
) -> Vec<SeriesPath> {
    struct Walk<'a> {
        board: &'a Board,
        through: &'a dyn Fn(PartId) -> bool,
        pass: &'a dyn Fn(NetId) -> bool,
        max_len: usize,
        parts: Vec<PartId>,
        nets: Vec<NetId>,
        out: Vec<SeriesPath>,
    }
    impl Walk<'_> {
        fn from(&mut self, at: NetId) {
            if self.parts.len() >= self.max_len {
                return;
            }
            let here: BTreeSet<PartId> = net_pins(self.board, at)
                .map(|p| self.board.pin(p).part)
                .collect();
            for p in here {
                if self.parts.contains(&p) || !(self.through)(p) {
                    continue;
                }
                let pn = part_nets(self.board, p);
                if pn.len() != 2 {
                    continue;
                }
                let Some(next) = pn.into_iter().find(|n| *n != at) else {
                    continue;
                };
                self.parts.push(p);
                self.out.push(SeriesPath {
                    parts: self.parts.clone(),
                    end: next,
                });
                if !self.nets.contains(&next) && (self.pass)(next) {
                    self.nets.push(next);
                    self.from(next);
                    self.nets.pop();
                }
                self.parts.pop();
            }
        }
    }
    let mut w = Walk {
        board,
        through,
        pass,
        max_len,
        parts: Vec::new(),
        nets: vec![start],
        out: Vec::new(),
    };
    w.from(start);
    w.out
}
