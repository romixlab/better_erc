//! Ceramic dielectric codes (EIA RS-198) and when one is as good as another.
//!
//! A class 2 code (`X7R`) is a low temperature (`X` −55 °C, `Y` −30 °C, `Z` +10 °C), a high temperature (`4` +65 °C
//! up to `9` +200 °C) and the largest capacitance change over that range (`R` ±15 %, `S` ±22 %). A class 1 code
//! (`C0G`, also written `NP0`) is a temperature coefficient: a figure, a multiplier and a tolerance in ppm/K. An
//! offered dielectric is as good as a required one when it covers at least the same temperatures with at most the
//! same change; any class 1 code beats any class 2, and a class 1 code beats another with a coefficient and
//! tolerance no larger.

/// A dielectric code read
#[derive(Clone, Copy, Debug, PartialEq)]
enum Code {
    /// Class 2: low and high temperature in °C, the largest capacitance change in percent
    Class2 {
        low_c: i32,
        high_c: i32,
        change_pct: f64,
    },
    /// Class 1: the coefficient's size and its tolerance, ppm/K
    Class1 {
        coefficient_ppm: f64,
        tolerance_ppm: f64,
    },
}

/// Reads a code, upper or lower case; `NP0` and `NPO` are `C0G`, `COG` is `C0G`
fn read(code: &str) -> Option<Code> {
    let c = code.trim().to_ascii_uppercase();
    let c = match c.as_str() {
        "NP0" | "NPO" | "COG" => "C0G".to_owned(),
        _ => c,
    };
    let b = c.as_bytes();
    let [a, d, e] = *b else {
        return None;
    };
    let low_c = match a {
        b'X' => Some(-55),
        b'Y' => Some(-30),
        b'Z' => Some(10),
        _ => None,
    };
    if let Some(low_c) = low_c {
        let high_c = match d {
            b'2' => 45,
            b'4' => 65,
            b'5' => 85,
            b'6' => 105,
            b'7' => 125,
            b'8' => 150,
            b'9' => 200,
            _ => return None,
        };
        let change_pct = match e {
            b'A' => 1.0,
            b'B' => 1.5,
            b'C' => 2.2,
            b'D' => 3.3,
            b'E' => 4.7,
            b'F' => 7.5,
            b'P' => 10.0,
            b'R' | b'L' => 15.0,
            b'S' => 22.0,
            b'T' => 33.0,
            b'U' => 56.0,
            b'V' => 82.0,
            _ => return None,
        };
        // `L` is +15/−40 % above 125 °C: as wide as `R` below it, wider above; ranked with `S`
        let change_pct = if e == b'L' { 40.0 } else { change_pct };
        return Some(Code::Class2 {
            low_c,
            high_c,
            change_pct,
        });
    }
    let figure = match a {
        b'C' => 0.0,
        b'B' => 0.3,
        b'L' => 0.8,
        b'A' => 0.9,
        b'M' => 1.0,
        b'P' => 1.5,
        b'R' => 2.2,
        b'S' => 3.3,
        b'T' => 4.7,
        b'V' => 5.6,
        b'U' => 7.5,
        _ => return None,
    };
    let multiplier = match d {
        b'0' => 1.0,
        b'1' => 10.0,
        b'2' => 100.0,
        b'3' => 1000.0,
        b'4' => 10_000.0,
        b'5' => 1.0,
        b'6' => 10.0,
        b'7' => 100.0,
        b'8' => 1000.0,
        b'9' => 10_000.0,
        _ => return None,
    };
    let tolerance_ppm = match e {
        b'G' => 30.0,
        b'H' => 60.0,
        b'J' => 120.0,
        b'K' => 250.0,
        b'L' => 500.0,
        b'M' => 1000.0,
        b'N' => 2500.0,
        _ => return None,
    };
    Some(Code::Class1 {
        coefficient_ppm: figure * multiplier,
        tolerance_ppm,
    })
}

/// Whether `offered` is the same as `required` or better: `None` when either is not a code this reads
pub(crate) fn covers(required: &str, offered: &str) -> Option<bool> {
    Some(match (read(required)?, read(offered)?) {
        (Code::Class2 { .. }, Code::Class1 { .. }) => true,
        (Code::Class1 { .. }, Code::Class2 { .. }) => false,
        (
            Code::Class2 {
                low_c: rl,
                high_c: rh,
                change_pct: rc,
            },
            Code::Class2 {
                low_c: ol,
                high_c: oh,
                change_pct: oc,
            },
        ) => ol <= rl && oh >= rh && oc <= rc,
        (
            Code::Class1 {
                coefficient_ppm: rc,
                tolerance_ppm: rt,
            },
            Code::Class1 {
                coefficient_ppm: oc,
                tolerance_ppm: ot,
            },
        ) => oc <= rc && ot <= rt,
    })
}

#[cfg(test)]
mod tests {
    use super::covers;

    #[test]
    fn same_or_better() {
        for (req, off) in [
            ("X7R", "X7R"),
            ("X5R", "X7R"),
            ("X7R", "X8R"),
            ("X7R", "C0G"),
            ("C0G", "NP0"),
            ("x7r", "X7R"),
            ("Y5V", "X5R"),
            ("U2J", "C0G"),
        ] {
            assert_eq!(covers(req, off), Some(true), "{off} for {req}");
        }
        for (req, off) in [
            ("X7R", "X5R"),
            ("X7R", "X7S"),
            ("X5R", "X6S"),
            ("X5R", "Y5V"),
            ("C0G", "X7R"),
            ("C0G", "U2J"),
            ("X8R", "X8L"),
        ] {
            assert_eq!(covers(req, off), Some(false), "{off} for {req}");
        }
        assert_eq!(covers("X7R", "ceramic"), None);
        assert_eq!(covers("", "X7R"), None);
    }
}
