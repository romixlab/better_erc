//! `DBG-14` No UART console reachable on a header or test points

use super::{named_pins, pin_reach, programmable};
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::pin_name::PinFunction;
use erc_model::{PartId, PinId};
use std::collections::BTreeMap;

pub struct Dbg14;

static META: CheckMeta = CheckMeta {
    id: "DBG-14",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg14 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The UART TX/RX pairs (by pin name and peripheral instance) of the MCUs, SoCs and FPGAs: none whose two
    /// lines reach a connector or test points. A board whose pin names state no UART is not judged (NET-12).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut with_uart: Vec<PartId> = Vec::new();
        for part in programmable(cx) {
            // (instance) -> (TX pins, RX pins)
            let mut uarts: BTreeMap<String, (Vec<PinId>, Vec<PinId>)> = BTreeMap::new();
            for (pin, f) in named_pins(cx, part) {
                match f.function {
                    PinFunction::UartTx => uarts.entry(f.instance).or_default().0.push(pin),
                    PinFunction::UartRx => uarts.entry(f.instance).or_default().1.push(pin),
                    _ => {}
                }
            }
            uarts.retain(|_, (tx, rx)| !tx.is_empty() && !rx.is_empty());
            if uarts.is_empty() {
                continue;
            }
            let out = |pins: &[PinId]| pins.iter().any(|p| !pin_reach(cx, *p).is_empty());
            if uarts.values().any(|(tx, rx)| out(tx) && out(rx)) {
                return Vec::new();
            }
            with_uart.push(part);
        }
        if with_uart.is_empty() {
            return Vec::new();
        }
        vec![
            cx.finding(&META)
                .parts(with_uart.iter().copied())
                .message(format!(
                    "no UART of {} reaches a header or test points: no console for boot logs and a shell",
                    designators(cx, with_uart.iter().copied())
                ))
                .fix("three pads or a 3-pin header (TX, RX, GND) at the IO level, ideally a USB-UART cable's pinout")
                .build(),
        ]
    }
}

inventory::submit!(Registered(&Dbg14));

#[cfg(test)]
mod tests {
    #[test]
    fn no_console() {
        let found = crate::testing::fingerprints(&super::Dbg14, "dbg-14_uart_console");
        assert_eq!(found, ["DBG-14:part U1"], "{found:?}");
    }
}
