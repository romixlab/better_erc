//! `LVL-9` Discrete translator with a gate reference that gives the wrong threshold for one side

use super::{device_rails, rail_text, shifters};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::NetId;

pub struct Lvl9;

static META: CheckMeta = CheckMeta {
    id: "LVL-9",
    category: "LVL",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Lvl9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The gates of a BSS138-pair shifter must be on the low side's rail: the supply of the devices on the FETs'
    /// source side when they share one, else the source side's pull-up rail, else the lower of the two pull-up
    /// rails. A gate on no rail, on a rail of a higher voltage than that, or (voltages unknown) on another rail is
    /// the finding.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let i2c = cx.i2c();
        let mut out = Vec::new();
        for s in shifters(cx) {
            let pull = |bus: usize| i2c.buses[bus].pull_up.map(|p| p.rail);
            let devices = device_rails(cx, s.low);
            let expected: Option<NetId> = if devices.len() == 1 {
                devices.first().copied()
            } else {
                pull(s.low)
            };
            let mut wrong: Vec<(NetId, String)> = Vec::new();
            for g in s
                .gates
                .iter()
                .flatten()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
            {
                let why = if !power.is_rail(g) {
                    Some(format!("{} is no rail", b.net(g).name))
                } else if let Some(e) = expected {
                    match (power.volts(g), power.volts(e)) {
                        (Some(vg), Some(ve)) if vg > ve + 0.05 => Some(format!(
                            "{} is above the low side's rail {}: the FETs never open and the low side follows the high side up",
                            rail_text(cx, g),
                            rail_text(cx, e)
                        )),
                        (None, _) | (_, None) if g != e => Some(format!(
                            "{} is not the low side's rail {}",
                            b.net(g).name,
                            b.net(e).name
                        )),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(why) = why {
                    wrong.push((g, why));
                }
            }
            if wrong.is_empty() {
                continue;
            }
            let reasons: Vec<&str> = wrong.iter().map(|(_, w)| w.as_str()).collect();
            out.push(
                cx.finding(&META)
                    .parts(s.fets)
                    .nets(wrong.iter().map(|(g, _)| *g))
                    .message(format!(
                        "shifter {} and {} between {} and {}: the gate {}",
                        b.part(s.fets[0]).designator,
                        b.part(s.fets[1]).designator,
                        i2c.buses[s.low].name,
                        i2c.buses[s.high].name,
                        reasons.join("; ")
                    ))
                    .fix("gates to the lower rail, sources on the lower side, drains on the higher side")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Lvl9));

#[cfg(test)]
mod tests {
    #[test]
    fn gate_on_the_high_rail() {
        let found = crate::testing::fingerprints(&super::Lvl9, "lvl-9_gate_reference");
        assert_eq!(found, ["LVL-9:net +5V|part Q3|part Q4"], "{found:?}");
    }
}
