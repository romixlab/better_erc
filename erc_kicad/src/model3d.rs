//! Resolving a footprint's 3D model path ([`erc_model::layout::ModelEntry::path`]): the `${VAR}` of
//! [`crate::libtable::KicadVars`] (the process environment, then `kicad_common.json`'s `environment.vars` and
//! `KIPRJMOD`, then a stock KiCad's `KICADn_3DMODEL_DIR`), a relative result against the project directory, and,
//! for a `.wrl` that still does not exist, the same path with a `.step` or `.stp` extension, as KiCad does.

use crate::libtable;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// Where a model's path led, or why it did not
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelPath {
    /// The file exists at `path`; `vars` names the `${VAR}` substituted to get there, in order, empty when the
    /// entry had none
    Found { path: PathBuf, vars: Vec<String> },
    /// A `${VAR}` (or `$(VAR)`) the environment, `kicad_common.json` and no stock KiCad set
    UnknownVar(String),
    /// Expanded to `path`, but no file is there (nor, for a `.wrl`, a `.step` or `.stp` beside it)
    NotFound(PathBuf),
}

/// Resolves a model's raw path (as the file writes it) against `project_dir`, with `var` answering each `${VAR}`
/// it finds: the process environment, then [`libtable::KicadVars::get`] for a real board
/// (`kicad_common.json`, `KIPRJMOD`, a stock KiCad's `KICADn_3DMODEL_DIR`)
pub fn resolve(raw: &str, project_dir: &Path, var: &dyn Fn(&str) -> Option<String>) -> ModelPath {
    let seen: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let expanded = libtable::expand(raw, &|name| {
        seen.borrow_mut().push(name.to_string());
        var(name)
    });
    let Some(expanded) = expanded else {
        let missing = seen.into_inner().pop().unwrap_or_default();
        return ModelPath::UnknownVar(missing);
    };
    let vars = seen.into_inner();
    let path = PathBuf::from(expanded);
    let path = if path.is_absolute() {
        path
    } else {
        project_dir.join(path)
    };
    if path.is_file() {
        return ModelPath::Found { path, vars };
    }
    if path.extension().and_then(|e| e.to_str()) == Some("wrl") {
        for alt in ["step", "stp"] {
            let candidate = path.with_extension(alt);
            if candidate.is_file() {
                return ModelPath::Found {
                    path: candidate,
                    vars,
                };
            }
        }
    }
    ModelPath::NotFound(path)
}

/// [`resolve`] against a project's [`libtable::KicadVars`] (the usual case: a board or library footprint)
pub fn resolve_with(raw: &str, project_dir: &Path, vars: &libtable::KicadVars) -> ModelPath {
    resolve(raw, project_dir, &|name| vars.get(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh temporary directory; `name` keeps concurrent tests from sharing one
    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("erc-model3d-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Variables from a fake environment, without touching the real process environment or `KicadVars`
    #[test]
    fn variables_from_a_fake_environment() {
        let d = dir("env");
        std::fs::write(d.join("r.step"), "").unwrap();
        let var = |n: &str| (n == "MY_3DMODEL_DIR").then(|| d.to_string_lossy().into_owned());
        assert_eq!(
            resolve("${MY_3DMODEL_DIR}/r.step", &d, &var),
            ModelPath::Found {
                path: d.join("r.step"),
                vars: vec!["MY_3DMODEL_DIR".into()],
            }
        );
        assert_eq!(
            resolve("${NOT_SET}/r.step", &d, &var),
            ModelPath::UnknownVar("NOT_SET".into())
        );
        // No variable at all: resolved against the project directory, `vars` empty
        assert_eq!(
            resolve("r.step", &d, &var),
            ModelPath::Found {
                path: d.join("r.step"),
                vars: vec![],
            }
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// A `.wrl` that is missing, but a `.step` beside it exists
    #[test]
    fn a_missing_wrl_tries_step_and_stp() {
        let d = dir("wrl");
        std::fs::write(d.join("a.step"), "").unwrap();
        std::fs::write(d.join("b.stp"), "").unwrap();
        let none = |_: &str| None;
        assert_eq!(
            resolve("a.wrl", &d, &none),
            ModelPath::Found {
                path: d.join("a.step"),
                vars: vec![],
            }
        );
        assert_eq!(
            resolve("b.wrl", &d, &none),
            ModelPath::Found {
                path: d.join("b.stp"),
                vars: vec![],
            }
        );
        assert_eq!(
            resolve("c.wrl", &d, &none),
            ModelPath::NotFound(d.join("c.wrl"))
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// The `environment.vars` of a fake `kicad_common.json`, read through [`libtable::KicadVars`]
    #[test]
    fn variables_from_a_fake_kicad_common_json() {
        let d = dir("common");
        let config = d.join("config");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(
            config.join("kicad_common.json"),
            r#"{"environment": {"vars": {"VHRD_3DMODEL_DIR": "/vhrd/3d"}}}"#,
        )
        .unwrap();
        let vars = libtable::KicadVars::new(&d, Some(&config));
        assert_eq!(vars.get("VHRD_3DMODEL_DIR"), Some("/vhrd/3d".to_string()));
        assert_eq!(
            resolve_with("${VHRD_3DMODEL_DIR}/r.step", &d, &vars),
            ModelPath::NotFound(PathBuf::from("/vhrd/3d/r.step")),
        );
        std::fs::remove_dir_all(&d).unwrap();
    }
}
