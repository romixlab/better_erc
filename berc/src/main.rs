use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use std::path::PathBuf;
use std::process::ExitCode;

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_SHA"),
    ", built ",
    env!("BUILD_TIME"),
    ")"
);

/// better_erc command line tools
#[derive(Parser)]
#[command(name = "berc", version = VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print a .kicad_sch or .kicad_pcb as sorted, line-oriented text for git diff
    ///
    /// One fact per line, no UUIDs, no coordinates, so re-saving or reordering never shows as a change.
    /// Set up once (see README "Readable git diffs"):
    ///   .gitattributes:  *.kicad_sch diff=kicad
    ///                    *.kicad_pcb diff=kicad
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
    },
}

fn main() -> ExitCode {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
    let cli = Cli::parse();
    match cli.command {
        Command::Textconv {
            file,
            with_geometry,
        } => {
            let bytes = match std::fs::read(&file) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("berc: can't read {}: {e}", file.display());
                    return ExitCode::FAILURE;
                }
            };
            let text = String::from_utf8_lossy(&bytes);
            let out = berc::textconv(&text, berc::Options { with_geometry });
            use std::io::Write;
            // A closed pipe (git diff into a pager that quit) is not an error worth a message
            let _ = std::io::stdout().lock().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
    }
}
