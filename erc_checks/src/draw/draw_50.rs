//! `DRAW-50` Note reference to a note the board's notes file does not have

use super::one;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::ItemKind as K;
use std::collections::BTreeSet;

pub struct Draw50;

const NOTES: Param = Param {
    name: "drawing.notes",
    default: 0.0,
    unit: PATHS,
    doc: "the board's notes files (Markdown, one `## N<number>` heading per note), relative to the project directory",
    choices: &[],
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-50",
    category: "DRAW",
    severity: Severity::Warning,
    tier: Tier::Mid,
    scope: Scope::Board,
    needs: &[Need::Geometry, Need::Project(NOTES.name)],
    overlaps: &[],
    params: &[NOTES],
};

/// A note id, `N` and a number (`N13`), as its number
fn note_id(word: &str) -> Option<u32> {
    let digits = word.strip_prefix('N')?;
    let all = !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit());
    all.then(|| digits.parse().ok()).flatten()
}

/// The note ids of a Markdown file: headings of any level whose text starts with a note id (`## N13 · AUX supply`)
fn notes_of(text: &str) -> BTreeSet<u32> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix('#')?.trim_start_matches('#');
            let word = rest.split_whitespace().next()?;
            let word = word.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
            note_id(word)
        })
        .collect()
}

/// The note ids a text refers to: each list of ids after the word `See`, joined by commas, `and`, `&` or `/`
fn references(text: &str) -> Vec<u32> {
    let words: Vec<&str> = text
        .split(|c: char| {
            c.is_whitespace() || matches!(c, ',' | '/' | '&' | ';' | '(' | ')' | '.' | ':')
        })
        .filter(|w| !w.is_empty())
        .collect();
    let mut out = Vec::new();
    let mut in_list = false;
    for w in words {
        if w.eq_ignore_ascii_case("see") {
            in_list = true;
        } else if let Some(id) = note_id(w).filter(|_| in_list) {
            out.push(id);
        } else if !(in_list && w.eq_ignore_ascii_case("and")) {
            in_list = false;
        }
    }
    out
}

impl Check for Draw50 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Every text, text box and table cell of each sheet against the note ids of the files `drawing.notes` names
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let Some(Ok(files)) = cx.path_files(NOTES.name) else {
            return Vec::new();
        };
        let known: BTreeSet<u32> = files.iter().flat_map(|(_, t)| notes_of(t)).collect();
        let named: Vec<&str> = files.iter().map(|(p, _)| *p).collect();
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let g = scene.geometry;
            let texts = scene.items.iter().filter(|i| i.kind == K::Text);
            let boxes = scene.items.iter().filter(|i| i.kind == K::TextBox);
            let tables = scene.items.iter().filter(|i| i.kind == K::Table);
            let free = texts
                .zip(&g.texts)
                .map(|(i, t)| (i, references(&t.text.text)));
            let boxed = boxes
                .zip(&g.text_boxes)
                .map(|(i, b)| (i, references(&b.text.text)));
            let cells = tables.zip(&g.tables).map(|(i, t)| {
                (
                    i,
                    t.cells
                        .iter()
                        .flat_map(|c| references(&c.text.text))
                        .collect(),
                )
            });
            for (item, refs) in free.chain(boxed).chain(cells) {
                let mut missing: Vec<u32> =
                    refs.into_iter().filter(|n| !known.contains(n)).collect();
                missing.dedup();
                if missing.is_empty() {
                    continue;
                }
                let ids: Vec<String> = missing.iter().map(|n| format!("N{n}")).collect();
                out.push(
                    one(cx, &META, scene, item, item.rect.min)
                        .message(format!(
                            "{} refers to {}, which {} has no heading for",
                            item.name,
                            ids.join(", "),
                            named.join(", ")
                        ))
                        .fix("write the note, or point at the note that took its place")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw50));

#[cfg(test)]
mod tests {
    use super::{notes_of, references};

    #[test]
    fn note_headings() {
        let md = "# Notes\n## N1 · Power\ntext N7\n### N13: AUX\n## N2x\n## About N3\n";
        assert_eq!(notes_of(md).into_iter().collect::<Vec<_>>(), [1, 13]);
    }

    #[test]
    fn see_lists() {
        assert_eq!(references("See N12, N14"), [12, 14]);
        assert_eq!(references("* Gain 1.15\nSee N13"), [13]);
        assert_eq!(references("see N3 and N4/N5 & N6."), [3, 4, 5, 6]);
        assert!(references("N12 alone, NX3020 and 5N1").is_empty());
        assert_eq!(references("See N2 for R5, N9 too"), [2]);
    }
}
