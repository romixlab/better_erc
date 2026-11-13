//! `berc fp model`: a footprint's 3D models measured against it, through the binary, on files the test writes

use better_erc::step::write::{Cuboid, cuboids};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("berc-fp-model-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn berc(args: &[&str], usage: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_berc"));
    c.args(args)
        .env_remove("COMPLETE")
        // No KiCad configuration of the host: kicad_common.json is not read
        .env("KICAD_CONFIG_HOME", "/nonexistent")
        .env("NO_COLOR", "1");
    match usage {
        Some(f) => c.env("BERC_USAGE_FILE", f).env_remove("BERC_USAGE"),
        None => c.env("BERC_USAGE", "off"),
    };
    c.output().unwrap()
}

fn footprint(name: &str, offset: &str) -> String {
    format!(
        r#"(footprint "{name}"
	(version 20240108)
	(generator "pcbnew")
	(layer "F.Cu")
	(attr smd)
	(fp_rect (start -2.5 -1.5) (end 2.5 1.5) (stroke (width 0.05) (type solid)) (fill none) (layer "F.CrtYd"))
	(pad "1" smd rect (at -1.5 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask"))
	(pad "2" smd rect (at 1.5 0) (size 1 1) (layers "F.Cu" "F.Paste" "F.Mask"))
	(model "${{KIPRJMOD}}/pkg.step" (offset (xyz {offset})) (scale (xyz 1 1 1)) (rotate (xyz 0 0 0)))
)
"#
    )
}

/// A library of two footprints, one with its model on its pads and one with it moved off them
fn library(d: &Path) -> PathBuf {
    std::fs::write(
        d.join("pkg.step"),
        cuboids(
            "pkg",
            &[
                Cuboid::new([-1.5, -0.8, 0.3], [1.5, 0.8, 1.3]),
                Cuboid::new([-1.9, -0.4, 0.0], [-1.1, 0.4, 0.3]),
                Cuboid::new([1.1, -0.4, 0.0], [1.9, 0.4, 0.3]),
            ],
        ),
    )
    .unwrap();
    let lib = d.join("Lib.pretty");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(lib.join("Good.kicad_mod"), footprint("Good", "0 0 0")).unwrap();
    std::fs::write(lib.join("Moved.kicad_mod"), footprint("Moved", "3 0 0")).unwrap();
    lib
}

fn text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn a_library_its_footprints_and_exit_codes() {
    let d = scratch("lib");
    let lib = library(&d);
    let lib_s = lib.to_str().unwrap();
    let good = berc(&["fp", "model", lib_s, "Good"], None);
    assert_eq!(good.status.code(), Some(0), "{good:?}");
    let out = text(&good);
    assert!(out.contains("3.800 x 1.600 x 1.300 mm"), "{out}");
    assert!(out.contains("height    1.300 mm above the board"), "{out}");
    assert!(out.contains("1 footprint, 1 model, no findings"), "{out}");

    let all = berc(&["fp", "model", lib_s, "--details"], None);
    assert_eq!(all.status.code(), Some(1), "{all:?}");
    let out = text(&all);
    assert!(out.contains("LIB-24 no lead on pad 1"), "{out}");
    // --details: each pad's share under a lead
    assert!(out.contains("pad 1      0.0 %"), "{out}");
    assert!(out.contains("pad 2     64.0 %"), "{out}");
    assert!(out.contains("findings: LIB-23 1, LIB-24 1"), "{out}");

    let json = berc(&["fp", "model", lib_s, "--format", "json"], None);
    let v: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(v["footprints"].as_array().unwrap().len(), 2);
    assert_eq!(v["footprints"][1]["findings"][1]["check"], "LIB-24");
    assert_eq!(
        v["footprints"][0]["models"][0]["analysis"]["extents"]["height"],
        1.3
    );

    // One file; a name that is not in the library, a reference given to a single file: usage errors
    let one = berc(
        &["fp", "model", lib.join("Good.kicad_mod").to_str().unwrap()],
        None,
    );
    assert_eq!(one.status.code(), Some(0), "{one:?}");
    let missing = berc(&["fp", "model", lib_s, "Nope"], None);
    assert_eq!(missing.status.code(), Some(2), "{missing:?}");
    let refd = berc(
        &[
            "fp",
            "model",
            lib.join("Good.kicad_mod").to_str().unwrap(),
            "U1",
        ],
        None,
    );
    assert_eq!(refd.status.code(), Some(2), "{refd:?}");
    let nothing = berc(&["fp", "model", d.to_str().unwrap()], None);
    assert_eq!(nothing.status.code(), Some(2), "{nothing:?}");
    assert!(
        String::from_utf8_lossy(&nothing.stderr).contains("a project with a board"),
        "{nothing:?}"
    );
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn usage_is_recorded() {
    let d = scratch("usage");
    let lib = library(&d);
    let file = d.join("usage.tsv");
    berc(
        &["fp", "model", lib.to_str().unwrap(), "--details"],
        Some(&file),
    );
    let lines = std::fs::read_to_string(&file).unwrap();
    let sigs: Vec<&str> = lines
        .lines()
        .map(|l| l.split('\t').nth(1).unwrap())
        .collect();
    assert_eq!(sigs, ["fp model"]);
    std::fs::remove_dir_all(&d).unwrap();
}
