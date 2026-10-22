//! The canonical dump of a board that uses every kind of entity: golden text, and `dump → read → dump` stable.

use erc_model::layout::*;
use erc_model::*;

fn sch(file: &str) -> Provenance {
    Provenance::Schematic { file: file.into() }
}

fn sample() -> Design {
    let root = "root.kicad_sch";
    let mut b = Board {
        name: "sample".into(),
        source: SourceInfo {
            tool: "kicad".into(),
            version: Some("10.0".into()),
            files: vec![root.into(), "power.kicad_sch".into()],
        },
        ..Board::default()
    };
    b.sheets.push(Sheet {
        name: String::new(),
        file: root.into(),
        parent: None,
        uuid_path: "/aaaa".into(),
        page: Some("1".into()),
    });
    b.sheets.push(Sheet {
        name: "Power supply".into(),
        file: "power.kicad_sch".into(),
        parent: Some(SheetId(0)),
        uuid_path: "/aaaa/bbbb".into(),
        page: Some("2".into()),
    });
    b.symbols.push(SymbolDef {
        name: "Device:R".into(),
        description: "Resistor".into(),
        keywords: "R res".into(),
        footprint_filters: vec!["R_*".into()],
        units: 1,
        pins: vec![
            PinDef {
                number: "1".into(),
                name: "~".into(),
                unit: 0,
                body_style: 0,
                etype: EcadPinType::Passive,
                style: "line".into(),
                hidden: false,
                alternates: vec![],
            },
            PinDef {
                number: "2".into(),
                name: "~".into(),
                unit: 0,
                body_style: 0,
                etype: EcadPinType::Passive,
                style: "line".into(),
                hidden: false,
                alternates: vec![PinAlternate {
                    name: "ALT \"two\"".into(),
                    etype: EcadPinType::Output,
                    style: "inverted".into(),
                }],
            },
        ],
        fields: vec![("Value".into(), "R".into())],
        hash: Some("1a2b".into()),
        jumper_pin_groups: vec![vec!["1".into(), "2".into()]],
        ..SymbolDef::default()
    });
    b.variants.push(Variant {
        name: "Plus".into(),
        description: Some("with the extras".into()),
    });
    b.net_classes.push(NetClass {
        name: "Power".into(),
        params: vec![Param::new(
            "track_width",
            Fact::new(
                "0.5".into(),
                Provenance::KicadProject {
                    file: "s.kicad_pro".into(),
                },
            ),
        )],
        patterns: vec!["+*".into()],
    });
    b.settings.text_vars.push(Param::new(
        "REV",
        Fact::new(
            "A".into(),
            Provenance::KicadProject {
                file: "s.kicad_pro".into(),
            },
        ),
    ));
    b.nets.push(Net {
        name: "/Power supply/VIN".into(),
        kind: NetKind::Named,
        class: Facts::one(Fact::new("Power".into(), sch(root))),
        pins: vec![],
        labels: vec![],
    });
    b.nets.push(Net {
        name: "GND".into(),
        kind: NetKind::Power,
        class: Facts::default(),
        pins: vec![],
        labels: vec![],
    });
    let mut value = Facts::one(Fact::new(Value::Text("10k".into()), sch("power.kicad_sch")));
    value.set(
        Fact::new(
            Value::Quantity(Quantity {
                value: 10e3,
                unit: Unit::Ohm,
            }),
            Provenance::Derived {
                rule: "value-as-class".into(),
            },
        )
        .with_note("resistor by symbol"),
    );
    let mut fields = vec![Field::new(
        "Value",
        Fact::new("10k".into(), sch("power.kicad_sch")),
    )];
    fields.push(Field {
        name: "Mpn1".into(),
        value: Facts(vec![
            Fact::new("RC0603FR-0710KL".into(), sch("power.kicad_sch")),
            Fact::new(
                "RC0603FR-0710KL".into(),
                Provenance::Manual {
                    who: "Roman".into(),
                },
            )
            .with_rank(Rank(950))
            .with_time(Time::from_utc(2026, 10, 22, 0, 0, 0))
            .with_note("checked on the reel"),
        ]),
    });
    for (i, d) in ["R1", "R?", "R?"].iter().enumerate() {
        b.parts.push(Part {
            designator: d.to_string(),
            symbol: Some(SymbolId(0)),
            lib_id: "Device:R".into(),
            sheet: SheetId(1),
            units: vec![UnitPlacement {
                unit: 1,
                sheet: SheetId(1),
                uuid: format!("u{i}"),
                at: Some(Placement {
                    at: Pt::from_mm(10.16, 20.32),
                    rotation: 90.0,
                    mirror: Some('x'),
                }),
            }],
            fields: fields.clone(),
            value: value.clone(),
            flags: PartFlags {
                dnp: Facts::one(Fact::new(false, sch("power.kicad_sch"))),
                ..PartFlags::default()
            },
            variants: vec![PartVariant {
                variant: VariantId(0),
                flags: PartFlags {
                    dnp: Facts::one(Fact::new(true, sch("power.kicad_sch"))),
                    ..PartFlags::default()
                },
                fields: vec![Field::new(
                    "Mpn1",
                    Fact::new("other".into(), sch("power.kicad_sch")),
                )],
            }],
            pins: vec![],
            ties: vec![],
        });
        for (n, net) in [("1", NetId(0)), ("2", NetId(1))] {
            b.pins.push(PinInst {
                part: PartId(i as u32),
                number: n.into(),
                name: "~".into(),
                unit: 1,
                etype: Facts::one(Fact::new(EcadPinType::Passive, sch("power.kicad_sch"))),
                hidden: false,
                net: Some(net),
                no_connect: false,
                alt: None,
            });
        }
    }
    b.labels.push(Label {
        kind: LabelKind::Hierarchical,
        text: "VIN".into(),
        sheet: SheetId(1),
        at: Pt::from_mm(1.0, 2.0),
        shape: Some("input".into()),
        child: None,
        fields: vec![],
        net: Some(NetId(0)),
        dangling: false,
    });
    b.labels.push(Label {
        kind: LabelKind::SheetPin,
        text: "VIN".into(),
        sheet: SheetId(0),
        at: Pt::from_mm(3.0, 4.0),
        shape: Some("input".into()),
        child: Some(SheetId(1)),
        fields: vec![],
        net: Some(NetId(0)),
        dangling: false,
    });
    b.labels.push(Label {
        kind: LabelKind::Directive,
        text: "Netclass=Power".into(),
        sheet: SheetId(0),
        at: Pt::from_mm(3.0, 5.0),
        shape: None,
        child: None,
        fields: vec![("Netclass".into(), "Power".into())],
        net: None,
        dangling: true,
    });
    b.no_connects.push(NoConnect {
        sheet: SheetId(1),
        at: Pt::from_mm(5.0, 5.0),
        pin: Some(PinId(1)),
    });
    let mut lay = Layout {
        file: "sample.kicad_pcb".into(),
        thickness: Some(1_600_000),
        layers: vec![LayerDef {
            ordinal: 0,
            name: "F.Cu".into(),
            kind: "signal".into(),
            user_name: Some("Top".into()),
        }],
        stackup: vec![StackupLayer {
            name: "dielectric 1".into(),
            kind: "core".into(),
            thickness: Some(1_510_000),
            material: Some("FR4".into()),
            epsilon_r: Some(4.5),
        }],
        net_names: vec!["GND".into(), "/Power supply/VIN".into()],
        setup: vec![("pad_to_mask_clearance".into(), "0".into())],
        ..Layout::default()
    };
    lay.footprints.push(Footprint {
        reference: "R1".into(),
        part: None,
        lib_id: "R:R_0603".into(),
        value: "10k".into(),
        side: Side::Back,
        at: Pt::from_mm(100.0, 50.0),
        rotation: 180.0,
        attrs: FootprintAttrs {
            mount: Some("smd".into()),
            dnp: true,
            ..FootprintAttrs::default()
        },
        fields: vec![Field::new(
            "Mpn1",
            Fact::new(
                "X".into(),
                Provenance::Board {
                    file: "sample.kicad_pcb".into(),
                },
            ),
        )],
        pads: vec![],
        graphics: vec![
            Shape {
                layer: "B.CrtYd".into(),
                geom: Geom::Rect(Pt::from_mm(99.0, 49.0), Pt::from_mm(101.0, 51.0)),
                width: 50_000,
                filled: false,
            },
            Shape {
                layer: "B.SilkS".into(),
                geom: Geom::Arc {
                    start: Pt::new(0, 0),
                    mid: Pt::new(1, 1),
                    end: Pt::new(2, 0),
                },
                width: 120_000,
                filled: false,
            },
        ],
        texts: vec![BoardText {
            layer: "B.SilkS".into(),
            text: "R1".into(),
            at: Pt::from_mm(100.0, 48.0),
            size: 1_000_000,
            hidden: false,
            mirrored: true,
        }],
        net_tie_pad_groups: vec![],
        models: vec!["${KICAD9_3DMODEL_DIR}/R.step".into()],
    });
    for (n, net) in [("1", "/Power supply/VIN"), ("2", "GND")] {
        lay.pads.push(Pad {
            footprint: FootprintId(0),
            number: n.into(),
            kind: "smd".into(),
            shape: "roundrect".into(),
            at: Pt::from_mm(99.2, 50.0),
            rotation: 180.0,
            size: Pt::from_mm(0.8, 0.95),
            drill: None,
            layers: vec!["B.Cu".into(), "B.Paste".into(), "B.Mask".into()],
            net_name: Some(net.into()),
            net: None,
            property: Some("pad_prop_testpoint".into()),
            pin_function: None,
            pin_type: Some("passive".into()),
            solder_mask_margin: None,
            solder_paste_margin: Some(-50_000),
            solder_paste_ratio: None,
            zone_connect: None,
            primitives: vec![Shape {
                layer: "B.Cu".into(),
                geom: Geom::Poly(vec![Pt::new(0, 0), Pt::new(1, 0), Pt::new(0, 1)]),
                width: 0,
                filled: true,
            }],
        });
    }
    lay.tracks.push(Track {
        layer: "F.Cu".into(),
        start: Pt::from_mm(1.0, 1.0),
        mid: None,
        end: Pt::from_mm(2.0, 1.0),
        width: 250_000,
        net_name: Some("GND".into()),
    });
    lay.vias.push(Via {
        at: Pt::from_mm(2.0, 1.0),
        size: 600_000,
        drill: 300_000,
        layers: vec!["F.Cu".into(), "B.Cu".into()],
        kind: "through".into(),
        net_name: Some("GND".into()),
    });
    lay.zones.push(Zone {
        name: None,
        net_name: Some("GND".into()),
        layers: vec!["B.Cu".into()],
        outline: vec![Pt::new(0, 0), Pt::new(10, 0), Pt::new(10, 10)],
        priority: 1,
        connect_pads: "thermal".into(),
        clearance: Some(200_000),
        min_thickness: Some(250_000),
        thermal_gap: Some(500_000),
        thermal_bridge_width: Some(500_000),
        keepout: vec![],
        filled: true,
    });
    lay.graphics.push(Shape {
        layer: "Edge.Cuts".into(),
        geom: Geom::Circle {
            center: Pt::from_mm(50.0, 50.0),
            radius: 40_000_000,
        },
        width: 50_000,
        filled: false,
    });
    b.layout = Some(lay);
    // What every importer does at the end
    rebuild(&mut b);
    Design::single(b)
}

