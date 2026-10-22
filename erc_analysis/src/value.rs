//! Part values: each part's Value field read in the unit its class gives it (`10k` on a resistor is 10 kΩ, `16M`
//! on a crystal 16 MHz), as the variant checked mounts it, with the tolerance, ratings and dielectric the field
//! states, the E-series it is in, and the source of the field. A part whose class has no value unit (an IC, a
//! connector) still reads a value that names its unit (`3V3`), else keeps its text.
//!
//! The value is the part's (`erc_model::Part::value`, which the importer fills by the tool's convention: KiCad's
//! `Value`, Altium's `Value`, else `Comment`), read from the text of the field it came from; never a tool's field by
//! its name.

use crate::class::{PartClass, PartClasses};
use erc_model::{
    Board, ESeries, Fact, PartId, Provenance, Quantity, Unit, Value, ValueWarning, VariantId, View,
};

/// A Value field read as a quantity
#[derive(Clone, Debug, PartialEq)]
pub struct PartValue {
    /// The field as written; empty when there is no Value field
    pub text: String,
    /// The number in base units, `None` when the text does not read as one
    pub value: Option<f64>,
    /// The unit read: the one written, else the part class's
    pub unit: Option<Unit>,
    /// The unit the part class expects (`None` for a class whose Value names the part)
    pub expected: Option<Unit>,
    /// Tolerance in percent, when the field states one
    pub tolerance: Option<f64>,
    /// Ratings stated after the value (`50V`, `0.1W`)
    pub ratings: Vec<Quantity>,
    /// A ceramic dielectric code (`X7R`)
    pub dielectric: Option<String>,
    /// Whether the part's value should be in an E-series: a resistor, capacitor or inductor (not a ferrite,
    /// whose impedance is no series value)
    pub series_expected: bool,
    /// The smallest E-series the value is in (when expected, for a positive value)
    pub series: Option<ESeries>,
    pub warnings: Vec<ValueWarning>,
    /// Where the field came from
    pub source: Option<Provenance>,
    /// The field the value is written in (`Value`, `Comment`), when the part has one
    pub field: Option<String>,
}

impl PartValue {
    /// What a finding calls the value: `Value`, or `Value (from Comment)` when another field holds it
    pub fn label(&self) -> String {
        label(self.field.as_deref().unwrap_or("Value"))
    }

    /// The value as a quantity
    pub fn quantity(&self) -> Option<Quantity> {
        Some(Quantity {
            value: self.value?,
            unit: self.unit?,
        })
    }

    /// Whether the E-series question applies: a resistance, capacitance or inductance above zero on a part
    /// whose value should be in a series
    pub fn series_applies(&self) -> bool {
        self.series_expected
            && matches!(self.unit, Some(Unit::Ohm | Unit::Farad | Unit::Henry))
            && self.expected == self.unit
            && self.value.is_some_and(|v| v > 0.0)
    }
}

/// The Value field of every part as one variant mounts it, by [`PartId`] (Need::Values)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Values(pub Vec<PartValue>);

impl Values {
    /// Reads every part's Value field in `view`, in the unit of its class in `classes`
    pub fn of(view: View, classes: &PartClasses) -> Values {
        Values(
            (0..view.board.parts.len())
                .map(PartId::from_index)
                .map(|p| {
                    let class = classes.get(p);
                    let mut v = read(view, p, class.value_unit());
                    if !(class.is_resistor()
                        || class.is_capacitor()
                        || class == PartClass::Inductor)
                    {
                        v.series_expected = false;
                        v.series = None;
                    }
                    v
                })
                .collect(),
        )
    }

    pub fn get(&self, part: PartId) -> &PartValue {
        &self.0[part.index()]
    }

    /// How many components have a Value field that reads in their class's unit, of those whose class has one
    pub fn coverage(&self, board: &Board) -> (usize, usize) {
        let mut have = 0;
        let mut of = 0;
        for i in 0..board.parts.len() {
            let p = PartId::from_index(i);
            let v = self.get(p);
            if !board.is_component(p) || v.expected.is_none() {
                continue;
            }
            of += 1;
            if v.value.is_some() {
                have += 1;
            }
        }
        (have, of)
    }
}

/// The text a part's value was read from
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueText<'b> {
    /// The field (`Value`, `Comment`)
    pub field: &'b str,
    pub fact: &'b Fact<String>,
}

impl ValueText<'_> {
    /// What a finding calls the value: `Value`, or `Value (from Comment)` when another field holds it
    pub fn label(&self) -> String {
        label(self.field)
    }
}

fn label(field: &str) -> String {
    if field == "Value" {
        "Value".into()
    } else {
        format!("Value (from {field})")
    }
}

