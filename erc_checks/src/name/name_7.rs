//! `NAME-7` No nets found in strict mode: suggest `+xVy` power names and non-strict mode

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::{Power, PowerOptions};
use erc_model::{BoardId, Provenance, Subject};

pub struct Name7;

static META: CheckMeta = CheckMeta {
    id: "NAME-7",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Name7 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Rails exist, and the strict reading (`+xVy` only, `PowerOptions::strict`) gives none of them a voltage
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let strict = Power::with(
            cx.view(),
            cx.classes(),
            cx.values(),
            PowerOptions { strict: true },
            &[],
        );
        if strict.rails.is_empty() || strict.voltage_coverage().0 > 0 {
            return Vec::new();
        }
        let names: Vec<&str> = strict
            .rails
            .keys()
            .map(|n| b.net(*n).name.as_str())
            .collect();
        let (loose, of) = cx.power().voltage_coverage();
        vec![
            cx.finding(&META)
                .subject(Subject::Board(BoardId(cx.board_id.0)))
                .message(format!(
                    "no rail name reads as `+xVy` ({}): strict mode finds no voltage; the full grammar reads {loose} of {of}",
                    names.join(", ")
                ))
                .fact(
                    "power names",
                    "strict: `+`, volts, `V`, decimals (`+3V3`)",
                    Provenance::Derived {
                        rule: "erc_analysis::power::read_name, strict".into(),
                    },
                )
                .fix("name rails `+3V3`, `+5V0`, or run in non-strict mode and check what it picked up")
                .build(),
        ]
    }
}

inventory::submit!(Registered(&Name7));

#[cfg(test)]
mod tests {
    #[test]
    fn no_strict_names() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name7, "name-7_no_strict_power_names"),
            ["NAME-7:board name-7_no_strict_power_names"]
        );
    }
}
