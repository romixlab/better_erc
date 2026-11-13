//! Limits: a plain number, or a linear expression of one supply (`"0.7*VDD"`, `"VDD-0.4"`, `"VREF+0.1"`).

use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

use crate::Env;

/// A limit of a level or supply.
#[derive(Debug, Clone, PartialEq)]
pub enum Limit {
    /// A number in the level's unit.
    Num(f64),
    /// `scale * var + offset`, in volts.
    Expr(Expr),
}

/// `scale * var + offset`.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub scale: f64,
    pub var: String,
    pub offset: f64,
}

impl Limit {
    /// Its value, with the supplies in `env`.
    pub fn eval(&self, env: &Env) -> Result<f64, String> {
        match self {
            Limit::Num(v) => Ok(*v),
            Limit::Expr(e) => env
                .get(&e.var)
                .map(|v| e.scale * v.get() + e.offset)
                .ok_or_else(|| format!("unknown supply {} in {e}", e.var)),
        }
    }

    /// An expression of a supply rather than a number.
    pub fn is_expr(&self) -> bool {
        matches!(self, Limit::Expr(_))
    }

    /// The supply an expression uses.
    pub fn var(&self) -> Option<&str> {
        match self {
            Limit::Num(_) => None,
            Limit::Expr(e) => Some(&e.var),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale != 1.0 {
            write!(f, "{}*", self.scale)?;
        }
        write!(f, "{}", self.var)?;
        if self.offset > 0.0 {
            write!(f, "+{}", self.offset)?;
        } else if self.offset < 0.0 {
            write!(f, "{}", self.offset)?;
        }
        Ok(())
    }
}

impl std::str::FromStr for Expr {
    type Err = String;

    /// `[k[*]]VAR[(+|-)c]`, spaces anywhere: `0.7*VDD`, `0.7 VDD`, `VDD - 0.4`, `-0.5*VEE+1`, `VCC-1.025`.
    fn from_str(text: &str) -> Result<Expr, String> {
        let s: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        let bad = || format!("limit {text:?}: expected a number or k*SUPPLY+c (0.7*VDD, VDD-0.4)");
        let start = s.find(|c: char| c.is_ascii_alphabetic()).ok_or_else(bad)?;
        let (head, rest) = s.split_at(start);
        let head = head.strip_suffix('*').unwrap_or(head);
        let scale = match head {
            "" | "+" => 1.0,
            "-" => -1.0,
            h => h.parse::<f64>().map_err(|_| bad())?,
        };
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let (var, tail) = rest.split_at(end);
        let offset = match tail {
            "" => 0.0,
            t if t.starts_with('+') || t.starts_with('-') => t.parse::<f64>().map_err(|_| bad())?,
            _ => return Err(bad()),
        };
        Ok(Expr {
            scale,
            var: var.to_string(),
            offset,
        })
    }
}

impl<'de> Deserialize<'de> for Limit {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Limit, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Limit;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a number or an expression like \"0.7*VDD\"")
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Limit, E> {
                Ok(Limit::Num(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Limit, E> {
                Ok(Limit::Num(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Limit, E> {
                Ok(Limit::Num(v as f64))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Limit, E> {
                v.parse::<Expr>().map(Limit::Expr).map_err(E::custom)
            }
        }
        d.deserialize_any(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(s: &str) -> Expr {
        s.parse().unwrap()
    }

    #[test]
    fn parses_the_forms_in_use() {
        assert_eq!(
            e("0.7*VDD"),
            Expr {
                scale: 0.7,
                var: "VDD".into(),
                offset: 0.0
            }
        );
        assert_eq!(
            e("0.3 VDD"),
            Expr {
                scale: 0.3,
                var: "VDD".into(),
                offset: 0.0
            }
        );
        assert_eq!(
            e("VDD - 0.4"),
            Expr {
                scale: 1.0,
                var: "VDD".into(),
                offset: -0.4
            }
        );
        assert_eq!(
            e("VCC-1.025"),
            Expr {
                scale: 1.0,
                var: "VCC".into(),
                offset: -1.025
            }
        );
        assert_eq!(
            e("0.5*VDDQ+0.1"),
            Expr {
                scale: 0.5,
                var: "VDDQ".into(),
                offset: 0.1
            }
        );
        assert_eq!(
            e("-VEE"),
            Expr {
                scale: -1.0,
                var: "VEE".into(),
                offset: 0.0
            }
        );
        assert!("0.7*".parse::<Expr>().is_err());
        assert!("VDD*2".parse::<Expr>().is_err());
        assert!("x0.7VDD".parse::<Expr>().is_err());
    }

    #[test]
    fn evaluates_with_the_supplies() {
        let env = Env::from([("VDD".to_string(), crate::Volts::try_new(3.3).unwrap())]);
        let l = Limit::Expr(e("VDD-0.4"));
        assert!((l.eval(&env).unwrap() - 2.9).abs() < 1e-12);
        assert!(Limit::Expr(e("VCC")).eval(&env).is_err());
    }
}
