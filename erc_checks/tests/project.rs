//! The library entry `berc check` stands on: open a path, run, compare with a baseline, decide the verdict

use erc_checks::{
    BaselineState, CheckError, CheckOptions, Exit, Format, Need, Project, Severity, read_baseline,
};
use std::path::Path;

fn sources() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test_schematics/sources"
    ))
}

#[test]
fn open_check_and_verdict() {
    let p = Project::open(&sources().join("i2c-1_no_pull_ups.kicad_sch")).unwrap();
    assert_eq!(
        (p.main.as_str(), p.importer),
        ("i2c-1_no_pull_ups.kicad_sch", "kicad")
    );
    let run = p.check(&CheckOptions::default()).unwrap();
    assert_eq!(run.report.findings.len(), 3);
    assert_eq!(run.verdict.exit, Exit::Findings);
    assert_eq!((run.verdict.failing, run.verdict.exit.code()), (3, 1));

    let required = p
        .check(&CheckOptions {
            require: vec![Need::Layout, Need::Graph],
            ..CheckOptions::default()
        })
        .unwrap();
    assert_eq!(required.verdict.exit, Exit::Unavailable);
    assert_eq!(required.verdict.unavailable.len(), 1);
    assert_eq!(required.verdict.unavailable[0].need, "layout");

    // Against itself: nothing new, nothing fails, nothing fixed
    let again = p
        .check(&CheckOptions {
            baseline: Some(run.report.clone()),
            ..CheckOptions::default()
        })
        .unwrap();
    assert_eq!(again.verdict.exit, Exit::Clean);
    let d = again.report.baseline.as_ref().unwrap();
    assert_eq!((d.new, d.unchanged, d.fixed.len()), (0, 3, 0));
    assert!(
        again
            .render(Format::Json, false)
            .contains("\"baseline\": \"unchanged\"")
    );
}

#[test]
fn compare_counts_repeated_fingerprints() {
    let p = Project::open(&sources().join("i2c-1_no_pull_ups.kicad_sch")).unwrap();
    let run = p.check(&CheckOptions::default()).unwrap();
    // The baseline has the first finding twice and lacks the last one
    let mut baseline = run.report.clone();
    let first = baseline.findings[0].clone();
    let last = baseline.findings.pop().unwrap();
    baseline.findings.push(first.clone());
    let mut report = run.report.clone();
    report.compare(&baseline);
    let states: Vec<_> = report
        .findings
        .iter()
        .map(|f| f.baseline.unwrap())
        .collect();
    assert_eq!(
        states,
        [
            BaselineState::Unchanged,
            BaselineState::Unchanged,
            BaselineState::New
        ]
    );
    let d = report.baseline.unwrap();
    assert_eq!((d.new, d.unchanged), (1, 2));
    assert_eq!(d.fixed, vec![first]);
    assert_ne!(d.fixed[0].fingerprint, last.fingerprint);
}

#[test]
fn fail_on_and_errors() {
    let p = Project::open(&sources().join("i2c-10_non_equal_pull_ups.kicad_sch")).unwrap();
    let only = |fail_on| CheckOptions {
        run: erc_checks::RunOptions {
            only: vec!["I2C-10".into()],
            ..Default::default()
        },
        fail_on,
        ..CheckOptions::default()
    };
    assert_eq!(
        p.check(&only(Severity::Error)).unwrap().verdict.exit,
        Exit::Clean
    );
    assert_eq!(
        p.check(&only(Severity::Warning)).unwrap().verdict.exit,
        Exit::Findings
    );
    assert!(matches!(
        Project::open(Path::new("/nonexistent")),
        Err(CheckError::NotFound(_))
    ));
    assert!(matches!(
        read_baseline(Path::new("/nonexistent.json")),
        Err(CheckError::Baseline { .. })
    ));
    assert_eq!(CheckError::EXIT_CODE, 2);
}

#[test]
fn sarif_uri_base() {
    use erc_checks::project::uri_base;
    assert_eq!(uri_base(Path::new(".")), "");
    assert_eq!(uri_base(Path::new("hw/my board")), "hw/my%20board/");
    assert_eq!(uri_base(Path::new("./hw/b/")), "hw/b/");
    let cwd = std::env::current_dir().unwrap();
    assert_eq!(uri_base(&cwd.join("x")), "x/");
    assert_eq!(
        uri_base(Path::new("/elsewhere/b c")),
        "file:///elsewhere/b%20c/"
    );
}

/// The app's file pickers: an Altium EDIF and its WireList from two directories make one design
#[test]
fn open_files_from_two_directories() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../erc_import/tests/fixtures");
    let elsewhere = Path::new(env!("CARGO_TARGET_TMPDIR")).join("open_files");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let wirelist = elsewhere.join("netlist_altium_wirelist.net");
    std::fs::copy(fixtures.join("netlist_altium_wirelist.net"), &wirelist).unwrap();
    let edif = elsewhere.join("edif/netlist_altium_edif.edf");
    std::fs::create_dir_all(edif.parent().unwrap()).unwrap();
    std::fs::copy(fixtures.join("netlist_altium_edif.edf"), &edif).unwrap();
    let alone = Project::open(&edif).unwrap();
    let both = Project::open_files(&edif, &[wirelist]).unwrap();
    assert_eq!(both.importer, "altium");
    assert_eq!(both.main, "netlist_altium_edif.edf");
    let footprint = |p: &Project| {
        let b = &p.imported.design.boards[0];
        b.part(b.part_by_designator("J2").unwrap())
            .field("Footprint")
            .map(String::from)
    };
    assert_eq!(
        footprint(&both).as_deref(),
        Some("HEADER-2.54-2x3-VSMT"),
        "from the WireList"
    );
    let run = both.check(&CheckOptions::default()).unwrap();
    assert_eq!(run.importer, "altium");
    assert_eq!(footprint(&alone), None, "no WireList beside the EDIF");
    let missing = Project::open_files(&edif, &[elsewhere.join("nothing.net")]);
    assert!(matches!(missing, Err(CheckError::NotFound(_))));
}
