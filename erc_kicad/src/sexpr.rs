//! A small s-expression reader for KiCad files: lists, bare atoms, quoted strings and `|raw|` blobs (embedded
//! files). Borrows from the input where it can, never recurses (deeply nested input can't overflow the stack)
//! and never panics: bad input is an `Err` with the byte offset.

use std::borrow::Cow;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Sexp<'a> {
    List(Vec<Sexp<'a>>),
    /// Bare token: symbol, keyword or number, kept as written
    Atom(&'a str),
    /// Quoted string, unescaped
    Str(Cow<'a, str>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for ParseError {}

/// Real KiCad files nest about 10 deep. The limit keeps the recursive printers (and `Drop`) off the stack edge.
const MAX_DEPTH: usize = 500;

static EMPTY: [Sexp<'static>; 0] = [];

impl<'a> Sexp<'a> {
    /// All items of a list, nothing for an atom
    pub fn items(&self) -> &[Sexp<'a>] {
        match self {
            Sexp::List(items) => items,
            _ => &EMPTY,
        }
    }

    /// First item of a list when it is an atom: `symbol` in `(symbol ...)`
    pub fn head(&self) -> Option<&str> {
        match self.items().first() {
            Some(Sexp::Atom(a)) => Some(a),
            _ => None,
        }
    }

    /// Items after the head
    pub fn args(&self) -> &[Sexp<'a>] {
        self.items().get(1..).unwrap_or(&[])
    }

    /// Text of an atom or string
    pub fn text(&self) -> Option<&str> {
        match self {
            Sexp::Atom(a) => Some(a),
            Sexp::Str(s) => Some(s),
            Sexp::List(_) => None,
        }
    }

    /// Text of the n-th argument
    pub fn arg(&self, n: usize) -> Option<&str> {
        self.args().get(n).and_then(Sexp::text)
    }

    /// Child lists with this head
    pub fn children<'s>(&'s self, head: &'s str) -> impl Iterator<Item = &'s Sexp<'a>> + 's {
        self.args().iter().filter(move |c| c.head() == Some(head))
    }

    /// First child list with this head
    pub fn child(&self, head: &str) -> Option<&Sexp<'a>> {
        self.args().iter().find(|c| c.head() == Some(head))
    }

    /// First argument of the first child with this head: `value("unit")` on `(unit 1)` is `1`
    pub fn value(&self, head: &str) -> Option<&str> {
        self.child(head).and_then(|c| c.arg(0))
    }

    /// `(head yes)` is true, `(head no)` false, `(head)` true, missing `None`
    pub fn flag(&self, head: &str) -> Option<bool> {
        self.child(head)
            .map(|c| !matches!(c.arg(0), Some("no" | "false")))
    }

    /// Number of an argument, `None` when missing or not a number
    pub fn num(&self, n: usize) -> Option<f64> {
        self.arg(n).and_then(|s| s.parse().ok())
    }
}

/// Parses all top-level expressions of `text`
pub fn parse(text: &str) -> Result<Vec<Sexp<'_>>, ParseError> {
    let bytes = text.as_bytes();
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    let mut open_at: Vec<usize> = Vec::new();
    let mut i = 0;
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
                stack.push(Vec::new());
                open_at.push(i);
                i += 1;
            }
            b')' => {
                if stack.len() < 2 {
                    return Err(ParseError {
                        offset: i,
                        message: "unexpected ')'",
                    });
                }
                let list = stack.pop().unwrap_or_default();
                open_at.pop();
                push(&mut stack, Sexp::List(list));
                i += 1;
            }
            b'"' => {
                let (s, end) = string(text, i)?;
                push(&mut stack, Sexp::Str(s));
                i = end;
            }
            b'|' => {
                // Raw blob, e.g. base64 data of embedded files: everything up to the next '|'
                let end = text[i + 1..].find('|').ok_or(ParseError {
                    offset: i,
                    message: "unterminated |raw| blob",
                })?;
                push(&mut stack, Sexp::Atom(&text[i..i + end + 2]));
                i += end + 2;
            }
            _ => {
                let start = i;
                while i < bytes.len()
                    && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')' | b'"')
                {
                    i += 1;
                }
                push(&mut stack, Sexp::Atom(&text[start..i]));
            }
        }
    }
    if let Some(&offset) = open_at.last() {
        return Err(ParseError {
            offset,
            message: "unclosed '('",
        });
    }
    Ok(stack.pop().unwrap_or_default())
}

