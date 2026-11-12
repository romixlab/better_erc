//! The usage record berc and the better_erc app share: one line per signal (a command, a flag that is a feature of
//! its own, a step of the app such as `app fp-review accept`), so parts nobody uses can be found and removed. Local
//! and cheap: a short line appended to a file, nothing sent anywhere; a write that fails never fails the caller.
//!
//! The file is `$BERC_USAGE_FILE`, else `$XDG_STATE_HOME/berc/usage.tsv`, else `~/.local/state/berc/usage.tsv`.
//! Each line: the time (Unix seconds), the signal and where it ran from (`terminal`, `script` or `app`), separated
//! by tabs. `BERC_USAGE=off` records nothing. `berc usage` sums the file per signal.

use std::io::Write;
use std::path::PathBuf;

/// The record file
pub fn file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("BERC_USAGE_FILE") {
        return Some(p.into());
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(state.join("berc/usage.tsv"))
}

/// `BERC_USAGE=off`: nothing is recorded
pub fn off() -> bool {
    std::env::var("BERC_USAGE").is_ok_and(|v| v == "off")
}

/// Appends one line per signal, run from `from` (`terminal`, `script`, `app`)
pub fn record(signals: &[String], from: &str) {
    if off() {
        return;
    }
    let Some(path) = file() else { return };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let lines: String = signals
        .iter()
        .map(|s| format!("{now}\t{s}\t{from}\n"))
        .collect();
    let _ = path.parent().map(std::fs::create_dir_all);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = f.write_all(lines.as_bytes());
    }
}
