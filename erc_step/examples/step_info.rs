//! `cargo run --release -p erc_step --example step_info FILE...`: what erc_step reads from STEP files: unit,
//! parts, faces, triangles, box, warnings. `SUMMARY=1`: one tab-separated line per file (path, parts, faces,
//! faces meshed, triangles, warnings, box, time), for comparing two builds over a set of files.

fn main() {
    for path in std::env::args().skip(1) {
        let t = std::time::Instant::now();
        match erc_step::read_file(&path, &erc_step::Options::default()) {
            Ok(m) if std::env::var("SUMMARY").is_ok() => {
                let meshed = m
                    .parts
                    .iter()
                    .flat_map(|p| &p.faces)
                    .filter(|f| f.mesh.is_some())
                    .count();
                let b = m
                    .bounds()
                    .map(|b| format!("{:.3?} {:.3?}", b.min, b.max))
                    .unwrap_or_default();
                println!(
                    "{path}\t{}\t{}\t{meshed}\t{}\t{}\t{b}\t{}ms",
                    m.parts.len(),
                    m.face_count(),
                    m.triangle_count(),
                    m.warnings.len(),
                    t.elapsed().as_millis()
                );
            }
            Ok(m) => {
                println!(
                    "{path}: unit {} ({} mm), {} parts, {} faces, {} triangles, {} ms",
                    m.unit.name,
                    m.unit.mm,
                    m.parts.len(),
                    m.face_count(),
                    m.triangle_count(),
                    t.elapsed().as_millis()
                );
                if let Some(b) = m.bounds() {
                    println!("  bounds {:.3?} .. {:.3?}", b.min, b.max);
                }
                for p in &m.parts {
                    let meshed = p.faces.iter().filter(|f| f.mesh.is_some()).count();
                    let one = erc_step::Model {
                        parts: vec![p.clone()],
                        ..Default::default()
                    };
                    let b = one
                        .bounds()
                        .map(|b| format!("{:.2?}..{:.2?}", b.min, b.max))
                        .unwrap_or_default();
                    println!(
                        "  #{} {:?} colour {:?} faces {}/{} edges {} at {:.3?} box {b}",
                        p.step_id,
                        p.name.chars().take(40).collect::<String>(),
                        p.colour,
                        meshed,
                        p.faces.len(),
                        p.edges.len(),
                        p.transform.translation
                    );
                }
                let mut unmeshed = std::collections::BTreeMap::new();
                for f in m
                    .parts
                    .iter()
                    .flat_map(|p| &p.faces)
                    .filter(|f| f.mesh.is_none())
                {
                    let k = format!("{:?}", f.surface);
                    *unmeshed
                        .entry(k.split([' ', '{']).next().unwrap_or_default().to_string())
                        .or_insert(0) += 1;
                }
                if !unmeshed.is_empty() {
                    println!("  not meshed by surface: {unmeshed:?}");
                }
                for w in m.warnings.iter().take(10) {
                    println!("  {:?} {w}", w.kind);
                }
                if m.warnings.len() > 10 {
                    println!("  ... {} warnings", m.warnings.len());
                }
            }
            Err(e) => println!("{path}: {e}"),
        }
    }
}
