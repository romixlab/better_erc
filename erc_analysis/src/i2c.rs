//! The I2C bus recogniser (the first piece of NET-4), ported from `erc_core`'s `i2c.rs` onto the board model.
//!
//! - A bus is a pair of nets named alike with `SCL` and `SDA` in the same place (`/SCL1_3V3` and `/SDA1_3V3` make
//!   bus `/I2C1_3V3`). Pin roles will replace the names as the source (NET-7).
//! - Its pull-up is a pair of resistors, one from each line, meeting on one net (the rail). Further resistors
//!   from a line to that rail are redundant pull-ups.
//! - The other parts on the lines are devices (ICs), connectors, test points, translators (a translator IC whose
//!   other side is another bus), discrete translators (a FET per line to another bus, the BSS138 pair, with the
//!   nets of their gates), ties (a resistor, 0 Ω or a few ohms, a ferrite, net tie or jumper per line, to another
//!   bus, or to an unnamed pair of nets that becomes an ad-hoc bus named `<bus>_to_<part>`), or unknown.
//! - Only fitted parts count: the buses are found on a [`View`], so a pull-up made DNP in the variant checked is
//!   no pull-up.
//! - Buses joined by ties form a direct segment (one electrical bus: one pull-up pair wanted); buses joined by
//!   ties or translators form one logical bus.
//!
//! The analysis records what it finds; the I2C checks judge it.

use crate::class::{PartClass, PartClasses};
use crate::graph::{
    net_chains, net_degree, net_parts, part_nets, part_nets_except_pins, parts_connected,
};
use erc_model::{Board, NetId, PartId, View};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Index of a bus in [`I2cBuses::buses`]
pub type BusIdx = usize;

/// One I2C bus: a pair of SCL and SDA nets
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct I2cBus {
    /// `/I2C1_3V3` from `/SCL1_3V3`, or `<bus>_to_<part>` for a bus found behind a tie
    pub name: String,
    pub scl: NetId,
    pub sda: NetId,
    pub pull_up: Option<PullUp>,
    /// Resistors from SCL or SDA to the pull-up's rail besides the pull-up pair
    pub redundant_pull_ups: BTreeSet<PartId>,
    pub nodes: Vec<Node>,
}

/// A pull-up pair
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullUp {
    pub scl: PartId,
    pub sda: PartId,
    /// The net both resistors meet on
    pub rail: NetId,
}

/// A part on a bus
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Node {
    Device(PartId),
    Connector(PartId),
    TestPoint(PartId),
    /// A translator IC with another bus on its other side
    Translator {
        part: PartId,
        other_side: BusIdx,
    },
    /// A FET per line to another bus (the BSS138 pair), with the nets of their gates (pins named `G`, `GATE`)
    DiscreteTranslator {
        scl_fet: PartId,
        sda_fet: PartId,
        other_side: BusIdx,
        scl_gate: Option<NetId>,
        sda_gate: Option<NetId>,
    },
    /// A resistor (0 Ω, a few ohms), ferrite, net tie or jumper per line to another bus
    Tie {
        scl_tie: PartId,
        sda_tie: PartId,
        other_side: BusIdx,
    },
    /// A part whose role on the bus is not known
    Unknown(PartId),
}

impl Node {
    fn mentions(&self, part: PartId) -> bool {
        match *self {
            Node::DiscreteTranslator {
                scl_fet, sda_fet, ..
            } => scl_fet == part || sda_fet == part,
            Node::Tie {
                scl_tie, sda_tie, ..
            } => scl_tie == part || sda_tie == part,
            _ => false,
        }
    }
}

/// Every I2C bus of a board and how they join
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct I2cBuses {
    pub buses: Vec<I2cBus>,
    /// Buses joined by ties: one electrical bus each, in order of their first bus
    pub direct_segments: Vec<BTreeSet<BusIdx>>,
    /// Buses joined by ties or translators: one logical bus each
    pub same_bus_segments: Vec<BTreeSet<BusIdx>>,
    /// What the recogniser could not place (DIAG-1): a part on a bus whose role is not known
    pub notes: Vec<String>,
}

impl I2cBuses {
    /// Finds the I2C buses of the base design of `board`
    pub fn of(board: &Board, classes: &PartClasses) -> I2cBuses {
        I2cBuses::on(board.view(None), classes)
    }

    /// Finds the I2C buses of `view`: the parts its variant fits
    pub fn on(view: View, classes: &PartClasses) -> I2cBuses {
        Recogniser {
            board: view.board,
            view,
            classes,
        }
        .run()
    }

    /// The pull-up pairs of a segment's buses (a direct segment should have one)
    pub fn pull_ups(&self, segment: &BTreeSet<BusIdx>) -> Vec<(BusIdx, PullUp)> {
        segment
            .iter()
            .filter_map(|i| Some((*i, self.buses[*i].pull_up?)))
            .collect()
    }

