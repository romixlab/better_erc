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

/// Exit codes of `berc waive`, under its --help
const WAIVE_AFTER_HELP: &str = "Exit codes: 0 written, 1 no finding or several match (without --all), 2 usage, \
import or write error.\n\nExamples:\n  berc waive hw/board I2C-1 R5 --reason \"pull-up on the mating board\"\n  \
berc waive hw/board 'BOM-14:part R9' --reason \"DNP in every variant\" --expires 2027-06-30";

/// Exit codes of `berc fp hash`, under its --help
const FP_HASH_AFTER_HELP: &str = "Exit codes: 0 hashed, 2 usage or read error.\n\nExamples:\n  \
berc fp hash Lib.pretty/SOT-23.kicad_mod\n  berc fp hash board.kicad_pcb U3 --canonical";

/// Exit codes of `berc fp check`, under its --help
const FP_CHECK_AFTER_HELP: &str = "Exit codes: 0 every footprint ok (with --lib: every symbol ok, without an MPN \
or without a footprint), 1 any other status, a board copy that differs from its library footprint or a footprint \
placed under another Lib:Name than its symbol names, 2 usage or read error.\n\nStatuses: ok (a review of the \
footprint's hash covers every MPN of its part), changed (an MPN is covered only by a review of another hash: the \
footprint differs from the one reviewed), other version, not in lock (run `berc parts refresh`), partial (some MPNs \
covered), uncovered, no MPN, no symbol (a footprint without a schematic symbol, not marked board-only). Board-only \
footprints and parts fitted in no variant don't fail the run.\n\nWith --lib, each symbol that names its MPNs is \
judged against the reviews --from holds for the footprint its Footprint field names: ok, changed, other version, \
partial, uncovered, missing (the footprint is not found), unjudged (no --from); symbols without an MPN or a \
footprint don't fail the run.\n\nExamples:\n  berc fp check\n  berc fp check hw/board --format json --details\n  \
berc fp check hw/board --footprints ~/kicad-library/footprints\n  \
berc fp check --lib symbols/Lib.kicad_symdir --footprints footprints --from parts.toml";

/// Exit codes of `berc fp todo`, under its --help
const FP_TODO_AFTER_HELP: &str = "Exit codes: 0 nothing to review, 1 footprints to review or that can't be \
reviewed, 2 usage or read error.\n\nOne line per board footprint hash and family (or MPN reviewed alone), with \
the MPNs and references it stands for. Order: changed first (reviewed before, the footprint moved on), then other \
version, uncovered; then not in lock (refresh the snapshot), unknown MPN (the part source has no part of it) and \
no MPN, which need fixing before a review. --all lists the reviewed ones too; --format json always has every \
entry, with the hash, what a review covers and the datasheet.\n\nExamples:\n  berc fp todo\n  \
berc fp todo hw/board --format json\n  berc fp todo hw/board --all";

/// Exit codes of `berc fp accept`, under its --help
const FP_ACCEPT_AFTER_HELP: &str = "Exit codes: 0 recorded (or a dry run), 2 usage, read or source error; \
nothing is recorded when anything before the record fails.\n\nCovers: every MPN of the references, each by its \
family where its MPN decodes into one (scheme and package, as `berc fp todo` lists it), else alone. --mpn and \
--family-of pick MPNs instead.\n\nExamples:\n  berc fp accept hw/board R1 R7 --from parts.toml --doc RC_Series@rev12 --page 4\n  \
berc fp accept hw/board U3 --from parts.toml --mpn TPS62840DLCR --dry-run\n  \
berc fp accept --hash fp1:0123456789abcdef --footprint-id Resistor_SMD:R_0402 --family-of RC0402FR-0710KL --from parts.toml";

const FP_MODEL_AFTER_HELP: &str = "Exit codes: 0 no warning, 1 a warning on any model (not found or not \
read, out of the courtyard, a pad without a lead, scaled, turned by an odd angle), 2 usage or read error. \
Suggestions (no model, not in mm) don't fail the run.\n\nFindings: LIB-22 model file missing or unreadable, \
LIB-23 model out of the courtyard, LIB-24 a pad without a lead, LIB-25 model scaled, LIB-26 model not in mm, \
LIB-27 model turned by an angle that is not a multiple of 90°, LIB-28 no model shown (berc explain LIB-24).\n\n\
Examples:\n  berc fp model hw/board\n  berc fp model hw/board U3 J1 --details\n  \
berc fp model ~/kicad-library/footprints/VHRD_Package_SO.pretty --format json\n  \
berc fp model Lib.pretty/SOT-23.kicad_mod";

