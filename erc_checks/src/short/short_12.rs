//! `SHORT-12` Differential pair with P tied to N

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::names::{DiffHalf, Half, diff_half};
use erc_analysis::power::{NameRole, read_name};
use erc_model::{NetId, PinId};
use std::collections::{BTreeMap, BTreeSet};

pub struct Short12;

static META: CheckMeta = CheckMeta {
    id: "SHORT-12",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

/// The half a pin name is, not counting supply pins (`V+`, `V-`)
fn pin_half(name: &str) -> Option<DiffHalf> {
    if matches!(
        read_name(name, false).role,
        NameRole::Rail(_) | NameRole::Ground(_)
    ) {
        return None;
    }
    diff_half(name)
}

impl Check for Short12 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A part whose positive and negative pins of one pair (D+/D-, DP/DM, TX_P/TX_N, CANH/CANL) are on one net;
    /// a net carrying labels of both halves of one pair (`USB_D_P` and `USB_D_N`)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for p in cx.components() {
            let mut halves: BTreeMap<String, [Vec<PinId>; 2]> = BTreeMap::new();
            for q in &b.part(p).pins {
                if let Some(h) = pin_half(&b.pin(*q).name) {
                    let i = usize::from(h.half == Half::Neg);
                    halves.entry(h.key()).or_default()[i].push(*q);
                }
            }
            for [pos, neg] in halves.values() {
                let nets = |pins: &[PinId]| -> BTreeSet<NetId> {
                    pins.iter().filter_map(|q| b.pin(*q).net).collect()
                };
                let Some(net) = nets(pos).intersection(&nets(neg)).next().copied() else {
                    continue;
                };
                let on = |pins: &[PinId]| pins.iter().copied().find(|q| b.pin(*q).net == Some(net));
                let (a, c) = (on(pos).expect("on the net"), on(neg).expect("on the net"));
                out.push(
                    cx.finding(&META)
                        .net(net)
                        .part(p)
                        .pin(a)
                        .pin(c)
                        .message(format!(
                            "{}: {} and {} of one differential pair are both on {}",
                            b.part(p).designator,
                            b.pin(a).name,
                            b.pin(c).name,
                            b.net(net).name
                        ))
                        .fix("relabel the pair: one of the two labels was copied without changing its half")
                        .build(),
                );
            }
        }
        for i in 0..b.nets.len() {
            let net = NetId::from_index(i);
            let mut halves: BTreeMap<String, [Vec<&str>; 2]> = BTreeMap::new();
            for l in &b.net(net).labels {
                let text = b.label(*l).text.as_str();
                if let Some(h) = diff_half(text) {
                    halves.entry(h.key()).or_default()[usize::from(h.half == Half::Neg)].push(text);
                }
            }
            for [pos, neg] in halves.values() {
                if pos.is_empty() || neg.is_empty() {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .net(net)
                        .message(format!(
                            "{}: labels {} and {} of both halves of one pair name one net",
                            b.net(net).name,
                            pos[0],
                            neg[0]
                        ))
                        .fix("relabel the pair")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Short12));

#[cfg(test)]
mod tests {
    #[test]
    fn halves_tied() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short12, "short-12_pair_halves_tied"),
            [
                "SHORT-12:net /ETH_TX_N",
                "SHORT-12:net /USB_D_P|part U1|pin U1.1|pin U1.2",
            ]
        );
    }
}
