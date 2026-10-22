//! Live values for shell completion, each with its help text: check ids and categories from the embedded
//! catalogue, the needs `--require` knows, and the variants of the design in the current directory.

use clap_complete::engine::CompletionCandidate;
use erc_checks::{Need, Project, catalogue::catalogue, registered};

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
