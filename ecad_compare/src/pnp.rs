use ecad_file_format::Designator;
use ecad_file_format::pnp::ComponentPosition;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct PositionChangeList {
    pub changed: Vec<Designator>,
    pub removed: Vec<Designator>,
    pub added: Vec<Designator>,
}

pub fn compare_positions(
    positions_a: &HashMap<Designator, ComponentPosition>,
    positions_b: &HashMap<Designator, ComponentPosition>,
) -> PositionChangeList {
    let mut change_list = PositionChangeList::default();
    for (designator_a, pos_a) in positions_a.iter() {
        if let Some(pos_b) = positions_b.get(designator_a) {
            if pos_a != pos_b {
                change_list.changed.push(designator_a.clone());
            }
        } else {
            change_list.removed.push(designator_a.clone());
        }
    }
    for (designator_b, _pos_b) in positions_b.iter() {
        if !positions_a.contains_key(designator_b) {
            change_list.added.push(designator_b.clone());
        }
    }
    change_list
}

#[cfg(test)]
mod tests {
    use crate::pnp::compare_positions;
    use ecad_file_format::Designator;
    use ecad_file_format::pnp::load_component_positions;
    use std::path::Path;

    #[test]
    fn compare_positions_works() {
        let positions_a =
            load_component_positions(&Path::new("../ecad_file_format/test_input/pnp_allegro.csv"))
                .unwrap();
        let positions_b = load_component_positions(&Path::new(
            "../ecad_file_format/test_input/pnp_altium_no_units.csv",
        ))
        .unwrap();
        let change_list = compare_positions(&positions_a, &positions_b);
        assert_eq!(change_list.changed[0], Designator("R1".into()));
        assert_eq!(change_list.removed[0], Designator("C1".into()));
        assert_eq!(change_list.added[0], Designator("J1".into()));
    }
}
