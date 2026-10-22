//! `DBG-18` I2C or SPI buses not reachable at a header or test points for a bus analyser

use super::named_pins;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::access::reach;
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::PinFunction;
use erc_model::NetId;
use std::collections::BTreeSet;

pub struct Dbg18;

static META: CheckMeta = CheckMeta {
    id: "DBG-18",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg18 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each I2C bus (NET-4) and SPI bus (by pin names: the clock net and the MOSI, MISO and CS nets of the
    /// parts on it) with lines that reach no test point or connector, directly or through a series part
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (kind, name, lines) in buses(cx) {
            let missing: Vec<NetId> = lines
                .iter()
                .copied()
                .filter(|n| reach(cx.view(), cx.classes(), cx.power(), *n).is_empty())
                .collect();
            if missing.is_empty() {
                continue;
            }
            let names: Vec<&str> = missing.iter().map(|n| b.net(*n).name.as_str()).collect();
            out.push(
                cx.finding(&META)
                    .nets(missing.iter().copied())
                    .message(format!(
                        "{kind} bus {name}: {} reach{} no test point or header for an analyser",
                        crate::text::and_list(&names),
                        if missing.len() == 1 { "es" } else { "" }
                    ))
                    .fix("a test point per line plus GND, or an unpopulated 2.54 mm header")
                    .build(),
            );
        }
        out
    }
}

/// The I2C and SPI buses: (kind, name, line nets)
pub(crate) fn buses(cx: &Ctx) -> Vec<(&'static str, String, Vec<NetId>)> {
    let b = cx.board();
    let mut out: Vec<(&'static str, String, Vec<NetId>)> = cx
        .i2c()
        .buses
        .iter()
        .map(|bus| ("I2C", bus.name.clone(), vec![bus.scl, bus.sda]))
        .collect();
    // SPI: every net with an SCK pin is a clock; the bus is it and the data and select nets of the parts on it
    let mut clocks: BTreeSet<NetId> = BTreeSet::new();
    for part in cx.view().fitted_components() {
        for (pin, f) in named_pins(cx, part) {
            if f.function == PinFunction::SpiSck
                && let Some(net) = b.pin(pin).net
                && !cx.power().is_power_net(net)
            {
                clocks.insert(net);
            }
        }
    }
    for clock in clocks {
        let mut lines = vec![clock];
        let parts: BTreeSet<_> = net_pins(b, clock).map(|p| b.pin(p).part).collect();
        for part in parts {
            for (pin, f) in named_pins(cx, part) {
                if matches!(
                    f.function,
                    PinFunction::SpiMosi | PinFunction::SpiMiso | PinFunction::SpiCs
                ) && let Some(net) = b.pin(pin).net
                    && !lines.contains(&net)
                    && !cx.power().is_power_net(net)
                {
                    lines.push(net);
                }
            }
        }
        if lines.len() > 1 {
            out.push(("SPI", b.net(clock).name.clone(), lines));
        }
    }
    out
}

inventory::submit!(Registered(&Dbg18));

#[cfg(test)]
mod tests {
    #[test]
    fn buses_without_access() {
        let found = crate::testing::fingerprints(&super::Dbg18, "dbg-18_bus_access");
        assert_eq!(
            found,
            [
                "DBG-18:net /IMU_CS|net /IMU_MISO|net /IMU_MOSI|net /IMU_SCK",
                "DBG-18:net /SDA"
            ],
            "{found:?}"
        );
    }
}
