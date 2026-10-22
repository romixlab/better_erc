//! `I2C-3` Pull-up from two series resistors to nowhere

use super::lines;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::i2c::rail_goes_nowhere;

pub struct I2c3;

static META: CheckMeta = CheckMeta {
    id: "I2C-3",
    category: "I2C",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for I2c3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A pull-up pair whose rail has nothing else on it, or only a resistor to a net with nothing else
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let i2c = cx.i2c();
        i2c.buses
            .iter()
            .filter_map(|bus| {
                let pu = bus.pull_up?;
                rail_goes_nowhere(b, cx.classes(), &pu).then(|| {
                    cx.finding(&META)
                        .nets(lines(bus))
                        .parts([pu.scl, pu.sda])
                        .net(pu.rail)
                        .message(format!(
                            "I2C bus {}: pull-ups {} and {} meet on {}, which reaches no rail",
                            bus.name,
                            b.part(pu.scl).designator,
                            b.part(pu.sda).designator,
                            b.net(pu.rail).name
                        ))
                        .fix("pull up straight from the rail")
                        .build()
                })
            })
            .collect()
    }
}

inventory::submit!(Registered(&I2c3));

#[cfg(test)]
mod tests {
    #[test]
    fn pull_ups_to_nowhere() {
        let found = crate::testing::fingerprints(&super::I2c3, "i2c-1_no_pull_ups");
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("net /SCL2|net /SDA2"), "{found:?}");
        assert!(found[1].contains("net /SCL3|net /SDA3"), "{found:?}");
    }
}
