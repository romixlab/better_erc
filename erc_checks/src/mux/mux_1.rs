//! `MUX-1` SCL goes to a pin named SCL, MISO to MISO etc

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::{Interface, Signal, interface_signals, name_signals};
use erc_model::{Board, NetId, NetKind, PinId};

pub struct Mux1;

static META: CheckMeta = CheckMeta {
    id: "MUX-1",
    category: "MUX",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

/// Every name a pin answers to: its own, the alternate chosen, the symbol's alternates
pub(crate) fn pin_names(b: &Board, pin: PinId) -> Vec<&str> {
    let p = b.pin(pin);
    let mut names = vec![p.name.as_str()];
    names.extend(p.alt.as_deref());
    if let Some(sym) = b.part(p.part).symbol.map(|s| &b.symbols[s.index()])
        && let Some(def) = sym.pins.iter().find(|d| d.number == p.number)
    {
        names.extend(def.alternates.iter().map(|a| a.name.as_str()));
    }
    names
}

fn shown(b: &Board, pin: PinId) -> String {
    let p = b.pin(pin);
    format!("{} {}", b.pin_name(pin), p.name)
}

impl Check for Mux1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Named nets that carry one I2C, SPI, USB or CAN signal by their name: a pin on them whose names (all its
    /// functions and alternates) carry signals of the same interface, none of them the net's, is a finding (`SDA` on
    /// `I2C1_SCL`, `MISO` on `SPI_MOSI`; `SDI`/`SDO` fit either SPI data net). UART is judged pin against pin: two
    /// parts' TX pins on one net, or RX pins with no TX and no connector to bring one
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let mut out = Vec::new();
        for n in (0..b.nets.len()).map(NetId::from_index) {
            let net = b.net(n);
            let pins: Vec<PinId> = net_pins(b, n).collect();
            if matches!(net.kind, NetKind::Named) {
                let mut by_interface: Vec<(Interface, Signal)> = Vec::new();
                for s in name_signals(&net.name) {
                    if s.interface() != Interface::Uart {
                        by_interface.push((s.interface(), s));
                    }
                }
                let mut wrong = Vec::new();
                for (interface, signal) in &by_interface {
                    // A name with two signals of one interface (`SCL_SDA`) says nothing about either
                    if by_interface.iter().filter(|(i, _)| i == interface).count() > 1 {
                        continue;
                    }
                    for pin in &pins {
                        let theirs: Vec<Signal> = interface_signals(pin_names(b, *pin))
                            .into_iter()
                            .filter(|s| s.interface() == *interface)
                            .collect();
                        if !theirs.is_empty() && !theirs.iter().any(|s| signal.fits(*s)) {
                            let names: Vec<&str> = theirs.iter().map(|s| s.name()).collect();
                            wrong.push((*pin, format!("{} ({})", shown(b, *pin), names.join("/"))));
                        }
                    }
                }
                if !wrong.is_empty() {
                    let signals: Vec<&str> = by_interface.iter().map(|(_, s)| s.name()).collect();
                    let what: Vec<&str> = wrong.iter().map(|(_, w)| w.as_str()).collect();
                    out.push(
                        cx.finding(&META)
                            .net(n)
                            .parts(wrong.iter().map(|(p, _)| b.pin(*p).part))
                            .message(format!(
                                "net {} ({}) lands on {}",
                                net.name,
                                signals.join(", "),
                                what.join(", ")
                            ))
                            .fix("swap the connection, or rename the net if the name is the error")
                            .build(),
                    );
                }
            }
            // UART, pin against pin: connectors and test points are left out (their names may be from either side)
            let uart: Vec<(PinId, Signal)> = pins
                .iter()
                .filter(|p| {
                    let c = classes.get(b.pin(**p).part);
                    !c.is_connector() && c != erc_model::PartClass::TestPoint
                })
                .filter_map(|p| {
                    let s: Vec<Signal> = interface_signals(pin_names(b, *p))
                        .into_iter()
                        .filter(|s| s.interface() == Interface::Uart)
                        .collect();
                    (s.len() == 1).then(|| (*p, s[0]))
                })
                .collect();
            let of = |sig: Signal| -> Vec<PinId> {
                uart.iter()
                    .filter(|(_, s)| *s == sig)
                    .map(|(p, _)| *p)
                    .collect()
            };
            let (tx, rx) = (of(Signal::Tx), of(Signal::Rx));
            let parts = |ps: &[PinId]| {
                let mut v: Vec<_> = ps.iter().map(|p| b.pin(*p).part).collect();
                v.sort();
                v.dedup();
                v.len()
            };
            let connector = pins
                .iter()
                .any(|p| classes.get(b.pin(*p).part).is_connector());
            let problem = if parts(&tx) > 1 {
                Some((tx.clone(), "TX pins of two parts meet: both drive the line"))
            } else if tx.is_empty() && parts(&rx) > 1 && !connector {
                Some((
                    rx.clone(),
                    "RX pins meet with no TX: nothing drives the line",
                ))
            } else {
                None
            };
            if let Some((ps, why)) = problem {
                let what: Vec<String> = ps.iter().map(|p| shown(b, *p)).collect();
                out.push(
                    cx.finding(&META)
                        .net(n)
                        .parts(ps.iter().map(|p| b.pin(*p).part))
                        .message(format!("net {}: {why} ({})", net.name, what.join(", ")))
                        .fix("cross the UART: TX of one part to RX of the other")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Mux1));

#[cfg(test)]
mod tests {
    #[test]
    fn swapped_signals() {
        // U2 is wired with SCL and SDA swapped, U3 with MOSI and MISO, U4 with TX and RX; U5, U6 (SDI, SDO) and U7
        // are wired right, and J1's TX pin (a connector) on U1's TX is not judged
        assert_eq!(
            crate::testing::fingerprints(&super::Mux1, "mux-1_swapped_signals"),
            [
                "MUX-1:net /I2C1_SCL|part U2",
                "MUX-1:net /I2C1_SDA|part U2",
                "MUX-1:net /SPI_MISO|part U3",
                "MUX-1:net /SPI_MOSI|part U3",
                "MUX-1:net /UART_RX|part U1|part U4",
                "MUX-1:net /UART_TX|part U1|part U4"
            ]
        );
    }
}
