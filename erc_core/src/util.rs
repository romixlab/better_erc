use regex::Regex;

pub(crate) fn collapse_underscores(input: &str) -> String {
    let re = Regex::new(r"_+").unwrap();
    re.replace_all(input, "_").to_string()
}
