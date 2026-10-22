//! Netlists of whole KiCad projects (a root sheet and its sub-sheets anywhere below its directory), exported by
//! the installed kicad-cli or by the kicad-cli of KiCad's container image for another major version (podman),
//! and cached by a hash of the project's files. The project is copied to a scratch folder first, so kicad-cli
//! never writes into the project (it leaves `.kicad_prl` files behind).

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

/// Which kicad-cli exports
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kicad {
    /// `kicad-cli` on PATH
    Installed,
    /// `docker.io/kicad/kicad:<major>.0` through podman
    Container(u32),
}

impl Kicad {
    /// The one that exports as KiCad `major` does: the installed kicad-cli when it is that version, else the
    /// container image
    pub fn for_major(major: u32) -> Kicad {
        if installed_major() == Some(major) {
            Kicad::Installed
        } else {
            Kicad::Container(major)
        }
    }

    fn tag(self) -> String {
        match self {
            Kicad::Installed => format!("installed-{}", installed_version().unwrap_or_default()),
            Kicad::Container(v) => format!("container-{v}"),
        }
    }
}

/// `10.0.6`, when kicad-cli is installed
pub fn installed_version() -> Option<String> {
    let out = Command::new("kicad-cli").arg("--version").output().ok()?;
    let v = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !v.is_empty()).then_some(v)
}

/// The installed kicad-cli's major version
pub fn installed_major() -> Option<u32> {
    installed_version()?.split('.').next()?.parse().ok()
}

/// Whether podman is there to run the container images
pub fn podman_available() -> bool {
    Command::new("podman")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The project files kicad-cli reads for a netlist: every `.kicad_sch` and `.kicad_pro` below `dir`, relative,
/// sorted
fn project_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        for e in fs::read_dir(dir.join(&rel))? {
            let e = e?;
            let name = e.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with('.') {
                continue;
            }
            let t = e.file_type()?;
            if t.is_dir() {
                stack.push(rel.join(name));
            } else if t.is_file() && (name.ends_with(".kicad_sch") || name.ends_with(".kicad_pro"))
            {
                out.push(rel.join(name));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The netlist of the project whose root sheet is `root_sch`, exported by `kicad` into `cache` unless a netlist
/// of the same files by the same kicad-cli is there already
pub fn cached_project_netlist(root_sch: &Path, kicad: Kicad, cache: &Path) -> io::Result<PathBuf> {
    let dir = root_sch.parent().unwrap_or(Path::new("."));
    let root_name = root_sch
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no root sheet name"))?;
    let files = project_files(dir)?;
    let mut h = Sha256::new();
    h.update(kicad.tag());
    h.update([0]);
    h.update(root_name);
    for f in &files {
        h.update([0]);
        h.update(f.to_string_lossy().as_bytes());
        h.update([0]);
        h.update(fs::read(dir.join(f))?);
    }
    let hash = hex::encode(h.finalize());
    let stem = root_name.trim_end_matches(".kicad_sch");
    fs::create_dir_all(cache)?;
    let out = cache.join(format!("{stem}_{}.net", &hash[..16]));
    if out.exists() {
        return Ok(out);
    }
    let work = cache.join(format!("work_{}", &hash[..16]));
    let _ = fs::remove_dir_all(&work);
    for f in &files {
        let to = work.join(f);
        fs::create_dir_all(to.parent().unwrap())?;
        fs::copy(dir.join(f), to)?;
    }
    let tmp_name = format!("{stem}.net");
    let status = match kicad {
        Kicad::Installed => Command::new("kicad-cli")
            .args(["sch", "export", "netlist", "-o", &tmp_name, root_name])
            .current_dir(&work)
            .output()?,
        Kicad::Container(v) => Command::new("podman")
            .args(["run", "--rm", "--userns=keep-id", "-v"])
            .arg(format!("{}:/w:Z", work.display()))
            .args(["-w", "/w", &format!("docker.io/kicad/kicad:{v}.0")])
            .args([
                "kicad-cli",
                "sch",
                "export",
                "netlist",
                "-o",
                &tmp_name,
                root_name,
            ])
            .output()?,
    };
    let produced = work.join(&tmp_name);
    if !status.status.success() || !produced.exists() {
        let _ = fs::remove_dir_all(&work);
        return Err(io::Error::other(format!(
            "{kicad:?} could not export {}: {}{}",
            root_sch.display(),
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        )));
    }
    fs::rename(&produced, &out)?;
    fs::remove_dir_all(&work)?;
    Ok(out)
}