/// The importers' last steps
fn rebuild(b: &mut Board) {
    b.finish();
}

#[test]
fn golden_and_round_trip() {
    let d = sample();
    let text = dump::write(&d);
    let golden = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/sample.dump");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(golden, &text).unwrap();
    }
    let want = std::fs::read_to_string(golden).unwrap_or_default();
    assert_eq!(
        text, want,
        "dump differs from tests/sample.dump (BLESS=1 to update)"
    );

    let back = dump::read(&text).unwrap();
    assert_eq!(dump::write(&back), text);
    assert!(back == d, "read gives back the dumped design");
}

#[test]
fn canonicalize_is_idempotent_and_order_free() {
    let d = sample();
    let mut b = d.boards[0].clone();
    b.canonicalize();
    assert!(b == d.boards[0]);
    // Reversed arenas come out the same
    let mut r = d.boards[0].clone();
    r.labels.reverse();
    let n = r.labels.len() as u32;
    for net in &mut r.nets {
        for l in &mut net.labels {
            l.0 = n - 1 - l.0;
        }
    }
    r.canonicalize();
    assert!(r == d.boards[0]);
}

#[test]
fn read_errors_name_the_line() {
    let e = dump::read("board x\nsheet / name \"\" file a uuid \"\"\npart R1 lib X sheet /nope/\n")
        .unwrap_err();
    assert_eq!(e.line, 3);
    assert!(e.message.contains("no sheet"), "{e}");
    assert!(dump::read("part R1\n").is_err());
    assert!(dump::read("board x\nfrobnicate\n").is_err());
}
