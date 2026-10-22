//! `TEST-5` Bring-up bus (UART, I2C, SPI, SWD) without test points

use crate::dbg::{buses, named_pins, programmable};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::access::reach;
use erc_analysis::pin_name::PinFunction;
use erc_model::NetId;
use std::collections::BTreeMap;

pub struct Test5;

static META: CheckMeta = CheckMeta {
    id: "TEST-5",
    category: "TEST",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Test5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every line of a bring-up bus with no test point or connector pin on it or one series part away: the
    /// UART TX/RX and SWDIO/SWCLK pins of MCUs, SoCs and FPGAs by name, the I2C buses (NET-4) and the SPI buses
    /// by pin names
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut lines: BTreeMap<NetId, String> = BTreeMap::new();
        for part in programmable(cx) {
            for (pin, f) in named_pins(cx, part) {
                let kind = match f.function {
                    PinFunction::UartTx | PinFunction::UartRx => "UART",
                    PinFunction::Swdio | PinFunction::Swclk => "SWD",
                    _ => continue,
                };
                if let Some(net) = b.pin(pin).net
                    && !cx.power().is_power_net(net)
                {
                    lines.entry(net).or_insert_with(|| kind.into());
                }
            }
        }
        for (kind, name, nets) in buses(cx) {
            for net in nets {
                lines
                    .entry(net)
                    .or_insert_with(|| format!("{kind} bus {name}"));
            }
        }
        let mut out = Vec::new();
        for (net, what) in lines {
            if !reach(cx.view(), cx.classes(), cx.power(), net).is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(net)
                    .message(format!(
                        "{} ({what}) has no test point or header pin",
                        b.net(net).name
                    ))
                    .fix("a test point per line plus a nearby ground; on SWD also NRST")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Test5));

#[cfg(test)]
mod tests {
    #[test]
    fn bring_up_lines_without_test_points() {
        let found = crate::testing::fingerprints(&super::Test5, "test-5_bring_up_test_points");
        assert_eq!(
            found,
            [
                "TEST-5:net /SDA",
                "TEST-5:net /SWCLK",
                "TEST-5:net /UART_RX"
            ],
            "{found:?}"
        );
    }
}
