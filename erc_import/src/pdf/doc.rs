//! The PDF as the importer sees it: the bookmark tree, the link annotations with their actions, and the words of
//! the pages with where they start. Everything is read through `hayro-syntax`, a lazy parser that returns `None`
//! instead of panicking, and everything this layer walks is bounded ([`Limits`]): a hostile or broken file costs
//! time and memory in proportion to its size, never more, and the importer gets what could be read.
//!
//! JavaScript is never run: an action's script is kept as text for the reader above to take literals from.

use hayro_syntax::Pdf;
use hayro_syntax::content::TypedIter;
use hayro_syntax::content::ops::TypedInstruction;
use hayro_syntax::object::{Array, Dict, MaybeRef, Name, Object, ObjectIdentifier, Rect, Stream};
use hayro_syntax::page::Resources;
use std::collections::{HashMap, HashSet};

/// What the reader walks at most. The defaults are far above any schematic seen (the largest Smart PDF here has
/// 14 000 bookmarks, 300 000 content operations and 18 form drawings) and far below what would exhaust a PC.
#[derive(Clone, Debug)]
pub(crate) struct Limits {
    /// File size
    pub bytes: usize,
    /// Bookmarks
    pub outline_items: usize,
    /// Bookmark nesting
    pub outline_depth: usize,
    /// Link annotations over all pages
    pub links: usize,
    /// Actions chained through `/Next` after one action
    pub next_actions: usize,
    /// Characters of one script
    pub script_chars: usize,
    /// Content operations over all pages and forms
    pub operations: usize,
    /// Form XObjects nested in a page
    pub form_depth: usize,
    /// Form XObjects drawn over all pages (each nested draw counts)
    pub forms: usize,
    /// Words over all pages
    pub words: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            bytes: 256 << 20,
            outline_items: 2_000_000,
            outline_depth: 64,
            links: 1_000_000,
            next_actions: 64,
            script_chars: 1 << 20,
            operations: 20_000_000,
            form_depth: 8,
            forms: 100_000,
            words: 5_000_000,
        }
    }
}

/// Why a file could not be opened at all
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OpenError {
    TooLarge { bytes: usize, limit: usize },
    NotPdf,
    Encrypted,
    Invalid,
}

/// An action's effect, as far as the importer cares
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Action {
    /// The page a `GoTo` (or a destination) leads to, from 0
    pub page: Option<usize>,
    /// The scripts of the `JavaScript` actions, in chain order
    pub scripts: Vec<String>,
}

/// A bookmark
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Bookmark {
    pub title: String,
    pub action: Action,
    /// Indices into [`Outline::items`], in order
    pub children: Vec<usize>,
}

/// The bookmark tree
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outline {
    pub items: Vec<Bookmark>,
    /// The top-level bookmarks
    pub roots: Vec<usize>,
    /// Set when a limit cut the walk short, with what was cut
    pub truncated: Option<String>,
    /// Bookmarks reached a second time (a loop in `/Next` or `/First`), not followed
    pub loops: usize,
}

/// A link annotation
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Link {
    pub page: usize,
    /// `x0, y0, x1, y1` in the page's default user space, normalised so that `x0 <= x1` and `y0 <= y1`
    pub rect: [f64; 4],
    pub action: Action,
}

/// A run of text without white space, at the origin of the string it is part of (default user space)
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Word {
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub text: String,
    /// Drawn with text rendering mode 3 (neither filled nor stroked): there for search, not for the eye
    pub invisible: bool,
}

/// What the page text walk left out
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TextGaps {
    /// Strings in composite (`Type0`) fonts, whose bytes are not character codes of one byte
    pub composite: usize,
    /// Set when a limit cut the walk short
    pub truncated: Option<String>,
}

/// An open PDF
pub(crate) struct Doc {
    pdf: Pdf,
    limits: Limits,
    /// Page objects by their identifier, to resolve destinations
    page_ids: HashMap<ObjectIdentifier, usize>,
}

