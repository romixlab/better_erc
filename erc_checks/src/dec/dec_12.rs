//! `DEC-12` Polarized capacitor with its negative terminal on the higher-potential net

use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::value::format_si;
use erc_model::{NetId, PartClass, PinId, Provenance, Unit};

pub struct Dec12;

static META: CheckMeta = CheckMeta {
    id: "DEC-12",
    category: "DEC",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[
        Need::Graph,
        Need::PartClasses,
        Need::Rails,
        Need::RailVoltages,
    ],
    overlaps: &[],
    params: &[],
};

impl Check for Dec12 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A fitted polarized capacitor whose `+` pin (named `+`, else number 1) is on a lower voltage than its `-`
    /// pin (named `-`, else number 2), both voltages known
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let mut out = Vec::new();
        for cap in cx.view().fitted_components() {
            if cx.classes().get(cap) != (PartClass::Capacitor { polarized: true }) {
                continue;
            }
            let Some((plus, minus)) = terminals(cx, cap) else {
                continue;
            };
            let (Some(np), Some(nm)) = (b.pin(plus).net, b.pin(minus).net) else {
                continue;
            };
            let (Some(vp), Some(vm)) = (power.volts(np), power.volts(nm)) else {
                continue;
            };
            if vp >= vm {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .part(cap)
                    .nets([np, nm])
                    .message(format!(
                        "{} is reversed: + on {} ({}), - on {} ({}), {} of reverse voltage",
                        b.part(cap).designator,
                        b.net(np).name,
                        format_si(vp, Unit::Volt),
                        b.net(nm).name,
                        format_si(vm, Unit::Volt),
                        format_si(vm - vp, Unit::Volt)
                    ))
                    .fact("+ side", format_si(vp, Unit::Volt), source(cx, np))
                    .fact("- side", format_si(vm, Unit::Volt), source(cx, nm))
                    .fix("flip the capacitor; on a negative rail the + goes to ground")
                    .build(),
            );
        }
        out
    }
}

/// The `+` and `-` pins of a polarized capacitor: by name, else pin 1 is `+` and pin 2 is `-` (KiCad's
/// `C_Polarized`)
fn terminals(cx: &Ctx, cap: erc_model::PartId) -> Option<(PinId, PinId)> {
    let b = cx.board();
    let pins = &b.part(cap).pins;
    if pins.len() != 2 {
        return None;
    }
    let named = |names: &[&str]| {
        pins.iter()
            .copied()
            .find(|p| names.contains(&b.pin(*p).name.as_str()))
    };
    let numbered = |n: &str| pins.iter().copied().find(|p| b.pin(*p).number == n);
    if let Some(plus) = named(&["+"]) {
        let minus = pins.iter().copied().find(|p| *p != plus)?;
        return Some((plus, minus));
    }
    if let Some(minus) = named(&["-", "−"]) {
        let plus = pins.iter().copied().find(|p| *p != minus)?;
        return Some((plus, minus));
    }
    Some((numbered("1")?, numbered("2")?))
}

/// Where a net's voltage comes from: the rail's resolved fact, or the ground it is
fn source(cx: &Ctx, net: NetId) -> Provenance {
    cx.power()
        .rails
        .get(&net)
        .and_then(|r| r.voltage.resolve())
        .map(|f| f.source.clone())
        .unwrap_or_else(|| Provenance::Derived {
            rule: "a ground is 0 V".into(),
        })
}

inventory::submit!(Registered(&Dec12));

#[cfg(test)]
mod tests {
    #[test]
    fn reversed_electrolytics() {
        let found = crate::testing::fingerprints(&super::Dec12, "dec-12_reversed_polarized");
        assert_eq!(
            found,
            [
                "DEC-12:net +12V|net GND|part C3",
                "DEC-12:net -5V|net GND|part C2"
            ],
            "{found:?}"
        );
    }
}
