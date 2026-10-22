//! Two boards' connectivity compared: each net as the set of `designator.pin` it connects, the nets of one
//! board matched to the other's by that set. This is the differential test between our KiCad importer and
//! KiCad's own netlist, and the base of a netlist diff.

use crate::canon::natural_cmp;
use crate::design::Board;
use std::collections::{BTreeSet, HashMap};

/// A net as the pins it connects
pub type PinSet = BTreeSet<String>;

/// How two boards' nets differ
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetDiff {
    /// Nets of the first board with no net of the same pins in the second, by name
    pub only_a: Vec<(String, PinSet)>,
    /// Nets of the second board with no net of the same pins in the first
    pub only_b: Vec<(String, PinSet)>,
    /// Same pins, different names: (name in a, name in b)
    pub renamed: Vec<(String, String)>,
    /// Nets with the same pins and the same name
    pub same: usize,
    /// Nets with the same pins whose resolved net classes differ, both stated: (name in a, class in a, class in b)
    pub classes: Vec<(String, String, String)>,
}

impl NetDiff {
    /// Same connectivity (names and classes may differ)
    pub fn same_connectivity(&self) -> bool {
        self.only_a.is_empty() && self.only_b.is_empty()
    }
}

/// Whether a part is in KiCad's netlist: not a virtual `#` part, not excluded from the board
pub fn netlisted(board: &Board, part: crate::design::PartId) -> bool {
    let p = board.part(part);
    !p.designator.starts_with('#') && p.flags.on_board.get().copied().unwrap_or(true)
}

/// The nets of a board as pin sets, by net name; nets without a netlisted pin are left out
pub fn pin_sets(board: &Board) -> Vec<(String, PinSet)> {
    let mut out = Vec::new();
    for net in &board.nets {
        let set: PinSet = net
            .pins
            .iter()
            .filter(|p| netlisted(board, board.pin(**p).part))
            .map(|p| board.pin_name(*p))
            .collect();
        if !set.is_empty() {
            out.push((net.name.clone(), set));
        }
    }
    out.sort_by(|a, b| natural_cmp(&a.0, &b.0));
    out
}

/// Compares the nets of `a` and `b`
pub fn compare(a: &Board, b: &Board) -> NetDiff {
    let sa = pin_sets(a);
    let sb = pin_sets(b);
    let mut by_set: HashMap<&PinSet, Vec<&str>> = HashMap::new();
    for (n, s) in &sb {
        by_set.entry(s).or_default().push(n);
    }
    let class = |board: &Board, name: &str| {
        board
            .net_by_name(name)
            .and_then(|i| board.net(i).class.get().cloned())
            .unwrap_or_default()
    };
    let mut d = NetDiff::default();
    let mut matched: HashMap<&PinSet, usize> = HashMap::new();
    for (n, s) in &sa {
        match by_set.get(s) {
            Some(names) => {
                let used = matched.entry(s).or_default();
                match names.get(*used) {
                    Some(other) => {
                        if *other == n {
                            d.same += 1;
                        } else {
                            d.renamed.push((n.clone(), other.to_string()));
                        }
                        let (ca, cb) = (class(a, n), class(b, other));
                        // A side that states no class (KiCad 8's netlist) is no disagreement
                        if ca != cb && !ca.is_empty() && !cb.is_empty() {
                            d.classes.push((n.clone(), ca, cb));
                        }
                    }
                    None => d.only_a.push((n.clone(), s.clone())),
                }
                *used += 1;
            }
            None => d.only_a.push((n.clone(), s.clone())),
        }
    }
    for (s, names) in &by_set {
        let used = matched.get(s).copied().unwrap_or(0);
        for n in names.iter().skip(used) {
            d.only_b.push((n.to_string(), (*s).clone()));
        }
    }
    d.only_b.sort_by(|x, y| natural_cmp(&x.0, &y.0));
    d
}
