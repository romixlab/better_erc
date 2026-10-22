//! `LIB-3` MOSFET symbol pins not named G/D/S

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::PartClass;

pub struct Lib3;

static META: CheckMeta = CheckMeta {
    id: "LIB-3",
    category: "LIB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

/// The pin names that say gate, source and drain
const NAMES: [&[&str]; 3] = [&["G", "GATE"], &["S", "SOURCE"], &["D", "DRAIN"]];

impl Check for Lib3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        cx.components()
            .filter(|p| classes.is(*p, PartClass::Mosfet))
            .filter_map(|p| {
                let part = b.part(p);
                let names: Vec<&str> = part.pins.iter().map(|q| b.pin(*q).name.as_str()).collect();
                let named = |n: &[&str]| names.iter().any(|x| n.iter().any(|m| x.eq_ignore_ascii_case(m)));
                if NAMES.iter().all(|n| named(n)) {
                    return None;
                }
                let shown: Vec<String> = part
                    .pins
                    .iter()
                    .map(|q| {
                        let pin = b.pin(*q);
                        if pin.name.is_empty() {
                            format!("{} unnamed", pin.number)
                        } else {
                            format!("{} {}", pin.number, pin.name)
                        }
                    })
                    .collect();
                let class = classes.fact(p);
                Some(
                    cx.finding(&META)
                        .part(p)
                        .message(format!(
                            "MOSFET {} ({}): pins not named G, D and S ({})",
                            part.designator,
                            part.lib_id,
                            shown.join(", ")
                        ))
                        .fact("class", class.value.name(), class.source.clone())
                        .fix("use a symbol with named pins in the package's order, or name them in the library")
                        .build(),
                )
            })
            .collect()
    }
}

inventory::submit!(Registered(&Lib3));

#[cfg(test)]
mod tests {
    #[test]
    fn unnamed_pins() {
        // Q2's pins are G, S, D
        assert_eq!(
            crate::testing::fingerprints(&super::Lib3, "lib-3_mosfet_pin_names"),
            ["LIB-3:part Q1"]
        );
    }
}
