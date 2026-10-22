//! `NAME-10` Active-low signal without a marker in its name

use super::named_nets;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::names::{Half, active_low, diff_half};
use erc_model::PinId;

pub struct Name10;

static META: CheckMeta = CheckMeta {
    id: "NAME-10",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph],
    overlaps: &[],
    params: &[],
};

/// Whether a pin is active low: its name carries a marker (not the `_N` of a pair whose `_P` the part also has),
/// or the symbol draws it with an inversion bubble
fn low_pin(cx: &Ctx, q: PinId) -> bool {
    let b = cx.board();
    let pin = b.pin(q);
    let part = b.part(pin.part);
    if let Some(def) = part.symbol.and_then(|s| {
        b.symbols[s.index()].pins.iter().find(|d| {
            d.number == pin.number && (d.unit == 0 || d.unit == pin.unit || pin.unit == 0)
        })
    }) && def.style.starts_with("inverted")
    {
        return true;
    }
    if active_low(&pin.name).is_none() {
        return false;
    }
    match diff_half(&pin.name) {
        Some(h) if h.half == Half::Neg => !part.pins.iter().any(|o| {
            diff_half(&b.pin(*o).name).is_some_and(|x| x.half == Half::Pos && x.key() == h.key())
        }),
        _ => true,
    }
}

impl Check for Name10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Named signal nets on an active-low pin (`~{RESET}`, `nCS`, `OE#`, an inversion bubble) whose own name has
    /// no active-low marker in any style; pins whose names carry no marker wait for pin roles (NET-7)
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for net in named_nets(cx) {
            let name = &b.net(net).name;
            if power.is_power_net(net) || active_low(name).is_some() {
                continue;
            }
            let low: Vec<PinId> = net_pins(b, net).filter(|q| low_pin(cx, *q)).collect();
            if low.is_empty() {
                continue;
            }
            let shown: Vec<String> = low
                .iter()
                .map(|q| format!("{} ({})", b.pin_name(*q), b.pin(*q).name))
                .collect();
            let mut f = cx.finding(&META).net(net);
            for q in &low {
                f = f.pin(*q);
            }
            out.push(
                f.message(format!(
                    "{name} goes to active-low {} but its name does not say so",
                    shown.join(", ")
                ))
                .fix("add the project's active-low marker to the net name (NAME-1)")
                .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Name10));

#[cfg(test)]
mod tests {
    #[test]
    fn unmarked_low_nets() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name10, "name-10_active_low_unmarked"),
            [
                "NAME-10:net /EXP_OE|pin U3.1",
                "NAME-10:net /FLASH_CS|pin U2.1",
                "NAME-10:net /HUB_RESET|pin U1.1",
            ]
        );
    }
}
