//! `probe::probe_access` on the pads of ordinary parts: passives by size, other parts by lead pitch, and never a
//! pad under a part's body. The project is written here.

use crate::probe::{LIB, symbol};
use better_erc::probe::{
    Access, AccessKind, CaseSize, NetAccess, NotCounted, ProbeAccess, ProbeOptions, Reach, Side,
    probe_access, render,
};
use std::path::PathBuf;

/// A pad of a footprint: number, net, position relative to the footprint, mm
type At = (&'static str, &'static str, f64, f64);

/// A footprint of the board, all its pads on the front but the listed through holes and bare copper
struct Part {
    reference: &'static str,
    footprint: &'static str,
    rotation: f64,
    pads: Vec<(At, Kind)>,
}

#[derive(Clone, Copy)]
enum Kind {
    Smd,
    /// Copper without a mask opening
    Covered,
    Hole,
}

fn part(reference: &'static str, footprint: &'static str, pads: &[At]) -> Part {
    Part {
        reference,
        footprint,
        rotation: 0.0,
        pads: pads.iter().map(|p| (*p, Kind::Smd)).collect(),
    }
}

/// Four leads in two rows `2 * x` apart, `pitch` apart within a row, with the nets of pads 1 to 4
fn leads(x: f64, pitch: f64, nets: [&'static str; 4]) -> [At; 4] {
    let y = pitch / 2.0;
    [
        ("1", nets[0], -x, -y),
        ("2", nets[1], -x, y),
        ("3", nets[2], x, y),
        ("4", nets[3], x, -y),
    ]
}

fn parts() -> Vec<Part> {
    let chip = |reference, footprint, half: f64, a, b| {
        part(
            reference,
            footprint,
            &[("1", a, -half, 0.0), ("2", b, half, 0.0)],
        )
    };
    let mut covered = chip("R3", "R_0402_1005Metric", 0.51, "COVERED", "GND");
    covered.pads[0].1 = Kind::Covered;
    let mut qfn = part(
        "U2",
        "QFN-24-1EP_4x4mm_P0.4mm_EP2.6x2.6mm",
        &leads(2.0, 0.4, ["QFN", "GND", "GND", "GND"]),
    );
    qfn.pads.push((("5", "QFN_EP", 0.0, 0.0), Kind::Smd));
    qfn.pads.push((("5", "QFN_EP", 0.1, 0.1), Kind::Hole));
    let exposed = |reference, rotation, lead, pad| {
        let mut p = part(
            reference,
            "SOIC-8-1EP_3.9x4.9mm_P1.27mm",
            &leads(2.5, 1.27, [lead, "GND", "GND", "GND"]),
        );
        p.pads.push((("5", pad, 0.0, 0.0), Kind::Smd));
        p.rotation = rotation;
        p
    };
    let mut dip = part("U5", "DIP-4_W7.62mm", &leads(3.81, 2.54, ["DIP"; 4]));
    dip.pads.iter_mut().for_each(|p| p.1 = Kind::Hole);
    vec![
        chip("R1", "R_0402_1005Metric", 0.51, "CHIP_0402", "GND"),
        chip("R2", "R_0201_0603Metric", 0.32, "CHIP_0201", "GND"),
        covered,
        chip("R4", "R_0402_1005Metric", 0.51, "NEAR", "FAR"),
        part("TP1", "TestPoint_Pad_D1.0mm", &[("1", "FAR", 0.0, 0.0)]),
        chip("D1", "D_SOD-523", 0.7, "SOD", "GND"),
        chip("D2", "D_Tiny", 0.3, "TINY", "GND"),
        part(
            "U1",
            "TSSOP-8_4.4x3mm_P0.65mm",
            &leads(2.9, 0.65, ["TSSOP", "GND", "GND", "GND"]),
        ),
        qfn,
        exposed("U3", 0.0, "SOIC", "SOIC_EP"),
        part(
            "U4",
            "VSSOP-8_2.3x2mm_P0.5mm",
            &leads(1.6, 0.5, ["FINE", "GND", "GND", "GND"]),
        ),
        dip,
        exposed("U6", 45.0, "TURNED", "TURNED_EP"),
    ]
}

fn schematic() -> String {
    let mut s = String::from(
        "(kicad_sch (version 20231120) (generator \"eeschema\") (generator_version \"8.0\")\n\
         (uuid \"a0000000-0000-4000-8000-000000000000\")\n(paper \"A4\")\n",
    );
    let pins: String = (1..=5)
        .map(|i| {
            format!(
                "(pin passive line (at -5.08 {} 0) (length 3.81) (name \"{i}\" (effects (font (size 1.27 1.27)))) \
                 (number \"{i}\" (effects (font (size 1.27 1.27)))))",
                -2.54 * f64::from(i - 1)
            )
        })
        .collect();
    let ic = format!(
        "(symbol \"T:IC\" (exclude_from_sim no) (in_bom yes) (on_board yes)\n\
         (property \"Reference\" \"U\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n\
         (property \"Value\" \"IC\" (at 0 0 0) (effects (font (size 1.27 1.27))))\n\
         (symbol \"IC_1_1\" {pins}))\n)\n"
    );
    let lib = LIB.trim_end().strip_suffix(')').unwrap();
    s += lib;
    s += &ic;
    for (i, p) in parts().iter().enumerate() {
        let lib_id = match p.reference.trim_end_matches(|c: char| c.is_ascii_digit()) {
            "R" | "D" => "Device:R",
            "TP" => "Connector:TestPoint",
            _ => "T:IC",
        };
        // A hole of an exposed pad shares its number, so a pin is named once
        let mut numbers: Vec<At> = p.pads.iter().map(|(at, _)| *at).collect();
        numbers.dedup_by_key(|a| a.0);
        let pins: Vec<(&str, f64, f64, &str)> = numbers
            .iter()
            .enumerate()
            .map(|(k, (number, net, _, _))| match lib_id {
                "Device:R" => (*number, 0.0, if k == 0 { 3.81 } else { -3.81 }, *net),
                "Connector:TestPoint" => (*number, 0.0, 0.0, *net),
                _ => (*number, -5.08, -2.54 * k as f64, *net),
            })
            .collect();
        let at = (25.4 * (i % 4) as f64 + 25.4, 25.4 * (i / 4) as f64 + 25.4);
        s += &symbol(i as u32 + 1, lib_id, p.reference, p.reference, at, &pins);
    }
    s + "(sheet_instances (path \"/\" (page \"1\"))))\n"
}

fn board() -> String {
    let mut s = String::from(
        r#"(kicad_pcb (version 20241229) (generator "pcbnew") (generator_version "9.0")
 (general (thickness 1.6) (legacy_teardrops no))
 (paper "A4")
 (layers (0 "F.Cu" signal) (2 "B.Cu" signal) (1 "F.Mask" user) (3 "B.Mask" user) (25 "Edge.Cuts" user))
 (setup (pad_to_mask_clearance 0))
"#,
    );
    for (i, p) in parts().iter().enumerate() {
        s += &format!(
            " (footprint \"T:{}\" (layer \"F.Cu\") (at {} 10 {})\n  \
             (property \"Reference\" \"{}\" (at 0 0 0) (layer \"F.SilkS\") (hide yes))\n",
            p.footprint,
            20 * (i + 1),
            p.rotation,
            p.reference
        );
        for ((number, net, x, y), kind) in &p.pads {
            let (kind, layers) = match kind {
                Kind::Smd => ("smd", "\"F.Cu\" \"F.Mask\""),
                Kind::Covered => ("smd", "\"F.Cu\""),
                Kind::Hole => ("thru_hole", "\"*.Cu\" \"*.Mask\""),
            };
            let drill = if kind == "thru_hole" {
                " (drill 0.3)"
            } else {
                ""
            };
            s += &format!(
                "  (pad \"{number}\" {kind} circle (at {x} {y} {}) (size 0.3 0.3){drill} (layers {layers}) (net \"/{net}\"))\n",
                p.rotation
            );
        }
        s += " )\n";
    }
    s + ")\n"
}

fn read(name: &str, opts: &ProbeOptions) -> ProbeAccess {
    let dir: PathBuf =
        std::env::temp_dir().join(format!("berc-probe-parts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("probe.kicad_sch"), schematic()).unwrap();
    std::fs::write(dir.join("probe.kicad_pcb"), board()).unwrap();
    std::fs::write(dir.join("probe.kicad_pro"), "{}\n").unwrap();
    let p = probe_access(&dir, opts).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    p
}

fn net<'p>(p: &'p ProbeAccess, name: &str) -> &'p NetAccess {
    p.nets
        .iter()
        .find(|n| n.net == format!("/{name}"))
        .unwrap_or_else(|| panic!("no net {name}: {}", render(p)))
}

/// The only access listed on the net
fn only<'p>(p: &'p ProbeAccess, name: &str) -> &'p Access {
    let access = &net(p, name).access;
    assert_eq!(access.len(), 1, "{name}: {access:?}");
    &access[0]
}

