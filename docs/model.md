# The board model, the importers and the crates

better_erc's core is a board model (`erc_model`) that every importer fills and every check reads, a KiCad importer
(`erc_kicad`) that reads the schematic, project file and board itself, and the other importers behind one registry
(`erc_import`). The analyses (`erc_analysis`) and the checks (`erc_checks`, [engine.md](engine.md)) run on it.

## The crates

| crate | what it is | depends on |
|---|---|---|
| `erc_model` | the board model, values, facts and provenance, the importer trait, the canonical dump, `netcmp`, `placecmp` | serde |
| `erc_kicad` | KiCad projects, schematics, boards and netlists; the s-expression reader | `erc_model` |
| `erc_import` | the importer registry; Altium, OrCAD, PnP and schematic PDFs (one cargo feature each) | `erc_model`, `erc_kicad` |
| `erc_analysis` | part classes, values, graph queries, the power structure, the I2C bus recogniser, pin functions by name, probe access, supply pins, pulls, amplifier pins by name, connector contacts (shell pins, Type-C), interface pins by name (USB, MDIO, CAN, SD, DDR, lanes) | `erc_model` |
| `erc_checks` | the check engine, the checks, the catalogue, reports; `project`, the run on a path | the above |
| `better_erc` | the library: all of the above re-exported, the run entry at the top, `diff` (`berc diff`) | the above |
| `berc` | the command line tool | `better_erc` |
| `better_erc_app` | the egui app, binary `better_erc` | `better_erc` |

One dependency on `better_erc` gives a Rust user the whole core: `better_erc::model`, `kicad`, `import`, `analysis`
and `checks`, and at the top `Project`, `check_path`, `CheckOptions`, `CheckRun`, `Report`, `Finding`, `Verdict`
([engine.md](engine.md) "A run on a path"). A `Workspace` that also watches the files and re-runs (a session for the
MCP server and the GUI) is not built yet; `Project` is the entry until then.

## erc_model

### Design, boards and ids

A `Design` is one board or several boards joined through connectors (`BoardLink`). A `Board` holds arenas indexed by
small integer ids:

| arena | id | what it is |
|---|---|---|
| `sheets` | `SheetId` | a sheet instance; a sheet file used twice is two instances, the root first; with its page number and its title block revision (`rev`) |
| `symbols` | `SymbolId` | a library symbol as embedded in the schematic: pins with number, name, electrical type, alternates |
| `parts` | `PartId` | one designator with all its units, fields, flags (DNP, in BOM, on board, ...) and per-variant overrides |
| `pins` | `PinId` | a pin instance on a part, with its own net, no-connect flag and chosen alternate |
| `nets` | `NetId` | a net with its full KiCad name (`/MCU/SWDIO`, `+3V3`, `Net-(R1-Pad2)`), kind and net class |
| `labels` | `LabelId` | local, global and hierarchical labels, sheet pins, power symbols, directive labels; the position when the source gives one |
| `no_connects` | | no-connect flags and the pin they sit on |
| `variants`, `net_classes` | `VariantId` | KiCad 10 variants; net classes with their rules, patterns and assignments |

Adjacency goes both ways (a net lists its pins, a part lists its pins, a pin names its net), so a query is O(degree).
Names live in one place and are resolved through the board (`Board::pin_name` gives `R1.2`, `Board::sheet_path` gives
`/MCU/`). The project settings (`ProjectSettings`: text variables, ERC and DRC severities and exclusions, design rules)
and the `Layout` read from the board file hang off the board too. `SourceInfo` says what the board was read from: the
tool (`kicad`, `kicad-netlist`, `altium`, `altium-pdf`, `orcad`, `pnp`), its version, the files, and for a board whose
connectivity is not design data, where it came from (`connectivity`: a Smart PDF's bookmarks, or one day a PDF's
drawing).