impl Doc {
    pub fn open(bytes: Vec<u8>, limits: Limits) -> Result<Doc, OpenError> {
        if bytes.len() > limits.bytes {
            return Err(OpenError::TooLarge {
                bytes: bytes.len(),
                limit: limits.bytes,
            });
        }
        if !is_pdf(&bytes) {
            return Err(OpenError::NotPdf);
        }
        let pdf = Pdf::new(bytes).map_err(|e| match e {
            hayro_syntax::LoadPdfError::Decryption(_) => OpenError::Encrypted,
            hayro_syntax::LoadPdfError::Invalid => OpenError::Invalid,
        })?;
        let page_ids = pdf
            .pages()
            .iter()
            .enumerate()
            .filter_map(|(i, p)| Some((p.raw().obj_id()?, i)))
            .collect();
        Ok(Doc {
            pdf,
            limits,
            page_ids,
        })
    }

    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// A page's media box as `x0, y0, x1, y1`
    pub fn media_box(&self, page: usize) -> Option<[f64; 4]> {
        let r = self.pdf.pages().get(page)?.media_box();
        Some([r.x0.min(r.x1), r.y0.min(r.y1), r.x0.max(r.x1), r.y0.max(r.y1)])
    }

    /// The document information's `Creator` and `Producer`
    pub fn creator(&self) -> (Option<String>, Option<String>) {
        let m = self.pdf.metadata();
        let s = |b: &Option<Vec<u8>>| b.as_deref().map(text_string);
        (s(&m.creator), s(&m.producer))
    }

