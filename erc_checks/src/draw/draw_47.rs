//! `DRAW-47` Run-on note text: long sentences or chains of `;`, `:` and `(` in a note box

use super::draw_43::ABBREVIATIONS;
use super::{CLEARANCE_TEXT, is_directive, nm, one};
use crate::meta::*;
use crate::{Ctx, Finding};
use erc_analysis::scene::{Item, ItemKind as K};
use erc_model::geometry::{DrawnTextBox, Rect};

pub struct Draw47;

const ITEM_WORDS: Param = Param {
    name: "drawing.note.item_words",
    default: 15.0,
    unit: "words",
    doc: "most words in a sentence of a note item",
};

const ITEM_MARKS: Param = Param {
    name: "drawing.note.item_marks",
    default: 3.0,
    unit: "marks",
    doc: "most `;`, `:` and `(` in one sentence of a note",
};

static META: CheckMeta = CheckMeta {
    id: "DRAW-47",
    category: "DRAW",
    severity: Severity::Suggestion,
    tier: Tier::Easy,
    scope: Scope::Board,
    needs: &[Need::Geometry],
    overlaps: &[],
    params: &[ITEM_WORDS, ITEM_MARKS, CLEARANCE_TEXT],
};

/// Whether a line's first word only marks a list item: `-`, `*`, `•`, `1.`, `2)`, `a)`
fn is_marker(word: &str) -> bool {
    if matches!(word, "-" | "*" | "•") {
        return true;
    }
    let Some(head) = word.strip_suffix(['.', ')']) else {
        return false;
    };
    let digits = !head.is_empty() && head.chars().all(|c| c.is_ascii_digit());
    let letter = word.ends_with(')') && head.chars().count() == 1;
    digits || letter
}

/// The sentences of a text. One runs over line ends until a full stop or a new item.
fn sentences(text: &str) -> Vec<Vec<&str>> {
    let mut out = Vec::new();
    let mut now: Vec<&str> = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace().peekable();
        let marked = words.peek().is_some_and(|w| is_marker(w));
        if line.trim().is_empty() || marked {
            out.push(std::mem::take(&mut now));
        }
        if marked {
            words.next();
        }
        for w in words {
            now.push(w);
            let abbreviation = ABBREVIATIONS.contains(&w.to_lowercase().as_str());
            if w.ends_with(['.', '!', '?']) && !abbreviation {
                out.push(std::mem::take(&mut now));
            }
        }
    }
    out.push(now);
    out.retain(|s| !s.is_empty());
    out
}

/// A sentence's length in words: a lone `+` or `<=` is no word
fn words(sentence: &[&str]) -> usize {
    let word = |w: &&&str| w.chars().any(char::is_alphanumeric);
    sentence.iter().filter(word).count()
}

fn marks(sentence: &[&str]) -> usize {
    let mark = |w: &&str| w.chars().filter(|c| matches!(c, ';' | ':' | '(')).count();
    sentence.iter().map(mark).sum()
}

/// What is too long in a text, one phrase per sentence
fn run_on(text: &str, max_words: f64, max_marks: f64) -> Vec<String> {
    let mut out = Vec::new();
    for s in sentences(text) {
        let (w, m) = (words(&s), marks(&s));
        let mut what = Vec::new();
        if w as f64 > max_words {
            what.push(format!("{w} words"));
        }
        if m as f64 > max_marks {
            what.push(format!("{m} of ; : ("));
        }
        if !what.is_empty() {
            let start = s.iter().take(4).copied().collect::<Vec<_>>().join(" ");
            out.push(format!("{} in \"{start} ...\"", what.join(" and ")));
        }
    }
    out
}

/// Whether a box stands flush to the right of a directive box: it explains that box line by line
fn explains(rect: Rect, directives: &[Rect], gap: i64) -> bool {
    directives.iter().any(|d| {
        let beside = rect.min.y < d.max.y && d.min.y < rect.max.y;
        beside && (rect.min.x - d.max.x).abs() <= gap
    })
}

impl Check for Draw47 {
    fn meta(&self) -> &'static CheckMeta {
        &META
    }

    /// Note boxes only: free text is `DRAW-32`'s and table cells hold values. Simulator directives are code, and the
    /// box flush beside them explains them line by line.
    fn run(&self, cx: &Ctx) -> Vec<Finding> {
        let max_words = cx.param(&META, ITEM_WORDS.name);
        let max_marks = cx.param(&META, ITEM_MARKS.name);
        let gap = nm(cx, &META, &CLEARANCE_TEXT);
        let mut out = Vec::new();
        for scene in cx.scenes() {
            let boxes: Vec<(&Item, &DrawnTextBox)> = scene
                .items
                .iter()
                .filter(|i| i.kind == K::TextBox)
                .zip(&scene.geometry.text_boxes)
                .collect();
            let code = |b: &&(&Item, &DrawnTextBox)| is_directive(&b.1.text.text);
            let directives: Vec<Rect> = boxes.iter().filter(code).map(|b| b.1.rect).collect();
            for (item, b) in &boxes {
                let skip = is_directive(&b.text.text) || explains(b.rect, &directives, gap);
                let long = run_on(&b.text.text, max_words, max_marks);
                if skip || long.is_empty() {
                    continue;
                }
                out.push(
                    one(cx, &META, scene, item, b.rect.min)
                        .message(format!("{}: {}", item.name, long.join("; ")))
                        .fix("a short lead line, then numbered items of one fact each")
                        .build(),
                );
            }
        }
        out
    }
}

inventory::submit!(Registered(&Draw47));

#[cfg(test)]
mod tests {
    use super::*;

    fn count(text: &str) -> Vec<usize> {
        sentences(text).iter().map(|s| words(s)).collect()
    }

    #[test]
    fn sentences_run_to_a_full_stop_or_a_new_item() {
        assert_eq!(count("One two\nthree four. Five"), [4, 1]);
        assert_eq!(count("Lead line\n1. First item\n2. Second"), [2, 2, 1]);
        assert_eq!(count("Lead\n- one\n- two\n\nThree four"), [1, 1, 1, 2]);
        assert_eq!(count("Holds 3.3 V, e.g. at 5 % high."), [7]);
        assert_eq!(count("Stays <= 3.6 V + 5 %."), [3 + 1]);
    }

    #[test]
    fn markers() {
        for w in ["-", "*", "•", "1.", "12)", "a)"] {
            assert!(is_marker(w), "{w}");
        }
        for w in ["-5V", "3.3", "No.", "a", "(a)"] {
            assert!(!is_marker(w), "{w}");
        }
    }

    #[test]
    fn long_or_chained() {
        let long = "One two three four five six seven eight nine ten.";
        assert_eq!(run_on(long, 15.0, 3.0), Vec::<String>::new());
        assert_eq!(
            run_on(long, 9.0, 3.0),
            ["10 words in \"One two three four ...\""]
        );
        assert_eq!(
            run_on("At a: b; c (d): e.", 15.0, 3.0),
            ["4 of ; : ( in \"At a: b; c ...\""]
        );
    }

    #[test]
    fn fires_on_its_sheet() {
        assert_eq!(
            crate::testing::fingerprints(&Draw47, "draw-47_run_on_note"),
            [
                "DRAW-47:item text box \"Chained: first (a); then...\"|sheet /",
                "DRAW-47:item text box \"This single sentence run...\"|sheet /"
            ]
        );
    }
}
