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
- **The app** (`better_erc`, egui, crate `better_erc_app`): an import tab that opens any design the registry reads
  (and Altium or OrCAD files picked one by one) and shows its findings with their explanations and the checks that
  did not run, and tabs for nets (power rails, grounds and switching nodes from names and pin types), I2C buses,
  style findings and inputs.
- **The library** (crate `better_erc`): all of the above for Rust code in one dependency.

## Documentation

- [docs/checks.md](docs/checks.md): the check catalogue, everything better_erc should catch on a schematic or
  netlist, by ID (`PIN-1`, `I2C-3`) and severity.
- [docs/engine.md](docs/engine.md): the check engine: checks and their data needs, findings, the report, the test
  schematics with their expectations, how to add a check.
- [docs/model.md](docs/model.md): the board model, the importers and the crates.
- [docs/review-checklist.md](docs/review-checklist.md): a condensed schematic review checklist that groups those
  IDs for a review pass.
- [FEATURES.md](FEATURES.md): what the code does today and what is planned, item by item.
- [CHANGELOG.md](CHANGELOG.md): what changed in each version.

## Install

You need a Rust toolchain (1.86 or newer, from [rustup](https://rustup.rs)). From a clone of this repository, with
[just](https://github.com/casey/just) and tpm: `just install` builds both binaries and puts each on PATH once
(kept releases in `~/.local/bin`). Without tpm:

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
git config --global diff.kicad.cachetextconv true
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
git config --global diff.kicad.cachetextconv true
```

`cachetextconv` keeps the converted text in git notes (`refs/notes/textconv/kicad`), so `git log -p` over a long
history converts each file version only once. `git diff --stat` and `--numstat` still count raw lines; that is
git, not berc. To see the raw diff once, pass `--no-textconv`.

The cache is keyed only by the blob's hash, not by which berc version produced the note: after a `berc` upgrade
that changes the output format, old blobs keep showing their old (but still cached) textconv text until the note
is dropped. There's no fix for this inside berc - git never calls textconv again for a blob it already has a
note for, no matter what the command would print now - so after an upgrade that changes output, clear it once
per clone:

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
