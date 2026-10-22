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
- `erc_analysis`: analyses over the board model: part classes, values, graph queries, the power structure, the I2C
  bus recogniser, packages from footprints
- `erc_checks`: the check engine and the checks, one file per catalogue id; findings, not-run reports, the report
  as JSON and text, the embedded catalogue ([docs/engine.md](docs/engine.md))
- `erc_import`: the importer registry, and the Altium, OrCAD and PnP importers into the board model
  ([docs/model.md](docs/model.md))
- `better_erc`: the library: the crates above re-exported, the run entry and report types at the top, `diff`
  (`TOOL-6`)
- `better_erc_app`: egui/eframe desktop app (`egui_tiles` tabs), binary `better_erc`
- `berc`: command line tool (`check`, `explain`, `checks list`, `diff --pnp`, `textconv`)
- The first crates, `ecad_file_format` (netlist model and readers), `erc_core` (`PcbAssembly`, analysis and checks)
  and `ecad_compare` (PnP compare), were removed in 0.4 once all they did ran on the crates above
- `test_schematics`: KiCad test schematics, one per check, with a netlist generator (`generate_netlists`)

## Import (`IMP`)

- ✅ `IMP-1` KiCad netlist from KiCad 8, 9 and 10, incl. pin types, net classes, units and their names; a test per
  version · `erc_kicad/src/netlist.rs`, `erc_kicad/tests/fixtures/netlist/kicad_versions`, `tests/netlist.rs`
- ✅ `IMP-2` KiCad schematic → netlist through kicad-cli, cached per file · `test_schematics/generate_netlists`
- ✅ `IMP-3` OrCAD netlist: nets, components, library parts; in the board model: pstxnet, pstxprt and pstchip found
  beside each other, sections as units with their letters, pages as sheets, pin types with the pstchip properties
  that gave them · `erc_import/src/orcad`
