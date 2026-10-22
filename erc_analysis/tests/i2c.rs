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
