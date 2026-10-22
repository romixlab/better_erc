//! The differential test: our schematic importer against KiCad's own netlist of the same schematic, every net as
//! the set of `designator.pin` it connects, names included, for every KiCad major version the schematic can be
//! read by.
//!
//! - The fixtures (`tests/fixtures`, `ecad_file_format/test_input/kicad_versions`) with their committed netlists
//!   from KiCad 8, 9 and 10 (`just kicad-fixtures` exports them again).
//! - The test schematics (`test_schematics/sources`), each sheet alone and the whole project, exported by the
//!   installed kicad-cli (cached by content hash); skipped with a message when kicad-cli is not installed.
//! - The KiCad projects under `~/git/hw` (or `BERC_HW_DIR`) when that folder exists: private boards, tested
//!   here and never committed. `BERC_DIFF_KICAD=8,9,10` adds the KiCad versions other than the installed one
//!   through podman; by default only the installed kicad-cli exports.
//!
//! Each fixture and test schematic also has a committed canonical dump of what we import (`tests/golden/sch`,
//! `BLESS=1` rewrites them), and `import → dump → read → dump` must be stable.

use erc_model::{DirFiles, ImportOptions, NoteKind, dump, netcmp};
use generate_netlists::project::{self, Kicad};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn cache() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("kicad_netlists")
}

/// A schematic with its committed netlists
struct Fixture {
    root: &'static str,
    /// KiCad major versions with a committed netlist `<root stem>.kicad<v>.net` next to it
    versions: &'static [u32],
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        root: "erc_kicad/tests/fixtures/connectivity.kicad_sch",
        versions: &[8, 9, 10],
    },
    Fixture {
        root: "erc_kicad/tests/fixtures/hierarchy.kicad_sch",
        versions: &[8, 9, 10],
    },
    Fixture {
        root: "erc_kicad/tests/fixtures/local_power.kicad_sch",
        versions: &[10],
    },
    Fixture {
        root: "erc_kicad/tests/fixtures/alias.kicad_sch",
        versions: &[8, 9, 10],
    },
    Fixture {
        root: "erc_kicad/tests/fixtures/alias_pro.kicad_sch",
        versions: &[10],
    },
    Fixture {
        root: "ecad_file_format/test_input/kicad_versions/fixture.kicad_sch",
        versions: &[8, 9, 10],
    },
    Fixture {
        root: "ecad_file_format/test_input/kicad_versions/fixture_kicad10.kicad_sch",
        versions: &[10],
    },
];

/// Where KiCad itself is wrong, and we read the design as the other versions do: (root, KiCad version, the
/// difference as [`render`] writes it, why)
const KNOWN: &[(&str, u32, &str, &str)] = &[
    (
        "erc_kicad/tests/fixtures/hierarchy.kicad_sch",
        9,
        "ours  /CTL.EN R14.2 R24.2 R4.1
ours  /DATA0 R11.2 R21.2
ours  /DATA2 R12.2 R22.2 R3.1
kicad /B/CTL.EN R24.2
kicad /B/D0 R21.2
kicad /B/D2 R22.2
kicad /CTL.EN R14.2 R4.1
kicad /DATA0 R11.2
kicad /DATA2 R12.2 R3.1
",
        "KiCad 9.0.9 does not carry buses through the sheet pins of the second instance of a sheet used twice; \
     KiCad 8 and 10 do",
    ),
    (
        "erc_kicad/tests/fixtures/alias.kicad_sch",
        9,
        "renamed ours /DP kicad /Sub/DP
ours  /DM R1.2 R11.2
ours  /X.DP R12.2 R2.1
kicad /DM R1.2
kicad /Sub/DM R11.2
kicad /Sub/X.DP R12.2
kicad /X.DP R2.1
",
        "KiCad 9.0.9's kicad-cli does not read the bus aliases of a KiCad 8 sheet (9 keeps them in the project \
         file); KiCad 10 reads them",
    ),
];

