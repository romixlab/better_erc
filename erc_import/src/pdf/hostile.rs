//! Hostile PDFs: loops, depths and sizes a broken or malicious file can have. Each must end, within its limit, in
//! a time that does not grow past the file's size, and say what it cut.

use super::doc::{Doc, Limits};
use super::PdfImporter;
use erc_model::*;
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use std::time::{Duration, Instant};

/// Object ids handed out in order; 1 is the catalog, 2 the page tree, 3 the page, 4 the font
struct Ids(i32);

impl Ids {
    fn new() -> Ids {
        Ids(5)
    }
    fn next(&mut self) -> Ref {
        self.0 += 1;
        Ref::new(self.0 - 1)
    }
}

const CATALOG: Ref = Ref::new(1);
const PAGES: Ref = Ref::new(2);
const PAGE: Ref = Ref::new(3);
const FONT: Ref = Ref::new(4);

/// One page with its content, annotations and form XObjects (name, id), and the catalog with `outlines` and
/// `names`
fn finish(
    mut pdf: Pdf,
    outlines: Option<Ref>,
    names: Option<Ref>,
    content: Ref,
    annots: &[Ref],
    forms: &[(&[u8], Ref)],
) -> Vec<u8> {
    pdf.type1_font(FONT).base_font(Name(b"Helvetica"));
    let mut p = pdf.page(PAGE);
    p.parent(PAGES)
        .media_box(Rect::new(0.0, 0.0, 842.0, 595.0))
        .contents(content);
    if !annots.is_empty() {
        p.annotations(annots.iter().copied());
    }
    let mut res = p.resources();
    res.fonts().pair(Name(b"F1"), FONT);
    let mut x = res.x_objects();
    for (name, r) in forms {
        x.pair(Name(name), *r);
    }
    x.finish();
    res.finish();
    p.finish();
    pdf.pages(PAGES).kids([PAGE]).count(1);
    let mut c = pdf.catalog(CATALOG);
    c.pages(PAGES);
    if let Some(o) = outlines {
        c.outlines(o);
    }
    if let Some(n) = names {
        c.pair(Name(b"Names"), n);
    }
    c.finish();
    pdf.finish()
}

fn empty_content(pdf: &mut Pdf, ids: &mut Ids) -> Ref {
    let r = ids.next();
    pdf.stream(r, &Content::new().finish());
    r
}

/// An outline whose items are written by `items`, given the outline root and the ids; returns its first item
fn with_outline(items: impl FnOnce(&mut Pdf, &mut Ids, Ref) -> Ref) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let mut ids = Ids::new();
    let root = ids.next();
    let first = items(&mut pdf, &mut ids, root);
    pdf.indirect(root)
        .dict()
        .pair(Name(b"Type"), Name(b"Outlines"))
        .pair(Name(b"First"), first);
    let content = empty_content(&mut pdf, &mut ids);
    finish(pdf, Some(root), None, content, &[], &[])
}

fn open(bytes: Vec<u8>, limits: Limits) -> Doc {
    Doc::open(bytes, limits).expect("opens")
}

/// The whole importer, as `berc check` runs it: sniff, then import; an error is fine, a panic or a hang is not
fn import(bytes: Vec<u8>) -> (Confidence, Result<Imported, ImportError>) {
    let files = MemFiles::default().with("hostile.pdf", bytes);
    let src = Source::new("hostile.pdf", &files);
    (
        PdfImporter.sniff(&src),
        PdfImporter.import(&src, &ImportOptions::default()),
    )
}

/// Generous for a debug build on a busy PC; a hang or a quadratic walk is far past it
const BOUND: Duration = Duration::from_secs(20);

fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let out = f();
    assert!(t.elapsed() < BOUND, "{what} took {:?}", t.elapsed());
    out
}

#[test]
fn next_loop() {
    let bytes = with_outline(|pdf, ids, root| {
        let (a, b) = (ids.next(), ids.next());
        pdf.outline_item(a)
            .title(pdf_writer::TextStr("A"))
            .parent(root)
            .next(b);
        pdf.outline_item(b)
            .title(pdf_writer::TextStr("B"))
            .parent(root)
            .next(a);
        a
    });
    let o = open(bytes.clone(), Limits::default()).outline(None);
    assert_eq!(o.items.len(), 2);
    assert_eq!(o.loops, 1);
    assert!(import(bytes).1.is_err(), "not a Smart PDF");
}

#[test]
fn first_is_itself() {
    let bytes = with_outline(|pdf, ids, root| {
        let a = ids.next();
        pdf.outline_item(a)
            .title(pdf_writer::TextStr("A"))
            .parent(root)
            .first(a)
            .next(a);
        a
    });
    let o = open(bytes, Limits::default()).outline(None);
    assert_eq!(o.items.len(), 1);
    assert_eq!(o.loops, 2, "through /First and through /Next");
}

