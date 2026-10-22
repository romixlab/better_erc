//! The power structure by name and pin type (the first layer of NET-5), ported from `erc_core`'s `power.rs` onto
//! the board model: rails, grounds and the switching nodes of converters. Every rail is a [`Fact`] with the rule
//! that made it one; the voltage, where there is one, is read from the net name.
//!
//! - A rail is a net named with a voltage (`+3V3`, `VDD_+1V8`: a `+` before `<volts>V<decimals>`; without the `+`
//!   too in non-strict mode, [`PowerOptions::strict`]), a net named `+V...`, `VDD...` or `VCC...`, or a net with a
//!   power pin (`power_in`, `power_out`) of a component on it.
//! - A ground is a net named `GND`, `VSS`, `VEE` anywhere in the name, or starting with `ISO`; it is never a rail.
//! - A switching node is the net between an IC and an inductor that is neither a ground nor a rail (a rail whose
//!   name has `LX` or `SW` still counts: its pins are often typed power), unless the node or the inductor's other
//!   side is an `RF` net. It is not a rail.
//! - The SCL and SDA nets of an I2C bus are not rails even when named with a voltage (`I2C1_SCL_3V3`).
//!
//! Sources, power trees and states come with the full NET-5 and NET-8; these rules stay as the fallback.

use crate::class::{PartClass, PartClasses};
use crate::graph::{net_pins, part_chains, part_nets};
use crate::i2c::I2cBuses;
use erc_model::{Board, EcadPinType, Fact, NetId, PartId, Provenance};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const BY_VOLTAGE_NAME: &str = "rail named with a voltage";
const BY_NAME: &str = "rail by name (+V, VDD, VCC)";
const BY_POWER_PIN: &str = "rail by a power pin on it";

/// The power structure of a board
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Power {
    /// Each rail with its voltage in volts when the name gives one
    pub rails: BTreeMap<NetId, Fact<Option<f64>>>,
    pub grounds: BTreeSet<NetId>,
    pub switching_nodes: BTreeSet<NetId>,
}

/// How rails are read from names
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerOptions {
    /// A voltage in a name only with a `+` before it (`+3V3`, not `VDD_1V8`); on by default
    pub strict: bool,
}

impl Default for PowerOptions {
    fn default() -> PowerOptions {
        PowerOptions { strict: true }
    }
}

impl Power {
    /// Finds the rails, grounds and switching nodes of `board`, in strict mode
    pub fn of(board: &Board, classes: &PartClasses, i2c: &I2cBuses) -> Power {
        Power::with(board, classes, i2c, PowerOptions::default())
    }

    /// [`Power::of`] with options
    pub fn with(board: &Board, classes: &PartClasses, i2c: &I2cBuses, opts: PowerOptions) -> Power {
        let derived = |v, rule: &str| Fact::new(v, Provenance::Derived { rule: rule.into() });
        let mut power = Power::default();
        for (i, net) in board.nets.iter().enumerate() {
            let id = NetId::from_index(i);
            let n = net.name.as_str();
            if n.contains("GND") || n.contains("VSS") || n.contains("VEE") || n.starts_with("ISO") {
                power.grounds.insert(id);
            }
            if let Some(v) = name_voltage(n, opts.strict) {
                power.rails.insert(id, derived(Some(v), BY_VOLTAGE_NAME));
            } else if n.contains("+V") || n.contains("VDD") || n.contains("VCC") {
                power.rails.insert(id, derived(None, BY_NAME));
            }
        }
        for i in 0..board.nets.len() {
            let id = NetId::from_index(i);
            let powered = net_pins(board, id).any(|p| {
                matches!(
                    board.pin(p).etype.get(),
                    Some(EcadPinType::PowerIn | EcadPinType::PowerOut)
                )
            });
            if powered && !power.grounds.contains(&id) {
                power
                    .rails
                    .entry(id)
                    .or_insert_with(|| derived(None, BY_POWER_PIN));
            }
        }
        power.switching_nodes = power.find_switching_nodes(board, classes);
        for net in &power.switching_nodes {
            power.rails.remove(net);
        }
        for bus in &i2c.buses {
            power.rails.remove(&bus.scl);
            power.rails.remove(&bus.sda);
        }
        power
    }

    /// Whether a net is a rail or a ground
    pub fn is_power_net(&self, net: NetId) -> bool {
        self.rails.contains_key(&net) || self.grounds.contains(&net)
    }

    fn find_switching_nodes(&self, board: &Board, classes: &PartClasses) -> BTreeSet<NetId> {
        let ic = |p: PartId| classes.get(p).is_ic();
        let inductor = |p: PartId| classes.is(p, PartClass::Inductor);
        let mut nodes = BTreeSet::new();
        for chain in part_chains(board, &[&ic, &inductor], &|_| false) {
            let [ic, l] = chain[..] else { continue };
            let l_nets = part_nets(board, l);
            let switching = part_nets(board, ic).into_iter().find(|n| {
                let name = &board.net(*n).name;
                l_nets.contains(n)
                    && !self.grounds.contains(n)
                    && (!self.rails.contains_key(n) || name.contains("LX") || name.contains("SW"))
            });
            let Some(switching) = switching else {
                continue;
            };
            let Some(other) = l_nets.iter().find(|n| **n != switching) else {
                continue;
            };
            let rf = |n: NetId| board.net(n).name.contains("RF");
            if !rf(switching) && !rf(*other) {
                nodes.insert(switching);
            }
        }
        nodes
    }
}

/// The voltage in a rail name: `+3V3` is 3.3, `+1V25` 1.25, `+12V0` 12. Strict: only with the `+` in front;
/// otherwise any `<volts>V<decimals>` (`VDD_1V8`).
pub fn name_voltage(name: &str, strict: bool) -> Option<f64> {
    let bytes = name.as_bytes();
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    for start in 0..bytes.len() {
        let after_plus = start > 0 && bytes[start - 1] == b'+';
        let run_start = start == 0 || !bytes[start - 1].is_ascii_digit();
        if !run_start || (strict && !after_plus) {
            continue;
        }
        let whole = digits(start);
        let v = start + whole;
        if whole == 0 || bytes.get(v) != Some(&b'V') {
            continue;
        }
        let frac = digits(v + 1);
        if frac == 0 {
            continue;
        }
        return format!("{}.{}", &name[start..v], &name[v + 1..v + 1 + frac])
            .parse()
            .ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voltages_in_names() {
        let strict = |n| name_voltage(n, true);
        assert_eq!(strict("+3V3"), Some(3.3));
        assert_eq!(strict("/VDD_+1V25"), Some(1.25));
        assert_eq!(strict("+12V0_IN"), Some(12.0));
        assert_eq!(strict("3V3"), None, "only with the +");
        assert_eq!(strict("+5V"), None, "no decimals: a +V rail by name");
        assert_eq!(strict("+V+3V3"), Some(3.3));
        // The catalogue's NAME example: non-strict reads VDD_1V8 only
        let loose = |n| name_voltage(n, false);
        assert_eq!(loose("VDD_1V8"), Some(1.8));
        assert_eq!(loose("VDD_12V5"), Some(12.5));
        assert_eq!(loose("+3.3V"), None);
        assert_eq!(loose("+5V"), None);
    }
}
