//! Imports any design the registry reads (Altium, OrCAD, PnP, a schematic PDF, KiCad) and prints its canonical
//! dump, the import notes on stderr: `cargo run -p erc_import --example dump -- path/to/board.pdf`

use erc_import::Registry;
use erc_model::{DirFiles, ImportOptions, Source, dump};
use std::path::Path;

fn main() {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!("usage: dump FILE");
        std::process::exit(2);
    };
    let path = Path::new(&arg);
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let files = DirFiles(dir.to_path_buf());
    let main = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match Registry::default().import(&Source::new(main, &files), &ImportOptions::default()) {
        Ok(imported) => {
            print!("{}", dump::write(&imported.design));
            for n in imported.notes {
                eprintln!("{n}");
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
