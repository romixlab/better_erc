use crate::Designator;
use crate::edif_netlist::load_edif_netlist;
use crate::netlist::Netlist;
use crate::wirelist::load_wirelist_netlist;
use anyhow::Result;
use std::path::Path;

pub fn load_altium_netlist(edif_path: &Path, wirelist_path: &Path) -> Result<Netlist> {
    let mut edif_netlist = load_edif_netlist(edif_path)?;
    let wirelsit_netlist = load_wirelist_netlist(wirelist_path)?;

    for ((_, lib_part_name), wirelist_lib_part) in wirelsit_netlist.lib_parts {
        let Some(edif_component) = edif_netlist.components.get(&Designator(lib_part_name.0)) else {
            continue;
        };
        let Some(edif_lib_part) = edif_netlist.lib_parts.get_mut(&edif_component.lib_source) else {
            continue;
        };
        for (pin_id, pin) in wirelist_lib_part.pins {
            if let Some(edif_pin) = edif_lib_part.pins.get_mut(&pin_id) {
                edif_pin.name = pin.name;
                edif_pin.default_mode = pin.default_mode;
            }
        }
    }

    Ok(edif_netlist)
}
