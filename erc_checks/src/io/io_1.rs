//! `IO-1` Input pin without a drive source, smart about resistors, FETs etc.

use super::undriven_inputs;
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};

pub struct Io1;

static META: CheckMeta = CheckMeta {
    id: "IO-1",
    category: "IO",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Io1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A net with input pins and nothing that can set its level ([`super::driven_nets`]): no output,
    /// bidirectional or open-drain pin, no rail, no resistor, FET or switch to a driven net; a connector or test
    /// point is no source (IO-10, IO-6 say which)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        undriven_inputs(cx)
            .into_iter()
            .map(|(net, pins)| {
                pins.iter()
                    .fold(cx.finding(&META).net(net), |f, p| f.pin(*p))
                    .message(format!(
                        "{} on {}: nothing drives the net, the input floats",
                        pin_list(cx, &pins),
                        b.net(net).name
                    ))
                    .fix("a driver, or a pull to the safe level")
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&Io1));

#[cfg(test)]
mod tests {
    #[test]
    fn floating_inputs() {
        let found = crate::testing::fingerprints(&super::Io1, "io-1_drive_sources");
        assert_eq!(
            found,
            [
                "IO-1:net /IN_CAP_ONLY|pin U2.3",
                "IO-1:net /IN_DEAD|pin U3.3"
            ],
            "{found:?}"
        );
    }
}
