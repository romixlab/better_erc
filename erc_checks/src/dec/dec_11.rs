//! `DEC-11` Parallel ceramics a decade apart

use super::{caps, designators};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::value::format_si;
use erc_model::{PartClass, PartId, Unit};

pub struct Dec11;

static META: CheckMeta = CheckMeta {
    id: "DEC-11",
    category: "DEC",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[Param {
        name: "max_ratio",
        default: 10.0,
        unit: "",
        doc: "neighbouring capacitor values on a rail this many times apart or more are a gap",
    }],
};

impl Check for Dec11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Per rail, the values of the fitted ceramic (non-polarized) capacitors to ground sorted; each gap of
    /// `max_ratio` or more between two neighbouring values is a finding
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let ratio = cx.param(&META, "max_ratio");
        let mut out = Vec::new();
        for net in cx.power().rails.keys() {
            // (value, the capacitors of that value), ascending
            let mut values: Vec<(f64, Vec<PartId>)> = Vec::new();
            for c in caps(cx, *net) {
                if cx.classes().get(c) != (PartClass::Capacitor { polarized: false }) {
                    continue;
                }
                let Some(v) = cx.values().get(c).value.filter(|v| *v > 0.0) else {
                    continue;
                };
                match values.iter_mut().find(|(x, _)| ((x - v) / v).abs() < 1e-9) {
                    Some((_, parts)) => parts.push(c),
                    None => values.push((v, vec![c])),
                }
            }
            values.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in values.windows(2) {
                let [(lo, small), (hi, large)] = pair else {
                    continue;
                };
                if hi / lo < ratio {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .net(*net)
                        .parts(small.iter().chain(large).copied())
                        .message(format!(
                            "{}: {} ({}) next to {} ({}), {:.0}× apart with no value in between: an anti-resonance peak",
                            b.net(*net).name,
                            format_si(*lo, Unit::Farad),
                            designators(cx, small.clone()),
                            format_si(*hi, Unit::Farad),
                            designators(cx, large.clone()),
                            hi / lo
                        ))
                        .fact("max_ratio", ratio, cx.param_source(&META, "max_ratio"))
                        .fix("several of one value, or a graded set (10 µF, 2.2 µF, 470 nF, 100 nF); for fast rails a PDN target and a simulation")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Dec11));

#[cfg(test)]
mod tests {
    #[test]
    fn decade_gaps() {
        let found = crate::testing::fingerprints(&super::Dec11, "dec-11_decade_apart");
        assert_eq!(
            found,
            [
                "DEC-11:net +1V2_CORE|part C1|part C2",
                "DEC-11:net +1V2_CORE|part C2|part C3"
            ],
            "{found:?}"
        );
    }
}
