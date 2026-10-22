//! `WIRE-1` Net with a single pin

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::normalised;
use erc_model::{NetId, NetKind, Provenance};

pub struct Wire1;

static META: CheckMeta = CheckMeta {
    id: "WIRE-1",
    category: "WIRE",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Wire1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Nets a label names that reach one component pin (not one under a no-connect flag); auto-named nets of
    /// unconnected pins are one-pin nets on purpose. A net whose name differs from another's only in case,
    /// separators or markers (WIRE-2) is named in the finding.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            let n = b.net(net);
            if n.kind != NetKind::Named {
                continue;
            }
            let pins: Vec<_> = net_pins(b, net).collect();
            let [pin] = pins[..] else { continue };
            if b.pin(pin).no_connect {
                continue;
            }
            let key = normalised(&n.name);
            let similar = b.nets.iter().enumerate().find(|(j, m)| {
                *j != i
                    && matches!(m.kind, NetKind::Named | NetKind::Power)
                    && normalised(&m.name) == key
            });
            let mut f = cx
                .finding(&META)
                .net(net)
                .pin(pin)
                .message(format!(
                    "{} reaches one pin only ({}): a label without a partner, or a misspelled one",
                    n.name,
                    b.pin_name(pin)
                ))
                .fix("rename the label to match its partner, or delete it and mark the pin no-connect (WIRE-8)");
            if let Some((_, m)) = similar {
                f = f.fact(
                    "similar net (WIRE-2)",
                    &m.name,
                    Provenance::Derived {
                        rule: "the names equal once case, separators and markers are normalised"
                            .into(),
                    },
                );
            }
            out.push(f.build());
        }
        out
    }
}

inventory::submit!(Registered(&Wire1));

#[cfg(test)]
mod tests {
    #[test]
    fn one_pin_nets() {
        assert_eq!(
            crate::testing::fingerprints(&super::Wire1, "wire-1_single_pin_nets"),
            [
                "WIRE-1:net /UART2_TX|pin U1.1",
                "WIRE-1:net /UART_2_TX|pin J1.1",
            ]
        );
    }
}