/// A net difference as text, one net per line
fn render(d: &netcmp::NetDiff) -> String {
    let mut out = String::new();
    for (a, b) in &d.renamed {
        out += &format!("renamed ours {a} kicad {b}\n");
    }
    for (who, list) in [("ours ", &d.only_a), ("kicad", &d.only_b)] {
        for (n, set) in list {
            let pins: Vec<&str> = set.iter().map(String::as_str).collect();
            out += &format!("{who} {n} {}\n", pins.join(" "));
        }
    }
    out
}

/// Our import of the project whose root sheet is `root`, read as KiCad `version` would (`None`: the files'
/// version)
fn import(root: &Path, version: Option<u32>) -> (erc_model::Board, Vec<erc_model::ImportNote>) {
    let dir = root.parent().unwrap().to_path_buf();
    let name = root.file_name().unwrap().to_str().unwrap();
    let opts = ImportOptions {
        kicad_version: version,
        ..Default::default()
    };
    let mut imported = erc_kicad::project::read(&DirFiles(dir), name, &opts)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()));
    (imported.design.boards.remove(0), imported.notes)
}

/// The difference between our nets and those of a KiCad netlist, as text (empty: the same)
fn differ(root: &Path, netlist: &Path, version: Option<u32>) -> String {
    let (ours, _) = import(root, version);
    let text =
        std::fs::read_to_string(netlist).unwrap_or_else(|e| panic!("{}: {e}", netlist.display()));
    let (theirs, _) = erc_kicad::netlist::read(&text, &netlist.display().to_string())
        .unwrap_or_else(|e| panic!("{}: {e}", netlist.display()));
    render(&netcmp::compare(&ours, &theirs))
}

/// The newest schematic format among the files of a design we read, as a KiCad major version
fn kicad_major(root: &Path, board: &erc_model::Board) -> u32 {
    let dir = root.parent().unwrap();
    board
        .source
        .files
        .iter()
        .filter_map(|f| {
            let text = std::fs::read_to_string(dir.join(f)).ok()?;
            let at = text.find("(version")? + "(version".len();
            text[at..].split(')').next()?.trim().parse::<u32>().ok()
        })
        .max()
        .map_or(10, erc_kicad::sch::kicad_of_format)
}

/// The golden dump of our import of `root` (default options), `name` naming the golden files
fn golden(root: &Path, name: &str, failed: &mut Vec<String>) {
    let (board, notes) = import(root, None);
    let unknown: Vec<_> = notes
        .iter()
        .filter(|n| n.kind == NoteKind::Unknown)
        .collect();
    if !unknown.is_empty() {
        failed.push(format!("{name}: unknown elements: {unknown:#?}"));
    }
    let notes: String = notes.iter().map(|n| format!("{n}\n")).collect();
    let design = erc_model::Design::single(board);
    let text = dump::write(&design);
    let path = repo()
        .join("erc_kicad/tests/golden/sch")
        .join(format!("{name}.dump"));
    let notes_path = path.with_extension("notes");
    if std::env::var_os("BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        if notes.is_empty() {
            let _ = std::fs::remove_file(&notes_path);
        } else {
            std::fs::write(&notes_path, &notes).unwrap();
        }
    }
    if std::fs::read_to_string(&path).unwrap_or_default() != text {
        failed.push(format!(
            "{name}: dump differs from {} (BLESS=1 to update)",
            path.display()
        ));
    }
    if std::fs::read_to_string(&notes_path).unwrap_or_default() != notes {
        failed.push(format!(
            "{name}: notes differ from {}",
            notes_path.display()
        ));
    }
    stable(&design, name, failed);
}

/// `dump → read → dump` gives the same text and the same design
fn stable(design: &erc_model::Design, name: &str, failed: &mut Vec<String>) {
    let text = dump::write(design);
    match dump::read(&text) {
        Ok(back) if dump::write(&back) == text && back == *design => {}
        Ok(_) => failed.push(format!("{name}: dump, read, dump is not stable")),
        Err(e) => failed.push(format!("{name}: the dump does not read back: {e}")),
    }
}

