//! `DBG-17` SWD header without NRST (and SWO): the probe cannot reset a stuck target or trace

use super::{pin_reach, pins_with, programmable};
use crate::meta::*;
use crate::text::{and_list, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::pin_name::PinFunction;
use erc_model::{PartClass, PartId, PinId};
use std::collections::BTreeSet;

pub struct Dbg17;

static META: CheckMeta = CheckMeta {
    id: "DBG-17",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg17 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// For each header that carries an MCU's SWDIO and SWCLK (directly or through a series part), the MCU's
    /// reset and SWO pins (by name, where it has them) must reach the same header; where SWD reaches test
    /// points only, they must reach a test point
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in programmable(cx) {
            let swdio = pins_with(cx, part, PinFunction::Swdio);
            let swclk = pins_with(cx, part, PinFunction::Swclk);
            if swdio.is_empty() || swclk.is_empty() {
                continue;
            }
            let parts_reached = |pins: &[PinId]| -> BTreeSet<PartId> {
                pins.iter()
                    .flat_map(|p| pin_reach(cx, *p))
                    .map(|r| b.pin(r.pin).part)
                    .collect()
            };
            let both: BTreeSet<PartId> = parts_reached(&swdio)
                .intersection(&parts_reached(&swclk))
                .copied()
                .collect();
            let headers: Vec<PartId> = both
                .iter()
                .copied()
                .filter(|p| cx.classes().get(*p).is_connector())
                .collect();
            let wanted = [
                (PinFunction::Reset, pins_with(cx, part, PinFunction::Reset)),
                (PinFunction::Swo, pins_with(cx, part, PinFunction::Swo)),
            ];
            let test_point = |p: &PartId| cx.classes().get(*p) == PartClass::TestPoint;
            // Where SWD comes out: each header, or the test points as one set
            let mut places: Vec<(Option<PartId>, BTreeSet<PartId>)> = headers
                .iter()
                .map(|h| (Some(*h), BTreeSet::from([*h])))
                .collect();
            if headers.is_empty() && both.iter().any(test_point) {
                places.push((None, BTreeSet::new()));
            }
            for (header, targets) in places {
                let missing: Vec<(PinFunction, Vec<PinId>)> = wanted
                    .iter()
                    .filter(|(_, pins)| !pins.is_empty())
                    .filter(|(_, pins)| {
                        !parts_reached(pins).iter().any(|p| {
                            if header.is_some() {
                                targets.contains(p)
                            } else {
                                test_point(p)
                            }
                        })
                    })
                    .cloned()
                    .collect();
                if missing.is_empty() {
                    continue;
                }
                let pins: Vec<PinId> = missing.iter().flat_map(|(_, p)| p.clone()).collect();
                let names: Vec<&str> = missing.iter().map(|(f, _)| f.name()).collect();
                let place = match header {
                    Some(h) => format!("header {}", b.part(h).designator),
                    None => "the SWD test points".into(),
                };
                let mut f = cx.finding(&META).part(part);
                if let Some(h) = header {
                    f = f.part(h);
                }
                let f = pins.iter().fold(f, |f, p| f.pin(*p));
                out.push(
                    f.message(format!(
                        "{place} carries SWDIO and SWCLK of {} but not its {} ({})",
                        b.part(part).designator,
                        and_list(&names),
                        pin_list(cx, &pins)
                    ))
                    .fix("a standard pinout with nRESET (ARM 10-pin: pin 10) and SWO (pin 6); on test-point designs an NRST pad")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Dbg17));

#[cfg(test)]
mod tests {
    #[test]
    fn header_without_reset() {
        let found = crate::testing::fingerprints(&super::Dbg17, "dbg-17_swd_without_reset");
        assert_eq!(
            found,
            ["DBG-17:part J1|part U1|pin U1.5|pin U1.6"],
            "{found:?}"
        );
    }
}
