# The board model and the KiCad importer

better_erc is being rebuilt on a new core: a board model (`erc_model`) that every importer fills and every check reads,
and a KiCad importer (`erc_kicad`) that reads the schematic, project file and board itself. The checks run on it
through the check engine ([engine.md](engine.md)). The old crates (`ecad_file_format`, `erc_core`) still feed the
desktop app until it moves onto the new core.

## erc_model

### Design, boards and ids

A `Design` is one board or several boards joined through connectors (`BoardLink`). A `Board` holds arenas indexed by
small integer ids:

| arena | id | what it is |
|---|---|---|
| `sheets` | `SheetId` | a sheet instance; a sheet file used twice is two instances, the root first |
| `symbols` | `SymbolId` | a library symbol as embedded in the schematic: pins with number, name, electrical type, alternates |
| `parts` | `PartId` | one designator with all its units, fields, flags (DNP, in BOM, on board, ...) and per-variant overrides |
| `pins` | `PinId` | a pin instance on a part, with its own net, no-connect flag and chosen alternate |
| `nets` | `NetId` | a net with its full KiCad name (`/MCU/SWDIO`, `+3V3`, `Net-(R1-Pad2)`), kind and net class |
| `labels` | `LabelId` | local, global and hierarchical labels, sheet pins, power symbols, directive labels |
| `no_connects` | | no-connect flags and the pin they sit on |
| `variants`, `net_classes` | `VariantId` | KiCad 10 variants; net classes with their rules, patterns and assignments |

Adjacency goes both ways (a net lists its pins, a part lists its pins, a pin names its net), so a query is O(degree).
Names live in one place and are resolved through the board (`Board::pin_name` gives `R1.2`, `Board::sheet_path` gives
`/MCU/`). The project settings (`ProjectSettings`: text variables, ERC and DRC severities and exclusions, design rules)
and the `Layout` read from the board file hang off the board too.

The `Layout` keeps geometry in integer nanometres, so it compares exactly: the layer table, stackup and setup,
footprints (position, rotation, side, attributes, fields, courtyard, silk and fab graphics and texts in absolute
coordinates, net-tie pad groups, 3D models), pads (number, kind, shape, position, rotation, size, drill, layers, net,
test point and other properties, mask and paste margins, zone connection, custom-pad primitives), tracks and arcs,
vias, zones and rule areas, board graphics (the outline is the `Edge.Cuts` layer) and texts. Footprints link to parts by
designator and pads to nets by name (`link_layout`).

### Facts and provenance

No attribute is a bare value. A field, a part value, a DNP flag, a pin's electrical type, a net class is a
`Fact<T> { value, source, rank, at, note }`:

- `source` is a `Provenance`: a schematic sheet file, the KiCad project file, the board file, a netlist, the better_erc
  project file, the part database, mx3, the vendor, a datasheet, a distributor, an unreviewed LLM extraction, a person
  (`Manual`), an analysis rule (`Derived`) or a default (`Assumed`).
- `rank` orders sources; each kind has a default (manual 1000, project file 900, design files 800, vendor 700,
  datasheet and part database 600, distributor 500, LLM 300, derived 200, assumed 100) that a user can override.
- `at` is when the source said it (design files carry none: their time is their commit), `note` says why.

A `Facts<T>` keeps one fact per source; a source stating the attribute again replaces only its own fact.
`Facts::resolve` gives what a check sees: the highest rank, the newest on a tie, with its source, so a finding can say
"10 kΩ (manual, Roman, 5 Oct 2026)". `Facts::disagreements` lists the facts that differ from the resolved one: a
conflict between sources is reported, not silently chosen.

Identity is structure, not a fact: designators, net names, sheet paths and pin numbers are plain fields.

### Values

`Value` is the Value field parsed into a `Quantity` with a `Unit` (Ω, F, H, V, A, Hz, W) or kept as text. A bare number
with a multiplier (`10k`, `100n`) has no unit of its own: `Value::parse` keeps it as text and `Value::parse_as` reads it
in the unit of a part class decided elsewhere, so the model never guesses a resistor from the letter `k`.

### Importers

`Importer` is the one trait every importer implements: `sniff` says whether it reads a `Source` (a main file among a
set of `Files`: a directory on disk, files in memory, or a wasm GUI's dropped files), `import` returns `Imported { design,
notes }`. An `ImportNote` records what the importer read past: an unknown element of a newer format, something
missing, something invalid, something known and not kept. Nothing is silently dropped and nothing panics.

### The canonical dump

`dump::write` renders a design as text, one fact per line, sorted, deterministic; `dump::read` reads it back to the same
design. It is the golden-file format of the importer tests (a parser change shows as a readable diff), and `import →
dump → read → dump` must give the same text:

```text
part R1 lib Device:R sheet /Power/ symbol Device:R
part R1 field Value = 10k @ sch power.kicad_sch
pin R1 2 name "" unit 1 net /Power/VIN
net /Power/VIN class = Power,Default @ pro board.kicad_pro note "effective net class"
pad R1 2 kind smd shape roundrect at 110,99.175 rot 90 size 0.8,0.95 layers F.Cu,F.Paste,F.Mask net /Power/VIN
```

Entities with the same key (two `R?`, a pad number on both sides) carry `#2`, `#3` in canonical order. Coordinates and
lengths are millimetres, rotations degrees.

