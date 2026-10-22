//! `DBG-2` No series resistors on an obvious debug header (e.g. SWD)

use super::{named_pins, programmable};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::PinFunction::*;
use erc_model::{PartId, PinId};
use std::collections::BTreeMap;

pub struct Dbg2;

static META: CheckMeta = CheckMeta {
    id: "DBG-2",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// SWD and JTAG pins (not reset, not SWO) of an MCU, SoC or FPGA on the same net as a connector pin: no
    /// series resistor between the pin and the header
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for part in programmable(cx) {
            // header -> the debug pins wired straight to it
            let mut direct: BTreeMap<PartId, Vec<PinId>> = BTreeMap::new();
            for (pin, f) in named_pins(cx, part) {
                if !matches!(f.function, Swdio | Swclk | Tck | Tms | Tdi | Tdo) {
                    continue;
                }
                let Some(net) = b.pin(pin).net else { continue };
                for other in net_pins(b, net) {
                    let header = b.pin(other).part;
                    if cx.classes().get(header).is_connector() {
                        let pins = direct.entry(header).or_default();
                        if !pins.contains(&pin) {
                            pins.push(pin);
                        }
                    }
                }
            }
            for (header, pins) in direct {
                let f = pins
                    .iter()
                    .fold(cx.finding(&META).part(part).part(header), |f, p| f.pin(*p));
                out.push(
                    f.message(format!(
                        "{} go straight to header {} with no series resistor",
                        pin_list(cx, &pins),
                        b.part(header).designator
                    ))
                    .fix("22-100 Ω in SWCLK and SWDIO (TCK, TMS, TDI, TDO) near the MCU; NRST and VTref without one")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Dbg2));

#[cfg(test)]
mod tests {
    #[test]
    fn header_without_series_resistors() {
        let found = crate::testing::fingerprints(&super::Dbg2, "dbg-2_swd_series_resistors");
        assert_eq!(
            found,
            ["DBG-2:part J1|part U1|pin U1.3|pin U1.4"],
            "{found:?}"
        );
    }
}
