//! `DEC-7` Fewer decoupling capacitors than power pins

use super::{caps, designators, pin_list};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::supply::power_in_by_part;
use erc_analysis::value::format_si;
use erc_model::{PartId, Unit, natural_cmp};

pub struct Dec7;

static META: CheckMeta = CheckMeta {
    id: "DEC-7",
    category: "DEC",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[Param {
        name: "max_decoupling",
        default: 1e-6,
        unit: "F",
        doc: "the largest capacitor counted as a pin's local decoupling",
    }],
};

impl Check for Dec7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Per rail, the small capacitors on it are handed out one per power-input pin, the parts with the fewest
    /// pins first; a part left with fewer than it has pins is a finding. A rail with no capacitor at all is
    /// DEC-1's.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let max = cx.param(&META, "max_decoupling");
        let mut out = Vec::new();
        for net in cx.power().rails.keys() {
            let all = caps(cx, *net);
            if all.is_empty() {
                continue;
            }
            // A capacitor whose value does not read counts as small (BOM-10 reports the value)
            let small: Vec<PartId> = all
                .into_iter()
                .filter(|c| cx.values().get(*c).value.is_none_or(|v| v <= max))
                .collect();
            let mut loads: Vec<(PartId, Vec<_>)> =
                power_in_by_part(cx.view(), *net).into_iter().collect();
            loads.sort_by(|(a, pa), (c, pc)| {
                pa.len()
                    .cmp(&pc.len())
                    .then_with(|| natural_cmp(&b.part(*a).designator, &b.part(*c).designator))
            });
            let mut pool = small.len();
            let mut served: Vec<PartId> = Vec::new();
            for (part, pins) in &loads {
                let got = pool.min(pins.len());
                pool -= got;
                if got < pins.len() {
                    let others = if served.is_empty() {
                        String::new()
                    } else {
                        format!(" after {} took theirs", designators(cx, served.clone()))
                    };
                    out.push(
                        cx.finding(&META)
                            .part(*part)
                            .net(*net)
                            .message(format!(
                                "{} has {} power pins on {} ({}) and {got} of the {} capacitors of at most {} there are left for it{others}",
                                b.part(*part).designator,
                                pins.len(),
                                b.net(*net).name,
                                pin_list(cx, pins),
                                small.len(),
                                format_si(max, Unit::Farad),
                            ))
                            .fact(
                                "max_decoupling",
                                format_si(max, Unit::Farad),
                                cx.param_source(&META, "max_decoupling"),
                            )
                            .fix("one capacitor per power pin, placed at the pin")
                            .build(),
                    );
                }
                served.push(*part);
            }
        }
        out
    }
}

inventory::submit!(Registered(&Dec7));

#[cfg(test)]
mod tests {
    #[test]
    fn too_few_per_pin() {
        let found = crate::testing::fingerprints(&super::Dec7, "dec-7_caps_per_pin");
        assert_eq!(found, ["DEC-7:net +3V3|part U1"], "{found:?}");
    }
}
