//! What the engine reports as available per need, from the analyses in the context: part classes with their
//! coverage, rails and their voltages, drawing facts, fitted parts.

use erc_checks::{Availability, Config, Ctx, Need};
use erc_model::{BoardId, DirFiles, ImportOptions, Imported};
use std::path::Path;

fn import(dir: &str, file: &str) -> Imported {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    erc_kicad::project::read(&DirFiles(dir), file, &ImportOptions::default()).unwrap()
}

fn availability(imported: &Imported, need: Need) -> Availability {
    let config = Config::default();
    Ctx::new(&imported.design, BoardId(0), None, &config).availability(need)
}

#[test]
fn from_the_analyses() {
    let power = import("../test_schematics/analyses", "power.kicad_sch");
    assert_eq!(availability(&power, Need::Rails), Availability::Available);
    // VBAT, VBUS and the auto-named net with a power pin carry no voltage
    assert_eq!(
        availability(&power, Need::RailVoltages),
        Availability::Partial {
            have: 8,
            of: 11,
            what: "rails with a voltage".into()
        }
    );
    // U2 is a `Fixture:Sensor` with a description no rule knows: an IC by its prefix, assumed
    assert!(matches!(
        availability(&power, Need::PartClasses),
        Availability::Partial {
            have: 15,
            of: 16,
            ..
        }
    ));
    assert_eq!(availability(&power, Need::Drawing), Availability::Available);
    assert_eq!(availability(&power, Need::Variant), Availability::Available);

    let parts = import("../test_schematics/analyses", "parts.kicad_sch");
    assert!(matches!(
        availability(&parts, Need::Rails),
        Availability::Missing { .. }
    ));
    assert!(matches!(
        availability(&parts, Need::RailVoltages),
        Availability::Missing { .. }
    ));
}

#[test]
fn a_netlist_has_no_drawing() {
    let text = include_str!("../../erc_analysis/tests/fixtures/buck.net");
    let (board, _) = erc_kicad::netlist::read(text, "buck.net").unwrap();
    let imported = Imported {
        design: erc_model::Design::single(board),
        notes: Vec::new(),
    };
    assert!(matches!(
        availability(&imported, Need::Drawing),
        Availability::Missing { .. }
    ));
    assert_eq!(
        availability(&imported, Need::Variant),
        Availability::Available
    );
    assert_eq!(
        availability(&imported, Need::Rails),
        Availability::Available
    );
}
