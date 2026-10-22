//! Connectivity by KiCad's rules, per sheet instance, then across the hierarchy.
//!
//! 1. Geometry, per sheet instance: items touch at their connection points (wire and bus ends, pins, labels,
//!    sheet pins, junctions, no-connect flags, bus entry ends). A label also connects to a wire or bus running
//!    through its anchor (KiCad 8: the first one only), a junction (KiCad 9 and later) to every wire through its position; a pin or a wire
//!    end on the middle of another wire does not connect. Wires and buses never join; a bus entry joins a
//!    wire, not the bus it ends on. The connected groups are KiCad's subgraphs.
//! 2. Drivers: each subgraph is named by its strongest driver: global label, global power pin, local power
//!    pin, local label, hierarchical label, sheet pin, pin, in that order; ties are broken as KiCad does (power
//!    symbols first, output sheet pins first, names without `-Pad` first, then by name).
//! 3. Same sheet instance: strongly driven subgraphs sharing a driver name join.
//! 4. Hierarchy: a sheet pin joins the subgraphs of the child instance whose hierarchical label has its name;
//!    the joined group takes the best name (strong, then priority, then the shallowest sheet, then by name).
//!    Bus members follow their bus through sheet pins by index (vectors) or member name (groups).
//! 5. Nets are the subgraphs grouped by their final full name (`/MCU/SWDIO`, `GND`, `Net-(R1-Pad2)`).

use super::bus::Bus;
use super::parse::{SchFile, SchLabelKind};
use super::{Inst, SymRes, label_netname, unescape};
use erc_model::{EcadPinType, PowerScope, Pt};
use std::collections::{BTreeMap, HashMap, HashSet};

/// KiCad's driver priorities (`CONNECTION_SUBGRAPH::PRIORITY`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    None = 0,
    Pin,
    SheetPin,
    HierLabel,
    LocalLabel,
    LocalPowerPin,
    GlobalPowerPin,
    Global,
}

/// An item of a sheet instance that takes part in connectivity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    Wire(usize),
    Bus(usize),
    Junction(usize),
    NoConnect(usize),
    Label(usize),
    Entry(usize),
    /// Symbol index in the file, pin index in its library symbol
    Pin(usize, usize),
    /// Sheet index in the file, pin index on the sheet
    SheetPin(usize, usize),
}

/// What a net is named after
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Label,
    Power,
    Pin,
}

/// The result: final nets and, per instance, the net of every item
#[derive(Debug, Default)]
pub struct Connectivity {
    /// Net names and what named them
    pub nets: Vec<(String, NameKind)>,
    pub insts: Vec<InstNets>,
}

/// Nets of one sheet instance's items
#[derive(Debug, Default)]
pub struct InstNets {
    pub item_net: HashMap<Item, usize>,
    /// Items alone in their subgraph (a dangling label, an unconnected pin)
    pub alone: HashSet<Item>,
    /// Items in a subgraph with a no-connect flag
    pub no_connect: HashSet<Item>,
    /// For each no-connect flag: the pins in its subgraph
    pub nc_pins: HashMap<usize, Vec<Item>>,
}

/// A driver of a subgraph
#[derive(Clone, Debug)]
struct Driver {
    item: Item,
    prio: Priority,
    /// Name without the sheet path
    name: String,
    /// Full name
    full: String,
    /// For sorting: a pin of a global / local power symbol, an output sheet pin
    global_power_parent: bool,
    local_power_parent: bool,
    output_sheet_pin: bool,
    bus: Option<Bus>,
    /// For a pin: its name when KiCad finds it unconnected (`unconnected-(...)`)
    nc_name: Option<String>,
}

#[derive(Debug)]
struct Subgraph {
    inst: usize,
    items: Vec<Item>,
    drivers: Vec<Driver>,
    driver: Option<usize>,
    strong: bool,
    is_bus: bool,
    has_nc: bool,
    absorbed_into: Option<usize>,
}

/// A transform matrix as KiCad's `TRANSFORM` (x1, y1, x2, y2)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform(i64, i64, i64, i64);

