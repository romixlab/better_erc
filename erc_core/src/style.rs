use crate::Severity;
use ecad_file_format::Designator;
use ecad_file_format::netlist::Netlist;
use ecad_file_format::passive_value::{PassiveValueParseWarning, parse_resistance_value};

#[derive(Debug)]
pub struct StyleDiagnostic {
    pub severity: Severity,
    pub designator: Designator,
    pub kind: StyleDiagnosticKind,
}

#[derive(Debug)]
pub enum StyleDiagnosticKind {
    WrongValue(String),
    NonStandardValue(PassiveValueParseWarning),
    NoValue,
    CalculateLaterValue,
    MosfetPinsNotNamed,
}

pub fn check_style(netlist: &Netlist, diagnostics: &mut Vec<StyleDiagnostic>) {
    for (designator, component) in &netlist.components {
        if component.value.is_empty() {
            diagnostics.push(StyleDiagnostic {
                severity: Severity::Error,
                designator: designator.clone(),
                kind: StyleDiagnosticKind::NoValue,
            });
            continue;
        }
        if component.value.starts_with('?') || component.value.ends_with('?') {
            diagnostics.push(StyleDiagnostic {
                severity: Severity::Warning,
                designator: designator.clone(),
                kind: StyleDiagnosticKind::CalculateLaterValue,
            });
            continue;
        }
        if component.value == "DNM" || component.value == "DNP" {
            continue;
        }
        if designator.is_resistor() {
            match parse_resistance_value(component.value.as_str()) {
                Ok((_val, w)) => {
                    if let Some(w) = w {
                        diagnostics.push(StyleDiagnostic {
                            severity: Severity::Warning,
                            designator: designator.clone(),
                            kind: StyleDiagnosticKind::NonStandardValue(w),
                        });
                    }
                }
                Err(e) => {
                    diagnostics.push(StyleDiagnostic {
                        severity: Severity::Error,
                        designator: designator.clone(),
                        kind: StyleDiagnosticKind::WrongValue(format!("{}", e)),
                    });
                }
            }
        }
    }
    check_for_transistors_to_have_pin_names(netlist, diagnostics);
}

fn check_for_transistors_to_have_pin_names(
    netlist: &Netlist,
    diagnostics: &mut Vec<StyleDiagnostic>,
) {
    for (designator, component) in &netlist.components {
        if !designator.is_transistor() {
            continue;
        }
        let Some(lib_part) = netlist.lib_parts.get(&component.lib_source) else {
            continue;
        };
        let d = lib_part.description.to_lowercase();
        if d.contains("mosfet")
            || d.contains("p-channel")
            || d.contains("n-channel")
            || d.contains("n channel")
            || d.contains("p channel")
        {
            let mut gate_found = false;
            let mut source_found = false;
            let mut drain_found = false;
            for pin in lib_part.pins.values() {
                if pin.name.0 == "G" || pin.name.0 == "GATE" {
                    gate_found = true;
                }
                if pin.name.0 == "S" || pin.name.0 == "SOURCE" {
                    source_found = true;
                }
                if pin.name.0 == "D" || pin.name.0 == "DRAIN" {
                    drain_found = true;
                }
            }
            if !(gate_found && source_found && drain_found) {
                diagnostics.push(StyleDiagnostic {
                    severity: Severity::Warning,
                    designator: designator.clone(),
                    kind: StyleDiagnosticKind::MosfetPinsNotNamed,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::style::check_style;
    use ecad_file_format::load_kicad_netlist;
    use std::path::PathBuf;

    #[test]
    fn able_to_find_style_diagnostics() {
        // let path =
        //     PathBuf::from("/Users/roman/Downloads/test_projects/vb135a_fdcan_iso_usb_hw.net");
        // let path = PathBuf::from("/Users/roman/Downloads/test_projects/vb125_eth_fdcan_pro.net");
        // let path =
        //     PathBuf::from("/Users/roman/Downloads/test_projects/vb133_d600plus_control_board.net");
        let path = PathBuf::from("/Users/roman/Downloads/test_projects/cannify_micro.net");
        let netlist = load_kicad_netlist(&path).unwrap();
        let mut diagnostics = Vec::new();
        check_style(&netlist, &mut diagnostics);
        println!("{diagnostics:#?}");
    }
}
