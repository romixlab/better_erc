//! Exports every test schematic in `test_schematics/sources` to a netlist and an SVG with kicad-cli, skipping
//! the ones whose cached netlist and picture are current. The tests export only the netlists, on their first
//! `get_netlist_path`: the committed pictures change only here.

fn main() {
    generate_netlists::generate_netlists();
}
