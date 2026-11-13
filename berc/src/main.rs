use better_erc::diff::DiffError;
use better_erc::model::placecmp::{Origin, PlaceOptions};
use better_erc::{CheckError, CheckOptions, Need, RunOptions, RunState, Severity, Stage, Tier};
use clap::builder::styling::Styles;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::engine::{ArgValueCandidates, ArgValueCompleter};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod complete;
mod progress;
mod usage;

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

/// Exit codes of `berc fp hash`, under its --help
const FP_HASH_AFTER_HELP: &str = "Exit codes: 0 hashed, 2 usage or read error.\n\nExamples:\n  \
berc fp hash Lib.pretty/SOT-23.kicad_mod\n  berc fp hash board.kicad_pcb U3 --canonical";

/// Exit codes of `berc fp check`, under its --help
const FP_CHECK_AFTER_HELP: &str = "Exit codes: 0 every footprint ok (with --lib: every symbol ok or without \
a footprint), 1 any other status, a schematic copy older than its library or a board copy that differs from its library \
footprint, 2 usage or read error.\n\nStatuses: ok, \
changed (the footprint differs from the one reviewed), unreviewed (no hash on the symbol), refused (the symbol sets \
no footprint, so a review means nothing), no symbol (a footprint without a schematic symbol, not marked board-only), \
other version, malformed. Board-only footprints without a symbol (logos, mounting holes) are listed last under their \
own heading and don't fail the run. A board copy that differs from its library footprint (found through \
--footprints and the fp-lib-table) fails the run too: the board was not updated from the library.\n\nWith --lib, \
each symbol of the library is checked against the footprint its \
Footprint field names, read through --footprints and the fp-lib-table: ok, changed, unreviewed, missing (the \
footprint is not found), refused (a hash on a symbol without a footprint), other version, malformed. Symbols without \
a footprint and without a hash are listed last and don't fail the run.\n\nExamples:\n  berc fp check\n  \
berc fp check hw/board --format json\n  berc fp check hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints\n  berc fp check --lib symbols/Lib.kicad_symdir --footprints footprints\n  \
berc fp check --lib symbols --footprints footprints";

/// Exit codes of `berc fp todo`, under its --help
const FP_TODO_AFTER_HELP: &str = "Exit codes: 0 nothing to review, 1 symbols to review or that can't be \
reviewed, 2 usage or read error.\n\nOrder: changed first (reviewed before, the footprint moved on), then other \
version, malformed, unreviewed; then missing (the library symbol or its footprint is not found) and refused (a hash \
on a symbol without a footprint), which need fixing before a review; by symbol within each. --all lists the ok \
ones and those without a footprint too; --format json always has every symbol, with its footprint file, the hash \
to write and the datasheet.\n\nExamples:\n  berc fp todo\n  berc fp todo hw/board --format json\n  \
berc fp todo hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints";

const FP_ACCEPT_AFTER_HELP: &str = "Exit codes: 0 written (or nothing to change), 2 usage, read or write error; \
nothing is written when anything fails.\n\nKiCad 8 and newer files are written; an older file is refused until KiCad \
8+ saved it. A library KiCad has open is read again by KiCad only when it is reopened.\n\nExamples:\n  \
berc fp accept hw/board U1 U2\n  berc fp accept hw/board R1 --dry-run\n  \
berc fp accept hw/board --symbol Device:R --hash fp1:0123456789abcdef";

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
    /// Compare two designs: their placements (--pnp) or their nets (--nets)
    ///
    /// --pnp: A and B are PnP files (KiCad, Altium, Allegro CSV), board files (.kicad_pcb) or KiCad projects with
    /// their board. A board compared with a PnP file is brought into the PnP file's frame first: from --origin, y
    /// up, without the footprints excluded from position files.
    ///
    /// --nets: A and B are any designs with nets (a KiCad project or schematic, a netlist, Altium EDIF or WireList,
    /// OrCAD, an Altium Smart PDF). Each net is the set of pins it connects: nets match by their pins whatever their
    /// names, so a schematic and a PDF of the same board compare even where the PDF names nets per sheet.
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
    /// Footprint review: the review hash of a footprint, every board footprint against its symbol's
    Fp {
        #[command(subcommand)]
        command: FpCommand,
    },
    /// How often each command ran on this PC, from the local usage record
    ///
    /// Every run appends a line to $BERC_USAGE_FILE, else $XDG_STATE_HOME/berc/usage.tsv, else
    /// ~/.local/state/berc/usage.tsv; nothing is sent anywhere. BERC_USAGE=off records nothing. Commands never run
    /// show 0, so unused ones stand out.
    Usage,
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

