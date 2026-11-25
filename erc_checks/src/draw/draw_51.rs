//! `DRAW-51` Block name that contradicts its parts

use super::block::{BLOCK_FIELD, blocks, finding, same_name};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::PartId;
use erc_model::class::{PartClass, RegulatorKind};
use std::collections::BTreeMap;

pub struct Draw51;

static META: CheckMeta = CheckMeta {
    id: "DRAW-51",
    category: "DRAW",
    severity: Severity::Info,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::PartClasses],
    overlaps: &[],
    params: &[],
};

/// Words of a block name that say it is a switching converter
const SWITCHING: [&str; 7] = [
    "dc-dc",
    "dcdc",
    "buck",
    "boost",
    "converter",
    "smps",
    "switcher",
];

/// Words of a block name that say it is a linear regulator
const LINEAR: [&str; 2] = ["ldo", "linear"];

/// Whether a name has one of the words, whole and case ignored; `DC-DC` stays one word
fn says(name: &str, words: &[&str]) -> bool {
    name.split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .any(|w| words.iter().any(|x| w.eq_ignore_ascii_case(x)))
}

/// What is wrong with a block of these classes, if anything
fn contradiction(name: &str, classes: &[PartClass]) -> Option<&'static str> {
    let inductor = classes.contains(&PartClass::Inductor);
    let ldo = classes.contains(&PartClass::Regulator {
        kind: Some(RegulatorKind::Ldo),
    });
    if says(name, &SWITCHING) && ldo && !inductor {
        Some("says converter, but holds a linear regulator and no inductor")
    } else if says(name, &LINEAR) && inductor {
        Some("says linear regulator, but holds an inductor")
    } else {
        None
    }
}

impl Check for Draw51 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The parts of each block on the whole board, by the classes they have; reported at the block's heading
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let view = cx.view();
        let classes = cx.classes();
        let mut by_block: BTreeMap<String, (String, Vec<PartId>)> = BTreeMap::new();
        for p in cx.components() {
            let Some(name) = view.field(p, BLOCK_FIELD).map(str::trim) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let entry = by_block
                .entry(name.to_lowercase())
                .or_insert_with(|| (name.to_string(), Vec::new()));
            entry.1.push(p);
        }
        let mut out = Vec::new();
        for (name, parts) in by_block.into_values() {
            let kinds: Vec<PartClass> = parts.iter().map(|p| classes.get(*p)).collect();
            let Some(why) = contradiction(&name, &kinds) else {
                continue;
            };
            let message = format!(
                "block \"{name}\" {why} ({})",
                crate::text::designators(cx, parts.iter().copied())
            );
            let heading = cx.scenes().iter().find_map(|scene| {
                let all = blocks(scene);
                let b = all
                    .iter()
                    .find(|b| same_name(&b.name, &name) && b.heading.is_some())?;
                finding(cx, &META, scene, b)
            });
            let f = match heading {
                Some(f) => f.parts(parts.iter().copied()),
                None => cx.finding(&META).parts(parts.iter().copied()),
            };
            out.push(
                f.message(message)
                    .fix("name the block after what it holds, or move the part to its own block")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Draw51));

#[cfg(test)]
mod tests {
    use super::contradiction;
    use erc_model::class::{PartClass, RegulatorKind};

    #[test]
    fn names_against_parts() {
        let ldo = PartClass::Regulator {
            kind: Some(RegulatorKind::Ldo),
        };
        let buck = PartClass::Regulator {
            kind: Some(RegulatorKind::Buck),
        };
        let l = PartClass::Inductor;
        let c = PartClass::Capacitor { polarized: false };
        assert!(contradiction("3V8 DC-DC 500mA", &[ldo, c]).is_some());
        assert!(contradiction("5V buck", &[buck, l, c]).is_none());
        assert!(contradiction("1V8 LDO", &[ldo, l]).is_some());
        assert!(contradiction("3V3 LDO 500mA", &[ldo, c]).is_none());
        assert!(contradiction("Boost converter", &[ldo, l]).is_none());
        assert!(contradiction("USB Hub - Power", &[ldo]).is_none());
    }
}
