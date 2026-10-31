//! `DRAW-30` Several power or ground symbols of one rail on one pin group of a part, instead of the pins wired
//! together to one symbol

use super::{location, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{PinRef, Scene, mm};
use erc_model::geometry::{Dir, DrawnSymbol};
use std::collections::BTreeSet;

pub struct Draw30;

const MAX_PER_GROUP: Param = Param {
    name: "max_symbols_per_rail",
    default: 1.0,
    unit: "",
    doc: "power or ground symbols a pin group may have",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-30",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[MAX_PER_GROUP],
};

/// The rail a pin reaches by a tie that touches no other part, and that tie's power symbols
fn rail_of<'s>(scene: &Scene<'s>, pin: PinRef) -> Option<(&'s str, Vec<usize>)> {
    let g = scene.geometry;
    let tie = scene.ties.iter().find(|t| t.pins.contains(&pin))?;
    let power = |s: &&PinRef| g.symbols[s.symbol].power;
    let symbols: Vec<usize> = tie.pins.iter().filter(power).map(|p| p.symbol).collect();
    let others = tie
        .pins
        .iter()
        .any(|p| !g.symbols[p.symbol].power && p.symbol != pin.symbol);
    let first = *symbols.first()?;
    (!others).then(|| (g.symbols[first].value.as_str(), symbols))
}

/// The visible pins of one side, in order along it
fn side_pins(s: &DrawnSymbol, side: Dir) -> Vec<usize> {
    let mut pins: Vec<usize> = (0..s.pins.len())
        .filter(|p| !s.pins[*p].hidden && s.pins[*p].outward == side)
        .collect();
    pins.sort_by_key(|p| {
        let at = s.pins[*p].at;
        if side.is_horizontal() { at.y } else { at.x }
    });
    pins
}

fn names(s: &DrawnSymbol, pins: &[usize]) -> String {
    let n: Vec<&str> = pins.iter().map(|p| s.pins[*p].number.as_str()).collect();
    n.join(", ")
}

/// The finding on a run of pins of one symbol
fn finding(
    cx: &Ctx,
    scene: &Scene,
    symbol: usize,
    pins: &[usize],
    rail: &str,
    power: &BTreeSet<usize>,
) -> Option<Finding> {
    let g = scene.geometry;
    let s = &g.symbols[symbol];
    let sheet = sheet_subject(cx, scene);
    let p = s.pins[pins[0]].at;
    let item = scene.pin_item(PinRef {
        symbol,
        pin: pins[0],
    })?;
    let list = names(s, pins);
    let mut f = cx
        .finding(&META)
        .subject(sheet)
        .item(sheet, format!("{} pins {list}", s.reference))
        .location(location(scene, item, p));
    for n in power {
        if let Some(i) = scene.items.iter().find(|i| i.owner == Some(*n)) {
            f = f.location(location(scene, i, g.symbols[*n].at));
        }
    }
    Some(
        f.message(format!(
            "{} pins {list} have {} {rail} symbols, from x {} y {}",
            s.reference,
            power.len(),
            mm(p.x),
            mm(p.y)
        ))
        .fix("wire the pins together to one symbol")
        .build(),
    )
}

/// The runs of next-door pins on one rail along a side: where each starts and ends, the rail, its symbols
fn runs<'s>(
    rails: &[Option<(&'s str, Vec<usize>)>],
) -> Vec<(usize, usize, &'s str, BTreeSet<usize>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < rails.len() {
        let Some((rail, _)) = &rails[i] else {
            i += 1;
            continue;
        };
        let same = |r: &&Option<(&str, Vec<usize>)>| r.as_ref().is_some_and(|(n, _)| n == rail);
        let end = i + rails[i..].iter().take_while(same).count();
        let power = rails[i..end].iter().flatten();
        let power = power.flat_map(|(_, s)| s.iter().copied()).collect();
        out.push((i, end, *rail, power));
        i = end;
    }
    out
}

impl Check for Draw30 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Runs of next-door pins of one side on one rail, each pin reaching it by a tie of this part alone. Parts of
    /// two pins are left out: a decoupling capacitor keeps its own symbols.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let max = cx.param(&META, MAX_PER_GROUP.name) as usize;
        let mut out = Vec::new();
        for scene in cx.scenes() {
            for (symbol, s) in scene.geometry.symbols.iter().enumerate() {
                if s.power || s.pins.iter().filter(|p| !p.hidden).count() <= 2 {
                    continue;
                }
                for side in [Dir::Left, Dir::Right, Dir::Up, Dir::Down] {
                    let pins = side_pins(s, side);
                    let rail = |p: &usize| rail_of(scene, PinRef { symbol, pin: *p });
                    let rails: Vec<_> = pins.iter().map(rail).collect();
                    for (start, end, rail, power) in runs(&rails) {
                        if power.len() > max {
                            out.extend(finding(cx, scene, symbol, &pins[start..end], rail, &power));
                        }
                    }
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw30));

#[cfg(test)]
mod tests {
    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw30, "draw-30_symbols_per_pin_group"),
            ["DRAW-30:item U1 pins 1, 2, 3|sheet /"]
        );
    }
}
