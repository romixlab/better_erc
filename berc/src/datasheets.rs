//! `berc parts datasheets` (TOOL-18): a design's datasheet URLs by MPN; with `--to-mx3`, stated in the mx3 part
//! store as each MPN's datasheet fact.

use crate::{ListFormat, error};
use better_erc::parts::datasheets::MpnDatasheets;
use clap::ValueHint;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// What fails a run: the design, the mx3 store or the build
const EXIT_ERROR: u8 = 2;

#[derive(clap::Args)]
pub(crate) struct PartsDatasheetsArgs {
    /// The design: a directory with one design in it, or a design file
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    project: PathBuf,
    /// State each MPN's URL in the mx3 part store as the datasheet fact of its part
    #[arg(long)]
    to_mx3: bool,
    /// With --to-mx3: ask mx3 what it has and list what would be written; write nothing
    #[arg(long, requires = "to_mx3")]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

const PARTS_DATASHEETS_AFTER_HELP: &str = "\
Only a Datasheet field that is an http(s) URL counts, a vendor's product page as much as a PDF; empty fields, \
files and mx3: links are left out. An MPN whose parts give different URLs is listed and never written.

With --to-mx3 each MPN's URL becomes a fact of the parameter with key `datasheet` (mx3's Datasheet Url, what \
`mx3 lib fetch --mpn` reads) on the MPN's part, from the source `kicad_field`, ranked below the makers' own \
data. A part that has the URL already, from any source, is left as it is, so a second run writes nothing. An MPN \
with no mx3 part, or several, is listed; nothing is created. The source is added once by an mx3 admin:
  mx3 param source add kicad_field \"KiCad field\" --rank 65 --kind import --writable-by lab:write

Exit codes: 0 listed or written; 2 the design, mx3 or the build failed.

Examples:
  berc parts datasheets boards/b153
  berc parts datasheets boards/b153 --to-mx3 --dry-run
  berc parts datasheets boards/b153 --to-mx3";

/// The after-help, with where the mx3 part store is found when this berc is built with it
pub(crate) fn after_help() -> String {
    #[cfg(feature = "mx3")]
    return format!(
        "{PARTS_DATASHEETS_AFTER_HELP}\n\nmx3: {}",
        better_erc::parts::mx3::CONNECTION_HELP
    );
    #[cfg(not(feature = "mx3"))]
    format!(
        "{PARTS_DATASHEETS_AFTER_HELP}\n\n--to-mx3 needs a berc built with the mx3 part store (cargo feature mx3)."
    )
}

/// The board's URLs, one line per MPN and URL
fn listed_text(board: &[MpnDatasheets]) -> String {
    let mut out = String::new();
    for m in board {
        for u in &m.urls {
            out.push_str(&format!(
                "{}  {}  ({})\n",
                m.mpn,
                u.url,
                u.references.join(" ")
            ));
        }
    }
    let disagree = board.iter().filter(|m| m.urls.len() > 1).count();
    out.push_str(&format!(
        "{} MPNs with a datasheet URL, {disagree} with URLs that disagree\n",
        board.len()
    ));
    out
}

pub(crate) fn parts_datasheets(args: &PartsDatasheetsArgs) -> ExitCode {
    let (name, board) = match better_erc::parts::project_datasheets(&args.project) {
        Ok(b) => b,
        Err(e) => {
            error(&e.to_string());
            return ExitCode::from(EXIT_ERROR);
        }
    };
    let out = if args.to_mx3 {
        match to_mx3(&name, &board, args.dry_run, args.format) {
            Ok(out) => out,
            Err(e) => {
                error(&e);
                return ExitCode::from(EXIT_ERROR);
            }
        }
    } else {
        match args.format {
            ListFormat::Text => listed_text(&board),
            ListFormat::Json => {
                serde_json::to_string_pretty(&board).expect("plain data serialises") + "\n"
            }
        }
    };
    let _ = std::io::stdout().lock().write_all(out.as_bytes());
    ExitCode::SUCCESS
}

#[cfg(feature = "mx3")]
fn to_mx3(
    name: &str,
    board: &[MpnDatasheets],
    dry_run: bool,
    format: ListFormat,
) -> Result<String, String> {
    use better_erc::parts::datasheets::mx3::{Outcome, Write, state};
    use better_erc::parts::mx3::{Connection, Mx3Source};
    let src = Mx3Source::connect(Connection::from_env()).map_err(|e| e.to_string())?;
    let write = if dry_run { Write::DryRun } else { Write::Yes };
    let stated = state(&src, board, name, write).map_err(|e| e.to_string())?;
    if matches!(format, ListFormat::Json) {
        return Ok(serde_json::to_string_pretty(&stated).expect("plain data serialises") + "\n");
    }
    let mut out = String::new();
    for s in &stated {
        let urls: Vec<&str> = s.urls.iter().map(|u| u.url.as_str()).collect();
        let part = s.part.map(|p| format!("  part {p}")).unwrap_or_default();
        let source = s
            .source
            .as_deref()
            .map(|k| format!("  from {k}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{}  {}  {}{part}{source}\n",
            s.mpn,
            s.outcome.as_str(),
            urls.join(" | ")
        ));
    }
    let count = |o: Outcome| stated.iter().filter(|s| s.outcome == o).count();
    let summary = [
        Outcome::Written,
        Outcome::WouldWrite,
        Outcome::Present,
        Outcome::NoPart,
        Outcome::SeveralParts,
        Outcome::Disagree,
        Outcome::BadUrl,
    ]
    .into_iter()
    .filter(|&o| count(o) > 0)
    .map(|o| format!("{} {}", count(o), o.as_str()))
    .collect::<Vec<_>>()
    .join(", ");
    let summary = if summary.is_empty() {
        "no MPN with a datasheet URL".to_owned()
    } else {
        summary
    };
    out.push_str(&format!("{} MPNs: {summary}\n", stated.len()));
    Ok(out)
}

#[cfg(not(feature = "mx3"))]
fn to_mx3(
    _name: &str,
    _board: &[MpnDatasheets],
    _dry_run: bool,
    _format: ListFormat,
) -> Result<String, String> {
    Err("--to-mx3: this berc is built without the mx3 part store (cargo feature mx3)".to_owned())
}
