# better_erc features and roadmap

The single source of truth for what the better_erc code does and what is planned. The checks themselves (what to
catch, with severity) are specified in the check catalogue, [docs/checks.md](docs/checks.md), with IDs like
`I2C-3` or `PIN-1`; this file tracks which of them run and everything around them: importers, the netlist model,
analysis, reports, views and the app.

Last full review: 5 Oct 2026 (commit `9952563`, last code change 24 Apr 2025). Statuses come from reading the code,
not from running it. Imported from the Notion tracker (exported 1 Oct 2026).

## How to use this file

- **Status**: ✅ done · 🚧 partial (the note says what is missing) · 🐛 known bugs · ⬜ stub (exists, does nothing
  useful yet) · 📋 planned · 💡 idea · ⛔ blocked · 🔍 needs a check
- **IDs** (`IMP-3`, `RPT-7`) are stable: never renumber or reuse. New items take the next free number of their area.
  Use them in commit messages, CHANGELOG entries and `TODO(NET-5)` comments. Check catalogue IDs (`I2C-3`) are used
  as they are, they don't get a second ID here.
- Each area lists what works first, then open items by priority. When finishing work, mark the item ✅ with a
  pointer (module, tab) in the same commit.

## Crates

- `erc_model`: the board model of the rewrite: arenas of parts, pins, nets, sheets, the layout; facts with
  provenance; the importer trait; the canonical dump ([docs/model.md](docs/model.md))
- `erc_kicad`: KiCad 8, 9 and 10 read by us: s-expression reader, schematic hierarchy with KiCad's connectivity,
  project file, board, the KiCad netlist as the oracle
- `erc_analysis`: analyses over the board model: part classes, values, graph queries, the I2C bus recogniser
- `erc_checks`: the check engine and the checks, one file per catalogue id; findings, not-run reports, the report
  as JSON and text, the embedded catalogue ([docs/engine.md](docs/engine.md))
- `erc_import`: the importer registry, and the Altium, OrCAD and PnP importers into the board model
  ([docs/model.md](docs/model.md))
- `ecad_file_format`: netlist, BOM and PnP parsers, the common `Netlist` model, passive value parsing
- `erc_core`: the first analysis and checks on the old `Netlist` (`PcbAssembly`, power structure, I2C buses), still
  feeding the desktop app; the checks themselves run in `erc_checks`
- `ecad_compare`: board file comparisons (PnP so far)
- `better_erc`: egui/eframe desktop app (`egui_tiles` tabs)
- `berc`: command line tools (`berc textconv` so far)
- `test_schematics`: KiCad test schematics, one per check, with a netlist generator (`generate_netlists`)

## Import (`IMP`)

- ✅ `IMP-1` KiCad netlist from KiCad 8, 9 and 10 (serde-lexpr), incl. pin types, net classes and units; a test per
  version · `ecad_file_format/src/kicad_netlist.rs`, `test_input/kicad_versions`
- ✅ `IMP-2` KiCad schematic → netlist through kicad-cli, cached per file · `test_schematics/generate_netlists`
- ✅ `IMP-3` OrCAD netlist: nets, components, library parts; in the board model: pstxnet, pstxprt and pstchip found
  beside each other, sections as units with their letters, pages as sheets, pin types with the pstchip properties
  that gave them · `erc_import/src/orcad`, old reader `orcad_netlist.rs`
