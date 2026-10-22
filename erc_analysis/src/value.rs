//! A part's Value field read in the unit its class gives it (`10k` on a resistor is 10 kΩ), per variant, with the
//! source of the field.

use erc_model::{Board, Fact, PartId, Provenance, Unit, Value, ValueWarning, VariantId};

/// A Value field read as a quantity
#[derive(Clone, Debug, PartialEq)]
pub struct PartValue {
    /// The field as written; empty when there is no Value field
    pub text: String,
    /// The number in base units, `None` when the text does not read as one
    pub value: Option<f64>,
    pub warnings: Vec<ValueWarning>,
    /// Where the field came from
    pub source: Option<Provenance>,
}

/// The Value field of `part` in `variant` (the variant's override first), with its source
pub fn value_field(
    board: &Board,
    part: PartId,
    variant: Option<VariantId>,
) -> Option<&Fact<String>> {
    let p = board.part(part);
    let in_variant = variant.and_then(|v| {
        p.variants
            .iter()
            .find(|pv| pv.variant == v)
            .and_then(|pv| pv.fields.iter().find(|f| f.name == "Value"))
            .and_then(|f| f.value.resolve())
    });
    in_variant.or_else(|| {
        p.fields
            .iter()
            .find(|f| f.name == "Value")
            .and_then(|f| f.value.resolve())
    })
}

/// The Value field of `part` read in `unit`
pub fn value_as(board: &Board, part: PartId, variant: Option<VariantId>, unit: Unit) -> PartValue {
    let fact = value_field(board, part, variant);
    let text = fact.map_or("", |f| f.value.as_str());
    let (value, warnings) = Value::parse_as(text, unit);
    PartValue {
        text: text.to_string(),
        value: value.quantity().map(|q| q.value),
        warnings,
        source: fact.map(|f| f.source.clone()),
    }
}

/// `4.7 kΩ`, `100 nF`: a value with an SI prefix, for messages
pub fn format_si(value: f64, unit: Unit) -> String {
    let symbol = match unit {
        Unit::Ohm => "Ω",
        Unit::Farad => "F",
        Unit::Henry => "H",
        Unit::Volt => "V",
        Unit::Ampere => "A",
        Unit::Hertz => "Hz",
        Unit::Watt => "W",
    };
    if value == 0.0 {
        return format!("0 {symbol}");
    }
    let prefixes = [
        (1e9, "G"),
        (1e6, "M"),
        (1e3, "k"),
        (1.0, ""),
        (1e-3, "m"),
        (1e-6, "µ"),
        (1e-9, "n"),
        (1e-12, "p"),
    ];
    let (scale, prefix) = prefixes
        .iter()
        .find(|(s, _)| value.abs() >= *s)
        .copied()
        .unwrap_or((1e-12, "p"));
    let n = value / scale;
    let n = (n * 1000.0).round() / 1000.0;
    format!("{n} {prefix}{symbol}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn si_text() {
        assert_eq!(format_si(4700.0, Unit::Ohm), "4.7 kΩ");
        assert_eq!(format_si(10.0, Unit::Ohm), "10 Ω");
        assert_eq!(format_si(0.0, Unit::Ohm), "0 Ω");
        assert_eq!(format_si(1e-7, Unit::Farad), "100 nF");
        assert_eq!(format_si(2.2e6, Unit::Ohm), "2.2 MΩ");
    }
}