- ✅ `IMP-4` Altium: EDIF netlist + WireList (for pin types) combined; in the board model: either file as the main one,
  symbols, sheets, part properties, nets, pin names and types with both files as sources (the WireList's type wins)
  · `erc_import/src/altium`
- ✅ `IMP-5` PnP CSV from KiCad, Altium and Allegro; in the board model as a layout of footprints (position, rotation,
  side, value, package, the other columns as fields), mm, mil or inch · `erc_import/src/pnp.rs`
- ✅ `IMP-6` Passive values (Ω with milli/micro, k, M, G; F, H, V, A, Hz, W) with notation warnings; the whole field
  read (`Value::read`: tolerance, ratings like `50V` and `1/4W`, dielectric, a ferrite's `@100MHz`); E-series E3..E192
  (`ESeries`: membership, nearest value); the unit from the part class (`Values`) · `erc_model/src/value.rs`,
  `erc_analysis/src/value.rs`
- ✅ `IMP-7` Import tab in the app: any design the importer registry reads (a file or a directory), and the KiCad
  schematic, Altium (EDIF + WireList) and OrCAD (three files) pickers, all loading; each design's report below:
  findings with id, severity, subjects, facts and the catalogue explanation, not-run checks with their hints, notes ·
  `better_erc_app/src/tabs/pcb_data_import.rs`, `Project::open_files`
- 📋 `IMP-8` OrCAD: one BOM per variant
- 📋 `IMP-9` Pinmux import from Excel
- 📋 `IMP-10` Xilinx bank info from pin names or Xilinx ASCII pin files
- 💡 `IMP-11` Watch the netlist file and reload on change
- ✅ `IMP-12` KiCad 9 and 10 netlists load (a fixture per KiCad version, `erc_kicad/tests/fixtures/netlist/kicad_versions`,
  `just netlist-fixtures`); the tests run on test schematics in `test_schematics` instead of project files that only
  existed on one PC
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
  rule areas, outline, texts), footprints linked to the schematic parts by KiCad's schematic UUID path (designator as the fallback, a `Conflict` note where they disagree), pads to nets by name · `erc_kicad/src/{pro,pcb}.rs`
- ✅ `IMP-15` One importer registry: the importer for a file picked by its name and contents (KiCad project,
  schematic, board, KiCad netlist, Altium, OrCAD, PnP), a directory scanned for what can be imported, one cargo
  feature per importer; every import gives notes for what it read past and never panics on broken input (a test on
  truncated and garbled fixtures), and a golden dump per fixture · `erc_import/src/lib.rs`, `erc_import/tests`
- 🚧 `IMP-16` Schematic PDFs as a netlist source. Done: Altium Smart PDFs (stage 1): sheets, parts with their units,
  pins and nets from the bookmarks (`Provenance::Pdf`, `PdfConfidence::Bookmarks`, design rank), nets joined across
  sheets by power names and same-named ports, labels with their search-word position, part properties from the
  popup scripts bound by position (`PdfConfidence::Position`: findings on them are estimates), read with
  `hayro-syntax` under bounds on every walk, scripts read as text and never run, owner-password files read; the
  Xyloni against its hand-made netlist, the internal set of Altium PDFs, hostile files; `berc check board.pdf`,
  `berc diff --nets` against an Altium export. Missing: Allegro Design Publisher PDFs (stage 2: connectivity from the
  drawing, `PdfConfidence::Geometry`), popups of llPDFLib files (bound by drawn designator text), the differential
  on a real board with both a Smart PDF and its EDIF/WireList · `erc_import/src/pdf`, `erc_import/tests/pdf.rs`,
  docs/model.md "The PDF importer"

## Netlist model and analysis (`NET`)

- ✅ `NET-1` One model for every importer: first the `Netlist` of `ecad_file_format`, now the board model (`NET-17`)
- ✅ `NET-2` Graph queries: nets of parts, parts on nets, parts connected, chains of parts between two nets ·
  `erc_analysis/src/graph.rs`
- ✅ `NET-3` Analysis results per board: first `PcbAssembly`, now the engine's context (`Ctx`, built lazily) and the
  run entry `Project`; `find_part_chains` is `graph::part_chains` · `erc_checks/src/{ctx,project}.rs`
- ✅ `NET-4` I2C bus finder: direct and common segments, connected parts, voltage translators · `erc_analysis/src/i2c.rs`.
  On the parts a variant fits (`I2cBuses::on`); segments tied by resistors, ferrites, net ties and jumpers; discrete
  translators (a BSS138 pair) with their gate nets; pull-ups per segment. Buses are still found by net names; pin roles
  come with `NET-7` · test schematic `i2c-18_discrete_translator`
- 🚧 `NET-5` Power structure, first layer: rails and grounds by the power name grammar (`+3V3`, `VDD_1V8`, `-5V`,
  `5V0_NEG`, `VBUS`, `VBAT`, `+V...`, `GND_ISO`; strict mode only `+xVy`), power symbols and power pins as the second
  signal, the project file as the third (`NetSource` hook); rail kinds (supply, USB, battery, analog, reference);
  voltage from the name as a derived fact with its rule (sign from a `NEG` marker); ground groups joined through net
  ties, 0 Ω and ferrites, and the links between them · `erc_analysis/src/power.rs`, `test_schematics/analyses/power`.
  Missing: sources (LDO, DC-DC, connectors), the power tree, power switches, power states (`NET-8`)
- 🚧 `NET-6` Switching nodes finder: the net between an IC and an inductor that is no ground or rail, by part class;
  switching nodes are no rails · `erc_analysis/src/power.rs`. Missing: the regulator's SW pin by pin role (`NET-7`)
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
- ✅ `NET-16` Drop lexpr / serde-lexpr (pinned to an upstream commit of Aug 2024): KiCad files are read by our own
  s-expression reader, `erc_kicad::sexpr`; lexpr left with `ecad_file_format`
- 🚧 `NET-17` Board model (`erc_model`): arena-indexed design of boards, sheets, symbols, parts, pin instances, nets,
  labels, variants, net classes, project settings and the layout; every attribute a `Fact` with provenance, rank, time
  and note (highest rank wins, newest on a tie, disagreements listed); typed `Value`; the `Importer` trait; the
  canonical dump (one sorted fact per line, read back losslessly) as the golden format of importer tests; `netcmp`.
  Docs: [docs/model.md](docs/model.md). Missing: pin roles and the analyses on top (stage 1+)
- ✅ `NET-19` Part classes: the full enum of architecture §1.3 with kinds (diode, BJT polarity, MOSFET channel,
  connector, switch, regulator, translator, memory, transceiver, logic family, fuse), each a fact with the rule that
  decided it: a `ClassSource` (the part database by MPN, `CFG-5`, plugs in here), the library symbol, description and
  keywords, net-tie pad groups, the designator prefix as an assumption; coverage per board for `Need::PartClasses` ·
  `erc_model/src/class.rs`, `erc_analysis/src/class.rs`, `test_schematics/analyses/parts`
- ✅ `NET-20` Variant views: a board as a variant mounts it (`View`: DNP, BOM flags and field overrides applied); the
  analyses and checks read the view, `Need::Variant` · `erc_model/src/view.rs`
- ✅ `NET-21` Drawing facts: labels with their scope (local, global, hierarchical, power, local power, directive), sheet
  pins matched to hierarchical labels, no-connect flags, dangling labels, flags on connected pins, as queryable facts
  for the WIRE, NAME and SHORT checks; `Need::Drawing` · `erc_analysis/src/drawing.rs`, `test_schematics/analyses/drawing`
- 🚧 `NET-22` Pin functions by name and probe access: what a pin name states (`SWDIO`, `PA13/JTMS-SWDIO`, `~{RESET}`,
  `USART1_TX`, `SPI2_MISO`, `EN`, `nINT`, `PGOOD`, with the peripheral instance), the name layer of pin roles (`NET-7`);
  test points and connector pins on a net or one series resistor, ferrite, tie or jumper away, nets joined by 0 Ω;
  power-input pins on a net and capacitors from it to a ground · `erc_analysis/src/{pin_name,access,supply}.rs`.
  Missing: pins named by their port only (`PA13`) need alternate functions (`NET-12`)
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
- ✅ `BOM-5` Placeholder values (`?`, `??k`, `TBD`, `TODO`, `calc`) · `bom/bom_5.rs`, `bom-10_values`
- ✅ `BOM-6` Placeholder value on a fitted part (folds `BOM-5` in) · `bom/bom_6.rs`, `bom-10_values`
- ✅ `BOM-9` Part without a value · `bom/bom_9.rs`, `bom-10_values`
- ✅ `BOM-10` Resistor, capacitor or inductor value that doesn't read, has odd notation or is in no E series (with the
  nearest E24 and E96 values) · `bom/bom_10.rs`, `bom-10_values`
