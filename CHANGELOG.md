# Changelog

All notable changes to better_erc are recorded here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). [FEATURES.md](FEATURES.md) holds the current
status of every feature; this file only records what changed. IDs in parentheses (`IMP-4`, `I2C-3`) refer to
FEATURES.md and the check catalogue.

All crates share one version (`[workspace.package]`). Every commit with real work bumps it (minor for features,
patch for fixes) and moves `[Unreleased]` under the new version. Nothing has been tagged yet. History up to
0.1.0 is grouped by month; it was backfilled on 6 Oct 2026 from the git history.

## [Unreleased]

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
