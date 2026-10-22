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

/// A Value field read whole: the value, and what else the field states after it (`100nF 50V X7R`, `4k7 1%`)
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub value: Value,
    /// Tolerance in percent (`1%`, `±5 %`), when the field states one
    pub tolerance: Option<f64>,
    /// Further quantities with their own unit after the value: ratings (`50V`, `0.1W`, `1/4W`)
    pub ratings: Vec<Quantity>,
    /// A ceramic dielectric code (`X7R`, `C0G`), upper case as written
    pub dielectric: Option<String>,
    pub warnings: Vec<ValueWarning>,
}

/// Something odd in a value that still parsed (BOM-10 reports these)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueWarning {
    /// Text after the value that is no tolerance, rating or dielectric (`100n 0603`, `4k7 Ohm metal film`)
    Trailing(String),
    /// The unit written differs from the one expected for the part (`10uF` on a resistor)
    UnitMismatch { written: Unit, expected: Unit },
    /// A decimal comma (`4,7k`)
    DecimalComma,
    /// More than one space between the number and its multiplier or unit (`4.7  k`)
    Spacing,
    /// A small `r` for ohms (`4r7`, `10r`)
    LowercaseR,
    /// `R` after a multiplier that already makes it ohms (`10kR`, `4K7R`)
    RedundantR,
    /// The unit as a separate word after a multiplier or RKM value (`4k7 Ohm`, `100n F`)
    SeparateUnit,
}

impl Value {
    /// Reads `text` as a quantity when it names its unit (`100nF`, `4.7 kΩ`, `2R2`, `3V3`, `12MHz`), otherwise
    /// keeps it as text. The warnings say what was odd about it.
    pub fn parse(text: &str) -> (Value, Vec<ValueWarning>) {
        let r = Value::read(text, None);
        (r.value, r.warnings)
    }

    /// Reads `text` in `unit` when it has none (`10k` as 10 kΩ for a resistor). A different written unit is kept
    /// and warned about.
    pub fn parse_as(text: &str, unit: Unit) -> (Value, Vec<ValueWarning>) {
        let r = Value::read(text, Some(unit));
        (r.value, r.warnings)
    }