/// One comparison; a difference not in [`KNOWN`] (or a known one that changed) is a failure
fn check(root_rel: &str, root: &Path, netlist: &Path, version: u32, failed: &mut Vec<String>) {
    let diff = differ(root, netlist, Some(version));
    let known = KNOWN
        .iter()
        .find(|k| k.0 == root_rel && k.1 == version)
        .map(|k| k.2);
    match known {
        Some(want) if diff == want => {}
        Some(want) => failed.push(format!(
            "{root_rel} KiCad {version}: the known difference changed\nwas:\n{want}now:\n{diff}"
        )),
        None if diff.is_empty() => {}
        None => failed.push(format!("{root_rel} KiCad {version}:\n{diff}")),
    }
}

#[test]
fn fixtures_equal_kicad() {
    let mut failed = Vec::new();
    for f in FIXTURES {
        let root = repo().join(f.root);
        for &v in f.versions {
            let net = root.with_file_name(format!(
                "{}.kicad{v}.net",
                root.file_stem().unwrap().to_str().unwrap()
            ));
            check(f.root, &root, &net, v, &mut failed);
        }
        let name = f
            .root
            .rsplit('/')
            .next()
            .unwrap()
            .trim_end_matches(".kicad_sch");
        golden(&root, name, &mut failed);
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn test_schematics_equal_kicad() {
    let Some(major) = project::installed_major() else {
        eprintln!("kicad-cli is not installed: the test schematics are not compared with KiCad");
        return;
    };
    let sources = repo().join("test_schematics/sources");
    let mut sheets: Vec<PathBuf> = std::fs::read_dir(&sources)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "kicad_sch"))
        .collect();
    sheets.sort();
    let mut failed = Vec::new();
    for sch in &sheets {
        let stem = sch.file_stem().unwrap().to_str().unwrap();
        let rel = format!("test_schematics/sources/{stem}.kicad_sch");
        // The project root is exported as a whole project, each other sheet on its own
        let net = if stem == "test_schematics" {
            project::cached_project_netlist(sch, Kicad::Installed, &cache()).unwrap()
        } else {
            generate_netlists::get_netlist_path(stem)
        };
        check(&rel, sch, &net, major, &mut failed);
        golden(sch, &format!("test_schematics/{stem}"), &mut failed);
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// The KiCad projects of a hw checkout: every `X.kicad_pro` with an `X.kicad_sch` next to it
fn hw_projects(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if let Some(stem) = name.strip_suffix(".kicad_pro") {
                let sch = d.join(format!("{stem}.kicad_sch"));
                if sch.exists() {
                    out.push(sch);
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
fn hw_boards_equal_kicad() {
    let dir = std::env::var_os("BERC_HW_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("git/hw")));
    let Some(dir) = dir.filter(|d| d.is_dir()) else {
        eprintln!("no hw checkout (~/git/hw or BERC_HW_DIR): hw boards not compared");
        return;
    };
    let installed = project::installed_major();
    let versions: Vec<u32> = match std::env::var("BERC_DIFF_KICAD") {
        Ok(v) => v.split(',').filter_map(|v| v.trim().parse().ok()).collect(),
        Err(_) => installed.into_iter().collect(),
    };
    if versions.is_empty() {
        eprintln!(
            "kicad-cli is not installed and BERC_DIFF_KICAD is not set: hw boards not compared"
        );
        return;
    }
    let podman = project::podman_available();
    let mut failed = Vec::new();
    let mut compared = 0;
    for sch in hw_projects(&dir) {
        let rel = sch.strip_prefix(&dir).unwrap().display().to_string();
        let (board, _) = import(&sch, None);
        let newest = kicad_major(&sch, &board);
        stable(&erc_model::Design::single(board), &rel, &mut failed);
        for &v in &versions {
            // An older KiCad cannot read a newer file format
            if v < newest {
                continue;
            }
            let kicad = Kicad::for_major(v);
            if kicad != Kicad::Installed && !podman {
                eprintln!("{rel}: KiCad {v} needs podman, not installed");
                continue;
            }
            match project::cached_project_netlist(&sch, kicad, &cache()) {
                Ok(net) => {
                    check(&rel, &sch, &net, v, &mut failed);
                    compared += 1;
                }
                Err(e) => failed.push(format!("{rel}: {e}")),
            }
        }
    }
    eprintln!("hw: {compared} comparisons with KiCad {versions:?}");
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
