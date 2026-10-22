use better_erc::diff::DiffError;
use better_erc::model::placecmp::{Origin, PlaceOptions};
use better_erc::{CheckError, CheckOptions, Need, RunOptions, RunState, Severity, Stage, Tier};
use clap::builder::styling::Styles;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::engine::ArgValueCandidates;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod complete;
mod progress;

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_SHA"),
    ", commit ",
    env!("BUILD_TIME"),
    ")"
);

/// Shown under --help. Completion is dynamic (clap_complete's `CompleteEnv`): the shell asks berc itself.
const AFTER_HELP: &str = "Shell completion: add `source <(COMPLETE=bash berc)` to ~/.bashrc, \
`source <(COMPLETE=zsh berc)` to ~/.zshrc, or `COMPLETE=fish berc | source` to ~/.config/fish/config.fish.";

/// Exit codes of `berc diff`, under its --help
const DIFF_AFTER_HELP: &str = "Exit codes: 0 the same, 1 different, 2 usage or import error.";

/// Exit codes of `berc check`, under its --help
const CHECK_AFTER_HELP: &str = "Exit codes: 0 nothing at or above --fail-on, 1 findings at or above --fail-on \
(with --baseline: new ones only), 2 usage or import error, 3 a --require'd need is not available.";

/// better_erc command line tools
///
/// Colour in help, errors and `check` text only on a terminal, and never with NO_COLOR set; `textconv` output is
/// always plain text, git colours the diff itself.
#[derive(Parser)]
#[command(name = "berc", version = VERSION, after_help = AFTER_HELP, styles = Styles::styled())]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the checks on a design, for CI: text, JSON, SARIF or Markdown, exit code by severity
    ///
    /// PROJECT is a directory with one design in it (its .kicad_pro) or a design file: .kicad_pro, .kicad_sch,
    /// a KiCad netlist, Altium EDIF, OrCAD pstxnet, a PnP file.
    #[command(after_help = CHECK_AFTER_HELP)]
    Check(CheckArgs),
    /// Explain a check of the catalogue (implemented or planned)
    Explain {
        /// The catalogue id (`I2C-3`)
        #[arg(add = ArgValueCandidates::new(complete::catalogue_ids))]
        id: String,
        #[arg(long, value_enum, default_value_t = ExplainFormat::Text)]
        format: ExplainFormat,
    },
    /// Compare two designs: so far their placements (--pnp)
    ///
    /// A and B are PnP files (KiCad, Altium, Allegro CSV), board files (.kicad_pcb) or KiCad projects with their
    /// board. A board compared with a PnP file is brought into the PnP file's frame first: from --origin, y up,
    /// without the footprints excluded from position files.
    #[command(after_help = DIFF_AFTER_HELP)]
    Diff(DiffArgs),
    /// The check catalogue
    Checks {
        #[command(subcommand)]
        command: ChecksCommand,
    },
    /// Print a .kicad_sch, .kicad_pcb or .kicad_pro as sorted, line-oriented text for git diff
    ///
    /// One fact per line, no UUIDs, no coordinates, so re-saving or reordering never shows as a change.
    /// Set up once (see README "Readable git diffs"):
    ///   .gitattributes:  *.kicad_sch diff=kicad
    ///                    *.kicad_pcb diff=kicad
    ///                    *.kicad_pro diff=kicad
    ///   git config --global diff.kicad.textconv "berc textconv"
    ///   git config --global diff.kicad.cachetextconv true
    #[command(verbatim_doc_comment)]
    Textconv {
        /// The KiCad file (git passes a temporary copy of each side)
        #[arg(value_hint = ValueHint::FilePath)]
        file: PathBuf,
        /// Add positions, rotations, wires and tracks
        #[arg(long)]
        with_geometry: bool,
        /// .kicad_pro only: also show settings that look like KiCad's default, folded out by default
        #[arg(long)]
        all: bool,
    },
}

