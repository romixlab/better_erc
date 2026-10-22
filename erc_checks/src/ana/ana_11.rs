//! `ANA-11` Unused op-amp channel left floating: wire it as a buffer to mid-rail or ground

use super::{connected, units};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::analog::amp_supplies;
use erc_model::{PartClass, PinId, Provenance};

pub struct Ana11;

static META: CheckMeta = CheckMeta {
    id: "ANA-11",
    category: "ANA",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Ana11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each op-amp channel with an input connected to nothing; with both inputs on rails or grounds; or wired as a
    /// follower whose non-inverting input sits on a supply rail of the part (or a ground): whether the output can
    /// reach that rail is part data (NET-7) not read yet, so that case is an estimate assuming it cannot
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for u in units(cx, PartClass::Opamp) {
            let (p, n, o) = (
                connected(cx, u.inp),
                connected(cx, u.inn),
                connected(cx, u.out),
            );
            let floating: Vec<PinId> = [(u.inp, p), (u.inn, n)]
                .into_iter()
                .filter_map(|(pin, net)| pin.filter(|_| net.is_none()))
                .collect();
            let f = cx.finding(&META).part(u.part);
            let finding = if !floating.is_empty() {
                let how = if o.is_none() && floating.len() == 2 {
                    "an unused channel left open"
                } else {
                    "an input connected to nothing"
                };
                Some(
                    f.pins(floating.iter().copied())
                        .message(format!(
                            "op-amp {}: {how} ({}); it picks up noise, swings between the rails and draws current",
                            u.label(b),
                            pin_list(cx, &floating)
                        )),
                )
            } else if let (Some(p), Some(n)) = (p, n)
                && power.is_power_net(p)
                && power.is_power_net(n)
            {
                Some(
                    f.pins(u.inp.into_iter().chain(u.inn)).message(format!(
                        "op-amp {}: both inputs on rails ({} and {}): outside the common-mode range or with a large differential voltage",
                        u.label(b),
                        b.net(p).name,
                        b.net(n).name
                    )),
                )
            } else if let (Some(p), Some(n), Some(o)) = (p, n, o)
                && n == o
                && power.is_power_net(p)
            {
                let (pos, neg) = amp_supplies(b, u.part);
                let at_rail = pos.contains(&p) || neg.contains(&p) || power.is_ground(p);
                at_rail.then(|| {
                    f.pins(u.inp)
                        .message(format!(
                            "op-amp {}: a follower with its input on {}, a supply rail: unless the output swings to that rail the loop saturates",
                            u.label(b),
                            b.net(p).name
                        ))
                        .fact(
                            "output swing to the rail",
                            "not reaching it",
                            Provenance::Assumed {
                                rule: "ANA-11: output swing not known (no part data), assumed not rail to rail".into(),
                            },
                        )
                })
            } else {
                None
            };
            if let Some(f) = finding {
                out.push(
                    f.fix("wire the channel as a follower with its input at mid-supply, or any voltage inside the input and output ranges")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Ana11));

#[cfg(test)]
mod tests {
    #[test]
    fn unused_channels() {
        let found = crate::testing::fingerprints(&super::Ana11, "ana-11_unused_channels");
        assert_eq!(
            found,
            [
                "ANA-11:part U1|pin U1.5|pin U1.6",
                "ANA-11:part U2|pin U2.5",
                "ANA-11:part U3|pin U3.5|pin U3.6"
            ],
            "{found:?}"
        );
    }
}