impl Transform {
    /// The transform of a symbol placed with `rotation` degrees and an optional `(mirror x|y)`
    pub fn new(rotation: i32, mirror: Option<char>) -> Transform {
        const CCW: Transform = Transform(0, 1, -1, 0);
        const CW: Transform = Transform(0, -1, 1, 0);
        let mut t = Transform(1, 0, 0, -1);
        match rotation.rem_euclid(360) {
            90 => t = t.then(CCW),
            180 => t = t.then(CCW).then(CCW),
            270 => t = t.then(CW),
            _ => {}
        }
        match mirror {
            Some('x') => t = t.then(Transform(1, 0, 0, -1)),
            Some('y') => t = t.then(Transform(-1, 0, 0, 1)),
            _ => {}
        }
        t
    }

    fn then(self, o: Transform) -> Transform {
        let Transform(x1, y1, x2, y2) = self;
        Transform(
            x1 * o.0 + x2 * o.1,
            y1 * o.0 + y2 * o.1,
            x1 * o.2 + x2 * o.3,
            y1 * o.2 + y2 * o.3,
        )
    }

    pub fn apply(self, p: (i64, i64)) -> (i64, i64) {
        (self.0 * p.0 + self.1 * p.1, self.2 * p.0 + self.3 * p.1)
    }
}

/// Whether `p` lies on the segment `a`-`b` (ends included)
fn on_segment(p: Pt, a: Pt, b: Pt) -> bool {
    let cross =
        (b.x - a.x) as i128 * (p.y - a.y) as i128 - (b.y - a.y) as i128 * (p.x - a.x) as i128;
    cross == 0
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
}

struct Dsu(Vec<usize>);

impl Dsu {
    fn new(n: usize) -> Dsu {
        Dsu((0..n).collect())
    }
    fn find(&mut self, mut x: usize) -> usize {
        while self.0[x] != x {
            self.0[x] = self.0[self.0[x]];
            x = self.0[x];
        }
        x
    }
    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[b.max(a)] = b.min(a);
        }
    }
}

/// The pins a placed unit shows: library pin indices with their schematic positions
pub fn unit_pins(file: &SchFile, sym: usize, unit: u32) -> Vec<(usize, Pt)> {
    let s = &file.symbols[sym];
    let Some(lib) = file.lib_symbols.get(s.lib_key()) else {
        return Vec::new();
    };
    let t = Transform::new(s.rotation, s.mirror);
    lib.pins
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            (p.unit == 0 || p.unit == unit) && (p.body_style == 0 || p.body_style == s.body_style)
        })
        .map(|(i, p)| {
            let (dx, dy) = t.apply(p.at);
            (i, Pt::new(s.at.x + dx, s.at.y + dy))
        })
        .collect()
}

/// Unit letters as KiCad writes them after a reference: A..Z, then AA, AB, ...
pub fn unit_letter(unit: u32) -> String {
    let mut n = unit;
    let mut s = Vec::new();
    while n > 0 {
        let r = (n - 1) % 26;
        s.push((b'A' + r as u8) as char);
        n = (n - 1) / 26;
    }
    s.iter().rev().collect()
}

/// What connectivity is built from: the sheet files, their instances, the placed symbols per instance, the
/// KiCad major version whose rules apply, and the bus aliases by name (all sheets' and the project's)
pub struct Input<'a> {
    pub files: &'a [SchFile],
    pub insts: &'a [Inst],
    pub syms: &'a [Vec<SymRes>],
    pub kicad: u32,
    pub aliases: &'a HashMap<String, Vec<String>>,
}