- ✅ `BOM-1` Resistor or capacitor values one step apart in the smallest series that has both · `bom/bom_1.rs`,
  `bom-1_values_a_step_apart` (the role filter waits for `NET-7`)
- ✅ `BOM-3` Assembly mixes: minority mount type, case size, connector family, board side · `bom/bom_3.rs`,
  `erc_analysis/src/package.rs`, `bom-3_assembly_mix`
- ✅ `BOM-4` Identical resistors on one part's pins, to a shared net or in series: array candidates · `bom/bom_4.rs`,
  `bom-4_resistor_arrays`
- ✅ `BOM-8` LED turned round: anode only to ground, cathode only to a rail · `bom/bom_8.rs`, `bom-8_led_polarity`
  (a cathode toward a high-side driver waits for `NET-7`)
- ✅ `BOM-11` One value in two packages · `bom/bom_11.rs`, `bom-11_packages`
- ✅ `BOM-14` Fitted part without an MPN · `bom/bom_14.rs`, `bom-14_mpn` (field names configurable with `CFG-2`)
- ✅ `BOM-17` One MPN with different values · `bom/bom_17.rs`, `bom-17_mpn_values`
- ✅ `LIB-3` MOSFET pins not named G, D, S · `lib/lib_3.rs`, `lib-3_mosfet_pin_names`
- ✅ `DEC-1` Rail feeding power pins with no capacitor to a ground · `dec/dec_1.rs`, `dec-1_no_decoupling`
- ✅ `DEC-4` Ferrite in series between two rails, with the LC resonance against the capacitance behind it (bead
  inductance `bead_inductance` = 1 µH assumed until part data, so an estimate) · `dec/dec_4.rs`, `dec-4_ferrite_beads`
- ✅ `DEC-7` Fewer capacitors of ≤ 1 µF (`max_decoupling`) than power pins: a rail's small capacitors handed out per
  pin, parts with the fewest pins first · `dec/dec_7.rs`, `dec-7_caps_per_pin`
- ✅ `DEC-8` Capacitors only on the source side of a series ferrite, inductor or resistor between two rails ·
  `dec/dec_8.rs`, `dec-8_source_side_caps`
- ✅ `DEC-10` A net of power pins and capacitors to ground only: nothing brings power in · `dec/dec_10.rs`,
  `dec-10_unsourced_bypass`
- ✅ `DEC-11` Ceramics on one rail ≥ 10× apart (`max_ratio`) with no value in between · `dec/dec_11.rs`,
  `dec-11_decade_apart`
- ✅ `DEC-12` Polarized capacitor with `+` on the lower voltage (pins by name, else 1 = `+`) · `dec/dec_12.rs`,
  `dec-12_reversed_polarized`
