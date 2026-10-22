//! `LVL-4` I2C discrete translator pull-up to the wrong voltage

use super::{device_rails, rail_text, shifters};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::NetId;

pub struct Lvl4;

static META: CheckMeta = CheckMeta {
    id: "LVL-4",
    category: "LVL",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[
        Need::Graph,
        Need::PartClasses,
        Need::Rails,
        Need::RailVoltages,
    ],
    overlaps: &[],
    params: &[],
};

impl Check for Lvl4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// In a BSS138-pair shifter each side's pull-ups go to that side's rail: both sides on one rail, the source
    /// (low) side on a higher voltage than the drain side, or a side pulled above the supply of the devices on it
    /// is the finding
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let i2c = cx.i2c();
        let mut out = Vec::new();
        for s in shifters(cx) {
            let pull = |bus: usize| i2c.buses[bus].pull_up.map(|p| p.rail);
            let (low, high) = (pull(s.low), pull(s.high));
            let mut why: Vec<String> = Vec::new();
            let mut nets: Vec<NetId> = Vec::new();
            if let (Some(l), Some(h)) = (low, high) {
                if l == h {
                    why.push(format!("both sides are pulled up to {}", rail_text(cx, l)));
                    nets.push(l);
                } else if let (Some(vl), Some(vh)) = (power.volts(l), power.volts(h))
                    && vl > vh + 0.05
                {
                    why.push(format!(
                        "the low (source) side is pulled to {}, above the high side's {}",
                        rail_text(cx, l),
                        rail_text(cx, h)
                    ));
                    nets.extend([l, h]);
                }
            }
            for (side, rail) in [(s.low, low), (s.high, high)] {
                let Some(rail) = rail else { continue };
                let Some(v) = power.volts(rail) else { continue };
                let supplies: Vec<(NetId, f64)> = device_rails(cx, side)
                    .into_iter()
                    .filter_map(|r| Some((r, power.volts(r)?)))
                    .collect();
                let Some(&(top, vmax)) = supplies.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else {
                    continue;
                };
                if v > vmax + 0.05 && !nets.contains(&rail) {
                    why.push(format!(
                        "{} is pulled to {}, above its devices' supply {}",
                        i2c.buses[side].name,
                        rail_text(cx, rail),
                        rail_text(cx, top)
                    ));
                    nets.push(rail);
                }
            }
            if why.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .parts(s.fets)
                    .nets(nets.iter().copied())
                    .message(format!(
                        "shifter {} and {} between {} and {}: {}",
                        b.part(s.fets[0]).designator,
                        b.part(s.fets[1]).designator,
                        i2c.buses[s.low].name,
                        i2c.buses[s.high].name,
                        why.join("; ")
                    ))
                    .fix("each side's pull-ups to that side's IO rail; or a PCA9306 / TCA9406 class part whose pins name the sides")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Lvl4));

#[cfg(test)]
mod tests {
    #[test]
    fn pull_ups_to_the_wrong_side() {
        let found = crate::testing::fingerprints(&super::Lvl4, "lvl-4_translator_pull_ups");
        assert_eq!(
            found,
            [
                "LVL-4:net +3V3|net +5V|part Q5|part Q6",
                "LVL-4:net +3V3|part Q3|part Q4"
            ],
            "{found:?}"
        );
    }
}
