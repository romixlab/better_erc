//! `DBG-3` No LEDs on the board at all: suggest at least one for debug

use super::{leds, programmable};
use crate::meta::*;
use crate::text::designators;
use crate::{Ctx, Finding};

pub struct Dbg3;

static META: CheckMeta = CheckMeta {
    id: "DBG-3",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg3 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A board with a fitted MCU, SoC or FPGA and no fitted LED
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let parts = programmable(cx);
        if parts.is_empty() || !leds(cx).is_empty() {
            return Vec::new();
        }
        vec![
            cx.finding(&META)
                .parts(parts.iter().copied())
                .message(format!(
                    "no LED on the board of {}: nothing shows that the firmware runs",
                    designators(cx, parts.iter().copied())
                ))
                .fix("one LED on a free GPIO (DBG-16); 1-2 mA is enough")
                .build(),
        ]
    }
}

inventory::submit!(Registered(&Dbg3));

#[cfg(test)]
mod tests {
    #[test]
    fn board_without_leds() {
        let found = crate::testing::fingerprints(&super::Dbg3, "dbg-3_no_leds");
        assert_eq!(found, ["DBG-3:part U1"], "{found:?}");
        // A board with LEDs, none of them driven by the MCU: DBG-16's, not this
        let found = crate::testing::fingerprints(&super::Dbg3, "dbg-16_heartbeat_led");
        assert!(found.is_empty(), "{found:?}");
    }
}
