//! The KiCad project file (`.kicad_pro`, JSON): net classes with their rules, patterns and assignments, text
//! variables, the variants list (KiCad 10), ERC and DRC rule severities and exclusions, the board design rules,
//! and the bus aliases (KiCad 9 and 10 keep them here). Every value becomes a fact with the project file as its
//! source. The rest of the file is editor state (viewports, plot paths, BOM presets) and is not kept.

use erc_model::*;
use serde_json::{Map, Value as Json};

/// What a project file says, before it is put on a board
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Project {
    pub net_classes: Vec<NetClass>,
    pub settings: ProjectSettings,
    pub variants: Vec<Variant>,
    /// Bus aliases by name, with their members
    pub bus_aliases: Vec<(String, Vec<String>)>,
}

/// A JSON scalar as the text KiCad would show (`0.15`, `true`, `error`)
fn scalar(v: &Json) -> Option<String> {
    match v {
        Json::String(s) => Some(s.clone()),
        Json::Number(n) => Some(n.to_string()),
        Json::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn path<'a>(root: &'a Json, keys: &[&str]) -> Option<&'a Json> {
    keys.iter().try_fold(root, |v, k| v.get(k))
}

struct Reader<'a> {
    file: &'a str,
    notes: Vec<ImportNote>,
}

impl Reader<'_> {
    fn fact(&self, value: String) -> Fact<String> {
        Fact::new(
            value,
            Provenance::KicadProject {
                file: self.file.to_string(),
            },
        )
    }

    fn param(&self, key: &str, value: String) -> Param {
        Param::new(key, self.fact(value))
    }

    fn invalid(&mut self, at: &str, message: &str) {
        self.notes.push(ImportNote {
            file: self.file.to_string(),
            at: at.to_string(),
            kind: NoteKind::Invalid,
            message: message.to_string(),
        });
    }

    /// The scalar entries of an object as params, sorted by key; other entries are noted
    fn params(&mut self, at: &str, obj: Option<&Json>) -> Vec<Param> {
        let Some(obj) = obj else { return Vec::new() };
        let Some(map) = obj.as_object() else {
            self.invalid(at, "not an object");
            return Vec::new();
        };
        let mut out = Vec::new();
        for (k, v) in map {
            match scalar(v) {
                Some(s) => out.push(self.param(k, s)),
                None if v.is_null() => {}
                None => self.invalid(&format!("{at}.{k}"), "not a number, text or yes/no"),
            }
        }
        out
    }

    /// Exclusions: each entry the exclusion's key, alone or with the comment the user gave it
    fn exclusions(&mut self, at: &str, list: Option<&Json>) -> Vec<Param> {
        let Some(list) = list else { return Vec::new() };
        let Some(items) = list.as_array() else {
            self.invalid(at, "not a list");
            return Vec::new();
        };
        let mut out = Vec::new();
        for (i, e) in items.iter().enumerate() {
            let (key, comment) = match e {
                Json::String(k) => (k.clone(), String::new()),
                Json::Array(a) => match (a.first().and_then(Json::as_str), a.get(1)) {
                    (Some(k), c) => (
                        k.to_string(),
                        c.and_then(Json::as_str).unwrap_or_default().to_string(),
                    ),
                    _ => {
                        self.invalid(&format!("{at}[{i}]"), "an exclusion without its key");
                        continue;
                    }
                },
                _ => {
                    self.invalid(&format!("{at}[{i}]"), "not an exclusion");
                    continue;
                }
            };
            out.push(self.param(&key, comment));
        }
        out
    }

    fn net_classes(&mut self, ns: &Json) -> Vec<NetClass> {
        let mut classes: Vec<NetClass> = Vec::new();
        for (i, c) in ns
            .get("classes")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let at = format!("net_settings.classes[{i}]");
            let Some(map) = c.as_object() else {
                self.invalid(&at, "not an object");
                continue;
            };
            let Some(name) = map.get("name").and_then(Json::as_str) else {
                self.invalid(&at, "a net class without a name");
                continue;
            };
            let rest: Map<String, Json> = map
                .iter()
                .filter(|(k, _)| *k != "name")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let params = self.params(&at, Some(&Json::Object(rest)));
            classes.push(NetClass {
                name: name.to_string(),
                params,
                patterns: Vec::new(),
                assigned: Vec::new(),
            });
        }
        let class = |classes: &mut Vec<NetClass>, name: &str| -> usize {
            match classes.iter().position(|c| c.name == name) {
                Some(i) => i,
                None => {
                    classes.push(NetClass {
                        name: name.to_string(),
                        params: Vec::new(),
                        patterns: Vec::new(),
                        assigned: Vec::new(),
                    });
                    classes.len() - 1
                }
            }
        };
        for (i, p) in ns
            .get("netclass_patterns")
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            match (
                p.get("netclass").and_then(Json::as_str),
                p.get("pattern").and_then(Json::as_str),
            ) {
                (Some(c), Some(pat)) => {
                    let k = class(&mut classes, c);
                    classes[k].patterns.push(pat.to_string());
                }
                _ => self.invalid(
                    &format!("net_settings.netclass_patterns[{i}]"),
                    "a pattern without its net class or pattern",
                ),
            }
        }
        if let Some(map) = ns.get("netclass_assignments").and_then(Json::as_object) {
            for (net, v) in map {
                // KiCad 8 writes one class name, 9 and 10 a list
                let names: Vec<&str> = match v {
                    Json::String(s) => vec![s.as_str()],
                    Json::Array(a) => a.iter().filter_map(Json::as_str).collect(),
                    _ => {
                        self.invalid(
                            &format!("net_settings.netclass_assignments.{net}"),
                            "not a net class name",
                        );
                        continue;
                    }
                };
                for c in names {
                    let k = class(&mut classes, c);
                    classes[k].assigned.push(net.clone());
                }
            }
        }
        classes
    }
}

