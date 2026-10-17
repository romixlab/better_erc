//! KiCad 10.0.6's own default values for `.kicad_pro` settings, so `pro.rs` can fold a setting that was just
//! added at its default (not merely at zero) into the per-section summary line.
//!
//! Transcribed by hand from the KiCad source (gitlab.com/kicad/code/kicad, tag `10.0.6`) rather than generated
//! from a fresh project file: this build has no headless KiCad (no Xvfb, no root to install one) and
//! `kicad-cli` has no "new project" or "project upgrade" subcommand, only `pcb upgrade` / `sch upgrade` for the
//! board and schematic files - the project file's defaults are only ever written by the GUI. Every value below
//! was cross-checked against the real fixture (`tests/fixtures/cannify_micro.{old,new}.kicad_pro`, an
//! untouched project before and after a plain open-and-save in KiCad 10.0.6): every one of the ~160 matched
//! exactly, which is also why the fixture's own diff is strong evidence these numbers are really KiCad's
//! defaults and not that project's own customization.
//!
//! Covers what actually shows up as noise when an older project is opened and saved in KiCad 10: the zone
//! defaults and footprint-field style flags (`pcbnew/board_design_settings.cpp`), the DRC and ERC rule
//! severity tables (`pcbnew/drc/drc_item.cpp`, `eeschema/erc/erc_settings.cpp`, `eeschema/erc/erc_item.cpp`),
//! the ERC pin conflict matrix (`eeschema/erc/erc_settings.cpp`, `m_defaultPinMap`) and
//! `schematic.reuse_designators` (`eeschema/schematic_settings.cpp`). Anything else (net class values, board
//! stackup, teardrop and track-width minimums, BOM presets, ...) isn't here: `pro.rs` falls back to folding
//! those only when they are JSON's own "nothing here" value, which is a weaker but still useful check.

use serde_json::{Value, json};
use std::collections::HashMap;

/// DRC rule severities (`board.design_settings.rule_severities.*`): "error" unless listed here. Default
/// overrides from `BOARD_DESIGN_SETTINGS::BOARD_DESIGN_SETTINGS`; settings keys from `pcbnew/drc/drc_item.cpp`.
const DRC_NOT_ERROR: &[(&str, &str)] = &[
    ("via_dangling", "warning"),
    ("track_dangling", "warning"),
    ("hole_to_hole", "warning"),
    ("holes_co_located", "warning"),
    ("missing_courtyard", "ignore"),
    ("copper_sliver", "warning"),
    ("connection_width", "warning"),
    ("track_not_centered_on_via", "ignore"),
    ("tuning_profile_track_geometries", "ignore"),
    ("duplicate_footprints", "warning"),
    ("missing_footprint", "warning"),
    ("extra_footprint", "warning"),
    ("footprint_symbol_mismatch", "warning"),
    ("footprint_symbol_field_mismatch", "warning"),
    ("footprint_filters_mismatch", "ignore"),
    ("net_conflict", "warning"),
    ("silk_overlap", "warning"),
    ("silk_over_copper", "warning"),
    ("silk_edge_clearance", "warning"),
    ("text_height", "warning"),
    ("text_thickness", "warning"),
    ("mirrored_text_on_front_layer", "warning"),
    ("nonmirrored_text_on_back_layer", "warning"),
    ("isolated_copper", "warning"),
    ("padstack", "warning"),
    ("footprint_type_mismatch", "ignore"),
    ("lib_footprint_issues", "warning"),
    ("lib_footprint_mismatch", "warning"),
    ("missing_tuning_profile", "warning"),
];

/// Every other DRC settings key, defaulting to "error" (same source as above)
const DRC_ERROR_KEYS: &[&str] = &[
    "shorting_items",
    "tracks_crossing",
    "clearance",
    "creepage",
    "starved_thermal",
    "copper_edge_clearance",
    "hole_clearance",
    "track_width",
    "track_angle",
    "track_segment_length",
    "annular_width",
    "drill_out_of_range",
    "microvia_drill_out_of_range",
    "courtyards_overlap",
    "malformed_courtyard",
    "invalid_outline",
    "solder_mask_bridge",
    "track_on_post_machined_layer",
    "unconnected_items",
    "length_out_of_range",
    "skew_out_of_range",
    "too_many_vias",
    "diff_pair_gap_out_of_range",
    "diff_pair_uncoupled_length_too_long",
    "items_not_allowed",
    "text_on_edge_cuts",
    "zones_intersect",
    "footprint",
    "pth_inside_courtyard",
    "npth_inside_courtyard",
    "item_on_disabled_layer",
    "unresolved_variable",
    "through_hole_pad_without_hole",
];

/// ERC rule severities (`erc.rule_severities.*`): "error" unless listed here. Default overrides from
/// `ERC_SETTINGS::ERC_SETTINGS`; settings keys from `eeschema/erc/erc_item.cpp`.
const ERC_NOT_ERROR: &[(&str, &str)] = &[
    ("endpoint_off_grid", "warning"),
    ("pin_to_pin", "warning"),
    ("similar_labels", "warning"),
    ("similar_power", "warning"),
    ("similar_label_and_power", "warning"),
    ("single_global_label", "ignore"),
    ("same_local_global_label", "warning"),
    ("ground_pin_not_ground", "warning"),
    ("isolated_pin_label", "warning"),
    ("multiple_net_names", "warning"),
    ("net_not_bus_member", "warning"),
    ("lib_symbol_issues", "warning"),
    ("lib_symbol_mismatch", "warning"),
    ("footprint_link_issues", "warning"),
    ("footprint_filter", "ignore"),
    ("no_connect_connected", "warning"),
    ("no_connect_dangling", "warning"),
    ("missing_unit", "warning"),
    ("missing_input_pin", "warning"),
    ("missing_bidi_pin", "warning"),
    ("simulation_model_issue", "ignore"),
    ("four_way_junction", "ignore"),
    ("label_multiple_wires", "warning"),
    ("unconnected_wire_endpoint", "warning"),
    ("stacked_pin_name", "warning"),
    ("field_name_whitespace", "warning"),
];

