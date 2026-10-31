//! `DRAW-42` Touching items too close in colour

use super::colour::delta_e;
use super::{location, sheet_subject};
use crate::meta::*;
use crate::{Ctx, Finding, Location};
use erc_analysis::scene::{Item, ItemKind as K, Scene, Tie, mm};
use erc_model::geometry::{Colour, Theme};
use erc_model::{Board, NetId, Pt};
use std::collections::BTreeMap;

pub struct Draw42;

const PAIR_MIN: Param = Param {
    name: "drawing.colour.pair_min",
    default: 20.0,
    unit: "CIEDE2000",
    doc: "least difference between touching items",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-42",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry, Need::ColourTheme],
    overlaps: &[],
    params: &[PAIR_MIN],
};

/// A wire's colour and what touches it
struct Touch<'s> {
    /// `pin`, `power symbol`, `label`
    what: &'static str,
    wire: Colour,
    other: Colour,
    at: Pt,
    item: Option<&'s Item>,
}

/// The same two colours on a sheet: how often, and the first place
struct Pair {
    wire: Colour,
    other: Colour,
    places: usize,
    first: Option<Location>,
    at: Pt,
}

/// The net of a tie: its first part pin's, else a power symbol's or a label's by name
fn tie_net(board: &Board, scene: &Scene, tie: &Tie) -> Option<NetId> {
    let g = scene.geometry;
    let of_pin = tie.pins.iter().find_map(|p| {
        let s = &g.symbols[p.symbol];
        let part = board.part(board.part_by_designator(&s.reference)?);
        let number = &s.pins[p.pin].number;
        let pin = part
            .pins
            .iter()
            .find(|id| board.pin(**id).number == *number);
        board.pin(*pin?).net
    });
    let of_power = || {
        tie.pins
            .iter()
            .map(|p| &g.symbols[p.symbol])
            .filter(|s| s.power)
            .find_map(|s| board.net_by_name(&s.value))
    };
    let of_label = || {
        let on_sheet = |l: &&erc_model::Label| Some(l.sheet) == scene.sheet;
        tie.labels.iter().find_map(|i| {
            let drawn = &g.labels[*i];
            let mut labels = board.labels.iter().filter(on_sheet);
            labels
                .find(|l| l.at == Some(drawn.at) && l.text == drawn.text.text)?
                .net
        })
    };
    of_pin.or_else(of_power).or_else(of_label)
}

/// The colour of a net's class: the first class of the effective `Analog,Default` that sets one
fn class_colour(board: &Board, net: NetId) -> Option<Colour> {
    let classes = board.net(net).class.get()?;
    classes.split(',').find_map(|name| {
        let class = board.net_classes.iter().find(|c| c.name == name)?;
        let param = class.params.iter().find(|p| p.key == "schematic_color")?;
        Colour::parse(param.value.get()?)
    })
}

/// What touches the wires of a tie, in the colours drawn on `theme`
fn touches<'s>(board: &Board, scene: &'s Scene, tie: &Tie, theme: &Theme) -> Vec<Touch<'s>> {
    let g = scene.geometry;
    let class = tie_net(board, scene, tie).and_then(|n| class_colour(board, n));
    let mut out = Vec::new();
    for w in tie.wires.iter().map(|i| &g.wires[*i]) {
        let Some(wire) = w.stroke.colour.or(class).or(theme.wire) else {
            continue;
        };
        for p in &tie.pins {
            let (s, at) = (&g.symbols[p.symbol], g.symbols[p.symbol].pins[p.pin].at);
            let (what, other) = if s.power {
                ("power symbol", s.colour.or(theme.component_outline))
            } else {
                ("pin", theme.pin)
            };
            if let (true, Some(other)) = (w.contains(at), other) {
                let item = scene.pin_item(*p);
                out.push(Touch {
                    what,
                    wire,
                    other,
                    at,
                    item,
                });
            }
        }
        for l in tie.labels.iter().map(|i| &g.labels[*i]) {
            // KiCad draws a label in the colour of its net class too: nothing on the sheet but a hand-set colour
            // would part them, and `DRAW-27` advises against that
            let by_class = l.text.colour.is_none() && w.stroke.colour.is_none() && class.is_some();
            let other = l.text.colour.or(class).or(theme.label(l.kind));
            if let (true, false, Some(other)) = (w.contains(l.at), by_class, other) {
                out.push(Touch {
                    what: "label",
                    wire,
                    other,
                    at: l.at,
                    item: scene
                        .items
                        .iter()
                        .find(|i| i.kind == K::Label && i.uuid == l.uuid),
                });
            }
        }
    }
    out
}

