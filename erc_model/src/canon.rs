//! The canonical order of a board's arenas: every importer ends with [`Board::canonicalize`], so two imports of
//! the same design are equal value for value, whatever order the files listed things in, and the dump reader
//! gets back exactly the board that was dumped.

use crate::design::{Board, LabelId, NetId, PartId, PinId, SheetId, SymbolId, VariantId};
use crate::dump;
use crate::layout::{FootprintId, PadId};
use std::cmp::Ordering;
use std::collections::HashMap;

/// Compares strings with digit runs as numbers: `R2` before `R10`, `/A/` before `/B/`
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.as_bytes(), b.as_bytes());
    loop {
        match (a.first(), b.first()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let da = a.iter().take_while(|c| c.is_ascii_digit()).count();
                let db = b.iter().take_while(|c| c.is_ascii_digit()).count();
                let (na, nb) = (&a[..da], &b[..db]);
                let ta = trim_zeros(na);
                let tb = trim_zeros(nb);
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                // Equal numbers: fewer leading zeros first, so the order stays total
                let ord = ord.then_with(|| na.len().cmp(&nb.len()));
                if ord != Ordering::Equal {
                    return ord;
                }
                a = &a[da..];
                b = &b[db..];
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(y);
                }
                a = &a[1..];
                b = &b[1..];
            }
        }
    }
}

fn trim_zeros(d: &[u8]) -> &[u8] {
    let n = d.iter().take_while(|c| **c == b'0').count();
    &d[n..]
}

/// Sorts `v` by `key` (stable) and returns the old index → new index map
fn sort_remap<T, K: Ord>(v: &mut Vec<T>, key: impl Fn(&T) -> K) -> Vec<usize> {
    sort_remap_with(v, |_, x| key(x))
}

/// A string key ordered naturally
#[derive(PartialEq, Eq)]
struct Nat(String);

impl PartialOrd for Nat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Nat {
    fn cmp(&self, other: &Self) -> Ordering {
        natural_cmp(&self.0, &other.0)
    }
}

impl Board {
    /// What every importer does last: the adjacency rebuilt from the pins and labels, the layout linked to the
    /// schematic by name, and the canonical order
    pub fn finish(&mut self) {
        rebuild_adjacency(self);
        link_layout(self);
        self.canonicalize();
    }

