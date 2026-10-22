//! `DBG-11` Suggest 0R on USART TX, RX, CTS, RTS and SPI MOSI, MISO, as they're often wrong

use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::{PinFunction::*, functions};
use erc_model::{NetId, PartClass, PinId};
use std::collections::BTreeMap;

pub struct Dbg11;

static META: CheckMeta = CheckMeta {
    id: "DBG-11",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net with a UART (TX, RX, RTS, CTS) or SPI data (MOSI, MISO) pin by name and a pin of another part
    /// that is not a passive or a test point: the two meet with no series resistor footprint between them
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut nets: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
        for part in cx.view().fitted_components() {
            for pin in &b.part(part).pins {
                let p = b.pin(*pin);
                let data = functions(&p.name).iter().any(|f| {
                    matches!(
                        f.function,
                        UartTx | UartRx | UartRts | UartCts | SpiMosi | SpiMiso
                    )
                });
                if let (true, Some(net)) = (data, p.net) {
                    nets.entry(net).or_default().push(*pin);
                }
            }
        }
        let mut out = Vec::new();
        for (net, data) in nets {
            let own: Vec<_> = data.iter().map(|p| b.pin(*p).part).collect();
            let others: Vec<PinId> = net_pins(b, net)
                .filter(|p| !data.contains(p))
                .filter(|p| {
                    let part = b.pin(*p).part;
                    let c = cx.classes().get(part);
                    cx.fitted(part)
                        && !(c.is_resistor()
                            || c.is_capacitor()
                            || c.is_diode()
                            || c.is_dc_tie()
                            || matches!(c, PartClass::TestPoint | PartClass::Inductor))
                })
                .collect();
            // Two data pins of different parts on one net meet directly too
            let parts_on_net = {
                let mut p = own.clone();
                p.sort();
                p.dedup();
                p.len()
            };
            if others.is_empty() && parts_on_net < 2 {
                continue;
            }
            let f = data
                .iter()
                .chain(&others)
                .fold(cx.finding(&META).net(net), |f, p| f.pin(*p));
            let mut all = data.clone();
            all.extend(&others);
            out.push(
                f.message(format!(
                    "{}: {} meet with no series resistor footprint",
                    b.net(net).name,
                    pin_list(cx, &all)
                ))
                .fix("a 0 Ω in each line near the MCU, or a 2×2 footprint that allows straight and crossed fitting")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dbg11));

#[cfg(test)]
mod tests {
    #[test]
    fn data_lines_without_resistors() {
        let found = crate::testing::fingerprints(&super::Dbg11, "dbg-11_uart_spi_resistors");
        assert_eq!(
            found,
            [
                "DBG-11:net /GPS_TX|pin U1.3|pin U2.2",
                "DBG-11:net /MISO|pin U1.6|pin U3.2"
            ],
            "{found:?}"
        );
    }
}
