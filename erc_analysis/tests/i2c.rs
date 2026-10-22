//! The I2C recogniser on the test schematics: the bus structure `erc_core`'s `i2c.rs` tests asserted, found on the
//! board model.

use erc_analysis::graph::net_parts;
use erc_analysis::{I2cBuses, PartClasses};
use erc_model::{Board, DirFiles, ImportOptions};
use std::collections::BTreeSet;
use std::path::Path;

fn board(name: &str) -> Board {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
    let imported = erc_kicad::project::read(
        &DirFiles(dir),
        &format!("{name}.kicad_sch"),
        &ImportOptions::default(),
    )
    .unwrap();
    imported.design.boards.into_iter().next().unwrap()
}

fn names<'a>(buses: &'a I2cBuses, segments: &[BTreeSet<usize>]) -> Vec<BTreeSet<&'a str>> {
    segments.iter().map(|s| buses.segment_names(s)).collect()
}

fn set<'a>(names: &[&'a str]) -> BTreeSet<&'a str> {
    names.iter().copied().collect()
}

#[test]
fn segments() {
    let b = board("i2c-7_segments");
    let buses = I2cBuses::of(&b, &PartClasses::of(&b));
    let direct = names(&buses, &buses.direct_segments);
    for s in [
        set(&["/I2C1_1V8"]),
        set(&["/I2C1_3V3_VCC2"]),
        set(&["/I2C1_3V3", "/I2C1_3V3_to_U408", "/I2C_U7"]),
        set(&["/I2C1_1V2"]),
        set(&["/I2C2_3V3"]),
    ] {
        assert!(direct.contains(&s), "{s:?} not in {direct:?}");
    }
    let same = names(&buses, &buses.same_bus_segments);
    for s in [
        set(&["/I2C2_3V3"]),
        set(&[
            "/I2C_U7",
            "/I2C1_1V8",
            "/I2C1_3V3_VCC2",
            "/I2C1_3V3_to_U408",
            "/I2C1_3V3",
            "/I2C1_1V2",
        ]),
    ] {
        assert!(same.contains(&s), "{s:?} not in {same:?}");
    }
}

#[test]
fn multiple_pull_ups() {
    let b = board("i2c-2_multiple_pull_ups");
    let buses = I2cBuses::of(&b, &PartClasses::of(&b));
    let bus = buses.by_name("/I2C1_1V8").unwrap();
    let pu = bus.pull_up.unwrap();
    let mut all: BTreeSet<_> = ["R507", "R508", "R509", "R510", "R511"]
        .map(|d| b.part_by_designator(d).unwrap())
        .into();
    all.remove(&pu.scl);
    all.remove(&pu.sda);
    assert_eq!(bus.redundant_pull_ups, all);
    assert!(net_parts(&b, &[pu.rail]).contains(&pu.scl));
}

/// The BSS138 pairs with their gates, the ferrite ties to the connector, the pull-ups per segment, and a fitted
/// pull-up made DNP
#[test]
fn discrete_translator() {
    use erc_analysis::i2c::Node;
    let b = board("i2c-18_discrete_translator");
    let classes = PartClasses::of(&b);
    let buses = I2cBuses::of(&b, &classes);
    let direct = names(&buses, &buses.direct_segments);
    assert_eq!(
        direct,
        [
            set(&["/I2C_1V8"]),
            set(&["/I2C_3V3"]),
            set(&["/I2C_5V", "/I2C_CONN"]),
        ]
    );
    let same = names(&buses, &buses.same_bus_segments);
    assert_eq!(
        same,
        [set(&["/I2C_1V8", "/I2C_3V3", "/I2C_5V", "/I2C_CONN"])]
    );
    let pulled: Vec<(String, usize)> = buses
        .direct_segments
        .iter()
        .map(|s| {
            (
                buses
                    .segment_names(s)
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" "),
                buses.pull_ups(s).len(),
            )
        })
        .collect();
    assert_eq!(
        pulled,
        [
            ("/I2C_1V8".to_string(), 0),
            ("/I2C_3V3".to_string(), 1),
            ("/I2C_5V /I2C_CONN".to_string(), 1)
        ]
    );
    let bus = buses.by_name("/I2C_3V3").unwrap();
    let mut gates: Vec<(String, String, String)> = bus
        .nodes
        .iter()
        .filter_map(|n| match *n {
            Node::DiscreteTranslator {
                scl_fet,
                other_side,
                scl_gate,
                sda_gate,
                ..
            } => {
                assert_eq!(scl_gate, sda_gate);
                Some((
                    b.part(scl_fet).designator.clone(),
                    buses.buses[other_side].name.clone(),
                    b.net(scl_gate?).name.clone(),
                ))
            }
            _ => None,
        })
        .collect();
    gates.sort();
    assert_eq!(
        gates,
        [
            ("Q1".to_string(), "/I2C_5V".to_string(), "+3V3".to_string()),
            ("Q3".to_string(), "/I2C_1V8".to_string(), "+1V8".to_string()),
        ]
    );
    // Without R1 fitted the 3.3 V segment has no pull-up pair
    let mut b2 = b.clone();
    let r1 = b2.part_by_designator("R1").unwrap();
    b2.parts[r1.index()].flags.dnp = erc_model::Facts::one(erc_model::Fact::new(
        true,
        erc_model::Provenance::Manual { who: "test".into() },
    ));
    let buses = I2cBuses::of(&b2, &PartClasses::of(&b2));
    assert!(buses.by_name("/I2C_3V3").unwrap().pull_up.is_none());
}