    /// Puts every arena and list into the canonical order and remaps the ids. Idempotent.
    pub fn canonicalize(&mut self) {
        // Sheets: root first, then by path
        let paths: Vec<String> = (0..self.sheets.len())
            .map(|i| self.sheet_path(SheetId::from_index(i)))
            .collect();
        let map = sort_remap_with(&mut self.sheets, |i, s| {
            (
                s.parent.is_some(),
                Nat(paths[i].clone()),
                s.uuid_path.clone(),
            )
        });
        let sheet = |s: SheetId| SheetId::from_index(map[s.index()]);
        for s in &mut self.sheets {
            s.parent = s.parent.map(sheet);
        }
        for p in &mut self.parts {
            p.sheet = sheet(p.sheet);
            for u in &mut p.units {
                u.sheet = sheet(u.sheet);
            }
        }
        for l in &mut self.labels {
            l.sheet = sheet(l.sheet);
            l.child = l.child.map(sheet);
        }
        for n in &mut self.no_connects {
            n.sheet = sheet(n.sheet);
        }

        // Symbols by name
        let map = sort_remap(&mut self.symbols, |s| Nat(s.name.clone()));
        for p in &mut self.parts {
            p.symbol = p.symbol.map(|s| SymbolId::from_index(map[s.index()]));
        }
        for s in &mut self.symbols {
            s.footprint_filters.sort();
            s.fields.sort();
            s.pins.sort_by(|a, b| {
                natural_cmp(&a.number, &b.number)
                    .then(a.unit.cmp(&b.unit))
                    .then(a.body_style.cmp(&b.body_style))
                    .then_with(|| natural_cmp(&a.name, &b.name))
            });
            for p in &mut s.pins {
                p.alternates.sort_by(|a, b| natural_cmp(&a.name, &b.name));
            }
            for g in &mut s.jumper_pin_groups {
                g.sort_by(|a, b| natural_cmp(a, b));
            }
            s.jumper_pin_groups.sort();
        }

        // Variants by name
        let map = sort_remap(&mut self.variants, |v| Nat(v.name.clone()));
        for p in &mut self.parts {
            for v in &mut p.variants {
                v.variant = VariantId::from_index(map[v.variant.index()]);
            }
            p.variants.sort_by_key(|v| v.variant);
        }

        // Parts by designator
        let map = sort_remap(&mut self.parts, |p| {
            (
                Nat(p.designator.clone()),
                p.units.first().map(|u| u.uuid.clone()),
            )
        });
        let part = |p: PartId| PartId::from_index(map[p.index()]);
        for p in &mut self.pins {
            p.part = part(p.part);
        }
        if let Some(l) = &mut self.layout {
            for f in &mut l.footprints {
                f.part = f.part.map(part);
            }
        }
        for p in &mut self.parts {
            p.units
                .sort_by(|a, b| (a.unit, &a.uuid).cmp(&(b.unit, &b.uuid)));
            p.fields.sort_by(|a, b| natural_cmp(&a.name, &b.name));
            for v in &mut p.variants {
                v.fields.sort_by(|a, b| natural_cmp(&a.name, &b.name));
            }
            for t in &mut p.ties {
                t.sort_by(|a, b| natural_cmp(a, b));
            }
            p.ties.sort();
        }

        // Nets by name (before pins, whose sort key does not use them)
        let map = sort_remap(&mut self.nets, |n| Nat(n.name.clone()));
        let net = |n: NetId| NetId::from_index(map[n.index()]);
        for p in &mut self.pins {
            p.net = p.net.map(net);
        }
        for l in &mut self.labels {
            l.net = l.net.map(net);
        }
        if let Some(lay) = &mut self.layout {
            for p in &mut lay.pads {
                p.net = p.net.map(net);
            }
        }

        // Pins by part, number, unit
        let map = sort_remap(&mut self.pins, |p| {
            (
                p.part,
                Nat(p.number.clone()),
                p.unit,
                p.net,
                Nat(p.name.clone()),
            )
        });
        let pin = |p: PinId| PinId::from_index(map[p.index()]);
        for p in &mut self.parts {
            for x in &mut p.pins {
                *x = pin(*x);
            }
            p.pins.sort();
        }
        for n in &mut self.nets {
            for x in &mut n.pins {
                *x = pin(*x);
            }
            n.pins.sort();
        }
        for n in &mut self.no_connects {
            n.pin = n.pin.map(pin);
        }

        // Labels and no-connect flags by their dump line
        let lines: Vec<String> = self
            .labels
            .iter()
            .map(|l| dump::label_line(self, l))
            .collect();
        let map = sort_remap_with(&mut self.labels, |i, _| Nat(lines[i].clone()));
        for n in &mut self.nets {
            for x in &mut n.labels {
                *x = LabelId::from_index(map[x.index()]);
            }
            n.labels.sort();
        }
        let lines: Vec<String> = self
            .no_connects
            .iter()
            .map(|n| dump::nc_line(self, n))
            .collect();
        sort_remap_with(&mut self.no_connects, |i, _| Nat(lines[i].clone()));

        self.net_classes
            .sort_by(|a, b| natural_cmp(&a.name, &b.name));
        for c in &mut self.net_classes {
            c.params.sort_by(|a, b| natural_cmp(&a.key, &b.key));
            c.patterns.sort();
        }
        let s = &mut self.settings;
        for list in [
            &mut s.text_vars,
            &mut s.erc_severities,
            &mut s.erc_exclusions,
            &mut s.drc_severities,
            &mut s.drc_exclusions,
            &mut s.rules,
        ] {
            list.sort_by(|a, b| natural_cmp(&a.key, &b.key));
        }
        self.source.files.sort();

        self.canonicalize_layout();
        self.canonicalize_facts();
    }

    /// Every fact list in canonical order
    fn canonicalize_facts(&mut self) {
        fn flags(f: &mut crate::design::PartFlags) {
            for name in crate::design::PartFlags::NAMES {
                if let Some(x) = f.by_name_mut(name) {
                    x.canonicalize();
                }
            }
        }
        for p in &mut self.parts {
            p.fields.iter_mut().for_each(|f| f.value.canonicalize());
            p.value.canonicalize();
            flags(&mut p.flags);
            for v in &mut p.variants {
                flags(&mut v.flags);
                v.fields.iter_mut().for_each(|f| f.value.canonicalize());
            }
        }
        self.pins.iter_mut().for_each(|p| p.etype.canonicalize());
        self.nets.iter_mut().for_each(|n| n.class.canonicalize());
        for c in &mut self.net_classes {
            c.params.iter_mut().for_each(|p| p.value.canonicalize());
        }
        let s = &mut self.settings;
        for list in [
            &mut s.text_vars,
            &mut s.erc_severities,
            &mut s.erc_exclusions,
            &mut s.drc_severities,
            &mut s.drc_exclusions,
            &mut s.rules,
        ] {
            list.iter_mut().for_each(|p| p.value.canonicalize());
        }
        if let Some(l) = &mut self.layout {
            for f in &mut l.footprints {
                f.fields.iter_mut().for_each(|x| x.value.canonicalize());
            }
        }
    }

