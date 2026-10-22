//! `NAME-11` Negative rail named without its sign

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::power::{NameRole, read_name};
use erc_model::{NetKind, PinId};

pub struct Name11;

static META: CheckMeta = CheckMeta {
    id: "NAME-11",
    category: "NAME",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Rails],
    overlaps: &[],
    params: &[],
};

impl Check for Name11 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Named rails with a negative marker (`5V0_NEG`, `VEE`, `VNEG`) whose voltage is not written with a minus
    /// (read by the marker's rule, or not at all), and named rails read as positive or without a voltage where
    /// negative supply pins (`VEE`, `V-`) sit. A marker the grammar does not know (`V5N`) stays unseen.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for (net, rail) in &power.rails {
            let n = b.net(*net);
            if !matches!(n.kind, NetKind::Named | NetKind::Power) {
                continue;
            }
            let name = read_name(&n.name, false);
            let positive = rail.volts().is_none_or(|v| v > 0.0);
            let by_marker = name.negative && (name.sign_from_marker || name.voltage.is_none());
            let neg_pins: Vec<PinId> = net_pins(b, *net)
                .filter(|q| {
                    let r = read_name(&b.pin(*q).name, false);
                    matches!(r.role, NameRole::Rail(_)) && r.negative
                })
                .collect();
            let message = if by_marker {
                match rail.volts() {
                    Some(v) => format!(
                        "{} is read as {v} V only by its negative marker: write the sign (`-{}V`)",
                        n.name, -v
                    ),
                    None => format!(
                        "{} is a negative rail by its name, without a voltage or a sign",
                        n.name
                    ),
                }
            } else if positive && !name.negative && !neg_pins.is_empty() {
                let shown: Vec<String> = neg_pins
                    .iter()
                    .map(|q| format!("{} ({})", b.pin_name(*q), b.pin(*q).name))
                    .collect();
                format!(
                    "{} carries negative supply pins {} but reads {}",
                    n.name,
                    shown.join(", "),
                    rail.volts()
                        .map_or("without a sign".into(), |v| format!("+{v} V"))
                )
            } else {
                continue;
            };
            let mut f = cx.finding(&META).net(*net);
            for q in &neg_pins {
                f = f.pin(*q);
            }
            if let Some(v) = rail.voltage.resolve() {
                f = f.fact("voltage", v.value, v.source.clone());
            }
            out.push(
                f.message(message)
                    .fix("name negative rails with a leading minus (`-5V0`, `-12V`)")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Name11));

#[cfg(test)]
mod tests {
    #[test]
    fn unsigned_negative_rails() {
        assert_eq!(
            crate::testing::fingerprints(&super::Name11, "name-11_negative_rail_names"),
            ["NAME-11:net 5V0_NEG|pin U1.4", "NAME-11:net VAUX|pin U2.4",]
        );
    }
}
