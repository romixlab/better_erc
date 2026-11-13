//! The extended internal set: the STEP models on this PC that are not ours to publish (`BERC_STEP_DIR`, default
//! `~/sync/hw_tracking`, skipped with a note when it is not there). Every model must read within the time budget
//! without a panic, and its face count and extents must match the golden list kept beside the set
//! (`erc_step-golden.tsv` in that directory): one line per file, its path relative to the directory, the faces,
//! the faces meshed, and the box in mm. More faces meshed than the list says is fine; fewer, or a box that moved
//! by more than 0.01 mm, fails. `BERC_STEP_GOLDEN_WRITE=1` writes the list from this run instead (files the list
//! does not have yet are added: every model of `vhrd-packages3D` is in the set).

use erc_step::{Options, read_file};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const GOLDEN: &str = "erc_step-golden.tsv";

fn set_dir() -> Option<PathBuf> {
    let dir = match std::env::var_os("BERC_STEP_DIR") {
        Some(d) => PathBuf::from(d),
        None => PathBuf::from(std::env::var_os("HOME")?).join("sync/hw_tracking"),
    };
    if dir.join("vhrd-packages3D").is_dir() {
        Some(dir)
    } else {
        eprintln!(
            "note: {} has no vhrd-packages3D: the extended STEP set is skipped",
            dir.display()
        );
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Line {
    faces: usize,
    meshed: usize,
    min: [f64; 3],
    max: [f64; 3],
}

fn parse_golden(text: &str) -> BTreeMap<String, Line> {
    let mut out = BTreeMap::new();
    for l in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let f: Vec<&str> = l.split('\t').collect();
        let n = |i: usize| {
            f[i].parse::<f64>()
                .unwrap_or_else(|_| panic!("{GOLDEN}: bad number in {l}"))
        };
        assert_eq!(f.len(), 9, "{GOLDEN}: 9 columns expected in {l}");
        out.insert(
            f[0].to_string(),
            Line {
                faces: n(1) as usize,
                meshed: n(2) as usize,
                min: [n(3), n(4), n(5)],
                max: [n(6), n(7), n(8)],
            },
        );
    }
    out
}

fn step_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            step_files(&p, out);
        } else if p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("step") || x.eq_ignore_ascii_case("stp"))
        {
            out.push(p);
        }
    }
}

#[test]
fn extended_set_reads_and_matches_golden() {
    let Some(dir) = set_dir() else { return };
    let golden_path = dir.join(GOLDEN);
    let golden = std::fs::read_to_string(&golden_path)
        .map(|t| parse_golden(&t))
        .unwrap_or_default();
    let write = std::env::var_os("BERC_STEP_GOLDEN_WRITE").is_some();
    if golden.is_empty() && !write {
        eprintln!(
            "note: no {}: run with BERC_STEP_GOLDEN_WRITE=1 to write it",
            golden_path.display()
        );
        return;
    }
    let mut files: Vec<String> = golden.keys().cloned().collect();
    let mut vhrd = Vec::new();
    step_files(&dir.join("vhrd-packages3D"), &mut vhrd);
    for p in vhrd {
        let rel = p.strip_prefix(&dir).unwrap().to_string_lossy().into_owned();
        if !golden.contains_key(&rel) {
            files.push(rel);
        }
    }
    files.sort();
    let results: Mutex<BTreeMap<String, Result<Line, String>>> = Mutex::new(BTreeMap::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(rel) = files.get(i) else { break };
                    let r = read_file(dir.join(rel), &Options::default())
                        .map_err(|e| e.to_string())
                        .and_then(|m| {
                            let meshed = m
                                .parts
                                .iter()
                                .flat_map(|p| &p.faces)
                                .filter(|f| f.mesh.is_some())
                                .count();
                            let b = m.bounds().ok_or("nothing meshed")?;
                            Ok(Line {
                                faces: m.face_count(),
                                meshed,
                                min: b.min,
                                max: b.max,
                            })
                        });
                    results.lock().unwrap().insert(rel.clone(), r);
                }
            });
        }
    });
    let results = results.into_inner().unwrap();
    let mut problems = Vec::new();
    for (rel, r) in &results {
        match (r, golden.get(rel)) {
            (Err(e), _) => problems.push(format!("{rel}: {e}")),
            (Ok(_), None) if !write => problems.push(format!(
                "{rel}: not in {GOLDEN} (BERC_STEP_GOLDEN_WRITE=1 adds it)"
            )),
            (Ok(_), None) => {}
            (Ok(_), Some(_)) if write => {}
            (Ok(got), Some(want)) => {
                let moved = (0..3)
                    .map(|k| {
                        (got.min[k] - want.min[k])
                            .abs()
                            .max((got.max[k] - want.max[k]).abs())
                    })
                    .fold(0.0, f64::max);
                if got.faces != want.faces || got.meshed < want.meshed || moved > 0.01 {
                    problems.push(format!("{rel}: got {got:?}, the list has {want:?}"));
                }
            }
        }
    }
    if write {
        let mut text = String::from(
            "# erc_step's extended set (erc_step/tests/extended.rs): path, faces, faces meshed, box min x y z, max x y z in mm\n",
        );
        for (rel, r) in &results {
            if let Ok(l) = r {
                let [a, b, c] = l.min;
                let [d, e, f] = l.max;
                text.push_str(&format!(
                    "{rel}\t{}\t{}\t{a:.4}\t{b:.4}\t{c:.4}\t{d:.4}\t{e:.4}\t{f:.4}\n",
                    l.faces, l.meshed
                ));
            }
        }
        std::fs::write(&golden_path, text).unwrap();
        eprintln!("wrote {} ({} files)", golden_path.display(), results.len());
    }
    assert!(
        problems.is_empty(),
        "{} of {} files:\n{}",
        problems.len(),
        results.len(),
        problems.join("\n")
    );
}
