//! `MEM` - memories: SD and eMMC pull-ups, clock and card-detect pulls, DDR ZQ resistors, DDR reset and clock enable
//! levels at power-up; card and DRAM pins by name ([`erc_analysis::interface`])

mod mem_1;
mod mem_10;
mod mem_3;
mod mem_8;

use crate::Ctx;
use erc_analysis::access::series_neighbours;
use erc_analysis::interface::{SdPin, sd_pins};
use erc_analysis::pull::{Pull, pulls};
use erc_model::class::ConnectorKind;
use erc_model::{NetId, PartClass, PartId, PinId};

fn emmc_text(cx: &Ctx, part: PartId) -> bool {
    let b = cx.board();
    let p = b.part(part);
    let symbol = p.symbol.map(|s| &b.symbols[s.index()]);
    format!(
        "{} {}",
        p.lib_id,
        symbol.map_or("", |s| s.description.as_str())
    )
    .to_lowercase()
    .contains("emmc")
}

/// The card side of an SD or eMMC bus: a fitted card socket, or a fitted memory (or IC that says eMMC) with `CMD`
/// and `DAT0` pins
pub(crate) fn card_parts(cx: &Ctx) -> Vec<PartId> {
    let b = cx.board();
    cx.view()
        .fitted_components()
        .filter(|p| match cx.classes().get(*p) {
            PartClass::Connector {
                kind: Some(ConnectorKind::Card),
            } => true,
            // An eMMC: a memory, or an IC whose library or description says eMMC (an MCU's `SDMMC1_CMD` is the host)
            c if matches!(c, PartClass::Memory { .. }) || (c.is_ic() && emmc_text(cx, *p)) => {
                let has = |f: SdPin| {
                    b.part(*p)
                        .pins
                        .iter()
                        .any(|q| sd_pins(&b.pin(*q).name).contains(&f))
                };
                has(SdPin::Cmd) && has(SdPin::Dat(0))
            }
            _ => false,
        })
        .collect()
}

/// The pins of a part with an SD function `keep` accepts, with their functions
pub(crate) fn sd_pins_of(cx: &Ctx, part: PartId, keep: &dyn Fn(&[SdPin]) -> bool) -> Vec<PinId> {
    let b = cx.board();
    b.part(part)
        .pins
        .iter()
        .copied()
        .filter(|p| {
            let f = sd_pins(&b.pin(*p).name);
            !f.is_empty() && keep(&f)
        })
        .collect()
}

/// A line: the net and the nets one series resistor, ferrite or tie away (a series resistor at the host or the
/// card)
pub(crate) fn line_nets(cx: &Ctx, net: NetId) -> Vec<NetId> {
    let mut nets = vec![net];
    for (_, far) in series_neighbours(cx.view(), cx.classes(), cx.power(), net) {
        if !nets.contains(&far) {
            nets.push(far);
        }
    }
    nets
}

/// The pull-ups (`up`) or pull-downs on a line ([`line_nets`])
pub(crate) fn line_pulls(cx: &Ctx, net: NetId, up: bool) -> Vec<Pull> {
    let mut out: Vec<Pull> = line_nets(cx, net)
        .into_iter()
        .flat_map(|n| {
            let p = pulls(cx.view(), cx.classes(), cx.values(), cx.power(), n);
            if up { p.up } else { p.down }
        })
        .collect();
    out.sort();
    out.dedup();
    out
}