#[test]
fn deep_outline() {
    const DEPTH: usize = 10_000;
    let bytes = with_outline(|pdf, ids, root| {
        let items: Vec<Ref> = (0..DEPTH).map(|_| ids.next()).collect();
        for (i, r) in items.iter().enumerate() {
            let mut it = pdf.outline_item(*r);
            it.title(pdf_writer::TextStr("Nets"))
                .parent(if i == 0 { root } else { items[i - 1] });
            if let Some(kid) = items.get(i + 1) {
                it.first(*kid).last(*kid);
            }
        }
        items[0]
    });
    let o = timed("deep outline", || {
        open(bytes.clone(), Limits::default()).outline(None)
    });
    assert_eq!(o.items.len(), Limits::default().outline_depth);
    assert!(o.truncated.as_deref().unwrap().contains("nested deeper"));
    // Every level is titled Nets, so it looks like a Smart PDF without a sheet: an error, not a hang
    let (sniff, imported) = timed("deep outline import", || import(bytes));
    assert_eq!(sniff, Confidence::Yes);
    let imported = imported.expect("imports what it could read");
    assert!(
        imported
            .notes
            .iter()
            .any(|n| n.message.contains("nested deeper")),
        "{:?}",
        imported.notes
    );
}

/// A sheet with this many components, each with one pin and its own net: far more than a real board
fn wide_smart(parts: usize) -> Vec<u8> {
    with_outline(|pdf, ids, root| {
        let sheet = ids.next();
        let comps = ids.next();
        let nets = ids.next();
        let items: Vec<[Ref; 4]> = (0..parts)
            .map(|_| [ids.next(), ids.next(), ids.next(), ids.next()])
            .collect();
        pdf.outline_item(sheet)
            .title(pdf_writer::TextStr("Wide.SchDoc(Wide)"))
            .parent(root)
            .first(comps)
            .last(nets);
        pdf.outline_item(comps)
            .title(pdf_writer::TextStr("Components"))
            .parent(sheet)
            .next(nets)
            .first(items[0][0])
            .last(items[parts - 1][0]);
        pdf.outline_item(nets)
            .title(pdf_writer::TextStr("Nets"))
            .parent(sheet)
            .first(items[0][2])
            .last(items[parts - 1][2]);
        for (i, [comp, pin, net, net_pin]) in items.iter().enumerate() {
            let d = format!("R{}", i + 1);
            let p = format!("{d}-1");
            let mut c = pdf.outline_item(*comp);
            c.title(pdf_writer::TextStr(&d))
                .parent(comps)
                .first(*pin)
                .last(*pin);
            if let Some(n) = items.get(i + 1) {
                c.next(n[0]);
            }
            c.finish();
            pdf.outline_item(*pin)
                .title(pdf_writer::TextStr(&p))
                .parent(*comp);
            let mut n = pdf.outline_item(*net);
            n.title(pdf_writer::TextStr(&format!("N{i}")))
                .parent(nets)
                .first(*net_pin)
                .last(*net_pin);
            if let Some(n2) = items.get(i + 1) {
                n.next(n2[2]);
            }
            n.finish();
            pdf.outline_item(*net_pin)
                .title(pdf_writer::TextStr(&p))
                .parent(*net);
        }
        sheet
    })
}

#[test]
fn many_bookmarks() {
    const PARTS: usize = 25_000;
    let bytes = wide_smart(PARTS);
    let (sniff, imported) = timed("100 000 bookmarks", || import(bytes.clone()));
    assert_eq!(sniff, Confidence::Yes);
    let b = &imported.expect("imports").design.boards[0];
    assert_eq!(b.parts.len(), PARTS);
    assert_eq!(b.nets.len(), PARTS);

    let few = Limits {
        outline_items: 1000,
        ..Limits::default()
    };
    let o = open(bytes, few).outline(None);
    assert_eq!(o.items.len(), 1000);
    assert!(o.truncated.as_deref().unwrap().contains("first 1000 bookmarks"));
}

/// A page that draws `Fm0`, with the given forms: (name, content, resources: names of other forms)
fn with_forms(forms: &[(&str, Vec<u8>, Vec<&str>)]) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let mut ids = Ids::new();
    let refs: Vec<Ref> = forms.iter().map(|_| ids.next()).collect();
    let by_name = |n: &str| refs[forms.iter().position(|f| f.0 == n).unwrap()];
    for (i, (_, content, uses)) in forms.iter().enumerate() {
        let mut f = pdf.form_xobject(refs[i], content);
        f.bbox(Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut res = f.resources();
        res.fonts().pair(Name(b"F1"), FONT);
        let mut x = res.x_objects();
        for u in uses {
            x.pair(Name(u.as_bytes()), by_name(u));
        }
    }
    let mut c = Content::new();
    c.x_object(Name(b"Fm0"));
    let content = ids.next();
    pdf.stream(content, &c.finish());
    let page_forms: Vec<(&[u8], Ref)> = vec![(b"Fm0", refs[0])];
    finish(pdf, None, None, content, &[], &page_forms)
}

fn draws(names: &[&str], times: usize) -> Vec<u8> {
    let mut c = Content::new();
    for _ in 0..times {
        for n in names {
            c.x_object(Name(n.as_bytes()));
        }
    }
    c.finish().to_vec()
}