    /// Reads a whole Value field: the value (in `unit` when the text names none; the part class decides the
    /// unit, never the letter alone), then the tolerance, ratings and dielectric after it. Text that does not
    /// start with a number stays text, with no warnings.
    pub fn read(text: &str, unit: Option<Unit>) -> Reading {
        let Some((q, warnings, rest)) = parse_head(text, unit) else {
            return Reading {
                value: Value::Text(text.to_string()),
                tolerance: None,
                ratings: Vec::new(),
                dielectric: None,
                warnings: Vec::new(),
            };
        };
        let mut r = Reading {
            value: Value::Quantity(q),
            tolerance: None,
            ratings: Vec::new(),
            dielectric: None,
            warnings,
        };
        read_rest(&rest, q.unit, &mut r);
        if let Some(u) = unit
            && q.unit != u
        {
            r.warnings.push(ValueWarning::UnitMismatch {
                written: q.unit,
                expected: u,
            });
        }
        r
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

const DIELECTRICS: &[&str] = &[
    "C0G", "COG", "NP0", "NPO", "X5R", "X6S", "X6T", "X7R", "X7S", "X7T", "X8L", "X8R", "Y5V",
    "Z5U",
];

/// The words after the value: tolerance, ratings, dielectric; anything else is trailing text
fn read_rest(rest: &str, unit: Unit, r: &mut Reading) {
    let rest = rest.replace(" %", "%");
    let mut unknown: Vec<&str> = Vec::new();
    for token in split_rest(&rest) {
        let tol = token
            .trim_start_matches('±')
            .trim_start_matches("+/-")
            .trim_start_matches("+-");
        if let Some(pct) = tol.strip_suffix('%')
            && let Ok(v) = pct.replace(',', ".").parse::<f64>()
        {
            r.tolerance.get_or_insert(v);
        } else if DIELECTRICS.contains(&token.to_ascii_uppercase().as_str()) {
            r.dielectric.get_or_insert(token.to_ascii_uppercase());
        } else if let Some(q) = fraction(token).or_else(|| {
            parse_head(token, None).and_then(|(q, _, rest)| rest.is_empty().then_some(q))
        }) {
            r.ratings.push(q);
        } else if unit_word(token)
            && unit_suffix(token.trim_start_matches(|c| multiplier(c).is_some())) == Some(unit)
        {
            r.warnings.push(ValueWarning::SeparateUnit);
        } else {
            unknown.push(token);
        }
    }
    if !unknown.is_empty() {
        r.warnings.push(ValueWarning::Trailing(unknown.join(" ")));
    }
}

/// The words of the text after a value: split at spaces, commas, semicolons and at `/` (but not inside a
/// fraction such as `1/4W`)
fn split_rest(rest: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = rest.char_indices().collect();
    for (k, &(i, c)) in chars.iter().enumerate() {
        let fraction_slash = c == '/'
            && k > 0
            && chars[k - 1].1.is_ascii_digit()
            && chars.get(k + 1).is_some_and(|(_, n)| n.is_ascii_digit());
        if c.is_whitespace() || c == ',' || c == ';' || (c == '/' && !fraction_slash) {
            if start < i {
                out.push(&rest[start..i]);
            }
            start = i + c.len_utf8();
        }
    }
    if start < rest.len() {
        out.push(&rest[start..]);
    }
    out
}

/// `1/4W` as 0.25 W
fn fraction(token: &str) -> Option<Quantity> {
    let (num, den) = token.split_once('/')?;
    let num: f64 = num.parse().ok()?;
    let (q, _, rest) = parse_head(den, None)?;
    (rest.is_empty() && q.value != 0.0).then(|| Quantity {
        value: round_sig(num / q.value),
        unit: q.unit,
    })
}

/// The quantity at the start of `text` with its warnings and the text after it; `None` when there is no number
/// or no unit (and no default unit)
fn parse_head(text: &str, default: Option<Unit>) -> Option<(Quantity, Vec<ValueWarning>, String)> {
    let mut warnings = Vec::new();
    let text = text.trim();
    // The first word is the value; the rest (`50V X7R`, `1%`) is read after it
    let (head, rest) = match text.find(|c: char| c.is_whitespace() || c == '/') {
        Some(i) => {
            // `4.7 kΩ`, `10 uF`: a lone number followed by a unit word belongs together, and so does a lone
            // multiplier when the unit is known (`4.7 k` for a resistor)
            let (h, r) = text.split_at(i);
            let gap = r.len() - r.trim_start_matches(|c: char| c.is_whitespace()).len();
            let r = r.trim_start_matches(|c: char| c.is_whitespace());
            let unit_word_len = r
                .find(|c: char| c.is_whitespace() || c == '/')
                .unwrap_or(r.len());
            let word = &r[..unit_word_len];
            if h.chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '-' || c == '+')
                && !word.is_empty()
                && (unit_word(word)
                    || default.is_some()
                        && word.chars().count() == 1
                        && word.chars().all(|c| multiplier(c).is_some()))
            {
                if gap > 1 {
                    warnings.push(ValueWarning::Spacing);
                }
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
    let (negative, head) = match head.strip_prefix(['-', '−']) {
        Some(h) => (true, h.to_string()),
        None => (false, head.strip_prefix('+').unwrap_or(&head).to_string()),
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

    // RKM: a multiplier or unit letter in place of the decimal point (`4k7`, `2R2`, `3V3`, `4n7`), the
    // multiplier's form optionally followed by a unit (`4k7Ω`, `4K7R`)
    let rkm = split_first(&tail).and_then(|(first, after)| {
        let frac_len = after.chars().take_while(|c| c.is_ascii_digit()).count();
        let (frac, unit) = after.split_at(frac_len);
        (frac_len > 0 && !number.contains('.')).then(|| (first, frac.to_string(), unit.to_string()))
    });
    let (unit_text, mult) = if let Some((first, frac, after)) = rkm
        && (after.is_empty() || multiplier(first).is_some() && unit_suffix(&after).is_some())
    {
        let frac: f64 = format!("0.{frac}").parse().ok()?;
        if let Some(m) = multiplier(first) {
            value = (value + frac) * m;
            if after == "R" || after == "r" {
                warnings.push(ValueWarning::RedundantR);
            }
            (after, 1.0)
        } else {
            let u = Unit::from_suffix(&first.to_string())?;
            if first == 'r' {
                warnings.push(ValueWarning::LowercaseR);
            }
            value += frac;
            (unit_name_for(u).to_string(), 1.0)
        }
    } else if let Some((first, after)) = split_first(&tail)
        && let Some(m) = multiplier(first)
        && (after.is_empty() || unit_suffix(&after).is_some())
        // `m` alone is milli, `M` alone mega; `mF`/`MHz` follow the unit
        && !(first == 'm' && after.is_empty() && default.is_none())
    {
        if after == "R" || after == "r" {
            warnings.push(ValueWarning::RedundantR);
        }
        (after, m)
    } else {
        if tail == "r" {
            warnings.push(ValueWarning::LowercaseR);
        }
        (tail, 1.0)
    };

    let unit = if unit_text.is_empty() {
        default?
    } else {
        unit_suffix(&unit_text)?
    };
    let value = if negative { -value } else { value };
    Some((
        Quantity {
            value: round_sig(value * mult),
            unit,
        },
        warnings,
        rest,
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

/// The IEC 60063 preferred number series. E3 to E24 have two significant digits (E24 is no subset of E192),
/// E48 to E192 three; each smaller series is a subset of the next larger one of its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ESeries {
    E3,
    E6,
    E12,
    E24,
    E48,
    E96,
    E192,
}

/// E24, as mantissa times 10
const E24: [u16; 24] = [
    10, 11, 12, 13, 15, 16, 18, 20, 22, 24, 27, 30, 33, 36, 39, 43, 47, 51, 56, 62, 68, 75, 82, 91,
];

/// E192, as mantissa times 100 (920 where the formula gives 919, as the standard lists it)
const E192: [u16; 192] = [
    100, 101, 102, 104, 105, 106, 107, 109, 110, 111, 113, 114, 115, 117, 118, 120, 121, 123, 124,
    126, 127, 129, 130, 132, 133, 135, 137, 138, 140, 142, 143, 145, 147, 149, 150, 152, 154, 156,
    158, 160, 162, 164, 165, 167, 169, 172, 174, 176, 178, 180, 182, 184, 187, 189, 191, 193, 196,
    198, 200, 203, 205, 208, 210, 213, 215, 218, 221, 223, 226, 229, 232, 234, 237, 240, 243, 246,
    249, 252, 255, 258, 261, 264, 267, 271, 274, 277, 280, 284, 287, 291, 294, 298, 301, 305, 309,
    312, 316, 320, 324, 328, 332, 336, 340, 344, 348, 352, 357, 361, 365, 370, 374, 379, 383, 388,
    392, 397, 402, 407, 412, 417, 422, 427, 432, 437, 442, 448, 453, 459, 464, 470, 475, 481, 487,
    493, 499, 505, 511, 517, 523, 530, 536, 542, 549, 556, 562, 569, 576, 583, 590, 597, 604, 612,
    619, 626, 634, 642, 649, 657, 665, 673, 681, 690, 698, 706, 715, 723, 732, 741, 750, 759, 768,
    777, 787, 796, 806, 816, 825, 835, 845, 856, 866, 876, 887, 898, 909, 920, 931, 942, 953, 965,
    976, 988,
];

impl ESeries {
    /// Smallest first
    pub const ALL: [ESeries; 7] = [
        ESeries::E3,
        ESeries::E6,
        ESeries::E12,
        ESeries::E24,
        ESeries::E48,
        ESeries::E96,
        ESeries::E192,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ESeries::E3 => "E3",
            ESeries::E6 => "E6",
            ESeries::E12 => "E12",
            ESeries::E24 => "E24",
            ESeries::E48 => "E48",
            ESeries::E96 => "E96",
            ESeries::E192 => "E192",
        }
    }

    /// The series' mantissas times 100 (`470` for 4.7), ascending
    pub fn mantissas(self) -> Vec<u16> {
        let (table, step, scale): (&[u16], usize, u16) = match self {
            ESeries::E3 => (&E24, 8, 10),
            ESeries::E6 => (&E24, 4, 10),
            ESeries::E12 => (&E24, 2, 10),
            ESeries::E24 => (&E24, 1, 10),
            ESeries::E48 => (&E192, 4, 1),
            ESeries::E96 => (&E192, 2, 1),
            ESeries::E192 => (&E192, 1, 1),
        };
        table.iter().step_by(step).map(|m| m * scale).collect()
    }

    /// Whether `value` (any decade, positive) is a value of the series
    pub fn contains(self, value: f64) -> bool {
        mantissa(value).is_some_and(|m| self.mantissas().contains(&m))
    }

    /// The smallest series that has `value`; `None` for a value in none (`5k`, `4.8k`) and for zero or a
    /// negative value
    pub fn of(value: f64) -> Option<ESeries> {
        ESeries::ALL.into_iter().find(|s| s.contains(value))
    }

    /// The value of the series nearest to `value` (in ratio), in the same unit
    pub fn nearest(self, value: f64) -> Option<f64> {
        if !(value.is_finite() && value > 0.0) {
            return None;
        }
        let decade = 10f64.powf(value.log10().floor());
        let candidates = self
            .mantissas()
            .into_iter()
            .map(|m| m as f64 / 100.0 * decade)
            .chain([
                10.0 * decade,
                decade / 10.0 * (self.mantissas().last().copied()? as f64 / 100.0),
            ]);
        candidates
            .min_by(|a, b| (a / value).ln().abs().total_cmp(&(b / value).ln().abs()))
            .map(round_sig)
    }
}

/// The three significant digits of a positive value (`470` for 4.7 kΩ), when it has no more
fn mantissa(value: f64) -> Option<u16> {
    if !(value.is_finite() && value > 0.0) {
        return None;
    }
    let x = value / 10f64.powf(value.log10().floor()) * 100.0;
    let r = x.round();
    if (x - r).abs() > 1e-6 * x {
        return None;
    }
    Some(if r >= 1000.0 { 100 } else { r as u16 })
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
        let (v, w) = Value::parse("100nF 0603 film");
        assert_eq!(v.quantity().map(|q| q.value), Some(1e-7));
        assert_eq!(w, vec![ValueWarning::Trailing("0603 film".into())]);
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

    #[test]
    fn notation_warnings() {
        let ohm = |t: &str| {
            let (v, w) = Value::parse_as(t, Unit::Ohm);
            (v.quantity().map(|q| q.value), w)
        };
        assert_eq!(ohm("4.7 k"), (Some(4700.0), vec![]));
        assert_eq!(ohm("4.7  k"), (Some(4700.0), vec![ValueWarning::Spacing]));
        assert_eq!(ohm("4r7"), (Some(4.7), vec![ValueWarning::LowercaseR]));
        assert_eq!(ohm("10r"), (Some(10.0), vec![ValueWarning::LowercaseR]));
        assert_eq!(ohm("10kR"), (Some(10000.0), vec![ValueWarning::RedundantR]));
        assert_eq!(ohm("10R"), (Some(10.0), vec![]));
        assert_eq!(ohm("2R2"), (Some(2.2), vec![]));
        assert_eq!(ohm("3K3"), (Some(3300.0), vec![]));
        assert_eq!(ohm("0"), (Some(0.0), vec![]));
        assert_eq!(ohm("10x"), (None, vec![]));
        // Without a known unit a lone multiplier is not joined
        assert_eq!(q("4.7 k"), None);
    }

    /// The old `ecad_file_format::passive_value` table (`parse_resistance_value`), its warnings renamed: a redundant
    /// space is [`ValueWarning::Spacing`], `SmallR` [`ValueWarning::LowercaseR`], `BigRInsteadOfOhmSymbol`
    /// [`ValueWarning::RedundantR`]
    #[test]
    fn old_resistance_table() {
        let ohm = |t: &str| {
            let (v, w) = Value::parse_as(t, Unit::Ohm);
            (v.quantity().map(|q| q.value), w)
        };
        for (text, want) in [
            ("100", 100.0),
            ("5k", 5e3),
            ("5M", 5e6),
            ("5G", 5e9),
            ("1R", 1.0),
            ("15.5", 15.5),
            ("5.53R", 5.53),
            ("1.0k", 1e3),
            ("1k2", 1.2e3),
            ("10 kΩ", 1e4),
            ("5Ω", 5.0),
            ("1mΩ", 1e-3),
            ("1μΩ", 1e-6),
            (" 0 ", 0.0),
        ] {
            assert_eq!(ohm(text), (Some(want), vec![]), "{text:?}");
        }
        assert_eq!(ohm(" 0  R "), (Some(0.0), vec![ValueWarning::Spacing]));
        assert_eq!(ohm("49r"), (Some(49.0), vec![ValueWarning::LowercaseR]));
        assert_eq!(
            ohm(" 499kR "),
            (Some(499e3), vec![ValueWarning::RedundantR])
        );
    }

    #[test]
    fn tolerance_ratings_dielectric() {
        let r = Value::read("100nF 50V X7R", None);
        assert_eq!(r.value.quantity().map(|q| q.value), Some(1e-7));
        assert_eq!(
            r.ratings,
            [Quantity {
                value: 50.0,
                unit: Unit::Volt
            }]
        );
        assert_eq!(r.dielectric.as_deref(), Some("X7R"));
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        let r = Value::read("4k7 1%", Some(Unit::Ohm));
        assert_eq!(r.value.quantity().map(|q| q.value), Some(4700.0));
        assert_eq!(r.tolerance, Some(1.0));
        assert!(r.warnings.is_empty());
        let r = Value::read("10k/±5 %/0.1W", Some(Unit::Ohm));
        assert_eq!(r.tolerance, Some(5.0));
        assert_eq!(
            r.ratings,
            [Quantity {
                value: 0.1,
                unit: Unit::Watt
            }]
        );
        let r = Value::read("0R 1/4W", Some(Unit::Ohm));
        assert_eq!(r.value.quantity().map(|q| q.value), Some(0.0));
        assert_eq!(
            r.ratings,
            [Quantity {
                value: 0.25,
                unit: Unit::Watt
            }]
        );
        let r = Value::read("10uF/25V", Some(Unit::Farad));
        assert_eq!(r.value.quantity().map(|q| q.value), Some(1e-5));
        assert_eq!(r.ratings[0].unit, Unit::Volt);
        let r = Value::read("BSS138", None);
        assert_eq!(r.value, Value::Text("BSS138".into()));
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn units_of_every_class() {
        let q = |t: &str, u: Unit| {
            Value::parse_as(t, u)
                .0
                .quantity()
                .map(|q| (q.value, q.unit))
        };
        assert_eq!(q("10uH", Unit::Henry), Some((1e-5, Unit::Henry)));
        assert_eq!(q("4u7", Unit::Henry), Some((4.7e-6, Unit::Henry)));
        assert_eq!(q("16M", Unit::Hertz), Some((16e6, Unit::Hertz)));
        assert_eq!(q("32.768k", Unit::Hertz), Some((32768.0, Unit::Hertz)));
        assert_eq!(q("12MHz", Unit::Hertz), Some((12e6, Unit::Hertz)));
        assert_eq!(q("500mA", Unit::Ampere), Some((0.5, Unit::Ampere)));
        assert_eq!(q("3V3", Unit::Volt), Some((3.3, Unit::Volt)));
        assert_eq!(q("-5V", Unit::Volt), Some((-5.0, Unit::Volt)));
        assert_eq!(q("250mW", Unit::Watt), Some((0.25, Unit::Watt)));
        assert_eq!(q("2n2", Unit::Farad), Some((2.2e-9, Unit::Farad)));
        assert_eq!(q("600R", Unit::Ohm), Some((600.0, Unit::Ohm)));
    }

    #[test]
    fn mixed_notation() {
        let ohm = |t: &str| {
            let (v, w) = Value::parse_as(t, Unit::Ohm);
            (v.quantity().map(|q| q.value), w)
        };
        assert_eq!(ohm("4K7R"), (Some(4700.0), vec![ValueWarning::RedundantR]));
        assert_eq!(ohm("4k7Ω"), (Some(4700.0), vec![]));
        assert_eq!(
            ohm("4k7 Ohm"),
            (Some(4700.0), vec![ValueWarning::SeparateUnit])
        );
        assert_eq!(
            ohm("4,7 k"),
            (Some(4700.0), vec![ValueWarning::DecimalComma])
        );
    }

    #[test]
    fn e_series() {
        assert_eq!(E192.len(), 192);
        assert!(E192.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(ESeries::E96.mantissas().len(), 96);
        assert_eq!(ESeries::E48.mantissas().len(), 48);
        assert_eq!(
            ESeries::E12.mantissas(),
            [100, 120, 150, 180, 220, 270, 330, 390, 470, 560, 680, 820]
        );
        assert_eq!(ESeries::E3.mantissas(), [100, 220, 470]);
        assert_eq!(ESeries::of(4700.0), Some(ESeries::E3));
        assert_eq!(ESeries::of(6800.0), Some(ESeries::E6));
        assert_eq!(ESeries::of(10.0), Some(ESeries::E3));
        assert_eq!(ESeries::of(5100.0), Some(ESeries::E24));
        assert_eq!(ESeries::of(4990.0), Some(ESeries::E96));
        assert_eq!(ESeries::of(1.01e3), Some(ESeries::E192));
        assert_eq!(ESeries::of(5000.0), None, "5k is in no series");
        assert_eq!(ESeries::of(4800.0), None);
        assert_eq!(ESeries::of(0.0), None);
        assert_eq!(ESeries::of(1e-7), Some(ESeries::E3), "100 nF");
        assert_eq!(ESeries::of(2.2e-12), Some(ESeries::E3));
        assert_eq!(ESeries::E24.nearest(5000.0), Some(5100.0));
        assert_eq!(ESeries::E96.nearest(5000.0), Some(4990.0));
        assert_eq!(ESeries::E12.nearest(9600.0), Some(10000.0));
        assert!(ESeries::E96.contains(97.6));
        assert!(!ESeries::E96.contains(4700.0));
    }
}