The `Layout` keeps geometry in integer nanometres, so it compares exactly: the layer table, stackup and setup,
footprints (position, rotation, side, attributes, fields, courtyard, silk and fab graphics and texts in absolute
coordinates, net-tie pad groups, 3D models), pads (number, kind, shape, position, rotation, size, drill, layers, net,
test point and other properties, mask and paste margins, zone connection, custom-pad primitives), tracks and arcs,
vias, zones and rule areas, board graphics (the outline is the `Edge.Cuts` layer) and texts. Footprints link to parts by
their schematic UUID path (`Footprint::schematic_path`, KiCad's own link, so a re-annotation does not break it), by
designator when there is no path or it names no symbol, and pads to nets by name (`link_layout`). The KiCad reader
gives a `Conflict` import note where the two disagree.

### Facts and provenance

No attribute is a bare value. A field, a part value, a DNP flag, a pin's electrical type, a net class is a
`Fact<T> { value, source, rank, at, note }`:

- `source` is a `Provenance`: a schematic sheet file, the KiCad project file, the board file, a pick-and-place file, a netlist, a
  schematic PDF (`Pdf { file, confidence }`: read from its bookmarks, bound by position, or traced from its drawing),
  the better_erc project file, the part database, mx3, the vendor, a datasheet, a distributor, an unreviewed LLM extraction, a person
  (`Manual`), an analysis rule (`Derived`) or a default (`Assumed`).
- `rank` orders sources; each kind has a default (manual 1000, project file 900, design files and a PDF's bookmarks
  800, a PDF's facts reconstructed by position or geometry 750, vendor 700,
  datasheet and part database 600, distributor 500, LLM 300, derived 200, assumed 100) that a user can override.
- `at` is when the source said it (design files carry none: their time is their commit), `note` says why.

A `Facts<T>` keeps one fact per source; a source stating the attribute again replaces only its own fact.
`Facts::resolve` gives what a check sees: the highest rank, the newest on a tie, with its source, so a finding can say
"10 kΩ (manual, Roman, 5 Oct 2026)". `Facts::disagreements` lists the facts that differ from the resolved one: a
conflict between sources is reported, not silently chosen. `Provenance::is_estimate` marks the sources a finding can
only estimate from (an assumption, a reconstructed PDF fact): [engine.md](engine.md) "Certainty".

Identity is structure, not a fact: designators, net names, sheet paths and pin numbers are plain fields.

### Values

A part's `value` is stated by the importer from the tool's convention (KiCad's `Value` field; Altium's `Value`, else
`Comment`), with a note naming the field; the checks read it, never a field by name ([engine.md](engine.md) "Values").
`Value` is that text parsed into a `Quantity` with a `Unit` (Ω, F, H, V, A, Hz, W) or kept as text. A bare number
with a multiplier (`10k`, `100n`) has no unit of its own: `Value::parse` keeps it as text and `Value::parse_as` reads it
in the unit of a part class decided elsewhere, so the model never guesses a resistor from the letter `k`.

`Value::read(text, unit)` reads the whole field into a `Reading`: the value (multipliers p to G, RKM notation like
`4k7`, `2R2`, `3V3`, a sign, `1/4W` fractions), then what follows it: the tolerance in percent (`1%`, `±5 %`), ratings
with their own unit (`50V`, `0.1W`), a ceramic dielectric (`X7R`, `C0G`) and, after `@`, a ferrite's test frequency
(`600R@100MHz`). What still parsed but is odd is a `ValueWarning`: trailing text, a unit other than the expected one, a
decimal comma, extra spacing, a small `r`, a redundant `R` (`10kR`), the unit as a separate word (`4k7 Ohm`). BOM-10
reports these.

`ESeries` is the IEC 60063 preferred number series E3 to E192: `ESeries::of(value)` gives the smallest series that has a
value (`None` for `5k` or `4.8k`), `contains` and `nearest` test and round in any decade.

### Part classes

`PartClass` (`erc_model::class`) is what kind of part a designator is, the enum of the architecture §1.3: passives
(resistor, capacitor with `polarized`, inductor, ferrite bead), diodes with their kind, BJTs with their polarity,
MOSFETs with their channel, connectors and switches with their kind, relays, crystals and oscillators, regulators with
their kind and load switches, level translators with their class, isolators, MCUs, SoCs, FPGAs, memories and
transceivers with their kind, op-amps and comparators, logic with its family, fuses with their kind, batteries,
transformers, net ties and jumpers, test points, fiducials and mounting holes, and `Transistor`, `Ic` or `Unknown` where
the kind is not known. A kind the source does not tell is `None`, never a guess. `PartClass::value_unit` gives the unit
a Value field is read in (Ω for a resistor or ferrite, F, H, Hz for a crystal, A for a fuse, V for a Zener or TVS), so
the class decides it. `erc_analysis::class` resolves it
([engine.md](engine.md) "The analyses").

### Variant views

`View` (`Board::view(variant)`) is a board as one variant mounts it: `fitted`, `in_bom`, `field` and `value_text`
apply the variant's DNP flags and field overrides (KiCad 10 variants, OrCAD per-variant BOMs); `View::base` is the
design without a variant. Analyses and checks that care about what is placed read the view, not the board.

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
that set, with renamed nets and differing net classes listed. It is the differential test's comparison and `berc diff
--nets`.

`placecmp::compare` compares two layouts' placements: each footprint by its reference, its position, rotation (modulo
360), side, value and package (the name without its `library:` prefix), within optional tolerances. The layouts can
come from board files or pick-and-place files, so two PnP files, two boards, or a board and its PnP file compare the
same way; it is the PnP compare of `berc diff --pnp`. A board file counts from the page's top left corner with y
down, a PnP file from an origin with y up, so `compare_boards` brings a board compared with a PnP file into the PnP
frame first (`to_placement_frame`): relative to `Origin::Page` (KiCad's default) or `Origin::Aux` (the board's
`aux_axis_origin`, KiCad's `--use-drill-file-origin`), y negated, rotations unchanged, footprints marked
`exclude_from_pos_files` left out: what kicad-cli's `pcb export pos` writes. `better_erc::diff::pnp` opens the two
sides by path, counts each side's footprints and test points, and renders the result as text or JSON.

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
  patterns) and `erc_kicad/tests/fixtures/netlist/kicad_versions`, against their committed netlists from KiCad 8, 9 and 10
  (`just kicad-fixtures` exports them again with KiCad's container images);
- the test schematics in `test_schematics/sources`, through the installed kicad-cli;
- every KiCad project under `~/git/hw` (or `BERC_HW_DIR`) when that folder exists, through the installed kicad-cli;
  `just test` compares a sample of six of them, `just test-full` (`BERC_HW_FULL=1`) all; `BERC_DIFF_KICAD=8,9,10` adds the other versions through podman. Exports are cached by a hash of the project files.

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

## erc_import

`erc_import::Registry` holds every importer: `Registry::default()` registers KiCad's (`erc_kicad::importers()`: the
project and the netlist), Altium, OrCAD, PnP and PDF, and `register` adds others. `pick` asks each importer to sniff the
main file and takes the surest (an earlier one on a tie), `import` imports with it, and `scan` lists every file of a
directory or file set that some importer reads. The dependency goes `erc_import` → `erc_kicad` → `erc_model`: the
trait lives in the model, so `erc_kicad` stands alone, and each importer is a cargo feature (`kicad`, `altium`,
`orcad`, `pnp`, `pdf`), so a KiCad-only tool or a wasm GUI builds without the rest.

- `altium`: the EDIF netlist ("EDIF for PCB") and the WireList, either one as the main file, the other found beside
  it (same name first, then the only file of its kind in the directory). EDIF gives the library symbols with their
  ports, the sheets, the parts with all their properties (the value from `Value`, else `Comment`) and the nets; the
  WireList gives pin names, pin types and footprints. A pin's type is stated by both files: EDIF's port direction
  (`INOUT` for nearly everything) ranks one below the design rank, so the WireList's type is the one a check sees.
  Altium's `POWER` type is read as `power_in` with a note saying the direction is not given. A pin the two files put on
  different nets is a note.
- `orcad`: OrCAD Capture's pstxnet, pstxprt and pstchip, any of them as the main file and the others found by name. The
  primitives of pstchip are the symbols (their body properties the parts' fields, `VALUE` as `Value` and `JEDEC_TYPE`
  as `Footprint`); a multi-section pin number (`(A3,0,0)`) gives one pin per section it is in; the part list's sections
  are the units, named by their section letters, on the sheet of their page (`/page3/`); pin types come from the pin
  properties (`BIDIRECTIONAL`, `PINUSE`, `OUTPUT_LOAD`, `INPUT_LOAD`), which the fact's note names. Nets named `N`
  plus digits are Capture's automatic names.
- `pnp`: pick-and-place CSV from KiCad, Altium and Allegro as a board with a layout only: one footprint per row with
  reference, position, rotation, side, value and package, the other columns as its fields (`Provenance::Placement`).
  The header row is the first naming five of the known columns, the separator the one the file uses most; positions
  are millimetres unless the header or the file information says mil or inch, kept as the file writes them.
- `pdf`: an Altium Smart PDF, the schematic print Altium writes with its netlist in the bookmarks, as a board with
  parts, pins, nets, sheets, labels and ports (tool `altium-pdf`, `Provenance::Pdf`). See [The PDF importer](#the-pdf-importer).

### The PDF importer

A schematic PDF lets better_erc check a vendor's board that comes as nothing else. An Altium Smart PDF has, per
schematic sheet, a bookmark `File.SchDoc(Name)` with `Components` (each unit, `U1A`, with its pin bookmarks `U1-C5`),
`Nets` (each net with its `Pins`, `NetLabels` and `Ports`) and `Ports`, and every pin, label and port bookmark runs a
script `_FindWord(page, "PIU10C5")` that names a hidden search word drawn at that spot. The importer reads:

- sheets from the bookmarks, nested as the bookmarks nest; a bookmark with Components but no `.SchDoc` (the Top
  Assembly or Multilayer Composite Print of a PCB print) is left out with a note;
- parts and pins from Components, the pins of each net from Nets (`PdfConfidence::Bookmarks`, ranked as a design
  file: this is Altium's netlist), labels with the position of their search word when there is one;
- nets across sheets: Altium names a net per sheet (a port does not name the net, so one net can be `NetR87_1` on one
  sheet and `NetSB45_2` on another); nets join by name only where no label names them (power ports), and through
  ports of one name among sibling sheets and with the parent's net of that name. The wires between sheet entries on
  a parent sheet are not in the bookmarks, so same-named entries are taken as wired, and a note says how many nets
  that joined;
- part properties from the popup each part's link opens (`ShowCompProps_<guid>()` → `app.popUpMenu("Value: 10k",
  ...)`), bound to its part by the hidden `CO`/`PI` words inside the link's rectangle (`PdfConfidence::Position`,
  rank 750: a finding resting on one is an estimate). A popup no part can be tied to is a note: some producers
  (llPDFLib) draw every search word at one point, and binding by the drawn designator text is stage 2.

The PDF is untrusted input: it is parsed by `hayro-syntax` (pure Rust, lazy, `None` instead of a panic) and every walk
is bounded (file size, bookmarks, nesting, links, chained actions, script length, content operations, form drawings,
words); a cut walk is a note. Scripts are read as text for their string and number literals, never run. Encrypted
files with an owner password only read as plain ones; a user password is an error. The not-run list says what a PDF
cannot give: no layout, no wires and no no-connect flags. Values are there when the popups are.

An Allegro (Design Publisher) PDF carries part attributes and label positions in its link scripts but no netlist:
its connectivity has to be traced from the drawing (wire segments, junction dots, pin stubs), stage 2 of the PDF
importer, `PdfConfidence::Geometry`. Until then such a PDF is offered as a maybe and its import says why it is not
read.

The tests (`erc_import/tests/pdf.rs`, `erc_import/src/pdf/hostile.rs`) read a synthetic Smart PDF that our own
generator writes (`tests/common/smart_pdf.rs`: sheets in sheets, units on two sheets, a bus, ports across siblings
and to the parent, popups bound by pins and inside a form XObject, a UTF-16 title), its encrypted copies, hostile
files (loops in bookmarks, actions and name trees, 10 000-deep outlines, 100 000 bookmarks, forms that draw
themselves or each other 10^9 times, a 5 MB script), and, when present on the PC (`BERC_PDF_DIR`, default
`~/sync`), the Altium Smart PDFs of the internal set, the Xyloni against its hand-made netlist. Vendor PDFs and their
dumps stay out of the public repo.

The Altium and OrCAD readers are pest grammars (`erc_import/grammar`) that take any unknown element or line as such
and report it, instead of failing the file. The tests (`erc_import/tests`) import every fixture through the registry
against a committed dump and notes (`BLESS=1` rewrites them), assert what the readers' tests of the removed `ecad_file_format`
asserted, and feed truncated and garbled copies of every fixture to every importer: an import fails with an
error or gives a design that dumps and reads back, it never panics.
