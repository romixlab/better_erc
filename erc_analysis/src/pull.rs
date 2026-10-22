//! Pull resistors: the fitted resistors from a net to a rail (pull-ups) and to a ground (pull-downs). What the IO
//! and CLK checks ask of every input, button, gate and reset net.

use crate::class::PartClasses;
use crate::graph::net_pins;
use crate::power::Power;
use crate::value::Values;
use erc_model::{NetId, PartId, View};

/// A fitted two-pin resistor from a net to a rail or a ground, with the rail or ground at its far end
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pull {
    pub part: PartId,
    pub to: NetId,
}

/// The pulls on one net
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pulls {
    /// To a rail
    pub up: Vec<Pull>,
    /// To a ground
    pub down: Vec<Pull>,
}

impl Pulls {
    pub fn is_empty(&self) -> bool {
        self.up.is_empty() && self.down.is_empty()
    }

    pub fn parts(&self) -> impl Iterator<Item = PartId> + '_ {
        self.up.iter().chain(&self.down).map(|p| p.part)
    }
}

/// The fitted two-pin resistors from `net` to a rail or a ground. A 0 Ω resistor ties the net to the rail, it does
/// not pull it, and is left out.
pub fn pulls(
    view: View,
    classes: &PartClasses,
    values: &Values,
    power: &Power,
    net: NetId,
) -> Pulls {
    let b = view.board;
    let mut out = Pulls::default();
    for pin in net_pins(b, net) {
        let r = b.pin(pin).part;
        if !view.fitted(r)
            || !classes.get(r).is_resistor()
            || b.part(r).pins.len() != 2
            || values.get(r).value == Some(0.0)
        {
            continue;
        }
        let far = b
            .part(r)
            .pins
            .iter()
            .filter(|p| **p != pin)
            .find_map(|p| b.pin(*p).net);
        match far {
            Some(to) if to != net && power.is_rail(to) => out.up.push(Pull { part: r, to }),
            Some(to) if to != net && power.is_ground(to) => out.down.push(Pull { part: r, to }),
            _ => {}
        }
    }
    out.up.sort();
    out.up.dedup();
    out.down.sort();
    out.down.dedup();
    out
}
