//! `DBG-1` SWD/JTAG not connected

use super::{pins_with, programmable, reached};
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::{Ctx, Finding};
use erc_analysis::access::access_pins;
use erc_analysis::pin_name::PinFunction::{self, *};
use erc_model::PinId;

pub struct Dbg1;

static META: CheckMeta = CheckMeta {
    id: "DBG-1",
    category: "DBG",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

const SWD: &[PinFunction] = &[Swdio, Swclk];
const JTAG: &[PinFunction] = &[Tck, Tms, Tdi, Tdo];

impl Check for Dbg1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// An MCU, SoC or FPGA whose debug pins (by name) do not reach a connector or test points, directly or
    /// through a series part: SWDIO and SWCLK, or the four JTAG signals, and a ground. A part whose pin names
    /// state no debug port is not judged (its port pins need alternate functions, NET-12).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let power = cx.power();
        let ground_reached = power
            .grounds
            .keys()
            .any(|g| !access_pins(cx.view(), cx.classes(), *g).is_empty());
        let mut out = Vec::new();
        for part in programmable(cx) {
            let port = |set: &[PinFunction]| -> Option<Vec<(PinFunction, Vec<PinId>)>> {
                let pins: Vec<_> = set.iter().map(|f| (*f, pins_with(cx, part, *f))).collect();
                pins.iter().all(|(_, p)| !p.is_empty()).then_some(pins)
            };
            let ports: Vec<_> = [port(SWD), port(JTAG)].into_iter().flatten().collect();
            if ports.is_empty() {
                continue;
            }
            let ok = |pins: &[(PinFunction, Vec<PinId>)]| pins.iter().all(|(_, p)| reached(cx, p));
            if ground_reached && ports.iter().any(|p| ok(p)) {
                continue;
            }
            // The first port the part has (SWD before JTAG) names the pins that do not reach
            let missing: Vec<PinId> = ports[0]
                .iter()
                .filter(|(_, p)| !reached(cx, p))
                .flat_map(|(_, p)| p.clone())
                .collect();
            let what = if missing.is_empty() {
                "no ground reaches a connector or test point".to_string()
            } else {
                format!(
                    "{} reach no connector or test point",
                    pin_list(cx, &missing)
                )
            };
            let f = missing
                .iter()
                .fold(cx.finding(&META).part(part), |f, p| f.pin(*p));
            out.push(
                f.message(format!(
                        "debug port of {} not brought out: {what}",
                        designators(cx, [part])
                    ))
                    .fix("a debug header (ARM 10-pin 1.27 mm, Tag-Connect) or test points for SWDIO, SWCLK, NRST, VTref and GND")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dbg1));

#[cfg(test)]
mod tests {
    #[test]
    fn debug_port_not_brought_out() {
        let found = crate::testing::fingerprints(&super::Dbg1, "dbg-1_swd_not_connected");
        assert_eq!(found, ["DBG-1:part U1|pin U1.3|pin U1.4"], "{found:?}");
    }
}
