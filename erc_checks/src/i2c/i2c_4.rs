//! `I2C-4` Pull-up to a non-rail net

use super::lines;
use crate::io::net_pulls;
use crate::meta::*;
use crate::rail::{MAX_FEED, fed};
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::i2c::rail_goes_nowhere;
use erc_model::{EcadPinType, NetId, PartId, PinId};
use std::collections::BTreeSet;

pub struct I2c4;

static META: CheckMeta = CheckMeta {
    id: "I2C-4",
    category: "I2C",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Rails],
    overlaps: &[],
    params: &[MAX_FEED],
};

impl Check for I2c4 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A pull-up pair whose far end is no rail (a signal such as an enable GPIO), or a rail by its name alone that
    /// holds only resistors and that no source reaches (a label typo, `3V3` for `+3V3`). A far end with nothing
    /// but the pair, or a resistor to nowhere, is `I2C-3`'s. Pull-ups to a controller's push-pull output on a bus
    /// whose lines also have pull-downs (straps read while the output is low) are a strap-gated bus: info.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let power = cx.power();
        let fed = fed(cx, &META);
        let mut out = Vec::new();
        for bus in &cx.i2c().buses {
            let Some(pu) = bus.pull_up else { continue };
            if rail_goes_nowhere(b, cx.classes(), &pu) {
                continue;
            }
            let far = pu.rail;
            let name = &b.net(far).name;
            let why = if !power.is_rail(far) {
                format!("{name}, which is no rail")
            } else if !fed.contains_key(&far)
                && net_pins(b, far).all(|p| cx.classes().get(b.pin(p).part).is_resistor())
            {
                format!(
                    "{name}, a rail by its name that holds only resistors and that no source reaches"
                )
            } else {
                continue;
            };
            let f = cx
                .finding(&META)
                .nets(lines(bus))
                .parts([pu.scl, pu.sda])
                .net(far);
            let gpio = (!power.is_rail(far))
                .then(|| push_pull_gpio(cx, far))
                .flatten();
            let straps: BTreeSet<PartId> = lines(bus)
                .into_iter()
                .flat_map(|n| net_pulls(cx, n).down)
                .map(|p| p.part)
                .collect();
            if let Some(pin) = gpio
                && !straps.is_empty()
            {
                let names: Vec<&str> = straps
                    .iter()
                    .map(|p| b.part(*p).designator.as_str())
                    .collect();
                out.push(
                    f.severity(Severity::Info)
                        .message(format!(
                            "I2C bus {}: pull-ups {} and {} go to {name}, driven by push-pull {}, with pull-down{} {} on the lines: a strap-gated bus, idle only once the GPIO is high",
                            bus.name,
                            b.part(pu.scl).designator,
                            b.part(pu.sda).designator,
                            b.pin_name(pin),
                            if names.len() > 1 { "s" } else { "" },
                            names.join(", "),
                        ))
                        .fix("drive the GPIO high before the bus is used, and note it in the firmware's pin setup")
                        .build(),
                );
                continue;
            }
            out.push(
                f.message(format!(
                    "I2C bus {}: pull-ups {} and {} go to {why}: the bus works only once something drives it high",
                    bus.name,
                    b.part(pu.scl).designator,
                    b.part(pu.sda).designator,
                ))
                .fix("pull up to the rail; a switched pull-up wants a bus switch or a FET with a documented default (IO-3)")
                .build(),
            );
        }
        out
    }
}

/// The push-pull output pin of a fitted MCU, SoC or FPGA on `net` (pin type output: the symbol's, or a chosen
/// alternate such as `PF0/PP`)
fn push_pull_gpio(cx: &Ctx, net: NetId) -> Option<PinId> {
    let b = cx.board();
    net_pins(b, net).find(|p| {
        let part = b.pin(*p).part;
        cx.fitted(part)
            && cx.classes().get(part).is_programmable()
            && b.pin(*p).etype.get() == Some(&EcadPinType::Output)
    })
}

inventory::submit!(Registered(&I2c4));

#[cfg(test)]
mod tests {
    #[test]
    fn pull_ups_to_no_rail() {
        let found = crate::testing::fingerprints(&super::I2c4, "i2c-4_pull_up_to_signal");
        assert_eq!(
            found,
            [
                "I2C-4:net /I2C_EN|net /SCL1|net /SDA1|part R1|part R2",
                "I2C-4:net /SCL2|net /SDA2|net 3V3|part R3|part R4"
            ],
            "{found:?}"
        );
    }

    #[test]
    fn strap_gated_bus_is_info() {
        use crate::meta::Severity;
        use crate::testing::report_with;
        let severities = |schematic| -> Vec<(String, Severity)> {
            report_with(&super::I2c4, schematic, &crate::RunOptions::default())
                .findings
                .iter()
                .map(|f| (f.fingerprint.clone(), f.severity))
                .collect()
        };
        // R1, R2 to I2C_EN, U1's PA1/PP; R3, R4 pull the lines down for straps
        let fp = "I2C-4:net /I2C_EN|net /I2C_SCL|net /I2C_SDA|part R1|part R2".to_owned();
        assert_eq!(
            severities("i2c-4_strap_gated_bus"),
            [(fp.clone(), Severity::Info)]
        );
        // The same bus without the pull-downs
        assert_eq!(severities("i2c-4_gpio_pull_ups"), [(fp, Severity::Error)]);
    }
}
