//! `HS` - high-speed signals: asymmetric differential pairs, USB D+ and D- crossed, common-mode chokes on
//! multi-gigabit lanes, parts on MIPI lanes; lanes and USB data by name ([`erc_analysis::interface`])

mod hs_15;
mod hs_4;
mod hs_7;
mod hs_9;

use crate::Ctx;
use erc_model::{PartClass, PartId};

/// A common-mode choke: a fitted part of four pins (two windings), an inductor, ferrite or transformer by class, or
/// one whose description says common-mode choke
pub(crate) fn is_choke(cx: &Ctx, part: PartId) -> bool {
    let b = cx.board();
    let p = b.part(part);
    if p.pins.len() != 4 || !cx.fitted(part) {
        return false;
    }
    let symbol = p.symbol.map(|s| &b.symbols[s.index()]);
    let text = format!(
        "{} {} {}",
        p.lib_id,
        symbol.map_or("", |s| s.description.as_str()),
        symbol.map_or("", |s| s.keywords.as_str())
    )
    .to_lowercase();
    matches!(
        cx.classes().get(part),
        PartClass::Inductor | PartClass::FerriteBead | PartClass::Transformer
    ) || text.contains("common mode")
        || text.contains("common-mode")
        || text.contains("choke")
}
