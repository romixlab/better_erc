//! `IO-7` Button to ground with a pull-down, or button to power with a pull-up

use super::{button_sides, net_pulls, same_level, switches};
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};

pub struct Io7;

static META: CheckMeta = CheckMeta {
    id: "IO-7",
    category: "IO",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &["IO-8"],
    params: &[],
};

impl Check for Io7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A switch between a signal net and one rail or ground, where a fitted resistor pulls the signal to that
    /// same level (the same ground, the same rail or one of its voltage): pressed and released read the same
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for sw in switches(cx) {
            let Some((far, signals)) = button_sides(cx, sw) else {
                continue;
            };
            for net in signals {
                let p = net_pulls(cx, net);
                let same: Vec<_> =
                    p.up.iter()
                        .chain(&p.down)
                        .filter(|r| same_level(cx, r.to, far))
                        .map(|r| r.part)
                        .collect();
                if same.is_empty() {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .net(net)
                        .part(sw)
                        .message(format!(
                            "{} pulls {} to {} and {} pulls it there too: the button does nothing",
                            b.part(sw).designator,
                            b.net(net).name,
                            b.net(far).name,
                            designators(cx, same)
                        ))
                        .fix("pull towards the other level")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Io7));

#[cfg(test)]
mod tests {
    #[test]
    fn button_pulled_its_own_way() {
        let found = crate::testing::fingerprints(&super::Io7, "io-7_button_pulls");
        assert_eq!(
            found,
            ["IO-7:net /BTN_A|part SW1", "IO-7:net /BTN_B|part SW2"],
            "{found:?}"
        );
    }
}