- ✅ `IMP-4` Altium: EDIF netlist + WireList (for pin types) combined; in the board model: either file as the main one,
  symbols, sheets, part properties, nets, pin names and types with both files as sources (the WireList's type wins)
  · `erc_import/src/altium`, old readers `edif_netlist.rs`, `wirelist.rs`, `altium_netlist.rs`
- ✅ `IMP-5` PnP CSV from KiCad, Altium and Allegro; in the board model as a layout of footprints (position, rotation,
  side, value, package, the other columns as fields), mm, mil or inch · `erc_import/src/pnp.rs`, old reader `pnp.rs`
- ✅ `IMP-6` Passive values (Ω with milli/micro, k, M) · `passive_value.rs`
- 🚧 `IMP-7` Import tab in the app: KiCad schematic, Altium and OrCAD netlist sources. Only Altium loads; the KiCad and
  OrCAD sources pick their files but load nothing yet · `tabs/pcb_data_import.rs`
- 📋 `IMP-8` OrCAD: one BOM per variant
- 📋 `IMP-9` Pinmux import from Excel
- 📋 `IMP-10` Xilinx bank info from pin names or Xilinx ASCII pin files
- 💡 `IMP-11` Watch the netlist file and reload on change
- ✅ `IMP-12` KiCad 9 and 10 netlists load (a fixture per KiCad version, `test_input/kicad_versions`); the `general` and
  `style` tests run on test schematics in `test_schematics` instead of project files that only existed on one PC
- ✅ `IMP-13` KiCad schematic hierarchy read by us (KiCad 8, 9, 10) into the board model: sheet instances, parts with
  units, fields, flags and KiCad 10 variants, pin instances, labels, no-connects, and connectivity by KiCad's rules
  (label scopes, power symbols global and KiCad 10 local, sheet pins, vector and group buses, bus entries and aliases,
  hidden power pins, junctions; the 8 vs 9/10 junction and label-on-crossing rules by `ImportOptions::kicad_version`);
  the KiCad netlist importer on the same `Importer` trait as the oracle. Differential test against kicad-cli's netlist
  (nets as pin sets, names and net classes): fixtures with KiCad 8, 9 and 10 netlists, the test schematics, and every
  `~/git/hw` board when present · `erc_kicad/src/{sch,netlist.rs,project.rs}`, `erc_kicad/tests/differential.rs`
- ✅ `IMP-14` KiCad project and board files into the model: `.kicad_pro` (net classes, patterns and assignments, text
  variables, variants, ERC and DRC severities and exclusions, design rules, bus aliases) and `.kicad_pcb` (layers,
  stackup, setup, footprints with fields, attributes, pads, courtyard, silk and fab graphics, tracks, vias, zones and
  rule areas, outline, texts), linked to the schematic by designator and net name · `erc_kicad/src/{pro,pcb}.rs`
- ✅ `IMP-15` One importer registry: the importer for a file picked by its name and contents (KiCad project,
  schematic, board, KiCad netlist, Altium, OrCAD, PnP), a directory scanned for what can be imported, one cargo
  feature per importer; every import gives notes for what it read past and never panics on broken input (a test on
  truncated and garbled fixtures), and a golden dump per fixture · `erc_import/src/lib.rs`, `erc_import/tests`

## Netlist model and analysis (`NET`)

- ✅ `NET-1` Common `Netlist` model with typed `Designator`, `NetName`, `PinName`, `PinId` · `netlist.rs`
- ✅ `NET-2` Graph queries: `find_chains`, `find_connected_parts`, `is_connected`, `are_parts_connected`, `connected_net`
- ✅ `NET-3` `PcbAssembly`: collects analysis results, `find_part_chains` · `erc_core/src/pcba.rs`
- ✅ `NET-4` I2C bus finder: direct and common segments, connected parts, voltage translators · `erc_analysis/src/i2c.rs`
  on the board model (ported from `erc_core/src/i2c.rs`, which still feeds the app). Buses are still found by net
  names; pin roles come with `NET-7`
- 🚧 `NET-5` Power structure: rails and grounds from net names only (strict mode needs `+xVy`); voltage parsed from
  the name. Missing: sources (LDO, DC-DC, connectors), 0R and shunt ties, power switches · `power.rs`
- 🚧 `NET-6` Switching nodes finder · `power.rs`
- 📋 `NET-7` Pin model: digital/analog/power in, out, bidir, open drain; pulls; thresholds vs bank voltage; source
  and sink current per pin and bank; quiescent current per mode
- 📋 `NET-8` Power states: which rails are on in each state, which parts are powered
- 📋 `NET-9` MCU IOs treated as tristate by default (needed by `IO-4`), with `default_gpio_state` override
- 📋 `NET-10` Follow a signal through resistors, translators, isolators, capacitors, FETs
- 📋 `NET-11` I2C addresses from straps (needs `NET-7`)
- 📋 `NET-12` MCU alternate functions from SVD / stm32-data (needed by `MUX-2`)
- 📋 `NET-13` Multi-board: several netlists joined through connectors
- 📋 `NET-14` Standard plug-in cards for multi-board (M.2 device, DP cable...)
- 💡 `NET-15` Select a piece of schematic and simulate it
- 🚧 `NET-16` Drop lexpr / serde-lexpr: `ecad_file_format` parses KiCad through them, pinned to upstream master `09a61d3` (Aug 2024) because crates.io 0.2.7 (Mar 2023) lacks numeric-looking symbols. The s-expr reader now lives in `erc_kicad::sexpr` (berc re-exports it) and the new importers use it; lexpr goes with `ecad_file_format` once the checks run on the new model
- 🚧 `NET-17` Board model (`erc_model`): arena-indexed design of boards, sheets, symbols, parts, pin instances, nets,
  labels, variants, net classes, project settings and the layout; every attribute a `Fact` with provenance, rank, time
  and note (highest rank wins, newest on a tie, disagreements listed); typed `Value`; the `Importer` trait; the
  canonical dump (one sorted fact per line, read back losslessly) as the golden format of importer tests; `netcmp`.
  Docs: [docs/model.md](docs/model.md). Missing: pin roles and the analyses on top (stage 1+)
- 🚧 `NET-19` Part classes: resistor, capacitor, MOSFET, BJT, connector, translator, IC, ..., each a fact with the rule
  that decided it (library symbol, description and keywords, designator prefix as an assumption) · `erc_analysis/src/class.rs`.
  Missing: the part database and MPN as sources, the subclasses (diode kinds, regulator kinds) of the full model
- ✅ `NET-18` Each net's effective net class as KiCad resolves it (directive labels, project patterns as anchored regex
  or wildcard, priority order, `Default` filling unset rules), equal to kicad-cli's netlist classes · `erc_kicad/src/netclass.rs`