/// Reads a project file; `file` names it in the facts and notes
pub fn read(text: &str, file: &str) -> Result<(Project, Vec<ImportNote>), ImportError> {
    let root: Json = serde_json::from_str(text).map_err(|e| ImportError::Invalid {
        file: file.to_string(),
        message: format!("not a KiCad project file: {e}"),
    })?;
    if !root.is_object() {
        return Err(ImportError::Invalid {
            file: file.to_string(),
            message: "not a KiCad project file: not a JSON object".into(),
        });
    }
    let mut r = Reader {
        file,
        notes: Vec::new(),
    };
    let mut p = Project::default();
    if let Some(ns) = root.get("net_settings") {
        p.net_classes = r.net_classes(ns);
    }
    p.settings.text_vars = r.params("text_variables", root.get("text_variables"));
    p.settings.erc_severities = r.params(
        "erc.rule_severities",
        path(&root, &["erc", "rule_severities"]),
    );
    p.settings.erc_exclusions = r.exclusions(
        "erc.erc_exclusions",
        path(&root, &["erc", "erc_exclusions"]),
    );
    fn at(k: &str) -> [&str; 3] {
        ["board", "design_settings", k]
    }
    p.settings.drc_severities = r.params(
        "board.design_settings.rule_severities",
        path(&root, &at("rule_severities")),
    );
    p.settings.drc_exclusions = r.exclusions(
        "board.design_settings.drc_exclusions",
        path(&root, &at("drc_exclusions")),
    );
    p.settings.rules = r.params("board.design_settings.rules", path(&root, &at("rules")));
    for (i, v) in path(&root, &["schematic", "variants"])
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        match v.get("name").and_then(Json::as_str) {
            Some(name) => p.variants.push(Variant {
                name: name.to_string(),
                description: v
                    .get("description")
                    .and_then(Json::as_str)
                    .filter(|d| !d.is_empty())
                    .map(String::from),
            }),
            None => r.invalid(
                &format!("schematic.variants[{i}]"),
                "a variant without a name",
            ),
        }
    }
    if let Some(map) = path(&root, &["schematic", "bus_aliases"]).and_then(Json::as_object) {
        for (name, members) in map {
            match members.as_array() {
                Some(m) => p.bus_aliases.push((
                    name.clone(),
                    m.iter()
                        .filter_map(Json::as_str)
                        .map(String::from)
                        .collect(),
                )),
                None => r.invalid(
                    &format!("schematic.bus_aliases.{name}"),
                    "not a list of members",
                ),
            }
        }
    }
    Ok((p, r.notes))
}