#[derive(clap::Args)]
struct CheckArgs {
    /// The design: a directory or a file
    #[arg(value_hint = ValueHint::AnyPath)]
    project: PathBuf,
    /// Check this variant (DNP and field overrides applied) instead of the base design
    #[arg(long, value_name = "V", add = ArgValueCandidates::new(complete::variants))]
    variant: Option<String>,
    /// Power state to check in: reserved, power states are not built yet (NET-8)
    #[arg(long, value_name = "S")]
    state: Option<String>,
    /// Only these check ids and categories, comma-separated (`I2C,BOM-5`)
    #[arg(long, value_name = "CAT,ID", value_delimiter = ',',
          add = ArgValueCandidates::new(complete::check_ids_and_categories))]
    only: Vec<String>,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// A previous JSON report (`--format json`): findings are marked new or unchanged, the fixed ones listed, and
    /// only new findings fail the run
    #[arg(long, value_name = "REPORT.json", value_hint = ValueHint::FilePath)]
    baseline: Option<PathBuf>,
    /// Findings of this severity or worse make the exit code 1
    #[arg(long, value_enum, default_value_t = FailOn::Error)]
    fail_on: FailOn,
    /// Data that must be there on every board, else exit code 3 (repeat or comma-separate)
    #[arg(long, value_name = "NEED", value_delimiter = ',', value_parser = parse_need,
          add = ArgValueCandidates::new(complete::needs))]
    require: Vec<Need>,
}

#[derive(clap::Args)]
struct DiffArgs {
    /// The first design: a file or a directory
    #[arg(value_hint = ValueHint::AnyPath)]
    a: PathBuf,
    /// The second design
    #[arg(value_hint = ValueHint::AnyPath)]
    b: PathBuf,
    /// Compare the placements: each footprint's position, rotation, side, value and package by reference
    #[arg(long, required = true)]
    pnp: bool,
    /// The origin the PnP file was exported from, for a board compared with one
    #[arg(long, value_enum, default_value_t = OriginArg::Page)]
    origin: OriginArg,
    /// Largest difference in x or y that counts as the same position, in mm
    #[arg(long, value_name = "MM", default_value_t = 0.0)]
    tolerance: f64,
    /// Largest difference in rotation that counts as the same, in degrees
    #[arg(long, value_name = "DEG", default_value_t = 0.0)]
    rotation_tolerance: f64,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

#[derive(Clone, Copy, ValueEnum)]
enum OriginArg {
    /// The page's top left corner (KiCad's default)
    Page,
    /// The board's drill and place file origin (KiCad's --use-drill-file-origin)
    Aux,
}

#[derive(Subcommand)]
enum ChecksCommand {
    /// Every check of the catalogue, implemented or planned; with --project what a run there does with each
    List {
        #[arg(long, value_enum)]
        tier: Option<TierArg>,
        /// ran and not-run need --project
        #[arg(long, value_enum)]
        status: Option<StatusArg>,
        /// The design to tell ran from not run on
        #[arg(long, value_name = "PROJECT", value_hint = ValueHint::AnyPath)]
        project: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = ListFormat::Text)]
        format: ListFormat,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    /// Findings by severity and category, the checks that did not run and what would enable them
    Text,
    /// The report, JSON schema 1 (usable as a --baseline later)
    Json,
    /// SARIF 2.1.0, for GitHub code scanning
    Sarif,
    /// Markdown, for a pull request comment
    Md,
}

