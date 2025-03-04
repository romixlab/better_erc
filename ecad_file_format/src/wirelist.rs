// use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar/wirelist.pest"]
#[allow(dead_code)]
struct WireListParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;
    use std::fs::read_to_string;

    #[test]
    fn can_read_altium_wirelist_netlist() {
        let contents = read_to_string("test_input/netlist_altium_wirelist.net").unwrap();
        let p = match WireListParser::parse(Rule::file, &contents) {
            Ok(p) => p,
            Err(e) => panic!("{}", e),
        };
        println!("{:#?}", p);
    }
}