fn push<'a>(stack: &mut [Vec<Sexp<'a>>], item: Sexp<'a>) {
    if let Some(top) = stack.last_mut() {
        top.push(item);
    }
}

/// Quoted string starting at `start` (the opening quote); returns the text and the offset after the closing quote
fn string(text: &str, start: usize) -> Result<(Cow<'_, str>, usize), ParseError> {
    let bytes = text.as_bytes();
    let mut i = start + 1;
    let mut owned: Option<String> = None;
    let mut plain_from = i;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let s = match owned {
                    Some(mut s) => {
                        s.push_str(&text[plain_from..i]);
                        Cow::Owned(s)
                    }
                    None => Cow::Borrowed(&text[plain_from..i]),
                };
                return Ok((s, i + 1));
            }
            b'\\' if i + 1 < bytes.len() => {
                let s = owned.get_or_insert_with(String::new);
                s.push_str(&text[plain_from..i]);
                let next = bytes[i + 1];
                match next {
                    b'n' => s.push('\n'),
                    b't' => s.push('\t'),
                    b'r' => s.push('\r'),
                    b'\\' => s.push('\\'),
                    b'"' => s.push('"'),
                    _ => s.push('\\'),
                }
                // An unknown escape keeps the backslash and goes on with the next char as plain text
                if matches!(next, b'n' | b't' | b'r' | b'\\' | b'"') {
                    i += 2;
                } else {
                    i += 1;
                }
                plain_from = i;
            }
            _ => i += 1,
        }
    }
    Err(ParseError {
        offset: start,
        message: "unterminated string",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_atoms_strings() {
        let v = parse(r#"(a 1 "two words" (b "x\"y\\z\nw") |raw (data)|)"#).unwrap();
        assert_eq!(v.len(), 1);
        let a = &v[0];
        assert_eq!(a.head(), Some("a"));
        assert_eq!(a.arg(0), Some("1"));
        assert_eq!(a.arg(1), Some("two words"));
        assert_eq!(a.value("b"), Some("x\"y\\z\nw"));
        assert_eq!(a.arg(3), Some("|raw (data)|"));
        assert_eq!(a.num(0), Some(1.0));
    }

    #[test]
    fn flags() {
        let v = parse("(s (dnp yes) (in_bom no) (power))").unwrap();
        assert_eq!(v[0].flag("dnp"), Some(true));
        assert_eq!(v[0].flag("in_bom"), Some(false));
        assert_eq!(v[0].flag("power"), Some(true));
        assert_eq!(v[0].flag("on_board"), None);
    }

    #[test]
    fn errors_not_panics() {
        assert!(parse("(a (b)").is_err());
        assert!(parse("a)").is_err());
        assert!(parse("(a \"open").is_err());
        assert!(parse("(a |open").is_err());
        assert!(parse("(a \"trailing backslash\\").is_err());
        assert_eq!(parse("").unwrap(), vec![]);
        // UTF-8 and an unknown escape
        let v = parse("(t \"ドングル \\q\")").unwrap();
        assert_eq!(v[0].arg(0), Some("ドングル \\q"));
    }

    #[test]
    fn deep_nesting_is_an_error() {
        let text = "(".repeat(200_000) + &")".repeat(200_000);
        assert_eq!(parse(&text).unwrap_err().message, "nesting too deep");
        let text = "(".repeat(100) + &")".repeat(100);
        assert!(parse(&text).is_ok());
    }
}
