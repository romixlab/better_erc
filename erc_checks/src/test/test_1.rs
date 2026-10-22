//! `TEST-1` Unconnected pins without test points (optional report table)

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_degree;
use erc_model::{EcadPinType, PinId};

pub struct Test1;

static META: CheckMeta = CheckMeta {
    id: "TEST-1",
    category: "TEST",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Test1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Per fitted IC, the pins that connect to nothing (no net, a net of their own, or a no-connect flag), so no
    /// test point can reach them: outputs first, then by pin. Pins typed no-connect are left out.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in cx.view().fitted_components() {
            let class = cx.classes().get(part);
            if !class.is_ic() && !class.is_programmable() {
                continue;
            }
            let mut pins: Vec<PinId> = b
                .part(part)
                .pins
                .iter()
                .copied()
                .filter(|p| {
                    let pin = b.pin(*p);
                    pin.etype.get() != Some(&EcadPinType::NoConnect)
                        && (pin.no_connect || pin.net.is_none_or(|n| net_degree(b, n) <= 1))
                })
                .collect();
            if pins.is_empty() {
                continue;
            }
            let output = |p: &PinId| {
                !matches!(
                    b.pin(*p).etype.get(),
                    Some(
                        EcadPinType::Output
                            | EcadPinType::TriState
                            | EcadPinType::OpenCollector
                            | EcadPinType::OpenEmitter
                            | EcadPinType::PowerOut
                    )
                )
            };
            pins.sort_by(|x, y| {
                output(x)
                    .cmp(&output(y))
                    .then_with(|| erc_model::natural_cmp(&b.pin(*x).number, &b.pin(*y).number))
            });
            let table: Vec<String> = pins
                .iter()
                .map(|p| {
                    let pin = b.pin(*p);
                    let kind = pin.etype.get().map_or("?", |t| t.name());
                    let flag = if pin.no_connect {
                        ", no-connect flag"
                    } else {
                        ""
                    };
                    if pin.name.is_empty() || pin.name == "~" {
                        format!("{} ({kind}{flag})", b.pin_name(*p))
                    } else {
                        format!("{} {} ({kind}{flag})", b.pin_name(*p), pin.name)
                    }
                })
                .collect();
            let f = pins
                .iter()
                .fold(cx.finding(&META).part(part), |f, p| f.pin(*p));
            out.push(
                f.message(format!(
                    "{}: {} pin{} unconnected, no test point can reach {}: {}",
                    b.part(part).designator,
                    pins.len(),
                    if pins.len() == 1 { "" } else { "s" },
                    if pins.len() == 1 { "it" } else { "them" },
                    table.join(", ")
                ))
                .fix("a test point on outputs and spare pins worth observing; a no-connect flag on the rest")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Test1));

#[cfg(test)]
mod tests {
    #[test]
    fn unconnected_pins() {
        let found = crate::testing::fingerprints(&super::Test1, "test-1_unconnected_pins");
        assert_eq!(
            found,
            ["TEST-1:part U2|pin U2.4|pin U2.5|pin U2.6"],
            "{found:?}"
        );
    }
}
