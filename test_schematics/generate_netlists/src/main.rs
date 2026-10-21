//! Exports every test schematic in `test_schematics/sources` to a netlist and an SVG with kicad-cli, skipping
//! the ones whose cached netlist is current. The tests do the same on their first `get_netlist_path`.

fn main() {
    generate_netlists::generate_netlists();
}