    fn catalog(&self) -> Option<Dict<'_>> {
        let xref = self.pdf.xref();
        xref.get::<Dict<'_>>(xref.root_id())
    }

    /// The bookmark tree; with `max` the walk stops after that many bookmarks (a cheap look for sniffing)
    pub fn outline(&self, max: Option<usize>) -> Outline {
        let mut out = Outline::default();
        let Some(first) = self
            .catalog()
            .and_then(|c| c.get::<Dict<'_>>("Outlines"))
            .and_then(|o| first_child(&o))
        else {
            return out;
        };
        let max = max.unwrap_or(usize::MAX).min(self.limits.outline_items);
        let mut seen: HashSet<ObjectIdentifier> = HashSet::new();
        // (parent bookmark or None for the top, first child, depth): each sibling chain is walked whole, so the
        // children of a bookmark keep their order whatever order the chains are taken in
        let mut chains: Vec<(Option<usize>, MaybeRef<Dict<'_>>, usize)> = vec![(None, first, 0)];
        'chains: while let Some((parent, first, depth)) = chains.pop() {
            if depth >= self.limits.outline_depth {
                out.truncated
                    .get_or_insert(format!("bookmarks nested deeper than {depth} not read"));
                continue;
            }
            let mut next = Some(first);
            while let Some(item) = next.take() {
                if out.items.len() >= max {
                    if max == self.limits.outline_items {
                        out.truncated
                            .get_or_insert(format!("only the first {max} bookmarks read"));
                    }
                    break 'chains;
                }
                let Some(dict) = self.resolve(item, &mut seen, &mut out.loops) else {
                    break;
                };
                let index = out.items.len();
                out.items.push(Bookmark {
                    title: dict
                        .get::<hayro_syntax::object::String<'_>>("Title")
                        .map(|t| text_string(t.as_bytes()))
                        .unwrap_or_default(),
                    action: self.item_action(&dict),
                    children: Vec::new(),
                });
                match parent {
                    Some(p) => out.items[p].children.push(index),
                    None => out.roots.push(index),
                }
                if let Some(child) = first_child(&dict) {
                    chains.push((Some(index), child, depth + 1));
                }
                next = dict.get_raw::<Dict<'_>>("Next");
            }
        }
        out
    }

    /// A bookmark's dictionary, once: a second visit is a loop
    fn resolve<'a>(
        &'a self,
        item: MaybeRef<Dict<'a>>,
        seen: &mut HashSet<ObjectIdentifier>,
        loops: &mut usize,
    ) -> Option<Dict<'a>> {
        match item {
            MaybeRef::Ref(r) => {
                if !seen.insert(r.into()) {
                    *loops += 1;
                    return None;
                }
                self.pdf.xref().get::<Dict<'_>>(r.into())
            }
            MaybeRef::NotRef(d) => Some(d),
        }
    }

    /// A bookmark's `/A` action or `/Dest`
    fn item_action(&self, dict: &Dict<'_>) -> Action {
        let mut a = match dict.get_raw::<Dict<'_>>("A") {
            Some(action) => self.action(action),
            None => Action::default(),
        };
        if a.page.is_none() {
            a.page = dict.get::<Object<'_>>("Dest").and_then(|d| self.dest_page(d));
        }
        a
    }

    /// An action and the actions chained after it through `/Next` (a dictionary or an array of them). A loop
    /// can only go through references, so only referenced actions are remembered.
    fn action<'a>(&'a self, first: MaybeRef<Dict<'a>>) -> Action {
        let mut out = Action::default();
        let mut queue: Vec<MaybeRef<Dict<'a>>> = vec![first];
        let mut seen: HashSet<ObjectIdentifier> = HashSet::new();
        let mut loops = 0;
        let mut taken = 0;
        while let Some(item) = queue.pop() {
            if taken > self.limits.next_actions {
                break;
            }
            taken += 1;
            let Some(a) = self.resolve(item, &mut seen, &mut loops) else {
                continue;
            };
            match a.get::<Name<'_>>("S").as_deref() {
                Some(b"GoTo") => {
                    if out.page.is_none() {
                        out.page = a.get::<Object<'_>>("D").and_then(|d| self.dest_page(d));
                    }
                }
                Some(b"JavaScript") => {
                    if let Some(js) = self.script(&a) {
                        out.scripts.push(js);
                    }
                }
                _ => {}
            }
            // Pushed in reverse, taken from the end: the chain's order is kept
            let mut next: Vec<MaybeRef<Dict<'a>>> = match a.get_raw::<Object<'a>>("Next") {
                Some(MaybeRef::Ref(r)) => vec![MaybeRef::Ref(r)],
                Some(MaybeRef::NotRef(Object::Dict(d))) => vec![MaybeRef::NotRef(d)],
                Some(MaybeRef::NotRef(Object::Array(arr))) => arr
                    .raw_iter()
                    .take(16)
                    .filter_map(|o| match o {
                        MaybeRef::Ref(r) => Some(MaybeRef::Ref(r)),
                        MaybeRef::NotRef(Object::Dict(d)) => Some(MaybeRef::NotRef(d)),
                        MaybeRef::NotRef(_) => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            next.reverse();
            queue.extend(next);
        }
        out
    }

    /// The text of a `JavaScript` action's `/JS`: a string or a stream, cut at the script limit
    fn script(&self, action: &Dict<'_>) -> Option<String> {
        let bytes: Vec<u8> = match action.get::<Object<'_>>("JS")? {
            Object::String(s) => s.as_bytes().to_vec(),
            Object::Stream(s) => s.decoded().ok()?.into_owned(),
            _ => return None,
        };
        let mut text = text_string(&bytes);
        if text.len() > self.limits.script_chars {
            let mut cut = self.limits.script_chars;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
        }
        Some(text)
    }

    /// The page an explicit destination (`[page /XYZ ...]`, or a dictionary with `/D`) leads to. Named
    /// destinations are looked up in the catalog's `/Dests`.
    fn dest_page(&self, dest: Object<'_>) -> Option<usize> {
        let arr: Array<'_> = match dest {
            Object::Array(a) => a,
            Object::Dict(d) => d.get::<Array<'_>>("D")?,
            Object::Name(n) => self.named_dest(&n)?,
            Object::String(s) => self.named_dest(&s)?,
            _ => return None,
        };
        let first = arr.raw_iter().next()?;
        match first {
            MaybeRef::Ref(r) => self.page_ids.get(&r.into()).copied(),
            // A page number, as remote destinations write it
            MaybeRef::NotRef(Object::Number(n)) => {
                let i = n.as_i64();
                (i >= 0 && (i as usize) < self.page_count()).then_some(i as usize)
            }
            _ => None,
        }
    }

    fn named_dest(&self, name: &[u8]) -> Option<Array<'_>> {
        let dests = self.catalog()?.get::<Dict<'_>>("Dests")?;
        match dests.get::<Object<'_>>(name)? {
            Object::Array(a) => Some(a),
            Object::Dict(d) => d.get::<Array<'_>>("D"),
            _ => None,
        }
    }

    /// The document-level scripts (the catalog's `/Names /JavaScript` name tree): name and script, in tree order
    pub fn document_scripts(&self) -> (Vec<(String, String)>, Option<String>) {
        let mut out = Vec::new();
        let Some(root) = self
            .catalog()
            .and_then(|c| c.get::<Dict<'_>>("Names"))
            .and_then(|n| n.get::<Dict<'_>>("JavaScript"))
        else {
            return (out, None);
        };
        let mut seen: HashSet<ObjectIdentifier> = HashSet::new();
        let mut loops = 0;
        // Nodes to visit, last first; kids pushed in reverse so the tree order is kept
        let mut todo: Vec<(MaybeRef<Dict<'_>>, usize)> = vec![(MaybeRef::NotRef(root), 0)];
        while let Some((node, depth)) = todo.pop() {
            let Some(node) = self.resolve(node, &mut seen, &mut loops) else {
                continue;
            };
            if let Some(names) = node.get::<Array<'_>>("Names") {
                let mut it = names.flex_iter();
                while let Some(key) = it.next::<hayro_syntax::object::String<'_>>() {
                    if out.len() >= self.limits.links {
                        return (
                            out,
                            Some(format!(
                                "only the first {} document scripts read",
                                self.limits.links
                            )),
                        );
                    }
                    let Some(action) = it.next::<Dict<'_>>() else {
                        break;
                    };
                    if let Some(js) = self.script(&action) {
                        out.push((text_string(key.as_bytes()), js));
                    }
                }
            }
            if depth < self.limits.outline_depth
                && let Some(kids) = node.get::<Array<'_>>("Kids")
            {
                let mut k: Vec<MaybeRef<Dict<'_>>> = kids
                    .raw_iter()
                    .filter_map(|o| match o {
                        MaybeRef::Ref(r) => Some(MaybeRef::Ref(r)),
                        MaybeRef::NotRef(Object::Dict(d)) => Some(MaybeRef::NotRef(d)),
                        MaybeRef::NotRef(_) => None,
                    })
                    .collect();
                k.reverse();
                todo.extend(k.into_iter().map(|n| (n, depth + 1)));
            }
        }
        (out, None)
    }

    /// The link annotations of every page, in page order
    pub fn links(&self) -> (Vec<Link>, Option<String>) {
        let mut out = Vec::new();
        for (page, p) in self.pdf.pages().iter().enumerate() {
            let Some(annots) = p.raw().get::<Array<'_>>("Annots") else {
                continue;
            };
            for a in annots.iter::<Dict<'_>>() {
                if out.len() >= self.limits.links {
                    return (
                        out,
                        Some(format!("only the first {} links read", self.limits.links)),
                    );
                }
                if a.get::<Name<'_>>("Subtype").as_deref() != Some(b"Link") {
                    continue;
                }
                let Some(r) = a.get::<Rect>("Rect") else {
                    continue;
                };
                let mut action = match a.get_raw::<Dict<'_>>("A") {
                    Some(d) => self.action(d),
                    None => Action::default(),
                };
                if action.page.is_none() {
                    action.page = a.get::<Object<'_>>("Dest").and_then(|d| self.dest_page(d));
                }
                out.push(Link {
                    page,
                    rect: [
                        r.x0.min(r.x1),
                        r.y0.min(r.y1),
                        r.x0.max(r.x1),
                        r.y0.max(r.y1),
                    ],
                    action,
                });
            }
        }
        (out, None)
    }

    /// The words of every page, with forms followed, in content order
    pub fn words(&self) -> (Vec<Word>, TextGaps) {
        let mut w = TextWalk {
            doc: self,
            words: Vec::new(),
            gaps: TextGaps::default(),
            ops: 0,
            forms: 0,
            decoded: HashMap::new(),
        };
        for (page, p) in self.pdf.pages().iter().enumerate() {
            let Some(content) = p.page_stream() else {
                continue;
            };
            let mut path = Vec::new();
            if !w.run(page, content, p.resources(), IDENTITY, &mut path) {
                break;
            }
        }
        (w.words, w.gaps)
    }
}

