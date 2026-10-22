//! `NAME` - Naming conventions

mod name_1;
mod name_10;
mod name_11;
mod name_12;
mod name_2;
mod name_4;
mod name_5;
mod name_6;
mod name_7;
mod name_9;

use crate::Ctx;
use erc_model::{NetId, NetKind, PartId};

/// A designator split into its letters and number: `R12` is (`R`, 12); `R?`, `#PWR01` and `TP` are none
pub(crate) fn designator(d: &str) -> Option<(&str, u32)> {
    let i = d.find(|c: char| c.is_ascii_digit())?;
    let (prefix, number) = d.split_at(i);
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some((prefix, number.parse().ok()?))
}

/// How the board numbers its designators
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Numbering {
    /// One series across the project (or a single sheet)
    Flat,
    /// The sheet's page number times `base` (100 or 1000) starts each sheet's series
    PerSheet { base: u32 },
}

/// The page number of a part's sheet, when the sheet has a numeric one
pub(crate) fn page(cx: &Ctx, part: PartId) -> Option<u32> {
    let b = cx.board();
    b.sheet(b.part(part).sheet)
        .page
        .as_deref()?
        .trim()
        .parse()
        .ok()
}

/// The numbered components, with their page
pub(crate) fn numbered_parts<'a>(cx: &Ctx<'a>) -> Vec<(PartId, &'a str, u32, Option<u32>)> {
    let b = cx.board();
    cx.components()
        .filter_map(|p| {
            let (prefix, n) = designator(&b.part(p).designator)?;
            Some((p, prefix, n, page(cx, p)))
        })
        .collect()
}

/// The board's scheme: per sheet when at least half the parts on numbered pages carry their page in the hundreds
/// (or thousands) of their number, and the parts sit on two pages or more
pub(crate) fn numbering(cx: &Ctx) -> Numbering {
    let parts = numbered_parts(cx);
    let paged: Vec<(u32, u32)> = parts
        .iter()
        .filter_map(|(_, _, n, pg)| Some((*n, (*pg)?)))
        .collect();
    let pages: std::collections::BTreeSet<u32> = paged.iter().map(|(_, pg)| *pg).collect();
    if pages.len() < 2 {
        return Numbering::Flat;
    }
    for base in [100, 1000] {
        let matching = paged.iter().filter(|(n, pg)| n / base == *pg).count();
        if matching * 2 >= paged.len() {
            return Numbering::PerSheet { base };
        }
    }
    Numbering::Flat
}

/// The nets a name could be fixed on: named by a label or a power symbol, not auto-named
pub(crate) fn named_nets<'a>(cx: &Ctx<'a>) -> impl Iterator<Item = NetId> + 'a {
    let b = cx.board();
    (0..b.nets.len())
        .map(NetId::from_index)
        .filter(move |n| matches!(b.net(*n).kind, NetKind::Named | NetKind::Power))
}

#[cfg(test)]
mod tests {
    #[test]
    fn designators() {
        assert_eq!(super::designator("R12"), Some(("R", 12)));
        assert_eq!(super::designator("TP3"), Some(("TP", 3)));
        assert_eq!(super::designator("R?"), None);
        assert_eq!(super::designator("#PWR01"), None);
        assert_eq!(super::designator("U1A"), None);
    }
}