fn text(word: &str) -> Vec<u8> {
    let mut c = Content::new();
    c.begin_text()
        .set_font(Name(b"F1"), 8.0)
        .show(Str(word.as_bytes()))
        .end_text();
    c.finish().to_vec()
}

#[test]
fn form_draws_itself() {
    let mut content = text("X");
    content.extend(draws(&["Fm0"], 1));
    let bytes = with_forms(&[("Fm0", content, vec!["Fm0"])]);
    let (words, gaps) = timed("a form drawing itself", || {
        open(bytes, Limits::default()).words()
    });
    assert_eq!(words.len(), 1, "drawn once, the loop not followed");
    assert_eq!(gaps.truncated, None);
}

#[test]
fn forms_drawing_forms() {
    // Ten forms, each drawing the next ten times: 10^9 drawings from a file of a few kilobytes
    let names: Vec<String> = (0..10).map(|i| format!("Fm{i}")).collect();
    let forms: Vec<(&str, Vec<u8>, Vec<&str>)> = (0..10)
        .map(|i| {
            if i == 9 {
                (names[i].as_str(), text("X"), vec![])
            } else {
                (
                    names[i].as_str(),
                    draws(&[&names[i + 1]], 10),
                    vec![names[i + 1].as_str()],
                )
            }
        })
        .collect();
    let bytes = with_forms(&forms);
    let few = Limits {
        forms: 20_000,
        ..Limits::default()
    };
    let (_, gaps) = timed("forms drawing forms", || open(bytes.clone(), few).words());
    assert!(
        gaps.truncated.as_deref().unwrap().contains("form drawings"),
        "{gaps:?}"
    );
    // The default limits: bounded too
    let (_, gaps) = timed("forms drawing forms, default limits", || {
        open(bytes, Limits::default()).words()
    });
    assert!(gaps.truncated.is_some());
}

/// A link on the page whose action is the given script, chained to `chain` more scripts through `/Next`
fn with_link(script: &[u8], chain: usize, loop_back: bool) -> Vec<u8> {
    let mut pdf = Pdf::new();
    let mut ids = Ids::new();
    let annot = ids.next();
    let actions: Vec<Ref> = (0..=chain).map(|_| ids.next()).collect();
    pdf.indirect(annot)
        .dict()
        .pair(Name(b"Type"), Name(b"Annot"))
        .pair(Name(b"Subtype"), Name(b"Link"))
        .pair(Name(b"Rect"), Rect::new(10.0, 10.0, 20.0, 20.0))
        .pair(Name(b"A"), actions[0]);
    for (i, r) in actions.iter().enumerate() {
        let mut d = pdf.indirect(*r).dict();
        d.pair(Name(b"S"), Name(b"JavaScript"))
            .pair(Name(b"JS"), Str(script));
        match actions.get(i + 1) {
            Some(n) => {
                d.pair(Name(b"Next"), *n);
            }
            None if loop_back => {
                d.pair(Name(b"Next"), actions[0]);
            }
            None => {}
        }
    }
    let content = empty_content(&mut pdf, &mut ids);
    finish(pdf, None, None, content, &[annot], &[])
}

#[test]
fn huge_script() {
    let mut script = b"app.popUpMenu(\"".to_vec();
    script.resize(5 << 20, b'a');
    let bytes = with_link(&script, 0, false);
    let limits = Limits::default();
    let (links, _) = timed("a 5 MB script", || open(bytes, limits.clone()).links());
    assert_eq!(links[0].action.scripts[0].len(), limits.script_chars);
}

#[test]
fn action_chains() {
    let limits = Limits::default();
    let (links, _) = open(with_link(b"x();", 1000, false), limits.clone()).links();
    assert_eq!(links[0].action.scripts.len(), limits.next_actions + 1);
    let (links, _) = open(with_link(b"x();", 3, true), limits).links();
    assert_eq!(links[0].action.scripts.len(), 4, "the loop back is not followed");
}

#[test]
fn script_tree_loop() {
    let mut pdf = Pdf::new();
    let mut ids = Ids::new();
    let names = ids.next();
    let root = ids.next();
    let leaf = ids.next();
    let action = ids.next();
    pdf.indirect(names).dict().pair(Name(b"JavaScript"), root);
    pdf.indirect(root)
        .dict()
        .insert(Name(b"Kids"))
        .array()
        .items([leaf, root]);
    let mut d = pdf.indirect(leaf).dict();
    let mut arr = d.insert(Name(b"Names")).array();
    arr.item(Str(b"f"));
    arr.item(action);
    arr.finish();
    d.insert(Name(b"Kids")).array().items([root, leaf]);
    d.finish();
    pdf.indirect(action)
        .dict()
        .pair(Name(b"S"), Name(b"JavaScript"))
        .pair(Name(b"JS"), Str(b"function f() {}"));
    let content = empty_content(&mut pdf, &mut ids);
    let bytes = finish(pdf, None, Some(names), content, &[], &[]);
    let (scripts, cut) = timed("a looping name tree", || {
        open(bytes, Limits::default()).document_scripts()
    });
    assert_eq!(scripts.len(), 1);
    assert_eq!(cut, None);
}
