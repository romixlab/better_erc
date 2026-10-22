//! `LVL` - logic levels between voltage domains: the rails on each part's supply pins, and the discrete
//! translators (the BSS138 pair) of [`erc_analysis::i2c`] with the rails of their pull-ups, gates and devices

mod lvl_4;
mod lvl_7;
mod lvl_9;

use crate::Ctx;
use crate::rail::ground_named;
use erc_analysis::feed::power_in_on;
use erc_analysis::i2c::{BusIdx, Node};
use erc_model::{NetId, PartId};
use std::collections::BTreeSet;

/// A discrete translator (a FET per line) with its two sides: the source side (low) and the drain side (high)
pub(crate) struct Shifter {
    pub fets: [PartId; 2],
    pub gates: [Option<NetId>; 2],
    /// The bus on the FETs' sources (the low side)
    pub low: BusIdx,
    /// The bus on the FETs' drains (the high side)
    pub high: BusIdx,
}

/// Every discrete translator of the board once, its sides told by the FET's source pin
pub(crate) fn shifters(cx: &Ctx) -> Vec<Shifter> {
    let b = cx.board();
    let i2c = cx.i2c();
    let mut seen: BTreeSet<PartId> = BTreeSet::new();
    let mut out = Vec::new();
    for (bi, bus) in i2c.buses.iter().enumerate() {
        for node in &bus.nodes {
            let Node::DiscreteTranslator {
                scl_fet,
                sda_fet,
                other_side,
                scl_gate,
                sda_gate,
            } = *node
            else {
                continue;
            };
            if !seen.insert(scl_fet) {
                continue;
            }
            let source = b
                .part(scl_fet)
                .pins
                .iter()
                .map(|p| b.pin(*p))
                .find(|p| matches!(p.name.to_uppercase().as_str(), "S" | "SOURCE"))
                .and_then(|p| p.net);
            let Some(source) = source else { continue };
            let (low, high) = if source == bus.scl || source == bus.sda {
                (bi, other_side)
            } else {
                (other_side, bi)
            };
            out.push(Shifter {
                fets: [scl_fet, sda_fet],
                gates: [scl_gate, sda_gate],
                low,
                high,
            });
        }
    }
    out
}

/// The rails the devices of a bus's direct segment are supplied from (their power-input pins, ground-named ones
/// left out)
pub(crate) fn device_rails(cx: &Ctx, bus: BusIdx) -> BTreeSet<NetId> {
    let b = cx.board();
    let i2c = cx.i2c();
    let power = cx.power();
    let segment = i2c
        .direct_segment(bus)
        .cloned()
        .unwrap_or_else(|| BTreeSet::from([bus]));
    let mut rails = BTreeSet::new();
    for bi in segment {
        for node in &i2c.buses[bi].nodes {
            let Node::Device(part) = *node else { continue };
            for pin in &b.part(part).pins {
                let Some(net) = b.pin(*pin).net else { continue };
                if power.is_rail(net)
                    && !ground_named(cx, *pin)
                    && power_in_on(cx.view(), net).contains(pin)
                {
                    rails.insert(net);
                }
            }
        }
    }
    rails
}

/// `+3V3 (3.3 V)`, or the name alone when the rail has no voltage
pub(crate) fn rail_text(cx: &Ctx, net: NetId) -> String {
    let name = &cx.board().net(net).name;
    match cx.power().volts(net) {
        Some(v) => format!(
            "{name} ({})",
            erc_analysis::value::format_si(v, erc_model::Unit::Volt)
        ),
        None => name.clone(),
    }
}
