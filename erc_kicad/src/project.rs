//! A KiCad project as one [`Importer`]: the project file when there is one, the schematic hierarchy (with the
//! project's bus aliases), and the board file when there is one, joined into one [`Board`].

use crate::{pro, sch};
use erc_model::*;

/// The KiCad project (`.kicad_pro`) or a schematic (`.kicad_sch`) with the project around it as an [`Importer`]
pub struct KicadImporter;

/// The files of the project `main` names (its `.kicad_pro`, or its root `.kicad_sch`): (project file, root sheet)
fn project_files(main: &str) -> (String, String) {
    let stem = main
        .strip_suffix(".kicad_pro")
        .or_else(|| main.strip_suffix(".kicad_sch"))
        .unwrap_or(main);
    (format!("{stem}.kicad_pro"), format!("{stem}.kicad_sch"))
}

/// Reads the project `main` names: its project file and board when they are there, its schematic hierarchy
pub fn read(files: &dyn Files, main: &str, opts: &ImportOptions) -> Result<Imported, ImportError> {
    let (pro_file, root) = project_files(main);
    let root = if main.ends_with(".kicad_sch") {
        main.to_string()
    } else {
        root
    };
    let mut notes = Vec::new();
    let project = match files.read(&pro_file) {
        Some(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| ImportError::Invalid {
                file: pro_file.clone(),
                message: "not UTF-8".into(),
            })?;
            let (p, n) = pro::read(&text, &pro_file)?;
            notes.extend(n);
            Some(p)
        }
        None => None,
    };
    let aliases = project.as_ref().map_or(&[][..], |p| &p.bus_aliases[..]);
    let (mut board, n) = sch::read(files, &root, opts, aliases)?;
    notes.extend(n);
    if let Some(p) = project {
        board.source.files.insert(0, pro_file);
        p.apply(&mut board);
    }
    board.finish();
    Ok(Imported {
        design: Design::single(board),
        notes,
    })
}

impl Importer for KicadImporter {
    fn name(&self) -> &'static str {
        "kicad"
    }

    fn sniff(&self, src: &Source) -> Confidence {
        let starts = |b: &[u8], tag: &[u8]| b.trim_ascii_start().starts_with(tag);
        match src.files.read(&src.main) {
            Some(b) if starts(&b, b"(kicad_sch") => Confidence::Yes,
            Some(_) if src.main.ends_with(".kicad_pro") => Confidence::Yes,
            Some(_) if src.main.ends_with(".kicad_sch") => Confidence::Maybe,
            _ => Confidence::No,
        }
    }

    fn import(&self, src: &Source, opts: &ImportOptions) -> Result<Imported, ImportError> {
        read(src.files, &src.main, opts)
    }
}