## Checks and diagnostics (`DIAG`)

Checks are named by their catalogue ID. Only the ones with code are listed; everything else in the catalogue is
planned.

Every catalogue check carries an implementation tier (`impl:` in [docs/checks.md](docs/checks.md), explained in each
detail page): `easy` runs on the netlist alone, `mid` needs part data or a project setting first, `hard` needs analysis
the code lacks, `judgement` is not a rule. Counts on 22 Oct 2026:

| category | easy | mid | hard | judgement | total |
|---|---|---|---|---|---|
| `PIN` | 3 | 13 | 1 | · | 17 |
| `RAIL` | 4 | 20 | 2 | · | 26 |
| `DEC` | 7 | 6 | · | · | 13 |
| `SEQ` | 1 | 3 | 3 | 2 | 9 |
| `GND` | 2 | 5 | · | · | 7 |
| `DRV` | 7 | 12 | 1 | · | 20 |
| `BAT` | · | 6 | · | 1 | 7 |
| `SHORT` | 9 | 3 | · | · | 12 |
| `WIRE` | 5 | 12 | · | · | 17 |
| `BKF` | · | 2 | 9 | 1 | 12 |
| `LEAK` | 1 | 4 | 2 | · | 7 |
| `LVL` | 3 | 11 | 1 | · | 15 |
| `IO` | 8 | 7 | 3 | · | 18 |
| `LOGIC` | 3 | 2 | 1 | 2 | 8 |
| `MUX` | 1 | 6 | · | 1 | 8 |
| `BOOT` | · | 8 | 2 | · | 10 |
| `CLK` | 7 | 12 | · | · | 19 |
| `I2C` | 8 | 11 | 2 | 1 | 22 |
| `SER` | 1 | 21 | 1 | 1 | 24 |
| `ETH` | 1 | 9 | 4 | · | 14 |
| `MEM` | 4 | 6 | 2 | · | 12 |
| `HS` | 4 | 10 | 2 | · | 16 |
| `RF` | · | 8 | 1 | · | 9 |
| `USB` | 6 | 11 | 2 | · | 19 |
| `CONN` | 6 | 11 | 2 | 1 | 20 |
| `ANA` | 8 | 11 | 3 | 1 | 23 |
| `RATE` | · | 23 | 1 | · | 24 |
| `ERR` | · | 1 | · | · | 1 |
| `DBG` | 12 | 8 | 1 | 3 | 24 |
| `TEST` | 4 | 1 | 2 | · | 7 |
| `BOM` | 11 | 5 | · | 2 | 18 |
| `NAME` | 10 | 2 | · | · | 12 |
| `LIB` | 3 | 12 | 2 | 1 | 18 |
| `LAY` | · | 9 | 14 | 2 | 25 |
| **all** | **139** | **291** | **64** | **19** | **513** |

