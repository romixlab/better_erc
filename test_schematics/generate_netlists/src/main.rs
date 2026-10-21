//! Exports every test schematic in `test_schematics/sources` to a netlist and an SVG with kicad-cli, skipping
//! the ones whose cached netlist is current. erc_core's build script does the same before its tests.

fn main() {
    generate_netlists::generate_netlists();
}
