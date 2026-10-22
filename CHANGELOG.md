# Changelog

All notable changes to better_erc are recorded here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). [FEATURES.md](FEATURES.md) holds the current
status of every feature; this file only records what changed. IDs in parentheses (`IMP-4`, `I2C-3`) refer to
FEATURES.md and the check catalogue.

All crates share one version (`[workspace.package]`). Every commit with real work bumps it (minor for features,
patch for fixes) and moves `[Unreleased]` under the new version. Nothing has been tagged yet. History up to
0.1.0 is grouped by month; it was backfilled on 6 Oct 2026 from the git history.

## [Unreleased]

### Added

- `erc_analysis::names`: the name grammar beyond power, for the NAME, WIRE and SHORT checks: active-low markers with
  their style (`~{X}`, `~X`, `X_N`, `X_n`, `nX`/`NRST`, `X#`, `Xn`), differential halves (`_P`/`_N`, `+`/`-`,
  `DP`/`DM`, `_H`/`_L`, glued `XP`/`XN` where both exist) and `pair_halves`, numbered families (`IO4`), the GPIO a pin
  name gives (`PA5`, `GPIO17`, `P0.12`), UART TX/RX pins and net-name words, and `normalised` names (WIRE-2).
- `erc_model::Sheet::rev`: the title block revision, read by the schematic and netlist importers and kept in the
  canonical dump (`rev A`).

## [0.4.4] - 2026-10-22

### Added

- Stage 1 easy checks, batch checks-dec: `DEC-1`, `DEC-4`, `DEC-7`, `DEC-8`, `DEC-10`, `DEC-11`, `DEC-12`, `DBG-1`, `DBG-2`, `DBG-3`, `DBG-4`, `DBG-7`, `DBG-8`, `DBG-10`, `DBG-11`, `DBG-14`, `DBG-16`, `DBG-17`, `DBG-18`, `TEST-1`, `TEST-2`, `TEST-4`, `TEST-5`, each with its test schematic and sidecar; `erc_analysis::{pin_name, access, supply}` (`NET-22`), `PartClass::is_programmable`.

### Changed

- Test schematic sidecars: findings compared exactly, `[[not_run]]` entries are hand-written assertions compared as a subset; `BLESS=1` writes findings only and keeps the not-run blocks; the generated not-run blocks are gone from all sidecars (`DIAG-6`).

## [0.4.3] - 2026-10-22

### Added

- The schematic PDF importer (`IMP-16`, stage 1), cargo feature `pdf` of `erc_import`: an Altium Smart PDF read into
  a board (tool `altium-pdf`) from its bookmarks: sheets, parts with their units, pins, nets, net labels and ports,
  nets joined across sheets by power names and same-named ports (Altium names nets per sheet), part properties
  from the popup scripts bound to their part by the hidden search words inside the link. Parsed with `hayro-syntax`
  (pure Rust, no unsafe of its own); every walk bounded (file size, bookmarks, nesting, links, action chains,
  script length, content operations, form drawings, words), scripts read as text, never run; owner-password files
  read, a user password is an error. `berc check board.pdf` works; tests on a synthetic Smart PDF from our own
  generator, its encrypted copies, hostile files, and the internal set of Altium PDFs when present (the Xyloni
  against its hand-made netlist).
- `erc_model`: `Provenance::Pdf { file, confidence }` with `PdfConfidence::{Bookmarks, Position, Geometry}`,
  `Rank::RECONSTRUCTED` (750), `Provenance::is_estimate`, `SourceInfo::connectivity`.
- `berc diff --nets A B` (`RPT-16`, first piece): the nets of two designs as pin sets, renamed, removed and added
  nets and net class changes, text or JSON, exit 1 on a difference; `better_erc::diff::nets`. The differential
  between a Smart PDF and an Altium export of the same board.

### Changed

- Checks read the part's value (`Part::value`), not the `Value` field by name: Altium's `Comment` counts where there is
  no `Value` (BOM-5, BOM-6, BOM-9, BOM-10, the I2C pull-up checks, the `Values` need), and findings name the field
  (`Value (from Comment)`); `erc_analysis::value::value_text` replaces `value_field`, `PartValue::field`.
