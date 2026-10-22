//! The power structure and the input pins on small designs: a buck converter with an MCU (`fixtures/buck.net`) and
//! the IO-1 test schematic, what `erc_core`'s `power.rs` and `general.rs` found, on the board model.

use erc_analysis::graph::input_pins;
use erc_analysis::{I2cBuses, PartClasses, Power};
use erc_model::{Board, DirFiles, ImportOptions};
use std::path::Path;

fn board(dir: &str, file: &str) -> Board {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let imported =
        erc_kicad::project::read(&DirFiles(dir), file, &ImportOptions::default()).unwrap();
    imported.design.boards.into_iter().next().unwrap()
}

fn names(board: &Board, nets: impl IntoIterator<Item = erc_model::NetId>) -> Vec<String> {
    nets.into_iter()
        .map(|n| board.net(n).name.clone())
        .collect()
}

#[test]
fn buck() {
    let text = include_str!("fixtures/buck.net");
    let (b, _) = erc_kicad::netlist::read(text, "buck.net").unwrap();
    let classes = PartClasses::of(&b);
    let power = Power::of(&b, &classes, &I2cBuses::of(&b, &classes));
    let rails: Vec<_> = power
        .rails
        .iter()
        .map(|(n, v)| (b.net(*n).name.clone(), v.value))
        .collect();
    assert_eq!(
        rails,
        [
            ("+1V25".to_string(), Some(1.25)),
            ("+5V".to_string(), None),
            ("/AUX".to_string(), None),
        ],
        "the I2C lines are not rails although named with a voltage, the switching node neither"
    );
    assert_eq!(names(&b, power.grounds.iter().copied()), ["GND"]);
    assert_eq!(names(&b, power.switching_nodes.iter().copied()), ["/SW"]);
    let gnd = b.net_by_name("GND").unwrap();
    assert!(power.is_power_net(gnd));
}

/// `general::able_to_find_unconnected_inputs`: U1A's +INA driven by its own output, -INA only reaching R1
#[test]
fn inputs() {
    let b = board("../test_schematics/sources", "io-1_inputs.kicad_sch");
    let found: Vec<_> = input_pins(&b)
        .into_iter()
        .map(|p| (b.net(b.pin(p).net.unwrap()).name.clone(), b.pin_name(p)))
        .collect();
    assert_eq!(
        found,
        [
            ("/SIG_DRIVEN".to_string(), "U1.3".to_string()),
            ("/SIG_FLOATING".to_string(), "U1.2".to_string())
        ]
    );
}

/// `PcbAssembly::find_part_chains` on the buck: IC to inductor through any net, and through no rail or ground
#[test]
fn part_chains() {
    use erc_analysis::PartClass;
    use erc_analysis::graph::part_chains;
    let (b, _) = erc_kicad::netlist::read(include_str!("fixtures/buck.net"), "buck.net").unwrap();
    let classes = PartClasses::of(&b);
    let power = Power::of(&b, &classes, &I2cBuses::of(&b, &classes));
    let ic = |p| classes.get(p).is_ic();
    let inductor = |p| classes.is(p, PartClass::Inductor);
    let names = |chains: Vec<Vec<erc_model::PartId>>| -> Vec<Vec<String>> {
        chains
            .into_iter()
            .map(|c| {
                c.into_iter()
                    .map(|p| b.part(p).designator.clone())
                    .collect()
            })
            .collect()
    };
    let all = part_chains(&b, &[&ic, &inductor], &|_| false);
    assert_eq!(
        names(all),
        [["U1", "L1"], ["U2", "L1"]],
        "U2 reaches L1 through +1V25"
    );
    let no_power = part_chains(&b, &[&ic, &inductor], &|n| power.is_power_net(n));
    assert_eq!(names(no_power), [["U1", "L1"]], "only through /SW");
}
