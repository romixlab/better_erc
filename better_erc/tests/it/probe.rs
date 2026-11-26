//! `probe::probe_access` on a project written here: a schematic of test points, a connector and resistors joined by
//! local labels (nets `/SIG_A`, ...), and its board with the footprints on either side and vias

use better_erc::probe::{
    AccessKind, NotCounted, PowerKind, ProbeAccess, ProbeOptions, Reach, Side, probe_access, render,
};
use std::path::PathBuf;

const LIB: &str = r#"(lib_symbols
(symbol "Device:R" (pin_numbers (hide yes)) (pin_names (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "R" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "R_1_1" (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))) (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
(symbol "Connector:TestPoint" (pin_numbers (hide yes)) (pin_names (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "TP" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "TestPoint" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "TestPoint_1_1" (pin passive line (at 0 0 90) (length 0) (name "1" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))
(symbol "Connector:USB_C_Receptacle_USB2.0_16P" (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "J" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "USB_C_Receptacle_USB2.0_16P" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "USB_C_Receptacle_USB2.0_16P_1_1" (pin bidirectional line (at -5.08 0 0) (length 3.81) (name "D+" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))) (pin bidirectional line (at -5.08 -2.54 0) (length 3.81) (name "D+" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
(symbol "Connector_Generic:Conn_01x02" (exclude_from_sim no) (in_bom yes) (on_board yes)
(property "Reference" "J" (at 0 0 0) (effects (font (size 1.27 1.27))))
(property "Value" "Conn_01x02" (at 0 0 0) (effects (font (size 1.27 1.27))))
(symbol "Conn_01x02_1_1" (pin passive line (at -5.08 0 0) (length 3.81) (name "Pin_1" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))) (pin passive line (at -5.08 -2.54 0) (length 3.81) (name "Pin_2" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
)
"#;

/// A placed symbol at `x`, `y` with a label on each pin's end: (pin number, x, y relative to the symbol as the
/// library draws it, y up, label)
fn symbol(
    n: u32,
    lib: &str,
    reference: &str,
    value: &str,
    at: (f64, f64),
    pins: &[(&str, f64, f64, &str)],
) -> String {
    let (x, y) = at;
    let mut s = format!(
        "(symbol (lib_id \"{lib}\") (at {x} {y} 0) (unit 1) (exclude_from_sim no) (in_bom yes) (on_board yes) (dnp no) \
         (uuid \"a0000000-0000-4000-8000-0000000000{n:02}\")\n  \
         (property \"Reference\" \"{reference}\" (at {x} {y} 0) (effects (font (size 1.27 1.27))))\n  \
         (property \"Value\" \"{value}\" (at {x} {y} 0) (effects (font (size 1.27 1.27))))\n"
    );
    for (pin, _, _, _) in pins {
        s += &format!("  (pin \"{pin}\" (uuid \"b0000000-0000-4000-8000-0000000{n:02}0{pin}\"))\n");
    }
    s += &format!(
        "  (instances (project \"probe\" (path \"/a0000000-0000-4000-8000-000000000000\" (reference \"{reference}\") (unit 1)))))\n"
    );
    for (i, (_, px, py, label)) in pins.iter().enumerate() {
        s += &format!(
            "(label \"{label}\" (at {} {} 0) (effects (font (size 1.27 1.27)) (justify left bottom)) \
             (uuid \"c0000000-0000-4000-8000-0000000{n:02}0{i}\"))\n",
            x + px,
            y - py
        );
    }
    s
}

/// TP1 on SIG_A; R1 from SIG_A to SIG_C (series, 22 Ω); J1 on SIG_B and GND; TP2 on SIG_D; R2 from SIG_E to GND;
/// R3 from SIG_F to GND; the USB-C receptacle J4 and the 2.54 mm header J2 on SIG_G; the 1.0 mm header J3 on
/// SIG_H
fn schematic() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    s += LIB;
    let r = |a: &'static str, b: &'static str| [("1", 0.0, 3.81, a), ("2", 0.0, -3.81, b)];
    s += &symbol(
        1,
        "Connector:TestPoint",
        "TP1",
        "TestPoint",
        (20.32, 20.32),
        &[("1", 0.0, 0.0, "SIG_A")],
    );
    s += &symbol(
        2,
        "Device:R",
        "R1",
        "22",
        (40.64, 20.32),
        &r("SIG_A", "SIG_C"),
    );
    s += &symbol(
        3,
        "Connector_Generic:Conn_01x02",
        "J1",
        "Conn_01x02",
        (60.96, 20.32),
        &[("1", -5.08, 0.0, "SIG_B"), ("2", -5.08, -2.54, "GND")],
    );
    s += &symbol(
        4,
        "Connector:TestPoint",
        "TP2",
        "TestPoint",
        (20.32, 50.8),
        &[("1", 0.0, 0.0, "SIG_D")],
    );
    s += &symbol(
        5,
        "Device:R",
        "R2",
        "10k",
        (40.64, 50.8),
        &r("SIG_E", "GND"),
    );
    s += &symbol(
        6,
        "Device:R",
        "R3",
        "10k",
        (60.96, 50.8),
        &r("SIG_F", "GND"),
    );
    let conn = |a: &'static str, b: &'static str| [("1", -5.08, 0.0, a), ("2", -5.08, -2.54, b)];
    s += &symbol(
        7,
        "Connector:USB_C_Receptacle_USB2.0_16P",
        "J4",
        "USB_C",
        (20.32, 81.28),
        &conn("SIG_G", "SIG_G"),
    );
    s += &symbol(
        8,
        "Connector_Generic:Conn_01x02",
        "J2",
        "Conn_01x02",
        (40.64, 81.28),
        &conn("SIG_G", "GND"),
    );
    s += &symbol(
        9,
        "Connector_Generic:Conn_01x02",
        "J3",
        "Conn_01x02",
        (60.96, 81.28),
        &conn("SIG_H", "GND"),
    );
    s + "(sheet_instances (path \"/\" (page \"1\"))))\n"
}

/// The pad pitch of a footprint by its reference: J4 a USB-C receptacle at 0.5 mm, J3 a 1.0 mm header, the rest
/// 2.54 mm
fn pitch_of(reference: &str) -> f64 {
    match reference {
        "J4" => 0.5,
        "J3" => 1.0,
        _ => 2.54,
    }
}

/// A footprint with one pad per (number, net), each on `layers`
fn footprint(
    reference: &str,
    side: &str,
    at: (f64, f64),
    kind: &str,
    layers: &str,
    pads: &[(&str, &str)],
    property: &str,
) -> String {
    let mut s = format!(
        " (footprint \"T:{reference}\" (layer \"{side}\") (at {} {})\n  \
         (property \"Reference\" \"{reference}\" (at 0 0 0) (layer \"F.SilkS\") (hide yes))\n",
        at.0, at.1
    );
    for (i, (number, net)) in pads.iter().enumerate() {
        let drill = if kind == "thru_hole" {
            " (drill 0.8)"
        } else {
            ""
        };
        s += &format!(
            "  (pad \"{number}\" {kind} circle (at {} 0) (size 1.5 1.5){drill} (layers {layers}){property} (net \"/{net}\"))\n",
            i as f64 * pitch_of(reference)
        );
    }
    s + " )\n"
}

fn board() -> String {
    let mut s = String::from(
        r#"(kicad_pcb (version 20241229) (generator "pcbnew") (generator_version "9.0")
 (general (thickness 1.6) (legacy_teardrops no))
 (paper "A4")
 (layers (0 "F.Cu" signal) (2 "B.Cu" signal) (1 "F.Mask" user) (3 "B.Mask" user) (25 "Edge.Cuts" user))
 (setup (pad_to_mask_clearance 0))
 (gr_rect (start 0 0) (end 100 100) (stroke (width 0.1) (type default)) (fill none) (layer "Edge.Cuts"))
"#,
    );
    let smd_top = "\"F.Cu\" \"F.Mask\"";
    let smd_bottom = "\"B.Cu\" \"B.Mask\"";
    s += &footprint(
        "TP1",
        "F.Cu",
        (10.0, 10.0),
        "smd",
        smd_top,
        &[("1", "SIG_A")],
        "",
    );
    s += &footprint(
        "R1",
        "F.Cu",
        (20.0, 10.0),
        "smd",
        smd_top,
        &[("1", "SIG_A"), ("2", "SIG_C")],
        "",
    );
    s += &footprint(
        "J1",
        "F.Cu",
        (30.0, 10.0),
        "thru_hole",
        "\"*.Cu\" \"*.Mask\"",
        &[("1", "SIG_B"), ("2", "GND")],
        "",
    );
    s += &footprint(
        "TP2",
        "B.Cu",
        (10.0, 30.0),
        "smd",
        smd_bottom,
        &[("1", "SIG_D")],
        "",
    );
    s += &footprint(
        "R2",
        "F.Cu",
        (20.0, 30.0),
        "smd",
        smd_top,
        &[("1", "SIG_E"), ("2", "GND")],
        "",
    );
    s += &footprint(
        "R3",
        "F.Cu",
        (30.0, 30.0),
        "smd",
        smd_top,
        &[("1", "SIG_F"), ("2", "GND")],
        "",
    );
    s += &footprint(
        "TP9",
        "B.Cu",
        (40.0, 30.0),
        "smd",
        smd_bottom,
        &[("1", "SIG_E")],
        " (property pad_prop_testpoint)",
    );
    for (r, x, pads) in [
        ("J4", 10.0, [("1", "SIG_G"), ("2", "SIG_G")]),
        ("J2", 20.0, [("1", "SIG_G"), ("2", "GND")]),
        ("J3", 30.0, [("1", "SIG_H"), ("2", "GND")]),
    ] {
        s += &footprint(r, "F.Cu", (x, 70.0), "smd", smd_top, &pads, "");
    }
    for (i, (net, layers)) in [
        ("SIG_C", "\"F.Cu\" \"B.Cu\""),
        ("SIG_F", "\"F.Cu\" \"B.Cu\""),
        ("SIG_F", "\"F.Cu\" \"In1.Cu\""),
    ]
    .iter()
    .enumerate()
    {
        s += &format!(
            " (via (at {} 50) (size 0.6) (drill 0.3) (layers {layers}) (net \"/{net}\") (uuid \"d0000000-0000-4000-8000-00000000000{i}\"))\n",
            10 + 5 * i
        );
    }
    s + ")\n"
}

fn project(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("berc-probe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("probe.kicad_sch"), schematic()).unwrap();
    std::fs::write(dir.join("probe.kicad_pcb"), board()).unwrap();
    std::fs::write(dir.join("probe.kicad_pro"), "{}\n").unwrap();
    dir
}

fn net<'p>(p: &'p ProbeAccess, name: &str) -> &'p better_erc::probe::NetAccess {
    p.nets
        .iter()
        .find(|n| n.net == format!("/{name}"))
        .unwrap_or_else(|| panic!("no net {name}: {}", render(p)))
}

#[test]
fn sides_series_and_vias() {
    let dir = project("sides");
    let p = probe_access(&dir, &ProbeOptions::default()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(p.schematic);
    assert_eq!(p.board, "probe.kicad_pcb");
    let reach = |name: &str| (net(&p, name).top, net(&p, name).bottom);
    assert_eq!(
        reach("SIG_A"),
        (Reach::Direct, Reach::None),
        "{}",
        render(&p)
    );
    assert_eq!(reach("SIG_B"), (Reach::Direct, Reach::Direct));
    assert_eq!(reach("SIG_C"), (Reach::Series, Reach::None));
    assert_eq!(reach("SIG_D"), (Reach::None, Reach::Direct));
    assert_eq!(reach("SIG_E"), (Reach::None, Reach::Direct));
    assert_eq!(reach("SIG_F"), (Reach::None, Reach::None));

    let a = &net(&p, "SIG_A").access;
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(
        (a[0].reference.as_str(), a[0].kind, a[0].sides.as_slice()),
        ("TP1", AccessKind::TestPoint, &[Side::Top][..])
    );
    let c = &net(&p, "SIG_C").access;
    assert_eq!(c[0].series.as_deref(), Some("R1"), "{c:?}");
    let b = &net(&p, "SIG_B").access;
    assert_eq!(
        (b[0].kind, b[0].sides.as_slice()),
        (AccessKind::Connector, &[Side::Top, Side::Bottom][..])
    );
    assert_eq!(
        net(&p, "SIG_E").access[0].reference,
        "TP9",
        "a pad marked as a test point counts"
    );

    let f = &net(&p, "SIG_F").vias;
    assert_eq!(f.len(), 2);
    assert_eq!(f[0].at_mm, [15.0, 50.0]);
    assert_eq!(f[0].sides, [Side::Top, Side::Bottom]);
    assert_eq!(f[1].sides, [Side::Top], "a blind via reaches one side");
    assert_eq!(net(&p, "GND").power, Some(PowerKind::Ground));
    assert_eq!(net(&p, "SIG_A").pads, 2);
}

#[test]
fn json_round_trips() {
    let dir = project("json");
    let p = probe_access(&dir, &ProbeOptions::default()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let json = serde_json::to_string(&p).unwrap();
    assert!(json.contains("\"top\":\"direct\""), "{json}");
    assert!(json.contains("\"kind\":\"test_point\""), "{json}");
    let back: ProbeAccess = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
}

#[test]
fn no_board_is_an_error() {
    let dir = project("noboard");
    std::fs::remove_file(dir.join("probe.kicad_pcb")).unwrap();
    let e = probe_access(&dir, &ProbeOptions::default()).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(e.to_string().contains("no board file"), "{e}");
}

#[test]
fn a_board_alone_counts_marked_pads_only() {
    let dir = project("alone");
    std::fs::remove_file(dir.join("probe.kicad_sch")).unwrap();
    std::fs::remove_file(dir.join("probe.kicad_pro")).unwrap();
    let p = probe_access(&dir.join("probe.kicad_pcb"), &ProbeOptions::default()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!p.schematic);
    assert_eq!(
        (net(&p, "SIG_E").top, net(&p, "SIG_E").bottom),
        (Reach::None, Reach::Direct)
    );
    assert_eq!(
        net(&p, "SIG_A").top,
        Reach::None,
        "TP1 has no class without a schematic"
    );
    assert!(render(&p).contains("only pads marked as test points count"));
}

#[test]
fn shielded_and_fine_pitch_connectors_do_not_count() {
    let dir = project("pitch");
    let p = probe_access(&dir, &ProbeOptions::default()).unwrap();
    let wide = probe_access(
        &dir,
        &ProbeOptions {
            connector_pitch_mm_min: 0.8,
        },
    )
    .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let g = net(&p, "SIG_G");
    assert_eq!(g.top, Reach::Direct, "the header: {}", render(&p));
    let usb: Vec<_> = g.access.iter().filter(|a| a.reference == "J4").collect();
    assert_eq!(usb.len(), 2, "{:?}", g.access);
    assert!(usb.iter().all(|a| a.not_counted
        == Some(NotCounted::Kind {
            kind: "UsbC".to_owned()
        })));
    let header = g.access.iter().find(|a| a.reference == "J2").unwrap();
    assert_eq!((header.pitch_mm, &header.not_counted), (Some(2.54), &None));
    let h = net(&p, "SIG_H");
    assert_eq!(h.top, Reach::None, "{}", render(&p));
    assert_eq!(
        h.access[0].not_counted,
        Some(NotCounted::Pitch {
            pitch_mm: 1.0,
            min_mm: 1.27
        })
    );
    assert!(
        render(&p).contains("J3.1 (top) not counted: pitch 1 < 1.27 mm"),
        "{}",
        render(&p)
    );
    assert_eq!(
        net(&wide, "SIG_H").top,
        Reach::Direct,
        "a lower minimum counts it"
    );
    assert_eq!(
        net(&wide, "SIG_G")
            .access
            .iter()
            .filter(|a| a.not_counted.is_some())
            .count(),
        2
    );
}
