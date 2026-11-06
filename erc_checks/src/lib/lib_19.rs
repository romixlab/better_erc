//! `LIB-19` Hidden symbol pin that is not grouped with a visible pin of the same meaning

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_model::{EcadPinType, NetKind, PinId};

pub struct Lib19;

const STRICT: Param = Param {
    name: "strict",
    default: 0.0,
    unit: "",
    doc: "1: also report hidden pins grouped with a visible pin of the same meaning, and hidden no-connect pins",
};

static META: CheckMeta = CheckMeta {
    id: "LIB-19",
    category: "LIB",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Variant],
    overlaps: &[],
    params: &[STRICT],
};

/// A name that says "the package's pad" and nothing else: such a pin means what the pin it is grouped with means
fn pad_name(name: &str) -> bool {
    [
        "EP",
        "EPAD",
        "PAD",
        "TAB",
        "PADDLE",
        "THERMAL",
        "THERMAL_PAD",
    ]
    .iter()
    .any(|n| name.eq_ignore_ascii_case(n))
}

/// The two pins mean the same: one name, or the hidden one is the pad
fn same_meaning(hidden: &str, visible: &str) -> bool {
    hidden.eq_ignore_ascii_case(visible) || pad_name(hidden)
}

/// How a hidden pin sits
enum Hidden {
    /// Under a visible pin of the same meaning on its net: allowed, reported only when strict
    Grouped(PinId),
    /// A no-connect pin on no net: allowed, reported only when strict
    NoConnect,
    /// On a net with visible pins of the part, none of the same meaning
    Mixed(Vec<PinId>),
    /// No visible pin of the part on its net: it connects by its name alone, or to nothing
    Alone,
}

impl Check for Lib19 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// For each hidden pin of a fitted part, the visible pins of the same part on its net: one of the same name (or
    /// the hidden pin is the pad) groups it, others mean it hides a different function under them, none means it
    /// joins its net unseen. Pins on no net and typed no-connect are left alone unless strict.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let strict = cx.param(&META, STRICT.name) != 0.0;
        let mut out = Vec::new();
        for p in cx.view().fitted_components() {
            let part = b.part(p);
            for &h in &part.pins {
                let pin = b.pin(h);
                if !pin.hidden {
                    continue;
                }
                let on_net = pin.net.filter(|n| b.net(*n).kind != NetKind::Unconnected);
                let visible: Vec<PinId> = on_net
                    .map(|n| {
                        part.pins
                            .iter()
                            .copied()
                            .filter(|q| !b.pin(*q).hidden && b.pin(*q).net == Some(n))
                            .collect()
                    })
                    .unwrap_or_default();
                let etype = pin.etype.get().copied();
                let state = if let Some(&m) = visible
                    .iter()
                    .find(|q| same_meaning(&pin.name, &b.pin(**q).name))
                {
                    Hidden::Grouped(m)
                } else if !visible.is_empty() {
                    Hidden::Mixed(visible)
                } else if on_net.is_none() && etype == Some(EcadPinType::NoConnect) {
                    Hidden::NoConnect
                } else {
                    Hidden::Alone
                };
                let what = format!("{} pin {} ({})", part.designator, pin.number, pin.name);
                let f = cx.finding(&META).part(p).pin(h);
                let f = match state {
                    Hidden::Grouped(m) if strict => {
                        let mate = b.pin(m);
                        f.pin(m)
                            .severity(Severity::Suggestion)
                            .message(format!(
                                "{what} is hidden under pin {} ({}): the same meaning, but the sheet shows one pin \
                                 where the part has more (strict)",
                                mate.number, mate.name
                            ))
                            .fix("draw every pin visibly in the symbol")
                    }
                    Hidden::NoConnect if strict => f
                        .severity(Severity::Suggestion)
                        .message(format!(
                            "{what} is a hidden no-connect pin: the sheet does not show it is left open (strict)"
                        ))
                        .fix("draw the pin visibly, with a no-connect flag"),
                    Hidden::Grouped(_) | Hidden::NoConnect => continue,
                    Hidden::Mixed(visible) => {
                        let names: Vec<String> = visible
                            .iter()
                            .map(|q| format!("{} ({})", b.pin(*q).number, b.pin(*q).name))
                            .collect();
                        f.pins(visible)
                            .message(format!(
                                "{what} is hidden on the net of pin {}, which means something else: the sheet never \
                                 shows that this pin is tied there",
                                names.join(", ")
                            ))
                            .fix(
                                "draw the pin visibly in the symbol and wire it on the sheet; only pins of the same \
                                 meaning (the grounds, the pad) may be grouped under one",
                            )
                    }
                    Hidden::Alone => {
                        let net = on_net.map(|n| b.net(n).name.clone());
                        let f = f.message(match &net {
                            Some(net) => format!(
                                "{what} is hidden and joins {net} by its name alone: no visible pin of {} is on that \
                                 net",
                                part.designator
                            ),
                            None => format!("{what} is hidden and connects to nothing"),
                        });
                        let f = match on_net {
                            Some(n) => f.net(n),
                            None => f,
                        };
                        f.fix("draw the pin visibly in the symbol and wire it on the sheet")
                    }
                };
                out.push(f.build());
            }
        }
        out
    }
}

inventory::submit!(Registered(&Lib19));

#[cfg(test)]
mod tests {
    use crate::RunOptions;

    // U1: GND pins and the pad hidden under one visible GND pin. U2: an N.C. pin (the TPD6S300A's 16 / 17, the
    // TPD8S300A's D4 / D3) hidden under GND. U3: a hidden power input VCC alone, joining VCC by name. U4: a hidden
    // no-connect pin.
    const SCHEMATIC: &str = "lib-19_hidden_pins";

    #[test]
    fn hidden_pins() {
        assert_eq!(
            crate::testing::fingerprints(&super::Lib19, SCHEMATIC),
            [
                "LIB-19:net VCC|part U3|pin U3.5",
                "LIB-19:part U2|pin U2.1|pin U2.3"
            ]
        );
    }

    #[test]
    fn hidden_pins_strict() {
        let mut opts = RunOptions::default();
        opts.config.params.insert("LIB-19.strict".into(), 1.0);
        assert_eq!(
            crate::testing::fingerprints_with(&super::Lib19, SCHEMATIC, &opts),
            [
                "LIB-19:net VCC|part U3|pin U3.5",
                "LIB-19:part U1|pin U1.3|pin U1.6",
                "LIB-19:part U1|pin U1.3|pin U1.7",
                "LIB-19:part U2|pin U2.1|pin U2.3",
                "LIB-19:part U4|pin U4.6",
            ]
        );
    }
}