/// Whether the bytes start like a PDF: `%PDF-` within the first kilobyte, as readers accept it
pub(crate) fn is_pdf(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(1024)]
        .windows(5)
        .any(|w| w == b"%PDF-")
}

fn first_child<'a>(d: &Dict<'a>) -> Option<MaybeRef<Dict<'a>>> {
    d.get_raw::<Dict<'a>>("First")
}

/// A PDF text string: UTF-16BE or UTF-8 with their byte order marks, otherwise PDFDocEncoding
pub(crate) fn text_string(b: &[u8]) -> String {
    if let Some(rest) = b.strip_prefix(b"\xFE\xFF") {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if let Some(rest) = b.strip_prefix(b"\xEF\xBB\xBF") {
        return String::from_utf8_lossy(rest).into_owned();
    }
    b.iter().map(|&c| pdf_doc_char(c)).collect()
}

/// PDFDocEncoding: Latin-1 except 0x18..0x1F and 0x80..0x9F, which hold typographic characters
fn pdf_doc_char(c: u8) -> char {
    const LOW: [char; 8] = ['˘', 'ˇ', 'ˆ', '˙', '˝', '˛', '˚', '˜'];
    const HIGH: [char; 32] = [
        '•', '†', '‡', '…', '—', '–', 'ƒ', '⁄', '‹', '›', '−', '‰', '„', '“', '”', '‘', '’', '‚', '™',
        'ﬁ', 'ﬂ', 'Ł', 'Œ', 'Š', 'Ÿ', 'Ž', 'ı', 'ł', 'œ', 'š', 'ž', '\u{FFFD}',
    ];
    match c {
        0x18..=0x1F => LOW[(c - 0x18) as usize],
        0x80..=0x9F => HIGH[(c - 0x80) as usize],
        0xA0 => '€',
        _ => c as char,
    }
}

type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` then `n`: a point is transformed by `m` first
fn mul(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn translate(tx: f64, ty: f64) -> Matrix {
    [1.0, 0.0, 0.0, 1.0, tx, ty]
}

fn num(n: &hayro_syntax::object::Number) -> f64 {
    let v = n.as_f64();
    if v.is_finite() { v } else { 0.0 }
}

/// The graphics and text state the word positions need
#[derive(Clone)]
struct State {
    ctm: Matrix,
    leading: f64,
    render: i64,
    /// The current font is a composite font
    composite: bool,
}

struct TextWalk<'d> {
    doc: &'d Doc,
    words: Vec<Word>,
    gaps: TextGaps,
    ops: usize,
    forms: usize,
    /// Form contents, decoded once: a form drawn many times is not inflated again each time
    decoded: HashMap<ObjectIdentifier, std::rc::Rc<[u8]>>,
}

impl TextWalk<'_> {
    /// Walks one content stream; `false` when a limit stopped the whole walk
    fn run(
        &mut self,
        page: usize,
        content: &[u8],
        resources: &Resources<'_>,
        ctm: Matrix,
        path: &mut Vec<ObjectIdentifier>,
    ) -> bool {
        let limits = &self.doc.limits;
        let mut stack: Vec<State> = Vec::new();
        let mut st = State {
            ctm,
            leading: 0.0,
            render: 0,
            composite: false,
        };
        let mut tm = IDENTITY;
        let mut tlm = IDENTITY;
        let mut ops = TypedIter::new(content);
        while let Some(op) = ops.next() {
            self.ops += 1;
            if self.ops > limits.operations {
                self.gaps.truncated.get_or_insert(format!(
                    "only the first {} content operations read",
                    limits.operations
                ));
                return false;
            }
            let shown: Option<Vec<u8>> = match op {
                TypedInstruction::SaveState(_) => {
                    // Unbalanced saves cost memory: a page that nests deeper than its operation budget is broken
                    if stack.len() < 4096 {
                        stack.push(st.clone());
                    }
                    None
                }
                TypedInstruction::RestoreState(_) => {
                    if let Some(s) = stack.pop() {
                        st = s;
                    }
                    None
                }
                TypedInstruction::Transform(t) => {
                    let m = [num(&t.0), num(&t.1), num(&t.2), num(&t.3), num(&t.4), num(&t.5)];
                    st.ctm = mul(&m, &st.ctm);
                    None
                }
                TypedInstruction::BeginText(_) => {
                    tm = IDENTITY;
                    tlm = IDENTITY;
                    None
                }
                TypedInstruction::TextFont(f) => {
                    st.composite = resources
                        .get_font(f.0)
                        .and_then(|d| d.get::<Name<'_>>("Subtype"))
                        .is_some_and(|s| s.as_ref() == b"Type0");
                    None
                }
                TypedInstruction::TextLeading(l) => {
                    st.leading = num(&l.0);
                    None
                }
                TypedInstruction::TextRenderingMode(m) => {
                    st.render = m.0.as_i64();
                    None
                }
                TypedInstruction::SetTextMatrix(m) => {
                    tlm = [num(&m.0), num(&m.1), num(&m.2), num(&m.3), num(&m.4), num(&m.5)];
                    tm = tlm;
                    None
                }
                TypedInstruction::NextLine(n) => {
                    tlm = mul(&translate(num(&n.0), num(&n.1)), &tlm);
                    tm = tlm;
                    None
                }
                TypedInstruction::NextLineAndSetLeading(n) => {
                    st.leading = -num(&n.1);
                    tlm = mul(&translate(num(&n.0), num(&n.1)), &tlm);
                    tm = tlm;
                    None
                }
                TypedInstruction::NextLineUsingLeading(_) => {
                    tlm = mul(&translate(0.0, -st.leading), &tlm);
                    tm = tlm;
                    None
                }
                TypedInstruction::ShowText(s) => Some(s.0.as_bytes().to_vec()),
                TypedInstruction::NextLineAndShowText(s) => {
                    tlm = mul(&translate(0.0, -st.leading), &tlm);
                    tm = tlm;
                    Some(s.0.as_bytes().to_vec())
                }
                TypedInstruction::ShowTextWithParameters(s) => {
                    tlm = mul(&translate(0.0, -st.leading), &tlm);
                    tm = tlm;
                    Some(s.2.as_bytes().to_vec())
                }
                TypedInstruction::ShowTexts(a) => {
                    let mut b = Vec::new();
                    for item in a.0.iter::<Object<'_>>() {
                        match item {
                            Object::String(s) => b.extend_from_slice(s.as_bytes()),
                            // A large negative kerning is how producers write a space
                            Object::Number(n) if num(&n) < -200.0 => b.push(b' '),
                            _ => {}
                        }
                    }
                    Some(b)
                }
                TypedInstruction::XObject(x) => {
                    if !self.form(page, resources, x.0, &st.ctm, path) {
                        return false;
                    }
                    None
                }
                _ => None,
            };
            let Some(bytes) = shown else {
                continue;
            };
            if st.composite {
                self.gaps.composite += 1;
                continue;
            }
            let at = mul(&tm, &st.ctm);
            let text: String = bytes.iter().map(|&c| c as char).collect();
            for word in text.split(|c: char| c.is_whitespace() || c.is_control()) {
                if word.is_empty() {
                    continue;
                }
                if self.words.len() >= limits.words {
                    self.gaps
                        .truncated
                        .get_or_insert(format!("only the first {} words read", limits.words));
                    return false;
                }
                self.words.push(Word {
                    page,
                    x: at[4],
                    y: at[5],
                    text: word.to_string(),
                    invisible: st.render == 3,
                });
            }
        }
        true
    }

    /// Walks a form XObject drawn with `Do`; images and loops are skipped
    fn form(
        &mut self,
        page: usize,
        resources: &Resources<'_>,
        name: &Name<'_>,
        ctm: &Matrix,
        path: &mut Vec<ObjectIdentifier>,
    ) -> bool {
        let Some(stream): Option<Stream<'_>> = resources.get_x_object(name) else {
            return true;
        };
        let dict = stream.dict();
        if dict.get::<Name<'_>>("Subtype").as_deref() != Some(b"Form") {
            return true;
        }
        let id = stream.obj_id();
        if path.contains(&id) || path.len() >= self.doc.limits.form_depth {
            return true;
        }
        self.forms += 1;
        if self.forms > self.doc.limits.forms {
            self.gaps.truncated.get_or_insert(format!(
                "only the first {} form drawings read",
                self.doc.limits.forms
            ));
            return false;
        }
        let content = match self.decoded.get(&id) {
            Some(c) => c.clone(),
            None => {
                let Ok(c) = stream.decoded() else {
                    return true;
                };
                let c: std::rc::Rc<[u8]> = c.into_owned().into();
                self.decoded.insert(id, c.clone());
                c
            }
        };
        let m: Matrix = dict
            .get::<Array<'_>>("Matrix")
            .map(|a| {
                let v: Vec<f64> = a.iter::<f64>().take(6).collect();
                if v.len() == 6 && v.iter().all(|x| x.is_finite()) {
                    [v[0], v[1], v[2], v[3], v[4], v[5]]
                } else {
                    IDENTITY
                }
            })
            .unwrap_or(IDENTITY);
        let own = dict.get::<Dict<'_>>("Resources").map(Resources::new);
        path.push(id);
        let go_on = self.run(
            page,
            &content,
            own.as_ref().unwrap_or(resources),
            mul(&m, ctm),
            path,
        );
        path.pop();
        go_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_strings() {
        assert_eq!(text_string(b"\xFE\xFF\x00U\x001"), "U1");
        assert_eq!(text_string(b"R1\x84"), "R1—");
        assert_eq!(text_string(b"\xEF\xBB\xBF\xC2\xB5"), "µ");
        assert_eq!(text_string(b"\xFE\xFF\x00"), "");
    }

    #[test]
    fn matrices() {
        let m = mul(&translate(1.0, 2.0), &[2.0, 0.0, 0.0, 2.0, 10.0, 20.0]);
        assert_eq!(m, [2.0, 0.0, 0.0, 2.0, 12.0, 24.0]);
    }

    #[test]
    fn not_pdf() {
        assert_eq!(
            Doc::open(b"hello".to_vec(), Limits::default()).err(),
            Some(OpenError::NotPdf)
        );
        let big = Limits {
            bytes: 4,
            ..Limits::default()
        };
        assert!(matches!(
            Doc::open(b"%PDF-1.7".to_vec(), big).err(),
            Some(OpenError::TooLarge { .. })
        ));
    }
}
