use anyhow::{Error, Result};
use human_repr::HumanCount;
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::fmt::{Debug, Formatter};

#[derive(Clone, PartialEq)]
pub struct Ohm(pub f32);

impl Debug for Ohm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.human_count("Ω"))
    }
}

#[derive(Debug)]
pub enum PassiveValueParseWarning {
    RedundantSpace,
    SmallR,
    BigRInsteadOfOhmSymbol,
}

#[derive(Parser)]
#[grammar = "grammar/passive_value.pest"]
struct PassiveValueParser;

pub fn parse_resistance_value(value: &str) -> Result<(Ohm, Option<PassiveValueParseWarning>)> {
    let pairs = match PassiveValueParser::parse(Rule::resistance, value) {
        Ok(pairs) => pairs,
        Err(e) => return Err(Error::msg(format!("{e}"))),
    };
    let resistance = pairs.into_iter().next().unwrap();
    let kind = resistance.as_rule();
    let mut pairs = resistance.into_inner();
    match kind {
        Rule::r_not_delimited => {
            let integer = pairs.next().unwrap().as_str();
            let (mul, warning) = parse_prefix_ohm(pairs.next())?;
            let val: f32 = integer.parse()?;
            Ok((Ohm(val * mul), warning))
        }
        Rule::r_letter_delimited => {
            let integer = pairs.next().unwrap().as_str();
            let (mul, warning) =
                parse_r_prefix(pairs.next().unwrap().into_inner().next().unwrap())?;
            let fractional = pairs.next().unwrap().as_str();
            let val = format!("{}.{}", integer, fractional);
            let val: f32 = val.parse()?;
            Ok((Ohm(val * mul), warning))
        }
        Rule::r_dot_delimited => {
            let integer = pairs.next().unwrap().as_str();
            let fractional = pairs.next().unwrap().as_str();
            let (mul, warning) = parse_prefix_ohm(pairs.next())?;
            let val = format!("{}.{}", integer, fractional);
            let val: f32 = val.parse()?;
            Ok((Ohm(val * mul), warning))
        }
        _ => Err(Error::msg("Invalid rule")),
    }
}

fn parse_prefix_ohm(
    prefix_ohm: Option<Pair<Rule>>,
) -> Result<(f32, Option<PassiveValueParseWarning>)> {
    match prefix_ohm {
        Some(p) => {
            let mut warning = None;
            let mut mul = 1.0;
            for p in p.into_inner() {
                match p.as_rule() {
                    Rule::space => {
                        if p.as_str().len() > 1 {
                            warning = Some(PassiveValueParseWarning::RedundantSpace);
                        }
                    }
                    Rule::r_prefix => {
                        let p = p.into_inner().next().unwrap();
                        let (m, w) = parse_r_prefix(p)?;
                        if w.is_some() {
                            warning = w;
                        }
                        mul = m;
                    }
                    Rule::ohm => {
                        if p.as_str() == "R" {
                            warning = Some(PassiveValueParseWarning::BigRInsteadOfOhmSymbol);
                        }
                    }
                    _ => {}
                }
            }
            Ok((mul, warning))
        }
        None => Ok((1.0, None)),
    }
}

fn parse_r_prefix(pair: Pair<Rule>) -> Result<(f32, Option<PassiveValueParseWarning>)> {
    match pair.as_rule() {
        Rule::micro => Ok((0.000001, None)),
        Rule::milli => Ok((0.001, None)),
        Rule::r => {
            if pair.as_str() == "r" {
                Ok((1.0, Some(PassiveValueParseWarning::SmallR)))
            } else {
                Ok((1.0, None))
            }
        }
        Rule::kilo => Ok((1_000.0, None)),
        Rule::mega => Ok((1_000_000.0, None)),
        Rule::giga => Ok((1_000_000_000.0, None)),
        _ => Err(Error::msg("Invalid rule")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistance_values() {
        let (v, w) = parse_resistance_value("100").unwrap();
        assert_eq!(v.0, 100.0);
        assert!(w.is_none());
        // assert_eq!(v.1, "100Ω");

        let (v, w) = parse_resistance_value("5k").unwrap();
        assert_eq!(v.0, 5000.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("5M").unwrap();
        assert_eq!(v.0, 5_000_000.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("5G").unwrap();
        assert_eq!(v.0, 5_000_000_000.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("1R").unwrap();
        assert_eq!(v.0, 1.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("15.5").unwrap();
        assert_eq!(v.0, 15.5);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("5.53R").unwrap();
        assert_eq!(v.0, 5.53);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("1.0k").unwrap();
        assert_eq!(v.0, 1000.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("1k2").unwrap();
        assert_eq!(v.0, 1200.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("10 kΩ").unwrap();
        assert_eq!(v.0, 10_000.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("5Ω").unwrap();
        assert_eq!(v.0, 5.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("1mΩ").unwrap();
        assert_eq!(v.0, 0.001);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value("1μΩ").unwrap();
        assert_eq!(v.0, 0.000001);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value(" 0 ").unwrap();
        assert_eq!(v.0, 0.0);
        assert!(w.is_none());

        let (v, w) = parse_resistance_value(" 0  R ").unwrap();
        assert_eq!(v.0, 0.0);
        assert!(matches!(w, Some(PassiveValueParseWarning::RedundantSpace)));

        let (v, w) = parse_resistance_value("49r").unwrap();
        assert_eq!(v.0, 49.0);
        assert!(matches!(w, Some(PassiveValueParseWarning::SmallR)));

        let (v, w) = parse_resistance_value(" 499kR ").unwrap();
        assert_eq!(v.0, 499_000.0);
        assert!(matches!(
            w,
            Some(PassiveValueParseWarning::BigRInsteadOfOhmSymbol)
        ));
    }
}
