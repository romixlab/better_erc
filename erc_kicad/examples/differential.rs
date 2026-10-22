//! Our schematic importer against a KiCad netlist of the same schematic:
//! `cargo run -p erc_kicad --example differential -- board.kicad_sch board.net [8|9|10]`

use erc_model::{DirFiles, netcmp};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (sch, net, version) = match args.as_slice() {
        [s, n] => (s, n, None),
        [s, n, v] => (s, n, v.parse().ok()),
        _ => {
            eprintln!("usage: differential ROOT.kicad_sch NETLIST.net [KICAD_MAJOR]");
            std::process::exit(2);
        }
    };
    let sch = Path::new(sch);
    let dir = sch
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let files = DirFiles(dir.to_path_buf());
    let name = sch.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let (ours, notes) = match erc_kicad::sch::read(
        &files,
        name,
        &erc_model::ImportOptions {
            kicad_version: version,
            ..Default::default()
        },
    ) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    for n in &notes {
        eprintln!("note: {n}");
    }
    let text = std::fs::read_to_string(net).expect("netlist");
    let (theirs, _) = erc_kicad::netlist::read(&text, net).expect("netlist parses");
    let d = netcmp::compare(&ours, &theirs);
    println!(
        "same {} renamed {} only-ours {} only-kicad {}",
        d.same,
        d.renamed.len(),
        d.only_a.len(),
        d.only_b.len()
    );
    for (a, b) in &d.renamed {
        println!("renamed  ours {a:?}  kicad {b:?}");
    }
    for (n, s) in &d.only_a {
        println!("ours     {n:?} {s:?}");
    }
    for (n, s) in &d.only_b {
        println!("kicad    {n:?} {s:?}");
    }
}