The checks run in `erc_checks` ([docs/engine.md](docs/engine.md)), one file per check under
`erc_checks/src/<category>/`, each with its unit test and a test schematic with expectations
(`test_schematics/sources/<id>_<slug>.kicad_sch` and `.expect.toml`).

- ✅ `I2C-1` No pull-ups on a direct bus segment · `i2c/i2c_1.rs`, schematic `i2c-1_no_pull_ups`
- ✅ `I2C-2` Redundant pull-ups: besides the pair on one bus, and several pairs on one segment · `i2c/i2c_2.rs`,
  `i2c-2_multiple_pull_ups`, `i2c-7_segments`
- ✅ `I2C-3` Pull-up through series resistors to nowhere · `i2c/i2c_3.rs`, `i2c-1_no_pull_ups`
- ✅ `I2C-7` Series tie resistor too high (`max_tie` = 100 Ω) · `i2c/i2c_7.rs`, `i2c-7_segments`
- ✅ `I2C-9` Pull-up outside the 2.2k-10k window (its ends inside), or not readable · `i2c/i2c_9.rs`, `i2c-9_non_standard_pull_ups`
- ✅ `I2C-10` SCL and SDA pull-ups differ · `i2c/i2c_10.rs`, `i2c-10_non_equal_pull_ups`
- ✅ `BOM-5` Placeholder values (`?`, `??k`, `TBD`, `TODO`) · `bom/bom_5.rs`, `bom-10_values`
- ✅ `BOM-6` Placeholder value on a fitted part (folds `BOM-5` in) · `bom/bom_6.rs`, `bom-10_values`
- ✅ `BOM-9` Part without a value · `bom/bom_9.rs`, `bom-10_values`
- 🚧 `BOM-10` Resistor value that doesn't read or has odd notation · `bom/bom_10.rs`, `bom-10_values`. Missing: the
  E-series test and capacitor values
- ✅ `LIB-3` MOSFET pins not named G, D, S · `lib/lib_3.rs`, `lib-3_mosfet_pin_names`
- ⬜ `IO-1` Inputs without a drive source: collects input pins and prints them, no diagnostic yet · `erc_core/src/general.rs`,
  test schematic `io-1_inputs`
- ✅ `DIAG-1` Parts on an I2C bus in no known role are analysis notes in the report, not findings · `erc_analysis/src/i2c.rs`
- ✅ `DIAG-2` Every catalogue check reported as ran, not run (the missing need, why, and what would enable it) or
  planned; each check declares its needs, availability is worked out per board before the run · `erc_checks/src/{meta,ctx,engine}.rs`
- ✅ `DIAG-3` One `Finding` across checks: catalogue id, severity (project override), confidence (certain, estimate
  when it rests on an assumed fact, candidate), subjects with names, variant, facts with their sources, fingerprint,
  link to the detail page, fix, locations; overlapping checks fold into one finding (`also BOM-5`) · `erc_checks/src/finding.rs`
