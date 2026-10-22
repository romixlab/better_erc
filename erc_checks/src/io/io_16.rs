//! `IO-16` Pull-up and pull-down both on one net: an accidental divider, neither level clean

use super::net_pulls;
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::access::series_neighbours;
use erc_analysis::analog::is_analog_input;
use erc_analysis::graph::net_pins;
use erc_analysis::value::format_si;
use erc_model::{NetId, PartClass, Unit};

pub struct Io16;

static META: CheckMeta = CheckMeta {
    id: "IO-16",
    category: "IO",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for Io16 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A signal net with a fitted resistor to a rail and another to a ground, unless the net or one series part
    /// away reaches an analog input by name (`FB`, `ADC`, `VREF`, amplifier inputs), a pin of an op-amp,
    /// comparator, regulator or load switch (feedback, enable and UVLO thresholds are dividers on purpose); with
    /// the divided voltage where values and the rail's voltage are known
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for net in (0..b.nets.len()).map(NetId::from_index) {
            if power.is_power_net(net) {
                continue;
            }
            let p = net_pulls(cx, net);
            if p.up.is_empty() || p.down.is_empty() {
                continue;
            }
            let mut reach = vec![net];
            reach.extend(
                series_neighbours(cx.view(), cx.classes(), power, net)
                    .into_iter()
                    .map(|(_, n)| n),
            );
            let analog = reach.iter().flat_map(|n| net_pins(b, *n)).any(|pin| {
                let part = b.pin(pin).part;
                let class = cx.classes().get(part);
                cx.fitted(part)
                    && (is_analog_input(&b.pin(pin).name)
                        || matches!(
                            class,
                            PartClass::Opamp
                                | PartClass::Comparator
                                | PartClass::Regulator { .. }
                                | PartClass::LoadSwitch
                        ))
            });
            if analog {
                continue;
            }
            let mut f = cx.finding(&META).net(net).parts(p.parts());
            let mut level = String::new();
            let parallel = |rs: &[erc_analysis::pull::Pull]| -> Option<f64> {
                rs.iter()
                    .map(|r| cx.values().get(r.part).value.filter(|v| *v > 0.0))
                    .try_fold(0.0, |g, v| v.map(|v| g + 1.0 / v))
                    .map(|g| 1.0 / g)
            };
            let rails: Vec<f64> = p.up.iter().filter_map(|r| power.volts(r.to)).collect();
            if let (Some(ru), Some(rd), Some(v)) =
                (parallel(&p.up), parallel(&p.down), rails.first())
                && rails.len() == p.up.len()
                && rails.iter().all(|x| (x - v).abs() < 1e-9)
            {
                let mid = v * rd / (ru + rd);
                level = format!(": the net sits at {}", format_si(mid, Unit::Volt));
                f = f.fact(
                    "divided voltage",
                    format_si(mid, Unit::Volt),
                    erc_model::Provenance::Derived {
                        rule: "IO-16: rail voltage × Rdown / (Rup + Rdown)".into(),
                    },
                );
            }
            out.push(
                f.message(format!(
                    "{} has a pull-up ({}) and a pull-down ({}){level}, neither level clean",
                    b.net(net).name,
                    designators(cx, p.up.iter().map(|r| r.part)),
                    designators(cx, p.down.iter().map(|r| r.part))
                ))
                .fix("remove one, or make one of them DNP for a strap option")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Io16));

#[cfg(test)]
mod tests {
    #[test]
    fn pull_up_and_down() {
        let found = crate::testing::fingerprints(&super::Io16, "io-16_pull_up_and_down");
        assert_eq!(found, ["IO-16:net /BOOT|part R1|part R2"], "{found:?}");
    }
}
