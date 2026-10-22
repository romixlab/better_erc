//! `I2C-7` Series tie resistor on an I2C line too high

use super::{lines, ohms, shown};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_analysis::i2c::Node;
use erc_analysis::value::format_si;
use erc_model::Unit;
use std::collections::BTreeSet;

pub struct I2c7;

static META: CheckMeta = CheckMeta {
    id: "I2C-7",
    category: "I2C",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[Param {
        name: "max_tie",
        default: 100.0,
        unit: "ohm",
        doc: "highest resistance of a series tie in an SCL or SDA line",
    }],
};

impl Check for I2c7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Ties (a resistor per line between two buses) above `max_tie`; a value that does not read counts as 0 Ω
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let i2c = cx.i2c();
        let max = cx.param(&META, "max_tie");
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for bus in &i2c.buses {
            for node in &bus.nodes {
                let Node::Tie {
                    scl_tie,
                    sda_tie,
                    other_side,
                } = *node
                else {
                    continue;
                };
                let resistors = [scl_tie, sda_tie];
                if !resistors
                    .iter()
                    .all(|r| cx.classes().is(*r, PartClass::Resistor))
                    || !seen.insert((scl_tie.min(sda_tie), scl_tie.max(sda_tie)))
                {
                    continue;
                }
                let values = resistors.map(|r| ohms(cx, r));
                let highest = values
                    .iter()
                    .map(|v| v.value.unwrap_or(0.0))
                    .fold(0.0, f64::max);
                if highest <= max {
                    continue;
                }
                let other = &i2c.buses[other_side];
                let mut f = cx
                    .finding(&META)
                    .nets(lines(bus))
                    .nets(lines(other))
                    .parts(resistors)
                    .message(format!(
                        "I2C bus {} to {}: tie {}/{} is {}, over {}",
                        bus.name,
                        other.name,
                        b.part(scl_tie).designator,
                        b.part(sda_tie).designator,
                        format_si(highest, Unit::Ohm),
                        format_si(max, Unit::Ohm)
                    ));
                for (r, v) in resistors.iter().zip(&values) {
                    if let Some(src) = &v.source {
                        f = f.fact(&b.part(*r).designator, shown(v), src.clone());
                    }
                }
                out.push(
                    f.fact(
                        "max_tie",
                        format_si(max, Unit::Ohm),
                        cx.param_source(&META, "max_tie"),
                    )
                    .fix("0 Ω ties; for protection a TVS, not a resistor")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&I2c7));

#[cfg(test)]
mod tests {
    #[test]
    fn tie_over_100_ohm() {
        let found = crate::testing::fingerprints(&super::I2c7, "i2c-7_segments");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("part R413|part R414"), "{found:?}");
    }
}
