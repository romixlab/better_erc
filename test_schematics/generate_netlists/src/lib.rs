//! Exports the test schematics in `test_schematics/sources` to netlists (cached by content hash in
//! `test_schematics/generated_netlists`, gitignored) and SVG pictures (`generated_pictures`, committed) with
//! kicad-cli. Tests call [`get_netlist_path`], which exports the netlists first, once per test process, and leaves
//! the tracked pictures alone; `just netlists` ([`generate_netlists`]) exports the pictures too, the one way they
//! change. Which pictures are current is remembered in `generated_pictures_cache` (gitignored).

pub mod project;

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::{fs, io};
use subprocess::Exec;

/// Netlists and pictures of what changed: what `just netlists` does, on purpose.
pub fn generate_netlists() {
    export(true);
}

fn export(with_pictures: bool) {
    let generated_netlists_folder = get_parent_folder("generated_netlists").unwrap();
    let generated_pictures_folder = get_parent_folder("generated_pictures").unwrap();
    let pictures_cache_folder = get_parent_folder("generated_pictures_cache").unwrap();
    println!("generated netlists folder: {:?}", generated_netlists_folder);
    let kicad_cli_path = kicad_cli_path();

    let mut cache_hits = 0;
    for sch_path in collect_schematics() {
        let file_name = sch_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap()
            .strip_suffix(".kicad_sch")
            .unwrap();

        let contents = fs::read(&sch_path).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let sha256 = hex::encode(hasher.finalize());

        let cached_path = generated_netlists_folder.join(format!("{file_name}_{sha256}.net"));
        // let image_path = generated_pictures_folder.join(format!("{file_name}.svg"));
        // println!("cached path: {:?}", cached_path);
        if fs::exists(&cached_path).unwrap() {
            cache_hits += 1;
        } else {
            remove_outdated(&generated_netlists_folder, file_name);
            let status = Exec::cmd(kicad_cli_path)
                .args(&[
                    "sch",
                    "export",
                    "netlist",
                    "-o",
                    cached_path.to_str().unwrap(),
                    sch_path.to_str().unwrap(),
                ])
                .join();
            println!("Generated netlist for: {file_name}: {status:?}");
        }

        let picture_marker = pictures_cache_folder.join(format!("{file_name}_{sha256}"));
        if with_pictures && !fs::exists(&picture_marker).unwrap() {
            remove_outdated(&pictures_cache_folder, file_name);
            let status = Exec::cmd(kicad_cli_path)
                .args(&[
                    "sch",
                    "export",
                    "svg",
                    "--exclude-drawing-sheet",
                    "--no-background-color",
                    "-o",
                    generated_pictures_folder.to_str().unwrap(),
                    sch_path.to_str().unwrap(),
                ])
                .join();
            drop_svg_date(&generated_pictures_folder.join(format!("{file_name}.svg")));
            fs::write(&picture_marker, "").unwrap();
            println!("Generated picture for: {file_name}: {status:?}");
        }
    }

    println!("Cache hits: {cache_hits}");
}

/// The netlist exported from `sources/{name}.kicad_sch`, exporting what changed first (once per process, so
/// tests running in parallel don't export at the same time)
pub fn get_netlist_path(name: &str) -> PathBuf {
    static GENERATED: Once = Once::new();
    GENERATED.call_once(|| export(false));
    let generated_netlists_folder = get_parent_folder("generated_netlists").unwrap();
    let mut paths = fs::read_dir(generated_netlists_folder)
        .unwrap()
        .filter_map(|e| {
            let Ok(e) = e else { return None };
            if e.file_name().to_str().unwrap().starts_with(name) {
                Some(e.path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if paths.is_empty() {
        panic!(
            "no netlist for {name}: is there a sources/{name}.kicad_sch, and is kicad-cli installed?"
        );
    } else if paths.len() > 1 {
        panic!("Internal error: more than one cached file for {name}");
    } else {
        paths.remove(0)
    }
}

/// KiCad writes the export time into the SVG's `<title>`; the pictures are committed, so without this every
/// regeneration (a fresh clone, a newer KiCad) would show all of them as changed
fn drop_svg_date(svg_path: &Path) {
    let Ok(svg) = fs::read_to_string(svg_path) else {
        return;
    };
    let without_date = svg
        .lines()
        .map(|line| match (line.find("<title>"), line.find(" date ")) {
            (Some(_), Some(date)) => format!("{}</title>", &line[..date]),
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(svg_path, without_date).unwrap();
}

fn remove_outdated(folder: &Path, file_name: &str) {
    for entry in fs::read_dir(folder).unwrap() {
        let Ok(entry) = entry else { continue };
        if entry.file_name().to_str().unwrap().starts_with(file_name) {
            // println!("Removing outdated netlist for: {:?}", entry.path());
            fs::remove_file(entry.path()).unwrap();
        }
    }
}

#[cfg(target_os = "macos")]
fn kicad_cli_path() -> &'static Path {
    Path::new("/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli")
}

#[cfg(not(target_os = "macos"))]
fn kicad_cli_path() -> &'static Path {
    Path::new("kicad-cli")
}

fn collect_schematics() -> Vec<PathBuf> {
    let path = get_parent_folder("sources").unwrap();
    fs::read_dir(path)
        .unwrap()
        .filter_map(|e| {
            let Ok(e) = e else { return None };
            if e.file_name().to_str() == Some("test_schematics.kicad_sch") {
                // skip root schematic
                return None;
            }
            if Path::new(&e.file_name())
                .extension()
                .and_then(|s| s.to_str())
                == Some("kicad_sch")
            {
                Some(e.path())
            } else {
                None
            }
        })
        .collect()
}

fn get_parent_folder(folder_name: &str) -> io::Result<PathBuf> {
    // let mut path = std::env::current_dir()?;
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut path = manifest_dir
        .parent()
        .ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "No parent directory found",
        ))?
        .to_path_buf();
    path.push(folder_name);
    fs::create_dir_all(&path)?; // generated_netlists and generated_pictures_cache are gitignored, missing on a fresh clone
    fs::canonicalize(path)
}
