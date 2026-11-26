//! `RAIL` - power rails: what feeds them and what they feed, on the rails of [`erc_analysis::power`] and the
//! sources and links of [`erc_analysis::feed`]

mod rail_1;
mod rail_18;
mod rail_19;
mod rail_5;

use crate::Ctx;
use crate::meta::{CheckMeta, Param};
use erc_analysis::feed::{Link, LinkKind, MAX_FEED_OHMS, SourceKind};
use erc_analysis::power::{NameRole, read_name};
use erc_model::{NetId, PartId, PinId};
use std::collections::BTreeMap;

pub(crate) use crate::text::{designators, pin_list};

/// The threshold of what a rail check accepts as a feed
pub(crate) const MAX_FEED: Param = Param {
    name: "max_feed",
    default: MAX_FEED_OHMS,
    unit: "Ω",
    doc: "the largest series resistor that still feeds a rail (an RC filter, a shunt)",
    choices: &[],
};

/// Whether a link carries power for the rail checks: anything but a resistor above `max_feed` (or of unknown
/// value)
pub(crate) fn feeds_power(cx: &Ctx, meta: &CheckMeta) -> impl Fn(&Link) -> bool {
    let max = cx.param(meta, MAX_FEED.name);
    move |l: &Link| l.kind != LinkKind::Resistor || l.ohms.is_some_and(|r| r <= max)
}

/// The nets any source reaches, with the link parts on the way; a resistor of any value into a net a zener holds
/// to ground (a shunt regulator) feeds it
pub(crate) fn fed(cx: &Ctx, meta: &CheckMeta) -> BTreeMap<NetId, Vec<PartId>> {
    let pass = feeds_power(cx, meta);
    let feeds = cx.feeds();
    let mut fed = feeds.fed_by(
        &[
            SourceKind::Regulator,
            SourceKind::LoadSwitch,
            SourceKind::PowerOut,
            SourceKind::Connector,
            SourceKind::Battery,
        ],
        &pass,
    );
    loop {
        let shunted: Vec<(NetId, Vec<PartId>)> = feeds
            .links
            .iter()
            .filter(|l| l.kind == LinkKind::Resistor)
            .flat_map(|l| [(l.a, l.b, l.part), (l.b, l.a, l.part)])
            .filter(|(from, to, _)| {
                fed.contains_key(from) && !fed.contains_key(to) && !feeds.shunts_on(*to).is_empty()
            })
            .map(|(from, to, r)| {
                let mut path = fed[&from].clone();
                path.push(r);
                (to, path)
            })
            .collect();
        if shunted.is_empty() {
            return fed;
        }
        for (net, path) in shunted {
            for (n, rest) in feeds.reach([net], &pass) {
                fed.entry(n)
                    .or_insert_with(|| path.iter().chain(&rest).copied().collect());
            }
        }
    }
}

/// Whether a pin is named like a ground (`GND`, `VSS`, `AGND`)
pub(crate) fn ground_named(cx: &Ctx, pin: PinId) -> bool {
    matches!(
        read_name(&cx.board().pin(pin).name, false).role,
        NameRole::Ground(_)
    )
}

/// Whether all component pins on a net belong to one part (a pin alone, or a part's pins joined to nothing else)
pub(crate) fn one_part(cx: &Ctx, net: NetId) -> Option<PartId> {
    let b = cx.board();
    let mut parts = erc_analysis::graph::net_pins(b, net).map(|p| b.pin(p).part);
    let first = parts.next()?;
    parts.all(|p| p == first).then_some(first)
}
