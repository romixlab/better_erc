#[cfg(test)]
mod tests {
    use ecad_file_format::kicad_netlist::load_kicad_netlist;
    use generate_netlists::get_netlist_path;

    #[test]
    fn able_to_recognize_i2c_bus() {}

    #[test]
    fn able_to_find_missing_i2c_pull_ups() {
        let path = get_netlist_path("i2c_no_pull_ups");
        let netlist = load_kicad_netlist(&path).unwrap();
        println!("{:#?}", netlist);
    }
}
