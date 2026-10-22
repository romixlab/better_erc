//! `USB-1` Type-C without 5k1

use super::{connected, pin, type_c};
use crate::meta::*;
use crate::text::pin_list;
use crate::{Ctx, Finding};
use erc_analysis::connector::TypeCPin;
use erc_analysis::graph::net_pins;
use erc_analysis::pull::pulls;
use erc_analysis::value::format_si;
use erc_model::{PinId, Unit};

pub struct Usb1;

static META: CheckMeta = CheckMeta {
    id: "USB-1",
    category: "USB",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values, Need::Rails],
    overlaps: &[],
    params: &[
        Param {
            name: "rd",
            default: 5100.0,
            unit: "ohm",
            doc: "the sink termination Rd on each CC pin",
        },
        Param {
            name: "tolerance",
            default: 0.2,
            unit: "",
            doc: "how far Rd may be from its value, as a fraction (±20 %; ±10 % for a sink that reads the advertised current)",
        },
    ],
};

/// A pin of a fitted IC (a CC or PD controller, a port protector with dead-battery Rd) named with a `CC` word
/// (`CC1`, `C_CC2`, `UCPD1_CC1`): it terminates CC itself
fn cc_controller_pin(cx: &Ctx, pin: PinId) -> bool {
    let b = cx.board();
    let p = b.pin(pin);
    let class = cx.classes().get(p.part);
    cx.fitted(p.part)
        && (class.is_ic() || class.is_programmable())
        && p.name.split('/').any(|a| {
            a.split(['_', ' ', '-']).any(|w| {
                w.trim_end_matches(|c: char| c.is_ascii_digit())
                    .eq_ignore_ascii_case("CC")
            })
        })
}

impl Check for Usb1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each CC pin of a Type-C receptacle: a resistor of `rd` (within `tolerance`) to a ground, or a CC pin of an
    /// IC on its net, must be there. A CC pin with a pull-up to a rail is a source's Rp (`USB-8` judges roles).
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let rd = cx.param(&META, "rd");
        let tol = cx.param(&META, "tolerance");
        let (min, max) = (rd * (1.0 - tol), rd * (1.0 + tol));
        let mut out = Vec::new();
        for part in type_c(cx, false) {
            for role in [TypeCPin::Cc1, TypeCPin::Cc2] {
                let Some(cc) = pin(cx, part, role) else {
                    continue;
                };
                let what = match connected(cx, cc) {
                    None => "is connected to nothing".to_string(),
                    Some(net) => {
                        if net_pins(b, net).any(|p| cc_controller_pin(cx, p)) {
                            continue;
                        }
                        let pulls = pulls(cx.view(), cx.classes(), cx.values(), cx.power(), net);
                        if !pulls.up.is_empty() {
                            continue;
                        }
                        let values: Vec<_> = pulls
                            .down
                            .iter()
                            .map(|p| (p.part, cx.values().get(p.part).value))
                            .collect();
                        if values
                            .iter()
                            .any(|(_, v)| v.is_none_or(|v| v >= min && v <= max))
                        {
                            continue;
                        }
                        if values.is_empty() {
                            format!("on {} has no resistor to ground", b.net(net).name)
                        } else {
                            let list: Vec<String> = values
                                .iter()
                                .map(|(r, v)| {
                                    format!(
                                        "{} {}",
                                        b.part(*r).designator,
                                        format_si(v.unwrap_or_default(), Unit::Ohm)
                                    )
                                })
                                .collect();
                            format!(
                                "on {} has only {} to ground, not {}",
                                b.net(net).name,
                                list.join(", "),
                                format_si(rd, Unit::Ohm)
                            )
                        }
                    }
                };
                let mut f = cx.finding(&META).part(part).pin(cc);
                if let Some(net) = connected(cx, cc) {
                    f = f.net(net);
                }
                out.push(
                    f.message(format!(
                        "Type-C receptacle {}: {} {} and goes to no CC controller: a Type-C source never turns VBUS on",
                        b.part(part).designator,
                        pin_list(cx, &[cc]),
                        what
                    ))
                    .fact("rd", format_si(rd, Unit::Ohm), cx.param_source(&META, "rd"))
                    .fact("tolerance", tol, cx.param_source(&META, "tolerance"))
                    .fix("one 5.1 kΩ to GND per CC pin, or a PD controller or port protector with dead-battery Rd")
                    .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Usb1));

#[cfg(test)]
mod tests {
    #[test]
    fn cc_without_rd() {
        let found = crate::testing::fingerprints(&super::Usb1, "usb-1_no_rd");
        assert_eq!(
            found,
            [
                "USB-1:net /CC1_J1|part J1|pin J1.A5",
                "USB-1:net /CC2_J1|part J1|pin J1.B5",
            ],
            "{found:?}"
        );
    }
}
