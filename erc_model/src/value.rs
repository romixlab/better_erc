//! Typed part values: the Value field parsed into a quantity with a unit (`4k7` as 4700 Ω once the part is known
//! to be a resistor, `100nF` as 100 nF on its own), or kept as text.
//!
//! A bare number with a multiplier (`10k`, `100n`) has no unit of its own: [`Value::parse`] keeps it as text, and
//! [`Value::parse_as`] reads it in the unit of a part class decided elsewhere (part classification), so the model
//! never guesses a resistor from the letter `k`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Physical unit of a quantity
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Unit {
    Ohm,
    Farad,
    Henry,
    Volt,
    Ampere,
    Hertz,
    Watt,
}

impl Unit {
    /// The name the dump writes
    pub fn name(self) -> &'static str {
        match self {
            Unit::Ohm => "ohm",
            Unit::Farad => "F",
            Unit::Henry => "H",
            Unit::Volt => "V",
            Unit::Ampere => "A",
            Unit::Hertz => "Hz",
            Unit::Watt => "W",
        }
    }

    pub fn from_name(name: &str) -> Option<Unit> {
        Some(match name {
            "ohm" => Unit::Ohm,
            "F" => Unit::Farad,
            "H" => Unit::Henry,
            "V" => Unit::Volt,
            "A" => Unit::Ampere,
            "Hz" => Unit::Hertz,
            "W" => Unit::Watt,
            _ => return None,
        })
    }

    /// The unit a suffix names: `Ω`, `ohm`, `R`, `F`, `H`, `V`, `A`, `Hz`, `W` (any case for the words)
    fn from_suffix(s: &str) -> Option<Unit> {
        Some(match s {
            "Ω" | "R" | "r" | "ohm" | "Ohm" | "OHM" | "ohms" | "Ohms" => Unit::Ohm,
            "F" | "f" => Unit::Farad,
            "H" | "h" => Unit::Henry,
            "V" | "v" => Unit::Volt,
            "A" | "a" => Unit::Ampere,
            "Hz" | "hz" | "HZ" => Unit::Hertz,
            "W" | "w" => Unit::Watt,
            _ => return None,
        })
    }
}

/// A number with a unit, in base SI units (4.7 kΩ is `4700.0` ohm)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.unit.name())
    }
}

/// A part's value: a quantity when it could be read as one, otherwise the text as written
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Quantity(Quantity),
    Text(String),
}

/// Something odd in a value that still parsed (BOM-10 reports these)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueWarning {
    /// Text after the quantity (`100n 50V X7R`): kept out of the number
    Trailing(String),
    /// The unit written differs from the one expected for the part (`10uF` on a resistor)
    UnitMismatch { written: Unit, expected: Unit },
    /// A decimal comma (`4,7k`)
    DecimalComma,
}

impl Value {
    /// Reads `text` as a quantity when it names its unit (`100nF`, `4.7 kΩ`, `2R2`, `3V3`, `12MHz`), otherwise
    /// keeps it as text. The warnings say what was odd about it.
    pub fn parse(text: &str) -> (Value, Vec<ValueWarning>) {
        match parse_quantity(text, None) {
            Some((q, w)) => (Value::Quantity(q), w),
            None => (Value::Text(text.to_string()), Vec::new()),
        }
    }

    /// Reads `text` in `unit` when it has none (`10k` as 10 kΩ for a resistor). A different written unit is kept
    /// and warned about.
    pub fn parse_as(text: &str, unit: Unit) -> (Value, Vec<ValueWarning>) {
        match parse_quantity(text, Some(unit)) {
            Some((q, mut w)) => {
                if q.unit != unit {
                    w.push(ValueWarning::UnitMismatch {
                        written: q.unit,
                        expected: unit,
                    });
                }
                (Value::Quantity(q), w)
            }
            None => (Value::Text(text.to_string()), Vec::new()),
        }
    }

    pub fn quantity(&self) -> Option<Quantity> {
        match self {
            Value::Quantity(q) => Some(*q),
            Value::Text(_) => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Quantity(q) => q.fmt(f),
            Value::Text(t) => f.write_str(t),
        }
    }
}

fn multiplier(c: char) -> Option<f64> {
    Some(match c {
        'p' => 1e-12,
        'n' => 1e-9,
        'u' | 'µ' | 'μ' => 1e-6,
        'm' => 1e-3,
        'k' | 'K' => 1e3,
        'M' => 1e6,
        'G' => 1e9,
        _ => return None,
    })
}

