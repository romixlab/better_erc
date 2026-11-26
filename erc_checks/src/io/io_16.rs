//! `IO-16` Pull-up and pull-down both on one net: an accidental divider, neither level clean

use super::net_pulls;
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::access::series_neighbours;
use erc_analysis::analog::is_analog_input;
use erc_analysis::graph::net_pins;
use erc_analysis::value::format_si;
use erc_model::{NetId, PartClass, PartId, Unit};

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
    /// away reaches an analog input by name or by the alternate chosen on the placed symbol (`FB`, `ADC`, `VREF`,
    /// `PA5/ADC1_IN5`, amplifier inputs), a pin of an op-amp, comparator, regulator or load switch (feedback, enable
    /// and UVLO thresholds are dividers on purpose), or a [`sense_divider`]; with the divided voltage where values
    /// and the rail's voltage are known
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
                let p = b.pin(pin);
                let class = cx.classes().get(p.part);
                cx.fitted(p.part)
                    && (is_analog_input(&p.name)
                        || p.alt.as_deref().is_some_and(is_analog_input)
                        || matches!(
                            class,
                            PartClass::Opamp
                                | PartClass::Comparator
                                | PartClass::Regulator { .. }
                                | PartClass::LoadSwitch
                        ))
            });
            if analog || sense_divider(cx, net, &p.up) {
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

/// Whether a fitted diode clamps `net` to a rail none of the pull-ups go to (anode on the net, cathode on the rail),
/// and no pull-up's rail is known to sit at or below that rail: the pull-ups and pull-downs bring a higher rail down
/// to the reader's level, a sense divider (VBUS detect)
fn sense_divider(cx: &Ctx, net: NetId, ups: &[erc_analysis::pull::Pull]) -> bool {
    let b = cx.board();
    let power = cx.power();
    let side = |part: PartId, names: &[&str]| {
        b.part(part)
            .pins
            .iter()
            .map(|p| b.pin(*p))
            .find(|p| names.iter().any(|n| p.name.eq_ignore_ascii_case(n)))
            .and_then(|p| p.net)
    };
    net_pins(b, net).any(|pin| {
        let part = b.pin(pin).part;
        let class = cx.classes().get(part);
        if !cx.fitted(part) || !class.is_diode() || class.is_led() {
            return false;
        }
        let (Some(anode), Some(clamp)) =
            (side(part, &["A", "ANODE"]), side(part, &["K", "CATHODE"]))
        else {
            return false;
        };
        anode == net
            && power.is_rail(clamp)
            && ups.iter().all(|u| {
                u.to != clamp
                    && match (power.volts(u.to), power.volts(clamp)) {
                        (Some(up), Some(down)) => up > down,
                        _ => true,
                    }
            })
    })
}

inventory::submit!(Registered(&Io16));

#[cfg(test)]
mod tests {
    #[test]
    fn pull_up_and_down() {
        let found = crate::testing::fingerprints(&super::Io16, "io-16_pull_up_and_down");
        assert_eq!(found, ["IO-16:net /BOOT|part R1|part R2"], "{found:?}");
    }

    #[test]
    fn chosen_adc_alternate_is_a_sense_divider() {
        let found = crate::testing::fingerprints(&super::Io16, "io-16_adc_alternate");
        assert_eq!(found, ["IO-16:net /LEVEL_B|part R3|part R4"], "{found:?}");
    }

    #[test]
    fn sense_divider_with_clamp() {
        let found = crate::testing::fingerprints(&super::Io16, "io-16_sense_divider");
        assert_eq!(
            found,
            [
                "IO-16:net /SENSE_LOW|part R5|part R6",
                "IO-16:net /STRAP|part R3|part R4"
            ],
            "{found:?}"
        );
    }
}