    /// The direct segment a bus is in
    pub fn direct_segment(&self, bus: BusIdx) -> Option<&BTreeSet<BusIdx>> {
        self.direct_segments.iter().find(|s| s.contains(&bus))
    }

    /// The buses of the direct segment of `bus`'s logical bus other than its own direct segment: the segments
    /// behind translators, buffers and discrete translators
    pub fn segments_of_logical_bus(&self, bus: BusIdx) -> Vec<&BTreeSet<BusIdx>> {
        let Some(logical) = self.same_bus_segments.iter().find(|s| s.contains(&bus)) else {
            return Vec::new();
        };
        self.direct_segments
            .iter()
            .filter(|s| s.iter().all(|b| logical.contains(b)))
            .collect()
    }

    pub fn by_name(&self, name: &str) -> Option<&I2cBus> {
        self.buses.iter().find(|b| b.name == name)
    }

    /// The bus names of a segment, sorted
    pub fn segment_names(&self, segment: &BTreeSet<BusIdx>) -> BTreeSet<&str> {
        segment
            .iter()
            .map(|i| self.buses[*i].name.as_str())
            .collect()
    }
}

struct Recogniser<'a> {
    board: &'a Board,
    view: View<'a>,
    classes: &'a PartClasses,
}

impl Recogniser<'_> {
    fn run(&self) -> I2cBuses {
        let mut buses = self.find_buses();
        self.find_interconnects(&mut buses);
        self.find_translators(&mut buses);
        let mut out = I2cBuses {
            buses,
            ..I2cBuses::default()
        };
        self.collect_segments(&mut out);
        for bus in &out.buses {
            for node in &bus.nodes {
                if let Node::Unknown(p) = node {
                    out.notes.push(format!(
                        "I2C bus {}: {} ({}) is on the bus in no known role",
                        bus.name,
                        self.board.part(*p).designator,
                        self.classes.get(*p).name()
                    ));
                }
            }
        }
        out
    }

    fn is_resistor(&self, p: PartId) -> bool {
        self.view.fitted(p) && self.classes.is(p, PartClass::Resistor)
    }

    /// A two-terminal part that ties a line to another: resistor, ferrite, net tie, jumper
    fn is_tie(&self, p: PartId) -> bool {
        let c = self.classes.get(p);
        self.view.fitted(p) && (c == PartClass::Resistor || c.is_dc_tie())
    }

    fn is_fet(&self, p: PartId) -> bool {
        self.view.fitted(p) && self.classes.get(p).is_transistor()
    }

    /// The fitted components on `nets`
    fn parts_on(&self, nets: &[NetId]) -> BTreeSet<PartId> {
        let mut parts = net_parts(self.board, nets);
        parts.retain(|p| self.view.fitted(*p));
        parts
    }

    /// The net of a part's pin named `G` or `GATE`
    fn gate(&self, part: PartId) -> Option<NetId> {
        self.board
            .part(part)
            .pins
            .iter()
            .map(|p| self.board.pin(*p))
            .find(|p| matches!(p.name.to_uppercase().as_str(), "G" | "GATE"))
            .and_then(|p| p.net)
    }

    /// A bus for every pair of nets named alike with `SCL` and `SDA`
    fn find_buses(&self) -> Vec<I2cBus> {
        let board = self.board;
        let mut buses = Vec::new();
        for (i, net) in board.nets.iter().enumerate() {
            let Some(at) = net.name.find("SCL") else {
                continue;
            };
            let (prefix, suffix) = (&net.name[..at], &net.name[at + 3..]);
            let Some(sda) = board.net_by_name(&format!("{prefix}SDA{suffix}")) else {
                continue;
            };
            let scl = NetId::from_index(i);
            buses.push(self.bus(
                collapse_underscores(&format!("{prefix}I2C{suffix}")),
                scl,
                sda,
            ));
        }
        buses
    }

    /// A bus on two nets with its pull-ups and parts
    fn bus(&self, name: String, scl: NetId, sda: NetId) -> I2cBus {
        let mut parts = self.parts_on(&[scl, sda]);
        let mut redundant_pull_ups = BTreeSet::new();
        let pull_up = self.pull_up(scl, sda);
        if let Some(pu) = pull_up {
            parts.remove(&pu.scl);
            parts.remove(&pu.sda);
            let r: &dyn Fn(PartId) -> bool = &|p| self.is_resistor(p);
            for line in [scl, sda] {
                for chain in net_chains(self.board, line, &[r], pu.rail) {
                    for (_, p) in chain {
                        if p != pu.scl && p != pu.sda {
                            parts.remove(&p);
                            redundant_pull_ups.insert(p);
                        }
                    }
                }
            }
        }
        I2cBus {
            name,
            scl,
            sda,
            pull_up,
            redundant_pull_ups,
            nodes: parts.into_iter().map(|p| self.node(p)).collect(),
        }
    }

    /// A resistor from SCL and one from SDA meeting on one net
    fn pull_up(&self, scl: NetId, sda: NetId) -> Option<PullUp> {
        let r: &dyn Fn(PartId) -> bool = &|p| self.is_resistor(p);
        let chains = net_chains(self.board, scl, &[r, r], sda);
        let chain = chains.first()?;
        let (pin, sda_r) = chain[1];
        Some(PullUp {
            scl: chain[0].1,
            sda: sda_r,
            rail: self.board.pin(pin).net?,
        })
    }

    /// The role of a part on a bus from its class; translators and ties are resolved later
    fn node(&self, p: PartId) -> Node {
        match self.classes.get(p) {
            PartClass::Connector { .. } => Node::Connector(p),
            PartClass::TestPoint => Node::TestPoint(p),
            c if c.is_ic() && !c.is_translator() => Node::Device(p),
            _ => Node::Unknown(p),
        }
    }

    /// Resistors and transistors from a bus's lines to another bus (ties, discrete translators), or to an unnamed
    /// pair of nets with one more part on it (a new bus behind a tie)
    fn find_interconnects(&self, buses: &mut Vec<I2cBus>) {
        let board = self.board;
        let mut changes: BTreeMap<BusIdx, Vec<Node>> = BTreeMap::new();
        let mut created: Vec<I2cBus> = Vec::new();
        let mentioned = |changes: &BTreeMap<BusIdx, Vec<Node>>, p: PartId| {
            changes.values().flatten().any(|n| n.mentions(p))
        };
        for (bi, bus) in buses.iter().enumerate() {
            for node in &bus.nodes {
                let Node::Unknown(part) = *node else {
                    continue;
                };
                if mentioned(&changes, part) {
                    continue;
                }
                let tie = self.is_tie(part);
                if !(tie || self.is_fet(part)) {
                    continue;
                }
                let nets = part_nets_except_pins(board, part, &["G", "GATE"]);
                let is_scl = nets.contains(&bus.scl);
                let (line, other_line) = if is_scl {
                    (bus.scl, bus.sda)
                } else {
                    (bus.sda, bus.scl)
                };
                let Some(&other_side) = nets.iter().find(|n| **n != line) else {
                    continue;
                };
                let same_kind = move |p: PartId| {
                    if tie { self.is_tie(p) } else { self.is_fet(p) }
                };
                let other_bus = buses
                    .iter()
                    .position(|b| b.scl == other_side || b.sda == other_side);
                if let Some(oi) = other_bus {
                    let other = &buses[oi];
                    let other_complement = if is_scl { other.sda } else { other.scl };
                    let chains = net_chains(board, other_complement, &[&same_kind], other_line);
                    let Some(&(_, complement)) = chains.first().and_then(|c| c.first()) else {
                        continue;
                    };
                    let (scl_part, sda_part) = (part, complement);
                    let (scl_part, sda_part) = if is_scl {
                        (scl_part, sda_part)
                    } else {
                        (sda_part, scl_part)
                    };
                    let node = |other_side| {
                        if tie {
                            Node::Tie {
                                scl_tie: scl_part,
                                sda_tie: sda_part,
                                other_side,
                            }
                        } else {
                            Node::DiscreteTranslator {
                                scl_fet: scl_part,
                                sda_fet: sda_part,
                                other_side,
                                scl_gate: self.gate(scl_part),
                                sda_gate: self.gate(sda_part),
                            }
                        }
                    };
                    changes.entry(bi).or_default().push(node(oi));
                    changes.entry(oi).or_default().push(node(bi));
                } else {
                    // The other side may be an IC or connector behind a 0 Ω on unnamed nets
                    let mut far_parts = self.parts_on(&[other_side]);
                    far_parts.remove(&part);
                    let mut targets = Vec::new();
                    for n in &bus.nodes {
                        let Node::Unknown(complement) = *n else {
                            continue;
                        };
                        if complement == part || !same_kind(complement) {
                            continue;
                        }
                        for far in &far_parts {
                            if parts_connected(board, complement, *far) {
                                targets.push((*far, complement));
                            }
                        }
                    }
                    let [(target, complement)] = targets[..] else {
                        continue;
                    };
                    let Some(&far_complement) = part_nets(board, complement)
                        .iter()
                        .find(|n| **n != other_line)
                    else {
                        continue;
                    };
                    let (scl, sda) = if is_scl {
                        (other_side, far_complement)
                    } else {
                        (far_complement, other_side)
                    };
                    let name = format!("{}_to_{}", bus.name, board.part(target).designator);
                    let new_idx = buses.len() + created.len();
                    created.push(self.bus(name, scl, sda));
                    let (scl_tie, sda_tie) = if is_scl {
                        (part, complement)
                    } else {
                        (complement, part)
                    };
                    let node = |other_side| Node::Tie {
                        scl_tie,
                        sda_tie,
                        other_side,
                    };
                    changes.entry(bi).or_default().push(node(new_idx));
                    changes.entry(new_idx).or_default().push(node(bi));
                }
            }
        }
        buses.extend(created);
        for (bi, nodes) in changes {
            let bus = &mut buses[bi];
            for node in nodes {
                bus.nodes
                    .retain(|n| !matches!(n, Node::Unknown(p) if node.mentions(*p)));
                bus.nodes.push(node);
            }
        }
    }

    /// Translator ICs: the other bus whose SCL the IC also touches
    fn find_translators(&self, buses: &mut [I2cBus]) {
        let scls: Vec<NetId> = buses.iter().map(|b| b.scl).collect();
        for bus in buses.iter_mut() {
            for node in bus.nodes.iter_mut() {
                let Node::Unknown(part) = *node else {
                    continue;
                };
                if !self.classes.get(part).is_translator() {
                    continue;
                }
                let mut nets = part_nets(self.board, part);
                nets.remove(&bus.scl);
                nets.remove(&bus.sda);
                let others: Vec<BusIdx> = (0..scls.len())
                    .filter(|i| nets.contains(&scls[*i]))
                    .collect();
                *node = match others[..] {
                    [other_side] => Node::Translator { part, other_side },
                    // A mux, or an MCU on several buses
                    _ => Node::Device(part),
                };
            }
        }
    }

    /// Direct segments (ties) and logical buses (ties and translators), as connected components
    fn collect_segments(&self, out: &mut I2cBuses) {
        let n = out.buses.len();
        let mut direct = UnionFind::new(n);
        let mut same = UnionFind::new(n);
        for (i, bus) in out.buses.iter().enumerate() {
            for node in &bus.nodes {
                match *node {
                    Node::Tie { other_side, .. } => {
                        direct.join(i, other_side);
                        same.join(i, other_side);
                    }
                    Node::Translator { other_side, .. }
                    | Node::DiscreteTranslator { other_side, .. } => same.join(i, other_side),
                    _ => {}
                }
            }
        }
        out.direct_segments = direct.sets();
        out.same_bus_segments = same.sets();
    }
}