- ✅ `DBG-1` MCU, SoC or FPGA whose SWD (or JTAG) pins by name reach no connector or test point, directly or through a
  series part, or no ground does · `dbg/dbg_1.rs`, `dbg-1_swd_not_connected`
- ✅ `DBG-2` SWD/JTAG pins on the same net as a connector pin: no series resistor · `dbg/dbg_2.rs`,
  `dbg-2_swd_series_resistors`
- ✅ `DBG-3` A board with an MCU, SoC or FPGA and no LED · `dbg/dbg_3.rs`, `dbg-3_no_leds`
- ✅ `DBG-4` Rail with no LED (and its resistor) to a ground · `dbg/dbg_4.rs`, `dbg-4_power_leds`
- ✅ `DBG-7` Enable pin (`EN`, `ON`, `SHDN`) tied to a rail, driven from one place, or pulled one way only: no resistor
  footprints to both levels · `dbg/dbg_7.rs`, `dbg-7_enable_pulls`
- ✅ `DBG-8` Open-drain output (pin type, or `INT`, `ALERT`, `PG`, `FAULT` on an IC) with no pull-up footprint, fitted or
  not · `dbg/dbg_8.rs`, `dbg-8_open_drain_pull_ups`
- ✅ `DBG-10` Rail feeding more than `max_parts` = 5 parts directly · `dbg/dbg_10.rs`, `dbg-10_crowded_rail`
- ✅ `DBG-11` UART (TX, RX, RTS, CTS) or SPI data line meeting another part or a connector with no series resistor ·
  `dbg/dbg_11.rs`, `dbg-11_uart_spi_resistors`
- ✅ `DBG-14` No UART TX/RX pair of an MCU, SoC or FPGA reaches a header or test points · `dbg/dbg_14.rs`,
  `dbg-14_uart_console`
- ✅ `DBG-16` LEDs on the board, none driven by an MCU pin (directly, through a resistor or through one transistor)
  (≈ `DBG-3`) · `dbg/dbg_16.rs`, `dbg-16_heartbeat_led`
- ✅ `DBG-17` Header (or test point set) with SWDIO and SWCLK but not the MCU's reset or SWO · `dbg/dbg_17.rs`,
  `dbg-17_swd_without_reset`
- ✅ `DBG-18` I2C bus (`NET-4`) or SPI bus (by pin names) with lines that reach no test point or header ·
  `dbg/dbg_18.rs`, `dbg-18_bus_access`
- ✅ `TEST-1` Report table per IC of the pins that connect to nothing (no net, a net of their own, a no-connect flag),
  outputs first · `test/test_1.rs`, `test-1_unconnected_pins`
- ✅ `TEST-2` Two or more test points on one net, nets joined by 0 Ω, net ties and jumpers counted as one, per board side
  from the layout (without one: an estimate) · `test/test_2.rs`, `test-2_duplicate_test_points`
- ✅ `TEST-4` Rail or ground with no test point on it or on a net joined to it by 0 Ω · `test/test_4.rs`,
  `test-4_rail_test_points`
- ✅ `TEST-5` Bring-up line (UART TX/RX and SWD of an MCU by name, I2C, SPI) with no test point or header pin ·
  `test/test_5.rs`, `test-5_bring_up_test_points`
- ✅ `SHORT-1` Two-terminal part with both ends on one net, transistor with G-S, D-S or B-E on one net · `short/short_1.rs`,
  `short-1_shunted_parts`
- ✅ `SHORT-2` Chain of passives that leads back to its start through nodes nothing else uses; a translator's two sides or
  a load switch's IN/OUT on one net · `short/short_2.rs`, `short-2_name_collision`
- ✅ `SHORT-3` Fitted 0 Ω, jumper, net tie or switch across another fitted part; two fitted links from one net to two rails
  (per variant) · `short/short_3.rs`, `short-3_fitted_options`
- 🚧 `SHORT-4` Resistors in series from a rail to a ground: 0 Ω, above `max_current` (50 mA), or above a power rating the
  Value field states · `short/short_4.rs`, `short-4_low_resistance`. Missing: the package's power rating (part data, `CFG-5`)
- ✅ `SHORT-5` Supply pin on a ground, ground pin or negative supply pin on a positive rail, by pin name · `short/short_5.rs`,
  `short-5_supply_pins_swapped`
- 🚧 `SHORT-6` Bidirectional or tri-state IC pin straight on a rail or ground · `short/short_6.rs`, `short-6_io_tied_to_rail`.
  Missing: input pins of parts with configurable IO (`NET-7`)
