//! Validate signal-level TOML files: `cargo run -p erc_levels --example validate -- [FILE...]`.
//!
//! The files are read as one database (the embedded one first unless `--alone`), so a fragment with
//! `[docs.X]` and `[family.Y]` tables can be checked before it is merged. Prints each problem and a
//! count of families, levels and verified levels; exits 1 on any problem.

use erc_levels::{BUILTIN_TOML, Database};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let alone = args.iter().any(|a| a == "--alone");
    args.retain(|a| a != "--alone");
    let mut text = if alone {
        String::new()
    } else {
        BUILTIN_TOML.to_string()
    };
    for a in &args {
        text.push('\n');
        text.push_str(&std::fs::read_to_string(a).unwrap_or_else(|e| panic!("{a}: {e}")));
    }
    let db = match Database::parse(&text) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let problems = db.validate();
    for p in &problems {
        println!("{p}");
    }
    let levels: usize = db.family.values().map(|f| f.level.len()).sum();
    let verified: usize = db
        .family
        .values()
        .map(|f| f.level.iter().filter(|l| l.verified).count())
        .sum();
    println!(
        "{} families, {} documents, {levels} levels ({verified} verified), {} problems",
        db.family.len(),
        db.docs.len(),
        problems.len()
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