/// The text of `part`'s value in `view`: the variant's override of the Value field first (KiCad variants), then the
/// field the part's value was read from: the first field of the value's source whose text reads as that value.
/// `None` when the part has no value.
pub fn value_text<'b>(view: View<'b>, part: PartId) -> Option<ValueText<'b>> {
    let p = view.board.part(part);
    let in_variant = view
        .variant
        .and_then(|v| p.variants.iter().find(|pv| pv.variant == v))
        .and_then(|pv| pv.fields.iter().find(|f| f.name == "Value"))
        .and_then(|f| Some((f.name.as_str(), f.value.resolve()?)));
    if let Some((field, fact)) = in_variant {
        return Some(ValueText { field, fact });
    }
    let value = p.value.resolve()?;
    p.fields.iter().find_map(|f| {
        let fact = f.value.resolve()?;
        (fact.source == value.source && Value::parse(&fact.value).0 == value.value).then_some(ValueText {
            field: f.name.as_str(),
            fact,
        })
    })
}

/// The value of `part` read in `unit` when it names none
pub fn value_as(board: &Board, part: PartId, variant: Option<VariantId>, unit: Unit) -> PartValue {
    read(board.view(variant), part, Some(unit))
}

/// The value of `part` in `view` ([`value_text`]), read in `unit` when it names none (`None`: only a written unit
/// counts)
pub fn read(view: View, part: PartId, unit: Option<Unit>) -> PartValue {
    let t = value_text(view, part);
    let fact = t.map(|t| t.fact);
    let text = fact.map_or("", |f| f.value.as_str());
    let r = Value::read(text, unit);
    let q = r.value.quantity();
    let mut v = PartValue {
        text: text.to_string(),
        value: q.map(|q| q.value),
        unit: q.map(|q| q.unit),
        expected: unit,
        tolerance: r.tolerance,
        ratings: r.ratings,
        dielectric: r.dielectric,
        series_expected: matches!(unit, Some(Unit::Ohm | Unit::Farad | Unit::Henry)),
        series: None,
        warnings: r.warnings,
        source: fact.map(|f| f.source.clone()),
        field: t.map(|t| t.field.to_string()),
    };
    if v.series_applies() {
        v.series = v.value.and_then(ESeries::of);
    }
    v
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
    use erc_model::{Facts, Field, Part, PartFlags, PartVariant, SheetId, Variant};

    #[test]
    fn si_text() {
        assert_eq!(format_si(4700.0, Unit::Ohm), "4.7 kΩ");
        assert_eq!(format_si(10.0, Unit::Ohm), "10 Ω");
        assert_eq!(format_si(0.0, Unit::Ohm), "0 Ω");
        assert_eq!(format_si(1e-7, Unit::Farad), "100 nF");
        assert_eq!(format_si(2.2e6, Unit::Ohm), "2.2 MΩ");
        assert_eq!(format_si(-5.0, Unit::Volt), "-5 V");
    }

    fn part(designator: &str, lib: &str, value: &str) -> Part {
        let sch = Provenance::Schematic {
            file: "a.kicad_sch".into(),
        };
        Part {
            designator: designator.into(),
            symbol: None,
            lib_id: lib.into(),
            sheet: SheetId(0),
            units: Vec::new(),
            fields: vec![Field::new("Value", Fact::new(value.into(), sch.clone()))],
            value: Facts::one(Fact::new(Value::parse(value).0, sch)),
            flags: PartFlags::default(),
            variants: Vec::new(),
            pins: Vec::new(),
            ties: Vec::new(),
        }
    }

    #[test]
    fn unit_from_the_class() {
        let mut r1 = part("R1", "Device:R", "5k 1%");
        r1.variants.push(PartVariant {
            variant: VariantId(0),
            flags: PartFlags::default(),
            fields: vec![Field::new(
                "Value",
                Fact::new("5k1".into(), Provenance::Manual { who: "t".into() }),
            )],
        });
        let board = Board {
            parts: vec![
                r1,
                part("C1", "Device:C", "100n 50V X7R"),
                part("Y1", "Device:Crystal", "16M"),
                part("U1", "MCU_ST_STM32G0:STM32G071KBTx", "STM32G071KBT6"),
                part("X1", "Fixture:Thing", "10k"),
            ],
            variants: vec![Variant {
                name: "B".into(),
                description: None,
            }],
            ..Board::default()
        };
        let classes = PartClasses::of(&board);
        let base = Values::of(board.view(None), &classes);
        let r1 = base.get(PartId(0));
        assert_eq!(r1.value, Some(5000.0));
        assert_eq!(r1.tolerance, Some(1.0));
        assert_eq!(r1.series, None, "5k is in no series");
        let c1 = base.get(PartId(1));
        assert_eq!((c1.value, c1.unit), (Some(1e-7), Some(Unit::Farad)));
        assert_eq!(c1.series, Some(ESeries::E3));
        assert_eq!(c1.dielectric.as_deref(), Some("X7R"));
        assert_eq!(base.get(PartId(2)).value, Some(16e6));
        assert_eq!(base.get(PartId(3)).value, None);
        assert_eq!(
            base.get(PartId(4)).value,
            None,
            "an X of unknown class: `10k` is no quantity"
        );
        let b = Values::of(board.view(Some(VariantId(0))), &classes);
        assert_eq!(b.get(PartId(0)).value, Some(5100.0));
        assert_eq!(b.get(PartId(0)).series, Some(ESeries::E24));
        assert_eq!(
            b.get(PartId(0)).source,
            Some(Provenance::Manual { who: "t".into() })
        );
        assert_eq!(base.coverage(&board), (3, 3));
    }
}
