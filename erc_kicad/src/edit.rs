//! Formatting-preserving edits of a KiCad s-expression file: the file is parsed into a [`Tree`] that knows where
//! every expression is in the text, changes are collected as [`Edits`] on byte ranges, and only those ranges are
//! written new. Everything else (whitespace, the order of items, number formats, what KiCad would write
//! differently today) stays byte for byte, so a file read and written without a change is the same file, and a
//! change is one diff hunk.
//!
//! What is edited on top of this: [`crate::symlib::set_property`], a property of a library symbol.

use crate::sexpr::{ParseError, string};
use std::borrow::Cow;
use std::ops::Range;

/// One expression and where it is in the text
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// From `(` to after `)` for a list, the token for an atom, the quotes included for a string
    pub span: Range<usize>,
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    List(Vec<Node>),
    /// Bare token (symbol, keyword, number) or a `|raw|` blob
    Atom,
    /// Quoted string
    Str,
}

/// A parsed text whose expressions know their place in it
#[derive(Clone, Debug)]
pub struct Tree<'a> {
    text: &'a str,
    roots: Vec<Node>,
}

/// Same limit as [`crate::sexpr::parse`]
const MAX_DEPTH: usize = 500;

impl<'a> Tree<'a> {
    /// Parses all top-level expressions of `text`, as [`crate::sexpr::parse`] reads them
    pub fn parse(text: &'a str) -> Result<Tree<'a>, ParseError> {
        let bytes = text.as_bytes();
        // Open lists: where each starts and its items so far
        let mut stack: Vec<(usize, Vec<Node>)> = vec![(0, Vec::new())];
        let mut i = 0;
        let push = |stack: &mut Vec<(usize, Vec<Node>)>, n: Node| {
            if let Some(top) = stack.last_mut() {
                top.1.push(n);
            }
        };
        while i < bytes.len() {
            match bytes[i] {
                b' ' | b'\t' | b'\n' | b'\r' => i += 1,
                b'(' => {
                    if stack.len() > MAX_DEPTH {
                        return Err(ParseError {
                            offset: i,
                            message: "nesting too deep",
                        });
                    }
                    stack.push((i, Vec::new()));
                    i += 1;
                }
                b')' => {
                    if stack.len() < 2 {
                        return Err(ParseError {
                            offset: i,
                            message: "unexpected ')'",
                        });
                    }
                    let (start, items) = stack.pop().unwrap_or_default();
                    push(
                        &mut stack,
                        Node {
                            span: start..i + 1,
                            kind: Kind::List(items),
                        },
                    );
                    i += 1;
                }
                b'"' => {
                    let (_, end) = string(text, i)?;
                    push(
                        &mut stack,
                        Node {
                            span: i..end,
                            kind: Kind::Str,
                        },
                    );
                    i = end;
                }
                b'|' => {
                    let end = text[i + 1..].find('|').ok_or(ParseError {
                        offset: i,
                        message: "unterminated |raw| blob",
                    })?;
                    push(
                        &mut stack,
                        Node {
                            span: i..i + end + 2,
                            kind: Kind::Atom,
                        },
                    );
                    i += end + 2;
                }
                _ => {
                    let start = i;
                    while i < bytes.len()
                        && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')' | b'"')
                    {
                        i += 1;
                    }
                    push(
                        &mut stack,
                        Node {
                            span: start..i,
                            kind: Kind::Atom,
                        },
                    );
                }
            }
        }
        if stack.len() > 1 {
            return Err(ParseError {
                offset: stack.last().map_or(0, |s| s.0),
                message: "unclosed '('",
            });
        }
        Ok(Tree {
            text,
            roots: stack.pop().map(|s| s.1).unwrap_or_default(),
        })
    }

    pub fn text(&self) -> &'a str {
        self.text
    }

    /// The top-level expressions
    pub fn roots(&self) -> &[Node] {
        &self.roots
    }

    /// The first top-level list with this head
    pub fn root(&self, head: &str) -> Option<&Node> {
        self.roots.iter().find(|r| self.head(r) == Some(head))
    }

    /// The text of `n` as written
    pub fn source(&self, n: &Node) -> &'a str {
        &self.text[n.span.clone()]
    }

    /// The value of an atom (as written) or a string (unescaped); `None` for a list
    pub fn value(&self, n: &Node) -> Option<Cow<'a, str>> {
        match n.kind {
            Kind::Atom => Some(Cow::Borrowed(self.source(n))),
            Kind::Str => string(self.text, n.span.start).ok().map(|(s, _)| s),
            Kind::List(_) => None,
        }
    }

    /// The head atom of a list: `symbol` in `(symbol ...)`
    pub fn head(&self, n: &Node) -> Option<&'a str> {
        match items(n).first() {
            Some(
                h @ Node {
                    kind: Kind::Atom, ..
                },
            ) => Some(self.source(h)),
            _ => None,
        }
    }

    /// The value of the n-th item after the head
    pub fn arg(&self, n: &Node, i: usize) -> Option<Cow<'a, str>> {
        items(n).get(i + 1).and_then(|a| self.value(a))
    }

    /// Child lists with this head
    pub fn children<'t>(
        &'t self,
        n: &'t Node,
        head: &'t str,
    ) -> impl Iterator<Item = &'t Node> + 't {
        items(n)
            .iter()
            .skip(1)
            .filter(move |c| self.head(c) == Some(head))
    }

    /// The first child list with this head
    pub fn child<'t>(&'t self, n: &'t Node, head: &str) -> Option<&'t Node> {
        items(n).iter().skip(1).find(|c| self.head(c) == Some(head))
    }

    /// The whitespace before `n` on its line, when only whitespace is before it there
    pub fn indent(&self, n: &Node) -> Option<&'a str> {
        let line = self.text[..n.span.start].rfind('\n').map_or(0, |i| i + 1);
        let before = &self.text[line..n.span.start];
        before
            .bytes()
            .all(|b| b == b' ' || b == b'\t')
            .then_some(before)
    }

    /// The text's line break: `\r\n` when it uses that, else `\n`
    pub fn newline(&self) -> &'static str {
        if self.text.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    }

    /// One level of indent: a tab when the text indents with tabs (KiCad 8+), else two spaces
    pub fn indent_unit(&self) -> &'static str {
        if self.text.contains("\n\t") {
            "\t"
        } else {
            "  "
        }
    }
}

