//! `BOM-10` Value that doesn't parse or isn't a standard series value

use super::placeholder;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;
use erc_analysis::value::format_si;
use erc_model::{ESeries, Unit, ValueWarning};

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
fn describe(w: &ValueWarning, class: &str) -> String {
    match w {
        ValueWarning::Trailing(t) => format!("text after the value (`{t}`)"),
        ValueWarning::UnitMismatch { written, .. } => {
            format!("written in {} on a {class}", written.name())
        }
        ValueWarning::DecimalComma => "a decimal comma".into(),
        ValueWarning::Spacing => "more than one space before the multiplier".into(),
        ValueWarning::LowercaseR => "a small `r` for ohms".into(),
        ValueWarning::RedundantR => "`R` after a multiplier".into(),
        ValueWarning::SeparateUnit => "the unit apart from its value".into(),
    }
}

/// `a resistance` for a resistor
fn quantity_name(unit: Unit) -> &'static str {
    match unit {
        Unit::Ohm => "a resistance",
        Unit::Farad => "a capacitance",
        Unit::Henry => "an inductance",
        Unit::Volt => "a voltage",
        Unit::Ampere => "a current",
        Unit::Hertz => "a frequency",
        Unit::Watt => "a power",
    }
}

impl Check for Bom10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Resistors, capacitors and inductors: a value that does not read in the class's unit, odd notation, or a
    /// value in no E-series (E3..E192)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let mut out = Vec::new();
        for p in cx.components() {
            let class = classes.get(p);
            if !(class.is_resistor() || class.is_capacitor() || class == PartClass::Inductor) {
                continue;
            }
            let v = cx.values().get(p);
            let Some(unit) = v.expected else { continue };
            let text = v.text.trim();
            if text.is_empty() || placeholder(text) || text == "DNP" || text == "DNM" {
                continue;
            }
            let designator = &b.part(p).designator;
            let mut odd: Vec<String> = Vec::new();
            if v.value.is_none() {
                odd.push(format!("does not read as {}", quantity_name(unit)));
            } else {
                odd.extend(v.warnings.iter().map(|w| describe(w, class.name())));
                if v.series_applies() && v.series.is_none() {
                    let value = v.value.unwrap_or_default();
                    let nearest: Vec<String> = [ESeries::E24, ESeries::E96]
                        .into_iter()
                        .filter_map(|s| {
                            Some(format!(
                                "{} {}",
                                s.name(),
                                format_si(s.nearest(value)?, unit)
                            ))
                        })
                        .collect();
                    odd.push(format!(
                        "{} is in no E-series (nearest {})",
                        format_si(value, unit),
                        nearest.join(", ")
                    ));
                }
            }
            if odd.is_empty() {
                continue;
            }
            let class_fact = classes.fact(p);
            let mut f = cx
                .finding(&META)
                .part(p)
                .message(format!("{designator}: `{}` {}", v.text, odd.join("; ")))
                .fact("class", class.describe(), class_fact.source.clone());
            if let Some(source) = &v.source {
                f = f.fact(v.label(), &v.text, source.clone());
            }
            out.push(
                f.fix("one notation across the project (`4k7`, `100n`, `2R2`), values from the E-series the parts come from")
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
    fn notation_parse_errors_and_series() {
        // R4 `10x`, R5 `4.7  k`, R6 `4r7`, R7 `10kR`, R11 `5k` (no E-series), R13 `4k7 Ohm`, C2 `4.8n` (no
        // E-series), C3 `4,7u`, L2 `10x`; R1 `4k7`, R12 `4k99` (E96), C1 `100n 50V X7R`, L1 `10u` are fine, R2
        // empty, R3/R9/R10 placeholders, R8 `DNP`
        assert_eq!(
            crate::testing::fingerprints(&super::Bom10, "bom-10_values"),
            [
                "BOM-10:part C2",
                "BOM-10:part C3",
                "BOM-10:part L2",
                "BOM-10:part R11",
                "BOM-10:part R13",
                "BOM-10:part R4",
                "BOM-10:part R5",
                "BOM-10:part R6",
                "BOM-10:part R7"
            ]
        );
    }
}
