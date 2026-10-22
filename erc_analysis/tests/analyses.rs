//! The analyses on their test schematics: every `test_schematics/analyses/<name>.kicad_sch` with a sidecar
//! `<name>.expect.toml` holding what the analyses find on it (part classes with their source, values, rails,
//! grounds, ground groups and links, switching nodes, I2C buses, drawing facts, variant views), one readable line
//! per fact. A sub-sheet is named `<root>_child.kicad_sch` and has no sidecar of its own. The test compares;
//! `BLESS=1` writes the sidecars from what the analyses find, for review in the diff.

use erc_analysis::drawing::{Drawing, LabelScope};
use erc_analysis::i2c::Node;
use erc_analysis::value::format_si;
use erc_analysis::{I2cBuses, PartClasses, Power, Values};
use erc_model::{Board, DirFiles, ImportOptions, NetId, PartId, Unit, View};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    classes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    values: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    rails: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    grounds: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ground_groups: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ground_links: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    switching_nodes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    i2c_buses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    i2c_segments: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sheet_pins: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    no_connects: Vec<String>,
    /// Per variant: the parts it leaves out and the values it changes
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    variants: BTreeMap<String, Vec<String>>,
}

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/analyses")
}

fn net(b: &Board, n: NetId) -> String {
    b.net(n).name.clone()
}

