use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::{fs, io};
use subprocess::Exec;

fn main() {
    let generated_netlists_folder = get_parent_folder("generated_netlists").unwrap();
    println!("Generated netlists folder: {:?}", generated_netlists_folder);
    let kicad_cli_path = kicad_cli_path();

    let mut cache_hits = 0;
    for sch_path in collect_schematics() {
        let file_name = sch_path
            .file_name()
            .map(|s| s.to_str())
            .flatten()
            .unwrap()
            .strip_suffix(".kicad_sch")
            .unwrap();

        let contents = fs::read(&sch_path).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let sha256 = hex::encode(&hasher.finalize());

        let cached_path = generated_netlists_folder.join(format!("{file_name}_{sha256}.net"));
        // println!("cached path: {:?}", cached_path);
        let file_exists = fs::exists(&cached_path).unwrap();
        if !file_exists {
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
            println!("Generating netlist for: {file_name}: {status:?}");
        } else {
            cache_hits += 1;
        }
    }

    println!("Cache hits: {cache_hits}");
}

#[cfg(target_os = "macos")]
fn kicad_cli_path() -> &'static Path {
    Path::new("/Applications/KiCad/KiCad.app/Contents/MacOS/kicad-cli")
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
                .map(|s| s.to_str())
                .flatten()
                == Some("kicad_sch".into())
            {
                Some(e.path())
            } else {
                None
            }
        })
        .collect()
}

fn get_parent_folder(folder_name: &str) -> std::io::Result<PathBuf> {
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
    Ok(fs::canonicalize(path)?)
}
