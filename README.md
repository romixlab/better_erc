# better_erc

Electrical rule checks for schematics that go further than the ERC built into an EDA tool: checks that know
what an I2C bus, a pull-up, a MOSFET or a power rail is, read from the netlist your EDA tool exports. Plus
`berc textconv`, which makes `git diff` of KiCad files readable.

For hardware engineers who review schematics (their own or someone else's) and want the repetitive part of a
review done by a tool, and for teams that keep KiCad projects in git.

The project is early. The KiCad diff tool is ready for daily use; the checks run from the shell and in CI with
`berc check` and as a Rust library; the desktop app is a first prototype.

## What works today

- **`berc textconv`**: `.kicad_sch`, `.kicad_pcb` and `.kicad_pro` files as sorted, line-oriented text for
  `git diff`, `git log -p` and anything else that uses git's textconv. See
  [Readable git diffs](#readable-git-diffs).
- **Import** into one board model ([docs/model.md](docs/model.md)): KiCad 8, 9 and 10 projects, schematics, boards
  and netlists read by better_erc itself (`erc_kicad`), OrCAD Capture netlists (pstxnet, pstxprt, pstchip), Altium
  EDIF plus WireList, Altium Smart PDFs (the netlist in their bookmarks: vendor boards that come as nothing but a
  PDF), and pick-and-place CSV files from KiCad, Altium and Allegro (`erc_import`), all picked by one importer
  registry.
- **Checks** (`erc_checks`), run on a KiCad project read by better_erc itself (`erc_kicad`), with one finding
  type, a report of which checks ran and which could not run for lack of data, and the report as JSON or text
  ([docs/engine.md](docs/engine.md)):
  - I2C buses found from net names, including segments behind ties and voltage translators; missing, redundant,
    unequal or out-of-range pull-ups, pull-ups through series resistors to nowhere, series resistors that are
    too large (`I2C-1`, `I2C-2`, `I2C-3`, `I2C-7`, `I2C-9`, `I2C-10`)
  - parts without a value, placeholder values like `?` or `TBD` (fitted or not), resistor values that don't read
    or are written in a non-standard way (`BOM-5`, `BOM-6`, `BOM-9`, `BOM-10`)
  - MOSFET symbols whose pins aren't named G, S and D (`LIB-3`)
- **`berc check`**: the checks on a KiCad project (or a KiCad netlist, Altium, OrCAD, Smart PDF or PnP file) from the shell
  or a CI job, as text, JSON, SARIF (GitHub code scanning) or Markdown (a PR comment), with an exit code by
  severity and a baseline so only new findings fail. `berc explain ID` and `berc checks list` show the catalogue.
  See [Checks in CI](#checks-in-ci).
- **`berc diff --pnp A B`**: two pick-and-place files, two boards, or a board and the PnP file exported from it
  compared footprint by footprint (position, rotation, side, value, package), with tolerances.
- **`berc diff --nets A B`**: the nets of any two designs (a schematic, a netlist, an Altium export, a Smart PDF)
  compared as the pins each net connects, whatever their names: renamed, removed and added nets.
- **`berc fp`**: footprint review. `berc fp hash` prints a footprint's review hash, the same for a library file and
  every board copy of it; `berc fp check` checks every board footprint against the hash on its library symbol (and
  against its library footprint), `berc fp check --lib` every symbol of a symbol library against its library
  footprint, and `berc fp todo` lists the library symbols a board uses that need a review; `berc fp model` reads
  each footprint's 3D models and measures them against it. See [Footprint review](#footprint-review).
- **The app** (`better_erc`, egui, crate `better_erc_app`): an import tab that opens any design the registry reads
  (and Altium or OrCAD files picked one by one) and shows its findings with their explanations and the checks that
  did not run, and tabs for nets (power rails, grounds and switching nodes from names and pin types), I2C buses,
  style findings and inputs; a footprint review tab that draws a library symbol's footprint over its datasheet
  drawing and writes the review hash into the symbol (see [Reviewing in the app](#reviewing-in-the-app)).
- **Signal levels** (crate `erc_levels`): a database of the levels logic families, buses and differential
  standards define (74xx and CD4000, JEDEC LVCMOS/SSTL/HSTL, I2C, RS-485, CAN, LVDS, ECL, USB, Ethernet and more),
  every number with the document, table and page it was read from. See [docs/signal-levels.md](docs/signal-levels.md).
- **The library** (crate `better_erc`): all of the above for Rust code in one dependency.

## Documentation

- [docs/checks.md](docs/checks.md): the check catalogue, everything better_erc should catch on a schematic or
  netlist, by ID (`PIN-1`, `I2C-3`) and severity.
- [docs/engine.md](docs/engine.md): the check engine: checks and their data needs, findings, the report, the test
  schematics with their expectations, how to add a check.
- [docs/model.md](docs/model.md): the board model, the importers and the crates.
- [docs/drawing-guide.md](docs/drawing-guide.md): how to draw a sheet that reads at a glance (signal flow, power and
  ground, grid, labels, text, title block, KiCad shortcuts), each rule with its reason and source, and our own sheets
  measured against it; the `DRAW` checks automate part of it.
- [docs/signal-levels.md](docs/signal-levels.md): the signal-level database: its format, where each number comes
  from, what `verified` means, and how to add a family.
- [docs/review-checklist.md](docs/review-checklist.md): a condensed schematic review checklist that groups those
  IDs for a review pass.
- [FEATURES.md](FEATURES.md): what the code does today and what is planned, item by item.
- [CHANGELOG.md](CHANGELOG.md): what changed in each version.

## Install

You need a Rust toolchain (1.86 or newer, from [rustup](https://rustup.rs)). From a clone of this repository, with
[just](https://github.com/casey/just): `just install` works anywhere and builds both binaries; with our release
tool on PATH it uses that (one copy kept on PATH), otherwise `cargo install`. Or by hand:

```sh
cargo install --locked --path berc             # the berc command line tool
cargo install --locked --path better_erc_app   # the desktop app (binary better_erc)
```

`berc --version` shows the version,
the git commit it was built from and that commit's time.

## Quick start

Readable diffs of a KiCad project in git, once per machine and once per repository:

```sh
git config --global diff.kicad.textconv "berc textconv"
printf '*.kicad_sch diff=kicad\n*.kicad_pcb diff=kicad\n*.kicad_pro diff=kicad\n' >> .gitattributes
git diff   # changed parts, values, nets and settings instead of raw s-expressions
```

Run the checks on a KiCad project (its directory, or the `.kicad_pro` / `.kicad_sch`):

```sh
berc check hw/board                  # findings worst first, the checks that could not run and why
berc explain I2C-3                   # what a check catches, why it matters, how to fix it
berc checks list --project hw/board  # every catalogue check: ran, not run (and why), planned
```

From Rust, the same run is `erc_checks::check_path` (or `erc_checks::Project::open` and `check`), see
[docs/engine.md](docs/engine.md).

Start the app with `just run` (or `better_erc` once installed).

Shell completion for `berc`: add `source <(COMPLETE=bash berc)` to `~/.bashrc`, `source <(COMPLETE=zsh berc)`
to `~/.zshrc`, or `COMPLETE=fish berc | source` to `~/.config/fish/config.fish`.

## Checks in CI

```sh
berc check PROJECT [--variant V] [--only CAT,ID] [--format text|json|sarif|md]
                   [--baseline report.json] [--fail-on error|warning|suggestion] [--require NEED]
                   [--set NAME=VALUE] [--theme THEME.json]
```

`PROJECT` is a directory with one design in it (its `.kicad_pro`) or a design file. Exit codes:

| code | meaning |
|---|---|
| 0 | no finding at or above `--fail-on` (default `error`) |
| 1 | findings at or above `--fail-on`; with `--baseline`, only findings that are not in the baseline count |
| 2 | usage or import error: no such path, several designs in the directory, an unreadable file, an unknown variant or check, a baseline of another schema |
| 3 | a `--require`d need is not available on a board (`--require layout` without a `.kicad_pcb`); wins over 1 |

`--format json` writes the report with its versioned schema; that file is also what `--baseline` takes: findings are
matched by board and fingerprint (check id and the names of the nets and parts involved), marked new or unchanged,
and the fixed ones listed. `--format sarif` writes SARIF 2.1.0: one rule per catalogue check, error → `error`,
warning → `warning`, suggestion → `note`, each finding located at the schematic file and the line of its symbol
(the file alone where no line is known). `--format md` is a summary for a pull request comment. `--state` is
reserved for power states and refused until they exist.

A GitHub Actions job that fails on errors, shows the findings in code scanning on the schematic files and puts the
Markdown summary on the run page:

```yaml
name: schematic checks
on: [push, pull_request]
jobs:
  berc:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write # upload SARIF to code scanning
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --locked --git https://github.com/romixlab/better_erc berc
      - name: berc check
        run: berc check hw/board --format sarif > berc.sarif
      - name: Summary
        if: always()
        run: berc check hw/board --format md >> "$GITHUB_STEP_SUMMARY" || true
      - name: Upload to code scanning
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: berc.sarif
          category: better_erc
```

Run it from the repository root with a relative project path: the SARIF file names the schematic files relative to
the current directory, which is what code scanning expects. To adopt better_erc on a board with known findings,
commit a baseline (`berc check hw/board --format json > hw/board/berc-baseline.json`) and add
`--baseline hw/board/berc-baseline.json`: from then on only new findings fail the job.

In this repository `just check` runs `berc check` on the test schematics against the committed
`test_schematics/berc-baseline.json` (they have findings on purpose), and `just check-baseline` rewrites it.

## Footprint review

A footprint checked once against its datasheet should not have to be checked again on the next board, unless it
changed. `berc fp` keeps that record as a hash on the library symbol.

`berc fp hash` prints a footprint's review hash, `fp1:` and 16 hex digits:

```sh
berc fp hash Lib.pretty/SOT-23.kicad_mod          # a library footprint
berc fp hash board.kicad_pcb U3                   # a footprint on a board, by its reference
berc fp hash board.kicad_pcb U3 --canonical       # and the text the hash is taken over
```

The hash covers what a review checks against the datasheet:

- every pad: number, type, shape, size, position, rotation, drill and its offset, layers, mask, paste, clearance and
  thermal settings, corner and chamfer ratios, custom pad shapes;
- the footprint's clearance, mask and paste settings;
- graphics on copper, mask, paste, courtyard and `Edge.Cuts` layers, zones and keepouts.

Silk, fab and user layers, texts, the 3D model and where the footprint sits on a board don't count. A board copy is
brought back to the footprint's own frame first (position, rotation and a flip to the back undone), so it has the hash
of its library file. Lengths are compared on a 100 nm grid and items in a fixed order, so a re-save in another KiCad
version or a reorder never changes the hash. The `fp1:` prefix is the version of these rules: a later change of them
gives `fp2:` hashes, and an `fp1:` hash then reads as made by another version instead of as a mismatch. The hex digits
are the first 8 bytes of the SHA-256 of the canonical text (`--canonical`, lines joined by a newline): a short
fingerprint to tell a footprint unchanged, not a signature.

Once a footprint is reviewed, `berc fp accept` puts its hash in the `Footprint_Review` field of the library symbol
that uses it (see [Accepting a review](#accepting-a-review)), or the app's Accept does. `berc fp check` then checks a
board:

```sh
berc fp check                       # the KiCad project in this directory
berc fp check hw/board --format json
```

For each footprint on the board it reads the review hash from its symbol's library symbol: in the `.kicad_sym`
found through the project's or the global `sym-lib-table`, else in the schematic's copy of the library symbol. Each
footprint is one of:

| Status | Meaning |
|---|---|
| ok | the footprint is the one reviewed |
| changed | the footprint differs from the one reviewed: review it again (with several review lines, the stale ones are named) |
| partial | every review line's hash matches, but some of the symbol's MPNs are on no line: their datasheets are not reviewed yet, named |
| unreviewed | the symbol has no review hash; the line shows the hash to put there once reviewed |
| refused | the symbol sets no footprint, so a review of the one on the board means nothing |
| no symbol | the footprint has no schematic symbol and is not marked board-only |
| board-only | no symbol, marked board-only in KiCad (logos, mounting holes): listed last under its own heading, not a failure |
| other version, malformed | the field holds a hash of another version, or no hash at all |

A schematic copy that holds a different hash than the library is reported as well: the schematic was not updated
from the library. So is a board copy that differs from its library footprint (the footprint the board names, found
through `--footprints` and the `fp-lib-table`; a board footprint without a library nickname is looked up by its
symbol's `Footprint` field): the line shows both hashes, and the board was not updated from the library.

A board footprint placed under a different `Lib:Name` than its symbol's own `Footprint` field (a library migration
half done, or a deliberate per-instance override) is reported too, naming both: a library migration that renamed or
moved a footprint library, left half done, or a board edited by hand without updating the schematic, looks exactly
like this. A footprint placed with no library nickname at all is not an override, there being nothing to compare its
name against. Footprints that could not be compared are summed up once per reason below the list. `--symbols DIR` adds symbol libraries (a
`.kicad_sym`, a `.kicad_symdir` or a folder of them, each named by its file name) searched before the
`sym-lib-table`, and `--footprints DIR` footprint libraries searched before the `fp-lib-table`, for a PC without
KiCad's global tables:

```sh
berc fp check hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints
```

The exit code is 0 when every footprint is ok or board-only and every copy up to date, 1 otherwise, 2 when the
project can't be read.

### Review lines: where a review was made

A review is made against one datasheet, and a datasheet is one maker's: the MPNs of one maker usually share one, and
two makers never do. So a symbol holds one review line per datasheet, each in a property of its own,
`Footprint_Review`, `Footprint_Review2`, `Footprint_Review3`, ... (KiCad needs field names unique, as with `Mpn1`,
`Mpn2`). Each is one line, the review hash first and where the review was made after it:

```text
fp1:7726812df851763a mpn=BSS138W-7-F,BSS138WQ-7-F doc=DS30154@rev16 pdf=b3:0123456789abcdef page=3 scale=0.02 at=150,230,90,mirror by=roman on=13nov2026
```

| Key | What |
|---|---|
| `mpn=` | the MPNs this datasheet covers, from the symbol's `Mpn`, `Mpn1`, `Mpn2`, ... fields, comma-separated; a comma, `%` or blank inside one is written `%2C`, `%25`, `%20` (Nexperia's `BSS138BKW,115` is `BSS138BKW%2C115`). No `mpn=`: the line reviews the whole symbol |
| `doc=` | the document number and revision, where the document library knows them: they survive a download of the same document with other bytes |
| `pdf=` | the PDF by the document library's content hash: `b3:` and 16 hex digits of its blake3 |
| `page=` | the page the land pattern is on, 1-based |
| `scale=` | the drawing's scale on that page, mm per PDF point |
| `at=` | where the footprint lay on the page: its origin in page points from the page's top left, its rotation in degrees, `mirror` when seen from below |
| `by=`, `on=` | who reviewed it, and the day |

The MPNs are written as strings, so reordering the `Mpn` fields breaks nothing. A key this version doesn't know stays
on the line as it is. `berc fp check` reads the first token of each line as its hash: a symbol is ok when every line's
hash is the footprint's and every MPN of the symbol is on some line (a symbol without `Mpn` fields needs a line
without `mpn=`); otherwise it lists the MPNs not reviewed yet (partial) and the stale lines (changed). A value with
the hash alone, as written before lines had a source, stays valid: a review of the whole symbol with no source
recorded. `--details` prints every line under its footprint or symbol, with its status:

```sh
berc fp check hw/board --details
berc fp check --lib Device --details
```

`--format json` holds each line read (`reviews.lines`: property, value, its parts, status) and the MPNs not reviewed
(`reviews.unreviewed_mpns`). `berc textconv` shows each line whole.

### What to review for a board

`berc fp todo` lists the library symbols a board's footprints come from, once each, and what each needs. The review
is of the library: each symbol is checked as `berc fp check --lib` checks it, against the library footprint its
`Footprint` field names, so a symbol reviewed once is done for every board that uses it.

```sh
berc fp todo                       # the KiCad project in this directory
berc fp todo hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints
berc fp todo hw/board --format json
```

The symbols to review come first, in the order to step through them: changed (reviewed before, the footprint moved
on), other version, malformed, unreviewed, partial (an MPN's datasheet not reviewed yet); by symbol within each. Each line shows the footprint, the hash it gets once
reviewed and the board references that use it, and (always shown, whatever the symbol's own status) the references
whose board places a different footprint than this line's, the same `Lib:Name` mismatch `berc fp check` names.
Below them, under their own heading, the symbols that can't be
reviewed until fixed: missing (the library symbol or its footprint is not found, the line says why) and refused. The
ok ones and those without a footprint are counted; `--all` lists them too. `--format json` always holds every symbol
with its footprint file, the hash now and the `Datasheet` field, for the app's review tab and scripts; the library
call is `better_erc::fp::todo`. A symbol whose board places another footprint than its own carries that one too:
`placed`, its library file `placed_file` and hash now `placed_hash`, or `placed_note` saying why no library has it.
`files` lists every file the list was made from (what the app's review tab watches). The exit code is 0 when there is nothing to review or fix, 1 otherwise, 2 when the
project can't be read.

### Accepting a review

`berc fp accept` writes the review hash of board footprints into their symbols:

```sh
berc fp accept hw/board U1 U2           # the hash of U1 and U2 now, into their library symbol
berc fp accept hw/board U1 --dry-run    # what would be written, nothing written
berc fp accept hw/board --symbol Device:R --hash fp1:0123456789abcdef
berc fp accept hw/board U1 --footprint VHRD_Package_QFP:LQFP-48   # also sets the symbol's own Footprint
berc fp accept hw/board Q1 --mpn BSS138W-7-F --mpn BSS138WQ-7-F --doc DS30154@rev16 --page 3
```

`--mpn` (repeatable), `--doc`, `--pdf` and `--page` say where the review was made: the review line of that
datasheet is written (see [Review lines](#review-lines-where-a-review-was-made)), with `by=` (`--by`, else the
login name) and today's date. Among the symbol's other lines, a line about the same datasheet (the same `pdf=`, or
the same document number) is replaced in its place; the MPNs it names leave the other lines, and a line left with
none goes; a line with the hash alone goes once lines with MPNs cover every MPN; a new line goes last. Without
them, the line is the hash alone and replaces the lines without MPNs. Every file gets the same lines, and a review
property left over past them is removed.

`--footprint LIB:NAME` also sets the library symbol's own `Footprint` field to it, the value `berc fp check` showed
as the one the board places: written the same way as the review hash, into the library and every schematic copy.
Given for references of more than one library symbol at once, it is refused: one value can't hold for several.

The review line goes into the `Footprint_Review` fields of each footprint's schematic symbol's library symbol, in two places:
the library's `.kicad_sym` (or the symbol's file in a `.kicad_symdir`), found through the project's or the global
`sym-lib-table`, and the copy of the symbol in every schematic sheet's `lib_symbols`, so `berc fp check` finds both
up to date. A library that is not found is named in the output, and only the schematic copies are written.

The `sym-lib-table` lookup follows the library wherever it actually lives, even for a project copied somewhere else
to try `accept` on: running it against a scratch copy of a project still writes into the real library the table
points at, not a copy of it, `--symbol`/`--hash` included. To try `accept` without touching the library in place,
copy the library too and rewrite the copied project's `sym-lib-table` (or the global one in KiCad's configuration
directory) to point at that copy.

Only the field is written. Every other byte of each file stays as it was, so the file's diff is the one field. A
field that is there gets its new value; a new field goes after the symbol's own fields, hidden, laid out as the
KiCad version that wrote the file lays it out (checked against kicad-cli in the tests). KiCad 8, 9 and 10 files are
written; an older file is refused until KiCad 8 or newer saved it. Each file is replaced through a temporary file
beside it. A library open in KiCad's symbol editor shows the field once it is opened again.

Nothing is written when anything is wrong:

- a symbol placed with more than one footprint (a generic `Device:C` on 0402 and 1210 pads): one hash can't hold
  for all of them;
- footprints of one symbol whose board copies differ;
- a symbol that sets no footprint, a footprint without a schematic symbol, a hash that is not `fp1:` and 16 hex
  digits.

The exit code is 0 when written (or nothing needed a change), 2 otherwise. `--format json` lists every file with its
old value.

### Checking a symbol library

`berc fp check --lib` checks a symbol library itself, before any board uses it: each symbol against the library
footprint its `Footprint` field names.

```sh
berc fp check --lib symbols/Lib.kicad_symdir --footprints footprints
berc fp check --lib symbols --footprints footprints   # every library in the folder
berc fp check --lib Device hw/board                   # a nickname from hw/board's or the global sym-lib-table
```

`--lib` takes a `.kicad_sym` file, a KiCad 9+ `.kicad_symdir` directory, a folder of them, or a nickname from the
`sym-lib-table`; it can be given more than once. A library given as a path is named by its file name
(`Lib.kicad_symdir` is `Lib`). The footprints are found through `--footprints` (a `.pretty` directory or a folder of
them, each named by its folder name, searched first), then the project's and the global `fp-lib-table`; the project is
the directory given, `.` by default. Each symbol is one of:

| Status | Meaning |
|---|---|
| ok | the footprint is the one reviewed |
| changed | the footprint differs from the one reviewed: review it again |
| unreviewed | the symbol has no review hash; the line shows the hash to put there once reviewed |
| missing | the footprint the symbol names is not found; the line says why |
| refused | the symbol has a review hash but sets no footprint, so the hash reviews nothing |
| other version, malformed | the field holds a hash of another version, or no hash at all |
| no footprint | the symbol sets no footprint and has no hash (a generic part, a power symbol): listed last, not a failure |

A derived symbol (`extends`) has its parent's footprint and hash unless it sets its own. The exit code is 0 when every
symbol is ok or has no footprint, 1 otherwise, 2 when a library can't be read.

`--allow NICKNAME` (repeatable, for `--lib` here and for `berc fp todo`) lists the library nicknames a symbol and its
footprint may be in: an exact name, or a prefix ending `*` (`--allow 'VHRD_*'`). A symbol's own library, or the one
its `Footprint` names, that is none of them is reported, naming it. Given none, there is no rule and nothing is
reported: a project only sets this once it has a library convention to enforce.

### 3D models against the footprint

`berc fp model` reads each footprint's 3D models (STEP) and measures them against the footprint: the third source of
truth beside the datasheet drawing and the footprint itself.

```sh
berc fp model hw/board                          # every footprint on the project's board
berc fp model hw/board U3 J1 --details          # these references, each pad's share under a lead
berc fp model ~/kicad-library/footprints/VHRD_Package_SO.pretty   # every footprint of a library
berc fp model Lib.pretty/SOT-23.kicad_mod --format json
```

Each `(model ...)` entry's file is found as KiCad finds it: `${VAR}` from the environment, KiCad's
`kicad_common.json`, `KIPRJMOD` (the board's directory, or the directory a library is in; `--project DIR` sets
it) and a stock KiCad's `KICADn_3DMODEL_DIR`; a `.wrl` tries `.step` and `.stp` beside it. The file is read
(`erc_step`) and placed as KiCad places it: the scale, then the rotation about x, y and z (each by the negated angle),
then the offset; a test holds this against `kicad-cli pcb export step`. For each model the command prints:

- the file and the variables that found it, or why it was not found;
- the file's unit, its parts and faces, and what the reader could not read (`--details` lists every warning);
- the extents in x, y and z, the height above the board and the depth below it (THT leads);
- the middle of the model against the middle of the pads;
- how far the model reaches out of the courtyard;
- whether the leads land on the pads: the model's bottom 0.2 mm seen from below against each pad's copper, how many
  pads have a lead over them and how much of the slice is off the pads (`--details`: each pad's share).

Coordinates are KiCad's 3D frame of the footprint in mm: x right, y up (the footprint editor's y negated, as the model
offset is written), z up from the board surface. The findings are catalogue checks: `LIB-22` model file missing or
unreadable, `LIB-23` out of the courtyard, `LIB-24` a pad without a lead, `LIB-25` scaled, `LIB-26` not in mm,
`LIB-27` turned by an angle that is not a multiple of 90°, `LIB-28` no model shown (`berc explain LIB-24`). The exit
code is 1 on any warning, 0 when there are only suggestions (no model, not in mm) or nothing, 2 on a read error.

### Reviewing in the app

The app's Footprint review tab does the review itself: it starts from a library symbol, since the symbol is what a
schematic places and what holds the hash, draws the footprint its `Footprint` field names over the datasheet's
drawing, and Accept writes the hash into that symbol's `Footprint_Review` field.

1. **Pick a symbol**: open a board or project (`.kicad_pcb`, `.kicad_pro`, `.kicad_sch`) for the library symbols
   its board uses, as `berc fp todo` lists them, those to review first; or a symbol library (a `.kicad_sym`, a
   `.kicad_symdir` folder, or a nickname from the `sym-lib-table`s) for its symbols. Each row shows the symbol's
   status (unreviewed, changed, ok, missing, no footprint). Picking one opens the library footprint its `Footprint`
   field names, found through the project's or the global `fp-lib-table`; from a board, when every reference places
   a different footprint than that field names, the one actually placed opens instead, noted above the canvas
   rather than left unsaid. That one too is read from its library file, since the review hash is the library
   footprint's; the note says when the board's copy differs from it (the board not updated from the library yet),
   and only a footprint no library table has is read from the board's copy. A symbol that sets no footprint, or whose
   symbol or footprint is not found, is refused with the reason. Next to review steps to the next symbol that
   needs one. A reviewed symbol lists its review lines (what each was made against, its MPNs, ok or stale) and the
   MPNs no line reviews yet.
2. **Open the datasheet**: the symbol's `Datasheet` field when it names a local file, a local document library
   search, or Open PDF. The library search runs a command (a setting) with the datasheet's file name, a part name
   in its web address, the `Mpn` fields or the value, until one finds something. The command prints a JSON list of
   `{"path": ..., "page": ...}`. What it finds is listed in a panel of its own, and for a symbol just picked the
   first one opens by itself. Picking another symbol opens its datasheet, or closes the last one when it has none.
   A symbol whose review recorded its datasheet opens that PDF first, found in the document library by its content
   hash, else by its document number, and with it the view the review was made in: the page, the scale and the
   footprint's placement. So does opening any datasheet a review line was made on. A line whose hash is no longer
   the footprint's is said to be stale, and a recorded PDF the library doesn't have is said too. The library's
   index comes from a command (a setting, `tpm lib docs --json --all` by default) that prints a JSON list of
   `{"path", "hash", "doc", "rev"}`, paths relative to the library's root; the file on disk is then found through
   the library name command below, which prints full paths.
   A symbol with `Mpn` fields gets a table, Datasheets by MPN, at the top of the Datasheet section: each MPN, the
   library document that covers it, and its review state (reviewed, reviewed elsewhere when its line was made on
   another document, stale, or not yet). Which document covers an MPN is the document library's answer, not
   better_erc's: a command (a setting, `library_mpn`, `tpm lib mpn --json` by default) gets the MPNs at the end and
   prints a JSON list of `{"mpn", "path", "hash", "doc", "rev", "why"}`, one object per document of an MPN, best
   first, or one with no `path` and `why` it has none. A series or catalogue file may cover several MPNs. Each MPN
   is asked once per session. Clicking a document opens it with the view of its review line, else the page search
   runs. An MPN without a document says why, and its Fetch button runs another command (`library_fetch`,
   `tpm lib fetch --mpn` by default) with the MPN, then asks about it again. There are no extra `Datasheet2..N`
   fields: the library holds the mapping.
   The page search then goes to the page with the land pattern, found by words such as "recommended land pattern"
   (a setting); the pages it found are listed to pick from. F shows the page at the view's full width.
3. **Set the scale**: R with the pointer at one end of a dimension of the drawing, R again at the other (or pick
   Scale on the mode bar and drag along it). The ruler snaps level: horizontal or vertical by the larger of its dx and dy, an end
   dragged goes where it is dragged and the other end follows on that axis (Shift held, or the setting off: any
   angle). Its ends snap to the drawing's own lines, read from the PDF's vector paths: within 10 screen pixels (a
   setting, 0 for never) to a line's end or two lines crossing, else onto the nearest line's centre; on a level
   ruler each coordinate on its own, so on a horizontal ruler an end takes the X of the extension line beside it
   and the Y of a line along it. Thin filled rectangles count as lines, and the edges of other filled shapes (a
   pad drawn filled) as lines too. Where an end would snap shows at the pointer before it is placed: the lines in
   orange, a square on an end, a cross on a crossing, a ring on a line. Alt held leaves an end where the pointer
   is. Z zooms in on the first end, where a click or a drag puts it exactly; Z again on the
   other end. The number the dimension prints between the ruler's ends, read from the page's text, is outlined
   and offered as a chip beside the ruler's length (`2.00 mm · D`): D or a click on it sets the scale from it. S goes back to the view and asks the length in a box in the middle of the canvas, the number typed
   shown large in mm: type the length and Enter sets the scale, Esc leaves the box without one (the side panel's
   field and Set scale do the same by mouse); Z a third time does what S does. The keys of the
   whole step: F, R, R, Z, Z, S, the length, Enter; with the snap and the dimension's number, often F, R, R, D. A page
   that states its own scale (`SCALE: 20X`) offers it as a button; check it with the ruler. Once the scale is set,
   the mode is Move again.
4. **Lay the footprint on the drawing**: drag it (the Move mode, M), nudge it with the arrow keys (Shift: ten times
   as far), turn it with T or ⟲ 90° or by an angle, mirror it with X for a drawing seen from below. Opacity, Blink
   (B) and Invert page (I, white-ish on black, for a dark drawing or just to see differently; the default is a
   setting) help to see both, and so does the blend mode: Normal (the footprint over the page by its opacity),
   Multiply (the drawing's lines stay dark through it), Screen, or Difference: with white copper (a setting),
   white paper under a pad goes black, so a footprint that covers the drawing's pads exactly leaves no light edge, and a pad that
   misses shows light (inverting the page keeps these meanings: Difference still reads the same, now on black).
   Three views show what the board makes on one layer, one view at a time (a radio choice, C cycles through
   them), each filled in its one colour: Copper (the pads' copper, the copper graphics and the package's vias,
   red), Mask (the mask openings only: each pad's by its margin, and the mask graphics, pink) and Paste (the paste
   apertures only, such as the windows of a split exposed pad, gray). The pads are not drawn outside the Copper
   view; holes and pad numbers are drawn over every view. The margins are the pad's, else the footprint's, else
   the open board's design settings (`pad_to_mask_clearance`, `pad_to_paste_clearance` and its ratio) for a
   footprint of that board or its project's libraries. Back pads (the back side in the same views), Courtyard,
   Fab, Silk and Board edge are shown or hidden each by its checkbox. The colours are settings only, with no
   picker in the tab. A package's vias (round plated pads without a number of their own, such as thermal vias in
   an exposed pad; KiCad has no vias in footprints) are copper with their holes; they are in the review hash as
   the pads they are.
   Scroll zooms at the pointer; the right or middle button pans; Page Up and Page Down turn pages. A help line
   under the canvas names the keys.
5. **Measure and write notes**: a bar above the canvas shows the modes, each with its key, the one on highlighted
   and named first in the help line: Move (M: the mouse moves the footprint), Scale (R: the ruler that sets the
   scale), Measure PDF (E: the ruler on the drawing's lines, snapping as the scale's does) and Measure footprint (G:
   the ruler on the footprint, its ends snapping to the pads' centres and, on rect and roundrect pads, to the middle
   of each side, where the pad's centre cross meets its edge; a cross in a ring marks a centre, a diamond a side).
   Esc, M or a mode's own key again go back to Move. A measuring ruler shows its length with dx and dy, and once let
   go its length goes into the review note as `M1 = 1.234 mm`; moving an end changes that value, a new ruler is `M2`.
   The scale's ruler waits while measuring and is back in Move. The note field sits above the canvas; Enter over the
   canvas gives it the keyboard, and Enter in it (or Add) appends the note to the notes file: a Markdown section per
   note with the symbol, the footprint, the datasheet and its page, the time (UTC) and the text. The file is a
   setting, by default `footprint-review-notes.md` beside the project or library under review (an absolute path puts
   it anywhere). The datasheet is named as the document library names it, by a command (a setting; `tpm lib open
   --json` by default) that prints a JSON list of `{"path", "value", "doc_no", "doc_rev"}`: by its document number and
   revision where they are known (`` `tpm d DS13560 rev6` ``, which still finds it after a rename or a move), else by
   the words that open it, else by its file. The next note's measurements count from M1 again.
6. **Select and save as DXF**: Select (V), the fifth mode, picks pieces of the datasheet's drawing as the PDF draws
   them: a line, a polyline, an arc or curve, a closed shape. A click picks the piece under the pointer (inside a
   filled shape counts), again drops it; a click on nothing drops them all. A drag selects what lies wholly inside
   its box; Shift adds to the selection. The pieces selected are drawn over the page in cyan, the one a click would
   pick in a lighter cyan. Save DXF (W, the button under the mode bar, or the pie) writes them as a DXF beside the
   notes file, `<symbol>-p<page>-<n>.dxf`, and appends `` DXF `Rev_Mixed-p2-1.dxf` `` to the note, so the note links
   to it. The DXF is in the footprint's millimetres: the page's points go through the scale and back through the
   footprint's placement (origin, rotation, bottom view), so KiCad's footprint editor, File → Import → Graphics at
   0, 0, drops the drawing in place over the pads (DXF's Y points up; the import turns it over). It is R12 ASCII
   (LINE, ARC, CIRCLE, POLYLINE) with a few comment lines naming the symbol, the footprint and the page, so an agent
   reads it as text too. Curves become arcs and circles where they follow a circle within the DXF curve tolerance (a
   setting, 0.005 mm), any other curve becomes straight pieces within it. Save stays disabled, saying why, until
   something is selected, the scale is set on the page shown and a footprint is shown.
7. **The 3D model and Measure model**: 3 (a setting, the 3D button in the mode bar, the pie) swaps the canvas between
   the page and the footprint's 3D model. The STEP files of its `(model ...)` entries are found and read as `berc fp
   model` does, on a worker thread, and placed as KiCad places them: on a 1.6 mm board whose top is z 0, with the
   front pads' copper on it. Drag turns the model, the right or middle button pans, the wheel zooms at the pointer, a
   double-click turns about the point under it, a right-click gives the preset views; the SpaceMouse drives all six
   axes there. Measure model (O, the sixth mode, which shows the model by itself) measures on the faces and edges as
   the STEP file writes them, not on the triangles: a click picks a face or an edge, a second click another. Two
   parallel planes give their distance and two planes at an angle the angle; two cylinders, holes or circles the
   distance of their axes (a pin pitch); a plane and an axis parallel to it the axis' height; a plane and anything
   else the point's height over it; the same cylinder or circle twice its diameter, the same edge twice its length;
   anything else the straight distance. The result is drawn on the view and goes into the review note as `M3 = 1.270
   mm (pitch, axis to axis)`, numbered with the rulers' measurements. What is under the pointer shows at the top of
   the view (`pad 2: plane facing +Z at z 0.035 mm`, `part: hole Ø 0.800 mm`). The side panel's 3D model section
   names each file with its parts, faces, size and height, and what could not be read. A footprint that names no
   model, or a file not found, says so in the view and the section.
8. **The model's views over the page**: the model's top, bottom, front, back, left and right views drawn over the
   datasheet at the page's scale, to hold against the package drawing. Each is the model's silhouette filled in its
   colour (a setting, `colors.model`), outlined, with the edges seen from that side (`colors.model_edges`); edges
   hidden behind a face are left out. The top view lies as the footprint does, the side views stand on the board's
   line through the origin, the bottom view is mirrored as a package drawing's is. K (a setting, the Moves button in
   the mode bar, the pie) picks what moves: the footprint, then each view in turn, then the footprint again; the view
   picked is shown, the top view first on the footprint and the others in the middle of the canvas. What moves is
   what the mouse in Move, the arrow keys, turn (T) and mirror (X) move and what the Placement section shows; each
   view keeps its own place, all share the page's scale. The side panel's Model views section shows and hides each
   view, picks what moves, and sets the views' blend mode (normal, multiply, screen, difference), opacity and blink
   (B blinks what moves). Measure footprint (G) snaps to the views' outline corners and edge ends too, so a model
   dimension measures on the page. The footprint is drawn over the views where they overlap.
9. **Accept** (A): when the footprint matches the drawing, Accept writes its review line into the symbol: the library
   symbol and, for a symbol from a project's list, the schematic's copies too (as `berc fp accept --symbol` does).
   The line records where the review was made: the datasheet by the document library's content hash and document
   number, the page, the scale, the footprint's placement, who (a setting, else the login name) and the day. It
   asks first which of the symbol's MPNs this datasheet covers, those found in the PDF's text (and those the
   line for this datasheet already names) ticked; none ticked makes a line for the whole symbol. It shows what
   its dry run would write, following the ticks: each file, each property, the old and the new value. When the
   board places a different footprint than the symbol's own `Footprint` field, a checkbox offers to also set that
   field to the placed one, showing both; checked, it writes the same as `berc fp accept --footprint` does.
   Enter (or
   Write) writes; Esc (or Cancel) writes nothing. It warns, per file and naming the lock file, when KiCad may
   overwrite the change on its next save: a file with KiCad's lock file beside it (`~name.kicad_sch.lck`, open in
   an editor there) that is held. A lock from this computer is held while a KiCad runs here (KiCad's locks name
   the user and host, no process id; a lock with a `pid` while that process lives); one from another computer
   always counts; an empty one is stale, as KiCad takes it. KiCad running with no lock beside the file is no
   warning.
   Once written, it lists the files written, or says why it wrote nothing, copies the hash to the clipboard and keeps the review
   (symbol's footprint, hash, datasheet, page, scale) in the tab's list. Accept stays disabled, saying why, until a
   symbol with a footprint is picked, a datasheet is open and the scale is set on the page shown.

#### Files changed on disk

The tab shows the files as they are on disk. It watches every file it read: the board and its schematics, the library
tables, the symbol libraries, the footprint files, the 3D models and the datasheet. A file saved in KiCad or another
editor is read again by itself once it stays quiet for a moment, and picking a symbol or a board footprint looks at
the files first. Reload (`F5`, a setting, or the button under the Open buttons) reads everything again; a line beside
it says what was read again and when. A reload keeps the view: the page, the scale, the footprint's placement, the 3D
camera and the list's filter. The review status follows the file, so a footprint changed since its review shows the
review as changed. A file counts as changed only when its content changed; what Accept writes into the symbols reloads
nothing.

#### With a SpaceMouse

A 3Dconnexion SpaceMouse drives the review through [spacenavd](https://spacenav.sourceforge.net/), the free driver
(`sudo pacman -S spacenavd`, `sudo systemctl enable --now spacenavd`; other distros: their spacenavd package). The
app reads spacenavd's socket itself ([ve_spacemouse](https://github.com/vhrdtech/vhrd_egui), pure Rust, no
libspnav), so the device is not needed to build or run it.

- **The puck moves the page**: slide it and the page follows the hand, lift it and the page moves up, push it away
  and the view zooms out about the pointer (pull it towards you to zoom in). The slide pans in screen points, so a
  push moves the page across the screen at the same speed at every zoom. P switches the puck to move the footprint
  over the page instead, at the same screen speed; zooming stays the view's.
- **Tap a button**: the left one lays the ruler (R at the pointer: once for the first end, again for the other), the
  right one switches what the puck moves (P).
- **Hold a button**: a pie menu opens at the pointer. Move out across its outer circle, or click, to do the action
  under the pointer; while the button is held, back through the middle and out again does the next one. The left
  button's ring has the page and the ruler (full width, zoom on a ruler end, type the length, the dimension's
  number, back to Move, measure the PDF, select the drawing, next and previous page, invert the page); the right
  one's the footprint (turn 90°, bottom view, blink, copper/mask/paste, measure the footprint); both buttons together
  Accept, Next to review, Write a note and Save the selection as DXF. Every slot names its
  action and its key (`Turn 90° · T`); a slot that can't be used now is drawn muted and says why under the menu
  ("No ruler yet: R lays one, or the left button's tap"). While Accept's question is open, the left tap writes
  and the right one cancels.
- **Calibrate…** (SpaceMouse, in the tab's side panel) measures which way the puck reads and which button is which:
  it asks for one motion at a time (slide right, lift up, push away, each button), works the mapping out from what
  it recorded and shows a table to confirm. The side panel says whether spacenavd is there and, when not, what to do.
- **One mapping for every app**: the mapping (axes, signs, dead zone, speeds, buttons) is not the app's own but
  ve_spacemouse's shared `spacemouse.toml`, the same settings panel as in every app on it. It is read from
  `.config/spacemouse.toml` in the folder the app started in and each parent, then `~/.config/spacemouse.toml`,
  merged key by key with the nearest file winning, as Cargo reads its config. The panel shows which file each value
  comes from; Save (and a used calibration) writes to every folder's file, `~/.config/spacemouse.toml`, or to this
  project's. A `[spacemouse]` section that `config.toml` kept up to 0.4.42 moves into `~/.config/spacemouse.toml`
  at the tab's first start, once: keys that file already has keep their values, then the section leaves
  `config.toml` (the settings' note says what moved).

A footprint opened alone (Open a footprint alone, a `.kicad_mod`) or picked from a board's footprint list is a
preview: it is drawn over the datasheet the same way, but there is no symbol to put the hash on, so Accept is
disabled and says so. For a board footprint the tab says whether the board copy is the same as its library file.

PDFs are read and drawn by [hayro](https://github.com/LaurenzV/hayro), in pure Rust, on a thread of its own. The
blend modes other than Normal are a wgpu shader that reads the page's own pixel under the footprint. The
PDF standard fonts that a datasheet uses without embedding them come from the Liberation fonts when installed, else
from egui's own font.

The settings (blink rate, the opacity, blend mode and colours at first, whether the page is inverted at first,
the page search words, the finest drawing scale, the font folders, the library search command, the library name
command and the notes file, the library docs command (`library_docs`), the MPN lookup and fetch commands (`library_mpn`,
`library_fetch`) and the reviewer (`reviewer`) for the review
record, the allowed library nicknames (`allowed_libraries`, the app's side of `--allow`, none
by default), how far Z zooms in, whether the ruler snaps level, how near its ends snap to the
drawing's lines, the DXF curve tolerance, and the keys F, M, R, E, G, Z, S, D, T, X, B, C, I, P, A, N, Enter, V, W, 3, O and K
by name in
`[footprint_review.keys]`; a file that still has M for the bottom view takes X for it) are in
`~/.config/better_erc/config.toml` (`$XDG_CONFIG_HOME`, or the file `$BERC_CONFIG` names), section
`[footprint_review]`; the tab's Settings shows and saves them. The SpaceMouse's mapping is in `spacemouse.toml` (above). Each step of a review
is recorded in berc's usage record (`app fp-review accept`, `app fp-review write`, ...), which `berc usage` counts in its `app` column.

## Development

The workspace has the crates of the core, `erc_model` (the board model, value parsing, the netlist and placement
comparisons), `erc_kicad` (KiCad read by us), `erc_import` (the importer registry with Altium, OrCAD and PnP),
`erc_analysis` (part classes, graph queries, power structure, I2C buses) and `erc_checks` (the engine, the checks,
the report and its formats); `better_erc`, the library that re-exports them; `berc` (the command line tool) and
`better_erc_app` (the app, binary `better_erc`), both on the library.
`test_schematics` holds a small KiCad schematic per check; the differential test exports them with `kicad-cli`
and is skipped with a message where KiCad is not installed.

```sh
just test       # all tests
just lint       # clippy with warnings as errors, and the format check
just fmt        # format the code
just run        # start the app
just check      # berc check on the test schematics, against their baseline
just netlists   # export the test schematics to netlists and pictures
just            # list every recipe
```

## Readable git diffs

`git diff` of a `.kicad_sch`, `.kicad_pcb` or `.kicad_pro` is mostly noise: UUIDs, instance paths, coordinates,
items written in a new order on every save, and - for the project file - dozens of new-but-default settings
added the moment a newer KiCad opens and saves an older project. `berc textconv` turns one KiCad file into sorted,
line-oriented text (one fact per line, no UUIDs, no coordinates), and git can diff that instead. The stored files
don't change; `git diff`, `git log -p`, `git show` and lazygit all show the readable form.

A `.kicad_sch`'s cached library symbols (`lib_symbols`) print their fields - `Datasheet`, `Footprint`,
`Footprint_Review`, ... - as their own `Name=value` lines, a hash underneath for the rest of the symbol (pins,
graphics, sub-symbols). `berc fp accept` (see [Accepting a review](#accepting-a-review)) writes the
`Footprint_Review` field into every sheet's `lib_symbols` copy, so this is what turns an accepted review into a
one-line diff instead of a changed hash taken on faith.

With `berc` installed (see [Install](#install)), tell git which files to convert, in the repo with the KiCad
files (`.gitattributes`, committed, or `.git/info/attributes` for yourself only):

```gitattributes
*.kicad_sch diff=kicad
*.kicad_pcb diff=kicad
*.kicad_pro diff=kicad
```

And how to convert them, once per machine:

```sh
git config --global diff.kicad.textconv "berc textconv"
```

No `diff.kicad.cachetextconv`: `berc textconv` is fast enough that caching it isn't worth the staleness it buys
(0.12 s on a 17 MB `.kicad_pcb`, 8 ms on the largest `.kicad_sch` of a real hardware repository). `cachetextconv`
would keep the converted text in git notes (`refs/notes/textconv/kicad`), per blob and per textconv command: git
stores the command as the cache's validity and drops the cache when the configured command differs. A `berc`
upgrade keeps the same command, so after one that changes the output format, old blobs keep showing their old
(but still cached) text until the command changes or the notes ref is cleared.
`git diff --stat` and `--numstat` still count raw lines; that is git, not berc. To see the raw diff once, pass
`--no-textconv`.

Turning `cachetextconv` on anyway (a very large repository, a slow disk): clear the cache once per clone after
every `berc` upgrade that changes textconv's output:

```sh
git update-ref -d refs/notes/textconv/kicad
```

What the text shows:

- Schematic: title block; sub-sheets with file, page and pins; every symbol as
  `R104 10k  lib=Device:R  fp=Resistor_SMD:R_0402_1005Metric  dnp  no_bom` with its fields under it (sorted), units
  of one part grouped, pin alternates, and variant overrides only where they differ from the symbol; power symbols
  counted per net; no-connects by the pin they sit on (`nc U201.5 PA3`); labels, global and hierarchical labels
  counted; text and text boxes in full (newlines as `\n`); tables as rows; wires, junctions and graphics counted;
  a short hash per library symbol, so an edited symbol shows as one changed line.
- Board: stackup, design rules and plot settings one per line; every footprint as
  `C206 100nF  fp=Capacitor_SMD:C_0402_1005Metric  side=top  rot=90  smd` with its fields, 3D model, variant
  overrides and the net of each pad; nets with pad, track and via counts; per-layer track and zone counts; vias by
  size; zones with their settings; the board outline size; text and dimensions.
- Project file: every setting as a dotted path (`board.design_settings.defaults.zones.min_clearance = 0.3`); net
  classes, the sheet list and other named lists keyed by name instead of position, so adding or removing one
  shows as one line; a setting that matches KiCad 10.0.6's own default (`pro_defaults`, transcribed from the
  KiCad source - no headless KiCad here to generate a fresh project file instead) or JSON's own "nothing here"
  value (`false`, `0`, `""`, `[]`, `{}`) is folded into one summary line per top-level section instead of
  printed, so opening a project in a newer KiCad and saving it - which can add dozens of such settings - shows
  as a small diff. The table covers the zone and DRC/ERC rule-severity defaults, the ERC pin conflict matrix and
  `reuse_designators`; anything else only gets the JSON-emptiness check, which can hide a setting that is
  genuinely and meaningfully zero. `--all` always shows every line.
- Anything berc doesn't know is printed as a one-line s-expression in `# other`, so nothing is silently dropped.
  A file that isn't a KiCad s-expression comes out as it is.

`berc textconv --with-geometry FILE` adds positions (footprints rounded to 0.1 mm), wires and tracks, for when
placement is what you are reviewing. `--all` (`.kicad_pro` only) shows settings otherwise folded as looking like
KiCad's default.