#[derive(Clone, Copy, ValueEnum)]
enum FailOn {
    Error,
    Warning,
    Suggestion,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExplainFormat {
    Text,
    Md,
}

#[derive(Clone, Copy, ValueEnum)]
enum ListFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum TierArg {
    Easy,
    Mid,
    Hard,
    Judgement,
}

#[derive(Clone, Copy, ValueEnum)]
enum StatusArg {
    Ran,
    NotRun,
    Planned,
}

fn parse_need(s: &str) -> Result<Need, String> {
    Need::from_slug(s).ok_or_else(|| {
        let names: Vec<String> = Need::FIXED.iter().map(|n| n.slug()).collect();
        format!("no need `{s}`; one of {}", names.join(", "))
    })
}

fn main() -> ExitCode {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();
    match cli.command {
        Command::Check(args) => check(args),
        Command::Explain { id, format } => explain(&id, format),
        Command::Diff(args) => diff(args),
        Command::Checks {
            command:
                ChecksCommand::List {
                    tier,
                    status,
                    project,
                    format,
                },
        } => list(tier, status, project.as_deref(), format),
        Command::Textconv {
            file,
            with_geometry,
            all,
        } => {
            let bytes = match std::fs::read(&file) {
                Ok(b) => b,
                Err(e) => {
                    error(&format!("can't read {}: {e}", file.display()));
                    return ExitCode::FAILURE;
                }
            };
            let text = String::from_utf8_lossy(&bytes);
            let out = berc::textconv(
                &text,
                berc::Options {
                    with_geometry,
                    show_defaults: all,
                },
            );
            // A closed pipe (git diff into a pager that quit) is not an error worth a message
            let _ = std::io::stdout().lock().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
    }
}

fn check(args: CheckArgs) -> ExitCode {
    if args.state.is_some() {
        error(
            "--state is reserved for power states, which are not built yet (NET-8): \
             the checks run with everything powered",
        );
        return ExitCode::from(CheckError::EXIT_CODE);
    }
    let baseline = match args.baseline.as_deref().map(better_erc::read_baseline) {
        Some(Ok(b)) => Some(b),
        Some(Err(e)) => return check_error(&e),
        None => None,
    };
    let opts = CheckOptions {
        run: RunOptions {
            variant: args.variant,
            only: args.only,
            ..RunOptions::default()
        },
        require: args.require,
        fail_on: match args.fail_on {
            FailOn::Error => Severity::Error,
            FailOn::Warning => Severity::Warning,
            FailOn::Suggestion => Severity::Suggestion,
        },
        baseline,
    };
    let spinner = progress::Delayed::start();
    let run = better_erc::check_path(&args.project, &opts, &mut |stage| {
        spinner.set(match stage {
            Stage::Importing(file) => format!("reading {file}"),
            Stage::Checking => "running the checks".into(),
        })
    });
    spinner.finish();
    let run = match run {
        Ok(r) => r,
        Err(e) => return check_error(&e),
    };
    let format = match args.format {
        Format::Text => better_erc::Format::Text,
        Format::Json => better_erc::Format::Json,
        Format::Sarif => better_erc::Format::Sarif,
        Format::Md => better_erc::Format::Md,
    };
    let out = run.render(format, true);
    if matches!(args.format, Format::Text) {
        // anstream drops the colours on a pipe and with NO_COLOR
        let _ = anstream::stdout().lock().write_all(out.as_bytes());
    } else {
        let _ = std::io::stdout().lock().write_all(out.as_bytes());
    }
    for u in &run.verdict.unavailable {
        error(&format!(
            "required {} not available on {}: {}",
            u.need, u.board, u.why
        ));
    }
    ExitCode::from(run.verdict.exit.code())
}

fn diff(args: DiffArgs) -> ExitCode {
    let opts = PlaceOptions {
        position_tolerance: (args.tolerance * 1e6).round() as i64,
        rotation_tolerance: args.rotation_tolerance,
        origin: match args.origin {
            OriginArg::Page => Origin::Page,
            OriginArg::Aux => Origin::Aux,
        },
    };
    let spinner = progress::Delayed::start();
    spinner.set("reading the designs".into());
    let d = better_erc::diff::pnp(&args.a, &args.b, &opts);
    spinner.finish();
    let d = match d {
        Ok(d) => d,
        Err(e) => {
            error(&e.to_string());
            return ExitCode::from(DiffError::EXIT_CODE);
        }
    };
    match args.format {
        ListFormat::Text => {
            let _ = anstream::stdout()
                .lock()
                .write_all(d.to_text_color().as_bytes());
        }
        ListFormat::Json => {
            let _ = writeln!(std::io::stdout().lock(), "{}", d.to_json());
        }
    }
    if d.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn check_error(e: &CheckError) -> ExitCode {
    error(&e.to_string());
    ExitCode::from(CheckError::EXIT_CODE)
}

fn explain(id: &str, format: ExplainFormat) -> ExitCode {
    let Some(e) = better_erc::explain(&id.to_uppercase()) else {
        error(&format!(
            "no check {id} in the catalogue (`berc checks list` lists them)"
        ));
        return ExitCode::from(2);
    };
    let out = match format {
        ExplainFormat::Text => e.to_text(),
        ExplainFormat::Md => e.to_markdown(),
    };
    let _ = std::io::stdout().lock().write_all(out.as_bytes());
    ExitCode::SUCCESS
}

fn list(
    tier: Option<TierArg>,
    status: Option<StatusArg>,
    project: Option<&Path>,
    format: ListFormat,
) -> ExitCode {
    let status = status.map(|s| match s {
        StatusArg::Ran => RunState::Ran,
        StatusArg::NotRun => RunState::NotRun,
        StatusArg::Planned => RunState::Planned,
    });
    let report = match project {
        Some(p) => {
            let spinner = progress::Delayed::start();
            let run = better_erc::check_path(p, &CheckOptions::default(), &mut |stage| {
                if let Stage::Importing(file) = stage {
                    spinner.set(format!("reading {file}"));
                }
            });
            spinner.finish();
            match run {
                Ok(r) => Some(r.report),
                Err(e) => return check_error(&e),
            }
        }
        None if matches!(status, Some(RunState::Ran | RunState::NotRun)) => {
            error("--status ran and not-run need a design: add --project PATH");
            return ExitCode::from(2);
        }
        None => None,
    };
    let tier = tier.map(|t| match t {
        TierArg::Easy => Tier::Easy,
        TierArg::Mid => Tier::Mid,
        TierArg::Hard => Tier::Hard,
        TierArg::Judgement => Tier::Judgement,
    });
    let checks: Vec<_> = better_erc::list_for(report.as_ref())
        .into_iter()
        .filter(|c| tier.is_none_or(|t| c.info.tier == t))
        .filter(|c| match status {
            None => true,
            Some(RunState::Planned) => c.info.state == better_erc::CheckState::Planned,
            Some(s) => c.run == Some(s),
        })
        .collect();
    match format {
        ListFormat::Json => {
            let json = serde_json::to_string_pretty(&checks).expect("serialises");
            let _ = writeln!(std::io::stdout().lock(), "{json}");
        }
        ListFormat::Text => {
            let mut out = anstream::stdout().lock();
            for c in &checks {
                let i = &c.info;
                let state = match c.run {
                    Some(r) => r.name(),
                    None if i.state == better_erc::CheckState::Implemented => "implemented",
                    None => "planned",
                };
                let sev = severity_style(i.severity);
                let _ = writeln!(
                    out,
                    "{:<8} {sev}{:<10}{sev:#} {:<9} {:<11} {}",
                    i.id,
                    i.severity.name(),
                    i.tier.name(),
                    state,
                    i.title
                );
            }
        }
    }
    ExitCode::SUCCESS
}

fn severity_style(s: Severity) -> anstyle::Style {
    match s {
        Severity::Error => anstyle::AnsiColor::Red.on_default().bold(),
        Severity::Warning => anstyle::AnsiColor::Yellow.on_default().bold(),
        Severity::Suggestion => anstyle::AnsiColor::Cyan.on_default(),
        Severity::Info => anstyle::Style::new().dimmed(),
    }
}

/// `berc: error: ...` on stderr, the `error:` in red on a terminal (anstream drops the colour when stderr isn't
/// one, or NO_COLOR is set)
fn error(msg: &str) {
    let red = anstyle::AnsiColor::Red.on_default().bold();
    anstream::eprintln!("berc: {red}error:{red:#} {msg}");
}
