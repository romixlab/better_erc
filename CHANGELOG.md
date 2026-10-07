# Changelog

All notable changes to better_erc are recorded here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). [FEATURES.md](FEATURES.md) holds the current
status of every feature; this file only records what changed. IDs in parentheses (`IMP-4`, `I2C-3`) refer to
FEATURES.md and the check catalogue.

All crates share one version (`[workspace.package]`). Every commit with real work bumps it (minor for features,
patch for fixes) and moves `[Unreleased]` under the new version. Nothing has been tagged yet. History up to
0.1.0 is grouped by month; it was backfilled on 6 Oct 2026 from the git history.

## [Unreleased]

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
