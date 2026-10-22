//! A KiCad project as one [`Importer`]: the project file when there is one, the schematic hierarchy (with the
//! project's bus aliases), and the board file when there is one, joined into one [`Board`].

use crate::{netclass, pcb, pro, sch};
use erc_model::*;

/// The KiCad project (`.kicad_pro`) or a schematic (`.kicad_sch`) with the project around it as an [`Importer`]
pub struct KicadImporter;

/// The files of the project `main` names (its `.kicad_pro`, root `.kicad_sch` or `.kicad_pcb`): (project file,
/// root sheet, board)
fn project_files(main: &str) -> (String, String, String) {
    let stem = [".kicad_pro", ".kicad_sch", ".kicad_pcb"]
        .iter()
        .find_map(|ext| main.strip_suffix(ext))
        .unwrap_or(main);
    (
        format!("{stem}.kicad_pro"),
        format!("{stem}.kicad_sch"),
        format!("{stem}.kicad_pcb"),
    )
}

/// A file as UTF-8 text, `None` when it is not there
fn text(files: &dyn Files, path: &str) -> Result<Option<String>, ImportError> {
    match files.read(path) {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| ImportError::Invalid {
                file: path.to_string(),
                message: "not UTF-8".into(),
            }),
        None => Ok(None),
    }
}

/// Reads the project `main` names: its project file and board when they are there (the board unless
/// [`ImportOptions::no_layout`]), its schematic hierarchy. A board without a schematic is read alone.
pub fn read(files: &dyn Files, main: &str, opts: &ImportOptions) -> Result<Imported, ImportError> {
    let (pro_file, root, pcb_file) = project_files(main);
    let root = if main.ends_with(".kicad_sch") {
        main.to_string()
    } else {
        root
    };
    let pcb_file = if main.ends_with(".kicad_pcb") {
        main.to_string()
    } else {
        pcb_file
    };
    let mut notes = Vec::new();
    let layout = match text(files, &pcb_file)? {
        Some(t) if !opts.no_layout => {
            let (lay, n) = pcb::read(&t, &pcb_file)?;
            notes.extend(n);
            Some(lay)
        }
        _ => None,
    };
    if main.ends_with(".kicad_pcb") && files.read(&root).is_none() {
        let name = pcb_file.rsplit('/').next().unwrap_or(&pcb_file);
        let mut board = Board {
            name: name.trim_end_matches(".kicad_pcb").to_string(),
            source: SourceInfo {
                tool: "kicad".into(),
                version: None,
                files: vec![pcb_file],
                connectivity: None,
            },
            layout,
            ..Board::default()
        };
        board.finish();
        return Ok(Imported {
            design: Design::single(board),
            notes,
        });
    }
    let project = match text(files, &pro_file)? {
        Some(text) => {
            let (p, n) = pro::read(&text, &pro_file)?;
            notes.extend(n);
            Some(p)
        }
        None => None,
    };
    let aliases = project.as_ref().map_or(&[][..], |p| &p.bus_aliases[..]);
    let (mut board, n) = sch::read(files, &root, opts, aliases)?;
    notes.extend(n);
    let class_source = match project {
        Some(p) => {
            board.source.files.insert(0, pro_file.clone());
            p.apply(&mut board);
            Provenance::KicadProject { file: pro_file }
        }
        None => Provenance::Schematic { file: root },
    };
    netclass::resolve(&mut board, &class_source);
    if let Some(lay) = layout {
        board.source.files.push(lay.file.clone());
        board.layout = Some(lay);
    }
    board.finish();
    link_notes(&board, &mut notes);
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
            Some(b) if starts(&b, b"(kicad_pcb") => Confidence::Yes,
            Some(_) if src.main.ends_with(".kicad_pro") => Confidence::Yes,
            Some(_) if src.main.ends_with(".kicad_sch") => Confidence::Maybe,
            _ => Confidence::No,
        }
    }

    fn import(&self, src: &Source, opts: &ImportOptions) -> Result<Imported, ImportError> {
        read(src.files, &src.main, opts)
    }
}

/// Where a footprint's link by schematic path and its designator disagree, or the path names no part
fn link_notes(board: &Board, notes: &mut Vec<ImportNote>) {
    let Some(lay) = &board.layout else { return };
    let paths = erc_model::part_paths(board);
    for f in &lay.footprints {
        let (Some(path), Some(part)) = (&f.schematic_path, f.part) else {
            continue;
        };
        let designator = &board.part(part).designator;
        let message = if !paths.contains_key(path) {
            format!("schematic path {path} names no symbol, linked to {designator} by designator")
        } else if *designator != f.reference {
            format!(
                "schematic path {path} is part {designator}, not {} (re-annotated since the board was updated?), linked by path",
                f.reference
            )
        } else {
            continue;
        };
        notes.push(ImportNote {
            file: lay.file.clone(),
            at: format!("footprint {}", f.reference),
            kind: NoteKind::Conflict,
            message,
        });
    }
}