#[derive(Subcommand)]
enum FpCommand {
    /// Print the review hash of a footprint: a library file, or a footprint on a board by its reference
    ///
    /// The hash (`fp1:` and 16 hex digits) covers what a footprint review checks against the datasheet: pads,
    /// copper, mask, paste, courtyard and board-edge graphics, zones and keepouts, clearance and mask settings.
    /// Silk, fab, texts, the 3D model and the footprint's place on the board don't count, so a board copy has the
    /// hash of its library file. Put it in the library symbol's `Footprint_Review` field once the footprint is
    /// reviewed.
    #[command(after_help = FP_HASH_AFTER_HELP)]
    Hash {
        /// A library footprint (.kicad_mod) or a board (.kicad_pcb)
        #[arg(value_hint = ValueHint::FilePath)]
        file: PathBuf,
        /// The footprint's reference, for a board
        #[arg(add = ArgValueCandidates::new(complete::board_references))]
        reference: Option<String>,
        /// Also print the canonical text the hash is taken over, one line per pad, graphic and zone
        #[arg(long)]
        canonical: bool,
        #[arg(long, value_enum, default_value_t = ListFormat::Text)]
        format: ListFormat,
    },
    /// Check every footprint of a board against the review hash on its library symbol, or every symbol of a library
    ///
    /// Each board footprint's review hash (as `berc fp hash` prints it) is compared with the `Footprint_Review`
    /// field of its symbol's library symbol: in the .kicad_sym found through the project's or the global
    /// sym-lib-table, else in the schematic's copy of it. A schematic copy that holds another hash than the library
    /// is reported too: the schematic was not updated from the library. So is a board copy that differs from its
    /// library footprint: the board was not updated from the library.
    ///
    /// With --lib, the symbols of a symbol library instead: each against the library footprint its `Footprint`
    /// field names, so a library is checked before any board uses it.
    #[command(after_help = FP_CHECK_AFTER_HELP)]
    Check(FpCheckArgs),
    /// List the library symbols of a board's footprints that need a footprint review, in review order
    ///
    /// Each library symbol a board footprint comes from, once, checked as `berc fp check --lib` checks a library:
    /// its `Footprint_Review` field against the library footprint its `Footprint` field names. Those unreviewed or
    /// changed since their review are the review's steps; the review is of the library, so every board using the
    /// symbol gains from it.
    #[command(after_help = FP_TODO_AFTER_HELP)]
    Todo(FpTodoArgs),
    /// Write the review hash of reviewed footprints into their library symbols
    ///
    /// Each board footprint's review hash now goes into the `Footprint_Review` field of its schematic symbol's
    /// library symbol: in the .kicad_sym (or .kicad_symdir file) found through the project's or the global
    /// sym-lib-table, and in every schematic sheet's copy of it, so `berc fp check` finds them ok. Only the field
    /// is written: the rest of each file stays byte for byte as it was, and a new field is laid out as KiCad lays
    /// it out. With --symbol and --hash, a hash for a library symbol given by hand.
    #[command(after_help = FP_ACCEPT_AFTER_HELP)]
    Accept(FpAcceptArgs),
}

/// Libraries searched before the sym-lib-table and fp-lib-table
#[derive(clap::Args)]
struct LibraryArgs {
    /// A symbol library (.kicad_sym, .kicad_symdir) or a folder of them, searched before the sym-lib-table, each
    /// by its file name; repeatable
    #[arg(long, value_name = "DIR", value_hint = ValueHint::AnyPath)]
    symbols: Vec<PathBuf>,
    /// A footprint library (.pretty) or a folder of them, searched before the fp-lib-table, each by its folder
    /// name; repeatable
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    footprints: Vec<PathBuf>,
    /// A library nickname symbols and their footprints may be in, repeatable: an exact name, or a prefix with a
    /// trailing `*` (`VHRD_*`). With --lib or in `berc fp todo`: any other library named is reported. None (the
    /// default): no rule
    #[arg(long, value_name = "NICKNAME")]
    allow: Vec<String>,
}

impl LibraryArgs {
    fn search(&self) -> better_erc::fp::LibrarySearch {
        better_erc::fp::LibrarySearch {
            symbols: self.symbols.clone(),
            footprints: self.footprints.clone(),
            allowed: self.allow.clone(),
            ..better_erc::fp::LibrarySearch::kicad()
        }
    }
}

#[derive(clap::Args)]
struct FpTodoArgs {
    /// The KiCad project: a directory, its .kicad_pro, .kicad_sch or .kicad_pcb
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    project: PathBuf,
    #[command(flatten)]
    libraries: LibraryArgs,
    /// Also list the symbols reviewed and unchanged, and those without a footprint
    #[arg(long)]
    all: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("what").required(true).args(["references", "symbol"])))]
