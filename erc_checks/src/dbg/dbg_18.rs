//! `DBG-18` I2C or SPI buses not reachable at a header or test points for a bus analyser

use super::buses;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::access::reach;
use erc_model::NetId;

pub struct Dbg18;

static META: CheckMeta = CheckMeta {
    id: "DBG-18",
    category: "DBG",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::PartClasses],
    overlaps: &[],
    params: &[],
};

impl Check for Dbg18 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Each I2C bus (NET-4) and SPI bus (by pin names: the clock net and the MOSI, MISO and CS nets of the
    /// parts on it) with lines that reach no test point or connector, directly or through a series part
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        let mut out = Vec::new();
        for (kind, name, lines) in buses(cx) {
            let missing: Vec<NetId> = lines
                .iter()
                .copied()
                .filter(|n| reach(cx.view(), cx.classes(), cx.power(), *n).is_empty())
                .collect();
            if missing.is_empty() {
                continue;
            }
            let names: Vec<&str> = missing.iter().map(|n| b.net(*n).name.as_str()).collect();
            out.push(
                cx.finding(&META)
                    .nets(missing.iter().copied())
                    .message(format!(
                        "{kind} bus {name}: {} reach{} no test point or header for an analyser",
                        crate::text::and_list(&names),
                        if missing.len() == 1 { "es" } else { "" }
                    ))
                    .fix("a test point per line plus GND, or an unpopulated 2.54 mm header")
                    .build(),
            );
        }
        out
    }
}

inventory::submit!(Registered(&Dbg18));

#[cfg(test)]
mod tests {
    #[test]
    fn buses_without_access() {
        let found = crate::testing::fingerprints(&super::Dbg18, "dbg-18_bus_access");
        assert_eq!(
            found,
            [
                "DBG-18:net /IMU_CS|net /IMU_MISO|net /IMU_MOSI|net /IMU_SCK",
                "DBG-18:net /SDA"
            ],
            "{found:?}"
        );
    }
}
