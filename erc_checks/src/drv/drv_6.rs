//! `DRV-6` Q-Q chain not back to back

use super::{DRAIN, SOURCE, pin_named};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_model::class::Channel;
use erc_model::{NetId, PartClass, PartId};

pub struct Drv6;

static META: CheckMeta = CheckMeta {
    id: "DRV-6",
    category: "DRV",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// A fitted MOSFET of known channel with its drain and source nets
fn fet(cx: &Ctx, q: PartId) -> Option<(Channel, NetId, NetId)> {
    let PartClass::Mosfet {
        channel: Some(channel),
    } = cx.classes().get(q)
    else {
        return None;
    };
    let (_, d) = pin_named(cx, q, DRAIN)?;
    let (_, s) = pin_named(cx, q, SOURCE)?;
    (cx.fitted(q) && d != s).then_some((channel, d, s))
}

impl Check for Drv6 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Two MOSFETs in series through a middle net that holds only them and gate-drive passives (resistors,
    /// capacitors, diodes), between two nets that are no grounds (a half bridge's ends are a rail and a ground),
    /// whose body diodes point the same way: N-channel cathode at the drain, P-channel at the source. Back to back
    /// is both cathodes or both anodes in the middle.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let fets: Vec<(PartId, Channel, NetId, NetId)> = cx
            .view()
            .fitted_components()
            .filter_map(|q| fet(cx, q).map(|(c, d, s)| (q, c, d, s)))
            .collect();
        let mut out = Vec::new();
        for (i, &(q1, c1, d1, s1)) in fets.iter().enumerate() {
            for &(q2, c2, d2, s2) in &fets[i + 1..] {
                let shared: Vec<NetId> = [d1, s1]
                    .into_iter()
                    .filter(|n| *n == d2 || *n == s2)
                    .collect();
                let [m] = shared[..] else { continue };
                if power.is_power_net(m) {
                    continue;
                }
                let other = |d: NetId, s: NetId| if d == m { s } else { d };
                let (x1, x2) = (other(d1, s1), other(d2, s2));
                if x1 == x2 || power.is_ground(x1) || power.is_ground(x2) {
                    continue;
                }
                let only_drive = net_pins(b, m).all(|p| {
                    let part = b.pin(p).part;
                    let c = cx.classes().get(part);
                    part == q1
                        || part == q2
                        || c.is_resistor()
                        || c.is_capacitor()
                        || (c.is_diode() && !c.is_led())
                });
                if !only_drive {
                    continue;
                }
                let cathode_in_middle = |c: Channel, d: NetId| match c {
                    Channel::N => d == m,
                    Channel::P => d != m,
                };
                if cathode_in_middle(c1, d1) == cathode_in_middle(c2, d2) {
                    continue;
                }
                let joint = |d: NetId| if d == m { "drain" } else { "source" };
                out.push(
                    cx.finding(&META)
                        .parts([q1, q2])
                        .net(m)
                        .message(format!(
                            "{} ({} at {}) and {} ({} at {}) are in series between {} and {} with their body diodes the same way: current from {} flows through both diodes",
                            b.part(q1).designator,
                            joint(d1),
                            b.net(m).name,
                            b.part(q2).designator,
                            joint(d2),
                            b.net(m).name,
                            b.net(x1).name,
                            b.net(x2).name,
                            if cathode_in_middle(c1, d1) { b.net(x1).name.as_str() } else { b.net(x2).name.as_str() },
                        ))
                        .fix("turn one FET around so the sources (or drains) meet in the middle, and drive both gates together")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Drv6));

#[cfg(test)]
mod tests {
    #[test]
    fn series_not_back_to_back() {
        let found = crate::testing::fingerprints(&super::Drv6, "drv-6_back_to_back");
        assert_eq!(found, ["DRV-6:net /MID_B|part Q3|part Q4"], "{found:?}");
    }
}
