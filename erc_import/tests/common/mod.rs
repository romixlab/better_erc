//! Fixtures shared by the tests

#![allow(dead_code)]

use erc_model::MemFiles;
use std::path::Path;

/// A golden name, the importer the registry must pick, the main file and the files beside it
pub const CASES: &[(&str, &str, &str, &[&str])] = &[
    (
        "altium",
        "altium",
        "netlist_altium_edif.edf",
        &["netlist_altium_wirelist.net"],
    ),
    (
        "altium_from_wirelist",
        "altium",
        "netlist_altium_wirelist.net",
        &["netlist_altium_edif.edf"],
    ),
    ("altium_edif_only", "altium", "netlist_altium_edif.edf", &[]),
    (
        "altium_wirelist_only",
        "altium",
        "netlist_altium_wirelist.net",
        &[],
    ),
    (
        "orcad",
        "orcad",
        "netlist_orcad_pstxnet.dat",
        &["netlist_orcad_pstxprt.dat", "netlist_orcad_pstchip.dat"],
    ),
    (
        "orcad_from_pstchip",
        "orcad",
        "netlist_orcad_pstchip.dat",
        &["netlist_orcad_pstxprt.dat", "netlist_orcad_pstxnet.dat"],
    ),
    (
        "orcad_pstxnet_only",
        "orcad",
        "netlist_orcad_pstxnet.dat",
        &[],
    ),
    ("pnp_kicad", "pnp", "pnp_kicad.csv", &[]),
    ("pnp_altium_no_units", "pnp", "pnp_altium_no_units.csv", &[]),
    (
        "pnp_altium_with_units",
        "pnp",
        "pnp_altium_with_units.csv",
        &[],
    ),
    ("pnp_allegro", "pnp", "pnp_allegro.csv", &[]),
];

pub fn fixture(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

pub fn files(main: &str, others: &[&str]) -> MemFiles {
    let mut f = MemFiles::default().with(main, fixture(main));
    for o in others {
        f = f.with(*o, fixture(o));
    }
    f
}