fn designators(b: &Board, parts: impl IntoIterator<Item = PartId>) -> String {
    parts
        .into_iter()
        .map(|p| b.part(p).designator.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn values_of(b: &Board, view: View, values: &Values) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for p in view.components() {
        let v = values.get(p);
        if v.text.trim().is_empty() {
            continue;
        }
        let mut line = match v.quantity() {
            Some(q) => format!("{} = {}", v.text, format_si(q.value, q.unit)),
            None => format!("{} (text)", v.text),
        };
        if let Some(t) = v.tolerance {
            line += &format!(" ±{t}%");
        }
        for r in &v.ratings {
            line += &format!(" {}", format_si(r.value, r.unit));
        }
        if let Some(d) = &v.dielectric {
            line += &format!(" {d}");
        }
        if let Some(s) = v.series {
            line += &format!(" {}", s.name());
        } else if v.series_applies() {
            line += " in no E-series";
        }
        if !v.warnings.is_empty() {
            line += &format!(" warnings {:?}", v.warnings);
        }
        out.insert(b.part(p).designator.clone(), line);
    }
    out
}

fn actual(b: &Board) -> Expect {
    let mut e = Expect::default();
    let classes = PartClasses::of(b);
    let base = b.view(None);
    for p in base.components() {
        let f = classes.fact(p);
        e.classes.insert(
            b.part(p).designator.clone(),
            format!("{} ({})", f.value.describe(), f.source),
        );
    }
    let values = Values::of(base, &classes);
    e.values = values_of(b, base, &values);
    let power = Power::of(base, &classes, &values);
    for (n, r) in &power.rails {
        let signals: Vec<String> = r.signals.iter().map(|s| format!("{:?}", s.value)).collect();
        let volts = match r.voltage.resolve() {
            Some(v) => format!("{} ({})", format_si(v.value, Unit::Volt), v.source),
            None => "no voltage".into(),
        };
        e.rails.insert(
            net(b, *n),
            format!("{volts}, {:?}, by {}", r.kind, signals.join(" ")),
        );
    }
    for (n, g) in &power.grounds {
        let signals: Vec<String> = g.signals.iter().map(|s| format!("{:?}", s.value)).collect();
        e.grounds.insert(
            net(b, *n),
            format!("{:?}, by {}", g.kind, signals.join(" ")),
        );
    }
    for g in &power.ground_groups {
        let nets: Vec<String> = g.grounds.iter().map(|n| net(b, *n)).collect();
        let mut line = nets.join(" ");
        if !g.ties.is_empty() {
            line += &format!(", tied by {}", designators(b, g.ties.iter().copied()));
        }
        e.ground_groups.push(line);
    }
    for l in &power.ground_links {
        let nets: Vec<String> = l.grounds.iter().map(|n| net(b, *n)).collect();
        e.ground_links.push(format!(
            "{} {:?}: {}",
            b.part(l.part).designator,
            l.joint,
            nets.join(" ")
        ));
    }
    e.switching_nodes = power.switching_nodes.iter().map(|n| net(b, *n)).collect();

    let i2c = I2cBuses::on(base, &classes);
    for bus in &i2c.buses {
        let mut line = format!("{}: {} {}", bus.name, net(b, bus.scl), net(b, bus.sda));
        if let Some(pu) = bus.pull_up {
            line += &format!(
                ", pull-up {} to {}",
                designators(b, [pu.scl, pu.sda]),
                net(b, pu.rail)
            );
        }
        for node in &bus.nodes {
            line += &match *node {
                Node::Device(p) => format!(", device {}", b.part(p).designator),
                Node::Connector(p) => format!(", connector {}", b.part(p).designator),
                Node::TestPoint(p) => format!(", test point {}", b.part(p).designator),
                Node::Translator { part, other_side } => format!(
                    ", translator {} to {}",
                    b.part(part).designator,
                    i2c.buses[other_side].name
                ),
                Node::DiscreteTranslator {
                    scl_fet,
                    sda_fet,
                    other_side,
                    scl_gate,
                    sda_gate,
                } => format!(
                    ", discrete translator {} to {} (gates {} {})",
                    designators(b, [scl_fet, sda_fet]),
                    i2c.buses[other_side].name,
                    scl_gate.map_or("-".into(), |n| net(b, n)),
                    sda_gate.map_or("-".into(), |n| net(b, n)),
                ),
                Node::Tie {
                    scl_tie,
                    sda_tie,
                    other_side,
                } => format!(
                    ", tie {} to {}",
                    designators(b, [scl_tie, sda_tie]),
                    i2c.buses[other_side].name
                ),
                Node::Unknown(p) => format!(", unknown {}", b.part(p).designator),
            };
        }
        e.i2c_buses.push(line);
    }
    for s in &i2c.direct_segments {
        let names: Vec<&str> = i2c.segment_names(s).into_iter().collect();
        let pulled = i2c.pull_ups(s).len();
        e.i2c_segments
            .push(format!("{}: {pulled} pull-up pair(s)", names.join(" ")));
    }

    if Drawing::available(b) {
        let d = Drawing::of(b);
        for (i, l) in b.labels.iter().enumerate() {
            let id = erc_model::LabelId::from_index(i);
            let scope = match d.scope(id) {
                LabelScope::Sheet(s) => format!("sheet {}", b.sheet_path(s)),
                LabelScope::Global => "global".into(),
                LabelScope::Hierarchical { sheet, parent } => format!(
                    "hierarchical {} to {}",
                    b.sheet_path(sheet),
                    parent.map_or("-".into(), |p| b.sheet_path(p))
                ),
                LabelScope::SheetPin { parent, child } => format!(
                    "sheet pin in {} to {}",
                    b.sheet_path(parent),
                    child.map_or("-".into(), |c| b.sheet_path(c))
                ),
                LabelScope::Directive(s) => format!("directive in {}", b.sheet_path(s)),
            };
            let mut line = format!(
                "{} {} `{}`: {scope}, net {}",
                b.sheet_path(l.sheet),
                l.kind.name(),
                l.text,
                l.net.map_or("-".into(), |n| net(b, n))
            );
            if l.dangling {
                line += ", dangling";
            }
            e.labels.push(line);
        }
        e.labels.sort();
        for (p, h) in &d.sheet_pin_pairs {
            e.sheet_pins.push(format!(
                "{} pin `{}` matches {} `{}`",
                b.sheet_path(b.label(*p).sheet),
                b.label(*p).text,
                b.sheet_path(b.label(*h).sheet),
                b.label(*h).text
            ));
        }
        for p in &d.unmatched_sheet_pins {
            e.sheet_pins.push(format!(
                "{} pin `{}` has no hierarchical label",
                b.sheet_path(b.label(*p).sheet),
                b.label(*p).text
            ));
        }
        for h in &d.unmatched_hierarchical {
            e.sheet_pins.push(format!(
                "{} hierarchical label `{}` has no sheet pin",
                b.sheet_path(b.label(*h).sheet),
                b.label(*h).text
            ));
        }
        let connected: Vec<usize> = d.flagged_but_connected(b).map(|f| f.index).collect();
        for f in &d.no_connects {
            let mut line = format!(
                "{} {}",
                b.sheet_path(f.sheet),
                f.pin
                    .map_or("on no pin".into(), |p| format!("on {}", b.pin_name(p)))
            );
            if connected.contains(&f.index) {
                line += ", but the pin is connected";
            }
            e.no_connects.push(line);
        }
        e.no_connects.sort();
    }

    for (vi, v) in b.variants.iter().enumerate() {
        let view = b.view(Some(erc_model::VariantId::from_index(vi)));
        let vvalues = Values::of(view, &classes);
        let mut lines = Vec::new();
        for p in view.components() {
            let d = &b.part(p).designator;
            if base.fitted(p) && !view.fitted(p) {
                lines.push(format!("{d} not fitted"));
            }
            if !base.fitted(p) && view.fitted(p) {
                lines.push(format!("{d} fitted"));
            }
            if values.get(p).text != vvalues.get(p).text {
                let vv = vvalues.get(p);
                lines.push(format!(
                    "{d} value {} ({})",
                    vv.text,
                    vv.source.as_ref().map_or("-".into(), |s| s.to_string())
                ));
            }
        }
        e.variants.insert(v.name.clone(), lines);
    }
    e
}

#[test]
fn analyses_match_their_expectations() {
    let dir = dir();
    let bless = std::env::var_os("BLESS").is_some();
    // Root schematics; a sub-sheet is named `<root>_child.kicad_sch`
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter_map(|n| n.strip_suffix(".kicad_sch").map(str::to_string))
        .filter(|n| !n.ends_with("_child"))
        .collect();
    names.sort();
    assert!(!names.is_empty());
    let mut failed = Vec::new();
    for name in &names {
        let imported = erc_kicad::project::read(
            &DirFiles(dir.clone()),
            &format!("{name}.kicad_sch"),
            &ImportOptions::default(),
        )
        .unwrap();
        let board = &imported.design.boards[0];
        let got = actual(board);
        let sidecar = dir.join(format!("{name}.expect.toml"));
        if bless {
            let text = format!(
                "# What the analyses find on {name}.kicad_sch (erc_analysis/tests/analyses.rs; BLESS=1 rewrites it)\n\n{}",
                toml::to_string_pretty(&got).unwrap()
            );
            std::fs::write(&sidecar, text).unwrap();
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&sidecar) else {
            failed.push(format!(
                "{name}.kicad_sch: no {name}.expect.toml (BLESS=1 writes it)"
            ));
            continue;
        };
        let want: Expect =
            toml::from_str(&text).unwrap_or_else(|e| panic!("{name}.expect.toml: {e}"));
        if want != got {
            failed.push(format!(
                "{name}: the analyses differ from {name}.expect.toml (BLESS=1 rewrites it):\n{}",
                diff(
                    &toml::to_string(&want).unwrap(),
                    &toml::to_string(&got).unwrap()
                )
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n\n"));
}

/// The lines only in `want` (`-`) and only in `got` (`+`)
fn diff(want: &str, got: &str) -> String {
    let w: Vec<&str> = want.lines().collect();
    let g: Vec<&str> = got.lines().collect();
    let mut out = String::new();
    for l in &w {
        if !g.contains(l) {
            out += &format!("- {l}\n");
        }
    }
    for l in &g {
        if !w.contains(l) {
            out += &format!("+ {l}\n");
        }
    }
    out
}