- ✅ `DIAG-4` Findings sorted worst first and grouped by category in catalogue order · `erc_checks/src/engine.rs`, `report.rs`
- ✅ `DIAG-5` The catalogue and its detail pages embedded in the library: `list()` (every check, implemented or
  planned), `explain(id)`, the not-run hints; a test holds each registered check's id, category, severity and tier to
  its catalogue line · `erc_checks/src/catalogue.rs`, `tests/catalogue.rs`
- ✅ `DIAG-6` Test schematics as the spec: `<id>_<slug>.kicad_sch` with an `.expect.toml` of findings and not-run
  checks; one test runs the engine on all of them, and fails on a schematic without expectations or a check no
  schematic shows firing (`BLESS=1` rewrites the sidecars) · `erc_checks/tests/schematics.rs`

## Configuration and waivers (`CFG`)

- 🚧 `CFG-1` Thresholds: in `erc_checks` each check declares its own with a default (`CheckMeta::params`), and
  `Config` overrides them and severities per project; the project file that sets them is `CFG-2`. `erc_core/src/config.rs`
  still holds the app's
- 📋 `CFG-2` Project file: net voltage ranges, currents, power states, I2C address choices, unknown part mapping
  (type and pin mapping)
- 📋 `CFG-3` Waivers: check, nets involved, user, comment, date; one-click waiver; voting (disagree first)
- 📋 `CFG-4` File-based config format for open source (RON?)
- 📋 `CFG-5` Custom rules for parts in the database

## Reports and calculators (`RPT`)

- 📋 `RPT-1` Decoupling capacitance per net, derated at working voltage
- 📋 `RPT-2` Quiescent / consumption current for a chosen power state
- 📋 `RPT-3` DC-DC: input and output caps, inductor value, saturation current
- 📋 `RPT-4` DC-DC power MOSFET losses
- 📋 `RPT-5` Rail sensitivity, e.g. battery on 3V3 actually going down to 3 V or lower
- 📋 `RPT-6` Voltage dividers: ratios, coefficients, multi-resistor HV dividers with max voltage
- 📋 `RPT-7` Divider with a cap: bandwidth
- 📋 `RPT-8` Zener in parallel with a voltage divider: offset graph
- 📋 `RPT-9` Filter characteristics
- 📋 `RPT-10` Op-amp: gain, mode, rail-to-rail, leakage, offset; derive transfer function
- 📋 `RPT-11` Shunt voltages with optional amplifier: resulting values and current ranges
- 📋 `RPT-12` Resistance between two nets through connectors, resistors, FETs, switches, zeners
- 📋 `RPT-13` Programmer report: GPIOs tied to power or input/output only, direction, drive strength, alternate
  configs, configurable I2C addresses, hw debounce
- 📋 `RPT-14` Logic ICs: truth tables across several ICs
- 📋 `RPT-15` Test points vs nets: power nets and buses without a test point
- 📋 `RPT-16` Netlist change report between revisions
- 📋 `RPT-17` BOM change report
- 📋 `RPT-18` Revision page: group net changes, add descriptions
- 📋 `RPT-19` Revision resistor calculator (table → alpha / numeric value)
- 📋 `RPT-20` Batch mode: run checks, generate reports and pictures from the command line
- 📋 `RPT-21` Mark findings as found by hand-written rules or by AI
- ✅ `RPT-22` Report of a run: design summary, every check ran / not run / planned, findings, candidates, notes,
  timing; JSON with a versioned schema (read back by `from_json`) and a text rendering · `erc_checks/src/report.rs`

## Views (`VIEW`)

