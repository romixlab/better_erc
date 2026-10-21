use clap::builder::styling::Styles;
use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use std::path::PathBuf;
use std::process::ExitCode;

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

/// better_erc command line tools
///
/// Colour in help and errors only on a terminal, and never with NO_COLOR set; `textconv` output is always plain
/// text, git colours the diff itself.
#[derive(Parser)]
#[command(name = "berc", version = VERSION, after_help = AFTER_HELP, styles = Styles::styled())]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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

fn main() -> ExitCode {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();
    match cli.command {
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
            use std::io::Write;
            // A closed pipe (git diff into a pager that quit) is not an error worth a message
            let _ = std::io::stdout().lock().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
    }
}

/// `berc: error: ...` on stderr, the `error:` in red on a terminal (anstream drops the colour when stderr isn't
/// one, or NO_COLOR is set)
fn error(msg: &str) {
    let red = anstyle::AnsiColor::Red.on_default().bold();
    anstream::eprintln!("berc: {red}error:{red:#} {msg}");
}