struct FpAcceptArgs {
    /// The KiCad project: a directory, its .kicad_pro, .kicad_sch or .kicad_pcb
    #[arg(value_hint = ValueHint::AnyPath)]
    project: PathBuf,
    /// The board footprints reviewed, by reference
    #[arg(add = ArgValueCandidates::new(complete::board_references))]
    references: Vec<String>,
    /// The library symbol (`Lib:Name`) to write --hash into, instead of board footprints
    #[arg(long, value_name = "LIB:NAME", requires = "hash")]
    symbol: Option<String>,
    /// The review hash for --symbol, as `berc fp hash` prints it
    #[arg(long, value_name = "HASH", requires = "symbol")]
    hash: Option<String>,
    /// Also set the library symbol's Footprint to this (`Lib:Name`): offered by `berc fp check` as the placed
    /// footprint, when it differs from what the symbol itself names
    #[arg(long, value_name = "LIB:NAME")]
    footprint: Option<String>,
    /// Show what would be written, write nothing
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

#[derive(clap::Args)]
struct FpCheckArgs {
    /// The KiCad project: a directory, its .kicad_pro, .kicad_sch or .kicad_pcb. With --lib, the directory whose
    /// sym-lib-table and fp-lib-table are read besides the global ones
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    project: PathBuf,
    /// Check every symbol of this symbol library instead of a board, repeatable: a .kicad_sym, a .kicad_symdir, a
    /// folder of them, or a nickname in the sym-lib-table
    #[arg(long, value_name = "LIB",
          add = ArgValueCompleter::new(complete::symbol_libraries))]
    lib: Vec<String>,
    #[command(flatten)]
    libraries: LibraryArgs,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
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
    /// A setting over its default, repeatable: `drawing.clearance.part=2`, `I2C-9.min_pull_up=1000`
    #[arg(long = "set", value_name = "NAME=VALUE")]
    set: Vec<String>,
    /// A KiCad colour theme (`colors/*.json`), repeatable: what `DRAW-42` looks at the sheets on
    #[arg(long, value_name = "THEME.json", value_hint = ValueHint::FilePath)]
    theme: Vec<PathBuf>,
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
#[command(group(clap::ArgGroup::new("mode").required(true).args(["pnp", "nets"])))]
struct DiffArgs {
    /// The first design: a file or a directory
    #[arg(value_hint = ValueHint::AnyPath)]
    a: PathBuf,
    /// The second design
    #[arg(value_hint = ValueHint::AnyPath)]
    b: PathBuf,
    /// Compare the placements: each footprint's position, rotation, side, value and package by reference
    #[arg(long)]
    pnp: bool,
    /// Compare the nets: each net as the pins it connects; renamed, removed and added nets and net class changes
    #[arg(long)]
    nets: bool,
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
    let matches = Cli::command().get_matches();
    usage::record(&usage::signals(&matches));
    let cli = match Cli::from_arg_matches(&matches) {
        Ok(c) => c,
        Err(e) => e.exit(),
    };
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
        Command::Fp {
            command:
                FpCommand::Hash {
                    file,
                    reference,
                    canonical,
                    format,
                },
        } => fp_hash(&file, reference.as_deref(), canonical, format),
        Command::Fp {
            command: FpCommand::Check(args),
        } => fp_check(&args),
        Command::Fp {
            command: FpCommand::Todo(args),
        } => fp_todo(&args),
        Command::Fp {
            command: FpCommand::Accept(args),
        } => fp_accept(&args),
        Command::Usage => {
            let out = usage::summary(&usage::known(&Cli::command()));
            let _ = std::io::stdout().lock().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
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
    let mut config = better_erc::Config::default();
    for set in &args.set {
        let parsed = set
            .split_once('=')
            .and_then(|(k, v)| Some((k, v.parse().ok()?)));
        let done = parsed.map_or(Err(format!("--set {set}: not NAME=VALUE")), |(k, v)| {
            config.set(k, v)
        });
        if let Err(e) = done {
            error(&e);
            return ExitCode::from(CheckError::EXIT_CODE);
        }
    }
    for path in &args.theme {
        let read = std::fs::read_to_string(path).map_err(|e| e.to_string());
        match read.and_then(|text| better_erc::kicad::theme::read(&text)) {
            Ok(theme) => config.themes.push(theme),
            Err(e) => {
                error(&format!("--theme {}: {e}", path.display()));
                return ExitCode::from(CheckError::EXIT_CODE);
            }
        }
    }
    let opts = CheckOptions {
        run: RunOptions {
            variant: args.variant,
            only: args.only,
            config,
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
    if args.nets {
        let spinner = progress::Delayed::start();
        spinner.set("reading the designs".into());
        let d = better_erc::diff::nets(&args.a, &args.b);
        spinner.finish();
        return match d {
            Ok(d) => show_diff(args.format, &d.to_text_color(), &d.to_json(), d.is_empty()),
            Err(e) => {
                error(&e.to_string());
                ExitCode::from(DiffError::EXIT_CODE)
            }
        };
    }
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
    show_diff(args.format, &d.to_text_color(), &d.to_json(), d.is_empty())
}

/// A comparison on stdout; exit 1 when it found a difference
fn show_diff(format: ListFormat, text: &str, json: &str, same: bool) -> ExitCode {
    match format {
        ListFormat::Text => {
            let _ = anstream::stdout().lock().write_all(text.as_bytes());
        }
        ListFormat::Json => {
            let _ = writeln!(std::io::stdout().lock(), "{json}");
        }
    }
    if same {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn fp_hash(file: &Path, reference: Option<&str>, canonical: bool, format: ListFormat) -> ExitCode {
    let h = match better_erc::fp::hash_file(file, reference) {
        Ok(h) => h,
        Err(e) => {
            error(&e.to_string());
            return ExitCode::from(better_erc::fp::FpError::EXIT_CODE);
        }
    };
    let mut out = anstream::stdout().lock();
    match format {
        ListFormat::Json => {
            let mut v = serde_json::to_value(&h).expect("serialises");
            if !canonical {
                v.as_object_mut().expect("an object").remove("canonical");
            }
            let _ = writeln!(
                out,
                "{}",
                serde_json::to_string_pretty(&v).expect("serialises")
            );
        }
        ListFormat::Text => {
            let _ = writeln!(out, "{}", h.hash);
            if canonical {
                let dim = anstyle::Style::new().dimmed();
                for l in &h.canonical {
                    let _ = writeln!(out, "{dim}{l}{dim:#}");
                }
            }
        }
    }
    ExitCode::SUCCESS
}

fn fp_check(args: &FpCheckArgs) -> ExitCode {
    use better_erc::fp::FpCheckError;
    if !args.lib.is_empty() {
        return fp_check_libraries(args);
    }
    let spinner = progress::Delayed::start();
    spinner.set(format!("reading {}", args.project.display()));
    let run = better_erc::fp::check(&args.project, &args.libraries.search());
    spinner.finish();
    let run = match run {
        Ok(r) => r,
        Err(e) => {
            error(&e.to_string());
            return ExitCode::from(FpCheckError::EXIT_CODE);
        }
    };
    show_diff(
        args.format,
        &run.to_text_color(),
        &run.to_json(),
        run.is_ok(),
    )
}

fn fp_check_libraries(args: &FpCheckArgs) -> ExitCode {
    use better_erc::fp::{LibCheckError, LibCheckOptions};
    let project = if args.project.is_file() {
        args.project
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    } else {
        args.project.clone()
    };
    let opts = LibCheckOptions {
        libraries: args.lib.clone(),
        project,
        search: args.libraries.search(),
    };
    let spinner = progress::Delayed::start();
    spinner.set(format!("reading {}", args.lib.join(", ")));
    let run = better_erc::fp::check_libraries(&opts);
    spinner.finish();
    match run {
        Ok(r) => show_diff(args.format, &r.to_text_color(), &r.to_json(), r.is_ok()),
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(LibCheckError::EXIT_CODE)
        }
    }
}

fn fp_todo(args: &FpTodoArgs) -> ExitCode {
    let spinner = progress::Delayed::start();
    spinner.set(format!("reading {}", args.project.display()));
    let run = better_erc::fp::todo(&args.project, &args.libraries.search());
    spinner.finish();
    match run {
        Ok(r) => show_diff(
            args.format,
            &r.to_text_color(args.all),
            &r.to_json(),
            r.is_ok(),
        ),
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(better_erc::fp::FpCheckError::EXIT_CODE)
        }
    }
}

fn fp_accept(args: &FpAcceptArgs) -> ExitCode {
    use better_erc::fp::{AcceptError, AcceptWhat, SymbolLibraries, kicad_config_dir};
    let what = match (&args.symbol, &args.hash) {
        (Some(symbol), Some(hash)) => AcceptWhat::Symbol {
            symbol: symbol.clone(),
            hash: hash.clone(),
        },
        _ => AcceptWhat::References(args.references.clone()),
    };
    let spinner = progress::Delayed::start();
    spinner.set(format!("reading {}", args.project.display()));
    let config = kicad_config_dir();
    let run = better_erc::fp::accept(
        &args.project,
        &what,
        &mut |dir| SymbolLibraries::new(dir, config.as_deref()),
        args.footprint.as_deref(),
        args.dry_run,
    );
    spinner.finish();
    match run {
        Ok(r) => {
            match args.format {
                ListFormat::Text => {
                    let _ = anstream::stdout()
                        .lock()
                        .write_all(r.to_text_color().as_bytes());
                }
                ListFormat::Json => {
                    let _ = writeln!(std::io::stdout().lock(), "{}", r.to_json());
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(AcceptError::EXIT_CODE)
        }
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