fn reach(p: &ProbeAccess, name: &str) -> (Reach, Reach) {
    (net(p, name).top, net(p, name).bottom)
}

const TOP: (Reach, Reach) = (Reach::Direct, Reach::None);
const NOWHERE: (Reach, Reach) = (Reach::None, Reach::None);

#[test]
fn passives_count_from_the_minimum_size() {
    let p = read("size", &ProbeOptions::default());
    assert_eq!(reach(&p, "CHIP_0402"), TOP, "{}", render(&p));
    let a = only(&p, "CHIP_0402");
    assert_eq!(
        (a.reference.as_str(), a.kind, a.size.as_deref(), a.pitch_mm),
        ("R1", AccessKind::PartPad, Some("0402"), None)
    );
    assert_eq!(a.sides, [Side::Top]);

    assert_eq!(reach(&p, "CHIP_0201"), NOWHERE);
    assert_eq!(
        only(&p, "CHIP_0201").not_counted,
        Some(NotCounted::Size {
            size: "0201".to_owned(),
            min: "0402".to_owned()
        })
    );
    assert!(
        render(&p).contains("R2.1 (top) not counted: size 0201 < 0402"),
        "{}",
        render(&p)
    );

    // Without a case code in the footprint name, the distance between the two pads tells
    assert_eq!(reach(&p, "SOD"), TOP);
    assert_eq!(only(&p, "SOD").pitch_mm, Some(1.4));
    assert_eq!(reach(&p, "TINY"), NOWHERE);
    assert_eq!(
        only(&p, "TINY").not_counted,
        Some(NotCounted::Pitch {
            pitch_mm: 0.6,
            min_mm: 0.8
        })
    );

    let smaller = read(
        "smaller",
        &ProbeOptions {
            passive_size_min: CaseSize::parse("0201").unwrap(),
            ..ProbeOptions::default()
        },
    );
    assert_eq!(reach(&smaller, "CHIP_0201"), TOP);
    assert_eq!(reach(&smaller, "TINY"), TOP);
}

