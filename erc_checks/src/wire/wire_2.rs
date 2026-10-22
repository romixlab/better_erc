//! `WIRE-2` Net names that differ only in case, separator or suffix

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::names::{base, normalised};
use erc_model::{NetId, NetKind};
use std::collections::{BTreeMap, BTreeSet};

pub struct Wire2;

static META: CheckMeta = CheckMeta {
    id: "WIRE-2",
    category: "WIRE",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

impl Check for Wire2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Named and power nets grouped by their normalised name (case, `_` `.` `-` separators, a leading `+`, the
    /// active-low marker style): a group with two different names is a finding. The same name on two sheets
    /// (`/A/SDA`, `/B/SDA`, local labels kept apart on purpose) is not.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut groups: BTreeMap<String, Vec<NetId>> = BTreeMap::new();
        for (i, n) in b.nets.iter().enumerate() {
            if matches!(n.kind, NetKind::Named | NetKind::Power) {
                groups
                    .entry(normalised(&n.name))
                    .or_default()
                    .push(NetId::from_index(i));
            }
        }
        groups
            .into_values()
            .filter_map(|nets| {
                let names: BTreeSet<&str> = nets.iter().map(|n| base(&b.net(*n).name)).collect();
                if names.len() < 2 {
                    return None;
                }
                let shown: Vec<&str> = nets.iter().map(|n| b.net(*n).name.as_str()).collect();
                Some(
                    cx.finding(&META)
                        .nets(nets.iter().copied())
                        .message(format!(
                            "{} are separate nets whose names differ only in case, separators or markers: one net meant?",
                            shown.join(", ")
                        ))
                        .fix("pick one name and rename the others; for power, one set of power symbols across the project")
                        .build(),
                )
            })
            .collect()
    }
}

inventory::submit!(Registered(&Wire2));

#[cfg(test)]
mod tests {
    #[test]
    fn similar_names() {
        assert_eq!(
            crate::testing::fingerprints(&super::Wire2, "wire-2_similar_names"),
            [
                "WIRE-2:net +3V3|net 3V3",
                "WIRE-2:net /RESET_N|net /RESET_n",
            ]
        );
    }
}
