//! `CONN-10` Many pins and few grounds on a connector: suggest more

use super::{connectors, contacts, position};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct Conn10;

static META: CheckMeta = CheckMeta {
    id: "CONN-10",
    category: "CONN",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "min_signals",
            default: 10.0,
            unit: "pins",
            doc: "a connector with fewer signal pins is not judged",
        },
        Param {
            name: "max_signals_per_ground",
            default: 6.0,
            unit: "pins",
            doc: "signal pins per ground pin above this are too few grounds",
        },
        Param {
            name: "max_distance",
            default: 8.0,
            unit: "pins",
            doc: "a signal pin further than this from the nearest ground in pin order has a large loop",
        },
    ],
};

impl Check for Conn10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A connector with at least `min_signals` signal pins (on a net that is no rail or ground) and some ground
    /// pins (none is `CONN-4`): more than `max_signals_per_ground` signals per ground, or, where pins are numbered,
    /// a signal pin more than `max_distance` pins from the nearest ground
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let min_signals = cx.param(&META, "min_signals");
        let max_ratio = cx.param(&META, "max_signals_per_ground");
        let max_distance = cx.param(&META, "max_distance");
        let mut out = Vec::new();
        for part in connectors(cx) {
            let pins = contacts(cx, part);
            let grounds: Vec<_> = pins
                .iter()
                .filter(|(_, n)| power.is_ground(*n))
                .map(|(p, _)| *p)
                .collect();
            let signals: Vec<_> = pins
                .iter()
                .filter(|(_, n)| !power.is_power_net(*n))
                .map(|(p, _)| *p)
                .collect();
            if grounds.is_empty() || (signals.len() as f64) < min_signals {
                continue;
            }
            let ratio = signals.len() as f64 / grounds.len() as f64;
            let ground_at: Vec<u32> = grounds.iter().filter_map(|p| position(cx, *p)).collect();
            let far = (ground_at.len() == grounds.len())
                .then(|| {
                    signals
                        .iter()
                        .filter_map(|s| {
                            let at = position(cx, *s)?;
                            let d = ground_at.iter().map(|g| g.abs_diff(at)).min()?;
                            Some((d, *s))
                        })
                        .max()
                })
                .flatten()
                .filter(|(d, _)| f64::from(*d) > max_distance);
            let mut what = Vec::new();
            if ratio > max_ratio {
                what.push(format!(
                    "{} signal pins on {} ground{} ({ratio:.1} per ground)",
                    signals.len(),
                    grounds.len(),
                    if grounds.len() == 1 { "" } else { "s" }
                ));
            }
            if let Some((d, s)) = far {
                what.push(format!(
                    "pin {} is {d} pins from the nearest ground",
                    b.pin(s).number
                ));
            }
            if what.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(part)
                    .message(format!(
                        "connector {}: {}: every edge shows up as ground bounce on the others, and the far signals have a large loop",
                        b.part(part).designator,
                        what.join("; ")
                    ))
                    .fact("max_signals_per_ground", max_ratio, cx.param_source(&META, "max_signals_per_ground"))
                    .fact("max_distance", max_distance, cx.param_source(&META, "max_distance"))
                    .fix("more grounds, spread between the signal groups; for fast interfaces one per pair or per two to three single-ended lines (CONN-5)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Conn10));

#[cfg(test)]
mod tests {
    #[test]
    fn few_grounds() {
        let found = crate::testing::fingerprints(&super::Conn10, "conn-10_few_grounds");
        assert_eq!(found, ["CONN-10:part J1", "CONN-10:part J2",], "{found:?}");
    }
}
