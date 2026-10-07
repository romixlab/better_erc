# better_erc


## Readable git diffs

`git diff` of a `.kicad_sch` or `.kicad_pcb` is mostly noise: UUIDs, instance paths, coordinates, items written in
a new order on every save. `berc textconv` turns one KiCad file into sorted, line-oriented text (one fact per line,
no UUIDs, no coordinates), and git can diff that instead. The stored files don't change; `git diff`, `git log -p`,
`git show` and lazygit all show the readable form.

Install the binary (from this repo):

```sh
cargo install --path berc
```

Tell git which files to convert, in the repo with the KiCad files (`.gitattributes`, committed, or
`.git/info/attributes` for yourself only):

```gitattributes
*.kicad_sch diff=kicad
*.kicad_pcb diff=kicad
```

And how to convert them, once per machine:

```sh
git config --global diff.kicad.textconv "berc textconv"
git config --global diff.kicad.cachetextconv true
```

`cachetextconv` keeps the converted text in git notes (`refs/notes/textconv/kicad`), so `git log -p` over a long
history converts each file version only once. `git diff --stat` and `--numstat` still count raw lines; that is
git, not berc. To see the raw diff once, pass `--no-textconv`.

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
- Anything berc doesn't know is printed as a one-line s-expression in `# other`, so nothing is silently dropped.
  A file that isn't a KiCad s-expression comes out as it is.

`berc textconv --with-geometry FILE` adds positions (footprints rounded to 0.1 mm), wires and tracks, for when
placement is what you are reviewing. Shell completion: `source <(COMPLETE=bash berc)` (or `zsh`, `fish`).
