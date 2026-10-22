//! `I2C-4` Pull-up to a non-rail net

use super::lines;
use crate::meta::*;
use crate::rail::{MAX_FEED, fed};
use crate::{Ctx, Finding};
use erc_analysis::graph::net_pins;
use erc_analysis::i2c::rail_goes_nowhere;

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
    /// but the pair, or a resistor to nowhere, is `I2C-3`'s.
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
            out.push(
                cx.finding(&META)
                    .nets(lines(bus))
                    .parts([pu.scl, pu.sda])
                    .net(far)
                    .message(format!(
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
}
