use anyhow::Result;
use ecad_compare::pnp::compare_positions;
use ecad_file_format::pnp::ComponentPosition;
use ecad_file_format::{Designator, load_component_positions};
use std::collections::HashMap;
use std::path::Path;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let path_a = args.get(1).expect("Path A");
    let path_b = args.get(2).expect("Path B");

    let positions_a = load_component_positions(&Path::new(path_a))?;
    println!("A file info:");
    print_info(&positions_a);

    let positions_b = load_component_positions(&Path::new(path_b))?;
    println!("B file info:");
    print_info(&positions_b);

    let change_list = compare_positions(&positions_a, &positions_b);
    println!("{:?}", change_list);
    Ok(())
}

fn print_info(positions: &HashMap<Designator, ComponentPosition>) {
    println!("Component count: {}", positions.len());
    let tp_count = positions.keys().fold(
        0,
        |acc, d| if d.0.starts_with("TP") { acc + 1 } else { acc },
    );
    println!("TP count: {}", tp_count);
}
