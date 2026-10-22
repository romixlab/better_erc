//! `LOGIC-3` Suggest buffering high fan-out signals

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::base;
use erc_model::{EcadPinType, NetId, NetKind, PinId};

pub struct Logic3;

static META: CheckMeta = CheckMeta {
    id: "LOGIC-3",
    category: "LOGIC",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Variant],
    overlaps: &[],
    params: &[
        Param {
            name: "fan_out",
            default: 8.0,
            unit: "inputs",
            doc: "inputs on one driver from which a buffer is suggested",
        },
        Param {
            name: "clock_fan_out",
            default: 3.0,
            unit: "inputs",
            doc: "inputs on one clock driver from which a buffer is suggested",
        },
    ],
};

/// Words that make a net or its driver a clock
const CLOCK: &[&str] = &[
    "CLK", "CLOCK", "SCK", "SCLK", "MCLK", "BCLK", "LRCLK", "XCLK", "REFCLK", "CLKIN", "CLKOUT",
];

fn clock(name: &str) -> bool {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(|w| {
            w.trim_end_matches(|c: char| c.is_ascii_digit())
                .to_ascii_uppercase()
        })
        .any(|w| CLOCK.contains(&w.as_str()))
}

impl Check for Logic3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Signal nets with exactly one output-capable pin (output, tri-state, bidirectional, open collector or
    /// emitter) on the fitted parts and at least `fan_out` input pins, or `clock_fan_out` when the net or its driver
    /// is named like a clock
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let fan_out = cx.param(&META, "fan_out");
        let clock_fan_out = cx.param(&META, "clock_fan_out");
        let mut out = Vec::new();
        for n in (0..b.nets.len()).map(NetId::from_index) {
            if b.net(n).kind == NetKind::Power {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, n)
                .filter(|p| cx.fitted(b.pin(*p).part))
                .collect();
            let ty = |p: &PinId| b.pin(*p).etype.get().copied();
            if pins
                .iter()
                .any(|p| matches!(ty(p), Some(EcadPinType::PowerIn | EcadPinType::PowerOut)))
            {
                continue;
            }
            let drivers: Vec<PinId> = pins
                .iter()
                .copied()
                .filter(|p| {
                    matches!(
                        ty(p),
                        Some(
                            EcadPinType::Output
                                | EcadPinType::TriState
                                | EcadPinType::Bidirectional
                                | EcadPinType::OpenCollector
                                | EcadPinType::OpenEmitter
                        )
                    )
                })
                .collect();
            let [driver] = drivers.as_slice() else {
                continue;
            };
            let inputs = pins
                .iter()
                .filter(|p| ty(p) == Some(EcadPinType::Input))
                .count();
            let is_clock = clock(base(&b.net(n).name)) || clock(&b.pin(*driver).name);
            let (limit, param) = if is_clock {
                (clock_fan_out, "clock_fan_out")
            } else {
                (fan_out, "fan_out")
            };
            if (inputs as f64) < limit {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(n)
                    .part(b.pin(*driver).part)
                    .message(format!(
                        "{}{} drives {inputs} inputs from {}",
                        if is_clock { "clock " } else { "" },
                        b.net(n).name,
                        b.pin_name(*driver)
                    ))
                    .fact(param, limit, cx.param_source(&META, param))
                    .fix("a buffer (74LVC1G125, a clock buffer for clocks), series termination at the driver, or a daisy-chain route")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Logic3));

#[cfg(test)]
mod tests {
    #[test]
    fn high_fan_out() {
        // LATCH_CLK from U1 to U2..U5 (4 clock inputs); RESET from U1 to U2..U9 (8 inputs); EN to U2..U8 (7)
        assert_eq!(
            crate::testing::fingerprints(&super::Logic3, "logic-3_fan_out"),
            [
                "LOGIC-3:net /LATCH_CLK|part U1",
                "LOGIC-3:net /RESET|part U1"
            ]
        );
    }
}
