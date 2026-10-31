//! `NAME-13` Power net name not in the `+<voltage>_<source or group>` form

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::NetId;
use std::collections::BTreeMap;

pub struct Name13;

static META: CheckMeta = CheckMeta {
    id: "NAME-13",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Rails],
    overlaps: &[],
    params: &[],
};

/// `+3V3_LDO`, `-5V_CP`, `+12V_IN`: a sign, the volts, `V` as the decimal point, `_`, a suffix
fn in_form(name: &str) -> bool {
    let base = name.rsplit('/').next().unwrap_or(name);
    let Some(rest) = base.strip_prefix(['+', '-']) else {
        return false;
    };
    let Some((volts, suffix)) = rest.split_once('_') else {
        return false;
    };
    let Some((whole, part)) = volts.split_once('V') else {
        return false;
    };
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    !whole.is_empty() && digits(whole) && digits(part) && !suffix.is_empty()
}

impl Check for Name13 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Rails grouped by voltage, to the millivolt; in a group of two or more each name must carry its source, a
    /// single rail of its voltage may stay plain
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut groups: BTreeMap<i64, Vec<NetId>> = BTreeMap::new();
        for (net, rail) in &power.rails {
            if let Some(v) = rail.volts() {
                groups
                    .entry((v * 1e3).round() as i64)
                    .or_default()
                    .push(*net);
            }
        }
        let mut out = Vec::new();
        for (mv, nets) in &groups {
            if nets.len() < 2 {
                continue;
            }
            for net in nets {
                let name = &b.net(*net).name;
                if in_form(name) {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .net(*net)
                        .message(format!(
                            "{name} is one of {} rails of {} V, without its source in the name",
                            nets.len(),
                            *mv as f64 / 1e3
                        ))
                        .fix("name it +<voltage>_<source or group>, like +3V3_LDO")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Name13));

#[cfg(test)]
mod tests {
    use super::in_form;

    #[test]
    fn name_form() {
        for good in ["+3V3_LDO", "/Power/+1V8_REF", "-5V_CP", "+12V_IN"] {
            assert!(in_form(good), "{good}");
        }
        for bad in ["+3V3", "VCC", "V3V3", "3.3V_A", "VDD_LDO", "+3.3V_A"] {
            assert!(!in_form(bad), "{bad}");
        }
    }
}