- 🚧 `VIEW-1` Nets tab: power rails, grounds, switching nodes, all nets with node count · `tabs/nets.rs`
- ⬜ `VIEW-2` I2C tab: debug dump of the buses · `tabs/i2c.rs`
- 🚧 `VIEW-3` Style tab: style findings list · `tabs/style.rs`
- 🚧 `VIEW-4` Inputs tab · `tabs/inputs.rs`
- 📋 `VIEW-5` One-pin nets and unnamed nets, click to see where they go
- 📋 `VIEW-6` Signal tracing: pick a net, follow it through parts (needs `NET-10`)
- 📋 `VIEW-7` Power diagram
- 📋 `VIEW-8` I2C device map
- 📋 `VIEW-9` Block diagram: collapse sections into black boxes, virtual harnesses
- 📋 `VIEW-10` Board-to-board diagram and connector analyzer
- 📋 `VIEW-11` Reverse/flip insertion visualiser: which net lands where
- 📋 `VIEW-12` List of interfaces
- 📋 `VIEW-13` Schematic viewer from the netlist (floating nodes placed input to output), or on top of the imported schematic
- 📋 `VIEW-14` Power state view: shade parts that are off
- 📋 `VIEW-15` Parasitics and internal circuitry (transformers, transistors, passives, IC pins)
- 📋 `VIEW-16` Alterations: parts recognised as MCUs, pins converted to tristate, changes from config files
- 📋 `VIEW-17` Pedantic review mode: each net with its parts, expanded to the whole interface, power domains
  coloured, voltages / currents / frequency shown (derived or entered), approve or reject, comments by audience
  (to/from firmware, mechanics, EE)
- 📋 `VIEW-18` Highlight the selected net in KiCad, or in the browser for Altium 365

## App (`APP`)

- ✅ `APP-1` egui app with tiles, tabs shown and hidden from the side panel, menu bar, debug window · `better_erc/src`
- 📋 `APP-2` Sidebar with view switching; netlist load and watch status in a status bar
- 📋 `APP-3` `better_erc --version` with version, git SHA and commit time like `berc` (the app has no command line
  options at all yet), so `just deploy` can check the installed app and not only that it is on PATH

## Tools (`TOOL`)

- ✅ `TOOL-1` PnP compare between two files; on the board model it compares any two layouts (two PnP files, two
  boards, a board and its PnP file) by reference: position, rotation modulo 360, side, value, package, with
  tolerances · `erc_model/src/placecmp.rs` (for `berc diff --pnp`), old `ecad_compare/examples/pnp_compare.rs`
- 📋 `TOOL-2` Schematic vs layout match (catalogue `LAY-1`)
- ✅ `TOOL-3` Readable git diffs: `berc textconv` prints a `.kicad_sch` / `.kicad_pcb` as sorted text without UUIDs
  or coordinates (`--with-geometry` adds them), and a `.kicad_pro` as dotted-path settings with named lists and
  KiCad-default-looking settings folded per section (`--all` shows them), for git's `diff.kicad.textconv` ·
  `berc/src/sch.rs`, `pcb.rs`, `pro.rs`, README "Readable git diffs"
- ✅ `TOOL-4` `berc` CLI basics: `--version` with version, git SHA (`-dirty`) and commit time (`berc/build.rs`); shell
  completion for bash, zsh and fish (`COMPLETE=bash berc`, file paths completed), told in `--help`; colour in help
  and errors only on a terminal and never with `NO_COLOR`; `textconv` output always plain (git colours the diff).
  No progress bar: `textconv` converts one file in milliseconds · `berc/src/main.rs`, `berc/tests/cli.rs`

## Code generation (`GEN`)

- 📋 `GEN-1` GPIO init from the netlist: embassy, RTIC, raw registers
- 📋 `GEN-2` Clock config
- 📋 `GEN-3` PCB revision aware code, BSP-like crates
- 📋 `GEN-4` Config of what to generate and where

## Collaboration and commercial (`BIZ`)

- 💡 `BIZ-1` Net by net review: approval, vote, seen by
- 💡 `BIZ-2` Previous revisions: who reviewed what, likes and dislikes
- 💡 `BIZ-3` Advanced reports in a paid tier
- 💡 `BIZ-4` Paid tier with more parts in the database
