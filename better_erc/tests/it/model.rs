//! `fp::model`: a footprint's 3D models measured against it, on footprints and STEP files the tests write; and
//! KiCad's model transform against `kicad-cli pcb export step` (skipped with a note without kicad-cli).

use better_erc::fp::model::{self, ModelOptions, Resolved};
use better_erc::fp::read_board;
use better_erc::step::write::{Cuboid, cuboids};
use std::path::{Path, PathBuf};

/// A fresh scratch directory per test
fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("berc-model-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn opts() -> ModelOptions {
    ModelOptions {
        config: None,
        ..ModelOptions::default()
    }
}

/// A two-pad SMD footprint, pads 1 mm square at x ±1.5, a courtyard of ±2.5 by ±1.5, and `model` as its
/// `(model ...)` entries
fn footprint(model: &str) -> String {
    format!(
        r#"(footprint "Pkg"
	(version 20240108)
	(generator "pcbnew")
	(layer "F.Cu")
	(attr smd)
	(fp_rect (start -2.5 -1.5) (end 2.5 1.5) (stroke (width 0.05) (type solid)) (fill none) (layer "F.CrtYd"))
	(pad "1" smd rect (at -1.5 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask"))
	(pad "2" smd rect (at 1.5 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask"))
	{model}
)
"#
    )
}

fn entry(path: &str, offset: &str, scale: &str, rotate: &str) -> String {
    format!(
        r#"(model "{path}" (offset (xyz {offset})) (scale (xyz {scale})) (rotate (xyz {rotate})))"#
    )
}

/// A body 3 by 1.6 by 1 mm standing 0.3 mm off the board, and two leads 0.8 mm square, 0.3 mm high, under the pads
fn package() -> String {
    cuboids(
        "pkg",
        &[
            Cuboid::new([-1.5, -0.8, 0.3], [1.5, 0.8, 1.3]),
            Cuboid::new([-1.9, -0.4, 0.0], [-1.1, 0.4, 0.3]),
            Cuboid::new([1.1, -0.4, 0.0], [1.9, 0.4, 0.3]),
        ],
    )
}

fn analyse(dir: &Path, model_entry: &str) -> model::ModelRun {
    let file = dir.join("Pkg.kicad_mod");
    std::fs::write(&file, footprint(model_entry)).unwrap();
    model::run(&file, &[], Some(dir), &opts(), &mut |_| {}).unwrap()
}

#[test]
fn a_package_on_its_pads() {
    let d = scratch("on-pads");
    std::fs::write(d.join("pkg.step"), package()).unwrap();
    let run = analyse(
        &d,
        &entry("${KIPRJMOD}/pkg.step", "0 0 0", "1 1 1", "0 0 0"),
    );
    let f = &run.footprints[0];
    assert!(f.findings.is_empty(), "{:?}", f.findings);
    assert!(run.is_ok());
    let m = &f.models[0];
    assert!(
        matches!(&m.resolved, Resolved::Found { vars, .. } if vars == &["KIPRJMOD"]),
        "{:?}",
        m.resolved
    );
    let a = m.analysis.as_ref().unwrap();
    assert_eq!(
        (a.unit.as_str(), a.parts, a.faces, a.faces_meshed),
        ("mm", 3, 18, 18)
    );
    let e = a.extents.unwrap();
    assert_eq!(e.min, [-1.9, -0.8, 0.0]);
    assert_eq!(e.max, [1.9, 0.8, 1.3]);
    assert_eq!((e.height, e.depth), (1.3, 0.0));
    assert_eq!(f.pads_centre, Some([0.0, 0.0]));
    assert_eq!(a.courtyard.as_ref().unwrap().overhang, 0.0);
    // Only the leads are in the bottom 0.2 mm: 0.64 mm² each, all of it over its pad
    let l = a.leads.as_ref().unwrap();
    assert!((l.area - 1.28).abs() < 0.01, "{l:?}");
    assert!(l.off_pads < 0.01, "{l:?}");
    for p in &l.pads {
        assert!(
            (p.area - 1.0).abs() < 0.01 && (p.covered - 0.64).abs() < 0.01,
            "{p:?}"
        );
    }
    let text = run.to_text_color(true);
    assert!(text.contains("3.800 x 1.600 x 1.300 mm"), "{text}");
    assert!(text.contains("2 of 2 pads under a lead"), "{text}");
    let json: serde_json::Value = serde_json::from_str(&run.to_json()).unwrap();
    assert_eq!(
        json["footprints"][0]["models"][0]["resolved"]["status"],
        "found"
    );
    std::fs::remove_dir_all(&d).unwrap();
}

/// Moved 3 mm right: the left lead over the right pad, the left pad bare, the body out of the courtyard
#[test]
fn a_package_moved_off_its_pads() {
    let d = scratch("off-pads");
    std::fs::write(d.join("pkg.step"), package()).unwrap();
    let run = analyse(
        &d,
        &entry("${KIPRJMOD}/pkg.step", "3 0 0", "1 1 1", "0 0 0"),
    );
    let f = &run.footprints[0];
    let ids: Vec<&str> = f.findings.iter().map(|x| x.check).collect();
    assert_eq!(ids, ["LIB-23", "LIB-24"], "{:?}", f.findings);
    assert_eq!(f.findings[1].message, "no lead on pad 1");
    let c = f.models[0]
        .analysis
        .as_ref()
        .unwrap()
        .courtyard
        .clone()
        .unwrap();
    assert!((c.overhang - 2.4).abs() < 1e-9, "{c:?}");
    assert!(!run.is_ok());
    std::fs::remove_dir_all(&d).unwrap();
}

/// Turned by 90° about z: the leads stand across the pads now, still on them; the extents turn with it
#[test]
fn a_package_turned() {
    let d = scratch("turned");
    std::fs::write(d.join("pkg.step"), package()).unwrap();
    let run = analyse(
        &d,
        &entry("${KIPRJMOD}/pkg.step", "0 0 0", "1 1 1", "0 0 90"),
    );
    let f = &run.footprints[0];
    let e = f.models[0].analysis.as_ref().unwrap().extents.unwrap();
    assert_eq!((e.min, e.max), ([-0.8, -1.9, 0.0], [0.8, 1.9, 1.3]));
    // The leads at y ±1.5 now: neither is over a pad at x ±1.5
    let ids: Vec<&str> = f.findings.iter().map(|x| x.check).collect();
    assert_eq!(ids, ["LIB-23", "LIB-24"], "{:?}", f.findings);
    std::fs::remove_dir_all(&d).unwrap();
}

/// A missing file, an unset variable, a scale, an odd angle, a hidden model, no model, another unit
#[test]
fn entries_that_need_a_look() {
    let d = scratch("entries");
    let missing = entry("${KIPRJMOD}/none.wrl", "0 0 0", "1 1 1", "0 0 0");
    let unset = entry("${NO_SUCH_3DMODEL_DIR}/x.step", "0 0 0", "1 1 1", "0 0 0");
    let run = analyse(&d, &format!("{missing}\n{unset}"));
    let f = &run.footprints[0];
    assert!(
        matches!(&f.models[0].resolved, Resolved::NotFound { path } if path.ends_with("none.wrl")),
        "{:?}",
        f.models[0].resolved
    );
    assert_eq!(
        f.models[1].resolved,
        Resolved::UnknownVar {
            var: "NO_SUCH_3DMODEL_DIR".into()
        }
    );
    let ids: Vec<&str> = f.findings.iter().map(|x| x.check).collect();
    assert_eq!(ids, ["LIB-22", "LIB-22"]);

    std::fs::write(d.join("pkg.step"), package()).unwrap();
    let odd = entry("${KIPRJMOD}/pkg.step", "0 0 0", "1.5 1 1", "0 0 45");
    let run = analyse(&d, &odd);
    let ids: Vec<&str> = run.footprints[0].findings.iter().map(|x| x.check).collect();
    assert!(
        ids.starts_with(&["LIB-25", "LIB-27"]),
        "{:?}",
        run.footprints[0].findings
    );

    let hidden = r#"(model "${KIPRJMOD}/pkg.step" (hide yes) (offset (xyz 0 0 0)) (scale (xyz 1 1 1)) (rotate (xyz 0 0 0)))"#;
    let run = analyse(&d, hidden);
    let ids: Vec<&str> = run.footprints[0].findings.iter().map(|x| x.check).collect();
    assert_eq!(ids, ["LIB-28"]);
    // A suggestion does not fail the run
    assert!(run.is_ok());
    let run = analyse(&d, "");
    assert_eq!(run.footprints[0].findings[0].message, "no 3D model");

    // The same package written in inches: read in mm, a suggestion
    let inch = package()
        .replace(
            "(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))",
            "(CONVERSION_BASED_UNIT('INCH',#9001)LENGTH_UNIT()NAMED_UNIT(#9002))",
        )
        .replace(
            "ENDSEC;\nEND-ISO",
            "#9000=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
             #9001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#9000);\n\
             #9002=DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\nENDSEC;\nEND-ISO",
        );
    std::fs::write(d.join("inch.step"), inch).unwrap();
    let run = analyse(
        &d,
        &entry("${KIPRJMOD}/inch.step", "0 0 0", "1 1 1", "0 0 0"),
    );
    let f = &run.footprints[0];
    let a = f.models[0].analysis.as_ref().unwrap();
    assert_eq!((a.unit.as_str(), a.unit_mm), ("inch", 25.4), "{a:?}");
    assert!((a.extents.unwrap().max[2] - 1.3 * 25.4).abs() < 1e-6);
    assert!(
        f.findings.iter().any(|x| x.check == "LIB-26"),
        "{:?}",
        f.findings
    );
    std::fs::remove_dir_all(&d).unwrap();
}

/// A library folder: every footprint, or those named; a name that is not there is an error
#[test]
fn a_library_and_its_names() {
    let d = scratch("library");
    let lib = d.join("Lib.pretty");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(d.join("pkg.step"), package()).unwrap();
    let with = entry("${KIPRJMOD}/pkg.step", "0 0 0", "1 1 1", "0 0 0");
    std::fs::write(lib.join("A.kicad_mod"), footprint(&with)).unwrap();
    std::fs::write(lib.join("B.kicad_mod"), footprint("")).unwrap();
    // KIPRJMOD is the folder the library is in
    let run = model::run(&lib, &[], None, &opts(), &mut |_| {}).unwrap();
    assert_eq!(run.footprints.len(), 2);
    assert_eq!(run.project_dir, d);
    assert!(
        run.footprints[0].findings.is_empty(),
        "{:?}",
        run.footprints[0].findings
    );
    let run = model::run(&lib, &["B".into()], None, &opts(), &mut |_| {}).unwrap();
    assert_eq!(run.footprints.len(), 1);
    assert!(model::run(&lib, &["C".into()], None, &opts(), &mut |_| {}).is_err());
    std::fs::remove_dir_all(&d).unwrap();
}

// ---------------------------------------------------------------------------------------------------------------
// KiCad as the oracle

/// A footprint on the oracle's board: reference, position, rotation, side, and its model's offset, scale, rotate
struct Placed {
    reference: &'static str,
    at: [f64; 2],
    rotation: f64,
    back: bool,
    offset: [f64; 3],
    scale: [f64; 3],
    rotate: [f64; 3],
}

const fn placed(
    reference: &'static str,
    at: [f64; 2],
    rotation: f64,
    back: bool,
    offset: [f64; 3],
    scale: [f64; 3],
    rotate: [f64; 3],
) -> Placed {
    Placed {
        reference,
        at,
        rotation,
        back,
        offset,
        scale,
        rotate,
    }
}

const ONE: [f64; 3] = [1.0; 3];
const ZERO: [f64; 3] = [0.0; 3];

/// The two references fix where KiCad puts the board's surfaces; the rest each try a setting, and several of them
/// together, at odd angles too
const BOARD: [Placed; 12] = [
    placed("F0", [10.0, 10.0], 0.0, false, ZERO, ONE, ZERO),
    placed("B0", [10.0, 90.0], 0.0, true, ZERO, ONE, ZERO),
    placed("OFS", [30.0, 10.0], 0.0, false, [1.0, 2.0, 3.0], ONE, ZERO),
    placed("RX", [50.0, 10.0], 0.0, false, ZERO, ONE, [90.0, 0.0, 0.0]),
    placed("RY", [70.0, 10.0], 0.0, false, ZERO, ONE, [0.0, 90.0, 0.0]),
    placed("RZ", [90.0, 10.0], 0.0, false, ZERO, ONE, [0.0, 0.0, 90.0]),
    placed(
        "RXYZ",
        [10.0, 30.0],
        0.0,
        false,
        ZERO,
        ONE,
        [10.0, 20.0, 30.0],
    ),
    placed(
        "FP",
        [30.0, 30.0],
        30.0,
        false,
        [0.5, -1.0, 0.2],
        ONE,
        [0.0, 0.0, 45.0],
    ),
    placed(
        "SC",
        [50.0, 30.0],
        90.0,
        false,
        [0.0, 1.0, 0.0],
        [2.0, 0.5, 1.5],
        [0.0, 0.0, 0.0],
    ),
    placed(
        "BK",
        [30.0, 90.0],
        0.0,
        true,
        [1.0, 2.0, 0.5],
        ONE,
        [0.0, 0.0, 90.0],
    ),
    placed(
        "BKR",
        [50.0, 90.0],
        30.0,
        true,
        [0.5, -1.0, 0.2],
        ONE,
        [10.0, 20.0, 30.0],
    ),
    placed(
        "BKS",
        [70.0, 90.0],
        -90.0,
        true,
        ZERO,
        [1.0, 2.0, 0.5],
        [90.0, 0.0, 0.0],
    ),
];

fn board_text(model: &Path) -> String {
    let mut s = String::from(
        r#"(kicad_pcb (version 20240108) (generator "pcbnew") (generator_version "8.0")
 (general (thickness 1.6) (legacy_teardrops no))
 (paper "A4")
 (layers (0 "F.Cu" signal) (31 "B.Cu" signal) (36 "B.SilkS" user "B.Silkscreen") (37 "F.SilkS" user "F.Silkscreen")
  (38 "B.Mask" user) (39 "F.Mask" user) (44 "Edge.Cuts" user) (46 "B.CrtYd" user "B.Courtyard")
  (47 "F.CrtYd" user "F.Courtyard"))
 (setup (pad_to_mask_clearance 0))
 (net 0 "")
 (gr_rect (start 0 0) (end 100 100) (stroke (width 0.1) (type default)) (fill none) (layer "Edge.Cuts"))
"#,
    );
    let xyz = |v: [f64; 3]| format!("{} {} {}", v[0], v[1], v[2]);
    for p in &BOARD {
        let layer = if p.back { "B.Cu" } else { "F.Cu" };
        s += &format!(
            r#" (footprint "T:Box" (layer "{layer}") (at {} {} {})
  (property "Reference" "{}" (at 0 0 0) (layer "F.SilkS") (hide yes))
  (pad "1" smd rect (at 0 0) (size 0.5 0.5) (layers "{layer}"))
  (model "{}" (offset (xyz {})) (scale (xyz {})) (rotate (xyz {}))))
"#,
            p.at[0],
            p.at[1],
            p.rotation,
            p.reference,
            model.display(),
            xyz(p.offset),
            xyz(p.scale),
            xyz(p.rotate)
        );
    }
    s + ")\n"
}

fn bounds_of(points: impl Iterator<Item = [f64; 3]>) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in points {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo, hi)
}

fn part_bounds(p: &better_erc::step::Part) -> ([f64; 3], [f64; 3]) {
    bounds_of(
        p.faces
            .iter()
            .filter_map(|f| f.mesh.as_ref())
            .flat_map(|m| m.positions.iter().copied()),
    )
}

/// An asymmetric box, so every turn and flip shows in its bounds: each model placed by `fp::model` as KiCad
/// places it, against the part kicad-cli's STEP export puts there, to 0.01 mm
#[test]
fn placement_matches_kicad_cli_step_export() {
    let d = scratch("oracle");
    let step = d.join("box.step");
    std::fs::write(
        &step,
        cuboids("box", &[Cuboid::new([0.0, 0.0, 0.0], [3.0, 2.0, 1.0])]),
    )
    .unwrap();
    let board = d.join("oracle.kicad_pcb");
    std::fs::write(&board, board_text(&step)).unwrap();
    let out = d.join("out.step");
    let export = std::process::Command::new("kicad-cli")
        .args([
            "pcb",
            "export",
            "step",
            "--no-board-body",
            "--user-origin",
            "0x0mm",
            "-f",
            "-o",
        ])
        .arg(&out)
        .arg(&board)
        .output();
    let Ok(export) = export else {
        eprintln!("kicad-cli not installed: KiCad's model placement not compared with an export");
        std::fs::remove_dir_all(&d).unwrap();
        return;
    };
    assert!(export.status.success(), "{export:?}");
    let exported =
        better_erc::step::read_file(&out, &better_erc::step::Options::default()).unwrap();
    let theirs: Vec<_> = exported.parts.iter().map(part_bounds).collect();
    assert_eq!(theirs.len(), BOARD.len(), "{theirs:?}");

    let lay = read_board(&board).unwrap();
    let mesh = better_erc::step::read_file(&step, &better_erc::step::Options::default()).unwrap();
    let ours = |reference: &str, surface_z: f64| {
        let f = lay
            .footprints
            .iter()
            .find(|f| f.reference == reference)
            .unwrap();
        let t = model::board_placement(f, surface_z).compose(&model::placement(&f.models[0]));
        bounds_of(
            mesh.parts
                .iter()
                .flat_map(|p| &p.faces)
                .filter_map(|f| f.mesh.as_ref())
                .flat_map(|m| m.positions.iter().map(|p| t.apply_point(*p))),
        )
    };
    let near = |a: &([f64; 3], [f64; 3]), b: &([f64; 3], [f64; 3])| {
        (0..3).all(|k| (a.0[k] - b.0[k]).abs() < 0.01 && (a.1[k] - b.1[k]).abs() < 0.01)
    };
    // Where the board's surfaces are: the references' boxes rest on them, x and y as ours at z 0
    let surface = |reference: &str, back: bool| {
        let o = ours(reference, 0.0);
        let t = theirs
            .iter()
            .find(|t| {
                (0..2).all(|k| (t.0[k] - o.0[k]).abs() < 0.01 && (t.1[k] - o.1[k]).abs() < 0.01)
            })
            .unwrap_or_else(|| panic!("{reference}: {o:?} not among {theirs:?}"));
        if back { t.1[2] } else { t.0[2] }
    };
    let (top, bottom) = (surface("F0", false), surface("B0", true));
    assert!(top > bottom, "top {top}, bottom {bottom}");
    for p in &BOARD {
        let o = ours(p.reference, if p.back { bottom } else { top });
        assert!(
            theirs.iter().any(|t| near(t, &o)),
            "{}: ours {o:?} not among kicad-cli's {theirs:?}",
            p.reference
        );
    }
    std::fs::remove_dir_all(&d).unwrap();
}
