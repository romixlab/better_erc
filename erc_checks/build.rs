//! Embeds the check catalogue (`docs/checks.md`) and its detail pages (`docs/checks/*.md`) so explanations and
//! not-run hints work offline: writes `$OUT_DIR/docs.rs` with an `include_str!` per file.

use std::fmt::Write;
use std::path::Path;
use std::{env, fs};

fn main() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs")
        .canonicalize()
        .expect("docs/ next to the crate");
    let catalogue = docs.join("checks.md");
    let pages_dir = docs.join("checks");
    println!("cargo::rerun-if-changed={}", catalogue.display());
    println!("cargo::rerun-if-changed={}", pages_dir.display());
    let mut pages: Vec<_> = fs::read_dir(&pages_dir)
        .expect("docs/checks")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    pages.sort();
    let mut out = String::new();
    writeln!(out, "/// `docs/checks.md`").unwrap();
    writeln!(
        out,
        "pub static CATALOGUE: &str = include_str!({:?});",
        catalogue
    )
    .unwrap();
    writeln!(out, "/// `docs/checks/<page>.md` by page name").unwrap();
    writeln!(out, "pub static PAGES: &[(&str, &str)] = &[").unwrap();
    for p in &pages {
        println!("cargo::rerun-if-changed={}", p.display());
        let stem = p.file_stem().unwrap().to_str().unwrap();
        writeln!(out, "    ({stem:?}, include_str!({p:?})),").unwrap();
    }
    writeln!(out, "];").unwrap();
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("docs.rs"),
        out,
    )
    .unwrap();
}
