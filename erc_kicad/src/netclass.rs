//! Each net's effective net class as KiCad resolves it (`NET_SETTINGS::GetEffectiveNetClass`): the classes the
//! net's directive labels name, and the classes whose patterns match the net's full name (a pattern matches as
//! an anchored regex when it is one, or as an anchored wildcard with `*` and `?`). None: `Default`. Several, or
//! one that leaves a rule unset: the classes by priority (a lower number first, then by name), with `Default`
//! last when it fills a rule none of them sets, written as KiCad's netlist writes them: `Analog,Default`.

use erc_model::*;
use regex::Regex;

/// The rules a class must set for the effective class not to need `Default` (`NET_SETTINGS::addMissingDefaults`)
const REQUIRED: &[&str] = &[
    "clearance",
    "track_width",
    "via_diameter",
    "via_drill",
    "microvia_diameter",
    "microvia_drill",
    "diff_pair_width",
    "diff_pair_gap",
    "wire_width",
    "bus_width",
];

/// A net class pattern as KiCad's matchers for net classes read it (`EDA_COMBINED_MATCHER`, `CTX_NETCLASS`)
struct Pattern {
    regex: Option<Regex>,
    wildcard: Regex,
}

impl Pattern {
    fn new(p: &str) -> Pattern {
        let anchored = |s: &str| {
            let s = s.strip_prefix('^').unwrap_or(s);
            let s = s.strip_suffix('$').unwrap_or(s);
            format!("^(?:{s})$")
        };
        let mut wild = String::new();
        for c in p.chars() {
            match c {
                '?' => wild.push('.'),
                '*' => wild.push_str(".*"),
                c if ".*+?^${}()|[]/\\".contains(c) => {
                    wild.push('\\');
                    wild.push(c);
                }
                c => wild.push(c),
            }
        }
        Pattern {
            regex: Regex::new(&anchored(p)).ok(),
            wildcard: Regex::new(&format!("^{wild}$")).expect("an escaped wildcard is a regex"),
        }
    }

    fn matches(&self, name: &str) -> bool {
        self.regex.as_ref().is_some_and(|r| r.is_match(name)) || self.wildcard.is_match(name)
    }
}

/// A class's priority: its `priority` rule, `Default` the lowest; a class only a label names comes just above
fn priority(class: Option<&NetClass>, name: &str) -> i64 {
    let given = class.and_then(|c| {
        c.params
            .iter()
            .find(|p| p.key == "priority")
            .and_then(|p| p.value.get()?.parse::<i64>().ok())
    });
    match given {
        Some(p) => p,
        None if name == "Default" => i32::MAX as i64,
        None => i32::MAX as i64 - 1,
    }
}

/// The rules of [`REQUIRED`] a class sets
fn set_rules(class: Option<&NetClass>) -> Vec<&'static str> {
    REQUIRED
        .iter()
        .copied()
        .filter(|r| class.is_some_and(|c| c.params.iter().any(|p| p.key == *r)))
        .collect()
}

/// Sets every net's class to KiCad's effective class, with `source` as its provenance; the classes named by
/// directive labels on the net and the project's net classes and patterns are what it is resolved from
pub fn resolve(board: &mut Board, source: &Provenance) {
    let patterns: Vec<(Pattern, &str)> = board
        .net_classes
        .iter()
        .flat_map(|c| {
            c.patterns
                .iter()
                .map(|p| (Pattern::new(p), c.name.as_str()))
        })
        .collect();
    let mut classes: Vec<Vec<String>> = vec![Vec::new(); board.nets.len()];
    for l in &board.labels {
        if l.kind != LabelKind::Directive {
            continue;
        }
        let Some(n) = l.net else { continue };
        for (k, v) in &l.fields {
            if k == "Netclass" && !v.is_empty() {
                classes[n.index()].push(v.clone());
            }
        }
    }
    for (i, net) in board.nets.iter().enumerate() {
        for (p, class) in &patterns {
            if p.matches(&net.name) {
                classes[i].push(class.to_string());
            }
        }
    }
    let lookup = |name: &str| board.net_classes.iter().find(|c| c.name == name);
    let mut effective = Vec::with_capacity(board.nets.len());
    for mut names in classes {
        names.sort();
        names.dedup();
        if names.is_empty() {
            effective.push("Default".to_string());
            continue;
        }
        names.sort_by(|a, b| {
            (priority(lookup(a), a), a.as_str()).cmp(&(priority(lookup(b), b), b.as_str()))
        });
        let set: std::collections::BTreeSet<&str> = names
            .iter()
            .flat_map(|n| {
                if n == "Default" {
                    REQUIRED.to_vec()
                } else {
                    set_rules(lookup(n))
                }
            })
            .collect();
        if set.len() < REQUIRED.len() && !names.iter().any(|n| n == "Default") {
            names.push("Default".to_string());
        }
        effective.push(names.join(","));
    }
    for (net, class) in board.nets.iter_mut().zip(effective) {
        net.class = Facts::one(Fact::new(class, source.clone()).with_note("effective net class"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns() {
        assert!(Pattern::new("+*").matches("+3V3"));
        assert!(!Pattern::new("+*").matches("/+3V3"));
        assert!(Pattern::new("*/+*").matches("/Power/+5V"));
        assert!(Pattern::new("GND").matches("GND"));
        assert!(!Pattern::new("GND").matches("AGND"));
        assert!(Pattern::new("USB_?").matches("USB_P"));
        assert!(Pattern::new("/USB_[PN]/").matches("/USB_P/"));
        assert!(Pattern::new("D[0-9]+").matches("D12"));
    }
}
