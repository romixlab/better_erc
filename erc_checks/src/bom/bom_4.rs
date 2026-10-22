//! `BOM-4` Propose consolidating identical resistors into resistor networks

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::package::Package;
use erc_analysis::value::format_si;
use erc_model::{EcadPinType, NetId, NetKind, PartId, Unit, natural_cmp};
use std::collections::{BTreeMap, BTreeSet};

pub struct Bom4;

static META: CheckMeta = CheckMeta {
    id: "BOM-4",
    category: "BOM",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[Param {
        name: "min_group",
        default: 2.0,
        unit: "parts",
        doc: "the smallest group of identical resistors worth an array (arrays come with 2, 4 or 8 elements)",
    }],
};

/// A group of identical resistors: value key, package, the parts on their near ends, the net their far ends
/// share (none: in series)
type Group = (String, String, Vec<PartId>, Option<NetId>);

/// A resistor's place in a group: the net of its end on the shared part, the net of its other end
struct Member {
    resistor: PartId,
    near: NetId,
    far: NetId,
}

impl Check for Bom4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Fitted two-pin resistors of one value and package whose one end is on pins of the same IC or connector
    /// (each on its own net) and whose other ends share a net (pull-ups to a rail) or are all apart (series
    /// resistors); a group of `min_group` or more is a finding, once per set of resistors
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let classes = cx.classes();
        let values = cx.values();
        let min = cx.param(&META, "min_group").max(2.0) as usize;
        // (value, package, part on the near end) -> members
        let mut groups: BTreeMap<(String, String, PartId), Vec<Member>> = BTreeMap::new();
        let mut shown_value: BTreeMap<String, String> = BTreeMap::new();
        for r in cx.view().fitted_components() {
            if !classes.get(r).is_resistor() {
                continue;
            }
            let pins = &b.part(r).pins;
            let v = values.get(r);
            let (Some(q), [a, c]) = (v.quantity(), pins.as_slice()) else {
                continue;
            };
            let (Some(na), Some(nc)) = (b.pin(*a).net, b.pin(*c).net) else {
                continue;
            };
            if na == nc || q.unit != Unit::Ohm || q.value == 0.0 {
                continue;
            }
            let pkg = Package::of(b, r)
                .map(|p| p.short().to_string())
                .unwrap_or_default();
            let value = format!("{:.6e}", q.value);
            shown_value
                .entry(value.clone())
                .or_insert_with(|| format_si(q.value, Unit::Ohm));
            for (near, far) in [(na, nc), (nc, na)] {
                // A supply net is the shared end, never the end on the host's signal pins
                let supply = b.net(near).pins.iter().any(|p| {
                    matches!(
                        b.pin(*p).etype.get(),
                        Some(EcadPinType::PowerIn | EcadPinType::PowerOut)
                    )
                });
                if supply || b.net(near).kind == NetKind::Power {
                    continue;
                }
                let hosts: BTreeSet<PartId> = net_pins(b, near)
                    .map(|p| b.pin(p).part)
                    .filter(|p| {
                        let c = classes.get(*p);
                        *p != r && cx.fitted(*p) && (c.is_ic() || c.is_connector())
                    })
                    .collect();
                for host in hosts {
                    groups
                        .entry((value.clone(), pkg.clone(), host))
                        .or_default()
                        .push(Member {
                            resistor: r,
                            near,
                            far,
                        });
                }
            }
        }
        // resistor set -> (value, package, hosts, far net shared)
        let mut sets: BTreeMap<Vec<PartId>, Group> = BTreeMap::new();
        for ((value, pkg, host), members) in groups {
            // One resistor per near net (two on one pin are not two array elements)
            let mut seen = BTreeSet::new();
            let members: Vec<Member> = members
                .into_iter()
                .filter(|m| seen.insert(m.near))
                .collect();
            let mut by_far: BTreeMap<NetId, Vec<&Member>> = BTreeMap::new();
            for m in &members {
                by_far.entry(m.far).or_default().push(m);
            }
            let mut subgroups: Vec<(Vec<PartId>, Option<NetId>)> = Vec::new();
            let mut apart = Vec::new();
            for (far, ms) in by_far {
                if ms.len() > 1 {
                    subgroups.push((ms.iter().map(|m| m.resistor).collect(), Some(far)));
                } else {
                    apart.push(ms[0].resistor);
                }
            }
            subgroups.push((apart, None));
            for (mut rs, far) in subgroups {
                rs.sort();
                rs.dedup();
                if rs.len() < min {
                    continue;
                }
                sets.entry(rs)
                    .or_insert_with(|| (value.clone(), pkg.clone(), Vec::new(), far))
                    .2
                    .push(host);
            }
        }
        let keys: Vec<Vec<PartId>> = sets.keys().cloned().collect();
        let mut out = Vec::new();
        for (rs, (value, pkg, hosts, far)) in sets {
            // A group inside a bigger one is the bigger one's finding
            let set: BTreeSet<PartId> = rs.iter().copied().collect();
            if keys
                .iter()
                .any(|k| k.len() > rs.len() && set.iter().all(|r| k.contains(r)))
            {
                continue;
            }
            let mut names: Vec<&str> = rs.iter().map(|p| b.part(*p).designator.as_str()).collect();
            names.sort_by(|x, y| natural_cmp(x, y));
            let mut host_names: Vec<&str> = hosts
                .iter()
                .map(|p| b.part(*p).designator.as_str())
                .collect();
            host_names.sort_by(|x, y| natural_cmp(x, y));
            let other = match far {
                Some(n) => format!("and {}", b.net(n).name),
                None => "in series".to_string(),
            };
            let pkg = if pkg.is_empty() {
                String::new()
            } else {
                format!(" {pkg}")
            };
            let mut f = cx
                .finding(&META)
                .parts(rs.iter().copied())
                .message(format!(
                    "{} × {}{pkg} ({}) between {} {other}: one array instead of {} placements",
                    rs.len(),
                    shown_value[&value],
                    names.join(", "),
                    host_names.join(" and "),
                    rs.len()
                ))
                .fact("min_group", min, cx.param_source(&META, "min_group"))
                .fix("use an isolated-element array where values and layout allow; keep discrete parts where rework or precision matters");
            if let Some(n) = far {
                f = f.net(n);
            }
            out.push(f.build());
        }
        out
    }
}

inventory::submit!(Registered(&Bom4));

#[cfg(test)]
mod tests {
    #[test]
    fn array_candidates() {
        // R1..R4 10k 0402 from U1's D0..D3 to +3V3; R5..R6 33R in series from U1 to J1; R7 10k 0603 to +3V3
        // (other package); R8 4k7 from U1 to +3V3 alone
        assert_eq!(
            crate::testing::fingerprints(&super::Bom4, "bom-4_resistor_arrays"),
            [
                "BOM-4:net /+3V3|part R1|part R2|part R3|part R4",
                "BOM-4:part R5|part R6"
            ]
        );
    }
}
