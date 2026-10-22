//! `NAME-4` UART: suggest `X_TX_Y_RX` style names

use super::named_nets;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::{Uart, uart_pin, uart_words};
use erc_model::PinId;

pub struct Name4;

static META: CheckMeta = CheckMeta {
    id: "NAME-4",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Name4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Nets on UART pins (by pin name: `TX`, `RXD`, `USART1_TX`, `U0TXD`) whose name says `TX` or `RX` once,
    /// not by both ends; and a net named by both ends (`MCU_TX_GPS_RX`) that does not join one TX and one RX pin.
    /// Pins named by their port only (`PA9`) wait for pin functions (NET-12).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for net in named_nets(cx) {
            let words = uart_words(&b.net(net).name);
            if words.is_empty() {
                continue;
            }
            let pins: Vec<(PinId, Uart)> = net_pins(b, net)
                .filter_map(|q| Some((q, uart_pin(&b.pin(q).name)?)))
                .collect();
            if pins.is_empty() {
                continue;
            }
            let count = |u: Uart| pins.iter().filter(|(_, x)| *x == u).count();
            let shown: Vec<String> = pins
                .iter()
                .map(|(q, _)| format!("{} ({})", b.pin_name(*q), b.pin(*q).name))
                .collect();
            let both = words.contains(&Uart::Tx) && words.contains(&Uart::Rx);
            let message = if !both {
                format!(
                    "{} joins {} and names one end only: whose {} is it?",
                    b.net(net).name,
                    shown.join(", "),
                    if words[0] == Uart::Tx { "TX" } else { "RX" }
                )
            } else if count(Uart::Tx) > 1 || count(Uart::Rx) > 1 {
                format!(
                    "{} is named by a driver and a receiver but joins {}: two {}s on one line",
                    b.net(net).name,
                    shown.join(", "),
                    if count(Uart::Tx) > 1 { "TX" } else { "RX" }
                )
            } else {
                continue;
            };
            let mut f = cx.finding(&META).net(net);
            for (q, _) in &pins {
                f = f.pin(*q);
            }
            out.push(
                f.message(message)
                    .fix("name UART nets by driver and receiver (`MCU_TX_GPS_RX`, `GPS_TX_MCU_RX`): a crossed wire reads wrong")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Name4));

#[cfg(test)]
mod tests {
    #[test]
    fn uart_names() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name4, "name-4_uart_names"),
            [
                "NAME-4:net /DBG_TX_MCU_RX|pin U1.6|pin U4.2",
                "NAME-4:net /GPS_RX|pin U1.2|pin U2.1",
                "NAME-4:net /GPS_TX|pin U1.1|pin U2.2",
                "NAME-4:net /MCU_TX_DBG_RX|pin U1.5|pin U4.1",
            ]
        );
    }
}
