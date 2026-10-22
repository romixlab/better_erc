//! `I2C-2` I2C multiple pull-ups

use super::{lines, names};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct I2c2;

static META: CheckMeta = CheckMeta {
    id: "I2C-2",
    category: "I2C",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for I2c2 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Resistors from a line to the pull-up rail besides the pair, and several pull-up pairs on one direct segment
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let i2c = cx.i2c();
        let designators = |parts: &mut dyn Iterator<Item = erc_model::PartId>| {
            parts
                .map(|p| b.part(p).designator.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut out = Vec::new();
        for bus in &i2c.buses {
            let Some(pu) = bus.pull_up else { continue };
            if bus.redundant_pull_ups.is_empty() {
                continue;
            }
            out.push(
                cx.finding(&META)
                    .nets(lines(bus))
                    .parts(bus.redundant_pull_ups.iter().copied())
                    .message(format!(
                        "I2C bus {}: {} pull up besides the pair {} and {}",
                        bus.name,
                        designators(&mut bus.redundant_pull_ups.iter().copied()),
                        b.part(pu.scl).designator,
                        b.part(pu.sda).designator
                    ))
                    .fix("keep one pair; make the others DNP")
                    .build(),
            );
        }
        for s in &i2c.direct_segments {
            let pulled: Vec<_> = s
                .iter()
                .filter(|i| i2c.buses[**i].pull_up.is_some())
                .collect();
            if pulled.len() < 2 {
                continue;
            }
            let parts: Vec<erc_model::PartId> = pulled
                .iter()
                .filter_map(|i| i2c.buses[**i].pull_up)
                .flat_map(|pu| [pu.scl, pu.sda])
                .collect();
            out.push(
                cx.finding(&META)
                    .nets(pulled.iter().flat_map(|i| lines(&i2c.buses[**i])))
                    .parts(parts.iter().copied())
                    .message(format!(
                        "I2C bus {}: {} pull-up pairs on one segment ({})",
                        names(cx, s),
                        pulled.len(),
                        designators(&mut parts.iter().copied())
                    ))
                    .fix("keep one pair per segment; make the others DNP")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&I2c2));

#[cfg(test)]
mod tests {
    #[test]
    fn redundant_pull_ups() {
        let found = crate::testing::fingerprints(&super::I2c2, "i2c-2_multiple_pull_ups");
        assert!(
            found.iter().any(|f| f.contains("net /SCL1_1V8")),
            "{found:?}"
        );
        let found = crate::testing::fingerprints(&super::I2c2, "i2c-7_segments");
        assert!(
            found.contains(&"I2C-2:net /SCL1_3V3|net /SDA1_3V3|net /UNM1|net /UNM2|part R401|part R403|part R415|part R416".to_string()),
            "{found:?}"
        );
    }
}