impl Project {
    /// Puts what the project says on a board read from its schematic: net classes and settings replace the
    /// board's (a schematic has none), variants the schematic did not name are added, descriptions filled in
    pub fn apply(self, board: &mut Board) {
        board.net_classes = self.net_classes;
        board.settings = self.settings;
        for v in self.variants {
            match board.variants.iter_mut().find(|b| b.name == v.name) {
                Some(b) => {
                    if b.description.is_none() {
                        b.description = v.description;
                    }
                }
                None => board.variants.push(v),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_sections() {
        let text = r#"{
          "net_settings": {
            "classes": [{"name": "Default", "clearance": 0.2, "priority": 2147483647},
                        {"name": "Power", "track_width": 0.5, "pcb_color": "rgb(194, 0, 194)"}],
            "netclass_patterns": [{"netclass": "Power", "pattern": "+*"}],
            "netclass_assignments": {"/USB_N": ["Diff"], "/X": "Power"}
          },
          "text_variables": {"REV": "B"},
          "erc": {"rule_severities": {"pin_not_connected": "ignore"},
                  "erc_exclusions": ["pin_not_driven|1|2|a|b", ["pin_to_pin|3|4|c|d", "checked"]]},
          "board": {"design_settings": {"rules": {"min_clearance": 0.0, "use_height_for_length_calcs": true},
                    "rule_severities": {"clearance": "error"}, "drc_exclusions": [["clearance|1|2|a|b", ""]]}},
          "schematic": {"variants": [{"name": "Main"}, {"name": "Plus", "description": "extras"}],
                        "bus_aliases": {"USB": ["DP", "DM"]}}
        }"#;
        let (p, notes) = read(text, "x.kicad_pro").unwrap();
        assert!(notes.is_empty(), "{notes:?}");
        let names: Vec<&str> = p.net_classes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Default", "Power", "Diff"]);
        assert_eq!(p.net_classes[1].patterns, ["+*"]);
        assert_eq!(p.net_classes[1].assigned, ["/X"]);
        assert_eq!(p.net_classes[2].assigned, ["/USB_N"]);
        let get = |l: &[Param], k: &str| {
            l.iter()
                .find(|p| p.key == k)
                .and_then(|p| p.value.get().cloned())
        };
        assert_eq!(
            get(&p.net_classes[0].params, "clearance").as_deref(),
            Some("0.2")
        );
        assert_eq!(get(&p.settings.text_vars, "REV").as_deref(), Some("B"));
        assert_eq!(p.settings.erc_exclusions.len(), 2);
        assert_eq!(
            get(&p.settings.erc_exclusions, "pin_to_pin|3|4|c|d").as_deref(),
            Some("checked")
        );
        assert_eq!(
            get(&p.settings.rules, "use_height_for_length_calcs").as_deref(),
            Some("true")
        );
        assert_eq!(p.variants[1].description.as_deref(), Some("extras"));
        assert_eq!(
            p.bus_aliases,
            [("USB".to_string(), vec!["DP".to_string(), "DM".to_string()])]
        );
    }

    #[test]
    fn not_json_is_an_error() {
        assert!(read("(kicad_sch)", "x.kicad_pro").is_err());
        assert!(read("[1]", "x.kicad_pro").is_err());
    }
}
