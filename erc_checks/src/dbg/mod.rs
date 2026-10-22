//! `DBG` - debug access, indicators, rework options: the debug, console and bus pins of [`erc_analysis::pin_name`],
//! probe access of [`erc_analysis::access`], LEDs and the rails they show

mod dbg_1;
mod dbg_10;
mod dbg_11;
mod dbg_14;
mod dbg_16;
mod dbg_17;
mod dbg_18;
mod dbg_2;
mod dbg_3;
mod dbg_4;
mod dbg_7;
mod dbg_8;

use crate::Ctx;
use erc_analysis::access::{Reach, reach};
use erc_analysis::graph::net_pins;
use erc_analysis::pin_name::{NamedFunction, PinFunction, functions};
use erc_model::{NetId, PartId, PinId};
use std::collections::BTreeSet;

/// The fitted MCUs, SoCs and FPGAs
pub(crate) fn programmable(cx: &Ctx) -> Vec<PartId> {
    cx.view()
        .fitted_components()
        .filter(|p| cx.classes().get(*p).is_programmable())
        .collect()
}

/// Every pin of a part with the functions its name states
pub(crate) fn named_pins(cx: &Ctx, part: PartId) -> Vec<(PinId, NamedFunction)> {
    let b = cx.board();
    b.part(part)
        .pins
        .iter()
        .flat_map(|p| functions(&b.pin(*p).name).into_iter().map(move |f| (*p, f)))
        .collect()
}

/// The pins of a part whose name states `function`
pub(crate) fn pins_with(cx: &Ctx, part: PartId, function: PinFunction) -> Vec<PinId> {
    let mut pins: Vec<PinId> = named_pins(cx, part)
        .into_iter()
        .filter(|(_, f)| f.function == function)
        .map(|(p, _)| p)
        .collect();
    pins.dedup();
    pins
}

/// Where a probe reaches a pin's net: test points and connector pins on it or one series part away
pub(crate) fn pin_reach(cx: &Ctx, pin: PinId) -> Vec<Reach> {
    match cx.board().pin(pin).net {
        Some(net) => reach(cx.view(), cx.classes(), cx.power(), net),
        None => Vec::new(),
    }
}

/// Whether any of `pins` reaches a test point or connector
pub(crate) fn reached(cx: &Ctx, pins: &[PinId]) -> bool {
    pins.iter().any(|p| !pin_reach(cx, *p).is_empty())
}

/// A net and the nets one fitted two-pin resistor away (rails and grounds included): the ends of an LED's
/// resistor chain
pub(crate) fn with_resistor_ends(cx: &Ctx, net: NetId) -> BTreeSet<NetId> {
    let b = cx.board();
    let mut out = BTreeSet::from([net]);
    for pin in net_pins(b, net) {
        let part = b.pin(pin).part;
        if !cx.fitted(part) || !cx.classes().get(part).is_resistor() || b.part(part).pins.len() != 2
        {
            continue;
        }
        out.extend(
            b.part(part)
                .pins
                .iter()
                .filter(|p| **p != pin)
                .filter_map(|p| b.pin(*p).net),
        );
    }
    out
}

/// The fitted LEDs
pub(crate) fn leds(cx: &Ctx) -> Vec<PartId> {
    cx.view()
        .fitted_components()
        .filter(|p| cx.classes().get(*p).is_led())
        .collect()
}

/// The two ends of an LED, each its pin's net with the nets one resistor away
pub(crate) fn led_ends(cx: &Ctx, led: PartId) -> Vec<BTreeSet<NetId>> {
    let b = cx.board();
    b.part(led)
        .pins
        .iter()
        .filter_map(|p| b.pin(*p).net)
        .map(|n| with_resistor_ends(cx, n))
        .collect()
}
