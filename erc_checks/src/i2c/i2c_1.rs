//! `I2C-1` No pull-ups on an I2C bus

use super::{lines, names};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct I2c1;

static META: CheckMeta = CheckMeta {
    id: "I2C-1",
    category: "I2C",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for I2c1 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A direct segment (buses joined by ties) none of whose buses has a pull-up pair
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let i2c = cx.i2c();
        i2c.direct_segments
            .iter()
            .filter(|s| s.iter().all(|i| i2c.buses[*i].pull_up.is_none()))
            .map(|s| {
                cx.finding(&META)
                    .nets(s.iter().flat_map(|i| lines(&i2c.buses[*i])))
                    .message(format!(
                        "I2C bus {}: no pull-ups on SCL and SDA",
                        names(cx, s)
                    ))
                    .fix("one pull-up pair per bus segment, to the IO rail of the devices on it")
                    .build()
            })
            .collect()
    }
}

inventory::submit!(Registered(&I2c1));

#[cfg(test)]
mod tests {
    #[test]
    fn bus_without_pull_ups() {
        // /I2C2 and /I2C3 have pull-ups that go nowhere (I2C-3), /I2C1 none at all
        assert_eq!(
            crate::testing::fingerprints(&super::I2c1, "i2c-1_no_pull_ups"),
            ["I2C-1:net /SCL1|net /SDA1"]
        );
    }
}
