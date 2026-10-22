//! `MEM-1` SD / eMMC CMD and DAT lines without pull-ups (10k-100k)

use super::{card_parts, line_pulls, sd_pins_of};
use crate::meta::*;
use crate::text::{and_list, pin_list};
use crate::usb::connected;
use crate::{Ctx, Finding};
use erc_analysis::interface::{SdPin, sd_pins};
use erc_analysis::value::format_si;
use erc_model::Unit;

pub struct Mem1;

static META: CheckMeta = CheckMeta {
    id: "MEM-1",
    category: "MEM",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "min_pull_up",
            default: 10_000.0,
            unit: "ohm",
            doc: "lower end of the CMD and DAT pull-up window, inside it",
        },
        Param {
            name: "max_pull_up",
            default: 100_000.0,
            unit: "ohm",
            doc: "upper end of the CMD and DAT pull-up window, inside it",
        },
    ],
};

impl Check for Mem1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// CMD and DAT pins of an SD socket or eMMC (by name) whose line (the net and one series resistor away) has no
    /// pull-up to a rail, or one outside the window. A socket's CMD and DAT0-DAT3 connected to nothing float too;
    /// an eMMC's unused DAT4-DAT7 are left alone.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let (min, max) = (
            cx.param(&META, "min_pull_up"),
            cx.param(&META, "max_pull_up"),
        );
        let mut out = Vec::new();
        for part in card_parts(cx) {
            let socket = cx.classes().get(part).is_connector();
            let lines = sd_pins_of(cx, part, &|f| {
                f.iter().any(|f| matches!(f, SdPin::Cmd | SdPin::Dat(_)))
            });
            for pin in lines {
                let name = &b.pin(pin).name;
                let what = match connected(cx, pin) {
                    None => {
                        let low_dat = sd_pins(name).iter().any(|f| {
                            matches!(f, SdPin::Cmd) || matches!(f, SdPin::Dat(n) if *n < 4)
                        });
                        if !socket || !low_dat {
                            continue;
                        }
                        "is connected to nothing".to_string()
                    }
                    Some(net) => {
                        let ups = line_pulls(cx, net, true);
                        if ups.is_empty() {
                            format!("({}) has no pull-up", b.net(net).name)
                        } else {
                            let outside: Vec<String> = ups
                                .iter()
                                .filter_map(|u| {
                                    let v = cx.values().get(u.part).value?;
                                    (v < min || v > max).then(|| {
                                        format!(
                                            "{} {}",
                                            b.part(u.part).designator,
                                            format_si(v, Unit::Ohm)
                                        )
                                    })
                                })
                                .collect();
                            if outside.len() < ups.len() || outside.is_empty() {
                                continue;
                            }
                            format!(
                                "({}) is pulled up by {}, outside {}..{}",
                                b.net(net).name,
                                and_list(&outside),
                                format_si(min, Unit::Ohm),
                                format_si(max, Unit::Ohm)
                            )
                        }
                    }
                };
                let mut f = cx.finding(&META).part(part).pin(pin);
                if let Some(net) = connected(cx, pin) {
                    f = f.net(net);
                }
                out.push(
                    f.message(format!(
                        "{} {} {}: open-drain during identification and tri-stated between commands, it floats",
                        if socket { "SD socket" } else { "eMMC" },
                        pin_list(cx, &[pin]),
                        what
                    ))
                    .fact(
                        "window",
                        format!("{}..{}", format_si(min, Unit::Ohm), format_si(max, Unit::Ohm)),
                        cx.param_source(&META, "min_pull_up"),
                    )
                    .fix("10 kΩ (up to 47 kΩ) from CMD and every DAT line to the card's VDD (SD) or VCCQ (eMMC), near the socket")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Mem1));

#[cfg(test)]
mod tests {
    #[test]
    fn lines_without_pull_ups() {
        let found = crate::testing::fingerprints(&super::Mem1, "mem-1_sd_pull_ups");
        assert_eq!(
            found,
            [
                "MEM-1:net /EMMC_D0|part U3|pin U3.6",
                "MEM-1:net /SD_D1|part J1|pin J1.8",
                "MEM-1:net /SD_D3|part J1|pin J1.2",
                "MEM-1:part J1|pin J1.1",
            ],
            "{found:?}"
        );
    }
}
