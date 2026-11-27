//! The lock file KiCad keeps beside a file it has open (`~NAME.EXT.lck`, `LOCKFILE::LockPathFor` in KiCad's
//! `include/lockfile.h`), so a writer of schematics, boards and libraries can tell KiCad would overwrite its edit.

use std::path::{Path, PathBuf};

/// KiCad's lock file of an open file, held
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KicadLock {
    /// The lock file
    pub path: PathBuf,
    /// Who holds it, `user@host`, when the lock says
    pub owner: Option<String>,
}

/// The path of the lock file KiCad keeps beside `file` while it has it open
pub fn lock_path(file: &Path) -> PathBuf {
    let name = file
        .file_name()
        .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
    file.with_file_name(format!("~{name}.lck"))
}

/// The lock of `file` when it is held. KiCad 8 to 10 write `{"username", "hostname"}` into it and no process id:
/// a lock from this computer (`host`) is held while a KiCad runs here (`running`), or while its `pid` lives should
/// a lock carry one; a lock from another computer, or one that names none, can't be checked from here and counts
/// as held. An empty or unreadable lock is stale, as KiCad 10 takes it.
pub fn held(file: &Path, host: Option<&str>, running: bool) -> Option<KicadLock> {
    let path = lock_path(file);
    let text = std::fs::read_to_string(&path).ok()?;
    let rec: serde_json::Value = serde_json::from_str(&text).ok()?;
    rec.as_object()?;
    let field = |k: &str| {
        rec.get(k)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
    };
    let short = |h: &str| h.split('.').next().unwrap_or(h).to_lowercase();
    let here = field("hostname")
        .zip(host)
        .is_some_and(|(h, me)| short(h) == short(me));
    if here {
        let held = match rec.get("pid").and_then(serde_json::Value::as_u64) {
            Some(pid) => process_alive(pid),
            None => running,
        };
        if !held {
            return None;
        }
    }
    let owner = match (field("username"), field("hostname")) {
        (Some(u), Some(h)) => Some(format!("{u}@{h}")),
        (u, h) => u.or(h).map(String::from),
    };
    Some(KicadLock { path, owner })
}

/// [`held`] with this computer's name and whether a KiCad runs here, as they are now
pub fn held_now(file: &Path) -> Option<KicadLock> {
    held(file, this_host().as_deref(), kicad_running())
}

/// This computer's name, as KiCad writes it into its locks (wxGetHostName: without the domain)
pub fn this_host() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
}

/// Whether process `pid` lives on this computer; where that can't be told (no `/proc`), it is taken to
fn process_alive(pid: u64) -> bool {
    if !Path::new("/proc/self").exists() {
        return true;
    }
    Path::new("/proc").join(pid.to_string()).exists()
}

/// Whether a KiCad process (the project manager or the schematic editor) runs on this computer
pub fn kicad_running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return false;
    };
    dir.flatten().any(|e| {
        std::fs::read_to_string(e.path().join("comm"))
            .is_ok_and(|c| matches!(c.trim(), "kicad" | "eeschema"))
    })
}