impl Check for Draw42 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every wire and what touches it on each theme: pairs closer than the setting, once per pair of colours
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let min = cx.param(&META, PAIR_MIN.name);
        let mut out = Vec::new();
        for theme in cx.themes() {
            for scene in cx.scenes() {
                let sheet = sheet_subject(cx, scene);
                let mut pairs: BTreeMap<(&str, String, String), Pair> = BTreeMap::new();
                for tie in &scene.ties {
                    for t in touches(cx.board(), scene, tie, theme) {
                        if delta_e(t.wire, t.other) >= min {
                            continue;
                        }
                        let key = (t.what, t.wire.to_string(), t.other.to_string());
                        let pair = pairs.entry(key).or_insert(Pair {
                            wire: t.wire,
                            other: t.other,
                            places: 0,
                            first: t.item.map(|i| location(scene, i, t.at)),
                            at: t.at,
                        });
                        pair.places += 1;
                    }
                }
                for ((what, wire, other), p) in pairs {
                    let first = p.first.as_ref().map_or("?", |l| l.what.as_str());
                    let mut f = cx
                        .finding(&META)
                        .subject(sheet)
                        .item(sheet, format!("{}: wire {wire}", theme.name))
                        .item(sheet, format!("{}: {what} {other}", theme.name));
                    if let Some(l) = p.first.clone() {
                        f = f.location(l);
                    }
                    out.push(
                        f.message(format!(
                            "on {}, wires {wire} and {what}s {other} differ by ΔE {:.1}, below {min}, at {} place{}, first {first} at x {} y {}",
                            theme.name,
                            delta_e(p.wire, p.other),
                            p.places,
                            if p.places == 1 { "" } else { "s" },
                            mm(p.at.x),
                            mm(p.at.y),
                        ))
                        .fix("change the theme, the net class or the library symbol, not the item")
                        .build(),
                    );
                }
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw42));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunOptions, run_with};
    use erc_model::{DirFiles, Fact, Facts, ImportOptions, NetClass, Param, Provenance};
    use std::path::Path;

    const SHEET: &str = "draw-42_touching_colours";

    fn themed() -> RunOptions {
        let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
        let text = std::fs::read_to_string(sources.join(format!("{SHEET}.theme.json"))).unwrap();
        let mut opts = RunOptions::default();
        opts.config.themes = vec![erc_kicad::theme::read(&text).unwrap()];
        opts
    }

    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints_with(&Draw42, SHEET, &themed()),
            [
                "DRAW-42:item Test theme: label rgb(0, 0, 194)|item Test theme: wire rgb(0, 0, 180)|sheet /",
                "DRAW-42:item Test theme: pin rgb(150, 108, 0)|item Test theme: wire rgb(150, 110, 10)|sheet /",
                "DRAW-42:item Test theme: power symbol rgb(60, 170, 24)|item Test theme: wire rgb(50, 165, 30)|sheet /",
            ]
        );
    }

    fn design() -> erc_model::Design {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
        erc_kicad::project::read(
            &DirFiles(dir),
            &format!("{SHEET}.kicad_sch"),
            &ImportOptions::default(),
        )
        .unwrap()
        .design
    }

    /// The findings with `name` a net class of `colour` on the net `net`
    fn with_class(net: &str, colour: &str) -> Vec<Finding> {
        let mut design = design();
        let board = &mut design.boards[0];
        let source = Provenance::Derived {
            rule: "test".into(),
        };
        let fact = Fact::new(colour.to_string(), source.clone());
        board.net_classes.push(NetClass {
            name: "Rail".into(),
            params: vec![Param::new("schematic_color", fact)],
            patterns: Vec::new(),
            assigned: Vec::new(),
        });
        let id = board.net_by_name(net).unwrap();
        board.nets[id.index()].class = Facts::one(Fact::new("Rail,Default".to_string(), source));
        run_with(&[&Draw42], &design, &themed()).unwrap().findings
    }

    #[test]
    fn a_net_class_colours_the_wire_and_leaves_its_label_alone() {
        let plain = with_class("/CLEAR", "rgba(0, 0, 0, 0.000)");
        assert_eq!(plain.len(), 3);
        // Gold on the wire of R2: its pin is now the theme's gold, the label takes the class colour as KiCad draws it
        let gold = with_class("/CLEAR", "rgb(150, 112, 10)");
        assert_eq!(gold.len(), 4, "{gold:#?}");
        assert!(
            gold.iter()
                .any(|f| f.message.contains("at 1 place, first R2.1 pin"))
        );
        assert!(!gold.iter().any(|f| f.message.contains("label CLEAR")));
    }

    #[test]
    fn says_it_did_not_run_without_a_theme() {
        let report = run_with(&[&Draw42], &design(), &RunOptions::default()).unwrap();
        let why: Vec<_> = report.not_run().collect();
        assert_eq!(why.len(), 1, "{why:?}");
        assert_eq!(why[0].need, "colour theme");
    }
}
