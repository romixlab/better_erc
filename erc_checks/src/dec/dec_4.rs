//! `DEC-4` Caution against ferrite beads without measurements

use super::{caps, designators};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::value::format_si;
use erc_model::{EcadPinType, NetId, PartClass, Provenance, Unit};

pub struct Dec4;

static META: CheckMeta = CheckMeta {
    id: "DEC-4",
    category: "DEC",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails, Need::Values],
    overlaps: &[],
    params: &[Param {
        name: "bead_inductance",
        default: 1e-6,
        unit: "H",
        doc: "low-frequency inductance assumed for a bead while its part data is not known",
    }],
};

impl Check for Dec4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every fitted ferrite between two rails, with the LC resonance it forms with the capacitance behind it
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let l = cx.param(&META, "bead_inductance");
        let mut out = Vec::new();
        for bead in cx.view().fitted_components() {
            if cx.classes().get(bead) != PartClass::FerriteBead || b.part(bead).pins.len() != 2 {
                continue;
            }
            let nets: Vec<NetId> = b
                .part(bead)
                .pins
                .iter()
                .filter_map(|p| b.pin(*p).net)
                .collect();
            let [x, y] = nets[..] else { continue };
            if x == y || !power.is_rail(x) || !power.is_rail(y) {
                continue;
            }
            let (source, load) = if source_score(cx, x) >= source_score(cx, y) {
                (x, y)
            } else {
                (y, x)
            };
            let behind = caps(cx, load);
            let farads: f64 = behind
                .iter()
                .filter_map(|c| cx.values().get(*c).value)
                .sum();
            let mut f = cx.finding(&META).part(bead).nets([source, load]);
            let resonance = if farads > 0.0 {
                let hz = 1.0 / (2.0 * std::f64::consts::PI * (l * farads).sqrt());
                f = f
                    .fact(
                        "capacitance behind it",
                        format!(
                            "{} ({})",
                            format_si(farads, Unit::Farad),
                            designators(cx, behind.iter().copied())
                        ),
                        Provenance::Derived {
                            rule: "capacitors from the load side to ground".into(),
                        },
                    )
                    .fact(
                        "bead inductance",
                        format_si(l, Unit::Henry),
                        Provenance::Assumed {
                            rule: "DEC-4 bead_inductance: no part data for the bead".into(),
                        },
                    );
                format!(
                    ": with {} behind it the LC resonance is near {}",
                    format_si(farads, Unit::Farad),
                    format_si(hz, Unit::Hertz)
                )
            } else {
                String::new()
            };
            out.push(
                f.message(format!(
                    "ferrite {} in series from {} to {}{resonance}; check its impedance at the rail current and the ripple",
                    b.part(bead).designator,
                    b.net(source).name,
                    b.net(load).name
                ))
                .fix("look at the bead's impedance curve at the operating current, damp the resonance (series R or a lossy capacitor), or use a 0 Ω or an RC filter; measure the ripple")
                .build(),
            );
        }
        out
    }
}

/// How much a net looks like the source side of a series part: a power output pin, then a connector, then
/// the number of parts on it
fn source_score(cx: &Ctx, net: NetId) -> (bool, bool, usize) {
    let b = cx.board();
    let pins: Vec<_> = net_pins(b, net).collect();
    (
        pins.iter()
            .any(|p| b.pin(*p).etype.get() == Some(&EcadPinType::PowerOut)),
        pins.iter()
            .any(|p| cx.classes().get(b.pin(*p).part).is_connector()),
        pins.len(),
    )
}

inventory::submit!(Registered(&Dec4));

#[cfg(test)]
mod tests {
    #[test]
    fn ferrites_on_rails() {
        let found = crate::testing::fingerprints(&super::Dec4, "dec-4_ferrite_beads");
        assert_eq!(found, ["DEC-4:net +3V3|net +3V3_A|part FB1"], "{found:?}");
    }
}