    fn canonicalize_layout(&mut self) {
        let Some(lay) = &mut self.layout else { return };
        let map = sort_remap(&mut lay.footprints, |f| {
            (Nat(f.reference.clone()), f.at, Nat(f.lib_id.clone()))
        });
        for p in &mut lay.pads {
            p.footprint = FootprintId::from_index(map[p.footprint.index()]);
        }
        let map = sort_remap(&mut lay.pads, |p| {
            (p.footprint, Nat(p.number.clone()), p.at)
        });
        for f in &mut lay.footprints {
            for x in &mut f.pads {
                *x = PadId::from_index(map[x.index()]);
            }
            f.pads.sort();
            f.fields.sort_by(|a, b| natural_cmp(&a.name, &b.name));
            let mut g: Vec<(String, _)> = f
                .graphics
                .drain(..)
                .map(|s| (dump::shape_tokens(&s).join(" "), s))
                .collect();
            g.sort_by(|a, b| natural_cmp(&a.0, &b.0));
            f.graphics = g.into_iter().map(|x| x.1).collect();
            let mut t: Vec<(String, _)> = f
                .texts
                .drain(..)
                .map(|s| (dump::text_tokens(&s).join(" "), s))
                .collect();
            t.sort_by(|a, b| natural_cmp(&a.0, &b.0));
            f.texts = t.into_iter().map(|x| x.1).collect();
            for grp in &mut f.net_tie_pad_groups {
                grp.sort_by(|a, b| natural_cmp(a, b));
            }
            f.net_tie_pad_groups.sort();
            f.models.sort();
        }
        for p in &mut lay.pads {
            let mut g: Vec<(String, _)> = p
                .primitives
                .drain(..)
                .map(|s| (dump::shape_tokens(&s).join(" "), s))
                .collect();
            g.sort_by(|a, b| natural_cmp(&a.0, &b.0));
            p.primitives = g.into_iter().map(|x| x.1).collect();
        }
        lay.net_names.sort_by(|a, b| natural_cmp(a, b));
        lay.net_names.dedup();
        sort_by_line(&mut lay.tracks, dump::track_tokens);
        sort_by_line(&mut lay.vias, dump::via_tokens);
        sort_by_line(&mut lay.zones, dump::zone_tokens);
        sort_by_line(&mut lay.graphics, dump::shape_tokens);
        sort_by_line(&mut lay.texts, dump::text_tokens);
        lay.setup.sort();
    }
}

fn sort_by_line<T>(v: &mut Vec<T>, tokens: impl Fn(&T) -> Vec<String>) {
    let lines: Vec<String> = v.iter().map(|x| tokens(x).join(" ")).collect();
    sort_remap_with(v, |i, _| Nat(lines[i].clone()));
}

/// Sorts `v` by `key(old index, item)` (stable) and returns the old index → new index map
fn sort_remap_with<T, K: Ord>(v: &mut Vec<T>, key: impl Fn(usize, &T) -> K) -> Vec<usize> {
    let keys: Vec<K> = v.iter().enumerate().map(|(i, x)| key(i, x)).collect();
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|a, b| keys[*a].cmp(&keys[*b]).then(a.cmp(b)));
    let mut old: Vec<Option<T>> = std::mem::take(v).into_iter().map(Some).collect();
    let mut map = vec![0; idx.len()];
    for (new, &o) in idx.iter().enumerate() {
        map[o] = new;
        if let Some(item) = old[o].take() {
            v.push(item);
        }
    }
    map
}

/// Rebuilds the derived adjacency from the pins and labels: each net's pin and label lists, each part's pin list
pub fn rebuild_adjacency(board: &mut Board) {
    for n in &mut board.nets {
        n.pins.clear();
        n.labels.clear();
    }
    for p in &mut board.parts {
        p.pins.clear();
    }
    for (i, pin) in board.pins.iter().enumerate() {
        let id = PinId::from_index(i);
        board.parts[pin.part.index()].pins.push(id);
        if let Some(n) = pin.net {
            board.nets[n.index()].pins.push(id);
        }
    }
    for (i, l) in board.labels.iter().enumerate() {
        if let Some(n) = l.net {
            board.nets[n.index()].labels.push(LabelId::from_index(i));
        }
    }
}

/// Links the layout to the schematic by name: footprints to parts by designator, pads to nets by net name
pub fn link_layout(board: &mut Board) {
    let parts: HashMap<&str, PartId> = board
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| (p.designator.as_str(), PartId::from_index(i)))
        .collect();
    let nets: HashMap<&str, NetId> = board
        .nets
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.as_str(), NetId::from_index(i)))
        .collect();
    let Some(lay) = &mut board.layout else { return };
    for f in &mut lay.footprints {
        f.part = parts.get(f.reference.as_str()).copied();
    }
    for p in &mut lay.pads {
        p.net = p.net_name.as_deref().and_then(|n| nets.get(n).copied());
    }
    for f in &mut lay.footprints {
        f.pads.clear();
    }
    for (i, p) in lay.pads.iter().enumerate() {
        lay.footprints[p.footprint.index()]
            .pads
            .push(PadId::from_index(i));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v = vec!["R10", "R2", "R1", "C1", "R02", "/B/", "/A/x", "R"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["/A/x", "/B/", "C1", "R", "R1", "R2", "R02", "R10"]);
    }
}
