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
  - sheet documentation: functional blocks (a `Block` field) without a note, placeholders left in notes and fields,
    `See N12` references to a missing note, block names that contradict their parts (`DRAW-48` to `DRAW-51`)
- **`berc check`**: the checks on a KiCad project (or a KiCad netlist, Altium, OrCAD, Smart PDF or PnP file) from the shell
  or a CI job, as text, JSON, SARIF (GitHub code scanning) or Markdown (a PR comment), with an exit code by
  severity and a baseline so only new findings fail. `berc explain ID` and `berc checks list` show the catalogue.
  See [Checks in CI](#checks-in-ci).
- **`berc diff --pnp A B`**: two pick-and-place files, two boards, or a board and the PnP file exported from it
  compared footprint by footprint (position, rotation, side, value, package), with tolerances.
- **`berc diff --nets A B`**: the nets of any two designs (a schematic, a netlist, an Altium export, a Smart PDF)
  compared as the pins each net connects, whatever their names: renamed, removed and added nets.
- **`berc place-fields PROJECT`**: symbol fields that overlap something (`DRAW-9`, `DRAW-34`) moved into a column
  beside their body; a dry run by default, `--write` edits the schematic. See [Placing fields](#placing-fields).
- **`berc fp`**: footprint review. `berc fp hash` prints a footprint's review hash, the same for a library file and
  every board copy of it; `berc fp check` checks every (board footprint hash, MPN) pair against the footprint reviews
  of the parts snapshot, `berc fp check --lib` every symbol of a symbol library against a part source's reviews,
  `berc fp todo` lists what a board needs reviewed, one line per footprint and family, and `berc fp accept` records a
  review in the part source; `berc fp model` reads each footprint's 3D models and measures them against it. See
  [Footprint review](#footprint-review).
- **The app** (`better_erc`, egui, crate `better_erc_app`): an import tab that opens any design the registry reads
  (and Altium or OrCAD files picked one by one) and shows its findings with their explanations and the checks that
  did not run, and tabs for nets (power rails, grounds and switching nodes from names and pin types), I2C buses,
  style findings and inputs; a footprint review tab that draws a footprint over its datasheet drawing and records
  the review in the part source (see [Reviewing in the app](#reviewing-in-the-app)).
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
- [docs/project-file.md](docs/project-file.md): the project file `<name>.berc.toml`: net voltages, derating
  factors, check settings and waivers, and `berc waive`.
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

The mx3 part store as a part source (`berc parts refresh` and `berc fp accept` without `--from`) is the cargo
feature `mx3`, off by default: `cargo install --locked --path berc --features mx3`. It adds an HTTP client (reqwest
with rustls); the default build has no network code.

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

Facts the schematic cannot state live in the project file next to the design, `<name>.berc.toml` (else
`berc.toml`): net voltages, derating factors, check settings, and waivers for findings accepted on this design.
`berc waive hw/board I2C-1 R5 --reason "..."` appends a waiver; a waived finding stays in the report, marked, and no
longer fails the run. See [docs/project-file.md](docs/project-file.md).

Some checks read files kept with the board. `--set drawing.notes=../doc/notes.md` names the board's notes file, a
Markdown file with one `## N13 · title` heading per note, relative to the project directory; `DRAW-50` then reports
each `See N13` on a sheet that points at a note the file lacks. Without the setting that check does not run. The
project file sets it too, as `notes` under `[drawing]`.

In this repository `just check` runs `berc check` on the test schematics against the committed
`test_schematics/berc-baseline.json` (they have findings on purpose), and `just check-baseline` rewrites it.

## Part facts: the parts snapshot

The checks that compare a part's MPNs with what its instance asks for (value, tolerance, dielectric, package, ratings)
and the footprint review read part facts from one committed file beside the project, `parts.lock.toml`, like
`Cargo.lock`. A check run never asks a server: CI runs offline and gives the same result on every PC.

```sh
berc parts refresh PROJECT --from parts.toml [--check] [--format text|json]
```

`berc parts refresh` writes the snapshot again from a part source. It holds every MPN field (`Mpn1`, `MPN_2`,
`Manufacturer_Part_Number`, ...) of the parts fitted in the base design or any variant, and per MPN the facts the
checks use: each with its value, the source that stated it, its rank and when. With a board file it also holds, for
each board footprint's review hash ([`berc fp hash`](#footprint-review)) and each MPN placed on it, the footprint
reviews that cover the pair. An MPN taken off the board leaves the file. The file is sorted and has one line per
fact, so its git diff after a refresh shows exactly what changed in the source: a new value, a retracted review.

| key | what |
|---|---|
| `capacitance`, `resistance`, `inductance` | the value, in F, Ω, H |
| `tolerance` | below and above the value, in % or in its unit |
| `rated_voltage`, `power`, `saturation_current` | ratings, in V, W, A |
| `dielectric`, `technology`, `case_eia` | words: `X7R`, `thick_film`, `0402` |

`--from FILE` takes part data from a file in the snapshot's own shape: hand-filled facts, or another board's
snapshot. Without it, a berc built with the `mx3` feature asks the mx3 part store: the facts by MPN
(`params/by_mpn`) and the reviews of each pair (`fp/review/check`). It finds the store as the mx3 command line does:
the URL from `MX3_URL`, else `[share] mx3_url` in tpm's `config.toml` (`TPM_ROOT`, else `~/git/tpm`), else
`http://localhost:3000`; the token from `MX3_TOKEN`, else `~/.config/mx3/token` (`mx3 token set`). The token is
sent only in the request header and never printed. `--check` writes nothing and exits 1 when the snapshot is out of date, for a CI step; exit 2 is a design,
source or file error. The project file can put the snapshot elsewhere: `[parts] lock = "data/parts.lock.toml"`.

An MPN on the board that the snapshot lacks is a finding, `BOM-20`, until the next refresh: a new part cannot reach
the BOM unchecked. Without a snapshot the checks that need it report themselves as not run.

## Display fields from the snapshot

```sh
berc fields sync PROJECT [--write] [--set NAME=VALUE]
```

A resistor's or capacitor's display fields state what all its MPNs guarantee, and `berc fields sync` writes them
from `parts.lock.toml` (`BOM-12` and `LIB-6` hold them to the same values):

| Field | Value |
|---|---|
| `Voltage` (capacitors), `Power` (resistors) | the lowest rating over all MPNs: `25V`; a power below 1 W in whole mW (62.5 mW is `63mW`), one of 1 W or more as `1W` |
| `Dielectric` (capacitors) | the worst code of the ceramic MPNs, the one every other code covers (`X5R` for `X5R` and `X7R`) |
| `Package` | the case every MPN has (`0402`) |

A `Voltage` or `Power` text states a rating when the rating, rounded half up to the precision the text is written
with, equals it: `63mW` states 62.5 mW (1/16 W), `62mW` does not. A fraction such as `1/16W` is exact. A field that
already states the rating keeps its text.

A field equal to the board default (for a rating: a default that states it), the text variables `DEFAULT_C_PACKAGE`, `DEFAULT_C_VOLTAGE`,
`DEFAULT_C_DIELECTRIC`, `DEFAULT_R_PACKAGE` and `DEFAULT_R_POWER`, is removed from the part. A part without the field
that is off the default gets one, shown and placed as `place-fields` places fields (`--set` takes its
`drawing.*` settings). Only these four fields change; nothing else is removed. An MPN missing the fact (no rated
voltage, no `case_eia`), a dielectric code that is not read, or MPNs that differ in case leave the field as it is and
are listed with the reason. A part with no MPN in the snapshot is not touched.

The default is a dry run: one line per field, then the totals. `--write` edits the `.kicad_sch` files in place and
keeps every other byte. Close the schematic in KiCad first, and run `berc parts refresh` before, since the fields
follow the snapshot. A run after `--write` finds nothing to do, and the `BOM-12` findings on these fields are gone.
Exit code 2: a design or file error, or no snapshot.

## Placing fields

```sh
berc place-fields PROJECT [--sheet NAME] [--write] [--set NAME=VALUE]
```

Each placed symbol whose shown fields overlap text, a body, a wire or the sheet edge (`DRAW-9`), or come closer than
the `drawing.clearance` margins (`DRAW-34`), gets the first of these that clears them:

Only a rating that holds its kind's default may be hidden: a resistor's Package or Power, a capacitor's Package,
Dielectric or Voltage. The defaults are the project's text variables `DEFAULT_R_PACKAGE`, `DEFAULT_R_POWER`,
`DEFAULT_C_PACKAGE`, `DEFAULT_C_DIELECTRIC` and `DEFAULT_C_VOLTAGE`, which a drawing sheet can show; a rating
without its variable is never hidden. Any other field, and any other value, stays shown, so a 250mW resistor or a
C0G capacitor keeps its rating on the sheet. With `drawing.fields.hide_defaults=always` every shown rating at its
default is hidden first, wherever it stands, and the fields left are then placed as below.

1. Hideable rating fields that touch a power symbol, when nothing else overlaps the fields:
   those fields hidden, the column left where it is. Closeness alone is no reason to move a column that fits.
2. A column: Reference, Value, then the rest top to bottom as they stood, justified toward the body, anchored on
   the field step 0.635 mm from the body, lines 2 mm apart whatever the text size, on the first side where it hits
   nothing and no wire runs through it, slid along the side (up to 5 mm) or set a little farther out (up to 2.5 mm)
   when it must. A horizontal two-pin part tries above, below, right, left, off the line its wires run along; every
   other part right, left, above, below. A one- or two-pin part (a test point, a resistor) tries every slide on a
   side before the next side; a column above or below is centred on the body. Fields that overlap nothing but
   stand more than 2 mm from their body are placed again the same way.
3. A packed two-pin part (resistor, capacitor, diode, inductor, ferrite bead), whose centred column across its axis
   overlaps a neighbour's body on one side, or a body or settled fields on both, a horizontal one before any column:
   Reference and Value at the ends of the part,
   the other fields hidden; only when all of them are hideable. A horizontal part has them before and after its body on the line above its axis; a
   vertical one has them centred above and below its pins, only where that spot is clear.
4. A column shortened by hiding its last hideable fields, the fewest that clear it.
5. Else the column with the least overlap, or the fields stay when nothing is better.

Field text is always upright, never turned (`DRAW-8`). Bodies, wires, labels, power symbols and notes never move.

The margins follow `drawing.clearance.spacing` (`--set drawing.clearance.spacing=sparse`, here and in `berc check`):

| Spacing | Two parts' bodies, pins and fields keep between them |
|---|---|
| `tight` (default) | only `drawing.clearance.packed`, 0.127 mm each: parts packed at grid pitch pass, overlaps still count |
| `sparse` | their own margins, `drawing.clearance.part` and `.text` (2.54 mm body to body, 1.905 mm field to body) |

Everything else (wires, labels, power symbols, notes, sheets) keeps its own margins in both.

Every layout follows one style, set by the `drawing.fields` settings (`--set`, or the project file):

| Setting | Default | Meaning |
|---|---|---|
| `drawing.fields.pitch` | 2 mm | baseline-to-baseline distance between the lines of a column |
| `drawing.fields.body_gap` | 0.635 mm | space between a column (or a packed part's Reference and Value) and the body and pins |
| `drawing.fields.step` | 0.25 mm | grid the anchors are placed on, and the step a column slides or moves out by |
| `drawing.fields.gap_max` | 2 mm | fields farther than this from their body are placed again |
| `drawing.fields.ends_raise` | 1.27 mm | how far above a horizontal packed part's pin line its Reference and Value sit |
| `drawing.fields.hide_defaults` | `in_the_way` | `in_the_way`: a default rating is hidden only where the fields need room; `always`: wherever it is shown |

Fields that overlap nothing are left alone when they follow the style or stand in some other arrangement. A column
(lines on one anchor line, one justification, on one side of the body) or a packed part's ends off the style, by
pitch, body gap, grid or a column slid off the middle of its side, is placed again in the style: a one- or two-pin part's on the first of its sides that fits,
any other part's on the same side when it fits there next to the body, else the first side where it does, and only where that conflicts no more than where it stands. One run evens out the
columns of a sheet; a second finds nothing to do.

The default is a dry run: each symbol with its layout, the fields it hides and what still overlaps (`DRAW-9`) or is
too close (`DRAW-34`) once the whole sheet is placed, then both counts before and after (the edited files read
again). `--write` edits the `.kicad_sch` files in place: only the moved fields' `(at ...)` and `justify` change,
hidden fields get `(hide yes)`, and their symbols lose `(fields_autoplaced yes)` so KiCad keeps the new places.
Close the schematic in KiCad first. `PROJECT` is a directory or a `.kicad_pro` as for `berc check`; `--sheet` takes a
sheet's name or file.

## Footprint review

A footprint checked once against its datasheet should not have to be checked again on the next board, unless it
changed. `berc fp` keeps that record as a footprint review in the part source: the footprint by its review hash, and
the parts it fits, each by its MPN or a whole family by its MPN's decoding (a review of an 0402 resistor footprint
against Yageo's RC series datasheet covers every RC0402 resistor). The parts snapshot
([`parts.lock.toml`](#part-facts-the-parts-snapshot)) holds, for each board footprint hash and each MPN placed on it,
the reviews that cover the pair, so a check runs offline. Nothing is stored on the symbols.

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

Once a footprint is reviewed, `berc fp accept` records the review (see [Accepting a review](#accepting-a-review)),
or the app's Accept does. `berc fp check` then checks a board against its parts snapshot:

```sh
berc fp check                       # the KiCad project in this directory
berc fp check hw/board --format json
```

Each board footprint is hashed as placed, and each MPN of its part (its `Mpn1`, `MPN_2`, ... fields in every variant
it is fitted in) is one pair with that hash. A footprint is ok when a review of its hash now covers every one of its
pairs. Each footprint is one of:

| Status | Meaning |
|---|---|
| ok | a review of the footprint's hash covers every MPN of its part |
| changed | an MPN is covered only by a review of another hash: the footprint differs from the one reviewed, review it again |
| other version | an MPN is covered only by reviews of another version of the review hash (`fp2:` against `fp1:`) |
| not in lock | the parts snapshot lacks a pair: run `berc parts refresh` |
| partial | some MPNs covered, others not yet, named |
| uncovered | no MPN covered |
| no MPN | the part has no MPN to review the footprint against |
| no symbol | the footprint has no schematic symbol and is not marked board-only |
| not fitted | the part is fitted in no variant: nothing to review, not a failure |
| board-only | no symbol, marked board-only in KiCad (logos, mounting holes): listed last under its own heading, not a failure |

The review is of the footprint as the board places it, so a board copy that differs from its library footprint is
reviewed as it is; it is reported too (the footprint the board names, found through `--footprints` and the
`fp-lib-table`; a board footprint without a library nickname is looked up by its symbol's `Footprint` field): the line
shows both hashes, and the board was not updated from the library.

A board footprint placed under a different `Lib:Name` than its symbol's own `Footprint` field (a library migration
half done, or a deliberate per-instance override) is reported too, naming both: a library migration that renamed or
moved a footprint library, left half done, or a board edited by hand without updating the schematic, looks exactly
like this. A footprint placed with no library nickname at all is not an override, there being nothing to compare its
name against. Footprints that could not be compared are summed up once per reason below the list. `--symbols DIR`
adds symbol libraries (a `.kicad_sym`, a `.kicad_symdir` or a folder of them, each named by its file name) searched
before the `sym-lib-table`, and `--footprints DIR` footprint libraries searched before the `fp-lib-table`, for a PC
without KiCad's global tables:

```sh
berc fp check hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints
```

`--details` prints each MPN under its footprint with the reviews that cover it, or the review of another hash it was
reviewed as. Without a parts snapshot the command exits 2, naming `berc parts refresh`. The exit code is 0 when every
footprint is ok, not fitted or board-only and every copy up to date, 1 otherwise, 2 when the project or the snapshot
can't be read.

### Where a review was made

A review is made against one datasheet, and records where, so the review can be looked at again:

| Field | What |
|---|---|
| `hash`, `footprint` | the footprint's review hash and its `Lib:Name` |
| `covers` | what it covers: MPNs, each alone, or families, each a decoding scheme and the package segments (`case_eia`, `package`) its parts share |
| `doc`, `doc_rev` | the document number and revision, where the document library knows them: they survive a download of the same document with other bytes |
| `pdf_blake3` | the PDF by the document library's content hash |
| `page` | the page the land pattern is on, 1-based |
| `scale` | the drawing's scale on that page, mm per PDF point |
| `at` | where the footprint lay on the page: its origin in page points from the page's top left, its rotation in degrees, `mirror` when seen from below |
| `reviewer`, `reviewed_on`, `note` | who reviewed it, the day, a note |
| `retracted` | set when the review was withdrawn (who, when, why): a retracted review covers nothing |

A review is never edited or deleted: a new one is recorded, and a wrong one is retracted. The parts snapshot holds
the reviews its pairs name, so its git diff after a refresh shows a new or a retracted review.

### What to review for a board

`berc fp todo` lists what a board's footprints need reviewed: one line per board footprint hash and what one review
of it covers, a family where the MPNs decode into one, else one MPN, with the references it stands for.

```sh
berc fp todo                       # the KiCad project in this directory
berc fp todo hw/board --symbols ~/kicad-library/symbols --footprints ~/kicad-library/footprints
berc fp todo hw/board --format json
```

The lines to review come first, in the order to step through them: changed (reviewed before, the footprint moved
on, naming the review it was reviewed as), other version, uncovered; by footprint and cover within each. Each line
shows the footprint, its hash, the MPNs and the references, and (always shown, whatever the line's own status) the
references whose board places a different footprint than their symbol's own `Footprint`, the same `Lib:Name`
mismatch `berc fp check` names. Below them, under their own heading, the lines that can't be reviewed until fixed:
not in lock (refresh the snapshot), unknown MPN (the part source has no part of it: add it there first) and no MPN.
The reviewed ones are counted; `--all` lists them too. `--format json` always holds every entry with its hash, cover,
MPNs, references, the reviews that cover it (`covered_by`) or that it was reviewed as (`other`), the symbol's
`Datasheet` field, the library footprint file and its hash now (`library_hash`, another than `hash` when the board
was not updated from the library), and `reviews`, every review an entry names; the library call is
`better_erc::fp::todo`. `files` lists every file the list was made from (what the app's review tab watches). The exit
code is 0 when there is nothing to review or fix, 1 otherwise, 2 when the project or the snapshot can't be read.

### Accepting a review

`berc fp accept` records a review of board footprints in a part source, then refreshes the project's parts snapshot
from it:

```sh
berc fp accept hw/board R1 R7 --from parts.toml --doc RC_Series@rev12 --page 4
berc fp accept hw/board U3 --from parts.toml --mpn TPS62840DLCR --dry-run
berc fp accept --hash fp1:0123456789abcdef --footprint-id Resistor_SMD:R_0402 --family-of RC0402FR-0710KL --from parts.toml
berc fp accept hw/board U1 --from parts.toml --footprint VHRD_Package_QFP:LQFP-48   # also sets the symbol's own Footprint
```

The references must share one review hash: one review is of one footprint. By default the review covers every MPN of
their parts, each by its family where its MPN decodes into one (the same family `berc fp todo` lists), else alone;
an MPN the source has no part of and that decodes into no family is left out, named. `--mpn MPN` covers that MPN
alone and `--family-of MPN` its family, both repeatable, in place of the default. `--hash` with `--footprint-id`
reviews a footprint by its hash instead, a library footprint before a board uses it (as `berc fp hash` prints it);
it needs `--mpn` or `--family-of`, and no snapshot is refreshed.

`--doc NO@REV`, `--pdf b3:HEX`, `--page`, `--by` (else the login name) and `--note` say where the review was made (see
[Where a review was made](#where-a-review-was-made)); the day is today's. `--dry-run` says what would be recorded and
records nothing.

`--from FILE` is the part source: a file in the snapshot's own shape, whose `[[review]]` list the review is added to.
Without it, a berc built with the `mx3` feature records the review in the mx3 part store (`fp/review/accept`),
found as for [`berc parts refresh`](#part-facts-the-parts-snapshot), and refreshes the snapshot from there.

`--footprint LIB:NAME` also sets the library symbol's own `Footprint` field to it, the value `berc fp check` showed
as the one the board places: written into the library's `.kicad_sym` (or the symbol's file in a `.kicad_symdir`),
found through the project's or the global `sym-lib-table`, and the copy of the symbol in every schematic sheet's
`lib_symbols`. Only the field is written; every other byte of each file stays as it was. KiCad 8, 9 and 10 files are
written; an older file is refused until KiCad 8 or newer saved it. The `sym-lib-table` lookup follows the library
wherever it actually lives, even for a project copied somewhere else: to try it without touching the library in
place, copy the library too and point the copied project's `sym-lib-table` at that copy.

Nothing is recorded when anything is wrong: references whose footprints differ, a footprint without a schematic
symbol, a part without an MPN, an MPN to cover that no reference carries, a family asked of an MPN that decodes into
none, a hash that is not `fp1:` and 16 hex digits. The exit code is 0 when recorded (or a dry run), 2 otherwise.
`--format json` holds the covers, the MPNs left out, the review as the source stored it and the snapshot refreshed.

### Checking a symbol library

`berc fp check --lib` checks a symbol library itself, before any board uses it: each symbol that names its MPNs,
the hash of the library footprint its `Footprint` field names paired with each MPN, judged against the reviews a part
source holds.

```sh
berc fp check --lib symbols/Lib.kicad_symdir --footprints footprints --from parts.toml
berc fp check --lib symbols --footprints footprints --from parts.toml   # every library in the folder
berc fp check --lib Device hw/board --from parts.toml                   # a nickname from a sym-lib-table
```

`--lib` takes a `.kicad_sym` file, a KiCad 9+ `.kicad_symdir` directory, a folder of them, or a nickname from the
`sym-lib-table`; it can be given more than once. A library given as a path is named by its file name
(`Lib.kicad_symdir` is `Lib`). The footprints are found through `--footprints` (a `.pretty` directory or a folder of
them, each named by its folder name, searched first), then the project's and the global `fp-lib-table`; the project is
the directory given, `.` by default. Each symbol is one of:

| Status | Meaning |
|---|---|
| ok | a review of the footprint's hash covers every MPN of the symbol |
| changed | an MPN is covered only by a review of another hash: the footprint differs from the one reviewed |
| other version | an MPN is covered only by reviews of another version of the review hash |
| partial | some MPNs covered, others not yet |
| uncovered | no MPN covered |
| missing | the footprint the symbol names is not found; the line says why |
| unjudged | no `--from`: the footprint is read and hashed, the MPNs not judged |
| no MPN | a footprint but no MPN: a generic symbol, reviewed on each board with the MPNs its instances carry; not a failure |
| no footprint | the symbol sets no footprint (a generic part, a power symbol): listed last, not a failure |

A derived symbol (`extends`) has its parent's fields unless it sets its own. The exit code is 0 when every symbol is
ok, has no MPN or no footprint, 1 otherwise, 2 when a library or the part source can't be read.

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

The app's Footprint review tab does the review itself: it draws a footprint over the datasheet's drawing, and Accept
records a review of its hash for the MPNs it fits in the part source, as `berc fp accept` does. The part source is a
setting (Settings, Review record: part source), a file in the parts snapshot's shape; without it Accept is off and
says so.

1. **Pick what to review**: open a board or project (`.kicad_pcb`, `.kicad_pro`, `.kicad_sch`) for its footprint
   hashes and covers, one row each as `berc fp todo` lists them, those to review first; or a symbol library (a
   `.kicad_sym`, a `.kicad_symdir` folder, or a nickname from the `sym-lib-table`s) for its symbols, judged as
   `berc fp check --lib` judges them against the part source. Each row shows its status (changed, uncovered,
   partial, ok, unknown MPN, no MPN, missing, no footprint). A board row opens the board's copy of the footprint,
   the one its first reference places: that is what the pairs are of. A note above the canvas says when the board
   places a different footprint than the symbol's own `Footprint` field names, and when the board's copy differs
   from its library footprint (the board not updated from the library yet). A library row opens the library
   footprint its `Footprint` field names, found through the project's or the global `fp-lib-table`. A symbol that
   sets no footprint, or whose symbol or footprint is not found, is refused with the reason. Next to review steps to
   the next row that needs one. A row lists the part source's reviews that name it (what each was made against, its
   MPNs, ok or of an earlier hash) and the MPNs no review covers yet.
2. **Open the datasheet**: the symbol's `Datasheet` field when it names a local file, a local document library
   search, or Open PDF. The library search runs a command (a setting) with the datasheet's file name, a part name
   in its web address, the `Mpn` fields or the value, until one finds something. The command prints a JSON list of
   `{"path": ..., "page": ...}`. What it finds is listed in a panel of its own, and for a symbol just picked the
   first one opens by itself. Picking another symbol opens its datasheet, or closes the last one when it has none.
   A row whose review recorded its datasheet opens that PDF first, found in the document library by its content
   hash, else by its document number, and with it the view the review was made in: the page, the scale and the
   footprint's placement. So does opening any datasheet a review was made on. A review of an earlier hash of the
   footprint is said so, and a recorded PDF the library doesn't have is said too. The library's
   index comes from a command (a setting, `tpm lib docs --json --all` by default) that prints a JSON list of
   `{"path", "hash", "doc", "rev"}`, paths relative to the library's root; the file on disk is then found through
   the library name command below, which prints full paths.
   A symbol with `Mpn` fields gets a table, Datasheets by MPN, at the top of the Datasheet section: each MPN, the
   library document that covers it, and its review state (reviewed, reviewed elsewhere when its review was made on
   another document, stale when only a review of an earlier hash covers it, or not yet). Which document covers an MPN is the document library's answer, not
   better_erc's: a command (a setting, `library_mpn`, `tpm lib mpn --json` by default) gets the MPNs at the end and
   prints a JSON list of `{"mpn", "path", "hash", "doc", "rev", "why"}`, one object per document of an MPN, best
   first, or one with no `path` and `why` it has none. A series or catalogue file may cover several MPNs. Each MPN
   is asked once per session. Clicking a document opens it with the view of its review, else the page search
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
9. **Accept** (A): when the footprint matches the drawing, Accept records a review of its hash in the part source,
   and for a board row refreshes the project's parts snapshot from it (as `berc fp accept` does); the list is read
   again and the row shows it reviewed. The review records where it was made: the datasheet by the document
   library's content hash, document number and revision, the page, the scale, the footprint's placement, the review
   note, who (a setting, else the login name) and the day. It asks first which MPNs this datasheet covers, and
   for each whether alone or its whole family (where its MPN decodes into one). A board row's MPNs share one cover:
   all ticked. A library row's MPNs found in the PDF's text are ticked, and those a review made on this datasheet
   covers, except an MPN that a review on another datasheet covers. The question shows what its dry run would
   record, following the ticks: the part source, what the review covers, the references, the MPNs left out. Each
   datasheet keeps its own view of the row (page, zoom, scale, placements, ruler, note), saved with the tab:
   opening another datasheet and coming back finds the first as it was left, accepted or not. When the board places
   a different footprint than the symbol's own `Footprint` field for every reference, a checkbox offers to also set
   that field to the placed one, showing both; checked, it writes the same as `berc fp accept --footprint` does,
   and lists the files. It then warns, per file and naming the lock file, when KiCad may overwrite that write on its
   next save: a file with KiCad's lock file beside it (`~name.kicad_sch.lck`, open in an editor there) that is held.
   A lock from this computer is held while a KiCad runs here (KiCad's locks name the user and host, no process id;
   a lock with a `pid` while that process lives); one from another computer always counts; an empty one is stale,
   as KiCad takes it. KiCad running with no lock beside the file is no warning. Enter (or Record) records; Esc (or
   Cancel) records nothing. Once recorded, it names the review and the snapshot refreshed, or says why it recorded
   nothing, copies the hash to the clipboard and keeps the review (footprint, hash, datasheet, page, scale) in the
   tab's list. Accept stays disabled, saying why, until a row with MPNs and a footprint is picked, a part source is
   set, a datasheet is open and the scale is set on the page shown.

#### Files changed on disk

The tab shows the files as they are on disk. It watches every file it read: the board and its schematics, the library
tables, the symbol libraries, the footprint files, the 3D models and the datasheet. A file saved in KiCad or another
editor is read again by itself once it stays quiet for a moment, and picking a symbol or a board footprint looks at
the files first. Reload (`F5`, a setting, or the button under the Open buttons) reads everything again; a line beside
it says what was read again and when. A reload keeps the view: the page, the scale, the footprint's placement, the 3D
camera and the list's filter. The review status follows the file, so a footprint changed since its review shows the
review as changed. A file counts as changed only when its content changed; what Accept writes reloads nothing.

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

A `.kicad_sch`'s cached library symbols (`lib_symbols`) print their fields - `Datasheet`, `Footprint`, `Mpn1`,
... - as their own `Name=value` lines, a hash underneath for the rest of the symbol (pins, graphics, sub-symbols).
`berc fp accept --footprint` (see [Accepting a review](#accepting-a-review)) writes the `Footprint` field into every
sheet's `lib_symbols` copy, so this is what turns that fix into a one-line diff instead of a changed hash taken on
faith.

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
