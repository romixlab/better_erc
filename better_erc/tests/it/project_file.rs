//! The project file (CFG-2) on a test schematic: found beside the design, net voltages in the power analysis,
//! waivers applied, stale, expired and unmatched, `Project::waive` appending one that reads back.

use better_erc::checks::Ctx;
use better_erc::checks::project_file::waiver::WaiverStatus;
use better_erc::model::{BoardId, Provenance, Time};
use better_erc::{CheckError, CheckOptions, Project};
use std::path::{Path, PathBuf};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("project-file-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `rail-1_unpowered_pins` as `board.kicad_sch` in a fresh directory, with `toml` as `board.berc.toml`
fn design(name: &str, toml: &str) -> PathBuf {
    let dir = scratch(name);
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test_schematics/sources/rail-1_unpowered_pins.kicad_sch");
    std::fs::copy(src, dir.join("board.kicad_sch")).unwrap();
    if !toml.is_empty() {
        std::fs::write(dir.join("board.berc.toml"), toml).unwrap();
    }
    dir.join("board.kicad_sch")
}

fn opts() -> CheckOptions {
    CheckOptions {
        today: Time::parse("2026-11-25").unwrap(),
        ..CheckOptions::default()
    }
}

const FILE: &str = r#"
[nets."+3V3"]
voltage = 3.4
[nets."+5V_ISO"]
voltage = { min = 4.5, max = 5.5, nominal = 5.0 }
[nets.NOT_HERE]
voltage = 1.8

[[waiver]]
fingerprint = "RAIL-1:net +3V3_OPA|part U4"
who = "Jane Doe"
date = 2026-11-01
reason = "U4 is fitted only with its supply"

[[waiver]]
fingerprint = "RAIL-1:net /VDD_EN|part U3"
input_hash = "in1:0000000000000000"
who = "Jane Doe"
date = 2026-11-01
reason = "judged on other inputs"

[[waiver]]
fingerprint = "RAIL-1:net unconnected-(U2-VCC-Pad1)|part U2"
who = "Jane Doe"
date = 2026-11-01
reason = "was fine until November"
expires = 2026-11-24

[[waiver]]
fingerprint = "RAIL-1:net GONE|part U9"
who = "Jane Doe"
date = 2026-11-01
reason = "the part was removed"

[intent]
vibration = true
"#;

#[test]
fn net_voltages_reach_the_power_analysis() {
    let p = Project::open(&design("nets", FILE)).unwrap();
    let file = p.file.as_ref().expect("board.berc.toml is found");
    assert_eq!(file.file, "board.berc.toml");
    let config = p.config(&better_erc::Config::default());
    let board = &p.imported.design.boards[0];
    let cx = Ctx::new(&p.imported.design, BoardId(0), None, &config);
    let net = |name: &str| {
        board
            .nets
            .iter()
            .position(|n| n.name == name)
            .map(better_erc::model::NetId::from_index)
            .unwrap()
    };
    let rail = &cx.power().rails[&net("+3V3")];
    assert_eq!(rail.volts(), Some(3.4), "the project ranks above the name");
    let v = rail.voltage.resolve().unwrap();
    assert_eq!(
        v.source,
        Provenance::ProjectFile {
            file: "board.berc.toml".into(),
            key: "nets.\"+3V3\".voltage".into()
        }
    );
    let iso = &cx.power().rails[&net("+5V_ISO")];
    assert_eq!(iso.volts(), Some(5.0));
    assert_eq!(iso.range.as_ref().map(|r| r.value), Some((4.5, 5.5)));
}

#[test]
fn waivers_apply_go_stale_expire_and_report_unmatched() {
    let p = Project::open(&design("waivers", FILE)).unwrap();
    let plain = Project::open(&design("plain", "")).unwrap();
    assert!(plain.file.is_none());
    let before = plain.check(&opts()).unwrap();
    let run = p.check(&opts()).unwrap();
    let r = &run.report;
    let status = |fp: &str| {
        r.findings
            .iter()
            .find(|f| f.fingerprint == fp)
            .and_then(|f| f.waiver.as_ref())
            .map(|w| w.status.clone())
    };
    assert_eq!(
        status("RAIL-1:net +3V3_OPA|part U4"),
        Some(WaiverStatus::Applied)
    );
    assert_eq!(
        status("RAIL-1:net /VDD_EN|part U3"),
        Some(WaiverStatus::Stale {
            was: "in1:0000000000000000".into(),
            now: None
        })
    );
    assert_eq!(
        status("RAIL-1:net unconnected-(U2-VCC-Pad1)|part U2"),
        Some(WaiverStatus::Expired)
    );
    assert_eq!(r.summary.waived, 1);
    assert_eq!(r.waivers.len(), 4);
    assert_eq!(r.waivers[3].status, None);
    let notes: Vec<&str> = r
        .notes
        .iter()
        .filter(|n| n.kind == "project")
        .map(|n| n.message.as_str())
        .collect();
    assert_eq!(
        notes,
        [
            "unknown key `intent`, ignored",
            "nets.NOT_HERE: no board has a net of this name",
            "waiver of RAIL-1:net /VDD_EN|part U3 (Jane Doe, 2026-11-01) is stale, the finding stands: its inputs changed (in1:0000000000000000 then, no hash now)",
            "waiver of RAIL-1:net unconnected-(U2-VCC-Pad1)|part U2 (Jane Doe, 2026-11-01) expired on 2026-11-24, the finding stands",
            "waiver of RAIL-1:net GONE|part U9 (Jane Doe, 2026-11-01) matched no finding: the finding is gone or its subjects changed",
        ]
    );
    // The waived finding does not fail a --fail-on warning run; the stale and expired ones do
    let warn = CheckOptions {
        fail_on: better_erc::Severity::Warning,
        ..opts()
    };
    let at_least_warning = r
        .findings
        .iter()
        .filter(|f| f.severity <= better_erc::Severity::Warning)
        .count();
    assert_eq!(
        p.check(&warn).unwrap().verdict.failing + 1,
        at_least_warning
    );
    assert!(before.report.waivers.is_empty());
    let text = r.to_text();
    assert!(text.contains("[waived]"), "{text}");
    assert!(text.contains("waived by Jane Doe on 2026-11-01: U4 is fitted only with its supply"));
    assert!(text.contains("[waiver stale]"));
}

#[test]
fn waive_appends_an_entry_that_applies() {
    let path = design("waive", "# board settings\n[derating]\nresistor_p = 2.5");
    let p = Project::open(&path).unwrap();
    let run = p.check(&opts()).unwrap();
    let picked = better_erc::checks::project::select(&run.report, "RAIL-1", &["U4".into()]);
    assert_eq!(picked.len(), 1);
    let (w, file) = p
        .waive(
            picked[0],
            "Jane Doe",
            "fitted with its supply",
            None,
            opts().today,
        )
        .unwrap();
    assert_eq!(file, path.with_file_name("board.berc.toml"));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.starts_with("# board settings\n[derating]\nresistor_p = 2.5\n\n[[waiver]]\n"),
        "{text}"
    );
    let again = Project::open(&path).unwrap();
    assert_eq!(again.file.as_ref().unwrap().waivers, vec![w]);
    assert_eq!(again.file.as_ref().unwrap().config.derating.resistor_p, 2.5);
    let run = again.check(&opts()).unwrap();
    assert_eq!(run.report.summary.waived, 1);
    // By fingerprint too, and a design without a project file gets one
    let bare = design("waive-new", "");
    let p = Project::open(&bare).unwrap();
    let run = p.check(&opts()).unwrap();
    let picked =
        better_erc::checks::project::select(&run.report, "RAIL-1:net /VDD_EN|part U3", &[]);
    assert_eq!(picked.len(), 1);
    let (_, file) = p.waive(picked[0], "J", "r", None, opts().today).unwrap();
    assert!(
        std::fs::read_to_string(file)
            .unwrap()
            .starts_with("[[waiver]]\n")
    );
}

#[test]
fn a_wrong_file_is_an_error_naming_file_and_key() {
    let e = Project::open(&design("wrong", "[derating]\nresistor_p = 0\n"))
        .err()
        .expect("refused");
    assert!(matches!(e, CheckError::ProjectFile { .. }));
    let msg = e.to_string();
    assert!(
        msg.ends_with("/board.berc.toml: derating.resistor_p: a factor from 1 to 10, not 0"),
        "{msg}"
    );
}
