//! `DRAW-43` Sentence in a note, heading or table cell starting with a lower-case letter

use super::one;
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, Scene};
use erc_model::Pt;
use std::collections::BTreeSet;

pub struct Draw43;

static META: CheckMeta = CheckMeta {
    id: "DRAW-43",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[],
};

/// Words that end in a full stop inside a sentence
const ABBREVIATIONS: &[&str] = &[
    "e.g.", "i.e.", "vs.", "approx.", "etc.", "max.", "min.", "typ.", "cf.", "ca.", "no.", "nom.",
    "fig.", "ref.", "incl.", "excl.",
];

/// The words of a sheet that keep their case wherever they stand: its nets and parts
fn known(scene: &Scene) -> BTreeSet<String> {
    let g = scene.geometry;
    let labels = g.labels.iter().map(|l| l.text.text.to_lowercase());
    let parts = g.symbols.iter().map(|s| s.reference.to_lowercase());
    let values = g
        .symbols
        .iter()
        .filter(|s| s.power)
        .map(|s| s.value.to_lowercase());
    labels.chain(parts).chain(values).collect()
}

/// Whether the word at the start of a sentence stays lower case: code, a parameter, a unit or value
fn exempt(word: &str, next: Option<&str>, known: &BTreeSet<String>) -> bool {
    let bare = word.trim_end_matches([',', ':', ';', '.']);
    let mut chars = bare.chars();
    let first = chars.next();
    !first.is_some_and(char::is_lowercase)
        || chars.next().is_some_and(char::is_uppercase)
        || word.contains(['(', '=', '/'])
        || word.ends_with(':')
        || next.is_some_and(|n| n.starts_with(['=', ':']))
        || known.contains(&bare.to_lowercase())
}

/// The sentence starts of a text that fail, with the words they start on
fn lower_starts(text: &str, known: &BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    // One word is a value in a cell, not a sentence
    if text.split_whitespace().nth(1).is_none() {
        return out;
    }
    let mut start = true;
    for line in text.lines() {
        let line = line.trim_start_matches([' ', '\t', '-', '*', '•']);
        if line.trim().is_empty() {
            start = true;
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        for (i, w) in words.iter().enumerate() {
            if start && !exempt(w, words.get(i + 1).copied(), known) {
                out.push(w.to_string());
            }
            start =
                w.ends_with(['.', '!', '?']) && !ABBREVIATIONS.contains(&w.to_lowercase().as_str());
        }
    }
    out
}

/// The texts of a scene to read: free text, text boxes and table cells, with their places
fn texts<'s>(scene: &'s Scene) -> Vec<(&'s Item, &'s str, Pt)> {
    use erc_analysis::scene::ItemKind as K;
    let g = scene.geometry;
    let of = |kind| scene.items.iter().filter(move |i: &&Item| i.kind == kind);
    let free = of(K::Text)
        .zip(&g.texts)
        .map(|(i, t)| (i, t.text.text.as_str(), t.text.at));
    let boxes = of(K::TextBox)
        .zip(&g.text_boxes)
        .map(|(i, t)| (i, t.text.text.as_str(), t.rect.min));
    let cells = of(K::Table).zip(&g.tables).flat_map(|(i, t)| {
        t.cells
            .iter()
            .map(move |c| (i, c.text.text.as_str(), c.rect.min))
    });
    free.filter(|(_, t, _)| !t.trim_start().starts_with('.'))
        .chain(boxes)
        .chain(cells)
        .collect()
}

impl Check for Draw43 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// A new line only starts a sentence after a blank line or a line that ended one; a simulator directive is code
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let known = known(scene);
            for (item, text, at) in texts(scene) {
                let bad = lower_starts(text, &known);
                if bad.is_empty() {
                    continue;
                }
                out.push(
                    one(cx, &META, scene, item, at)
                        .message(format!("{}: starts with {}", item.name, bad.join(", ")))
                        .fix("start each sentence with a capital letter")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw43));

#[cfg(test)]
mod tests {
    use super::lower_starts;
    use std::collections::BTreeSet;

    fn lower(text: &str) -> Vec<String> {
        lower_starts(text, &BTreeSet::from(["vout".to_string()]))
    }

    #[test]
    fn sentence_starts() {
        assert_eq!(lower("the clamp holds. then it stops"), ["the", "then"]);
        assert_eq!(lower("Goes on\nover two lines"), Vec::<String>::new());
        assert_eq!(lower("Ends here.\nnext one"), ["next"]);
        assert_eq!(lower("One\n\nanother paragraph"), ["another"]);
        assert_eq!(lower("- first in a list"), ["first"]);
    }

    #[test]
    fn what_stays_lower_case() {
        for t in [
            "mA at the pin",
            "i(V1290) peak",
            "nX cells",
            "vout rises",
            "tau = 3 ms",
            "gain: 20",
            "e.g. this",
        ] {
            let got = lower(t);
            assert!(got.is_empty() || t.starts_with("e.g."), "{t}: {got:?}");
        }
        assert_eq!(lower("Roughly, e.g. this one"), Vec::<String>::new());
    }

    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&super::Draw43, "draw-43_sentence_capitals"),
            ["DRAW-43:item text box \"the first sentence. The ...\"|sheet /"]
        );
    }
}
