#[cfg(test)]
mod tests {
    use generate_netlists::get_netlist_path;

    #[test]
    fn able_to_find_missing_i2c_pull_ups() {
        let path = get_netlist_path("i2c_no_pull_ups");
        let contents = std::fs::read_to_string(path).unwrap();
        println!("{}", contents);
    }
}
