//! String literals out of the JavaScript a PDF carries, read as text and never run: the arguments of a call
//! (`app.popUpMenu("Value: 10K", ...)`, `_FindWord(0, "PIU10G9")`) and the names of functions defined or called.
//! Only literals are taken; any expression in their place is skipped, so a script cannot make the reader compute
//! anything.

/// The arguments of every call to `name` in `script`: string literals (also inside nested arrays, flattened in
/// order) and integer literals (as their text); any other argument is left out
pub(crate) fn call_args(script: &str, name: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let b = script.as_bytes();
    let mut from = 0;
    while let Some(i) = script[from..].find(name) {
        let start = from + i;
        from = start + name.len();
        // A whole identifier: `x_FindWord(` is not `_FindWord(`
        if start > 0 && is_ident(b[start - 1]) {
            continue;
        }
        let mut j = skip_space(b, from);
        if b.get(j) != Some(&b'(') {
            continue;
        }
        j += 1;
        let (args, end) = arguments(script, j);
        from = end.max(from);
        out.push(args);
    }
    out
}

/// The function names a script defines with `function NAME(`
pub(crate) fn defined_functions(script: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = script[from..].find("function") {
        let start = from + i;
        from = start + "function".len();
        if start > 0 && is_ident(script.as_bytes()[start - 1]) {
            continue;
        }
        let b = script.as_bytes();
        let j = skip_space(b, from);
        let k = ident_end(b, j);
        if k > j && b.get(skip_space(b, k)) == Some(&b'(') {
            out.push(&script[j..k]);
        }
    }
    out
}

/// The functions a script calls without arguments (`ShowCompProps_02B9();`), in order
pub(crate) fn called_functions(script: &str) -> Vec<&str> {
    let b = script.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if is_ident(b[i]) && !b[i].is_ascii_digit() && (i == 0 || !is_ident(b[i - 1])) {
            let k = ident_end(b, i);
            let p = skip_space(b, k);
            if b.get(p) == Some(&b'(') && b.get(skip_space(b, p + 1)) == Some(&b')') {
                let name = &script[i..k];
                if name != "function" {
                    out.push(name);
                }
            }
            i = k;
        } else {
            i += 1;
        }
    }
    out
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

fn ident_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_ident(b[i]) {
        i += 1;
    }
    i
}

fn skip_space(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// The literal arguments from just after `(` to the matching `)`; returns them and the index after `)`
fn arguments(s: &str, mut i: usize) -> (Vec<String>, usize) {
    let b = s.as_bytes();
    let mut args = Vec::new();
    // Brackets opened inside the call; a literal only counts where nothing but brackets and commas surround it
    let mut depth: usize = 0;
    let mut plain = true;
    let mut pending: Option<String> = None;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' => {
                let (lit, end) = string_literal(s, i);
                i = end;
                if pending.is_some() {
                    plain = false;
                }
                pending = Some(lit);
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                if depth == 0 {
                    if b[i] == b')' {
                        if let (Some(p), true) = (pending.take(), plain) {
                            args.push(p);
                        }
                        return (args, i + 1);
                    }
                } else {
                    depth -= 1;
                }
                if b[i] == b']'
                    && let (Some(p), true) = (pending.take(), plain)
                {
                    args.push(p);
                }
            }
            b',' => {
                if let (Some(p), true) = (pending.take(), plain) {
                    args.push(p);
                }
                pending = None;
                plain = true;
            }
            c if c.is_ascii_digit() && pending.is_none() && (i == 0 || !is_ident(b[i - 1])) => {
                let k = ident_end(b, i);
                let t = &s[i..k];
                if t.bytes().all(|c| c.is_ascii_digit()) {
                    pending = Some(t.to_string());
                } else {
                    plain = false;
                }
                i = k;
                continue;
            }
            c if c.is_ascii_whitespace() => {}
            _ => {
                // An operator, identifier or anything else: this argument is an expression, not a literal
                plain = false;
            }
        }
        i += 1;
    }
    (args, b.len())
}

