//! `IO-10` Input driven only from a connector: floats when nothing is plugged in

use super::undriven_inputs;
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::access::reach;

pub struct Io10;

static META: CheckMeta = CheckMeta {
    id: "IO-10",
    category: "IO",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &["IO-6"],
    params: &[],
};

impl Check for Io10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An IO-1 net (inputs nothing on the board drives) with a connector pin on it or one series part away: its
    /// only driver is off the board
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (net, pins) in undriven_inputs(cx) {
            let connectors: Vec<_> = reach(cx.view(), cx.classes(), cx.power(), net)
                .into_iter()
                .map(|r| r.pin)
                .filter(|p| cx.classes().get(b.pin(*p).part).is_connector())
                .collect();
            if connectors.is_empty() {
                continue;
            }
            out.push(
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {}: only {} can drive it, the input floats while nothing is plugged in",
                        pin_list(cx, &pins),
                        b.net(net).name,
                        pin_list(cx, &connectors)
                    ))
                    .fix("a pull to the inactive level on this board; ESD protection at the connector")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Io10));

#[cfg(test)]
mod tests {
    #[test]
    fn connector_only_inputs() {
        let found = crate::testing::fingerprints(&super::Io10, "io-10_connector_inputs");
        assert_eq!(
            found,
            [
                "IO-10:net /EXT_IN|pin U1.5",
                "IO-10:net /REMOTE_EN|pin U1.3"
            ],
            "{found:?}"
        );
    }
}
