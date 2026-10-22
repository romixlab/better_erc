//! `I2C-10` SCL and SDA pull-ups of different value

use super::{lines, ohms, shown};
use crate::meta::*;
use crate::{Ctx, Finding};

pub struct I2c10;

static META: CheckMeta = CheckMeta {
    id: "I2C-10",
    category: "I2C",
    severity: Severity::Warning,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses, Need::Values],
    overlaps: &[],
    params: &[],
};

impl Check for I2c10 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for bus in &cx.i2c().buses {
            let Some(pu) = bus.pull_up else { continue };
            let (scl, sda) = (ohms(cx, pu.scl), ohms(cx, pu.sda));
            let (Some(a), Some(c)) = (scl.value, sda.value) else {
                continue;
            };
            if a == c {
                continue;
            }
            let mut f = cx
                .finding(&META)
                .nets(lines(bus))
                .parts([pu.scl, pu.sda])
                .message(format!(
                    "I2C bus {}: SCL pull-up {} is {}, SDA pull-up {} is {}",
                    bus.name,
                    b.part(pu.scl).designator,
                    shown(&scl),
                    b.part(pu.sda).designator,
                    shown(&sda)
                ));
            for (p, v) in [(pu.scl, &scl), (pu.sda, &sda)] {
                if let Some(src) = &v.source {
                    f = f.fact(&b.part(p).designator, shown(v), src.clone());
                }
            }
            out.push(f.fix("the same value on both lines").build());
        }
        out
    }
}

inventory::submit!(Registered(&I2c10));

#[cfg(test)]
mod tests {
    #[test]
    fn different_values() {
        assert_eq!(
            crate::testing::fingerprints(&super::I2c10, "i2c-10_non_equal_pull_ups"),
            ["I2C-10:net /SCL|net /SDA|part R601|part R602"]
        );
    }
}
