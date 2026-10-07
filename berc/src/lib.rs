//! KiCad files as sorted, line-oriented text, for `git diff` through a textconv filter: one fact per line, no
//! UUIDs, no coordinates unless asked for, so reordering and re-saving never show up as changes.

pub mod common;
pub mod pcb;
pub mod sch;
pub mod sexpr;

pub use common::Options;

/// Text form of a KiCad file. Never fails: input that isn't a KiCad s-expression comes back as it is, with a note
/// on the first line, so `git diff` still shows something.
pub fn textconv(text: &str, opts: Options) -> String {
    let parsed = match sexpr::parse(text) {
        Ok(p) => p,
        Err(e) => {
            return format!("# berc: not a KiCad s-expression file ({e}), shown as is\n{text}");
        }
    };
    let [root] = parsed.as_slice() else {
        return format!(
            "# berc: {} top-level expressions, expected one, shown as is\n{text}",
            parsed.len()
        );
    };
    match root.head() {
        Some("kicad_sch") => sch::render(root, opts),
        Some("kicad_pcb") => pcb::render(root, opts),
        _ => generic(root, opts),
    }
}

/// Any other KiCad s-expression file (symbol library, footprint, ...): one line per top-level item, sorted
fn generic(root: &sexpr::Sexp, opts: Options) -> String {
    let mut lines: Vec<String> = root
        .args()
        .iter()
        .map(|i| common::compact(i, opts))
        .collect();
    common::sort_natural(&mut lines);
    let mut out = format!("{}\n", root.head().unwrap_or("?"));
    for l in lines {
        out += &l;
        out.push('\n');
    }
    out
}
