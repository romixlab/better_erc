//! `MEM-3` SD / eMMC CLK with a pull-up, or CD / WP switch pins without one

use super::{card_parts, line_pulls, sd_pins_of};
use crate::meta::*;
use crate::text::{designators, pin_list};
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::interface::SdPin;

pub struct Mem3;

static META: CheckMeta = CheckMeta {
    id: "MEM-3",
    category: "MEM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Mem3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// The CLK line of an SD socket or eMMC with a pull-up (a series resistor is no pull); a socket's card-detect or
    /// write-protect switch pin (not `DAT3/CD`, a data line) on a signal net with no pull-up or pull-down on its line.
    /// A switch pin on a ground or rail is the switch's other side.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for part in card_parts(cx) {
            for pin in sd_pins_of(cx, part, &|f| f.contains(&SdPin::Clk)) {
                let Some(net) = connected(cx, pin) else {
                    continue;
                };
                let ups = line_pulls(cx, net, true);
                if ups.is_empty() {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .pin(pin)
                        .parts(ups.iter().map(|u| u.part))
                        .message(format!(
                            "clock {} ({}) has a pull-up, {}: CLK is driven push-pull, the pull-up only adds load and a stub on the fastest line",
                            pin_list(cx, &[pin]),
                            b.net(net).name,
                            designators(cx, ups.iter().map(|u| u.part))
                        ))
                        .fix("remove the CLK pull-up; keep a 22-33 Ω series resistor at the host if needed")
                        .build(),
                );
            }
            if !cx.classes().get(part).is_connector() {
                continue;
            }
            let switches = sd_pins_of(cx, part, &|f| {
                f.iter()
                    .any(|f| matches!(f, SdPin::CardDetect | SdPin::WriteProtect))
                    && !f.iter().any(|f| matches!(f, SdPin::Dat(_)))
            });
            for pin in switches {
                let Some(net) = connected(cx, pin) else {
                    continue;
                };
                if power.is_power_net(net)
                    || !line_pulls(cx, net, true).is_empty()
                    || !line_pulls(cx, net, false).is_empty()
                {
                    continue;
                }
                out.push(
                    cx.finding(&META)
                        .part(part)
                        .pin(pin)
                        .net(net)
                        .message(format!(
                            "switch contact {} ({}) has no pull: the input floats whenever the switch is open",
                            pin_list(cx, &[pin]),
                            b.net(net).name
                        ))
                        .fix("10-100 kΩ to the logic rail (or to ground), matching the switch's other side (MEM-5)")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Mem3));

#[cfg(test)]
mod tests {
    #[test]
    fn clock_and_switch_pulls() {
        let found = crate::testing::fingerprints(&super::Mem3, "mem-3_clk_and_detect_pulls");
        assert_eq!(
            found,
            [
                "MEM-3:net /SD1_DET|part J1|pin J1.9",
                "MEM-3:part J1|part R6|pin J1.5",
            ],
            "{found:?}"
        );
    }
}
