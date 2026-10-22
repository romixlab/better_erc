//! The check catalogue (`docs/checks.md`) and its detail pages (`docs/checks/*.md`), embedded at build time and
//! parsed on first use: ids, severities, tiers and titles of all checks, the category order, the explanation of
//! each check and what data it needs. `checks list`, `explain` and the not-run hints read it, so they work
//! offline; the catalogue sync test holds the registered checks to it.

use crate::meta::{Severity, Tier, registered};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/docs.rs"));
}

/// A category of the catalogue (`### 🚌 I2C - I2C`)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    /// `I2C`
    pub id: String,
    /// `I2C`, `Rails and regulators`
    pub title: String,
}

/// One index line of the catalogue
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// `I2C-3`
    pub id: String,
    /// `I2C`
    pub category: String,
    pub severity: Severity,
    /// The line's text between the id and `· impl:`
    pub title: String,
    pub tier: Tier,
    /// The FEATURES.md ids it depends on, as written (`NET-4, NET-10`, `netlist`)
    pub depends: String,
    /// The detail page (`i2c`) and anchor (`i2c-3`)
    pub page: String,
    pub anchor: String,
}

/// The parsed catalogue
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalogue {
    pub categories: Vec<Category>,
    pub entries: Vec<Entry>,
}

static CATALOGUE: LazyLock<Catalogue> = LazyLock::new(|| Catalogue::parse(embedded::CATALOGUE));

/// The embedded catalogue
pub fn catalogue() -> &'static Catalogue {
    &CATALOGUE
}

impl Catalogue {
    /// Reads the catalogue's category headings and index lines; anything else is prose
    pub fn parse(text: &str) -> Catalogue {
        let mut cat = Catalogue::default();
        for line in text.lines() {
            if let Some(h) = line.strip_prefix("### ") {
                // `🚌 I2C - I2C`: an emoji, the id, the title
                let h = h.split_once(' ').map_or(h, |(_, rest)| rest);
                if let Some((id, title)) = h.split_once(" - ") {
                    cat.categories.push(Category {
                        id: id.trim().to_string(),
                        title: title.trim().to_string(),
                    });
                }
            } else if line.starts_with("- <a id=\"")
                && let Some(e) = parse_entry(line)
            {
                cat.entries.push(e);
            }
        }
        cat
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Position of a category in the catalogue; unknown ones last
    pub fn category_index(&self, category: &str) -> usize {
        self.categories
            .iter()
            .position(|c| c.id == category)
            .unwrap_or(usize::MAX)
    }
}

/// `- <a id="i2c-3"></a>🛑 \`I2C-3\` Title · impl: easy (NET-4, NET-10) · [📖](checks/i2c.md#i2c-3) <!-- -->`
fn parse_entry(line: &str) -> Option<Entry> {
    let rest = line.split_once("</a>")?.1;
    let (emoji, rest) = rest.split_once(' ')?;
    let severity = match emoji {
        "🛑" => Severity::Error,
        "⚠️" => Severity::Warning,
        "💡" => Severity::Suggestion,
        _ => return None,
    };
    let rest = rest.strip_prefix('`')?;
    let (id, rest) = rest.split_once('`')?;
    let (title, rest) = rest.split_once(" · impl: ")?;
    let (tier_part, rest) = rest.split_once(" · [📖](checks/")?;
    let (tier, depends) = match tier_part.split_once(' ') {
        Some((t, d)) => (t, d.trim().trim_start_matches('(').trim_end_matches(')')),
        None => (tier_part, ""),
    };
    let link = rest.split_once(')')?.0;
    let (page, anchor) = link.split_once(".md#")?;
    Some(Entry {
        id: id.to_string(),
        category: id.split('-').next()?.to_string(),
        severity,
        title: title.trim().to_string(),
        tier: Tier::from_name(tier)?,
        depends: depends.to_string(),
        page: page.to_string(),
        anchor: anchor.to_string(),
    })
}

/// Sort key of a check id in catalogue order: category position, then number
pub fn order_key(id: &str) -> (usize, u32, String) {
    let (category, number) = id.split_once('-').unwrap_or((id, ""));
    (
        catalogue().category_index(category),
        number.parse().unwrap_or(u32::MAX),
        id.to_string(),
    )
}

/// The detail page section of a check, from its `## \`ID\`` heading to the next heading of that level
pub fn detail(id: &str) -> Option<&'static str> {
    let entry = catalogue().entry(id)?;
    let page = embedded::PAGES
        .iter()
        .find(|(name, _)| *name == entry.page)?
        .1;
    let heading = format!("## `{id}` ");
    let start = page.find(&heading)?;
    let body = &page[start..];
    let end = body[3..].find("\n## ").map_or(body.len(), |i| i + 4);
    Some(body[..end].trim_end())
}

