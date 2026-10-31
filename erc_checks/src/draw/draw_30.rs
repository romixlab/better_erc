//! `DRAW-30` Several power or ground symbols of one rail on one pin group of a part, instead of the pins wired
//! together to one symbol

use super::{location, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{PinRef, Scene, mm};
use erc_model::Pt;
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
            let sheet = sheet_subject(cx, scene);
            for (symbol, s) in scene.geometry.symbols.iter().enumerate() {
                if s.power || s.pins.iter().filter(|p| !p.hidden).count() <= 2 {
                    continue;
                }
                for side in [Dir::Left, Dir::Right, Dir::Up, Dir::Down] {
                    let pins = side_pins(s, side);
                    let rails: Vec<Option<(&str, Vec<usize>)>> = pins
                        .iter()
                        .map(|p| rail_of(scene, PinRef { symbol, pin: *p }))
                        .collect();
                    let mut i = 0;
                    while i < pins.len() {
                        let Some((rail, _)) = &rails[i] else {
                            i += 1;
                            continue;
                        };
                        let same = |r: &Option<(&str, Vec<usize>)>| {
                            r.as_ref().is_some_and(|(n, _)| n == rail)
                        };
                        let end = i + rails[i..].iter().take_while(|r| same(r)).count();
                        let symbols: BTreeSet<usize> = rails[i..end]
                            .iter()
                            .flatten()
                            .flat_map(|(_, s)| s.iter().copied())
                            .collect();
                        if symbols.len() > max {
                            let first = PinRef {
                                symbol,
                                pin: pins[i],
                            };
                            let p: Pt = s.pins[pins[i]].at;
                            if let Some(item) = scene.pin_item(first) {
                                let mut f = cx
                                    .finding(&META)
                                    .subject(sheet)
                                    .item(
                                        sheet,
                                        format!("{} pins {}", s.reference, names(s, &pins[i..end])),
                                    )
                                    .location(location(scene, item, p));
                                for n in &symbols {
                                    let power = scene.items.iter().find(|i| i.owner == Some(*n));
                                    if let Some(power) = power {
                                        let at = scene.geometry.symbols[*n].at;
                                        f = f.location(location(scene, power, at));
                                    }
                                }
                                out.push(
                                    f.message(format!(
                                        "{} pins {} have {} {rail} symbols, from x {} y {}",
                                        s.reference,
                                        names(s, &pins[i..end]),
                                        symbols.len(),
                                        mm(p.x),
                                        mm(p.y)
                                    ))
                                    .fix("wire the pins together to one symbol")
                                    .build(),
                                );
                            }
                        }
                        i = end;
                    }
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw30));
