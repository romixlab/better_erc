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
  footprint, and `berc fp todo` lists the library symbols a board uses that need a review. See
  [Footprint review](#footprint-review).
- **The app** (`better_erc`, egui, crate `better_erc_app`): an import tab that opens any design the registry reads
  (and Altium or OrCAD files picked one by one) and shows its findings with their explanations and the checks that
  did not run, and tabs for nets (power rails, grounds and switching nodes from names and pin types), I2C buses,
  style findings and inputs; a footprint review tab that draws a library footprint over its datasheet drawing and
  gives the review hash (see [Reviewing in the app](#reviewing-in-the-app)).
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

Once a footprint is reviewed, put its hash in the `Footprint_Review` field of the library symbol that uses it (in
the symbol editor, then update the schematic from the library). `berc fp check` then checks a board:

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
| changed | the footprint differs from the one reviewed: review it again |
| unreviewed | the symbol has no review hash; the line shows the hash to put there once reviewed |
| refused | the symbol sets no footprint, so a review of the one on the board means nothing |
| no symbol | the footprint has no schematic symbol and is not marked board-only |
| board-only | no symbol, marked board-only in KiCad (logos, mounting holes): listed last under its own heading, not a failure |
| other version, malformed | the field holds a hash of another version, or no hash at all |

A schematic copy that holds a different hash than the library is reported as well: the schematic was not updated
from the library. So is a board copy that differs from its library footprint (the footprint the board names, found
through `--footprints` and the `fp-lib-table`; a board footprint without a library nickname is looked up by its
symbol's `Footprint` field): the line shows both hashes, and the board was not updated from the library. Footprints
that could not be compared are summed up once per reason below the list. `--symbols DIR` adds symbol libraries (a
`.kicad_sym`, a `.kicad_symdir` or a folder of them, each named by its file name) searched before the
`sym-lib-table`, and `--footprints DIR` footprint libraries searched before the `fp-lib-table`, for a PC without
KiCad's global tables:

```sh
berc fp check hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints
```

The exit code is 0 when every footprint is ok or board-only and every copy up to date, 1 otherwise, 2 when the
project can't be read.

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
on), other version, malformed, unreviewed; by symbol within each. Each line shows the footprint, the hash it gets once
reviewed and the board references that use it. Below them, under their own heading, the symbols that can't be
reviewed until fixed: missing (the library symbol or its footprint is not found, the line says why) and refused. The
ok ones and those without a footprint are counted; `--all` lists them too. `--format json` always holds every symbol
with its footprint file, the hash now and the `Datasheet` field, for the app's review tab and scripts; the library
call is `better_erc::fp::todo`. The exit code is 0 when there is nothing to review or fix, 1 otherwise, 2 when the
project can't be read.

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

### Reviewing in the app

The app's Footprint review tab does the review itself: it draws the footprint over the datasheet's drawing, and
Accept gives the hash for the `Footprint_Review` field.

1. **Open a footprint**: a library footprint (`.kicad_mod`), or a board (`.kicad_pcb`) to pick from. A board lists
   one row per library footprint with the references that use it. A picked footprint is reviewed from its library
   file, found through the project's or the global `fp-lib-table`; the tab says whether the board copy is the same.
   Where the library file is not found, the board copy is reviewed.
2. **Open the datasheet**: the footprint's `Datasheet` field when it names a local file, a local document library
   search, or Open PDF. The library search runs a command (a setting) with the datasheet's file name, a part name
   in its web address, the `Mpn` fields or the value, until one finds something. The command prints a JSON list of
   `{"path": ..., "page": ...}`. The page search then goes to the page with the land pattern, found by words such as
   "recommended land pattern" (a setting); the pages it found are listed to pick from.
3. **Set the scale**: pick Ruler and drag along a dimension of the drawing (drag an end to adjust it), type its
   length and Set scale. A page that states its own scale (`SCALE: 20X`) offers it as a button; check it with the
   ruler.
4. **Lay the footprint on the drawing**: drag it (Move footprint), nudge it with the arrow keys (Shift: ten times
   as far), turn it with R or ⟲ 90° or by an angle, mirror it with M for a drawing seen from below. Opacity, Blink
   (B), the colours (presets for the pads, a picker per layer group) and the layers shown (pads, courtyard, fab,
   silk, copper, mask, paste) help to see both. The blend mode mixes the footprint into the page: Normal (over it
   by its opacity), Multiply (the drawing's lines stay dark through it), Screen, or Difference: with white pads,
   white paper under a pad goes black, so a footprint that covers the drawing's pads exactly leaves no light edge,
   and a pad that misses shows light. Scroll zooms at the pointer; the right or middle button pans; Page Up and
   Page Down turn pages.
5. **Accept**: when the footprint matches the drawing, Accept copies its review hash to the clipboard and keeps the
   review (footprint, hash, datasheet, page, scale) in the tab's list. Accept stays disabled, saying why, until a
   footprint and a datasheet are open and the scale is set on the page shown.

PDFs are read and drawn by [hayro](https://github.com/LaurenzV/hayro), in pure Rust, on a thread of its own. The
blend modes other than Normal are a wgpu shader that reads the page's own pixel under the footprint. The
PDF standard fonts that a datasheet uses without embedding them come from the Liberation fonts when installed, else
from egui's own font.

The settings (blink rate, the opacity, blend mode and colours at first, the page search words, the finest drawing
scale, the font folders, the library search command) are in `~/.config/better_erc/config.toml` (`$XDG_CONFIG_HOME`, or the file
`$BERC_CONFIG` names), section `[footprint_review]`; the tab's Settings shows and saves them. Each step of a review
is recorded in berc's usage record (`app fp-review accept`, ...), which `berc usage` counts in its `app` column.

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
