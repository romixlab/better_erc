//! Imports a KiCad project, schematic or netlist and prints its canonical dump, the import notes on stderr:
//! `cargo run -p erc_kicad --example dump -- path/to/board.kicad_pro`

use erc_model::{DirFiles, ImportOptions, Importer, Source, dump};
use std::path::Path;

fn main() {
    let Some(arg) = std::env::args().nth(1) else {
        eprintln!("usage: dump FILE (.kicad_pro, .kicad_sch or .net)");
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
    let src = Source::new(main, &files);
    let importers: Vec<Box<dyn Importer>> = erc_kicad::importers();
    let Some(imp) = importers.iter().max_by_key(|i| i.sniff(&src)) else {
        return;
    };
    match imp.import(&src, &ImportOptions::default()) {
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
