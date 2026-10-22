//! `I2C-9` Pull-up value outside the recommended range

use super::{lines, ohms, shown};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::value::format_si;
use erc_model::Unit;

pub struct I2c9;

static META: CheckMeta = CheckMeta {
    id: "I2C-9",
    category: "I2C",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[
        Param {
            name: "min_pull_up",
            default: 2200.0,
            unit: "ohm",
            doc: "lower end of the pull-up window; a value at it is reported",
        },
        Param {
            name: "max_pull_up",
            default: 10_000.0,
            unit: "ohm",
            doc: "upper end of the pull-up window; a value at it is reported",
        },
    ],
};

impl Check for I2c9 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Pull-ups at or beyond either end of the window, or whose value does not read
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let (min, max) = (
            cx.param(&META, "min_pull_up"),
            cx.param(&META, "max_pull_up"),
        );
        let mut out = Vec::new();
        for bus in &cx.i2c().buses {
            let Some(pu) = bus.pull_up else { continue };
            let resistors = [pu.scl, pu.sda];
            let values = resistors.map(|r| ohms(cx, r));
            let bad: Vec<usize> = (0..2)
                .filter(|i| values[*i].value.is_none_or(|r| r <= min || r >= max))
                .collect();
            if bad.is_empty() {
                continue;
            }
            let what: Vec<String> = bad
                .iter()
                .map(|i| {
                    format!(
                        "{} {}",
                        b.part(resistors[*i]).designator,
                        shown(&values[*i])
                    )
                })
                .collect();
            let mut f = cx
                .finding(&META)
                .nets(lines(bus))
                .parts(bad.iter().map(|i| resistors[*i]))
                .message(format!(
                    "I2C bus {}: pull-up {} outside {}..{}",
                    bus.name,
                    what.join(", "),
                    format_si(min, Unit::Ohm),
                    format_si(max, Unit::Ohm)
                ));
            for i in &bad {
                if let Some(src) = &values[*i].source {
                    f = f.fact(
                        &b.part(resistors[*i]).designator,
                        shown(&values[*i]),
                        src.clone(),
                    );
                }
            }
            out.push(
                f.fact(
                    "window",
                    format!(
                        "{}..{}",
                        format_si(min, Unit::Ohm),
                        format_si(max, Unit::Ohm)
                    ),
                    cx.param_source(&META, "min_pull_up"),
                )
                .fix("pick a value inside the window for the bus voltage and mode (TI SLVA689)")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&I2c9));

#[cfg(test)]
mod tests {
    use crate::testing::fingerprints;

    #[test]
    fn outside_the_window() {
        assert_eq!(
            fingerprints(&super::I2c9, "i2c-9_non_standard_pull_ups"),
            ["I2C-9:net /SCL|net /SDA|part R301|part R302"]
        );
        let found = fingerprints(&super::I2c9, "i2c-7_segments");
        for net in ["net /SCL1_3V3_VCC2|", "net /SCL1_3V3|", "net /SCL2_3V3|"] {
            assert!(found.iter().any(|f| f.contains(net)), "{net} {found:?}");
        }
    }
}
