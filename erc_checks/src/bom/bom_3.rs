//! `BOM-3` Big and small connectors, big and small parts, SMT and THT mixed and similar

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::package::Package;
use erc_model::class::ConnectorKind;
use erc_model::layout::Side;
use erc_model::{PartClass, PartId, natural_cmp};
use std::collections::BTreeMap;

pub struct Bom3;

static META: CheckMeta = CheckMeta {
    id: "BOM-3",
    category: "BOM",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Variant],
    overlaps: &[],
    params: &[Param {
        name: "minority",
        default: 0.2,
        unit: "ratio",
        doc: "a group of parts at most this share of its kind (mount type, case size, connector family, side) is a minority",
    }],
};

/// A connector whose family is a choice (headers, wire-to-board, terminal blocks), not set by its function (USB,
/// HDMI, RJ45, card sockets, coax)
fn chosen_family(class: PartClass) -> bool {
    matches!(
        class,
        PartClass::Connector {
            kind: None | Some(ConnectorKind::Header | ConnectorKind::TerminalBlock)
        }
    )
}

/// The family of a connector footprint: the name up to the first token with a digit, with the pitch
/// (`JST_PH_B4B-PH-K_1x04_P2.00mm_Vertical` is `JST_PH P2.00mm`, `PinHeader_1x04_P2.54mm_Vertical` is
/// `PinHeader P2.54mm`)
fn family(pkg: &Package) -> String {
    let tokens: Vec<&str> = pkg.name.split('_').collect();
    let head: Vec<&str> = tokens
        .iter()
        .take_while(|t| !t.chars().any(|c| c.is_ascii_digit()))
        .copied()
        .collect();
    let mut f = if head.is_empty() {
        pkg.name.clone()
    } else {
        head.join("_")
    };
    if let Some(pitch) = tokens.iter().find(|t| {
        t.starts_with('P') && t.ends_with("mm") && t[1..2].chars().all(|c| c.is_ascii_digit())
    }) {
        f = format!("{f} {pitch}");
    }
    f
}

impl Check for Bom3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Over the fitted parts: the minority mount type (THT among SMD), each minority chip case size, each
    /// minority connector family among the connectors whose family is a choice, and the minority board side
    /// when the board file was read
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let share = cx.param(&META, "minority");
        let parts: Vec<(PartId, Package)> = cx
            .view()
            .fitted_components()
            .filter(|p| !classes.get(*p).is_mechanical())
            .filter_map(|p| Some((p, Package::of(b, p)?)))
            .collect();
        // The minority groups: their parts, what they are, the fix
        let mut found: Vec<(Vec<PartId>, String, &'static str)> = Vec::new();
        let mut report = |groups: BTreeMap<String, Vec<PartId>>, what: &str, fix: &'static str| {
            let total: usize = groups.values().map(Vec::len).sum();
            if groups.len() < 2 {
                return;
            }
            let (major, major_n) = groups
                .iter()
                .max_by(|a, c| {
                    a.1.len()
                        .cmp(&c.1.len())
                        .then_with(|| natural_cmp(c.0, a.0))
                })
                .map(|(k, v)| (k.clone(), v.len()))
                .expect("two groups");
            for (key, mut ps) in groups {
                if key == major || ps.len() as f64 > share * total as f64 {
                    continue;
                }
                ps.sort_by(|a, c| natural_cmp(&b.part(*a).designator, &b.part(*c).designator));
                let names: Vec<&str> = ps.iter().map(|p| b.part(*p).designator.as_str()).collect();
                let message = format!(
                    "{} {what} {key} ({}) among {major_n} {major}",
                    ps.len(),
                    names.join(", ")
                );
                found.push((ps, message, fix));
            }
        };
        let mut mounts: BTreeMap<String, Vec<PartId>> = BTreeMap::new();
        let mut sizes: BTreeMap<String, Vec<PartId>> = BTreeMap::new();
        let mut families: BTreeMap<String, Vec<PartId>> = BTreeMap::new();
        let mut sides: BTreeMap<String, Vec<PartId>> = BTreeMap::new();
        for (p, pkg) in &parts {
            if let Some(m) = pkg.mount {
                mounts.entry(m.name().into()).or_default().push(*p);
            }
            if let Some(s) = pkg.size {
                sizes.entry(s.into()).or_default().push(*p);
            }
            if chosen_family(classes.get(*p)) {
                families.entry(family(pkg)).or_default().push(*p);
            }
            if let Some(side) = pkg.side {
                let s = match side {
                    Side::Front => "front",
                    Side::Back => "back",
                };
                sides.entry(s.into()).or_default().push(*p);
            }
        }
        report(
            mounts,
            "parts",
            "pick an SMD (or through-hole) twin where one exists; if the mix stays, plan the extra soldering step",
        );
        report(
            sizes,
            "parts in case",
            "move them to the common case size unless a power, voltage or DC-bias rating needs it",
        );
        report(
            families,
            "connectors of the family",
            "use the board's common connector family where the mating side allows",
        );
        report(
            sides,
            "parts on the",
            "move them to the other side where layout allows, or plan the second reflow pass",
        );
        // One finding per set of parts: a THT header that is also the odd connector family is one finding
        let mut merged: Vec<(Vec<PartId>, Vec<String>, Vec<&str>)> = Vec::new();
        for (ps, message, fix) in found {
            match merged.iter_mut().find(|m| m.0 == ps) {
                Some(m) => {
                    m.1.push(message);
                    m.2.push(fix);
                }
                None => merged.push((ps, vec![message], vec![fix])),
            }
        }
        merged
            .into_iter()
            .map(|(ps, messages, fixes)| {
                cx.finding(&META)
                    .parts(ps)
                    .message(messages.join("; "))
                    .fact("minority", share, cx.param_source(&META, "minority"))
                    .fix(fixes.join("; "))
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&Bom3));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families() {
        let f = |s: &str| family(&Package::from_footprint(s).unwrap());
        assert_eq!(
            f("Connector_JST:JST_PH_B4B-PH-K_1x04_P2.00mm_Vertical"),
            "JST_PH P2.00mm"
        );
        assert_eq!(
            f("Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical"),
            "PinHeader P2.54mm"
        );
    }

    #[test]
    fn minorities() {
        // 9 SMD parts and J4 THT; 6 × 0402 and R7 in 0201; J1..J3 JST PH, J4 a pin header; J5 USB-C (no choice)
        assert_eq!(
            crate::testing::fingerprints(&Bom3, "bom-3_assembly_mix"),
            ["BOM-3:part J4", "BOM-3:part R7"]
        );
    }
}
