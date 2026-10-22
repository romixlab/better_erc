# The check engine

`erc_checks` runs the checks of the [catalogue](checks.md) on a design of the [board model](model.md). Each check is
one file with its catalogue id, the data it needs and a pure function from the design to findings; the engine works
out which checks can run on a design, runs them in parallel, says which could not run and what would enable them,
and returns one report. `erc_analysis` holds what the checks build on ([The analyses](#the-analyses)): part classes,
values, rails and grounds, the I2C bus recogniser, drawing facts and graph queries, each built once per board and
variant in the check context. The library crate `better_erc` re-exports all of it (see [model.md](model.md)
"The crates").

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
   context builds an analysis the first time a check (or the availability of step 1) asks for it and keeps it, so
   a check pays only for what it uses and nothing is computed twice.
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
use better_erc::{CheckOptions, Format, Need, Severity, check_path, read_baseline};

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
  them. `Project::open_files(main, others)`: a design whose files were picked one by one (the app's Altium and
  OrCAD pickers), read as if beside each other. `Project::variants()` lists the variants (shell completion uses
  it), `Project::check(&opts)` runs.
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

`berc` (the `berc` crate) is a thin client of the library (`better_erc`); nothing it does is missing there.

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
- `berc diff --pnp A B [--origin page|aux] [--tolerance MM] [--rotation-tolerance DEG] [--format text|json]`:
  `better_erc::diff::pnp`, the placements of two designs (PnP files, boards, KiCad projects with their board)
  footprint by footprint; a board against a PnP file in the PnP frame ([model.md](model.md)). Exit codes 0 the
  same, 1 different, 2 usage or import error.
- `berc diff --nets A B [--format text|json]`: `better_erc::diff::nets`, the nets of any two designs with
  connectivity (a KiCad project or schematic, a netlist, Altium EDIF or WireList, OrCAD, a Smart PDF), each net as
  the set of pins it connects (`erc_model::netcmp`): nets match by their pins whatever their names, and the report
  lists renamed, removed and added nets and net class changes. It is the differential between a Smart PDF and an
  Altium export of the same board, and the first piece of RPT-16 (between two git refs is still to come). Same exit
  codes. The BOM and findings modes come with RPT-17.
- `berc textconv FILE`: readable git diffs of KiCad files (README "Readable git diffs").

Text goes through `anstream`, so colour is dropped on a pipe and with `NO_COLOR`; a spinner on stderr shows what a
run is doing once it has taken two seconds, never when stderr is not a terminal. Shell completion (`COMPLETE=bash
berc`, `zsh`, `fish`) asks berc itself and offers check ids and categories from the embedded catalogue, the needs,
and the variants of the design in the current directory, each with its help text. `berc --version` names the
version, git commit and commit time.

## The analyses

`Ctx` holds the design, the board, the variant checked and the project's `Config`, and builds each analysis of
`erc_analysis` on first use (a `OnceLock` each, shared read-only by the checks running in parallel). Every derived fact
is a `Fact` with `Provenance::Derived { rule }` (or `Assumed` for a guess), so a finding can name the rule it rests on.

| accessor | analysis | what it holds | need |
|---|---|---|---|
| `cx.view()` | `erc_model::View` | the board as the variant checked mounts it: `fitted`, `flag`, `in_bom`, `field`, `field_fact`, `value_text`, `fitted_components` | `Variant` |
| `cx.classes()` | `class::PartClasses` | each part's `PartClass` (architecture §1.3: resistor, polarized capacitor, Schottky diode, N-channel MOSFET, USB-C connector, buck regulator, pass-FET translator, ...), from a `ClassSource` (the part database's hook), the library symbol, its description and keywords, net-tie pad groups, else the designator prefix (assumed); `coverage` counts them | `PartClasses` |
| `cx.values()` | `value::Values` | each part's value ([Values](#values)) in the variant checked, read in the unit its class gives it (`PartClass::value_unit`: Ω, F, H, Hz, A, V for Zener and TVS), with tolerance, ratings (`50V`, `1/4W`), dielectric, the smallest E-series it is in (resistors, capacitors, inductors), the parse warnings, the field it is written in and its source | `Values` |
| `cx.power()` | `power::Power` | rails (`Rail`: kind, the signals that make it one, the voltage per source), grounds (`Ground`: kind, signals), ground groups joined at DC, every part linking two grounds, switching nodes | `Rails`, `RailVoltages` |
| `cx.i2c()` | `i2c::I2cBuses` | the I2C buses of the fitted parts: pull-up pairs, devices, ties, translators, discrete translators with their gate nets, direct and logical segments, `pull_ups(segment)` | `Graph`, `PartClasses` |
| `cx.drawing()` | `drawing::Drawing` | every label's scope (sheet, global, hierarchical, sheet pin, directive), labels by text, sheet pins matched with hierarchical labels and the unmatched ones, no-connect flags with their pin, `dangling`, `flagged_but_connected` | `Drawing` |
| `access::reach(view, classes, power, net)` | `access` | where a probe reaches a net: test point and connector pins on it or one series resistor, ferrite, net tie or jumper away (a far side on a rail or ground is a pull, not a path); `zero_ohm_groups`, the nets joined by 0 Ω resistors, net ties and jumpers | `Graph`, `PartClasses` |
| `supply::power_in_pins(view, net)`, `caps_to_ground` | `supply` | the power-input pins of the fitted parts on a net, by part; the fitted capacitors from a net to a ground | `Rails` |
| `pin_name::functions(name)` | `pin_name` | what a pin name states: SWD, JTAG, SWO, reset, UART TX/RX/RTS/CTS, SPI SCK/MOSI/MISO/CS, I2C, enable, interrupt, power good, fault, with the peripheral instance (`USART1` in `PA9/USART1_TX`) | `Graph` |

**Part classes.** A library symbol decides by its library and name (`Device:R`, `Transistor_FET:*`,
`Regulator_Linear:*`, `MCU_*`, `74xGxx:*`, `Connector:USB_C_*`); a kind the name leaves open is taken from the
description (`P-Channel`, `Schottky`, `bidirectional TVS`, `step-down`). A symbol of an unknown library is classified
by its description and keywords, matched as whole words in order (a `gate driver` is no MOSFET, a `load switch` no
switch, `isolated` is no isolator). The designator prefix is the last resort, an assumption a finding reports as an
estimate.

**Rails and grounds** (`power`, NET-5 first layer). A net name is read by a grammar (`read_name`): voltage tokens
`+3V3`, `3V3`, `-5V`, `+5V`, `12V0`, `3.3V`, `3P3V`; power words `VDD*`, `VCC*`, `VBUS`, `VBAT`, `VIN`, `VSYS`,
`VREF`, `VDDA`, `AVDD`, `+V...`, `V+`, `V-`, `VEE` (a negative rail, not a ground); ground words `GND*`, `AGND`,
`DGND`, `PGND`, `VSS*`, `GND_ISO`, `CHASSIS`, `SHIELD`, `EARTH`, `PE`, `0V`; and signal words (`EN`, `PG`, `SENSE`,
`FB`, `SCL`, `TX`, ...) that make a name a signal whatever voltage it mentions (`VDD_EN`, `SCL1_3V3`). The sheet path
is left out and auto-named nets have no name to read. A net is a rail by its name, a power symbol on it (not
`PWR_FLAG`), or power pins of components on it; a ground by its name or by pins named like a ground. `NetSource` is
the hook for the project file (CFG-2), whose declarations rank first. A rail's voltage is a `Facts<f64>`: the name's
reading (`Derived`, "voltage from the net name"; a name with a `NEG` marker and no sign reads negative and its rule
says so), the project's when there is one. `PowerOptions::strict` reads voltages from `+xVy` only, as the first
better_erc did (NAME-7). Grounds joined by a fitted net tie, jumper, 0 Ω resistor, ferrite or inductor form one
`GroundGroup`; every fitted part on two grounds is a `GroundLink` with its `Joint` (an R or C to chassis, an IC with
AGND and DGND pins). Switching nodes (the net between an IC and an inductor that no name makes a rail) are neither
rail nor ground.

**I2C** (`i2c`, NET-4). A bus is a pair of nets named alike with `SCL` and `SDA`; its pull-up a pair of resistors
meeting on one net; a resistor, ferrite, net tie or jumper per line to another bus a tie (one direct segment), a FET
per line a discrete translator (the BSS138 pair, with the nets of its gates), a translator IC with another bus on its
other side a translator (one logical bus, several direct segments). Only the fitted parts of the variant count, so a
pull-up made DNP is no pull-up. Pin roles will add pins named `SCL`/`SDA` as a source (NET-7).

**Drawing facts** (`drawing`): what the WIRE, NAME and SHORT checks need beyond connectivity. Labels keep their kind
(`local`, `global`, `hierarchical`, `sheet_pin`, `power`, `local_power` for KiCad 10 local power symbols,
`directive`), sheet, position, net and whether they dangle; the analysis adds the scope and the matching.

**Pin functions by name** (`pin_name`, the name layer of pin roles, NET-7): a pin name is split into alternates at
`/` (`PA13/JTMS-SWDIO`) and words at `_`, `-` and spaces; each word is matched whole (`SWDIO`, `TRACESWO`, `NRST`,
`TXD`, `MOSI`, `NSS`, `EN`, `SHDN`, `ALERT`, `PGOOD`), past an overbar (`~{RESET}`), a leading `n` or trailing `N`
(active low) and a number (`EN2`, `INT1`). A pin named by its port alone (`PA13`) says nothing until alternate
functions are known (NET-12); the DBG and TEST checks do not judge a part whose names state no debug port or UART.

The analyses have their own test schematics in `test_schematics/analyses` (`power`, `parts` with a KiCad 10 variant,
`drawing` with a sub-sheet), each with a sidecar `<name>.expect.toml` of what they find, one line per fact
(`erc_analysis/tests/analyses.rs`; `BLESS=1` rewrites them).

## Needs

`Need` is the data contract between a check and the design. The engine asks the context, so availability comes from
the same analyses the checks then use:

| need | available when |
|---|---|
| `Graph` | the board has parts |
| `PartClasses` | always; partial (with a coverage note) when some components are classified only by their designator prefix (assumed) or not at all |
| `Values` | components have a value (partial when only some do); see [Values](#values) |
| `Rails` | some net reads as a rail or a ground |
| `RailVoltages` | rails carry a voltage (partial when only some do; missing when none does) |
| `Variant` | the design states what is fitted: a KiCad schematic or netlist, variants, or DNP flags |
| `Layout` | the board file was read |
| `Drawing` | the design came from KiCad schematics (labels, no-connect flags), not a netlist or a Smart PDF |
| `MultiBoard` | the design joins boards through connectors |
| `PinRoles`, `PinThresholds`, `PinFunctions`, `PowerStates`, `Conduction`, `PartDb`, `Project(key)` | not yet: the analyses and the project file are still to come, and the hint names the FEATURES.md id |

## Values

Checks read a part's value from the model (`Part::value`), never a tool's field by its name. The importer fills it by
the tool's convention, as a fact whose note names the field: KiCad's `Value`; Altium's `Value`, else `Comment` (where
Altium designers put an IC's part number), in the EDIF and WireList import and the Smart PDF alike. The field itself
stays as the tool wrote it. `erc_analysis::value::value_text` gives the text a value was read from (a KiCad variant's
override of `Value` first, then the field of the value's source whose text reads as that value), so a finding can
name it: the fact is `Value`, or `Value (from Comment)`.

## Certainty

A finding is `certain` when every fact it rests on comes from the design or a stated source, an `estimate` when one
is assumed (a default, a designator prefix) or reconstructed (`Provenance::is_estimate`): a Smart PDF's property
bound to its part by where the popup is drawn (`PdfConfidence::Position`), or anything on a board whose nets were
traced from a drawing (`SourceInfo::connectivity` with `PdfConfidence::Geometry`). A Smart PDF's netlist, read from
its bookmarks, is as certain as a netlist file.

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
3. A missing analysis goes into `erc_analysis` (with its test, and its test schematic in `test_schematics/analyses`
   when it is observable there), a lazy accessor on `Ctx` and, when it is new data, a `Need` with its availability;
   a missing accessor of the model goes into `erc_model` (with its test).
4. `test_schematics/sources/<id>_<slug>.kicad_sch` with the failing and the correct case, and its
   `.expect.toml`; `just netlists` for the picture.
5. FEATURES.md and CHANGELOG.

## The checks that run

| check | what it finds | schematic |
|---|---|---|
| `I2C-1` | a direct bus segment without a pull-up pair (a segment behind a translator too) | `i2c-1_no_pull_ups`, `i2c-18_discrete_translator` |
| `I2C-2` | resistors from a line to the pull-up rail besides the pair; several pairs on one segment | `i2c-2_multiple_pull_ups`, `i2c-7_segments` |
| `I2C-3` | a pull-up pair whose rail has nothing else on it, or a resistor to nowhere | `i2c-1_no_pull_ups` |
| `I2C-7` | a tie between two buses over `max_tie` | `i2c-7_segments` |
| `I2C-9` | a pull-up outside the 2.2k-10k window (its ends inside), or whose value does not read | `i2c-9_non_standard_pull_ups` |
| `I2C-10` | SCL and SDA pull-ups of different value | `i2c-10_non_equal_pull_ups` |
| `BOM-5` | a placeholder value (`?`, `??k`, `TBD`, `TODO`) | `bom-10_values` |
| `BOM-6` | a placeholder value on a fitted part (folds `BOM-5` in) | `bom-10_values` |
| `BOM-9` | a component without a value | `bom-10_values` |
| `BOM-10` | a resistor, capacitor or inductor value that does not read in its unit, odd notation, or a value in no E-series | `bom-10_values` |
| `LIB-3` | a MOSFET whose pins are not named G, D and S | `lib-3_mosfet_pin_names` |
| `DEC-1` | a rail feeding power pins with no capacitor to a ground | `dec-1_no_decoupling` |
| `DEC-4` | a ferrite between two rails, with the LC resonance against the capacitance behind it (`bead_inductance` assumed) | `dec-4_ferrite_beads` |
| `DEC-7` | fewer capacitors of at most `max_decoupling` (1 µF) on a rail than a part has power pins there, the rail's capacitors handed out to the parts with the fewest pins first | `dec-7_caps_per_pin` |
| `DEC-8` | a series ferrite, inductor or resistor between two rails with the capacitors only on the side without the power pins | `dec-8_source_side_caps` |
| `DEC-10` | a net of power pins and capacitors to ground only | `dec-10_unsourced_bypass` |
| `DEC-11` | neighbouring ceramic values on a rail `max_ratio` (10×) apart or more | `dec-11_decade_apart` |
| `DEC-12` | a polarized capacitor with `+` on the lower voltage | `dec-12_reversed_polarized` |
| `DBG-1` | an MCU, SoC or FPGA whose SWD or JTAG pins reach no connector or test point, or no ground does | `dbg-1_swd_not_connected` |
| `DBG-2` | SWD or JTAG pins on the same net as a header pin | `dbg-2_swd_series_resistors` |
| `DBG-3` | a board with an MCU, SoC or FPGA and no LED | `dbg-3_no_leds` |
| `DBG-4` | a rail with no LED to a ground | `dbg-4_power_leds` |
| `DBG-7` | an enable pin without resistor footprints to both levels | `dbg-7_enable_pulls` |
| `DBG-8` | an open-drain output with no pull-up footprint | `dbg-8_open_drain_pull_ups` |
| `DBG-10` | a rail feeding more than `max_parts` (5) parts directly | `dbg-10_crowded_rail` |
| `DBG-11` | a UART or SPI data line meeting another part or a connector with no series resistor | `dbg-11_uart_spi_resistors` |
| `DBG-14` | no UART TX/RX pair of an MCU reaching a header or test points | `dbg-14_uart_console` |
| `DBG-16` | LEDs, none driven by an MCU pin (folds `DBG-3` in) | `dbg-16_heartbeat_led` |
| `DBG-17` | a header with SWDIO and SWCLK but not the MCU's reset or SWO | `dbg-17_swd_without_reset` |
| `DBG-18` | an I2C or SPI bus with lines no analyser can reach | `dbg-18_bus_access` |
| `TEST-1` | per IC, the pins that connect to nothing (a report table) | `test-1_unconnected_pins` |
| `TEST-2` | several test points on one net or 0 Ω group and board side | `test-2_duplicate_test_points` |
| `TEST-4` | a rail or ground with no test point | `test-4_rail_test_points` |
| `TEST-5` | a bring-up line (UART, SWD, I2C, SPI) with no test point or header pin | `test-5_bring_up_test_points` |

The I2C buses come from `erc_analysis::i2c` ([The analyses](#the-analyses)), ported from the first analysis code (the
removed `erc_core`). Pin roles will replace the names as the way to find a bus (NET-4, NET-7).
