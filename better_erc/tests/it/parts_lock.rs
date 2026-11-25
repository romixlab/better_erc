//! The parts snapshot (CFG-9) beside a design: read by `Project::open`, where `[parts] lock` says, and BOM-20 on
//! the MPNs it lacks

use better_erc::{CheckError, CheckOptions, Project};
use std::path::{Path, PathBuf};

fn sources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_schematics/sources")
}

/// `bom-20_parts_lock` as `board.kicad_sch` in a fresh directory, its snapshot at `lock` (none: no snapshot)
fn design(name: &str, lock: Option<&str>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("parts-lock-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        sources().join("bom-20_parts_lock.kicad_sch"),
        dir.join("board.kicad_sch"),
    )
    .unwrap();
    if let Some(lock) = lock {
        let to = dir.join(lock);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(sources().join("bom-20_parts_lock.parts.lock.toml"), to).unwrap();
    }
    dir
}

fn bom_20(dir: &Path) -> (Vec<String>, Vec<String>) {
    let p = Project::open(&dir.join("board.kicad_sch")).unwrap();
    let run = p.check(&CheckOptions::default()).unwrap();
    let found = run
        .report
        .findings
        .iter()
        .filter(|f| f.check == "BOM-20")
        .map(|f| f.message.clone())
        .collect();
    let not_run = run
        .report
        .not_run()
        .filter(|n| n.check == "BOM-20")
        .map(|n| n.hint.clone())
        .collect();
    (found, not_run)
}

#[test]
fn mpns_the_snapshot_lacks_are_findings() {
    let (found, not_run) = bom_20(&design("beside", Some("parts.lock.toml")));
    assert_eq!(
        found,
        [
            "MPN ERJ-2RKF1002X on R2 is not in the parts snapshot: its facts and footprint reviews are not checked",
            "MPN RC0402FR-071KL on R4 is not in the parts snapshot: its facts and footprint reviews are not checked",
        ]
    );
    assert!(not_run.is_empty());
}

#[test]
fn the_project_file_names_another_place() {
    let dir = design("elsewhere", Some("data/board.lock.toml"));
    std::fs::write(
        dir.join("board.berc.toml"),
        "[parts]\nlock = \"data/board.lock.toml\"\n",
    )
    .unwrap();
    assert_eq!(bom_20(&dir).0.len(), 2);
}

#[test]
fn without_a_snapshot_the_check_does_not_run() {
    let (found, not_run) = bom_20(&design("none", None));
    assert!(found.is_empty());
    assert_eq!(not_run.len(), 1);
    assert!(
        not_run[0].starts_with(
            "no parts snapshot beside the project: `berc parts refresh` writes parts.lock.toml"
        ),
        "{}",
        not_run[0]
    );
}

#[test]
fn a_broken_snapshot_is_an_error() {
    let dir = design("broken", None);
    std::fs::write(dir.join("parts.lock.toml"), "version = 9\nsource = \"x\"\n").unwrap();
    let Err(e) = Project::open(&dir.join("board.kicad_sch")) else {
        panic!("opened")
    };
    assert!(matches!(e, CheckError::PartsLock { .. }), "{e}");
    assert!(
        e.to_string()
            .ends_with("/parts.lock.toml: format version 9, this better_erc reads up to 1"),
        "{e}"
    );
}
