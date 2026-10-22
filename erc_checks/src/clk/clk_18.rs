//! `CLK-18` Clock output fanned out to several loads without a buffer or series termination

use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::{PinFunction, functions, states};
use erc_model::{EcadPinType, NetId, PartClass, PinId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Clk18;

static META: CheckMeta = CheckMeta {
    id: "CLK-18",
    category: "CLK",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// Whether a net name is a clock's by a `CLK` word (`MCLK`, `REF_CLK`, `CLK_25M`)
fn clock_name(name: &str) -> bool {
    erc_analysis::names::base(name)
        .to_uppercase()
        .split(['_', '-', '.', ' '])
        .any(|w| w.contains("CLK"))
}

impl Check for Clk18 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A clock driver directly on a net with two or more loads (pins of other fitted ICs, MCUs, connectors): an
    /// oscillator's output, a pin named as a clock output (`MCO`, `CLKOUT`), or an IC's push-pull output on a net
    /// named with `CLK`; buses' clocks (SPI, I2C, SWD, JTAG pins by name on the net) are left out. A series
    /// resistor at the driver puts the loads on another net, so it does not fire.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut drivers: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            for pin in &b.part(part).pins {
                let p = b.pin(*pin);
                let Some(net) = p.net else { continue };
                if power.is_power_net(net) {
                    continue;
                }
                let output = matches!(
                    p.etype.get(),
                    Some(EcadPinType::Output | EcadPinType::TriState)
                );
                let driver = (class == PartClass::Oscillator && output)
                    || states(&p.name, PinFunction::ClockOut)
                    || (output && class.is_ic() && clock_name(&b.net(net).name));
                if driver {
                    drivers.entry(net).or_default().push(*pin);
                }
            }
        }
        let mut out = Vec::new();
        for (net, pins) in drivers {
            let bus = net_pins(b, net).any(|p| {
                functions(&b.pin(p).name).iter().any(|f| {
                    f.function.is_spi()
                        || f.function.is_debug()
                        || matches!(f.function, PinFunction::I2cScl | PinFunction::I2cSda)
                })
            });
            if bus {
                continue;
            }
            let driver_parts: BTreeSet<_> = pins.iter().map(|p| b.pin(*p).part).collect();
            let loads: Vec<PinId> = net_pins(b, net)
                .filter(|p| {
                    let pin = b.pin(*p);
                    let class = cx.classes().get(pin.part);
                    cx.fitted(pin.part)
                        && !driver_parts.contains(&pin.part)
                        && (class.is_ic() || class.is_programmable() || class.is_connector())
                        && !matches!(
                            pin.etype.get(),
                            Some(EcadPinType::PowerIn | EcadPinType::PowerOut)
                        )
                })
                .collect();
            if loads.len() < 2 {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} drives the clock {} straight into {} loads ({}): a branched route rings and each load sees the others' reflections",
                        pin_list(cx, &pins),
                        b.net(net).name,
                        loads.len(),
                        pin_list(cx, &loads)
                    ))
                    .fix("a clock buffer (CDCLVC11xx class), or one series resistor per branch at the driver; daisy-chain only for low frequencies")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Clk18));

#[cfg(test)]
mod tests {
    #[test]
    fn clock_fan_out() {
        let found = crate::testing::fingerprints(&super::Clk18, "clk-18_clock_fan_out");
        assert_eq!(
            found,
            ["CLK-18:net /CLK25|pin X1.3", "CLK-18:net /MCO_OUT|pin U1.3"],
            "{found:?}"
        );
    }
}