/// What the detail page says a check needs (`the bus, resistor chains`)
pub fn needs_text(id: &str) -> Option<&'static str> {
    let section = detail(id)?;
    let line = section.lines().find(|l| l.contains(" · needs: "))?;
    let after = line.split_once(" · needs: ")?.1;
    Some(after.split(" · [").next().unwrap_or(after).trim())
}

/// A check's explanation for `explain`: catalogue line and detail page section
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Explanation {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub tier: Tier,
    /// `docs/checks/i2c.md#i2c-3`
    pub link: String,
    /// The detail page section, Markdown
    pub text: String,
    /// Whether better_erc runs it today
    pub implemented: bool,
}

impl Explanation {
    /// The detail page section as Markdown, with a line saying whether better_erc runs the check
    pub fn to_markdown(&self) -> String {
        let mut text = if self.text.is_empty() {
            format!(
                "## `{}` {}\n\n{} · impl: {}",
                self.id,
                self.title,
                self.severity,
                self.tier.name()
            )
        } else {
            self.text.clone()
        };
        text += &format!("\n\n*{}* · `{}`\n", self.state_line(), self.link);
        text
    }

    /// The same as plain text: no emphasis, code marks, anchors or link targets
    pub fn to_text(&self) -> String {
        plain(&self.to_markdown())
    }

    fn state_line(&self) -> &'static str {
        if self.implemented {
            "better_erc runs this check"
        } else {
            "planned: better_erc does not run this check yet"
        }
    }
}

/// Markdown as plain text: `**x**`, `*x*` and `` `x` `` lose their marks, `[text](target)` keeps the text, HTML
/// anchors go, a heading loses its `#`s
pub fn plain(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    for line in md.lines() {
        let line = line.trim_start_matches('#').trim_start_matches(' ');
        let mut rest = line;
        let mut l = String::with_capacity(line.len());
        while let Some(c) = rest.chars().next() {
            if let Some(tail) = rest.strip_prefix("<a ")
                && let Some(end) = tail.find("</a>")
            {
                rest = &tail[end + 4..];
                continue;
            }
            if c == '['
                && let Some(close) = rest.find("](")
                && let Some(end) = rest[close..].find(')')
            {
                l.push_str(&rest[1..close]);
                rest = &rest[close + end + 1..];
                continue;
            }
            if c != '*' && c != '`' {
                l.push(c);
            }
            rest = &rest[c.len_utf8()..];
        }
        out.push_str(l.trim_end());
        out.push('\n');
    }
    out
}

/// The explanation of a catalogue check, implemented or planned
pub fn explain(id: &str) -> Option<Explanation> {
    let e = catalogue().entry(id)?;
    Some(Explanation {
        id: e.id.clone(),
        title: e.title.clone(),
        severity: e.severity,
        tier: e.tier,
        link: crate::meta::doc_link(id),
        text: detail(id).unwrap_or_default().to_string(),
        implemented: crate::meta::check(id).is_some(),
    })
}

/// Whether a catalogue check has code
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckState {
    /// Registered: it runs when its needs are available
    Implemented,
    /// In the catalogue, no code yet
    Planned,
}

/// One line of `checks list`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckInfo {
    pub id: String,
    pub category: String,
    pub title: String,
    pub severity: Severity,
    pub tier: Tier,
    pub state: CheckState,
    /// Needs of an implemented check (`graph`, `values`)
    pub needs: Vec<String>,
    pub link: String,
}

/// Every catalogue check, in catalogue order, implemented or planned
pub fn list() -> Vec<CheckInfo> {
    let checks = registered();
    catalogue()
        .entries
        .iter()
        .map(|e| {
            let check = checks.iter().find(|c| c.meta().id == e.id);
            CheckInfo {
                id: e.id.clone(),
                category: e.category.clone(),
                title: e.title.clone(),
                severity: e.severity,
                tier: e.tier,
                state: if check.is_some() {
                    CheckState::Implemented
                } else {
                    CheckState::Planned
                },
                needs: check.map_or_else(Vec::new, |c| {
                    c.meta().needs.iter().map(|n| n.name()).collect()
                }),
                link: crate::meta::doc_link(&e.id),
            }
        })
        .collect()
}