/// The items of a list, nothing for an atom or string
pub fn items(n: &Node) -> &[Node] {
    match &n.kind {
        Kind::List(items) => items,
        _ => &[],
    }
}

/// `s` as a quoted string the way KiCad writes one: `"` and `\` escaped, line breaks and tabs as `\n`, `\r`, `\t`
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Changes to a text as byte ranges and what replaces them
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Edits {
    list: Vec<(Range<usize>, String)>,
}

/// Two edits touch the same bytes
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlap {
    pub at: usize,
}

impl Edits {
    pub fn new() -> Edits {
        Edits::default()
    }

    /// `range` of the text becomes `with`
    pub fn replace(&mut self, range: Range<usize>, with: impl Into<String>) {
        self.list.push((range, with.into()));
    }

    /// `text` goes in at byte `at`
    pub fn insert(&mut self, at: usize, text: impl Into<String>) {
        self.list.push((at..at, text.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// `text` with every edit made; inserts at one place keep the order they were made in
    pub fn apply(&self, text: &str) -> Result<String, Overlap> {
        let mut list: Vec<&(Range<usize>, String)> = self.list.iter().collect();
        list.sort_by_key(|(r, _)| (r.start, r.end));
        let mut out =
            String::with_capacity(text.len() + list.iter().map(|e| e.1.len()).sum::<usize>());
        let mut at = 0;
        for (r, with) in list {
            if r.start < at || r.end > text.len() || r.start > r.end {
                return Err(Overlap { at: r.start });
            }
            out.push_str(&text[at..r.start]);
            out.push_str(with);
            at = r.end;
        }
        out.push_str(&text[at..]);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_and_values() {
        let text = "(a 1 \"two \\\"q\\\"\" (b |raw)| x))\n(c)";
        let t = Tree::parse(text).unwrap();
        assert_eq!(t.roots().len(), 2);
        let a = &t.roots()[0];
        assert_eq!(t.source(a), "(a 1 \"two \\\"q\\\"\" (b |raw)| x))");
        assert_eq!(t.head(a), Some("a"));
        assert_eq!(t.arg(a, 0).as_deref(), Some("1"));
        assert_eq!(t.arg(a, 1).as_deref(), Some("two \"q\""));
        let b = t.child(a, "b").unwrap();
        assert_eq!(t.source(b), "(b |raw)| x)");
        assert_eq!(t.arg(b, 0).as_deref(), Some("|raw)|"));
        assert_eq!(t.root("c").map(|c| t.source(c)), Some("(c)"));
    }

    /// The same errors as the reader, never a panic
    #[test]
    fn errors() {
        for bad in ["(a (b)", "a)", "(a \"open", "(a |open"] {
            assert!(Tree::parse(bad).is_err(), "{bad}");
        }
        assert!(Tree::parse(&("(".repeat(1000) + &")".repeat(1000))).is_err());
    }

    #[test]
    fn edits_apply_in_place() {
        let text = "(a (b 1)\n\t(c 2))";
        let t = Tree::parse(text).unwrap();
        let a = &t.roots()[0];
        let c = t.child(a, "c").unwrap();
        assert_eq!(t.indent(c), Some("\t"));
        assert_eq!(t.indent(t.child(a, "b").unwrap()), None);
        let mut e = Edits::new();
        assert_eq!(e.apply(text).unwrap(), text);
        e.replace(items(c)[1].span.clone(), "3");
        e.insert(c.span.end, "\n\t(d)");
        e.insert(c.span.end, " ; after d");
        assert_eq!(
            e.apply(text).unwrap(),
            "(a (b 1)\n\t(c 3)\n\t(d) ; after d)"
        );
        e.replace(0..3, "x");
        e.replace(1..2, "y");
        assert!(e.apply(text).is_err());
    }

    #[test]
    fn quoting() {
        assert_eq!(quote("fp1:00ff"), "\"fp1:00ff\"");
        assert_eq!(quote("a\"b\\c\nd"), r#""a\"b\\c\nd""#);
        let q = quote("x\"\\\n\t\ry ü");
        let t = Tree::parse(&q).unwrap();
        assert_eq!(t.value(&t.roots()[0]).as_deref(), Some("x\"\\\n\t\ry ü"));
    }
}
