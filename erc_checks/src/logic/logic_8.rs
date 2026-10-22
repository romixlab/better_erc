//! `LOGIC-8` Polarity mismatch by name

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::{Polarity, base, mismatch, polarity};
use erc_model::{EcadPinType, NetId, NetKind, PinId};

pub struct Logic8;

static META: CheckMeta = CheckMeta {
    id: "LOGIC-8",
    category: "LOGIC",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Variant],
    overlaps: &[],
    params: &[],
};

fn shown(p: Polarity) -> &'static str {
    if p.active_low {
        "active low"
    } else {
        "active high"
    }
}

impl Check for Logic8 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// On each net, the output pins (output, open collector or emitter, tri-state) and the input pins of fitted
    /// parts whose names state a polarity (`~{RESET}`, `PGOOD`, `INT_N`); when no output states one, the net's own
    /// name stands for the driver. A driver and an input whose polarities disagree, with the sense of their words
    /// taken into account (`PG` into `~SHDN` agrees, `PGOOD` into `~EN` does not), is a finding: on one net there is
    /// no inverter between them
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for n in (0..b.nets.len()).map(NetId::from_index) {
            let net = b.net(n);
            if net.kind == NetKind::Power {
                continue;
            }
            let pins: Vec<PinId> = net_pins(b, n)
                .filter(|p| cx.fitted(b.pin(*p).part))
                .collect();
            let known = |want: &dyn Fn(EcadPinType) -> bool| -> Vec<(PinId, Polarity)> {
                pins.iter()
                    .filter(|p| b.pin(**p).etype.get().copied().is_some_and(want))
                    .filter_map(|p| Some((*p, polarity(&b.pin(*p).name)?)))
                    .collect()
            };
            let drivers = known(&|t| {
                matches!(
                    t,
                    EcadPinType::Output
                        | EcadPinType::OpenCollector
                        | EcadPinType::OpenEmitter
                        | EcadPinType::TriState
                )
            });
            let inputs = known(&|t| t == EcadPinType::Input);
            if inputs.is_empty() {
                continue;
            }
            // (who drives, its polarity, its pin if a pin)
            let sources: Vec<(String, Polarity, Option<PinId>)> = if drivers.is_empty() {
                match (net.kind, polarity(base(&net.name))) {
                    (NetKind::Named, Some(p)) => vec![(format!("net name {}", net.name), p, None)],
                    _ => Vec::new(),
                }
            } else {
                drivers
                    .iter()
                    .map(|(pin, p)| {
                        (
                            format!("{} {}", b.pin_name(*pin), b.pin(*pin).name),
                            *p,
                            Some(*pin),
                        )
                    })
                    .collect()
            };
            let mut pairs = Vec::new();
            let mut parts = Vec::new();
            for (what, from, pin) in &sources {
                for (input, to) in &inputs {
                    if Some(*input) == *pin || !mismatch(*from, *to) {
                        continue;
                    }
                    pairs.push(format!(
                        "{what} ({}) into {} {} ({})",
                        shown(*from),
                        b.pin_name(*input),
                        b.pin(*input).name,
                        shown(*to)
                    ));
                    parts.extend(pin.map(|p| b.pin(p).part));
                    parts.push(b.pin(*input).part);
                }
            }
            if pairs.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .net(n)
                    .parts(parts)
                    .message(format!("{}: {}", net.name, pairs.join("; ")))
                    .fix("fix the wiring, add an inverter, or rename so the polarity is visible")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Logic8));

#[cfg(test)]
mod tests {
    #[test]
    fn polarity_mismatches() {
        // U1 PGOOD into U2 ~EN (mismatch); U1 PG into U3 ~SHDN (as meant); U4 ~RESET into U5 RESET (mismatch);
        // ~HUB_RESET from a GPIO (net name) into U6 RESET (mismatch); U7 ~INT into U8 ~IRQ (fine)
        assert_eq!(
            crate::testing::fingerprints(&super::Logic8, "logic-8_polarity"),
            [
                "LOGIC-8:net /HUB_RESET_N|part U6",
                "LOGIC-8:net /PG_A|part U1|part U2",
                "LOGIC-8:net /RST_B|part U4|part U5"
            ]
        );
    }
}