/// Exit codes of `berc place-fields`, under its --help
const PLACE_FIELDS_AFTER_HELP: &str = "Exit codes: 0 planned (or written), 2 usage, read or write error; nothing \
is written when any file fails to edit.\n\nA written file differs only in the moved fields' (at ...) and justify, \
the hidden fields' (hide yes), and its moved symbols lose (fields_autoplaced yes), so KiCad keeps the new places. Close the schematic in KiCad \
before --write.\n\nExamples:\n  berc place-fields hw/board\n  berc place-fields hw/board --sheet power --write";

/// Exit codes of `berc fields sync`, under its --help
const FIELDS_SYNC_AFTER_HELP: &str = "Exit codes: 0 planned (or written), 2 usage, read or write error, no \
parts.lock.toml; nothing is written when any file fails to edit.\n\nClose the schematic in KiCad before --write. \
Run `berc parts refresh` first.\n\nExamples:\n  berc fields sync hw/board\n  berc fields sync hw/board --write";

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
    /// Accept a finding for this project: append a [[waiver]] to its project file
    ///
    /// FINDING is a check id with subject names that pick one finding (`I2C-1 R5`, `BOM-14 "part R9"`), or a
    /// full fingerprint as the JSON report gives it (`I2C-1:net /SDA|part R5`). The design is checked first; the
    /// waiver keeps the finding's input hash, so it goes stale when what was judged changes. Written to
    /// `<name>.berc.toml` beside the design, else `berc.toml`, created when neither exists.
    #[command(after_help = WAIVE_AFTER_HELP)]
    Waive(WaiveArgs),
    /// Explain a check of the catalogue (implemented or planned)
    Explain {
        /// The catalogue id (`I2C-3`)
        #[arg(add = ArgValueCandidates::new(complete::catalogue_ids))]
        id: String,
        #[arg(long, value_enum, default_value_t = ExplainFormat::Text)]
        format: ExplainFormat,
    },
    /// Move symbol fields that overlap something into a column beside their body (DRAW-9, DRAW-34)
    ///
    /// Each placed symbol whose shown fields overlap text, a body, a wire or the sheet edge, or come closer than
    /// the drawing.clearance margins, gets the first that clears them: rating fields touching a power symbol
    /// hidden in place; a column (Reference, Value, the rest) on the first side where it hits nothing (above
    /// first for a horizontal two-pin part, else right); Reference and Value at the ends of a packed two-pin part,
    /// the rest hidden; a column shortened by its last rating fields. Text stays upright. The layout follows the drawing.fields settings (line pitch 2 mm, 0.635 mm from
    /// the body, on 0.25 mm steps); a column clear of everything but off that style is placed again, and fields
    /// more than 2 mm from their body too. Bodies, wires, labels, power symbols and notes never move. A dry run by
    /// default; --write edits the .kicad_sch files in place.
    #[command(after_help = PLACE_FIELDS_AFTER_HELP)]
    PlaceFields(PlaceFieldsArgs),
    /// Compare two designs: their placements (--pnp) or their nets (--nets)
    ///
    /// --pnp: A and B are PnP files (KiCad, Altium, Allegro CSV), board files (.kicad_pcb) or KiCad projects with
    /// their board. A board compared with a PnP file is brought into the PnP file's frame first: from --origin, y
    /// up, without the footprints excluded from position files, and with --exclude-dnp and --exclude-from-bom
    /// also without those. The summary line of a side says how many were left out.
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
    /// The parts snapshot: the part facts and footprint reviews the checks read (CFG-9)
    Parts {
        #[command(subcommand)]
        command: PartsCommand,
    },
    /// The display fields of resistors and capacitors, written from the parts snapshot
    Fields {
        #[command(subcommand)]
        command: FieldsCommand,
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
enum FieldsCommand {
    /// Write Voltage, Power, Dielectric and Package from parts.lock.toml: the worst over each part's MPNs
    ///
    /// Voltage and Power state the lowest rating over all MPNs of a part, Dielectric the worst code, Package the
    /// case they share (mW below 1 W, V). A rating text may round at most drawing.fields.rating_rounding_max
    /// percent (default 1). A field equal to the board default (the DEFAULT_C_PACKAGE, DEFAULT_C_VOLTAGE,
    /// DEFAULT_C_DIELECTRIC, DEFAULT_R_PACKAGE, DEFAULT_R_POWER text variables) is taken off the part; a part
    /// without the field and off the default gets one, placed as place-fields places fields. Only these fields
    /// change. An MPN without the fact leaves the field alone; it is listed. A dry run by default.
    #[command(after_help = FIELDS_SYNC_AFTER_HELP)]
    Sync(FieldsSyncArgs),
}

#[derive(clap::Args)]
struct FieldsSyncArgs {
    /// The design: a directory with one .kicad_pro, or the .kicad_pro
    #[arg(value_hint = ValueHint::AnyPath)]
    project: PathBuf,
    /// Edit the .kicad_sch files in place; without it nothing is written
    #[arg(long)]
    write: bool,
    /// A setting over its default, repeatable: `drawing.fields.rating_rounding_max=0.5`, or one for placing new
    /// fields (`drawing.fields.pitch=2.54`)
    #[arg(long = "set", value_name = "NAME=VALUE")]
    set: Vec<String>,
}

#[derive(Subcommand)]
enum PartsCommand {
    /// Write parts.lock.toml again from a part source: the facts of every MPN on the board, the footprint reviews
    ///
    /// Every MPN field (Mpn1, MPN_2, ...) of the parts fitted in the base design or any variant, and each pair of a
    /// board footprint's review hash with the MPNs placed on it. The file goes beside the project, or where the
    /// project file's [parts] lock says; it is sorted, so its git diff shows what changed in the source. The
    /// checks read only this file.
    #[command(after_help = with_part_source(PARTS_REFRESH_AFTER_HELP))]
    Refresh(PartsRefreshArgs),
}

#[derive(clap::Args)]
struct PartsRefreshArgs {
    /// The design: a directory with one design in it, or a design file
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    project: PathBuf,
    /// Take part data from this file, in the shape of parts.lock.toml (hand-filled facts, another board's lock),
    /// instead of the mx3 part store
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    from: Option<PathBuf>,
    #[command(flatten)]
    libraries: LibraryPathArgs,
    /// Write nothing; exit 1 when the snapshot is out of date (for CI)
    #[arg(long)]
    check: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

const PARTS_REFRESH_AFTER_HELP: &str = "\
Exit codes: 0 written or unchanged; 1 out of date (--check); 2 the design, the source or the file failed.

Examples:
  berc parts refresh boards/b153 --from parts.toml
  berc parts refresh --check --from parts.toml";

#[derive(Subcommand)]
enum FpCommand {
    /// Print the review hash of a footprint: a library file, or a footprint on a board by its reference
    ///
    /// The hash (`fp1:` and 16 hex digits) covers what a footprint review checks against the datasheet: pads,
    /// copper, mask, paste, courtyard and board-edge graphics, zones and keepouts, clearance and mask settings.
    /// Silk, fab, texts, the 3D model and the footprint's place on the board don't count, so a board copy has the
    /// hash of its library file. `berc fp accept` records a review of it.
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
    /// Check every footprint of a board against the footprint reviews of its parts snapshot, or every symbol of a
    /// library
    ///
    /// Each board footprint's review hash (as `berc fp hash` prints it) and each MPN of its part (`Mpn1`, `Mpn2`,
    /// ..., in every variant it is fitted in) make a pair; parts.lock.toml (`berc parts refresh`) holds the reviews
    /// covering each pair. Ok when a review of the hash covers every MPN, by the MPN or by its family. A board copy
    /// that differs from its library footprint is reported too (the board was not updated from the library), and a
    /// footprint placed under another Lib:Name than its symbol's Footprint field names.
    ///
    /// With --lib, the symbols of a symbol library instead: each symbol's MPNs against the reviews a part source
    /// (--from) holds for the library footprint its Footprint field names, so a library is checked before any
    /// board uses it.
    #[command(after_help = FP_CHECK_AFTER_HELP)]
    Check(FpCheckArgs),
    /// List what a board's footprints need reviewed: one line per footprint and family or MPN
    ///
    /// Each board footprint hash with the MPNs of the parts placed with it, grouped by what one review covers: a
    /// family (an MPN scheme and its package segments, `yageo.rc case_eia=0402`) or an MPN alone. A review is of
    /// the footprint against the family's datasheet, recorded in the part store, so every board using the
    /// footprint with parts of that family gains from it.
    #[command(after_help = FP_TODO_AFTER_HELP)]
    Todo(FpTodoArgs),
    /// Record a footprint review in the part source and refresh the parts snapshot
    ///
    /// The board footprints given were checked against a datasheet: their review hash now (one for all of them)
    /// is recorded as reviewed for their parts' MPNs, by family or alone, with where the review was made (document,
    /// PDF, page) and who made it. The project's parts.lock.toml is then written again from the source, so
    /// `berc fp check` finds them ok. Nothing is written into the schematic or the libraries, except the symbol's
    /// Footprint with --footprint.
    #[command(after_help = with_part_source(FP_ACCEPT_AFTER_HELP))]
    Accept(FpAcceptArgs),
    /// Read each footprint's 3D models and measure them against the footprint
    ///
    /// Each `(model ...)` entry's file is found as KiCad finds it (`${VAR}` from the environment,
    /// kicad_common.json and a stock KiCad; a `.wrl` tries `.step` and `.stp` beside it), read (STEP only) and
    /// placed as KiCad places it: scale, rotation, offset. Then per model: the file's unit, parts and faces and
    /// what failed to read; the extents in x, y and z; the height above the board and the depth below it; the
    /// middle of the model against the middle of the pads; how far it reaches out of the courtyard; and whether
    /// the leads land on the pads (the model's bottom 0.2 mm seen from below, against each pad's copper).
    ///
    /// Coordinates are KiCad's 3D frame of the footprint, mm: x right, y up (the footprint editor's y negated,
    /// as the model offset is written), z up from the board surface.
    #[command(after_help = FP_MODEL_AFTER_HELP)]
    Model(FpModelArgs),
}

#[derive(clap::Args)]
struct FpModelArgs {
    /// A KiCad project (a directory, its .kicad_pro, .kicad_sch or .kicad_pcb), a footprint library (.pretty) or
    /// one footprint (.kicad_mod)
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    target: PathBuf,
    /// Only these footprints: references on a board, footprint names in a library
    #[arg(add = ArgValueCandidates::new(complete::model_names))]
    names: Vec<String>,
    /// The project directory model paths and KIPRJMOD are resolved against; default the board's directory, or
    /// the directory a library is in
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    project: Option<PathBuf>,
    /// Also print each pad's share under a lead and every warning of the STEP reader
    #[arg(long)]
    details: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

/// Libraries searched before the sym-lib-table and fp-lib-table, and the libraries allowed
#[derive(clap::Args)]
struct LibraryArgs {
    #[command(flatten)]
    paths: LibraryPathArgs,
    /// A library nickname symbols and their footprints may be in, repeatable: an exact name, or a prefix with a
    /// trailing `*` (`VHRD_*`). With --lib or in `berc fp todo`: any other library named is reported. None (the
    /// default): no rule
    #[arg(long, value_name = "NICKNAME")]
    allow: Vec<String>,
}

/// Libraries searched before the sym-lib-table and fp-lib-table, where a board footprint's library footprint is
/// found
#[derive(clap::Args)]
struct LibraryPathArgs {
    /// A symbol library (.kicad_sym, .kicad_symdir) or a folder of them, searched before the sym-lib-table, each
    /// by its file name; repeatable
    #[arg(long, value_name = "DIR", value_hint = ValueHint::AnyPath)]
    symbols: Vec<PathBuf>,
    /// A footprint library (.pretty) or a folder of them, searched before the fp-lib-table, each by its folder
    /// name; repeatable
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    footprints: Vec<PathBuf>,
}

impl LibraryArgs {
    fn search(&self) -> better_erc::fp::LibrarySearch {
        better_erc::fp::LibrarySearch {
            allowed: self.allow.clone(),
            ..self.paths.search()
        }
    }
}

impl LibraryPathArgs {
    fn search(&self) -> better_erc::fp::LibrarySearch {
        better_erc::fp::LibrarySearch {
            symbols: self.symbols.clone(),
            footprints: self.footprints.clone(),
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
    /// Also list the footprints and families reviewed
    #[arg(long)]
    all: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("what").required(true).args(["references", "hash"])))]
struct FpAcceptArgs {
    /// The KiCad project: a directory, its .kicad_pro, .kicad_sch or .kicad_pcb
    #[arg(value_hint = ValueHint::AnyPath, default_value = ".")]
    project: PathBuf,
    /// The board footprints reviewed, by reference: the review is of their library footprint
    #[arg(add = ArgValueCandidates::new(complete::board_references))]
    references: Vec<String>,
    #[command(flatten)]
    libraries: LibraryPathArgs,
    /// Record the review in this file, in the shape of parts.lock.toml, instead of the mx3 part store
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    from: Option<PathBuf>,
    /// A footprint by its review hash instead of board footprints (a library footprint, as `berc fp hash`
    /// prints it); needs --mpn or --family-of
    #[arg(long, value_name = "HASH", requires = "footprint_id")]
    hash: Option<String>,
    /// The `Lib:Name` of the footprint --hash names, for display
    #[arg(long, value_name = "LIB:NAME", requires = "hash")]
    footprint_id: Option<String>,
    /// Cover this MPN alone (one part), repeatable
    #[arg(long, value_name = "MPN")]
    mpn: Vec<String>,
    /// Cover this MPN's family (its scheme and package segments), repeatable
    #[arg(long, value_name = "MPN")]
    family_of: Vec<String>,
    /// Also set the symbol's Footprint to this (`Lib:Name`), in its library and every schematic copy: offered by
    /// `berc fp check` as the placed footprint, when it differs from what the symbol itself names
    #[arg(long, value_name = "LIB:NAME", conflicts_with = "hash")]
    footprint: Option<String>,
    /// The datasheet's document number and revision, `DS30154@Rev16`
    #[arg(long, value_name = "NO@REV")]
    doc: Option<String>,
    /// The datasheet PDF by the document library's content hash: `b3:` and its blake3 hex
    #[arg(long, value_name = "b3:HEX")]
    pdf: Option<String>,
    /// The datasheet page the land pattern is on, 1-based
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    page: Option<u32>,
    /// Who reviewed it; default the login name
    #[arg(long, value_name = "NAME")]
    by: Option<String>,
    /// A note kept with the review
    #[arg(long, value_name = "TEXT")]
    note: Option<String>,
    /// Show what would be recorded, record nothing
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
    /// With --lib: the part source whose reviews the symbols' MPNs are judged against, a file in the shape of
    /// parts.lock.toml
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, requires = "lib")]
    from: Option<PathBuf>,
    #[command(flatten)]
    libraries: LibraryArgs,
    /// Also print the covered MPNs under each footprint or symbol, with the reviews that cover them
    #[arg(long)]
    details: bool,
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
}

#[derive(clap::Args)]
struct WaiveArgs {
    /// The design: a directory or a file
    #[arg(value_hint = ValueHint::AnyPath)]
    project: PathBuf,
    /// A check id, or a full fingerprint
    #[arg(add = ArgValueCandidates::new(complete::catalogue_ids))]
    finding: String,
    /// Subject names that pick the finding among the check's (`R5`, `/SDA`, `part R5`)
    subjects: Vec<String>,
    /// Why the finding is acceptable on this design
    #[arg(long)]
    reason: String,
    /// Who accepts it; default: `git config user.name`
    #[arg(long)]
    who: Option<String>,
    /// The last day it applies (`2027-06-30`)
    #[arg(long, value_name = "DATE")]
    expires: Option<String>,
    /// The variant the finding is in
    #[arg(long, value_name = "V", add = ArgValueCandidates::new(complete::variants))]
    variant: Option<String>,
    /// Waive every finding that matches, not only one
    #[arg(long)]
    all: bool,
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
    /// A setting over its default, repeatable: `drawing.clearance.part=2`, `drawing.clearance.spacing=sparse`,
    /// `I2C-9.min_pull_up=1000`
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
struct PlaceFieldsArgs {
    /// The design: a directory with one .kicad_pro, or the .kicad_pro
    #[arg(value_hint = ValueHint::AnyPath)]
    project: PathBuf,
    /// Only this sheet: its name in the parent sheet or its file
    #[arg(long, value_name = "NAME")]
    sheet: Option<String>,
    /// Edit the .kicad_sch files in place; without it nothing is written
    #[arg(long)]
    write: bool,
    /// A setting over its default, repeatable: `drawing.clearance.text=0.5`, `drawing.clearance.spacing=sparse`,
    /// `drawing.fields.pitch=2.54`
    #[arg(long = "set", value_name = "NAME=VALUE")]
    set: Vec<String>,
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
    /// Leave the board's do-not-populate footprints out, as `kicad-cli pcb export pos --exclude-dnp` does
    #[arg(long)]
    exclude_dnp: bool,
    /// Leave the board's footprints marked exclude_from_bom out, for a fab output without them
    #[arg(long)]
    exclude_from_bom: bool,
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
        Command::Waive(args) => waive(&args),
        Command::Explain { id, format } => explain(&id, format),
        Command::Diff(args) => diff(args),
        Command::PlaceFields(args) => place_fields(&args),
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
        Command::Fp {
            command: FpCommand::Model(args),
        } => fp_model(&args),
        Command::Parts {
            command: PartsCommand::Refresh(args),
        } => parts_refresh(&args),
        Command::Fields {
            command: FieldsCommand::Sync(args),
        } => fields_sync(&args),
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

/// The settings of `--set NAME=VALUE` flags over the defaults
fn settings(sets: &[String]) -> Result<better_erc::Config, String> {
    let mut config = better_erc::Config::default();
    for set in sets {
        let (k, v) = set
            .split_once('=')
            .ok_or_else(|| format!("--set {set}: not NAME=VALUE"))?;
        config.set_text(k, v)?;
    }
    Ok(config)
}

fn place_fields(args: &PlaceFieldsArgs) -> ExitCode {
    let config = match settings(&args.set) {
        Ok(c) => c,
        Err(e) => {
            error(&e);
            return ExitCode::from(CheckError::EXIT_CODE);
        }
    };
    let opts = better_erc::place::PlaceOptions {
        sheet: args.sheet.clone(),
        write: args.write,
        config,
    };
    let spinner = progress::Delayed::start();
    spinner.set("placing fields".into());
    let placed = better_erc::place::place(&args.project, &opts);
    spinner.finish();
    match placed {
        Ok(p) => {
            let out = better_erc::place::render(&p);
            let _ = anstream::stdout().lock().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(CheckError::EXIT_CODE)
        }
    }
}

fn fields_sync(args: &FieldsSyncArgs) -> ExitCode {
    let config = match settings(&args.set) {
        Ok(c) => c,
        Err(e) => {
            error(&e);
            return ExitCode::from(CheckError::EXIT_CODE);
        }
    };
    let opts = better_erc::fields::SyncOptions {
        write: args.write,
        config,
    };
    let spinner = progress::Delayed::start();
    spinner.set("syncing fields".into());
    let synced = better_erc::fields::sync(&args.project, &opts);
    spinner.finish();
    match synced {
        Ok(s) => {
            let out = better_erc::fields::render(&s);
            let _ = anstream::stdout().lock().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(CheckError::EXIT_CODE)
        }
    }
}

fn waive(args: &WaiveArgs) -> ExitCode {
    let expires = match args
        .expires
        .as_deref()
        .map(|e| (e, better_erc::model::Time::parse(e)))
    {
        Some((_, Some(t))) => Some(t),
        Some((e, None)) => {
            error(&format!("--expires {e}: not a date like 2027-06-30"));
            return ExitCode::from(CheckError::EXIT_CODE);
        }
        None => None,
    };
    let who = match args.who.clone().or_else(git_user) {
        Some(w) => w,
        None => {
            error("no --who and no `git config user.name`: say who accepts the finding");
            return ExitCode::from(CheckError::EXIT_CODE);
        }
    };
    let project = match better_erc::Project::open(&args.project) {
        Ok(p) => p,
        Err(e) => return check_error(&e),
    };
    let today = better_erc::checks::project::today();
    let opts = CheckOptions {
        run: RunOptions {
            variant: args.variant.clone(),
            ..RunOptions::default()
        },
        today,
        ..CheckOptions::default()
    };
    let run = match project.check(&opts) {
        Ok(r) => r,
        Err(e) => return check_error(&e),
    };
    let picked = better_erc::checks::project::select(&run.report, &args.finding, &args.subjects);
    let any = !picked.is_empty();
    let picked: Vec<_> = picked.into_iter().filter(|f| !f.waived()).collect();
    match picked.len() {
        0 if any => {
            error(&format!(
                "{} {}: already waived",
                args.finding,
                args.subjects.join(" ")
            ));
            return ExitCode::FAILURE;
        }
        0 => {
            error(&format!(
                "no finding to waive matches {} {}",
                args.finding,
                args.subjects.join(" ")
            ));
            return ExitCode::FAILURE;
        }
        1 => {}
        _ if args.all => {}
        n => {
            let list: Vec<&str> = picked.iter().map(|f| f.fingerprint.as_str()).collect();
            error(&format!(
                "{n} findings match; name more subjects, a fingerprint, or --all:\n  {}",
                list.join("\n  ")
            ));
            return ExitCode::FAILURE;
        }
    }
    for f in picked {
        match project.waive(f, &who, &args.reason, expires, today) {
            Ok((w, path)) => {
                let pinned = w
                    .input_hash
                    .as_deref()
                    .map_or(String::new(), |h| format!(", input {h}"));
                println!("waived {}{pinned} in {}", w.fingerprint, path.display());
            }
            Err(e) => {
                error(&format!("can't write the project file: {e}"));
                return ExitCode::from(CheckError::EXIT_CODE);
            }
        }
    }
    ExitCode::SUCCESS
}

/// `git config user.name`, when git is there and it is set
fn git_user() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()?;
    let name = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (out.status.success() && !name.is_empty()).then_some(name)
}

/// The settings of `--set` and the colour themes of `--theme`
fn check_config(args: &CheckArgs) -> Result<better_erc::Config, String> {
    let mut config = settings(&args.set)?;
    for path in &args.theme {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("--theme {}: {e}", path.display()))?;
        let theme = better_erc::kicad::theme::read(&text)
            .map_err(|e| format!("--theme {}: {e}", path.display()))?;
        config.themes.push(theme);
    }
    Ok(config)
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
    let config = match check_config(&args) {
        Ok(c) => c,
        Err(e) => {
            error(&e);
            return ExitCode::from(CheckError::EXIT_CODE);
        }
    };
    let opts = CheckOptions {
        run: RunOptions {
            variant: args.variant,
            only: args.only,
            config,
            // The project's own, read with it
            parts: None,
        },
        require: args.require,
        fail_on: match args.fail_on {
            FailOn::Error => Severity::Error,
            FailOn::Warning => Severity::Warning,
            FailOn::Suggestion => Severity::Suggestion,
        },
        baseline,
        today: better_erc::checks::project::today(),
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
        exclude_dnp: args.exclude_dnp,
        exclude_from_bom: args.exclude_from_bom,
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
        &run.to_text_color(args.details),
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
    let source = match args
        .from
        .as_deref()
        .map(better_erc::parts::FileSource::open)
    {
        None => None,
        Some(Ok(s)) => Some(s),
        Some(Err(e)) => {
            error(&e.to_string());
            return ExitCode::from(LibCheckError::EXIT_CODE);
        }
    };
    let spinner = progress::Delayed::start();
    spinner.set(format!("reading {}", args.lib.join(", ")));
    let run = better_erc::fp::check_libraries(
        &opts,
        source.as_ref().map(|s| s as &dyn better_erc::parts::Source),
    );
    spinner.finish();
    match run {
        Ok(r) => show_diff(
            args.format,
            &r.to_text_color(args.details),
            &r.to_json(),
            r.is_ok(),
        ),
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

fn fp_model(args: &FpModelArgs) -> ExitCode {
    let spinner = progress::Delayed::start();
    let opts = better_erc::fp::model::ModelOptions::default();
    let run = better_erc::fp::model::run(
        &args.target,
        &args.names,
        args.project.as_deref(),
        &opts,
        &mut |name| spinner.set(format!("reading the models of {name}")),
    );
    spinner.finish();
    match run {
        Ok(r) => show_diff(
            args.format,
            &r.to_text_color(args.details),
            &r.to_json(),
            r.is_ok(),
        ),
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(better_erc::fp::FpError::EXIT_CODE)
        }
    }
}

/// Where `parts refresh` and `fp accept` find the part source, under their --help
fn with_part_source(help: &str) -> String {
    #[cfg(feature = "mx3")]
    let source = format!(
        "Part source: --from FILE, else {}",
        better_erc::parts::mx3::CONNECTION_HELP
    );
    #[cfg(not(feature = "mx3"))]
    let source = "Part source: --from FILE (this berc is built without the mx3 part store, cargo feature mx3)."
        .to_owned();
    format!("{help}\n\n{source}")
}

/// The part source: the file `from` names, else the mx3 part store where this berc is built with it
fn part_source(from: Option<&Path>) -> Result<Box<dyn better_erc::parts::Source>, String> {
    if let Some(from) = from {
        return better_erc::parts::FileSource::open(from)
            .map(|s| Box::new(s) as Box<dyn better_erc::parts::Source>)
            .map_err(|e| e.to_string());
    }
    #[cfg(feature = "mx3")]
    {
        use better_erc::parts::mx3::{Connection, Mx3Source};
        Mx3Source::connect(Connection::from_env())
            .map(|s| Box::new(s) as Box<dyn better_erc::parts::Source>)
            .map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "mx3"))]
    Err(
        "no part source: name a file with --from (this berc is built without the mx3 part store, cargo \
         feature mx3)"
            .to_owned(),
    )
}

fn fp_accept(args: &FpAcceptArgs) -> ExitCode {
    use better_erc::fp::{AcceptError, AcceptWhat, Covers};
    use better_erc::parts::NewReview;
    let fail = |e: &dyn std::fmt::Display| {
        error(&e.to_string());
        ExitCode::from(AcceptError::EXIT_CODE)
    };
    let source = match part_source(args.from.as_deref()) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let what = match (&args.hash, &args.footprint_id) {
        (Some(hash), Some(footprint)) => AcceptWhat::Hash {
            hash: hash.clone(),
            footprint: footprint.clone(),
        },
        _ => AcceptWhat::References(args.references.clone()),
    };
    let covers = Covers {
        alone: args.mpn.clone(),
        family_of: args.family_of.clone(),
    };
    if matches!(what, AcceptWhat::Hash { .. })
        && covers.alone.is_empty()
        && covers.family_of.is_empty()
    {
        return fail(&"--hash needs the MPNs it covers: --mpn or --family-of");
    }
    let (doc, doc_rev) = match args.doc.as_deref().map(|d| d.split_once('@')) {
        Some(Some((no, rev))) => (Some(no.to_owned()), Some(rev.to_owned())),
        Some(None) => (args.doc.clone(), None),
        None => (None, None),
    };
    let review = NewReview {
        doc,
        doc_rev,
        pdf_blake3: args
            .pdf
            .as_deref()
            .map(|p| p.strip_prefix("b3:").unwrap_or(p).to_owned()),
        page: args.page,
        reviewer: args
            .by
            .clone()
            .or_else(|| std::env::var("USER").ok())
            .filter(|b| !b.trim().is_empty())
            .unwrap_or_default(),
        reviewed_on: better_erc::fp::today(),
        note: args.note.clone().unwrap_or_default(),
        ..NewReview::default()
    };
    let project = matches!(what, AcceptWhat::References(_)).then_some(args.project.as_path());
    let spinner = progress::Delayed::start();
    spinner.set(format!("reading {}", args.project.display()));
    let run = better_erc::fp::accept(
        project,
        &args.libraries.search(),
        &what,
        source.as_ref(),
        &review,
        &covers,
        args.dry_run,
    );
    spinner.finish();
    let run = match run {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let mut symbol = Vec::new();
    if let Some(footprint) = &args.footprint {
        match set_footprint(&args.project, &args.references, footprint, args.dry_run) {
            Ok(w) => symbol = w,
            Err(e) => return fail(&e),
        }
    }
    match args.format {
        ListFormat::Text => {
            let mut text = run.to_text_color();
            for w in &symbol {
                let what = match (&w.old, w.changed) {
                    (_, false) => "Footprint already set".to_owned(),
                    (None, true) => "Footprint added".to_owned(),
                    (Some(old), true) => format!("Footprint was {old}"),
                };
                text += &format!("  {}: {what}\n", w.file.display());
            }
            let _ = anstream::stdout().lock().write_all(text.as_bytes());
        }
        ListFormat::Json => {
            let mut v = serde_json::to_value(&run).expect("serialises");
            v["symbol_footprint"] = serde_json::to_value(&symbol).expect("serialises");
            let _ = writeln!(
                std::io::stdout().lock(),
                "{}",
                serde_json::to_string_pretty(&v).expect("serialises")
            );
        }
    }
    ExitCode::SUCCESS
}

/// The symbol of board footprints `refs` given `footprint` as its Footprint, in its library and every schematic
/// copy; refused for references of several symbols
fn set_footprint(
    project: &Path,
    refs: &[String],
    footprint: &str,
    dry_run: bool,
) -> Result<Vec<better_erc::fp::Written>, String> {
    use better_erc::fp::{
        SymbolLibraries, kicad_config_dir, set_symbol_footprint, sheets_and_symbols,
    };
    let (sheets, symbols) = sheets_and_symbols(project, refs).map_err(|e| e.to_string())?;
    let [symbol] = symbols.as_slice() else {
        return Err(format!(
            "--footprint names a Footprint for several symbols ({}): accept one symbol's footprints at a time",
            symbols.join(", ")
        ));
    };
    let dir = if project.is_dir() {
        project.to_path_buf()
    } else {
        project.parent().unwrap_or(Path::new(".")).to_path_buf()
    };
    let mut libs = SymbolLibraries::new(&dir, kicad_config_dir().as_deref());
    set_symbol_footprint(&mut libs, &sheets, symbol, footprint, dry_run).map_err(|e| e.to_string())
}

fn parts_refresh(args: &PartsRefreshArgs) -> ExitCode {
    use better_erc::parts::{PartsRefreshError, WriteLock, refresh_project};
    let source = match part_source(args.from.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            error(&e.to_string());
            return ExitCode::from(PartsRefreshError::EXIT_CODE);
        }
    };
    let write = if args.check {
        WriteLock::Never
    } else {
        WriteLock::Changed
    };
    match refresh_project(
        &args.project,
        &args.libraries.search(),
        source.as_ref(),
        write,
    ) {
        Ok(r) => {
            let out = match args.format {
                ListFormat::Text => r.to_text(),
                ListFormat::Json => r.to_json() + "\n",
            };
            let _ = std::io::stdout().lock().write_all(out.as_bytes());
            if args.check && r.changed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            error(&e.to_string());
            ExitCode::from(PartsRefreshError::EXIT_CODE)
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