#[test]
fn leads_count_from_the_minimum_pitch() {
    let p = read("pitch", &ProbeOptions::default());
    assert_eq!(reach(&p, "TSSOP"), TOP, "{}", render(&p));
    let a = only(&p, "TSSOP");
    assert_eq!((a.kind, a.pitch_mm), (AccessKind::PartPad, Some(0.65)));
    assert_eq!(reach(&p, "FINE"), NOWHERE);
    assert_eq!(
        only(&p, "FINE").not_counted,
        Some(NotCounted::Pitch {
            pitch_mm: 0.5,
            min_mm: 0.65
        })
    );
    assert_eq!(
        reach(&p, "DIP"),
        (Reach::Direct, Reach::Direct),
        "through holes are reached from both sides"
    );
    let finer = read(
        "finer",
        &ProbeOptions {
            part_pitch_mm_min: 0.5,
            ..ProbeOptions::default()
        },
    );
    assert_eq!(reach(&finer, "FINE"), TOP);
    assert_eq!(reach(&finer, "QFN"), NOWHERE, "under the body at any pitch");
}

#[test]
fn pads_under_the_body_never_count() {
    let p = read("body", &ProbeOptions::default());
    for name in ["QFN", "QFN_EP", "SOIC_EP", "TURNED_EP"] {
        assert_eq!(reach(&p, name), NOWHERE, "{name}: {}", render(&p));
        assert!(
            net(&p, name)
                .access
                .iter()
                .all(|a| a.not_counted == Some(NotCounted::UnderBody)),
            "{name}: {:?}",
            net(&p, name).access
        );
    }
    assert_eq!(
        net(&p, "QFN_EP").access.len(),
        2,
        "the pad and its thermal via: {:?}",
        net(&p, "QFN_EP").access
    );
    // The leads beside an exposed pad count, in a turned footprint too
    assert_eq!(reach(&p, "SOIC"), TOP);
    assert_eq!(reach(&p, "TURNED"), TOP, "{}", render(&p));
    assert_eq!(only(&p, "TURNED").pitch_mm, Some(1.27));
    assert!(
        render(&p).contains("U3.5 (top) not counted: under the body"),
        "{}",
        render(&p)
    );
}

#[test]
fn covered_copper_and_a_series_parts_own_pad_are_not_listed() {
    let p = read("series", &ProbeOptions::default());
    assert!(
        net(&p, "COVERED").access.is_empty(),
        "no mask opening: {:?}",
        net(&p, "COVERED").access
    );
    let near: Vec<_> = net(&p, "NEAR")
        .access
        .iter()
        .map(|a| (a.reference.as_str(), a.pad.as_str(), a.series.as_deref()))
        .collect();
    assert_eq!(near, [("R4", "1", None), ("TP1", "1", Some("R4"))]);
    let json = serde_json::to_string(&p).unwrap();
    assert!(json.contains("\"kind\":\"part_pad\""), "{json}");
    assert!(json.contains("\"why\":\"under_body\""), "{json}");
    assert_eq!(serde_json::from_str::<ProbeAccess>(&json).unwrap(), p);
}
