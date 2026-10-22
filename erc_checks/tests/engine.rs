//! The engine with checks of its own (outside the registry): needs and not-run reports, overlaps, project
//! overrides of severity and thresholds, variants, the report's JSON and text.

use erc_checks::*;
use erc_model::{DirFiles, ImportOptions, Imported};
use std::path::Path;

fn import(name: &str) -> Imported {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources");
    erc_kicad::project::read(
        &DirFiles(dir),
        &format!("{name}.kicad_sch"),
        &ImportOptions::default(),
    )
    .unwrap()
}

/// A finding per component whose Value is `?`
struct Placeholder;
static PLACEHOLDER: CheckMeta = CheckMeta {
    id: "BOM-5",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Values],
    overlaps: &[],
    params: &[Param {
        name: "limit",
        default: 1.0,
        unit: "",
        doc: "a threshold",
    }],
};
impl Check for Placeholder {
    fn meta(&self) -> &'static CheckMeta {
        &PLACEHOLDER
    }
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter(|p| b.field(*p, "Value", cx.variant) == Some("?"))
            .map(|p| {
                cx.finding(&PLACEHOLDER)
                    .part(p)
                    .message(format!("limit {}", cx.param(&PLACEHOLDER, "limit")))
                    .build()
            })
            .collect()
    }
}

/// The same parts, fitted only: overlaps BOM-5
struct Mounted;
static MOUNTED: CheckMeta = CheckMeta {
    id: "BOM-6",
    category: "BOM",
    severity: Severity::Error,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Values],
    overlaps: &["BOM-5"],
    params: &[],
};
impl Check for Mounted {
    fn meta(&self) -> &'static CheckMeta {
        &MOUNTED
    }
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let b = cx.board();
        cx.components()
            .filter(|p| b.field(*p, "Value", cx.variant) == Some("?") && cx.fitted(*p))
            .map(|p| cx.finding(&MOUNTED).part(p).message("mounted").build())
            .collect()
    }
}

/// Needs the board file
struct NeedsLayout;
static NEEDS_LAYOUT: CheckMeta = CheckMeta {
    id: "LAY-1",
    category: "LAY",
    severity: Severity::Error,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Graph, Need::Layout],
    overlaps: &[],
    params: &[],
};
impl Check for NeedsLayout {
    fn meta(&self) -> &'static CheckMeta {
        &NEEDS_LAYOUT
    }
    fn run(&self, _: &Ctx) -> Vec<Finding> {
        panic!("must not run without a layout")
    }
}

const CHECKS: &[&dyn Check] = &[&Placeholder, &Mounted, &NeedsLayout];

#[test]
fn needs_overlaps_and_ordering() {
    let imported = import("bom-10_values");
    let report = run_with(CHECKS, &imported.design, &RunOptions::default()).unwrap();
    // R3 (fitted) and R9 (DNP) have `?`: BOM-6 on R3 folds BOM-5 on R3 in, BOM-5 on R9 stays
    let found: Vec<(&str, Vec<String>, &[String])> = report
        .findings
        .iter()
        .map(|f| (f.check.as_str(), f.subject_names(), &f.also[..]))
        .collect();
    assert_eq!(
        found,
        [
            ("BOM-5", vec!["part R9".to_string()], &[][..]),
            (
                "BOM-6",
                vec!["part R3".to_string()],
                &["BOM-5".to_string()][..]
            ),
        ]
    );
    let f = &report.findings[1];
    assert_eq!(f.fingerprint, "BOM-6:part R3");
    assert_eq!(f.confidence, Confidence::Certain);
    assert_eq!(f.explain, "docs/checks/bom.md#bom-6");
    assert_eq!(f.locations[0].file, "bom-10_values.kicad_sch");
    assert_eq!(f.locations[0].at, Some((60.0, 50.0)));

    let lay = report.checks.iter().find(|c| c.id == "LAY-1").unwrap();
    let Status::NotRun(n) = &lay.status else {
        panic!("{lay:?}")
    };
    assert_eq!(n.need, "layout");
    assert!(n.hint.starts_with("no layout"), "{}", n.hint);
    assert!(n.hint.contains("(the check needs: "), "{}", n.hint);
    let planned = report
        .checks
        .iter()
        .filter(|c| c.status == Status::Planned)
        .count();
    assert_eq!(report.summary.planned, planned);
    assert_eq!((report.summary.ran, report.summary.not_run), (2, 1));
    assert_eq!(report.summary.errors, 2);
    assert!(
        report.checks.iter().any(|c| c.id == "I2C-1"),
        "registered or planned, in the list"
    );

    let back = Report::from_json(&report.to_json()).unwrap();
    assert_eq!(back, report);
    let text = report.to_text();
    assert!(text.contains("BOM - BOM hygiene"), "{text}");
    assert!(
        text.contains("error      BOM-6    mounted (also BOM-5)"),
        "{text}"
    );
    assert!(text.contains("LAY-1    needs layout: no layout"), "{text}");
}

#[test]
fn overrides_and_selection() {
    let imported = import("bom-10_values");
    let mut opts = RunOptions::default();
    opts.config.severity.insert("BOM-5".into(), Severity::Info);
    opts.config.params.insert("BOM-5.limit".into(), 7.0);
    opts.only = vec!["BOM-5".into()];
    let report = run_with(CHECKS, &imported.design, &opts).unwrap();
    assert_eq!(report.findings.len(), 2);
    assert!(report.findings.iter().all(|f| f.severity == Severity::Info));
    assert_eq!(report.findings[0].message, "limit 7");
    assert!(report.checks.iter().all(|c| c.id != "LAY-1"));

    opts.only = vec!["NOPE-1".into()];
    assert_eq!(
        run_with(CHECKS, &imported.design, &opts).unwrap_err(),
        RunError::UnknownCheck("NOPE-1".into())
    );
    let opts = RunOptions {
        variant: Some("Lite".into()),
        ..RunOptions::default()
    };
    assert_eq!(
        run_with(CHECKS, &imported.design, &opts).unwrap_err(),
        RunError::UnknownVariant("Lite".into())
    );
}
