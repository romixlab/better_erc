//! The units positions and lengths are written in, in messages and fixes (CFG-10): millimetres, mils or inches,
//! to match the editor's coordinate display. Every check formats through [`Units`]; nothing else knows the units.
//!
//! Millimetres are written as before: positions without a unit word (`x 12.7 y 25.4`), lengths with one
//! (`0.15 mm`). Mils and inches always carry their word (`x 500 y 1000 mil`, `5.91 mil`), so a number is never
//! read in the wrong unit. Item names (`wire at x 12.7 y 25.4`) and the JSON `Location.at` stay in millimetres:
//! they are identities and machine data, not text for a reader.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Nanometres per mil
const NM_PER_MIL: f64 = 25_400.0;

/// Nanometres per inch
const NM_PER_INCH: f64 = 25_400_000.0;

/// The units of positions and lengths in messages
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Units {
    /// Millimetres, two decimals
    #[default]
    #[serde(rename = "mm")]
    Mm,
    /// Thousandths of an inch, two decimals
    #[serde(rename = "mil")]
    Mil,
    /// Inches, four decimals
    #[serde(rename = "in")]
    Inch,
}

/// A units name that is not `mm`, `mil` or `in`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownUnits(pub String);

impl fmt::Display for UnknownUnits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "units are mm, mil or in, not {:?}", self.0)
    }
}

impl std::error::Error for UnknownUnits {}

impl FromStr for Units {
    type Err = UnknownUnits;

    fn from_str(s: &str) -> Result<Units, UnknownUnits> {
        match s {
            "mm" => Ok(Units::Mm),
            "mil" => Ok(Units::Mil),
            "in" => Ok(Units::Inch),
            _ => Err(UnknownUnits(s.to_owned())),
        }
    }
}

impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl Units {
    /// The name the project file and `--units` take, and the word after a number
    pub fn name(self) -> &'static str {
        match self {
            Units::Mm => "mm",
            Units::Mil => "mil",
            Units::Inch => "in",
        }
    }

    /// Nanometres in these units, rounded as written
    fn value(self, nm: i64) -> f64 {
        match self {
            Units::Mm => erc_analysis::scene::mm(nm),
            Units::Mil => (nm as f64 / NM_PER_MIL * 100.0).round() / 100.0,
            Units::Inch => (nm as f64 / NM_PER_INCH * 10_000.0).round() / 10_000.0,
        }
    }

    /// The word after a coordinate: none for millimetres, as positions were always written
    fn coord_suffix(self) -> &'static str {
        match self {
            Units::Mm => "",
            Units::Mil => " mil",
            Units::Inch => " in",
        }
    }

    /// A length with its unit: `0.15 mm`, `5.91 mil`, `0.0059 in`
    pub fn len(self, nm: i64) -> String {
        format!("{} {}", self.value(nm), self.name())
    }

    /// A length as a bare number, for a second length right after one written with [`Units::len`]
    /// (`1 mm apart, less than 2`)
    pub fn num(self, nm: i64) -> String {
        self.value(nm).to_string()
    }

    /// One coordinate: `12.7` in millimetres, `500 mil`, `0.5 in`
    pub fn coord(self, nm: i64) -> String {
        format!("{}{}", self.value(nm), self.coord_suffix())
    }

    /// A position: `x 12.7 y 25.4` in millimetres, `x 500 y 1000 mil`
    pub fn xy(self, x: i64, y: i64) -> String {
        format!(
            "x {} y {}{}",
            self.value(x),
            self.value(y),
            self.coord_suffix()
        )
    }

    /// A box by its corners: `x 10..20 y 30..40` in millimetres, `x 394..787 y 1181..1575 mil`
    pub fn span(self, x: (i64, i64), y: (i64, i64)) -> String {
        format!(
            "x {}..{} y {}..{}{}",
            self.value(x.0),
            self.value(x.1),
            self.value(y.0),
            self.value(y.1),
            self.coord_suffix()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MM: i64 = 1_000_000;

    #[test]
    fn millimetres_read_as_before() {
        let u = Units::Mm;
        assert_eq!(u.len(150_000), "0.15 mm");
        assert_eq!(u.xy(12_700_000, 25_400_000), "x 12.7 y 25.4");
        assert_eq!(u.coord(126 * MM), "126");
        assert_eq!(u.num(2 * MM), "2");
        assert_eq!(
            u.span((10 * MM, 20 * MM), (30 * MM, 40 * MM)),
            "x 10..20 y 30..40"
        );
    }

    #[test]
    fn mils_carry_their_word() {
        let u = Units::Mil;
        assert_eq!(u.len(150_000), "5.91 mil");
        assert_eq!(u.xy(12_700_000, 25_400_000), "x 500 y 1000 mil");
        assert_eq!(u.coord(2_540_000), "100 mil");
        assert_eq!(u.num(2_540_000), "100");
    }

    #[test]
    fn inches_to_four_decimals() {
        let u = Units::Inch;
        assert_eq!(u.len(150_000), "0.0059 in");
        assert_eq!(u.xy(12_700_000, 25_400_000), "x 0.5 y 1 in");
    }

    #[test]
    fn names_round_trip() {
        for u in [Units::Mm, Units::Mil, Units::Inch] {
            assert_eq!(u.name().parse::<Units>(), Ok(u));
        }
        assert_eq!(
            "inch".parse::<Units>(),
            Err(UnknownUnits("inch".to_owned()))
        );
    }
}
