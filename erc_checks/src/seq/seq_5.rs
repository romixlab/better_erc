//! `SEQ-5` High voltage net without self-discharge resistors

use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};
use erc_analysis::graph::series_paths;
use erc_analysis::supply::caps_to_ground;
use erc_analysis::value::format_si;
use erc_model::{NetId, PartId, Unit};

pub struct Seq5;

static META: CheckMeta = CheckMeta {
    id: "SEQ-5",
    category: "SEQ",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[
        Need::Graph,
        Need::PartClasses,
        Need::Values,
        Need::Rails,
        Need::RailVoltages,
    ],
    overlaps: &[],
    params: &[
        Param {
            name: "min_voltage",
            default: 50.0,
            unit: "V",
            doc: "the rail voltage (magnitude) above which a net is no longer safe to touch",
        },
        Param {
            name: "max_chain",
            default: 4.0,
            unit: "",
            doc: "the most resistors (and LEDs) in series followed from the rail to a ground",
        },
    ],
};

impl Check for Seq5 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A rail above `min_voltage` with capacitors to ground and no chain of fitted resistors (an LED with its
    /// resistor counts) from it to a ground to discharge them
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let values = cx.values();
        let min_voltage = cx.param(&META, "min_voltage");
        let max_chain = cx.param(&META, "max_chain") as usize;
        let bleeder_part = |p: PartId| {
            let c = cx.classes().get(p);
            cx.fitted(p) && (c.is_resistor() || c.is_led())
        };
        let node = |n: NetId| !power.is_power_net(n);
        let mut out = Vec::new();
        for (rail, r) in &power.rails {
            let Some(volts) = r.volts() else { continue };
            if volts.abs() <= min_voltage {
                continue;
            }
            let caps = caps_to_ground(cx.view(), cx.classes(), power, *rail);
            if caps.is_empty() {
                continue;
            }
            let bled = series_paths(b, *rail, &bleeder_part, &node, max_chain)
                .into_iter()
                .any(|p| {
                    power.is_ground(p.end)
                        && p.parts.iter().any(|x| cx.classes().get(*x).is_resistor())
                });
            if bled {
                continue;
            }
            let farads: f64 = caps.iter().filter_map(|c| values.get(*c).value).sum();
            let mut f = cx
                .finding(&META)
                .net(*rail)
                .parts(caps.iter().copied())
                .fact(
                    "min_voltage",
                    format_si(min_voltage, Unit::Volt),
                    cx.param_source(&META, "min_voltage"),
                );
            if let Some(v) = r.voltage.resolve() {
                f = f.fact(
                    "rail voltage",
                    format_si(v.value, Unit::Volt),
                    v.source.clone(),
                );
            }
            let stored = if farads > 0.0 {
                format!(
                    " ({} holding {:.2} J)",
                    format_si(farads, Unit::Farad),
                    0.5 * farads * volts * volts
                )
            } else {
                String::new()
            };
            out.push(
                f.message(format!(
                    "{} ({}) has {}{stored} and no resistor to discharge them when the supply is removed",
                    b.net(*rail).name,
                    format_si(volts, Unit::Volt),
                    designators(cx, caps.iter().copied()),
                ))
                .fix("a bleeder resistor (or a chain rated for the voltage) to ground, sized for the discharge time and its dissipation in operation")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Seq5));

#[cfg(test)]
mod tests {
    #[test]
    fn high_voltage_without_bleeder() {
        let found = crate::testing::fingerprints(&super::Seq5, "seq-5_bleeder");
        assert_eq!(found, ["SEQ-5:net +60V|part C1|part C2"], "{found:?}");
    }
}
