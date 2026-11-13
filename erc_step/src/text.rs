//! The text fix before parsing.

/// Rewrites a `$` (not given) in the first parameter of a simple entity to `''`: `#5=ADVANCED_FACE($,(#3),#4,.T.);`
/// becomes `#5=ADVANCED_FACE('',(#3),#4,.T.);`. The first parameter is the entity's name, a string, in every entity
/// truck reads. Some exporters (TI, Toshiba, Renesas, ADI and IXYS models among them) write `$` there, and truck
/// then drops the whole entity, with up to 40 % of a model's faces. Strings are skipped, so text in a string is
/// never changed. Returns the input unchanged (borrowed) when nothing needs a fix.
pub fn fix_unnamed(text: &str) -> std::borrow::Cow<'_, str> {
    let b = text.as_bytes();
    let mut out: Option<String> = None;
    let mut copied = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\'' => {
                // a string: '' is an escaped quote inside it
                i += 1;
                while i < b.len() {
                    if b[i] == b'\'' {
                        if b.get(i + 1) == Some(&b'\'') {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                // a comment
                i = text[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
            }
            b'=' => {
                // `= KEYWORD (` then spaces, `$`, spaces, and `,` or `)`
                let mut j = skip_space(b, i + 1);
                let kw_start = j;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                    j += 1;
                }
                if j == kw_start || !b[kw_start].is_ascii_alphabetic() {
                    i += 1;
                    continue;
                }
                j = skip_space(b, j);
                if b.get(j) != Some(&b'(') {
                    i = j;
                    continue;
                }
                let dollar = skip_space(b, j + 1);
                if b.get(dollar) == Some(&b'$') {
                    let after = skip_space(b, dollar + 1);
                    if matches!(b.get(after), Some(b',') | Some(b')')) {
                        let o = out.get_or_insert_with(|| String::with_capacity(text.len() + 64));
                        o.push_str(&text[copied..dollar]);
                        o.push_str("''");
                        copied = dollar + 1;
                    }
                }
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    match out {
        Some(mut o) => {
            o.push_str(&text[copied..]);
            std::borrow::Cow::Owned(o)
        }
        None => std::borrow::Cow::Borrowed(text),
    }
}

fn skip_space(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::fix_unnamed;

    #[test]
    fn first_dollar_becomes_empty_name() {
        assert_eq!(
            fix_unnamed("#5=ADVANCED_FACE($,(#3),#4,.T.);"),
            "#5=ADVANCED_FACE('',(#3),#4,.T.);"
        );
        assert_eq!(
            fix_unnamed("#5 = VERTEX_POINT ( $ ) ;"),
            "#5 = VERTEX_POINT ( '' ) ;"
        );
        assert_eq!(
            fix_unnamed("#6=AXIS2_PLACEMENT_3D('',#1,$,$);"),
            "#6=AXIS2_PLACEMENT_3D('',#1,$,$);"
        );
    }

    #[test]
    fn strings_and_complex_entities_untouched() {
        let s = "#1=PRODUCT('a=B($,x)','',$,(#2));\n#2=(NAMED_UNIT($)LENGTH_UNIT());";
        assert_eq!(fix_unnamed(s), s);
        assert!(matches!(fix_unnamed(s), std::borrow::Cow::Borrowed(_)));
    }
}
