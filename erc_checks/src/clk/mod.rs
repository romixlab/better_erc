//! `CLK` - clocks and reset: crystal case pads, reset inputs without a pull-up or capacitor, reset buttons, push-pull
//! drivers on reset lines, shared resets, clock fan-out; reset and clock pins by name ([`erc_analysis::pin_name`])

mod clk_14;
mod clk_15;
mod clk_18;
mod clk_3;
mod clk_5;
mod clk_6;
mod clk_7;

use crate::Ctx;
use erc_analysis::pin_name::{PinFunction, active_low, states};
use erc_model::{EcadPinType, NetId, PinId};
use std::collections::BTreeMap;

/// A reset input: a pin of a fitted IC, MCU, SoC or FPGA whose name states reset, of a type that is no output or
/// supply (a supervisor's `RESET` output and a header's `nRESET` are sources, not inputs)
pub(crate) fn is_reset_input(cx: &Ctx, pin: PinId) -> bool {
    let b = cx.board();
    let p = b.pin(pin);
    let class = cx.classes().get(p.part);
    cx.fitted(p.part)
        && (class.is_ic() || class.is_programmable())
        && !p.no_connect
        && !matches!(
            p.etype.get(),
            Some(
                EcadPinType::Output
                    | EcadPinType::OpenCollector
                    | EcadPinType::OpenEmitter
                    | EcadPinType::PowerOut
                    | EcadPinType::PowerIn
                    | EcadPinType::NoConnect
            )
        )
        && states(&p.name, PinFunction::Reset)
}

/// Whether a reset input is active low: its name says so (`NRST`, `~{RESET}`, `RUN`, `CHIP_PU`), or it is an
/// MCU's, SoC's or FPGA's (their resets are active low)
pub(crate) fn is_active_low(cx: &Ctx, pin: PinId) -> bool {
    let b = cx.board();
    let p = b.pin(pin);
    active_low(&p.name) || cx.classes().get(p.part).is_programmable()
}

/// The reset inputs by the net they are on; nets that are a rail or a ground (a reset tied off) are left out
pub(crate) fn reset_nets(cx: &Ctx) -> BTreeMap<NetId, Vec<PinId>> {
    let b = cx.board();
    let power = cx.power();
    let mut out: BTreeMap<NetId, Vec<PinId>> = BTreeMap::new();
    for part in cx.view().fitted_components() {
        for pin in &b.part(part).pins {
            if let Some(net) = b.pin(*pin).net
                && !power.is_power_net(net)
                && is_reset_input(cx, *pin)
            {
                out.entry(net).or_default().push(*pin);
            }
        }
    }
    out
}

/// The active-low reset nets: those with at least one active-low reset input, with all their reset inputs
pub(crate) fn active_low_reset_nets(cx: &Ctx) -> BTreeMap<NetId, Vec<PinId>> {
    reset_nets(cx)
        .into_iter()
        .filter(|(_, pins)| pins.iter().any(|p| is_active_low(cx, *p)))
        .collect()
}

/// The push-pull outputs (pin type `output`) of fitted parts on a net
pub(crate) fn push_pull_outputs(cx: &Ctx, net: NetId) -> Vec<PinId> {
    let b = cx.board();
    erc_analysis::graph::net_pins(b, net)
        .filter(|p| {
            let pin = b.pin(*p);
            cx.fitted(pin.part) && pin.etype.get() == Some(&EcadPinType::Output)
        })
        .collect()
}