- A finding resting on a fact reconstructed from a PDF by position or geometry, or on a board whose nets were traced
  from a drawing, is an estimate. Not-run hints for a PDF board say what a PDF cannot give (layout, wires,
  no-connect flags), and its findings point at the PDF and the page (`symbol U1 on page 3 (MCU.SchDoc)`).
- `Label::at` is optional (a PDF does not always say where a label is); the dump leaves `at` out then.
- The workspace needs Rust 1.92 (`rust-version`), for `hayro-syntax`.

## [0.4.2] - 2026-10-22

### Added

- `erc_model::class::PartClass`: the part class enum of architecture §1.3 with kinds (diode kind, BJT polarity, MOSFET
  channel, connector, switch, regulator, translator, memory, transceiver, logic family, fuse), `value_unit` for the
  unit a Value field is read in. `erc_analysis::class` resolves it as a fact: a `ClassSource` hook (the part database
  by MPN, later), the library symbol, description and keywords, net-tie pad groups, the designator prefix as Assumed;
  `PartClasses::coverage` for `Need::PartClasses` (`NET-19`).
- `erc_model::value`: `Value::read` reads the whole Value field (`Reading`: tolerance, ratings like `50V` and `1/4W`,
  dielectric, a ferrite's `@100MHz`, signs, RKM with a unit), `ValueWarning::SeparateUnit` (`4k7 Ohm`); `ESeries` E3 to
  E192 with membership and the nearest value. `erc_analysis::value::Values`: each fitted part's value in the unit of
  its class, per variant (`IMP-6`).
- `erc_model::View` (`Board::view(variant)`): a board as a variant mounts it, DNP and field overrides applied;
  `Need::Variant` (`NET-20`).
- `erc_analysis::power` rewritten on the power name grammar (§1.4): voltage tokens (`+3V3`, `VDD_1V8`, `+5V`, `3.3V`,
  `3P3V`, `-5V`, `VCC3V3`), power words with rail kinds (USB, battery, analog, reference), ground words with ground
  kinds, signal words that are no rail (`SCL_3V3`, `PWR_EN`); power symbols and power pins as signals, a `NetSource`
  hook for the project file; the voltage a `Facts<f64>` derived from the name; `GroundGroup`s joined at DC through net
  ties, jumpers, 0 Ω, ferrites and inductors, and `GroundLink`s between them (`NET-5` first layer, `NET-6`).
- `erc_analysis::drawing::Drawing`: labels with their scope, sheet pins matched to hierarchical labels, no-connect
  flags, dangling labels, flags on connected pins; `Need::Drawing` (`NET-21`). `LabelKind::LocalPower` for KiCad 10
  local power symbols (`erc_kicad` reads them as such).
- `erc_analysis::i2c`: `I2cBuses::on(view, classes)` on the fitted parts; ferrite, net-tie and jumper ties; discrete
  translators (a BSS138 pair) with their gate nets; `pull_ups(segment)`, `direct_segment`, `segments_of_logical_bus`
  (`NET-4`). Test schematic `i2c-18_discrete_translator`.
- `erc_checks::Ctx` builds the analyses lazily (`view`, `classes`, `values`, `power`, `i2c`, `drawing`), and
  availability comes from them (`Availability::of`, `Ctx::availability`): part class coverage, rails, rail voltages
  with their coverage, variant, drawing (`DIAG-2`).
- `test_schematics/analyses`: power, parts (with a KiCad 10 variant) and drawing schematics with `.expect.toml`
  sidecars, checked by `erc_analysis/tests/analyses.rs` (`BLESS=1` rewrites them).

### Changed

- `BOM-10` checks capacitors and inductors too and reports a value in no E series with the nearest E24 and E96
  values; the unit comes from the part class, not the letter.
- The power name parser reads the whole grammar by default: `+5V` and `+3.3V` now have a voltage; the old `+xVy`-only
  reading is `PowerOptions::strict`. `5V0_NEG` reads −5 V (the sign from its marker, said in the fact's note), `VEE`
  is a negative rail and no longer a ground, `ISO_5V` is no ground.
- The I2C recogniser and the checks on it see only the parts the variant fits: a DNP pull-up no longer counts.

## [0.4.1] - 2026-10-22

### Added

- `erc_kicad`: footprints link to schematic parts by the footprint's schematic UUID path, KiCad's own link
  (`Footprint::schematic_path`, read from `(path ...)`, in the canonical dump as `path`), so a part re-annotated in
  the schematic but not yet on the board still finds its footprint; the designator is the fallback when there is no
  path or it names no symbol. Where they disagree the import gives a new `NoteKind::Conflict` note (`IMP-14`).
- `just test-full`: `just test` with every hw board in the differential (`BERC_HW_FULL=1`).

### Changed

- `just test` compares a sample of six hw boards in the differential test instead of all ~41 (30-50 s); the full set
  runs with `just test-full`, for CI and before landing.

## [0.4.0] - 2026-10-22

### Added

- `erc_analysis::power` (`NET-5`, first layer): rails (by a voltage in the name, `+V`/`VDD`/`VCC`, or a power pin on
  the net, each a fact with its rule and the voltage from the name), grounds and switching nodes, ported from
  `erc_core`'s `power.rs`; the I2C lines and switching nodes are not rails. `name_voltage` reads `+1V25` as 1.25 V
  (the old parser read 2.25 V). `erc_analysis::graph::input_pins`: the input pins on a net (the IO-1 stub's list).
- `berc diff --pnp A B` (`TOOL-1`): the PnP compare of the old `ecad_compare` tool on `erc_model::placecmp`, each side
  any design `berc check` opens (PnP files, a `.kicad_pcb`, a KiCad project with its board); `--origin page|aux` for a
  board compared with a PnP file (brought into the PnP frame: from the origin, y up, without footprints excluded
  from position files, as kicad-cli's `pcb export pos` writes it), `--tolerance MM`, `--rotation-tolerance DEG`,
  `--format text|json`; exit 0 same, 1 different, 2 error. The text names each side's footprint and test point
  count, as the old example did. `placecmp::{Origin, to_placement_frame, aux_origin, is_board_file}`; the diff types
  serialise.
- The library facade (`TOOL-6`), crate `better_erc` (lib): `model`, `kicad`, `import`, `analysis`, `checks`
  re-exported, the run entry (`Project`, `check_path`, `CheckOptions`, `CheckRun`, `Verdict`) and the report types at
  the top, and `better_erc::diff::pnp`. berc and the app depend on it alone.
- `Project::open_files(main, others)`: a design whose files were picked one by one from anywhere (an Altium EDIF and
  its WireList, the OrCAD trio), read as if beside each other; the app's import tab uses it.
- `erc_analysis::power::PowerOptions { strict }`: the old non-strict mode (`VDD_1V8` without the `+`) kept;
  `erc_analysis::graph::part_chains`, `PcbAssembly::find_part_chains` on the board model.
- The old readers' tests as acceptance tests on the new core: `ecad_file_format::kicad_netlist`'s asserts in
  `erc_kicad/tests/netlist.rs`, the passive value table in `erc_model::value`.

### Changed

- **Breaking:** the app crate is `better_erc_app` (directory `better_erc_app/`, binary still `better_erc`, so
  `just install`, `just run` and `just deploy` work as before); the crate name `better_erc` is the library facade.
- The app runs on the new core (`IMP-7`): the import tab opens any design through the importer registry (a file or
  a directory), keeps the KiCad schematic, Altium (EDIF + WireList) and OrCAD (three files) pickers, which now all
  load, and shows each design's report: findings with their check id, severity, subjects, facts and the catalogue's
  explanation, the checks that did not run and why, the import notes; a design loaded again replaces itself, Close
  removes it. The context holds the designs with their check run and per board the part classes, I2C buses and power
  structure. Nets (`VIEW-1`: rails with their voltage, grounds, switching nodes, nets with node count), I2C
  (`VIEW-2`: the I2C findings and the buses with designators and net names), Style (`VIEW-3`: BOM and LIB findings)
  and Inputs (`VIEW-4`) read them.
- The KiCad netlist fixtures move from `ecad_file_format/test_input` to `erc_kicad/tests/fixtures/netlist`;
  `just netlist-fixtures` exports them there.

### Removed

- **Breaking:** the old crates `ecad_file_format` (netlist model and readers), `erc_core` (`PcbAssembly`, its checks
  and analyses) and `ecad_compare` (PnP compare), and with them `lexpr` and `serde-lexpr` (`NET-16`). Everything they
  did runs on the new core: the readers in `erc_kicad` and `erc_import`, values in `erc_model::value`, the
  analyses in `erc_analysis` (classes, graph, I2C, power), the eleven checks in `erc_checks`, the PnP compare as
  `berc diff --pnp`; their tests are acceptance tests there.

### Fixed

- The KiCad netlist importer keeps KiCad 10's unit names (`A`, `B`) and unit count on the symbol, as the old reader
  did (`IMP-12`).

## [0.3.9] - 2026-10-22

### Added

- `berc check PROJECT` for CI (`RPT-20`): a directory with one design (its `.kicad_pro`) or any design file the
  importer registry reads, `--variant`, `--only CAT,ID`, `--format text|json|sarif|md`, `--baseline report.json`,
  `--fail-on error|warning|suggestion`, `--require NEED`; exit codes 0 clean, 1 findings at or above `--fail-on`,
  2 usage or import error, 3 a required need unavailable. `--state` is reserved for power states and refused.
- `erc_checks::project`, the library entry under it, for the MCP server and the GUI too: `Project::open` through the
  importer registry, `Project::check` / `check_path` with `CheckOptions { run, require, fail_on, baseline }`,
  `CheckRun` with the report and the `Verdict` (exit code as data), typed `CheckError`s, `read_baseline`, progress
  stages. Cargo feature `import`, on by default.
- Baselines (`RPT-23`): `Report::compare` marks each finding new or unchanged by board and fingerprint and keeps the
  fixed ones; with a baseline only new findings fail.
- SARIF 2.1.0 (`RPT-24`) for GitHub code scanning, with rule ids from the catalogue and findings at the schematic
  file and the line of their symbol; Markdown (`RPT-25`) for a PR comment; coloured text on a terminal.
- `berc explain ID` and `berc checks list [--tier] [--status] [--project]` (`TOOL-5`), `erc_checks::list_for`,
  `Explanation::to_text` / `to_markdown`; completion of check ids, categories, needs and variants of the design in
  the current directory; a spinner for runs over two seconds.
- `erc_kicad::sexpr::Lines`: the line of any parsed expression; `UnitPlacement::line` (in the canonical dump as
  `line N`) and `Location::line` carry a symbol's line into findings. `Need::slug` / `from_slug`.
- README "Checks in CI" with a GitHub Actions example uploading SARIF to code scanning; `just check` runs berc check on
  the test schematics against `test_schematics/berc-baseline.json`, `just check-baseline` rewrites it;
  docs/engine.md describes the library entry, the formats and the command line.

## [0.3.8] - 2026-10-22

### Added

- `erc_import`, every importer behind one registry (`IMP-15`): the importer for a file is picked by its name and
  contents, KiCad's through `erc_kicad::importers()`; one cargo feature per importer so a KiCad-only or wasm build
  leaves the rest out.
- Altium (EDIF plus WireList), OrCAD (pstxnet, pstxprt, pstchip) and pick-and-place CSV files read into the board
  model (`IMP-3`, `IMP-4`, `IMP-5`), their pest grammars moved along and taught to read past unknown elements: every
  attribute a fact with the file it came from, notes for everything not kept, no panic on broken input (a test feeds
  truncated and garbled fixtures to every importer), a golden dump per fixture, and the old readers' test assertions
  as acceptance tests.
- `erc_model::placecmp`, the PnP compare on two layouts (two PnP files, two boards, or a board and its PnP file),
  rotations modulo 360 and tolerances, for `berc diff --pnp` (`TOOL-1`).
- `erc_model`: `Provenance::Placement` for facts from a PnP file, `SymbolDef::unit_names` (OrCAD section letters).

### Changed

- The Altium, OrCAD and PnP fixtures moved from `ecad_file_format/test_input` to `erc_import/tests/fixtures`; the old
  readers' tests read them there.

## [0.3.7] - 2026-10-22

### Added

- `erc_checks`, the check engine (`DIAG-1` to `DIAG-6`, `RPT-22`): one file per catalogue check with its id,
  severity, tier, scope, declared data needs, overlaps and thresholds, registered statically; per board the engine
  works out which needs are available, runs the checks that can run in parallel over one context whose analyses are
  built on first use, and reports the others as not run with why and what would enable them. One `Finding` for every
  check (severity with a project override, confidence, subjects with names, variant, facts with their sources,
  fingerprint, detail page link, fix, locations); a finding of an overlapping check folds the other in (`also
  BOM-5`). The `Report` (summary, every catalogue check ran / not run / planned, findings worst first by category,
  candidates, notes, timing) as JSON with a versioned schema and as text. The catalogue and its detail pages are
  embedded: `list()`, `explain(id)` and the not-run hints work offline. Described in [docs/engine.md](docs/engine.md).
- The eleven shipped checks run on the new core: `I2C-1`, `I2C-2`, `I2C-3`, `I2C-7`, `I2C-9`, `I2C-10`, `BOM-5`,
  `BOM-6`, `BOM-9`, `BOM-10`, `LIB-3`, each with its unit test. `I2C-9` and `I2C-10` are findings of their own now,
  `BOM-5` is an error as the catalogue says, `I2C-9`'s window is inclusive as the catalogue says (2.2 kΩ and 10 kΩ
  no longer fire; `erc_core` reported the ends with the lumped `I2C-1`), `BOM-6` reports a placeholder on a fitted part (in the variant checked)
  apart from `BOM-5`, and `BOM-5` also catches `TBD` and `TODO`.
- `erc_analysis` (`NET-4`, `NET-19`): part classes from the library symbol, the description, or the designator prefix
  as an assumption (a finding resting on it is an estimate); values read in their class's unit per variant; graph
  queries; the I2C bus recogniser ported from `erc_core` (buses, pull-up pairs and redundant pull-ups, ties, discrete
  and IC translators, segments; parts in no known role become notes).
- Test schematics as the spec: every `test_schematics/sources/<id>_<slug>.kicad_sch` has an `.expect.toml` of the
  findings and not-run checks the engine must report on it, and one test compares them (`BLESS=1` rewrites them); a
  test holds every registered check to its catalogue line.
- `erc_model`: `Board::fitted` and `Board::field` per variant, `Board::is_component`; the value parser warns about more
  than one space before the multiplier, a small `r` and an `R` after a multiplier.

### Changed

- The test schematics are named after their check (`i2c_segments` is `i2c-7_segments`, `style_values` is
  `bom-10_values`); the MOSFETs moved to `lib-3_mosfet_pin_names`, and `bom-10_values` gained a placeholder on a DNP
  part and a `TBD`.
- The catalogue lines and detail pages of the ported checks say what the code checks now, without "runs today:
  `style.rs`".

## [0.3.6] - 2026-10-22

### Added

- `erc_model`, the board model of the rewrite (`NET-17`): an arena-indexed design of boards, sheets, symbols, parts,
  pin instances, nets, labels, variants, net classes, project settings and the board layout; every attribute a fact
  with its source, rank, time and note, resolved to the highest rank (newest on a tie) with disagreements listed; typed
  values; one `Importer` trait for every tool; a canonical text dump, one sorted fact per line, that reads back to the
  same design and is the golden format of the importer tests. Described in [docs/model.md](docs/model.md).
- `erc_kicad`, KiCad 8, 9 and 10 read by better_erc itself (`IMP-13`, `IMP-14`, `NET-18`): the schematic hierarchy
  with KiCad's connectivity rules (label scopes, power symbols, sheet pins, vector and group buses, bus entries and
  aliases, hidden power pins, junctions, the version differences between KiCad 8 and 9/10), the project file (net
  classes, text variables, variants, ERC and DRC settings, design rules), each net's effective net class, and the board
  (footprints, pads, tracks, vias, zones, graphics, stackup). The KiCad netlist importer moved onto the same trait.
- The differential test: our nets against kicad-cli's netlist as pin sets, names and net classes, on fixtures with
  KiCad 8, 9 and 10 netlists, the test schematics, and every board under `~/git/hw` when it is there (41 boards equal
  KiCad 10's netlists, 94 comparisons over KiCad 8, 9 and 10). `just kicad-fixtures` exports the fixture netlists.

### Changed

- The s-expression reader moved from `berc` to `erc_kicad::sexpr`; `berc` re-exports it and `berc textconv` is
  unchanged (`NET-16`).

## [0.3.5] - 2026-10-22

### Added

- Detail pages for every check, one file per category under `docs/checks/` (34 pages, 513 sections): what the check
  catches, why the board fails without it, a concrete example (our own boards' misses where there is one), how
  better_erc can detect it and what data that needs, the fix, and references that were fetched and checked before
  being cited (vendor app notes and datasheets, specs, Wikipedia, books by edition and chapter). Each section has an
  `Impl.` line: how hard the check is to implement in pure Rust with no LLM (`easy` 139 · `mid` 291 · `hard` 64 ·
  `judgement` 19) and the FEATURES.md ids it depends on. The index `docs/checks.md` carries the tier and a 📖 link on
  every line, and explains the tiers in its header; FEATURES.md has the per-category tier table.

### Changed

- Catalogue fixes found while writing the pages: `ETH-1` had current- and voltage-mode PHY centre-tap termination the
  wrong way round; `BOM-12`'s example attributed B200A's 13k3-for-13k7 to B153A (whose R219/R222 were 13k3 for 3k3);
  `NAME-9`'s example pins (IO4 = GPIO17, IO5 = GPIO16); `I2C-12` SMBus 3.0 thresholds are 0.8 / 1.35 V; `I2C-14`
  names the push-pull TXB class as the problem on I2C, not the open-drain TXS class.

## [0.3.4] - 2026-10-21

### Changed

- Check catalogue (`docs/checks.md`) grown from 166 to 513 checks. New categories: `BAT` (batteries and chargers),
  `WIRE` (connectivity hygiene: one-pin nets, near-duplicate net names, no-connect misuse), `BOOT` (boot modes and
  strapping pins), `ETH` (Ethernet PHY, magnetics, RGMII/RMII), `ERR` (known errata by MPN). Every category got
  new entries; the real misses of the B125, B135, B153, B156 and B200 assemblies and bring-ups became checks
  (`RAIL-4` fixed regulator voltage vs net, `RAIL-5` rail without a source, `MUX-4` GPIO on an NRST pin, `IO-12`
  output enables without a pull, `IO-13` peripheral reset held by a resetting controller, `BOM-12` value vs MPN,
  `LIB-6` footprint vs MPN package, `LVL-11` TXS-class shifters on push-pull lines, `SEQ-6` hard turn-on into a
  capacitive load, `USB-7`/`USB-8` CC resistor sources and roles, among others), and the P2623 I2C-in-the-wild
  notes became `I2C-11`..`I2C-22`. New rule "one check, one concern": 22 lumped entries were split, the old ID
  keeping its first concern and the rest taking new numbers (`PIN-1` → `PIN-4`..`PIN-6`, `I2C-1` → `I2C-9`,
  `I2C-10`, `SER-1` → `SER-7`, `SER-8`, ...), each marked with a `split` comment. The review checklist follows
  the catalogue. FEATURES.md: `I2C-1`'s code also covers `I2C-9` and `I2C-10` as one finding for now.

## [0.3.3] - 2026-10-21

### Changed

- Crate docs for `ecad_file_format`, `erc_core` and `ecad_compare`; an empty test module in erc_core removed.
- `Cargo.lock` is committed (it was gitignored), so `cargo install --locked` and `just install` build every
  clone with the same dependency versions.
- Building erc_core no longer runs kicad-cli: its build script exported the test schematics on every build, also
  when building or installing the app, and wrote into the source tree. The tests now export them on their first
  `get_netlist_path` (once per test process), `generate_netlists` is a dev-dependency only, and `just netlists`
  exports on its own. A missing netlist names the likely cause (no schematic, no kicad-cli).
- README: what better_erc is and who it is for, what works today (berc textconv, the importers, the checks that
  run, the app and how far its import goes), install, a quick start for diffs and for running the checks from
  Rust, development recipes, and links to the check catalogue, the review checklist and FEATURES.md. The Rust
  example is also a doc test on `PcbAssembly`, so it keeps compiling. FEATURES.md `IMP-7` now says only Altium
  loads in the app's import tab.
- `berc` (TOOL-4): help is coloured and `berc: error: ...` has a red `error:`, both only on a terminal and never
  with `NO_COLOR` (`textconv` output stays plain); `--help` says how to turn on shell completion. Tests for the
  binary itself: the `--version` line, a plain error on a pipe, completion scripts for bash, zsh and fish.
- `justfile`: new `fmt`, `run` (the app), `netlists` (export the test schematics with kicad-cli, through a new
  `generate_netlists` binary; before, only erc_core's build script could) and `netlist-fixtures`; each recipe has
  a one-line description for `just --list`.
- The `general` and `style` tests (IMP-12) read new test schematics instead of OrCAD and Altium project files
  that only existed on one PC, and now assert what they find instead of printing it: `style_values` has one
  part per style finding (BOM-5/6, BOM-9, BOM-10 with each value warning, LIB-3, a fine value and a DNP), and
  `io_inputs` an input driven and one floating for the IO-1 stub. A second test takes the in-repo OrCAD fixture
  through `PcbAssembly`, the path the old `general` test covered. `input_without_driving_source` gets its
  input list from a new `inputs()`, which the test reads; it still prints the same list.

### Fixed

- KiCad 9 and 10 netlists load again (IMP-12): the KiCad netlist reader knows the elements those versions add,
  per KiCad's own netlist exporter source: net classes (kept as the net's `class` property), component classes,
  and in KiCad 10 the unit list (now the component's `sections`), per-component variants, jumper pins and
  groups. Fixtures exported by kicad-cli 8.0.9, 9.0.9 and 10.0.7 from one small schematic
  (`ecad_file_format/test_input/kicad_versions`, `just netlist-fixtures` regenerates them from KiCad's container
  images) must read to the same netlist; a load error now names the file. The five I2C tests are green again.
- `BOM-9` (part without a value) fires on KiCad designs: KiCad writes an empty value as `~` in the netlist, which
  the reader took for a value, so the check never saw an empty one; `~` now reads as empty. Found by the new
  `style_values` test schematic.
- The `find_net_chains` doc example compiles (a missing `;`); it was never run while the erc_core tests failed
  first.
- Regenerating the test schematic pictures (`test_schematics/generated_pictures`, on every fresh clone or
  schematic change) no longer shows all of them as changed: `generate_netlists` drops the export time KiCad
  writes into each SVG's `<title>`.

## [0.3.2] - 2026-10-21

### Added

- `docs/checks.md`: the check catalogue (IDs like `PIN-1`, `I2C-3`, severity, overlaps), moved here from the tpm
  repo so it ships with the public code. `docs/review-checklist.md`: the condensed schematic review checklist,
  each ID linking back to its catalogue entry. README, FEATURES.md and CLAUDE.md now point here instead of tpm;
  wording scrubbed of internal project names, board names and tools (the IO-9 and LOGIC-5 examples, an I2C-8
  bench note, and two other asides) for a public repo.

## [0.3.1] - 2026-10-17

### Added

- `berc textconv` reads `.kicad_pro` too: every project setting as a dotted path
  (`board.design_settings.defaults.zones.min_clearance = 0.3`), net classes and other named lists keyed by name,
  and settings that match KiCad 10.0.6's own default (`pro_defaults`, a table transcribed from the KiCad
  source - zones, the DRC/ERC rule severities, the ERC pin conflict matrix, `reuse_designators`) or JSON's
  "nothing here" value (`false`/`0`/`""`/`[]`/`{}`) folded into one summary line per section, so opening a
  project in a newer KiCad and saving it - which can add dozens of such settings - shows as a small diff instead
  of a wall of JSON (cannify_micro.kicad_pro, real fixture: 72 raw diff lines to 13). `--all` shows the folded
  settings too (TOOL-3).

### Changed

- `justfile install` also installs `berc` (`cargo install --locked --path berc`), and `deploy` checks
  `berc --version` instead of only `better_erc` being on PATH.
- `berc --version` labels its timestamp `commit` instead of `built`, and that timestamp is now the commit's own
  time instead of the build clock's, like `tpm_cli`'s `build.rs` - so it doesn't change between rebuilds of the
  same commit, and a worktree build no longer looks stale to cargo.

### Fixed

- `just lint` is green again: mechanical clippy fixes across `ecad_file_format`, `erc_core`, `ecad_compare`,
  `better_erc` and `generate_netlists` (needless borrows, `.iter().count()` to `.len()`, `for_kv_map`,
  collapsible ifs, an unnecessary `let` binding, `len_without_is_empty`, a missing `Default` impl, a couple of
  unused imports), no behaviour change. Pre-existing drift, unrelated to the `.kicad_pro` work above.

## [0.3.0] - 2026-10-09

### Changed

- `justfile` gets a `default` recipe (`@just --list`), so a bare `just` only lists the recipes instead of
  running tests, builds or deploying.

## [0.2.1] - 2026-10-07

### Changed

- `lexpr` and `serde-lexpr` come from upstream git, pinned to `09a61d3`, instead of a `../../lexpr-rs` checkout,
  so the workspace builds on any PC. Dropping them completely is planned (NET-16).

### Fixed

- The workspace builds on Linux: `generate_netlists` uses `kicad-cli` from PATH outside macOS, and creates the
  gitignored `generated_netlists` folder on a fresh clone. Seven `erc_core` tests still fail there (IMP-12).

## [0.2.0] - 2026-10-07

### Added

- `berc` crate with `berc textconv FILE`: a `.kicad_sch` or `.kicad_pcb` as sorted, line-oriented text for git's
  textconv, so `git diff` and `git log -p` show what a person changed (symbols, fields, variant overrides, text,
  labels, footprints and their pad nets, design rules), not UUIDs, coordinates and reordering; `--with-geometry`
  adds positions; `berc --version` shows the git SHA and build time. Setup in README "Readable git diffs" (TOOL-3)
- `FEATURES.md` tracker with stable area IDs (IMP, NET, DIAG, CFG, RPT, VIEW, APP, TOOL, GEN, BIZ); `CLAUDE.md`
  points at it and at the check catalogue in tpm (`ideas/P2503-better-erc/rules.md`)
- This changelog
- `justfile` with the standard recipes `test`, `lint`, `install` and `deploy` (pull --ff-only, refuses on local
  changes, install); documented in CLAUDE.md

## [0.1.0] - 2025-04-24

### April 2025

- egui app (`better_erc` crate) with tiles and tabs, shown and hidden from the side panel, new icon (APP-1)
- Netlist view and first tabs: nets, style, inputs, import (VIEW-1, VIEW-3, VIEW-4, IMP-7, in progress)
- `pnp_compare` tool: compare two PnP files (TOOL-1)

### March 2025

- Common `Netlist` model with typed `Designator`, `NetName`, `PinName`, `PinId`; nodes kept in a `HashSet`
  (NET-1)
- Graph queries: `find_chains`, `find_connected_parts`, `is_connected`, `are_parts_connected`, `connected_net`
  (NET-2)
- `PcbAssembly` collects the analysis results, with `find_part_chains` (NET-3)
- Power structure from net names (`derive_power_structure`, NET-5, in progress)
- OrCAD netlist: nets, components and library parts (IMP-3)
- Altium netlist: EDIF parser combined with the WireList for pin types (IMP-4)
- KiCad: pin types and library parts parsed (IMP-1)
- KiCad schematics turned into netlists through kicad-cli, cached per file and regenerated when they change
  (IMP-2)
- Passive value parser: Ω with milli and micro, k, M, printed with human-repr (IMP-6)
- I2C bus finder: direct and common segments, connected parts, voltage translators (NET-4)
- I2C checks: missing pull-ups, non-standard or unequal SCL/SDA pull-ups, redundant pull-ups, pull-ups through
  series resistors to nowhere, series tie resistor too high (I2C-1, I2C-2, I2C-3, I2C-7)
- Style checks: "calculate later" values, parts without a value, values that don't parse or aren't standard,
  MOSFETs without named pins (BOM-5, BOM-6, BOM-9, BOM-10, LIB-3)
- Thresholds moved from hardcoded values into `erc_core/src/config.rs` (CFG-1)
- PnP file comparator in `ecad_compare`
- Test schematics re-annotated with page X100 designators; SVG pictures generated for them

### February 2025

- pest grammars for the OrCAD netlist, part list and library parts, and for the Altium WireList
- KiCad netlist export format and reader (IMP-1)
- PnP CSV reader for KiCad, Altium and Allegro (IMP-5)