/// A JavaScript string literal starting at the quote `s[i]`: its value and the index after the closing quote (the
/// end of the text when it is not closed)
fn string_literal(s: &str, i: usize) -> (String, usize) {
    let quote = s.as_bytes()[i] as char;
    let mut out = String::new();
    let mut chars = s[i + 1..].char_indices().peekable();
    let base = i + 1;
    while let Some((k, c)) = chars.next() {
        match c {
            c if c == quote => return (out, base + k + 1),
            '\\' => {
                let Some((_, e)) = chars.next() else { break };
                match e {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'v' => out.push('\u{b}'),
                    '0' => out.push('\0'),
                    // A line continuation
                    '\n' | '\u{2028}' | '\u{2029}' => {}
                    '\r' => {
                        if chars.peek().is_some_and(|(_, c)| *c == '\n') {
                            chars.next();
                        }
                    }
                    'x' => {
                        let h: String = (0..2).filter_map(|_| chars.next().map(|(_, c)| c)).collect();
                        push_code(&mut out, &h);
                    }
                    'u' => {
                        if chars.peek().is_some_and(|(_, c)| *c == '{') {
                            chars.next();
                            let mut h = String::new();
                            for (_, c) in chars.by_ref() {
                                if c == '}' || h.len() > 8 {
                                    break;
                                }
                                h.push(c);
                            }
                            push_code(&mut out, &h);
                        } else {
                            let h: String =
                                (0..4).filter_map(|_| chars.next().map(|(_, c)| c)).collect();
                            push_code(&mut out, &h);
                        }
                    }
                    other => out.push(other),
                }
            }
            c => out.push(c),
        }
    }
    (out, s.len())
}

fn push_code(out: &mut String, hex: &str) {
    match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
        Some(c) => out.push(c),
        None => out.push('\u{FFFD}'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROPS: &str = r#"function ShowCompProps_02B9E5D4(){var sChoice = app.popUpMenu("Comment: CAPACITOR","Footprint: CAPC0402M","Value: 100nF", ["Sub", "A: \"q\"", 'B: \u00b5\x41']); if (sChoice) {var nUrlPos = sChoice.search(/:\s((http|https|ftp):\/\/).+/); if (nUrlPos !== -1) {app.launchURL(sChoice.substr(nUrlPos+2), true);}}}"#;

    #[test]
    fn popup_menu_literals() {
        let calls = call_args(PROPS, "app.popUpMenu");
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            [
                "Comment: CAPACITOR",
                "Footprint: CAPC0402M",
                "Value: 100nF",
                "Sub",
                "A: \"q\"",
                "B: µA"
            ]
        );
        assert_eq!(defined_functions(PROPS), ["ShowCompProps_02B9E5D4"]);
    }

    #[test]
    fn find_word_and_calls() {
        let s = r#"_FindWord(0, "PIU10G9"); x_FindWord(1, "no"); _FindWord(a+1, "expr");"#;
        let calls = call_args(s, "_FindWord");
        assert_eq!(calls, [vec!["0".to_string(), "PIU10G9".into()], vec!["expr".into()]]);
        assert_eq!(
            called_functions("ShowCompProps_0EB7(); f(1); g( )"),
            ["ShowCompProps_0EB7", "g"]
        );
    }

    #[test]
    fn expressions_are_not_literals() {
        let calls = call_args(r#"f("a" + x, "b", 'c'.toUpperCase(), 12)"#, "f");
        assert_eq!(calls, [vec!["b".to_string(), "12".into()]]);
    }

    #[test]
    fn broken_scripts() {
        for s in ["f(", "f(\"abc", "f(\"\\", "f(\"\\u{110000}\")", "f(\"\\xZ\")", "f([[[", "f)"] {
            let _ = call_args(s, "f");
        }
        assert_eq!(call_args("f(\"\\u{110000}\")", "f"), [vec!["\u{FFFD}".to_string()]]);
    }
}
