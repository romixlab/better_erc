//! The catalogue sync test: every registered check matches its line in `docs/checks.md` (id, category, severity,
//! tier), has its detail page section, declares overlaps the catalogue marks with `≈`, and `list` reports every
//! other catalogue id as planned.

use erc_checks::catalogue::{catalogue, detail, needs_text};
use erc_checks::{CheckState, list, registered};
use std::collections::BTreeSet;

#[test]
fn registered_checks_match_the_catalogue() {
    let cat = catalogue();
    let mut ids = BTreeSet::new();
    let mut wrong = Vec::new();
    for c in registered() {
        let m = c.meta();
        assert!(ids.insert(m.id), "{} registered twice", m.id);
        let Some(e) = cat.entry(m.id) else {
            wrong.push(format!("{}: not in docs/checks.md", m.id));
            continue;
        };
        if m.category != e.category {
            wrong.push(format!(
                "{}: category {} in code, {} in the catalogue",
                m.id, m.category, e.category
            ));
        }
        if m.severity != e.severity {
            wrong.push(format!(
                "{}: severity {} in code, {} in the catalogue",
                m.id, m.severity, e.severity
            ));
        }
        if m.tier != e.tier {
            wrong.push(format!(
                "{}: tier {} in code, {} in the catalogue",
                m.id,
                m.tier.name(),
                e.tier.name()
            ));
        }
        if detail(m.id).is_none() || needs_text(m.id).is_none() {
            wrong.push(format!(
                "{}: no section with a `needs:` line on docs/checks/{}.md",
                m.id, e.page
            ));
        }
        for o in m.overlaps {
            let marked = |a: &str, b: &str| {
                cat.entry(a)
                    .is_some_and(|x| x.title.contains(&format!("≈ {b}")))
            };
            if !(marked(m.id, o) || marked(o, m.id)) {
                wrong.push(format!(
                    "{}: overlaps {o}, but neither catalogue line marks it with ≈",
                    m.id
                ));
            }
        }
        let mut params = BTreeSet::new();
        for p in m.params {
            if !params.insert(p.name) {
                wrong.push(format!("{}: param {} twice", m.id, p.name));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn list_reports_the_rest_as_planned() {
    let all = list();
    assert_eq!(all.len(), catalogue().entries.len());
    let implemented: BTreeSet<&str> = all
        .iter()
        .filter(|c| c.state == CheckState::Implemented)
        .map(|c| c.id.as_str())
        .collect();
    let registered: BTreeSet<&str> = registered().iter().map(|c| c.meta().id).collect();
    assert_eq!(implemented, registered);
    let planned = all
        .iter()
        .filter(|c| c.state == CheckState::Planned)
        .count();
    assert_eq!(planned, all.len() - registered.len());
    let i2c3 = all.iter().find(|c| c.id == "I2C-3").unwrap();
    assert_eq!(i2c3.needs, ["graph", "part classes"]);
    assert_eq!(i2c3.link, "docs/checks/i2c.md#i2c-3");
    let e = erc_checks::explain("I2C-3").unwrap();
    assert!(e.implemented && e.text.contains("**What it catches.**"));
    assert!(!erc_checks::explain("PIN-1").unwrap().implemented);
}