/// A pull-up rail that goes nowhere (I2C-3): the rail has only the pull-up pair on it, or a third resistor that
/// ends on a net with nothing else
pub fn rail_goes_nowhere(board: &Board, classes: &PartClasses, pu: &PullUp) -> bool {
    match net_degree(board, pu.rail) {
        2 => true,
        3 => {
            let mut parts = net_parts(board, &[pu.rail]);
            parts.remove(&pu.scl);
            parts.remove(&pu.sda);
            let Some(third) = parts
                .into_iter()
                .find(|p| classes.is(*p, PartClass::Resistor))
            else {
                return false;
            };
            part_nets(board, third)
                .into_iter()
                .find(|n| *n != pu.rail)
                .is_some_and(|far| net_degree(board, far) == 1)
        }
        _ => false,
    }
}

fn collapse_underscores(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if !(c == '_' && out.ends_with('_')) {
            out.push(c);
        }
    }
    out
}

struct UnionFind(Vec<usize>);

impl UnionFind {
    fn new(n: usize) -> UnionFind {
        UnionFind((0..n).collect())
    }

    fn root(&mut self, i: usize) -> usize {
        let mut r = i;
        while self.0[r] != r {
            r = self.0[r];
        }
        self.0[i] = r;
        r
    }

    fn join(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.root(a), self.root(b));
        self.0[ra.max(rb)] = ra.min(rb);
    }

    /// The sets, each ordered, in order of their smallest member
    fn sets(&mut self) -> Vec<BTreeSet<usize>> {
        let mut by_root: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for i in 0..self.0.len() {
            let r = self.root(i);
            by_root.entry(r).or_default().insert(i);
        }
        by_root.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(collapse_underscores("/I2C1__3V3"), "/I2C1_3V3");
        assert_eq!(collapse_underscores("/_I2C_"), "/_I2C_");
    }

    #[test]
    fn union_find() {
        let mut u = UnionFind::new(5);
        u.join(3, 1);
        u.join(4, 3);
        let sets: Vec<Vec<usize>> = u
            .sets()
            .into_iter()
            .map(|s| s.into_iter().collect())
            .collect();
        assert_eq!(sets, [vec![0], vec![1, 3, 4], vec![2]]);
    }
}