/// Every other ERC settings key, defaulting to "error" (same source as above)
const ERC_ERROR_KEYS: &[&str] = &[
    "pin_not_connected",
    "pin_not_driven",
    "power_pin_not_driven",
    "label_dangling",
    "wire_dangling",
    "bus_entry_needed",
    "duplicate_reference",
    "unit_value_mismatch",
    "different_unit_footprint",
    "different_unit_net",
    "duplicate_sheet_names",
    "hier_label_mismatch",
    "bus_definition_conflict",
    "bus_to_bus_conflict",
    "bus_to_net_conflict",
    "unannotated",
    "unresolved_variable",
    "undefined_netclass",
    "extra_units",
    "missing_power_pin",
];

pub fn table() -> HashMap<String, Value> {
    let mut d = HashMap::new();

    for (key, sev) in DRC_NOT_ERROR {
        d.insert(
            format!("board.design_settings.rule_severities.{key}"),
            json!(sev),
        );
    }
    for key in DRC_ERROR_KEYS {
        d.insert(
            format!("board.design_settings.rule_severities.{key}"),
            json!("error"),
        );
    }
    for (key, sev) in ERC_NOT_ERROR {
        d.insert(format!("erc.rule_severities.{key}"), json!(sev));
    }
    for key in ERC_ERROR_KEYS {
        d.insert(format!("erc.rule_severities.{key}"), json!("error"));
    }

    // `defaults.zones.*`, from `BOARD_DESIGN_SETTINGS::BOARD_DESIGN_SETTINGS`
    let zone_defaults: &[(&str, Value)] = &[
        ("min_clearance", json!(0.3)),
        ("min_thickness", json!(0.25)),
        ("fill_mode", json!(0)),
        ("hatch_thickness", json!(1.0)),
        ("hatch_gap", json!(1.5)),
        ("hatch_orientation", json!(0.0)),
        ("hatch_smoothing_level", json!(0)),
        ("hatch_smoothing_value", json!(0.1)),
        ("border_display_style", json!(2)),
        ("border_hatch_pitch", json!(0.5)),
        ("thermal_relief_gap", json!(0.5)),
        ("thermal_relief_spoke_width", json!(0.5)),
        ("pad_connection", json!(1)),
        ("corner_smoothing", json!(0)),
        ("corner_radius", json!(0.0)),
        ("remove_islands", json!(0)),
        ("min_island_area", json!(10.0)),
    ];
    for (key, val) in zone_defaults {
        d.insert(
            format!("board.design_settings.defaults.zones.{key}"),
            val.clone(),
        );
    }

    for key in [
        "apply_defaults_to_fp_fields",
        "apply_defaults_to_fp_text",
        "apply_defaults_to_fp_shapes",
        "apply_defaults_to_fp_dimensions",
        "apply_defaults_to_fp_barcodes",
    ] {
        d.insert(
            format!("board.design_settings.defaults.{key}"),
            json!(false),
        );
    }

    // ERC pin-to-pin conflict matrix (ELECTRICAL_PINTYPE order: In, Out, Bidi, TriState, Passive, NIC,
    // Unspecified, PowerIn, PowerOut, OpenCollector, OpenEmitter, NC); OK=0, WARNING=1, PP_ERROR=2
    d.insert(
        "erc.pin_map".to_string(),
        json!([
            [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2],
            [0, 2, 0, 1, 0, 0, 1, 0, 2, 2, 2, 2],
            [0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 2],
            [0, 1, 0, 0, 0, 0, 1, 1, 2, 1, 1, 2],
            [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 2],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2],
            [1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 2],
            [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 2],
            [0, 2, 1, 2, 0, 0, 1, 0, 2, 2, 2, 2],
            [0, 2, 0, 1, 0, 0, 1, 0, 2, 0, 0, 2],
            [0, 2, 1, 1, 0, 0, 1, 0, 2, 0, 0, 2],
            [2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
        ]),
    );

    // `eeschema/schematic_settings.cpp`: `PARAM_LAMBDA<bool>( "reuse_designators", ..., true )`
    d.insert("schematic.reuse_designators".to_string(), json!(true));

    d
}

/// Deep-equal, but numbers compare by value (`1` and `1.0` are the same default), so a KiCad project's own
/// int/float formatting quirks can't defeat the fold
pub fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| value_eq(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| value_eq(v, w)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicate_rule_keys() {
        let mut seen = std::collections::HashSet::new();
        for (k, _) in DRC_NOT_ERROR {
            assert!(seen.insert(*k), "duplicate DRC key {k}");
        }
        for k in DRC_ERROR_KEYS {
            assert!(seen.insert(*k), "duplicate DRC key {k}");
        }
        let mut seen = std::collections::HashSet::new();
        for (k, _) in ERC_NOT_ERROR {
            assert!(seen.insert(*k), "duplicate ERC key {k}");
        }
        for k in ERC_ERROR_KEYS {
            assert!(seen.insert(*k), "duplicate ERC key {k}");
        }
    }

    #[test]
    fn number_equality_ignores_int_vs_float() {
        assert!(value_eq(&json!(1), &json!(1.0)));
        assert!(!value_eq(&json!(1), &json!(2)));
    }
}
