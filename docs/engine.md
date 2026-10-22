# The check engine

`erc_checks` runs the checks of the [catalogue](checks.md) on a design of the [board model](model.md). Each check is
one file with its catalogue id, the data it needs and a pure function from the design to findings; the engine works
out which checks can run on a design, runs them in parallel, says which could not run and what would enable them,
and returns one report. `erc_analysis` holds what the checks build on: part classes, values, graph queries and the
I2C bus recogniser.

## A run

```rust
use erc_checks::{RunOptions, run_imported};
use erc_model::{DirFiles, ImportOptions};

let imported = erc_kicad::project::read(&DirFiles("board".into()), "board.kicad_pro", &ImportOptions::default())?;
let report = run_imported(&imported, &RunOptions::default())?;
println!("{}", report.to_text());      // or report.to_json()
```

`RunOptions` picks a variant by name (`DNP` and field overrides of KiCad 10 variants apply), restricts the run to
some check ids or categories (`I2C-3`, `BOM`), and carries the project's [`Config`](#thresholds-and-overrides). A run
goes board by board:

1. **Availability.** For every [need](#needs) of the checks, the engine works out what this board has: available,
   partial (known for some parts: the checks run on those and the report says how many), or missing (with why).
2. **Checks.** Every check whose needs are all there runs, in parallel (rayon), on one shared context (`Ctx`). The
   context builds an analysis the first time a check asks for it (part classes, the I2C buses) and keeps it, so a
   check pays only for what it uses and nothing is computed twice.
3. **Not run.** A check with a missing need is reported as `NotRun { check, need, hint }`: the hint says why the
   data is missing and what would provide it, followed by what the check's detail page says it needs.
4. **Overlaps.** A check that overlaps another (the catalogue's `≈`) supersedes that one's finding on the same
   subjects: the other finding is dropped and named in `also` (`BOM-6 ... (also BOM-5)`).
5. **Order.** Findings worst first, then in catalogue order (category, number).

## Findings

One type for every check:

| field | what |
|---|---|
| `check` | the catalogue id (`I2C-1`) |
| `severity` | `error`, `warning`, `suggestion` (the catalogue's 🛑 ⚠️ 💡) or `info`; the project can override it |
| `confidence` | `certain`; `estimate` when it rests on an assumed fact (a part class taken from the designator prefix); `candidate` for the judgement checks, a list for a human or an LLM, never counted as an error |
| `board`, `subjects` | the board, and the nets, parts, pins, sheets, labels or pads involved, each with its id and display name (`net /SDA`, `part R3`) |
| `variant`, `state` | the variant checked; the power state (once power states exist) |
| `message`, `fix` | what is wrong on this design, and how to fix it |
| `facts` | the facts it rests on, each with its source (`R601 = 3 kΩ` from `i2c-10_non_equal_pull_ups.kicad_sch`, a threshold from the check's default or the project file) |
| `fingerprint` | the check id and the sorted subject names (`I2C-10:net /SCL\|net /SDA\|part R601\|part R602`): what waivers and baselines key on |
| `explain` | the detail page section (`docs/checks/i2c.md#i2c-10`) |
| `locations` | the sheet files and positions of the parts and labels involved |
| `also` | the overlapping checks this finding folded in |

## A run on a path

What `berc check` does, and what the MCP server and the GUI reuse: `erc_checks::project` (cargo feature `import`,
on by default) opens a path through the importer registry, runs the engine, compares with a baseline and decides
the exit code as data.

```rust
use erc_checks::{CheckOptions, Format, Need, Severity, check_path, read_baseline};

let opts = CheckOptions {
    require: vec![Need::Layout],
    fail_on: Severity::Warning,
    baseline: Some(read_baseline("berc-baseline.json".as_ref())?),
    ..CheckOptions::default()           // run: RunOptions { variant, only, config }
};
let run = check_path("hw/board".as_ref(), &opts, &mut |stage| eprintln!("{stage:?}"))?;
print!("{}", run.render(Format::Sarif, false));
std::process::exit(run.verdict.exit.code().into());
```

- `Project::open(path)`: a design file (whatever an importer reads), or a directory: its `.kicad_pro` when there is
  exactly one, else the one file an importer is surest about; several equally likely files are an error naming
  them. `Project::variants()` lists the variants (shell completion uses it), `Project::check(&opts)` runs.
- `CheckRun { dir, main, importer, report, verdict }`; `render(format, color)` gives text (coloured on request),
  JSON, SARIF (file URIs relative to the current directory) or Markdown.
- `Verdict { exit, fail_on, failing, unavailable }`: `Exit::Unavailable` (code 3) when a required need is missing
  on a board, else `Exit::Findings` (1) when findings at or above `fail_on` exist (with a baseline: new ones only;
  candidates never count), else `Exit::Clean` (0).
- `CheckError`: the path does not exist, holds no design or several, the import failed, the run options do not
  fit the design (unknown variant or check), the baseline cannot be read or has another schema; all exit code 2.

## The report

`Report { schema, tool, summary, checks, findings, candidates, notes, timing, baseline }`:

- `summary`: each board (tool, sheets, parts, nets, variants, layout or not), the counts per severity, and how many
  checks ran, did not run and are planned;
- `checks`: every catalogue check with its status: `ran` (finding count, time), `not_run` (need and hint) or
  `planned` (in the catalogue, no code yet);
- `notes`: import notes (what the importer read past), analysis notes (an I2C part in no known role) and coverage
  notes (a need known for only some parts).

- `baseline`: after `compare` with a previous report, the delta: how many findings are new and unchanged, and the
  baseline's findings that are gone (`fixed`); each finding carries `baseline: new | unchanged`. Findings are
  matched by board and fingerprint, a fingerprint found twice in the baseline matches two findings, candidates are
  not compared.

`to_json` writes it with a versioned schema (`schema: 1`, `erc_checks::report::SCHEMA_VERSION`; `baseline` and a
location's `line` are optional fields, absent when empty), `from_json` reads it back. The renderings:

- `to_text` (`to_text_color` with ANSI colours): findings worst first, grouped by category, new ones marked, then
  candidates, the fixed findings, the checks that did not run with what would enable each, the notes and a summary
  line;
- `to_markdown`: a headline with the counts, the baseline delta, a table of the findings, and the fixed findings,
  candidates, not-run checks and notes folded in `<details>`, for a pull request comment;
- `to_sarif(uri_base, main)`: SARIF 2.1.0 with our own small structs (`report::sarif`): one run, a rule per check
  that ran or found something (id = catalogue id, short description = its title, help = its detail page section,
  default level from the catalogue's severity), a result per finding with level error → `error`, warning →
  `warning`, suggestion and info → `note` (a candidate is `kind: review`, level `none`), located at the schematic
  file and line of its symbols, or the label files of its nets, or the design file alone; a partial fingerprint
  `berc/v1` (board and fingerprint) so code scanning tracks a finding across commits; `baselineState` after a
  compare; the checks that did not run as notifications.

A symbol's line comes from `erc_kicad`: the s-expression reader borrows its atoms from the file text, so
`sexpr::Lines` finds the line of any expression without the parser tracking positions, and `UnitPlacement::line`
keeps it in the model.

## The command line

`berc` (the `berc` crate) is a thin client of the library; nothing it does is missing there.

- `berc check PROJECT [--variant V] [--only CAT,ID] [--format text|json|sarif|md] [--baseline report.json]
  [--fail-on error|warning|suggestion] [--require NEED]`: `check_path` and `render`. Exit codes 0 clean, 1 findings
  at or above `--fail-on` (default `error`; with `--baseline` new ones only), 2 usage or import error, 3 a required
  need unavailable (before 1). `--require` takes the needs by slug, repeated or comma-separated (`graph`,
  `part-classes`, `values`, `rails`, `rail-voltages`, `pin-roles`, `pin-thresholds`, `pin-functions`,
  `power-states`, `conduction`, `layout`, `drawing`, `part-db`, `multi-board`; `project:KEY` for a project key a
  check declares). `--state S` is reserved for power states (NET-8) and refused with exit code 2.
- `berc explain ID [--format text|md]`: `explain(id)`, the catalogue line and detail page section of any catalogue
  check, implemented or planned, as plain text or Markdown.
- `berc checks list [--tier easy|mid|hard|judgement] [--status ran|not-run|planned] [--project PATH]
  [--format text|json]`: `list_for(report)`, every catalogue check with severity, tier and state; with `--project`
  what a run there does with each (ran, not run with the hint, planned), which `--status ran` and `not-run` need.
- `berc textconv FILE`: readable git diffs of KiCad files (README "Readable git diffs").

Text goes through `anstream`, so colour is dropped on a pipe and with `NO_COLOR`; a spinner on stderr shows what a
run is doing once it has taken two seconds, never when stderr is not a terminal. Shell completion (`COMPLETE=bash
berc`, `zsh`, `fish`) asks berc itself and offers check ids and categories from the embedded catalogue, the needs,
and the variants of the design in the current directory, each with its help text. `berc --version` names the
version, git commit and commit time.

## Needs

`Need` is the data contract between a check and the design. What the engine can tell today:

| need | available when |
|---|---|
| `Graph` | the board has parts |
| `PartClasses` | always: by library symbol, by description, else by designator prefix (marked assumed) |
| `Values` | components have a Value field (partial when only some do) |
| `Layout` | the board file was read |
| `Drawing` | the design came from KiCad schematics (labels, no-connect flags), not a netlist |
| `MultiBoard` | the design joins boards through connectors |
| `Rails`, `RailVoltages`, `PinRoles`, `PinThresholds`, `PinFunctions`, `PowerStates`, `Conduction`, `PartDb`, `Project(key)` | not yet: the analyses and the project file are still to come, and the hint names the FEATURES.md id |

## The catalogue in the binary

`docs/checks.md` and the detail pages under `docs/checks/` are embedded at build time, so these work offline:

- `erc_checks::list()`: every catalogue check with its severity, tier, needs and state, `implemented` or `planned`;
- `erc_checks::explain(id)`: the catalogue line and the detail page section of any check;
- the not-run hints.

The catalogue is the source of truth for ids, severities and tiers: `erc_checks/tests/catalogue.rs` parses it and
fails when a registered check's id, category, severity or tier differs from its line, when its detail page section
or `needs:` line is missing, or when it declares an overlap no catalogue line marks with `≈`.

## Test schematics and expectations

Every schematic in `test_schematics/sources` is named after the check it is for, `<id>_<slug>.kicad_sch`
(`i2c-7_segments.kicad_sch`), holds failing cases next to correct ones that must not fire, and has a sidecar
`<id>_<slug>.expect.toml` with everything the engine must report on it:

```toml
[[finding]]
check = "BOM-6"
subjects = ["part R3"]
also = ["BOM-5"]          # optional
confidence = "estimate"   # optional, "certain" by default

[[not_run]]
check = "LAY-1"
need = "layout"
```

`erc_checks/tests/schematics.rs` imports each schematic with `erc_kicad`, runs every registered check and compares
findings and not-run checks with the sidecar. It fails on a difference, on a schematic without a sidecar (or the
other way round), on a name that does not start with a catalogue id, and on a registered check that no schematic
shows firing. `BLESS=1 cargo test -p erc_checks --test schematics` writes the sidecars from what the engine finds,
for review in the diff. `just netlists` exports the SVG pictures of the schematics.

## Thresholds and overrides

A check's thresholds are `Param`s in its `CheckMeta` with a default and a unit (`I2C-7` `max_tie` = 100 Ω, `I2C-9`
`min_pull_up` and `max_pull_up`). `Config` is what a project changes: a severity per check, a value per
`CHECK-ID.param`, checks to leave out. The better_erc project file (CFG-2) will fill it; a finding names the source
of the threshold it used.

## Adding a check

1. The catalogue line in [checks.md](checks.md) and its section on the category's detail page, with the `needs:`
   line (the not-run hint quotes it).
2. `erc_checks/src/<category>/<id>.rs` (`i2c/i2c_3.rs`): a `static META: CheckMeta` with the catalogue's id,
   category, severity and tier, the needs, overlaps and params; `impl Check` whose `run` builds findings with
   `cx.finding(&META).net(..).part(..).message(..).fact(..).fix(..).build()`; `inventory::submit!(Registered(&..))`;
   a unit test on its schematic (`crate::testing::fingerprints`). Add the module to the category's `mod.rs`.
3. A missing analysis goes into `erc_analysis` (with its test) and a lazy accessor on `Ctx`; a missing accessor of
   the model goes into `erc_model` (with its test).
4. `test_schematics/sources/<id>_<slug>.kicad_sch` with the failing and the correct case, and its
   `.expect.toml`; `just netlists` for the picture.
5. FEATURES.md and CHANGELOG.

## The checks that run

| check | what it finds | schematic |
|---|---|---|
| `I2C-1` | a direct bus segment without a pull-up pair | `i2c-1_no_pull_ups` |
| `I2C-2` | resistors from a line to the pull-up rail besides the pair; several pairs on one segment | `i2c-2_multiple_pull_ups`, `i2c-7_segments` |
| `I2C-3` | a pull-up pair whose rail has nothing else on it, or a resistor to nowhere | `i2c-1_no_pull_ups` |
| `I2C-7` | a tie between two buses over `max_tie` | `i2c-7_segments` |
| `I2C-9` | a pull-up outside the 2.2k-10k window (its ends inside), or whose value does not read | `i2c-9_non_standard_pull_ups` |
| `I2C-10` | SCL and SDA pull-ups of different value | `i2c-10_non_equal_pull_ups` |
| `BOM-5` | a placeholder value (`?`, `??k`, `TBD`, `TODO`) | `bom-10_values` |
| `BOM-6` | a placeholder value on a fitted part (folds `BOM-5` in) | `bom-10_values` |
| `BOM-9` | a component without a value | `bom-10_values` |
| `BOM-10` | a resistor value that does not read, or odd notation | `bom-10_values` |
| `LIB-3` | a MOSFET whose pins are not named G, D and S | `lib-3_mosfet_pin_names` |

The I2C buses come from `erc_analysis::i2c`, ported from the first `erc_core` code: SCL and SDA nets named alike,
pull-up pairs meeting on one net, ties and discrete translators (a resistor or FET per line to another bus, or to an
unnamed pair of nets that becomes a bus of its own), translator ICs, and the segments they form. Pin roles will
replace the names as the way to find a bus (NET-4, NET-7).
