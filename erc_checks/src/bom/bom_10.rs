//! `BOM-10` Value that doesn't parse or isn't a standard series value

use super::placeholder;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_analysis::value::value_as;
use erc_model::{Unit, ValueWarning};

pub struct Bom10;

static META: CheckMeta = CheckMeta {
    id: "BOM-10",
    category: "BOM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

/// What is odd about a value's notation
fn describe(w: &ValueWarning) -> String {
    match w {
        ValueWarning::Trailing(t) => format!("text after the value (`{t}`)"),
        ValueWarning::UnitMismatch { written, .. } => {
            format!("written in {} on a resistor", written.name())
        }
        ValueWarning::DecimalComma => "a decimal comma".into(),
        ValueWarning::Spacing => "more than one space before the multiplier".into(),
        ValueWarning::LowercaseR => "a small `r` for ohms".into(),
        ValueWarning::RedundantR => "`R` after a multiplier".into(),
    }
}

impl Check for Bom10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Resistors so far (capacitors and the E-series test are still to come)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let mut out = Vec::new();
        for p in cx.components() {
            if !classes.is(p, PartClass::Resistor) {
                continue;
            }
            let v = value_as(b, p, cx.variant, Unit::Ohm);
            let text = v.text.trim();
            if text.is_empty() || placeholder(text) || text == "DNP" || text == "DNM" {
                continue;
            }
            let designator = &b.part(p).designator;
            let message = if v.value.is_none() {
                format!("{designator}: `{}` does not read as a resistance", v.text)
            } else if !v.warnings.is_empty() {
                let odd: Vec<String> = v.warnings.iter().map(describe).collect();
                format!("{designator}: `{}` has {}", v.text, odd.join(", "))
            } else {
                continue;
            };
            let class = classes.fact(p);
            let mut f = cx.finding(&META).part(p).message(message).fact(
                "class",
                class.value.name(),
                class.source.clone(),
            );
            if let Some(source) = v.source {
                f = f.fact("Value", &v.text, source);
            }
            out.push(
                f.fix("one notation across the project: `4k7`, `100R`, `2R2`")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Bom10));

#[cfg(test)]
mod tests {
    #[test]
    fn notation_and_parse_errors() {
        // R4 `10x`, R5 `4.7  k`, R6 `4r7`, R7 `10kR`; R1 `4k7` is fine, R2 empty, R3/R9/R10 placeholders, R8 `DNP`
        assert_eq!(
            crate::testing::fingerprints(&super::Bom10, "bom-10_values"),
            [
                "BOM-10:part R4",
                "BOM-10:part R5",
                "BOM-10:part R6",
                "BOM-10:part R7"
            ]
        );
    }
}
