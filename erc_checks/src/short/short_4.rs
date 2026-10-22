//! `SHORT-4` Power to ground through low-value resistors

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::series_paths;
use erc_analysis::value::format_si;
use erc_model::{NetId, PartClass, PartId, Provenance, Unit};

pub struct Short4;

static META: CheckMeta = CheckMeta {
    id: "SHORT-4",
    category: "SHORT",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "max_current",
            default: 0.05,
            unit: "A",
            doc: "the current a resistor path from a rail to ground may draw",
        },
        Param {
            name: "max_chain",
            default: 3.0,
            unit: "",
            doc: "the most resistors in series followed from a rail to a ground",
        },
    ],
};

impl Check for Short4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Paths of fitted resistors in series from a rail to a ground (through signal nets, dividers included): a
    /// 0 Ω path is a dead short; with the rail's voltage, a path drawing more than `max_current`, or burning more
    /// than a resistor's power rating where its value states one
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let values = cx.values();
        let power = cx.power();
        let max_current = cx.param(&META, "max_current");
        let max_chain = cx.param(&META, "max_chain") as usize;
        let resistor = |p: PartId| {
            cx.fitted(p) && classes.get(p) == PartClass::Resistor && values.get(p).value.is_some()
        };
        let node = |n: NetId| !power.is_power_net(n) && !power.switching_nodes.contains(&n);
        let mut out = Vec::new();
        for (rail, r) in &power.rails {
            for path in series_paths(b, *rail, &resistor, &node, max_chain) {
                if !power.is_ground(path.end) {
                    continue;
                }
                let ohms: f64 = path.parts.iter().filter_map(|p| values.get(*p).value).sum();
                let volts = r.volts().map(f64::abs);
                let current = volts.map(|v| if ohms > 0.0 { v / ohms } else { f64::INFINITY });
                let over_current = current.is_some_and(|i| i > max_current);
                // A resistor's own power rating, where the value states one (`1/4W`)
                let overheated: Vec<PartId> = match current {
                    Some(i) if i.is_finite() => path
                        .parts
                        .iter()
                        .copied()
                        .filter(|p| {
                            let v = values.get(*p);
                            let watts = v.ratings.iter().find(|q| q.unit == Unit::Watt);
                            watts.is_some_and(|w| i * i * v.value.unwrap_or(0.0) > w.value)
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                if ohms > 0.0 && !over_current && overheated.is_empty() {
                    continue;
                }
                let names: Vec<&str> = path
                    .parts
                    .iter()
                    .map(|p| b.part(*p).designator.as_str())
                    .collect();
                let mut f = cx
                    .finding(&META)
                    .nets([*rail, path.end])
                    .parts(path.parts.iter().copied())
                    .fact(
                        "path resistance",
                        format_si(ohms, Unit::Ohm),
                        Provenance::Derived {
                            rule: "the Value fields along the path, in series".into(),
                        },
                    );
                let what = if ohms == 0.0 {
                    "a dead short".to_string()
                } else if let Some(i) = current {
                    format!(
                        "{} from {}",
                        format_si(i, Unit::Ampere),
                        format_si(volts.unwrap_or(0.0), Unit::Volt)
                    )
                } else {
                    unreachable!("a nonzero path is reported only with a current")
                };
                if let Some(v) = r.voltage.resolve() {
                    f = f.fact(
                        "rail voltage",
                        format_si(v.value, Unit::Volt),
                        v.source.clone(),
                    );
                }
                if over_current {
                    f = f.fact(
                        "max_current",
                        format_si(max_current, Unit::Ampere),
                        cx.param_source(&META, "max_current"),
                    );
                }
                let hot = if overheated.is_empty() {
                    String::new()
                } else {
                    let h: Vec<&str> = overheated
                        .iter()
                        .map(|p| b.part(*p).designator.as_str())
                        .collect();
                    format!("; {} above its power rating", h.join(", "))
                };
                out.push(
                    f.message(format!(
                        "{} to {} through {} ({}): {what}{hot}",
                        b.net(*rail).name,
                        b.net(path.end).name,
                        names.join(", "),
                        format_si(ohms, Unit::Ohm)
                    ))
                    .fix("correct the value (10 for 10k?) or the wiring; size a real load for its current and power")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Short4));

#[cfg(test)]
mod tests {
    #[test]
    fn low_resistance_to_ground() {
        assert_eq!(
            crate::testing::fingerprints(&super::Short4, "short-4_low_resistance"),
            [
                "SHORT-4:net +3V3|net GND|part R1",
                "SHORT-4:net +5V|net GND|part R3|part R4",
            ]
        );
    }
}