- ✅ `SHORT-7` IO pins of MCUs, SoCs, FPGAs tied together with no other driver (I2C lines left out) · `short/short_7.rs`,
  `short-7_shorted_ios`
- ✅ `SHORT-11` Two push-pull outputs on one net (fitted parts) · `short/short_11.rs`, `short-11_output_conflict`
- ✅ `SHORT-12` P and N pins of one pair on one net; labels of both halves on one net · `short/short_12.rs`,
  `short-12_pair_halves_tied`
- ✅ `WIRE-1` Labelled net with a single pin, naming a `WIRE-2` look-alike · `wire/wire_1.rs`, `wire-1_single_pin_nets`
- ✅ `WIRE-2` Net names equal once case, separators, a leading `+` and the active-low marker style are normalised ·
  `wire/wire_2.rs`, `wire-2_similar_names`
- ✅ `WIRE-11` Auto-named net with more than `max_pins` (4) pins or pins on several sheets · `wire/wire_11.rs`,
  `wire-11_unnamed_nets`
- ✅ `WIRE-13` Output whose net reaches no input, looking through `max_hops` (3) series parts · `wire/wire_13.rs`,
  `wire-13_output_to_nowhere`
- ✅ `WIRE-14` Ground-named pin on a net that is no ground (or a ground only by such pins) · `wire/wire_14.rs`,
  `wire-14_ground_pin_not_ground`
- ✅ `NAME-1` Active-low net names in several marker styles · `name/name_1.rs`, `name-1_inverted_signal_styles`
- ✅ `NAME-2` Differential pairs in several naming styles; two halves of one pair in two styles · `name/name_2.rs`,
  `name-2_diff_pair_styles`
- 🚧 `NAME-4` UART nets named by one end, or by both ends and joining two TX or two RX pins, by pin name · `name/name_4.rs`,
  `name-4_uart_names`. Missing: pins named by their port only (`NET-12`)
- 🚧 `NAME-5` Gaps in designator series (per sheet block) on a first revision by the title block, or an unstated one ·
  `name/name_5.rs`, `name-5_designator_gaps`. Missing: the project setting that turns it on for later revisions (`CFG-2`)
- ✅ `NAME-6` Parts outside their sheet's series (page × 100 or × 1000); flat numbering on a multi-page design ·
  `name/name_6.rs`, `name-6_per_sheet_designators`
- ✅ `NAME-7` No rail named `+xVy` in strict mode · `name/name_7.rs`, `name-7_no_strict_power_names`
- ✅ `NAME-9` Numbered net family out of order against an IC's GPIO numbers · `name/name_9.rs`, `name-9_io_order`
- ✅ `NAME-10` Net on an active-low pin (marker or inversion bubble) without a marker · `name/name_10.rs`,
  `name-10_active_low_unmarked`
- 🚧 `NAME-11` Negative rail with its sign from a marker only or none; negative supply pins on a rail read positive ·
  `name/name_11.rs`, `name-11_negative_rail_names`. Missing: rails a negative regulator feeds (power sources, `NET-5`)
- ✅ `NAME-12` Unannotated designators, one designator placed twice for one unit · `name/name_12.rs`, `name-12_designators`
- ✅ `LIB-14` Unit of a multi-unit symbol not placed, with its floating pins · `lib/lib_14.rs`, `lib-14_unplaced_units`
  (schematics only: the netlist importer keeps no unit per library pin yet)
- ✅ `LIB-15` IC symbol without power pins (by type or name, hidden ones count) · `lib/lib_15.rs`, `lib-15_no_power_pins`
- ✅ `MUX-1` Interface signal on a pin named for another (SCL/SDA, MOSI/MISO, D+/D-, CANH/CANL), UART TX meeting TX ·
  `mux/mux_1.rs`, `erc_analysis/src/names/signals.rs`, `mux-1_swapped_signals` (alternates from part data with `NET-12`)
- ✅ `LOGIC-3` High fan-out: one driver on 8 inputs, a clock on 3 · `logic/logic_3.rs`, `logic-3_fan_out`
- ✅ `LOGIC-7` Open-collector or open-emitter outputs alone on a net with no pull-up or pull-down · `logic/logic_7.rs`,
  `logic-7_open_drain` (by KiCad pin types; open drain by part data and MCU pins in open-drain mode with `NET-7`)
- ✅ `LOGIC-8` Polarity mismatch by name, with the sense of the function word · `logic/logic_8.rs`,
  `erc_analysis/src/names/signals.rs`, `logic-8_polarity`
