//! The usage record: every run of `berc` appends one line per signal, the subcommand it ran (`fp check`) and the
//! flags given that are features of their own (`check --baseline`), so commands nobody uses can be found and
//! removed. Local and cheap: a short line appended to a file, nothing sent anywhere; a write that fails never fails
//! the command. Shell completion is not recorded.
//!
//! The file is `$BERC_USAGE_FILE`, else `$XDG_STATE_HOME/berc/usage.tsv`, else `~/.local/state/berc/usage.tsv`.
//! Each line: the time (Unix seconds), the signal and where it ran from (`terminal` when stdin is a terminal, else
//! `script`), separated by tabs. `BERC_USAGE=off` records nothing. `berc usage` sums the file per signal.

use clap::ArgMatches;
use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

/// Flags that are features of their own, recorded beside their command: `check --baseline`
const MEANINGFUL_FLAGS: &[&str] = &[
    "baseline",
    "variant",
    "require",
    "pnp",
    "nets",
    "canonical",
    "with_geometry",
];

/// The signals of a run: the subcommand path, then one per meaningful flag given on the command line
pub fn signals(matches: &ArgMatches) -> Vec<String> {
    let mut path = Vec::new();
    let mut m = matches;
    while let Some((sub, sub_m)) = m.subcommand() {
        path.push(sub);
        m = sub_m;
    }
    let path = path.join(" ");
    let mut out = vec![path.clone()];
    for flag in MEANINGFUL_FLAGS {
        // try_ first: a command without this flag is not an error
        let given = m.try_contains_id(flag).unwrap_or(false)
            && m.value_source(flag) == Some(clap::parser::ValueSource::CommandLine);
        if given {
            out.push(format!("{path} --{}", flag.replace('_', "-")));
        }
    }
    out
}

/// Every signal a run can give, from the command tree: the leaf subcommands and their meaningful flags
pub fn known(cmd: &clap::Command) -> Vec<String> {
    fn walk(cmd: &clap::Command, path: &str, out: &mut Vec<String>) {
        let subs: Vec<&clap::Command> = cmd
            .get_subcommands()
            .filter(|c| c.get_name() != "help")
            .collect();
        if subs.is_empty() && !path.is_empty() {
            out.push(path.to_string());
            for a in cmd.get_arguments() {
                let id = a.get_id().as_str();
                if MEANINGFUL_FLAGS.contains(&id) {
                    out.push(format!("{path} --{}", id.replace('_', "-")));
                }
            }
        }
        for s in subs {
            let p = if path.is_empty() {
                s.get_name().to_string()
            } else {
                format!("{path} {}", s.get_name())
            };
            walk(s, &p, out);
        }
    }
    let mut out = Vec::new();
    walk(cmd, "", &mut out);
    out
}

pub fn file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("BERC_USAGE_FILE") {
        return Some(p.into());
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(state.join("berc/usage.tsv"))
}

fn off() -> bool {
    std::env::var("BERC_USAGE").is_ok_and(|v| v == "off")
}

pub fn record(signals: &[String]) {
    if off() {
        return;
    }
    let Some(path) = file() else { return };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let from = if std::io::stdin().is_terminal() {
        "terminal"
    } else {
        "script"
    };
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

/// Runs per signal from the record, every known signal listed (0 for the unused), then any others the record has
pub fn summary(known: &[String]) -> String {
    let mut counts: BTreeMap<String, (u64, u64)> =
        known.iter().map(|k| (k.clone(), (0, 0))).collect();
    let text = file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(_), Some(sig), from) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let c = counts.entry(sig.to_string()).or_default();
        if from == Some("terminal") {
            c.0 += 1;
        } else {
            c.1 += 1;
        }
    }
    let width = counts.keys().map(String::len).max().unwrap_or(0);
    let mut out = format!(
        "{:<width$}  {:>8}  {:>6}\n",
        "command", "terminal", "script"
    );
    for (sig, (t, s)) in &counts {
        out += &format!("{sig:<width$}  {t:>8}  {s:>6}\n");
    }
    match file() {
        Some(p) if off() => {
            out += &format!("from {} (recording is off: BERC_USAGE=off)\n", p.display())
        }
        Some(p) => out += &format!("from {}\n", p.display()),
        None => out += "no record: neither BERC_USAGE_FILE, XDG_STATE_HOME nor HOME is set\n",
    }
    out
}
