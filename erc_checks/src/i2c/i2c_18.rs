//! `I2C-18` Bus segment without its own pull-ups

use super::{lines, names};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct I2c18;

static META: CheckMeta = CheckMeta {
    id: "I2C-18",
    category: "I2C",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &["I2C-1"],
    params: &[],
};

impl Check for I2c18 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A direct segment of a logical bus split by translators, buffers, isolators or discrete translators that has
    /// no pull-up pair of its own (`I2C-1` on that segment, folded in). Muxes are not recognised yet.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let i2c = cx.i2c();
        let mut out = Vec::new();
        for logical in &i2c.same_bus_segments {
            let Some(first) = logical.first() else {
                continue;
            };
            let segments = i2c.segments_of_logical_bus(*first);
            if segments.len() < 2 {
                continue;
            }
            for s in &segments {
                if !i2c.pull_ups(s).is_empty() {
                    continue;
                }
                let others: Vec<&str> = segments
                    .iter()
                    .filter(|o| *o != s)
                    .flat_map(|o| o.iter().map(|i| i2c.buses[*i].name.as_str()))
                    .collect();
                out.push(
                    cx.finding(&META)
                        .nets(s.iter().flat_map(|i| lines(&i2c.buses[*i])))
                        .message(format!(
                            "I2C bus {} has no pull-ups of its own: the translator, buffer or isolator to {} does not pass the other side's",
                            names(cx, s),
                            others.join(", ")
                        ))
                        .fix("a pull-up pair per segment, to that segment's rail")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&I2c18));

#[cfg(test)]
mod tests {
    #[test]
    fn segments_without_pull_ups() {
        let found = crate::testing::fingerprints(&super::I2c18, "i2c-18_discrete_translator");
        assert_eq!(found, ["I2C-18:net /SCL_1V8|net /SDA_1V8"], "{found:?}");
        let found = crate::testing::fingerprints(&super::I2c18, "i2c-18_isolated_segment");
        assert_eq!(found, ["I2C-18:net /SCL_ISO|net /SDA_ISO"], "{found:?}");
    }
}