`netcmp::compare` compares two boards' connectivity: each net as the set of `designator.pin` it connects, matched by
that set, with renamed nets and differing net classes listed. It is the differential test's comparison and the base of
a netlist diff.

## erc_kicad

`erc_kicad::project::read` (and `KicadImporter`) reads a KiCad project from its `.kicad_pro`, its root `.kicad_sch` or
its `.kicad_pcb`: the project file and the board when they are there (the board unless `ImportOptions::no_layout`), the
schematic hierarchy, joined into one board. A board without a schematic is read alone. `NetlistImporter` reads a
`kicad-cli sch export netlist` file.

- `sexpr`: the s-expression reader every KiCad file goes through: zero-copy, iterative, never panics.
- `sch`: the hierarchy (every sheet instance with its own references) and connectivity built by KiCad's rules (KiCad
  10's behaviour; KiCad bugs of older releases are not copied): wires,
  junctions and pins by geometry, local labels per sheet instance, global labels and power symbols everywhere, KiCad 10
  local power symbols per sheet, hierarchical labels through sheet pins, vector buses (`D[0..7]`) and group buses
  (`CTL{EN RST}`, members `CTL.EN`), bus entries, bus aliases from the sheets (KiCad 8) and the project file (9, 10),
  hidden power pins, no-connect flags, alternate pin functions, KiCad's net naming (`Net-(R1-Pad2)`,
  `unconnected-(U1-PA3-Pad12)`). Where KiCad versions differ the importer follows `ImportOptions::kicad_version`
  (default: the newest format among the sheet files): KiCad 8 does not join a junction to a wire passing through it and
  gives a label on the crossing of two wires to one of them only; 9 and 10 do both. Before format 20250318 a field value
  or pin name `~` means empty; since then it is the text `~`.
- `pro`: the project file: net classes with their rules, patterns and assignments, text variables, variants, ERC and
  DRC severities and exclusions, design rules, bus aliases. Editor state (viewports, plot settings) is not kept.
- `netclass`: each net's effective net class as KiCad resolves it: the classes its directive labels name and the classes
  whose patterns match its full name (an anchored regex, or a wildcard with `*` and `?`), ordered by priority, with
  `Default` last when it fills a rule none of them sets, written as the netlist writes it (`Usb,Wide,Default`).
- `pcb`: the board, KiCad 8 and 9 (nets by code through the net table) and 10 (nets by name).
- `netlist`: the KiCad netlist, read into the same model (no geometry, no labels): the oracle of the schematic importer,
  and the importer for a board that only exists as a `.net`.

### The differential test

`erc_kicad/tests/differential.rs` compares our import with kicad-cli's netlist of the same schematic, net by net as pin
sets, names included, for every KiCad version that can read the file, and net classes against KiCad 10 (KiCad 9's
netlist writes only the classes of directive labels, KiCad 8's none):

- the fixtures in `erc_kicad/tests/fixtures` (connectivity cases on one sheet, a hierarchy with a sheet used twice and
  buses through sheet pins, KiCad 10 local power symbols, bus aliases in a sheet and in a project file with net class
  patterns) and `ecad_file_format/test_input/kicad_versions`, against their committed netlists from KiCad 8, 9 and 10
  (`just kicad-fixtures` exports them again with KiCad's container images);
- the test schematics in `test_schematics/sources`, through the installed kicad-cli;
- every KiCad project under `~/git/hw` (or `BERC_HW_DIR`) when that folder exists, through the installed kicad-cli;
  `BERC_DIFF_KICAD=8,9,10` adds the other versions through podman. Exports are cached by a hash of the project files.

Our connectivity follows KiCad 10's behaviour. Where an older release disagrees because of a bug, the importer does not
copy it and the test records the difference as known, with the KiCad release and the bug next to it: kicad-cli 9.0.9
drops buses into the second instance of a repeated sheet, and ignores the bus aliases of a KiCad 8 sheet; KiCad 8 and 10
agree on both. Rule changes between versions that are not bugs (the KiCad 8 junction and label-on-crossing rules) are
followed by `ImportOptions::kicad_version`.
Every fixture and test schematic also has a committed canonical dump (`erc_kicad/tests/golden`, `BLESS=1` rewrites
them). `erc_kicad/tests/board.rs` reads one board saved by KiCad 8, 9 and 10 and checks that all three give the same
geometry and nets, and that the board links to its schematic.

To see where our reading and KiCad's differ on any project:

```sh
cargo run -p erc_kicad --example differential -- board.kicad_sch board.net [8|9|10]
cargo run -p erc_kicad --example dump -- board.kicad_pro
```