- ✅ `ANA-3` Shunt (resistor below `max_shunt` = 1 Ω) whose ends reach a current-sense amplifier's sense inputs (by
  name, directly or through one series resistor each) with no capacitor across them and no RC at the output ·
  `ana/ana_3.rs`, `ana-3_shunt_filters`
- ✅ `ANA-4` Shunt with an end on a rail or ground where a sense input sits too: suggest a net tie · `ana/ana_4.rs`,
  `ana-4_shunt_kelvin`
- ✅ `ANA-10` Op-amp channel (pins by name, KiCad's `~` output) whose output reaches its inverting input through no
  resistor, capacitor, inductor, diode or transistor, or reaches only the non-inverting one · `ana/ana_10.rs`,
  `ana-10_opamp_feedback`
- ✅ `ANA-11` Op-amp channel with an input connected to nothing, both inputs on rails, or a follower with its input on
  a supply rail (an estimate: output swing is part data) · `ana/ana_11.rs`, `ana-11_unused_channels`
- ✅ `ANA-12` Comparator channel with no resistor from the output to the non-inverting input and no hysteresis pin ·
  `ana/ana_12.rs`, `ana-12_comparator_hysteresis`
- ✅ `ANA-16` Op-amp gain network resistors outside 1 kΩ-1 MΩ (`min_resistance`, `max_resistance`), feedback
  capacitors below 1 pF (`min_capacitance`) · `ana/ana_16.rs`, `ana-16_gain_resistor_range`
- ✅ `ANA-21` Op-amp input whose DC path (resistors, inductors, ferrites, ties) reaches no rail, ground, connector or
  driving pin while a capacitor (or a transformer, an estimate) sits on it · `ana/ana_21.rs`, `ana-21_dc_return`
- ✅ `ANA-22` Divider from a rail to a ground feeding an op-amp input (directly or through one resistor) with no
  capacitor to ground · `ana/ana_22.rs`, `ana-22_bias_divider_caps`
- ✅ `CLK-3` Crystal of four pins or more with two connected and the others connected to nothing; with the board
  file, crystal footprint pads beyond the symbol's pins with no net · `clk/clk_3.rs`, `clk-3_crystal_case_pads`
- ✅ `CLK-5` Active-low reset input by name (`NRST`, `~{RESET}`, `RUN`, `CHIP_PU`; an MCU's reset) with no pull-up
  and no push-pull driver; internal pull-ups wait for part data · `clk/clk_5.rs`, `clk-5_reset_pull_ups`
- ✅ `CLK-6` Reset net with a switch on it or one series resistor away and no capacitor to ground (≈ by `CLK-7`) ·
  `clk/clk_6.rs`, `clk-6_reset_buttons`
- ✅ `CLK-7` Active-low reset net with no capacitor to ground · `clk/clk_7.rs`, `clk-7_reset_capacitors`
- ✅ `CLK-14` Reset net with a push-pull output and another driver (bidirectional or open-drain pin, connector,
  switch, a second output) · `clk/clk_14.rs`, `clk-14_push_pull_reset`
- ✅ `CLK-15` Reset net shared by an MCU, SoC or FPGA and other parts' reset inputs · `clk/clk_15.rs`,
  `clk-15_shared_reset`
- ✅ `CLK-18` Clock driver (oscillator output, `MCO`/`CLKOUT` pin, an IC output on a `CLK` net; bus clocks left out)
  straight into two or more loads · `clk/clk_18.rs`, `clk-18_clock_fan_out`
- ✅ `IO-1` Input pins (pin type) on a net nothing drives: no output, bidirectional or open-drain pin, no IC pin of
  unknown role, no rail, and no resistor, inductor, ferrite, crystal, FET channel or switch to a driven net (a
  fixpoint over the board); a connector or test point is no source; pins with a no-connect flag left out ·
  `io/io_1.rs`, `io-1_drive_sources`, `io-1_inputs`
- ✅ `IO-6` IO-1 net a test point, a connector (on it or one series part away) or a switch reaches: a touch sensor
  (≈ `IO-1`) · `io/io_6.rs`, `io-6_touch_inputs`
- ✅ `IO-7` Switch to a rail or ground with a fitted pull to that same level on its signal side · `io/io_7.rs`,
  `io-7_button_pulls`
- ✅ `IO-8` Switch to a rail or ground with no pull to the other level, and an open-drain output that is no flag or
  I2C line on an MCU net with no pull-up: the firmware needs an internal pull (errata that rule internal pulls out
  wait for part data, `CFG-5`) (≈ by `IO-7`) · `io/io_8.rs`, `io-8_internal_pulls`
- ✅ `IO-10` IO-1 net with a connector pin on it or one series part away: driven only from off the board (≈ `IO-6`) ·
  `io/io_10.rs`, `io-10_connector_inputs`
- ✅ `IO-11` MOSFET (pins G, S, D) whose gate, or a net one series resistor away, has no resistor to the source or the
  off level, and no push-pull output of a non-MCU IC drives it · `io/io_11.rs`, `io-11_gate_pulls`
- ✅ `IO-15` Interrupt, fault or power-good output by name, open-drain by pin type (or an IC pin whose type does not
  say push-pull), on a net with no pull-up and no push-pull driver · `io/io_15.rs`, `io-15_open_drain_flags`
- ✅ `IO-16` Signal net with fitted pull-up and pull-down, with the divided voltage; nets reaching an analog input by
  name, an amplifier, regulator or load switch are dividers on purpose · `io/io_16.rs`, `io-16_pull_up_and_down`
- ✅ `DIAG-1` Parts on an I2C bus in no known role are analysis notes in the report, not findings · `erc_analysis/src/i2c.rs`
- ✅ `DIAG-2` Every catalogue check reported as ran, not run (the missing need, why, and what would enable it) or
  planned; each check declares its needs, availability is worked out per board before the run from the analyses
  (part class coverage, rails, rail voltages with coverage, variant, drawing) · `erc_checks/src/{meta,ctx,engine}.rs`
- ✅ `DIAG-3` One `Finding` across checks: catalogue id, severity (project override), confidence (certain, estimate
  when it rests on an assumed fact, candidate), subjects with names, variant, facts with their sources, fingerprint,
  link to the detail page, fix, locations; overlapping checks fold into one finding (`also BOM-5`) · `erc_checks/src/finding.rs`
- ✅ `DIAG-4` Findings sorted worst first and grouped by category in catalogue order · `erc_checks/src/engine.rs`, `report.rs`
- ✅ `DIAG-5` The catalogue and its detail pages embedded in the library: `list()` (every check, implemented or
  planned), `explain(id)`, the not-run hints; a test holds each registered check's id, category, severity and tier to
  its catalogue line · `erc_checks/src/catalogue.rs`, `tests/catalogue.rs`
- ✅ `DIAG-6` Test schematics as the spec: `<id>_<slug>.kicad_sch` with an `.expect.toml` of findings (exact) and
  hand-written not-run assertions (a subset); one test runs the engine on all of them, and fails on a schematic without
  expectations or a check no schematic shows firing (`BLESS=1` rewrites the findings, keeps the not-run blocks) ·
  `erc_checks/tests/schematics.rs`

## Configuration and waivers (`CFG`)

- 🚧 `CFG-1` Thresholds: in `erc_checks` each check declares its own with a default (`CheckMeta::params`), and
  `Config` overrides them and severities per project; the project file that sets them is `CFG-2`
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
- 🚧 `RPT-16` Netlist change report between revisions. Done: `berc diff --nets A B` between two files (any two
  designs, nets matched as pin sets: renamed, removed, added, net class changes, text or JSON). Missing: between two
  git refs · `better_erc/src/diff.rs`, `erc_model/src/netcmp.rs`, `berc/tests/diff.rs`
- 📋 `RPT-17` BOM change report
- 📋 `RPT-18` Revision page: group net changes, add descriptions
- 📋 `RPT-19` Revision resistor calculator (table → alpha / numeric value)
- 🚧 `RPT-20` Batch mode: run checks, generate reports and pictures from the command line. Done: `berc check
  PROJECT` on a directory or design file (the importer registry picks KiCad, KiCad netlist, Altium, OrCAD, PnP) with
  `--variant`, `--only`, `--format text|json|sarif|md`, `--baseline`, `--fail-on`, `--require`; exit codes 0 clean,
  1 findings at or above `--fail-on`, 2 usage or import error, 3 a required need unavailable; the library entry
  `erc_checks::project` (`check_path`, `Project`, `CheckRun`, `Verdict`, typed `CheckError`). Missing: pictures
  (`just netlists` exports SVGs, berc does not); `--state` waits for power states (NET-8) · `erc_checks/src/project.rs`,
  `berc/src/main.rs`, `berc/tests/check.rs`
- 📋 `RPT-21` Mark findings as found by hand-written rules or by AI
- ✅ `RPT-22` Report of a run: design summary, every check ran / not run / planned, findings, candidates, notes,
  timing; JSON with a versioned schema (read back by `from_json`) and a text rendering (coloured on request) ·
  `erc_checks/src/report/mod.rs`
- ✅ `RPT-23` Baseline delta: a run compared with a previous JSON report marks each finding new or unchanged (by board
  and fingerprint, repeated fingerprints counted) and lists the fixed ones; with `berc check --baseline` only new
  findings fail · `Report::compare` in `erc_checks/src/report/mod.rs`
- ✅ `RPT-24` SARIF 2.1.0 output for GitHub code scanning: rule ids = catalogue ids with title and detail page as
  help, error/warning/suggestion → error/warning/note, results at the schematic file and symbol line (`sexpr::Lines`,
  `UnitPlacement::line`), partial fingerprints, baseline states, not-run checks as notifications · `erc_checks/src/report/sarif.rs`
- ✅ `RPT-25` Markdown report for a pull request comment: counts, baseline delta, findings table, fixed findings,
  candidates, not-run checks and notes folded · `erc_checks/src/report/markdown.rs`

## Views (`VIEW`)

- 🚧 `VIEW-1` Nets tab: power rails with their voltage, grounds, switching nodes, all nets with node count · `tabs/nets.rs`
- 🚧 `VIEW-2` I2C tab: the I2C findings, and the buses with net names, pull-ups, nodes and segments as text · `tabs/i2c.rs`
- 🚧 `VIEW-3` Style tab: the BOM and LIB findings · `tabs/style.rs`
- 🚧 `VIEW-4` Inputs tab: input and power input pins per symbol with the parts using it · `tabs/inputs.rs`
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

- ✅ `APP-1` egui app with tiles, tabs shown and hidden from the side panel, menu bar, debug window · `better_erc_app/src`
- 📋 `APP-2` Sidebar with view switching; netlist load and watch status in a status bar
- 📋 `APP-3` `better_erc --version` with version, git SHA and commit time like `berc` (the app has no command line
  options at all yet), so `just deploy` can check the installed app and not only that it is on PATH

## Tools (`TOOL`)

- ✅ `TOOL-1` PnP compare between two files; on the board model it compares any two layouts (two PnP files, two
  boards, a board and its PnP file) by reference: position, rotation modulo 360, side, value, package, with
  tolerances; a board against a PnP file in the PnP frame (origin page or aux, y up, footprints excluded from
  position files left out); `berc diff --pnp A B` with each side's footprint and test point count, text or JSON,
  exit 1 on a difference · `erc_model/src/placecmp.rs`, `better_erc/src/diff.rs`, `berc/tests/diff.rs`
- 📋 `TOOL-2` Schematic vs layout match (catalogue `LAY-1`)
- ✅ `TOOL-3` Readable git diffs: `berc textconv` prints a `.kicad_sch` / `.kicad_pcb` as sorted text without UUIDs
  or coordinates (`--with-geometry` adds them), and a `.kicad_pro` as dotted-path settings with named lists and
  KiCad-default-looking settings folded per section (`--all` shows them), for git's `diff.kicad.textconv` ·
  `berc/src/sch.rs`, `pcb.rs`, `pro.rs`, README "Readable git diffs"
- ✅ `TOOL-4` `berc` CLI basics: `--version` with version, git SHA (`-dirty`) and commit time (`berc/build.rs`); shell
  completion for bash, zsh and fish (`COMPLETE=bash berc`, file paths completed), told in `--help`; colour in help
  and errors only on a terminal and never with `NO_COLOR`; `textconv` output always plain (git colours the diff).
  No progress bar for `textconv` (one file in milliseconds); `berc check` and `checks list` show a spinner on stderr
  after two seconds, only on a terminal · `berc/src/main.rs`, `berc/src/progress.rs`, `berc/tests/cli.rs`
- ✅ `TOOL-5` The catalogue from the shell: `berc explain ID` (the detail page section of any catalogue check as plain
  text or Markdown) and `berc checks list` (every catalogue id, `--tier`, `--status ran|not-run|planned` with
  `--project`, text or JSON); completion of check ids, categories, needs and the variants of the design in the
  current directory, each with a help text · `berc/src/main.rs`, `berc/src/complete.rs`, `erc_checks::list_for`
- 🚧 `TOOL-6` The library facade: crate `better_erc` re-exports `erc_model`, `erc_kicad`, `erc_import`,
  `erc_analysis` and `erc_checks`, with the run entry (`Project`, `check_path`, `CheckRun`, `Verdict`) and the report
  types at the top and `diff::pnp`; berc and the app depend on it alone. Missing: `Workspace` and a watching
  `Session` (architecture 5.2), queries, waivers · `better_erc/src/lib.rs`

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