/// The quantity at the start of `text`; `None` when there is no number or no unit (and no default unit)
fn parse_quantity(text: &str, default: Option<Unit>) -> Option<(Quantity, Vec<ValueWarning>)> {
    let mut warnings = Vec::new();
    let text = text.trim();
    // The first word is the value; the rest (`50V X7R`, `1%`) is trailing text
    let (head, rest) = match text.find(|c: char| c.is_whitespace() || c == '/') {
        Some(i) => {
            // `4.7 kΩ`, `10 uF`: a lone number followed by a unit word belongs together
            let (h, r) = text.split_at(i);
            let r = r.trim_start_matches(|c: char| c.is_whitespace());
            let unit_word_len = r
                .find(|c: char| c.is_whitespace() || c == '/')
                .unwrap_or(r.len());
            let word = &r[..unit_word_len];
            if h.chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
                && !word.is_empty()
                && unit_word(word)
            {
                let end = text.len() - r.len() + unit_word_len;
                (
                    format!("{h}{word}"),
                    text[end..]
                        .trim_start_matches([' ', '/'])
                        .trim()
                        .to_string(),
                )
            } else {
                (h.to_string(), r.trim_start_matches('/').trim().to_string())
            }
        }
        None => (text.to_string(), String::new()),
    };
    let mut head = head;
    if head.contains(',') {
        if head.matches(',').count() > 1 {
            return None;
        }
        head = head.replace(',', ".");
        warnings.push(ValueWarning::DecimalComma);
    }
    let chars: Vec<char> = head.chars().collect();
    let digits_end = chars
        .iter()
        .position(|c| !(c.is_ascii_digit() || *c == '.'))
        .unwrap_or(chars.len());
    if digits_end == 0 {
        return None;
    }
    let number: String = chars[..digits_end].iter().collect();
    let tail: String = chars[digits_end..].iter().collect();
    let mut value: f64 = number.parse().ok()?;

    // RKM: a multiplier or unit letter in place of the decimal point (`4k7`, `2R2`, `3V3`, `4n7`)
    let (unit_text, mult) = if let Some((first, after)) = split_first(&tail)
        && !after.is_empty()
        && after.chars().all(|c| c.is_ascii_digit())
        && !number.contains('.')
    {
        let frac: f64 = format!("0.{after}").parse().ok()?;
        if let Some(m) = multiplier(first) {
            value = (value + frac) * m;
            (String::new(), 1.0)
        } else {
            let u = Unit::from_suffix(&first.to_string())?;
            value += frac;
            (unit_name_for(u).to_string(), 1.0)
        }
    } else if let Some((first, after)) = split_first(&tail)
        && let Some(m) = multiplier(first)
        && (after.is_empty() || unit_suffix(&after).is_some())
        // `m` alone is milli, `M` alone mega; `mF`/`MHz` follow the unit
        && !(first == 'm' && after.is_empty() && default.is_none())
    {
        (after, m)
    } else {
        (tail, 1.0)
    };

    let unit = if unit_text.is_empty() {
        default?
    } else {
        unit_suffix(&unit_text)?
    };
    if !rest.is_empty() {
        warnings.push(ValueWarning::Trailing(rest));
    }
    Some((
        Quantity {
            value: round_sig(value * mult),
            unit,
        },
        warnings,
    ))
}

/// A unit with an optional multiplier: `kΩ`, `uF`, `Ω`
fn unit_word(w: &str) -> bool {
    unit_suffix(w).is_some()
        || split_first(w)
            .is_some_and(|(c, rest)| multiplier(c).is_some() && unit_suffix(&rest).is_some())
}

fn split_first(s: &str) -> Option<(char, String)> {
    let mut it = s.chars();
    let c = it.next()?;
    Some((c, it.collect()))
}

fn unit_suffix(s: &str) -> Option<Unit> {
    Unit::from_suffix(s)
}

fn unit_name_for(u: Unit) -> &'static str {
    match u {
        Unit::Ohm => "R",
        Unit::Volt => "V",
        Unit::Ampere => "A",
        Unit::Farad => "F",
        Unit::Henry => "H",
        Unit::Hertz => "Hz",
        Unit::Watt => "W",
    }
}

/// Rounds away the float noise of `4.7 * 1e3` (4700.000000000001) to 12 significant digits
fn round_sig(v: f64) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let text = format!("{v:.11e}");
    text.parse().unwrap_or(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> Option<(f64, Unit)> {
        Value::parse(text).0.quantity().map(|q| (q.value, q.unit))
    }

    #[test]
    fn explicit_units() {
        assert_eq!(q("100nF"), Some((1e-7, Unit::Farad)));
        assert_eq!(q("4.7 kΩ"), Some((4700.0, Unit::Ohm)));
        assert_eq!(q("2R2"), Some((2.2, Unit::Ohm)));
        assert_eq!(q("0R"), Some((0.0, Unit::Ohm)));
        assert_eq!(q("3V3"), Some((3.3, Unit::Volt)));
        assert_eq!(q("12MHz"), Some((12e6, Unit::Hertz)));
        assert_eq!(q("10uH"), Some((1e-5, Unit::Henry)));
        assert_eq!(q("1µF"), Some((1e-6, Unit::Farad)));
        assert_eq!(q("22mF"), Some((0.022, Unit::Farad)));
    }

    #[test]
    fn bare_numbers_are_text_until_the_class_is_known() {
        assert_eq!(q("10k"), None);
        assert_eq!(q("4k7"), None);
        assert_eq!(q("STM32G0"), None);
        assert_eq!(q("~"), None);
        let (v, w) = Value::parse_as("4k7", Unit::Ohm);
        assert_eq!(v.quantity().map(|q| q.value), Some(4700.0));
        assert!(w.is_empty());
        let (v, _) = Value::parse_as("100n", Unit::Farad);
        assert_eq!(v.quantity().map(|q| q.value), Some(1e-7));
        let (v, _) = Value::parse_as("1m", Unit::Ohm);
        assert_eq!(v.quantity().map(|q| q.value), Some(1e-3));
    }

    #[test]
    fn warnings() {
        let (v, w) = Value::parse("100nF 50V X7R");
        assert_eq!(v.quantity().map(|q| q.value), Some(1e-7));
        assert_eq!(w, vec![ValueWarning::Trailing("50V X7R".into())]);
        let (_, w) = Value::parse_as("10uF", Unit::Ohm);
        assert_eq!(
            w,
            vec![ValueWarning::UnitMismatch {
                written: Unit::Farad,
                expected: Unit::Ohm
            }]
        );
        let (v, w) = Value::parse_as("4,7k", Unit::Ohm);
        assert_eq!(v.quantity().map(|q| q.value), Some(4700.0));
        assert_eq!(w, vec![ValueWarning::DecimalComma]);
    }
}
