//! The usage record: every run of `berc` appends one line per signal, the subcommand it ran (`fp check`) and the
//! flags given that are features of their own (`check --baseline`), so commands nobody uses can be found and
//! removed. Local and cheap: a short line appended to a file, nothing sent anywhere; a write that fails never fails
//! the command. Shell completion is not recorded.
//!
//! The record itself, its file and its lines are [`better_erc::usage`], shared with the better_erc app: a run from
//! a terminal (stdin is one) is `terminal`, else `script`; the app's steps are `app`. `berc usage` sums the file
//! per signal.

use clap::ArgMatches;
use std::collections::BTreeMap;
use std::io::IsTerminal;

/// Flags that are features of their own, recorded beside their command: `check --baseline`
const MEANINGFUL_FLAGS: &[&str] = &[
    "baseline",
    "variant",
    "require",
    "pnp",
    "nets",
    "canonical",
    "with_geometry",
    "lib",
    "footprints",
    "symbols",
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

pub use better_erc::usage::file;

/// Records a run's signals, from a `terminal` when stdin is one, else from a `script`
pub fn record(signals: &[String]) {
    let from = if std::io::stdin().is_terminal() {
        "terminal"
    } else {
        "script"
    };
    better_erc::usage::record(signals, from);
}

/// Runs per signal from the record, every known signal listed (0 for the unused), then any others the record has
pub fn summary(known: &[String]) -> String {
    let mut counts: BTreeMap<String, [u64; 3]> =
        known.iter().map(|k| (k.clone(), [0; 3])).collect();
    let text = file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (Some(_), Some(sig), from) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let c = counts.entry(sig.to_string()).or_default();
        match from {
            Some("terminal") => c[0] += 1,
            Some("app") => c[2] += 1,
            _ => c[1] += 1,
        }
    }
    let width = counts.keys().map(String::len).max().unwrap_or(0);
    let mut out = format!(
        "{:<width$}  {:>8}  {:>6}  {:>4}\n",
        "command", "terminal", "script", "app"
    );
    for (sig, [t, s, a]) in &counts {
        out += &format!("{sig:<width$}  {t:>8}  {s:>6}  {a:>4}\n");
    }
    match file() {
        Some(p) if better_erc::usage::off() => {
            out += &format!("from {} (recording is off: BERC_USAGE=off)\n", p.display())
        }
        Some(p) => out += &format!("from {}\n", p.display()),
        None => out += "no record: neither BERC_USAGE_FILE, XDG_STATE_HOME nor HOME is set\n",
    }
    out
}
