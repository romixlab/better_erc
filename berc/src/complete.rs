//! Live values for shell completion, each with its help text: check ids and categories from the embedded
//! catalogue, the needs `--require` knows, the variants of the design in the current directory, and the references
//! of the board named earlier on the command line.

use better_erc::checks::{Need, Project, catalogue::catalogue, registered};
use clap_complete::engine::CompletionCandidate;

/// Every catalogue id, implemented or planned (`berc explain`)
pub fn catalogue_ids() -> Vec<CompletionCandidate> {
    catalogue()
        .entries
        .iter()
        .map(|e| CompletionCandidate::new(&e.id).help(Some(e.title.clone().into())))
        .collect()
}

/// The checks that run and every category (`--only`)
pub fn check_ids_and_categories() -> Vec<CompletionCandidate> {
    let cat = catalogue();
    let mut out: Vec<CompletionCandidate> = cat
        .categories
        .iter()
        .map(|c| CompletionCandidate::new(&c.id).help(Some(c.title.clone().into())))
        .collect();
    out.extend(registered().iter().map(|c| {
        let id = c.meta().id;
        let title = cat.entry(id).map_or("", |e| e.title.as_str());
        CompletionCandidate::new(id).help(Some(title.to_string().into()))
    }));
    out
}

/// What `--require` accepts
pub fn needs() -> Vec<CompletionCandidate> {
    Need::FIXED
        .iter()
        .map(|n| CompletionCandidate::new(n.slug()).help(Some(n.about().into())))
        .collect()
}

/// The variants of the design in the current directory, when there is exactly one
pub fn variants() -> Vec<CompletionCandidate> {
    let Ok(project) = Project::open(std::path::Path::new(".")) else {
        return Vec::new();
    };
    project
        .variants()
        .into_iter()
        .map(|v| {
            CompletionCandidate::new(v).help(Some(format!("variant of {}", project.main).into()))
        })
        .collect()
}

/// The footprint references of the board named earlier on the command line, each with its library footprint: a
/// `.kicad_pcb` (`berc fp hash BOARD REF`), or a project's directory, `.kicad_pro` or `.kicad_sch` with its board
/// beside it (`berc fp accept PROJECT REF`)
pub fn board_references() -> Vec<CompletionCandidate> {
    let Some(board) = std::env::args().skip(1).find_map(|a| board_of(&a)) else {
        return Vec::new();
    };
    let Ok(lay) = better_erc::fp::read_board(std::path::Path::new(&board)) else {
        return Vec::new();
    };
    let mut refs: Vec<(String, String)> = lay
        .footprints
        .iter()
        .filter(|f| !f.reference.is_empty())
        .map(|f| (f.reference.clone(), f.lib_id.clone()))
        .collect();
    refs.sort_by(|a, b| better_erc::model::natural_cmp(&a.0, &b.0));
    refs.dedup_by(|a, b| a.0 == b.0);
    refs.into_iter()
        .map(|(r, lib)| CompletionCandidate::new(r).help(Some(lib.into())))
        .collect()
}

/// What `berc fp model` takes after its target: the footprints of a `.pretty` named earlier on the command line,
/// each with its file, else the references of the board ([`board_references`])
pub fn model_names() -> Vec<CompletionCandidate> {
    let lib = std::env::args()
        .skip(1)
        .map(std::path::PathBuf::from)
        .find(|a| a.extension().is_some_and(|e| e == "pretty") && a.is_dir());
    let Some(lib) = lib else {
        return board_references();
    };
    let mut names: Vec<String> = std::fs::read_dir(&lib)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "kicad_mod"))
        .filter_map(|p| Some(p.file_stem()?.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| CompletionCandidate::new(n).help(Some("footprint".into())))
        .collect()
}

/// The symbol libraries of the sym-lib-table in the current directory and the global one, by nickname, and the
/// paths that start with `current` (`berc fp check --lib`)
pub fn symbol_libraries(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    use clap_complete::engine::{PathCompleter, ValueCompleter};
    let config = better_erc::fp::kicad_config_dir();
    let libs = better_erc::kicad::libtable::LibTables::new(
        better_erc::kicad::libtable::LibKind::Symbol,
        std::path::Path::new("."),
        config.as_deref(),
    );
    let prefix = current.to_string_lossy();
    let mut out: Vec<CompletionCandidate> = libs
        .libraries()
        .into_iter()
        .filter(|e| e.name.starts_with(prefix.as_ref()))
        .map(|e| CompletionCandidate::new(&e.name).help(Some(e.uri.clone().into())))
        .collect();
    out.extend(PathCompleter::any().complete(current));
    out
}

/// The board an argument names: the `.kicad_pcb` itself, the one beside a `.kicad_pro` or `.kicad_sch` of the same
/// name, or the only one in a directory
fn board_of(arg: &str) -> Option<String> {
    let p = std::path::Path::new(arg);
    if arg.ends_with(".kicad_pcb") {
        return Some(arg.to_string());
    }
    if arg.ends_with(".kicad_pro") || arg.ends_with(".kicad_sch") {
        let b = p.with_extension("kicad_pcb");
        return b.is_file().then(|| b.to_string_lossy().into_owned());
    }
    if !p.is_dir() {
        return None;
    }
    let mut boards = std::fs::read_dir(p)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|f| f.extension().is_some_and(|x| x == "kicad_pcb"));
    let first = boards.next()?;
    boards
        .next()
        .is_none()
        .then(|| first.to_string_lossy().into_owned())
}