/// Builds the connectivity of all sheet instances as KiCad major version `inp.kicad` does
pub fn build(inp: &Input) -> Connectivity {
    let (files, insts) = (inp.files, inp.insts);
    let mut sgs: Vec<Subgraph> = Vec::new();
    let mut inst_items: Vec<HashMap<Item, usize>> = Vec::new();
    for (ii, inst) in insts.iter().enumerate() {
        let (items_sg, new) = geometry(inp, ii, inst, sgs.len());
        sgs.extend(new);
        inst_items.push(items_sg);
    }

    // 3. Strong subgraphs of one sheet instance with a common driver name join
    let mut by_inst: Vec<Vec<usize>> = vec![Vec::new(); insts.len()];
    for (i, s) in sgs.iter().enumerate() {
        by_inst[s.inst].push(i);
    }
    let mut dsu = Dsu::new(sgs.len());
    for list in &by_inst {
        let mut by_name: HashMap<(bool, &str), usize> = HashMap::new();
        for &i in list {
            if !sgs[i].strong {
                continue;
            }
            for d in &sgs[i].drivers {
                if d.prio < Priority::HierLabel {
                    continue;
                }
                match by_name.get(&(sgs[i].is_bus, d.name.as_str())) {
                    Some(&j) => dsu.union(i, j),
                    None => {
                        by_name.insert((sgs[i].is_bus, d.name.as_str()), i);
                    }
                }
            }
        }
    }
    // Merge absorbed subgraphs into their root
    for i in 0..sgs.len() {
        let r = dsu.find(i);
        if r != i {
            let (items, drivers, nc) = {
                let s = &mut sgs[i];
                s.absorbed_into = Some(r);
                (
                    std::mem::take(&mut s.items),
                    std::mem::take(&mut s.drivers),
                    s.has_nc,
                )
            };
            let root = &mut sgs[r];
            root.items.extend(items);
            root.drivers.extend(drivers);
            root.has_nc |= nc;
        }
    }
    for s in sgs.iter_mut().filter(|s| s.absorbed_into.is_none()) {
        resolve(s);
    }
    let root_of = |i: usize, sgs: &[Subgraph]| sgs[i].absorbed_into.unwrap_or(i);

    // 4. Hierarchy: sheet pins to the hierarchical labels of the child instance
    let mut child_of: HashMap<(usize, usize), usize> = HashMap::new();
    for (ii, inst) in insts.iter().enumerate() {
        if let (Some(p), Some(s)) = (inst.parent, inst.sheet_in_parent) {
            child_of.insert((p, s), ii);
        }
    }
    // Hierarchical labels per instance and name
    let mut hier: HashMap<(usize, String), Vec<usize>> = HashMap::new();
    for (i, s) in sgs.iter().enumerate() {
        if s.absorbed_into.is_some() {
            continue;
        }
        for it in &s.items {
            if let Item::Label(l) = it {
                let file = &files[insts[s.inst].file];
                let lab = &file.labels[*l];
                if lab.kind == SchLabelKind::Hierarchical {
                    hier.entry((s.inst, lab.text.clone())).or_default().push(i);
                }
            }
        }
    }
    let mut hdsu = Dsu::new(sgs.len());
    for (i, s) in sgs.iter().enumerate() {
        if s.absorbed_into.is_some() {
            continue;
        }
        for it in &s.items {
            if let Item::SheetPin(sh, p) = it {
                let file = &files[insts[s.inst].file];
                let pin = &file.sheets[*sh].pins[*p];
                let Some(&child) = child_of.get(&(s.inst, *sh)) else {
                    continue;
                };
                if let Some(list) = hier.get(&(child, pin.name.clone())) {
                    for &c in list {
                        if sgs[c].is_bus == s.is_bus {
                            hdsu.union(i, c);
                        }
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, s) in sgs.iter().enumerate() {
        if s.absorbed_into.is_none() {
            groups.entry(hdsu.find(i)).or_default().push(i);
        }
    }
    // The name each live subgraph ends with, and what named it
    let mut name: Vec<Option<(String, NameKind)>> = vec![None; sgs.len()];
    let mut best_of: HashMap<usize, usize> = HashMap::new();
    for members in groups.values() {
        let best = members
            .iter()
            .copied()
            .filter(|&m| sgs[m].driver.is_some())
            .max_by(|&a, &b| rank(&sgs[a], insts).cmp(&rank(&sgs[b], insts)));
        let Some(best) = best else { continue };
        let d = &sgs[best].drivers[sgs[best].driver.unwrap_or(0)];
        let kind = match d.item {
            Item::Pin(..) if d.prio >= Priority::LocalPowerPin => NameKind::Power,
            Item::Pin(..) => NameKind::Pin,
            _ => NameKind::Label,
        };
        for &m in members {
            name[m] = Some((d.full.clone(), kind));
            best_of.insert(m, best);
        }
    }

    // Bus members follow their bus through the hierarchy
    for members in groups.values() {
        let Some(&best) = members.first().and_then(|m| best_of.get(m)) else {
            continue;
        };
        if !sgs[best].is_bus || members.len() < 2 {
            continue;
        }
        let bd = &sgs[best].drivers[sgs[best].driver.unwrap_or(0)];
        let Some(best_bus) = &bd.bus else { continue };
        let prefix = &bd.full[..bd.full.len() - bd.name.len()];
        let best_members: HashMap<String, String> = best_bus
            .member_keys()
            .into_iter()
            .zip(best_bus.member_names())
            .collect();
        for &m in members {
            // Every bus name in this subgraph: its members, by key
            for d in &sgs[m].drivers {
                let Some(b) = &d.bus else { continue };
                for (key, local) in b.member_keys().into_iter().zip(b.member_names()) {
                    let Some(target) = best_members.get(&key) else {
                        continue;
                    };
                    let target = format!("{prefix}{target}");
                    // Net subgraphs of this instance driven by the member's name
                    for &n in &by_inst[sgs[m].inst] {
                        let n = root_of(n, &sgs);
                        let s = &sgs[n];
                        if s.is_bus || !s.strong {
                            continue;
                        }
                        let Some(di) = s.driver else { continue };
                        if s.drivers[di].prio >= Priority::GlobalPowerPin {
                            continue;
                        }
                        if s.drivers
                            .iter()
                            .any(|d| d.prio >= Priority::HierLabel && d.name == local)
                        {
                            // Rename the member net's whole hierarchical group
                            let g = hdsu.find(n);
                            for &x in &groups[&g] {
                                name[x] = Some((target.clone(), NameKind::Label));
                            }
                        }
                    }
                }
            }
        }
    }

    // A weakly driven subgraph whose only driver is one pin is "unconnected-(...)"
    for (i, s) in sgs.iter().enumerate() {
        if s.absorbed_into.is_some() || s.strong || s.drivers.len() != 1 {
            continue;
        }
        if let Some(n) = &s.drivers[0].nc_name {
            name[i] = Some((n.clone(), NameKind::Pin));
        }
    }

    // 5. Nets by final name
    let mut out = Connectivity::default();
    let mut net_ids: HashMap<String, usize> = HashMap::new();
    let mut sg_net: Vec<Option<usize>> = vec![None; sgs.len()];
    for (i, s) in sgs.iter().enumerate() {
        if s.absorbed_into.is_some() || s.is_bus {
            continue;
        }
        if let Some((n, kind)) = &name[i] {
            let id = *net_ids.entry(n.clone()).or_insert_with(|| {
                out.nets.push((n.clone(), *kind));
                out.nets.len() - 1
            });
            sg_net[i] = Some(id);
        }
    }
    out.insts = (0..insts.len()).map(|_| InstNets::default()).collect();
    for (ii, items) in inst_items.iter().enumerate() {
        let res = &mut out.insts[ii];
        for (item, &sg) in items {
            let r = root_of(sg, &sgs);
            if let Some(n) = sg_net[r] {
                res.item_net.insert(*item, n);
            }
            if sgs[r].has_nc {
                res.no_connect.insert(*item);
            }
        }
        // Alone: the subgraph (before joining by name) holds only this item
        let mut count: HashMap<usize, usize> = HashMap::new();
        for &sg in items.values() {
            *count.entry(sg).or_default() += 1;
        }
        for (item, sg) in items {
            if count[sg] == 1 {
                res.alone.insert(*item);
            }
        }
        for (item, sg) in items {
            if let Item::NoConnect(nc) = item {
                let pins: Vec<Item> = items
                    .iter()
                    .filter(|(i, s)| *s == sg && matches!(i, Item::Pin(..)))
                    .map(|(i, _)| *i)
                    .collect();
                res.nc_pins.insert(*nc, pins);
            }
        }
    }
    out
}

/// Ordering of the best driver in a hierarchical group: global names first, then strong, priority, the
/// shallowest sheet, the smallest name
fn rank(
    s: &Subgraph,
    insts: &[Inst],
) -> (
    bool,
    bool,
    Priority,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<String>,
) {
    let d = &s.drivers[s.driver.unwrap_or(0)];
    (
        d.prio >= Priority::GlobalPowerPin,
        s.strong,
        d.prio,
        std::cmp::Reverse(insts[s.inst].depth),
        std::cmp::Reverse(d.full.clone()),
    )
}

/// Picks the subgraph's driver by priority and KiCad's tie rules; drops weak drivers of a strong subgraph
fn resolve(s: &mut Subgraph) {
    let Some(top) = s.drivers.iter().map(|d| d.prio).max() else {
        s.driver = None;
        s.strong = false;
        return;
    };
    s.strong = top >= Priority::HierLabel;
    if top == Priority::None {
        s.driver = None;
        return;
    }
    let mut best: Option<usize> = None;
    for (i, d) in s.drivers.iter().enumerate() {
        if d.prio != top {
            continue;
        }
        best = Some(match best {
            None => i,
            Some(b) if better(d, &s.drivers[b]) => i,
            Some(b) => b,
        });
    }
    s.driver = best;
    // A sheet pin that drives its subgraph is promoted to strong (no conflict check needed for joining)
    if top == Priority::SheetPin {
        s.strong = true;
    }
    if top >= Priority::HierLabel {
        let keep: Vec<Driver> = s
            .drivers
            .iter()
            .filter(|d| d.prio >= Priority::HierLabel)
            .cloned()
            .collect();
        let chosen = best.map(|b| s.drivers[b].item);
        s.drivers = keep;
        s.driver = chosen.and_then(|c| s.drivers.iter().position(|d| d.item == c));
    }
}

/// KiCad's candidate order among drivers of equal priority
fn better(a: &Driver, b: &Driver) -> bool {
    if let (Some(ab), Some(bb)) = (&a.bus, &b.bus) {
        let am = ab.member_names();
        let bm = bb.member_names();
        let a_sup = bm.iter().all(|m| am.contains(m));
        let b_sup = am.iter().all(|m| bm.contains(m));
        if a_sup != b_sup {
            return a_sup;
        }
    }
    if matches!(a.item, Item::Pin(..)) && matches!(b.item, Item::Pin(..)) {
        if a.global_power_parent != b.global_power_parent {
            return a.global_power_parent;
        }
        if a.local_power_parent != b.local_power_parent {
            return a.local_power_parent;
        }
    }
    if matches!(a.item, Item::SheetPin(..))
        && matches!(b.item, Item::SheetPin(..))
        && a.output_sheet_pin != b.output_sheet_pin
    {
        return a.output_sheet_pin;
    }
    let a_low = a.name.contains("-Pad");
    let b_low = b.name.contains("-Pad");
    if a_low != b_low {
        return !a_low;
    }
    a.name < b.name
}

/// The subgraphs of one sheet instance, numbered from `base`; and the subgraph of each item
fn geometry(
    inp: &Input,
    ii: usize,
    inst: &Inst,
    base: usize,
) -> (HashMap<Item, usize>, Vec<Subgraph>) {
    let (files, syms, kicad) = (inp.files, inp.syms, inp.kicad);
    let f = &files[inst.file];
    let mut items: Vec<Item> = Vec::new();
    let mut pts: HashMap<Pt, Vec<usize>> = HashMap::new();
    let mut add = |items: &mut Vec<Item>, it: Item, at: &[Pt]| {
        let i = items.len();
        items.push(it);
        for p in at {
            pts.entry(*p).or_default().push(i);
        }
    };
    for (i, w) in f.wires.iter().enumerate() {
        add(&mut items, Item::Wire(i), &[w.0, w.1]);
    }
    for (i, b) in f.buses.iter().enumerate() {
        add(&mut items, Item::Bus(i), &[b.0, b.1]);
    }
    for (i, j) in f.junctions.iter().enumerate() {
        add(&mut items, Item::Junction(i), &[*j]);
    }
    for (i, n) in f.no_connects.iter().enumerate() {
        add(&mut items, Item::NoConnect(i), &[*n]);
    }
    for (i, l) in f.labels.iter().enumerate() {
        add(&mut items, Item::Label(i), &[l.at]);
    }
    for (i, e) in f.bus_entries.iter().enumerate() {
        add(&mut items, Item::Entry(i), &[e.0, e.1]);
    }
    let mut pin_pos: HashMap<(usize, usize), Pt> = HashMap::new();
    for (si, _) in f.symbols.iter().enumerate() {
        let unit = syms[ii][si].unit;
        for (pi, at) in unit_pins(f, si, unit) {
            pin_pos.insert((si, pi), at);
            add(&mut items, Item::Pin(si, pi), &[at]);
        }
    }
    for (si, sh) in f.sheets.iter().enumerate() {
        for (pi, p) in sh.pins.iter().enumerate() {
            add(&mut items, Item::SheetPin(si, pi), &[p.at]);
        }
    }

    let is_wire = |it: &Item| matches!(it, Item::Wire(_));
    let is_bus = |it: &Item| matches!(it, Item::Bus(_));
    let bus_at = |p: Pt| f.buses.iter().any(|b| on_segment(p, b.0, b.1));
    let mut dsu = Dsu::new(items.len());
    for (p, list) in &pts {
        let has_bus = bus_at(*p);
        for (x, &a) in list.iter().enumerate() {
            for &b in &list[x + 1..] {
                let (ia, ib) = (&items[a], &items[b]);
                // Wires and buses never join; pins and no-connect flags only join wires
                if (is_wire(ia) && is_bus(ib)) || (is_bus(ia) && is_wire(ib)) {
                    continue;
                }
                let wire_only = |it: &Item| matches!(it, Item::Pin(..) | Item::NoConnect(_));
                if (wire_only(ia) && is_bus(ib)) || (is_bus(ia) && wire_only(ib)) {
                    continue;
                }
                // A bus entry does not join the bus it ends on, nor anything else at that end
                let entry = |it: &Item| matches!(it, Item::Entry(_));
                if (entry(ia) || entry(ib)) && has_bus {
                    continue;
                }
                dsu.union(a, b);
            }
        }
    }
    // Labels join wires and buses through their anchor; junctions join wires and buses through their position
    let segs: Vec<(usize, Pt, Pt, bool)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, it)| match it {
            Item::Wire(w) => Some((i, f.wires[*w].0, f.wires[*w].1, false)),
            Item::Bus(b) => Some((i, f.buses[*b].0, f.buses[*b].1, true)),
            _ => None,
        })
        .collect();
    // KiCad 8 joins a junction only to what ends at it; KiCad 9 and later also to wires running through it
    let junction_midpoints = kicad >= 9;
    for (i, it) in items.iter().enumerate() {
        let at = match it {
            Item::Label(l) => f.labels[*l].at,
            Item::Junction(j) if junction_midpoints => f.junctions[*j],
            _ => continue,
        };
        for &(si, a, b, _) in &segs {
            if on_segment(at, a, b) {
                dsu.union(i, si);
                // KiCad 8 gives a label on a crossing of wires to one of them only: the first in the file
                if kicad < 9 && matches!(it, Item::Label(_)) {
                    break;
                }
            }
        }
    }

    let mut comp: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..items.len() {
        comp.entry(dsu.find(i)).or_default().push(i);
    }
    let mut out = Vec::new();
    let mut item_sg = HashMap::new();
    for members in comp.values() {
        let idx = base + out.len();
        let its: Vec<Item> = members.iter().map(|&m| items[m]).collect();
        for it in &its {
            item_sg.insert(*it, idx);
        }
        out.push(subgraph(inp, ii, its));
    }
    (item_sg, out)
}

/// A subgraph with its drivers resolved
fn subgraph(inp: &Input, ii: usize, items: Vec<Item>) -> Subgraph {
    let (files, insts, syms) = (inp.files, inp.insts, inp.syms);
    let inst = &insts[ii];
    let f = &files[inst.file];
    let has_nc = items.iter().any(|it| match it {
        Item::NoConnect(_) => true,
        Item::Pin(s, p) => f
            .lib_symbols
            .get(f.symbols[*s].lib_key())
            .is_some_and(|l| l.pins[*p].etype == EcadPinType::NoConnect),
        _ => false,
    });
    let mut drivers = Vec::new();
    for it in &items {
        let d = match *it {
            Item::Label(l) => {
                let lab = &f.labels[l];
                let prio = match lab.kind {
                    SchLabelKind::Local => Priority::LocalLabel,
                    SchLabelKind::Global => Priority::Global,
                    SchLabelKind::Hierarchical => Priority::HierLabel,
                    SchLabelKind::Directive => continue,
                };
                let name = label_netname(&lab.text);
                let full = if lab.kind == SchLabelKind::Global {
                    name.clone()
                } else {
                    format!("{}{name}", inst.path)
                };
                Driver {
                    item: *it,
                    prio,
                    bus: Bus::parse(&unescape(&lab.text)).map(|b| b.with_aliases(inp.aliases)),
                    name,
                    full,
                    global_power_parent: false,
                    local_power_parent: false,
                    output_sheet_pin: false,
                    nc_name: None,
                }
            }
            Item::SheetPin(s, p) => {
                let pin = &f.sheets[s].pins[p];
                let name = label_netname(&pin.name);
                Driver {
                    item: *it,
                    prio: Priority::SheetPin,
                    bus: Bus::parse(&unescape(&pin.name)).map(|b| b.with_aliases(inp.aliases)),
                    full: format!("{}{name}", inst.path),
                    name,
                    global_power_parent: false,
                    local_power_parent: false,
                    output_sheet_pin: pin.shape == "output",
                    nc_name: None,
                }
            }
            Item::Pin(s, p) => pin_driver(f, inst, &syms[ii][s], s, p, has_nc),
            _ => continue,
        };
        drivers.push(d);
    }
    let mut sg = Subgraph {
        inst: ii,
        items,
        drivers,
        driver: None,
        strong: false,
        is_bus: false,
        has_nc,
        absorbed_into: None,
    };
    resolve(&mut sg);
    sg.is_bus = sg.driver.is_some_and(|d| sg.drivers[d].bus.is_some());
    sg
}

/// A symbol pin as a driver: power pins name their net, other pins get KiCad's default net name
fn pin_driver(f: &SchFile, inst: &Inst, res: &SymRes, s: usize, p: usize, has_nc: bool) -> Driver {
    let sym = &f.symbols[s];
    let lib = f.lib_symbols.get(sym.lib_key());
    let pin = lib.map(|l| &l.pins[p]);
    let power = lib.and_then(|l| l.power);
    let is_power_in = pin.is_some_and(|p| p.etype == EcadPinType::PowerIn);
    let hidden = pin.is_some_and(|p| p.hidden);
    let global_power = is_power_in
        && (power == Some(PowerScope::Global) || (power != Some(PowerScope::Local) && hidden));
    let local_power = is_power_in && power == Some(PowerScope::Local);
    let value = sym.property("Value").unwrap_or("");
    let mut nc_name = None;
    let (prio, name, full) = if global_power || local_power {
        let name = if power.is_some() {
            label_netname(value)
        } else {
            label_netname(pin.map_or("", |p| p.name.as_str()))
        };
        if local_power {
            (
                Priority::LocalPowerPin,
                name.clone(),
                format!("{}{name}", inst.path),
            )
        } else {
            (Priority::GlobalPowerPin, name.clone(), name)
        }
    } else {
        let excluded = res.reference.starts_with('#') || res.on_board == Some(false);
        let prio = if excluded {
            Priority::None
        } else {
            Priority::Pin
        };
        let name = default_pin_name(f, res, s, p, has_nc);
        nc_name = Some(default_pin_name(f, res, s, p, true));
        (prio, name.clone(), name)
    };
    Driver {
        item: Item::Pin(s, p),
        prio,
        bus: None,
        name,
        full,
        global_power_parent: power == Some(PowerScope::Global),
        local_power_parent: power == Some(PowerScope::Local),
        output_sheet_pin: false,
        nc_name,
    }
}

/// The name a pin shows: the alternate chosen, else the library name
fn shown_name(f: &SchFile, s: usize, p: usize) -> String {
    let sym = &f.symbols[s];
    let Some(lib) = f.lib_symbols.get(sym.lib_key()) else {
        return String::new();
    };
    let pin = &lib.pins[p];
    if let Some(alt) = sym.alternates.get(&pin.number) {
        return alt.clone();
    }
    pin.name.clone()
}

/// KiCad's default net name of a pin: `Net-(R1-Pad2)`, `Net-(U1-PA3)`, `unconnected-(U1A-+INA-Pad3)`
fn default_pin_name(f: &SchFile, res: &SymRes, s: usize, p: usize, force_nc: bool) -> String {
    let sym = &f.symbols[s];
    let Some(lib) = f.lib_symbols.get(sym.lib_key()) else {
        return format!("Net-({}-Pad?)", res.reference);
    };
    let pin = &lib.pins[p];
    let unconnected = force_nc || pin.etype == EcadPinType::NoConnect;
    let mut name = String::from(if unconnected {
        "unconnected-("
    } else {
        "Net-("
    });
    let shown = shown_name(f, s, p);
    let number = &pin.number;
    let has_multiple = unit_pins(f, s, res.unit).iter().any(|&(q, _)| {
        let other = &lib.pins[q];
        shown_name(f, s, q) == shown
            && other.number != *number
            && unconnected == (other.etype == EcadPinType::NoConnect)
    });
    if res.reference.ends_with('?') {
        let hash = sym
            .uuid
            .chars()
            .filter(|c| *c != '-')
            .take(8)
            .collect::<String>();
        name += &format!("{}-{hash}-Pad{number})", res.reference);
    } else if !shown.is_empty() && shown != *number {
        name += &res.reference;
        if lib.units > 1 {
            name += &unit_letter(res.unit);
        }
        name += "-";
        name += &label_netname(&shown);
        if unconnected || has_multiple {
            name += &format!("-Pad{}", label_netname(number));
        }
        name += ")";
    } else {
        name += &format!("{}-Pad{})", res.reference, label_netname(number));
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transforms() {
        // Library y is up, the sheet's y is down
        assert_eq!(Transform::new(0, None).apply((0, 10)), (0, -10));
        assert_eq!(Transform::new(90, None).apply((0, 10)), (-10, 0));
        assert_eq!(Transform::new(180, None).apply((0, 10)), (0, 10));
        assert_eq!(Transform::new(270, None).apply((0, 10)), (10, 0));
        assert_eq!(Transform::new(0, Some('y')).apply((5, 0)), (-5, 0));
        assert_eq!(Transform::new(0, Some('x')).apply((0, 10)), (0, 10));
    }

    #[test]
    fn unit_letters() {
        assert_eq!(unit_letter(1), "A");
        assert_eq!(unit_letter(26), "Z");
        assert_eq!(unit_letter(27), "AA");
    }

    #[test]
    fn segments() {
        let (a, b) = (Pt::new(0, 0), Pt::new(10, 0));
        assert!(on_segment(Pt::new(5, 0), a, b));
        assert!(on_segment(Pt::new(10, 0), a, b));
        assert!(!on_segment(Pt::new(11, 0), a, b));
        assert!(!on_segment(Pt::new(5, 1), a, b));
    }
}