/// What a run did with a catalogue check
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Ran,
    NotRun,
    Planned,
}

impl RunState {
    pub fn name(self) -> &'static str {
        match self {
            RunState::Ran => "ran",
            RunState::NotRun => "not-run",
            RunState::Planned => "planned",
        }
    }
}

/// A catalogue check with what a run on a design did with it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListedCheck {
    #[serde(flatten)]
    pub info: CheckInfo,
    /// Ran, not run or planned on the design of the report; `None` without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunState>,
    /// Why it did not run and what would enable it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Every catalogue check ([`list`]), with its state in `report` when there is one (`berc checks list`)
pub fn list_for(report: Option<&crate::report::Report>) -> Vec<ListedCheck> {
    use crate::report::Status;
    list()
        .into_iter()
        .map(|info| {
            let status = report.and_then(|r| r.checks.iter().find(|c| c.id == info.id));
            let (run, hint) = match status.map(|s| &s.status) {
                Some(Status::Ran { .. }) => (Some(RunState::Ran), None),
                Some(Status::NotRun(n)) => (Some(RunState::NotRun), Some(n.hint.clone())),
                Some(Status::Planned) => (Some(RunState::Planned), None),
                None => (None, None),
            };
            ListedCheck { info, run, hint }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_line() {
        let e = parse_entry("- <a id=\"i2c-3\"></a>🛑 `I2C-3` Pull-up from two series resistors to nowhere (part of I2C-1) · impl: easy (NET-4, NET-10) · [📖](checks/i2c.md#i2c-3) <!-- CHK-162 -->").unwrap();
        assert_eq!(e.id, "I2C-3");
        assert_eq!(e.category, "I2C");
        assert_eq!(e.severity, Severity::Error);
        assert_eq!(
            e.title,
            "Pull-up from two series resistors to nowhere (part of I2C-1)"
        );
        assert_eq!(e.tier, Tier::Easy);
        assert_eq!(e.depends, "NET-4, NET-10");
        assert_eq!((e.page.as_str(), e.anchor.as_str()), ("i2c", "i2c-3"));
        let w = parse_entry("- <a id=\"lib-3\"></a>⚠️ `LIB-3` MOSFET symbol pins not named G/D/S · impl: easy (netlist) · [📖](checks/lib.md#lib-3)").unwrap();
        assert_eq!(
            (w.severity, w.depends.as_str()),
            (Severity::Warning, "netlist")
        );
    }

    #[test]
    fn explanation_as_text_and_markdown() {
        let e = explain("I2C-3").unwrap();
        let md = e.to_markdown();
        assert!(md.starts_with("## `I2C-3` Pull-up"), "{md}");
        assert!(md.contains("**What it catches.**"));
        assert!(md.contains("better_erc runs this check"));
        let text = e.to_text();
        assert!(
            text.starts_with("I2C-3 Pull-up from two series resistors to nowhere\n"),
            "{text}"
        );
        assert!(text.contains("What it catches. A pull-up"), "{text}");
        assert!(
            text.contains("resistor chains · catalogue entry\n"),
            "{text}"
        );
        assert!(
            !text.contains('`') && !text.contains("**") && !text.contains("<a "),
            "{text}"
        );
        let planned = explain("PIN-1").unwrap();
        assert!(!planned.implemented);
        assert!(
            planned
                .to_text()
                .contains("planned: better_erc does not run this check yet")
        );
    }

    #[test]
    fn embedded_catalogue_and_pages() {
        let c = catalogue();
        assert!(c.entries.len() > 500, "{} entries", c.entries.len());
        assert_eq!(c.categories[0].id, "PIN");
        assert!(c.category_index("I2C") < c.category_index("BOM"));
        assert!(detail("I2C-3").unwrap().starts_with("## `I2C-3` "));
        assert!(!detail("I2C-3").unwrap().contains("## `I2C-4`"));
        assert_eq!(needs_text("I2C-3"), Some("the bus, resistor chains"));
        assert!(order_key("I2C-10") > order_key("I2C-9"));
    }
}
