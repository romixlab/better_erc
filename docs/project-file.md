# The project file

A design says a lot about itself, but not everything a review needs: what voltage a net carries when its name does
not say, how much margin a part must keep below its rating, which checks a project runs differently, and which
findings were looked at and accepted. The project file holds these facts, next to the design and committed with it,
so a CI run, another PC and a reviewer all see the same ones.

## Where it is

`<name>.berc.toml` next to `<name>.kicad_pro`, else `berc.toml` in the same directory. For a design opened from
another file (a `.kicad_sch`, a netlist) `<name>` is that file's name without its extension. `berc check`,
`berc place-fields` and `berc waive` read it whenever they open the design; the library does the same in
`Project::open` (`Project::file`).

A file that is not TOML, or a key with a wrong value, stops the run with exit code 2 and a message naming the file and
the key:

```text
berc: error: hw/board/board.berc.toml: derating.resistor_p: a factor from 1 to 10, not 0.8
```

A key better_erc does not know is a warning, shown under the report's notes (`project board.berc.toml: unknown key
`intent`, ignored`), so a file written for a newer version still works with an older one.

## An example

```toml
[nets."+3V3"]
voltage = 3.3
[nets.VBAT]
voltage = { min = 3.0, max = 4.2, nominal = 3.7 }

[derating]
ceramic_capacitor_v = 2.0

[checks]
disabled = ["NAME-5"]
severity = { "I2C-3" = "warning" }
[checks."I2C-9"]
min_pull_up = 2200

[drawing.fields]
hide_defaults = "always"

[parts]
lock = "parts.lock.toml"

[[waiver]]
fingerprint = "I2C-1:net /SDA|part R5"
who = "Jane Doe"
date = 2026-11-25
reason = "the pull-up sits on the mating board"
expires = 2027-06-30
```

## `[nets]`: net voltages

One table per net, by the net's name as the schematic has it (`"+3V3"`, `"/I2C/SDA"`; names with characters other
than letters, digits, `_` and `-` are quoted). `voltage` is one value in volts, or a range:

| Form | Meaning |
|---|---|
| `voltage = 3.3` | the net is at 3.3 V |
| `voltage = { min = 3.0, max = 4.2, nominal = 3.7 }` | the net moves between min and max; checks compute with nominal |
| `voltage = { min = 9, max = 36 }` | a range without a nominal value: checks that need one value fall back on the net name |

A net with a voltage is a rail in the power analysis (of the kind its name reads as, a supply otherwise), or a
ground when the voltage is 0 V and the name reads as one. The project's voltage ranks above the one the name gives
(`+3V3` read as 3.3 V); both are kept, so a finding names where its voltage came from
(`nets."+3V3".voltage`, project file). A net name no board has is a note in the report.

## `[derating]`: margin below the rating

Each factor `k` asks for a rating of at least `k` times the working value: with `ceramic_capacitor_v = 1.5`, a
capacitor on a 3.3 V rail needs a rating of 5 V or more. A factor runs from 1 to 10; keys the file does not set keep
their default.

| Key | Default | Parts and rating |
|---|---|---|
| `ceramic_capacitor_v` | 1.5 | Ceramic capacitors (class I and II), rated voltage over working voltage |
| `electrolytic_capacitor_v` | 1.25 | Aluminium electrolytic capacitors, rated voltage over working voltage |
| `polymer_capacitor_v` | 1.25 | Aluminium polymer capacitors, rated voltage over working voltage |
| `tantalum_capacitor_v` | 2.0 | Tantalum capacitors with an MnO2 cathode, rated voltage over working voltage |
| `tantalum_polymer_capacitor_v` | 1.25 | Tantalum polymer capacitors, rated voltage over working voltage |
| `film_capacitor_v` | 1.25 | Film capacitors, rated voltage over working voltage |
| `resistor_p` | 2.0 | Resistors, rated power over dissipated power |
| `resistor_v` | 1.25 | Resistors, limiting element voltage over working voltage |
| `inductor_isat` | 1.3 | Inductors, saturation current over peak current |
| `inductor_irms` | 1.25 | Inductors, rated (heating) current over RMS current |
| `ferrite_bead_i` | 1.25 | Ferrite beads, rated current over working current |
| `diode_vr` | 1.25 | Diodes, rated reverse voltage over working reverse voltage |
| `mosfet_vds` | 1.25 | MOSFETs, rated drain-source voltage over working voltage |

The rating checks read these through `Ctx::derating`, and name the factor and its source in their findings.

## `[checks]` and `[drawing]`: check settings

- `disabled`: check ids not to run.
- `severity`: a severity per check id (`error`, `warning`, `suggestion`, `info`) over the catalogue's.
- `[checks."ID"]`: the check's thresholds by the names it declares, the same as `--set ID.NAME=VALUE` takes
  (`[checks."I2C-9"] min_pull_up = 2200`).
- `[drawing]`: the settings several checks and `berc place-fields` share, by their dotted names
  (`[drawing.fields] hide_defaults = "always"` is `drawing.fields.hide_defaults`).
  `notes = "../doc/notes.md"` names the board's notes file that `DRAW-50` reads, relative to the project directory;
  several files are separated by commas.

A check id the catalogue does not have is a warning; a threshold no check declares is an unknown key. `--set
NAME=VALUE` on the command line wins over the file.

## `[parts]`: the parts snapshot

- `lock`: where the parts snapshot is, relative to the project directory (default `parts.lock.toml`). A path outside
  the directory is refused. `berc parts refresh` writes it there and `berc check` reads it from there; see the README,
  "Part facts: the parts snapshot".

## `[[waiver]]`: accepted findings

A waiver accepts one finding for this project. The finding stays in the report, marked `[waived]` with who accepted it
and why, and no longer fails the run or counts in the error and warning totals (the summary counts it as waived).
SARIF marks it as an accepted external suppression.

| Key | Required | Meaning |
|---|---|---|
| `fingerprint` | yes | the finding's fingerprint: check id and the sorted names of its subjects (`I2C-1:net /SDA\|part R5`), as the JSON report gives it |
| `who` | yes | who accepted it |
| `date` | yes | when (`2026-11-25`) |
| `reason` | yes | why it holds on this design |
| `input_hash` | no | the finding's input hash when it was judged |
| `expires` | no | the last day it applies |
| `board` | no | only on this board, for a design with several |

The fingerprint does not change when a sheet is redrawn or parts move, only when the subjects change. So a waiver
can outlive what it judged: a capacitor accepted at 6.3 V on a 3.3 V rail would stay accepted after the rail became
5 V. Checks that judge values (ratings, part data against requirements) give their findings an input hash: the hash of
every value the verdict rests on, written `in1:` and 16 hex digits and shown under the finding in the text report.
A waiver that holds an input hash applies only while the finding's hash is the same; when any input changes, the
waiver is stale, the finding counts again, and the report says so:

```text
project board.berc.toml: waiver of RATE-1:part C4 (Jane Doe, 2026-11-25) is stale, the finding stands: its inputs changed (in1:3f0c... then, in1:91aa... now)
```

An expired waiver does not apply either, and a waiver that matches no finding (fixed, or its subjects renamed) is a
note, so the file can be cleaned up. The JSON report lists every waiver with what it met (`waivers`) and each
finding's waiver with its status (`applied`, `stale`, `expired`).

The first answer to a finding is a better design; a waiver is for what holds on this design only.

### `berc waive`

```sh
berc waive PROJECT FINDING [SUBJECT...] --reason TEXT [--who NAME] [--expires DATE] [--variant V] [--all]
```

Checks the design, picks the finding and appends its waiver to the project file, creating `<name>.berc.toml` when
there is none. The rest of the file is left as it is. `FINDING` is a check id with subject names that pick one
finding (`berc waive hw/board I2C-1 R5`), or a fingerprint (`berc waive hw/board 'I2C-1:net /SDA|part R5'`). The
waiver keeps the finding's input hash. `--who` defaults to `git config user.name`; `--all` waives every finding that
matches instead of refusing when several do.

| Code | Meaning |
|---|---|
| 0 | written |
| 1 | no finding matches, it is already waived, or several match without `--all` |
| 2 | usage, import or write error |

## For checks: input hashes and derating

A check that judges values names each input of its verdict on the finding builder, and the builder hashes them:

```rust
cx.finding(&META)
    .part(c)
    .input("C4.Vworking", vworking)
    .input("Mpn1.rated_voltage", rated)
    .input("derating.ceramic_capacitor_v", cx.derating().ceramic_capacitor_v)
    .fact("derating", k, cx.derating().source("ceramic_capacitor_v"))
    .message(..)
    .build()
```

The hash is `erc_checks::finding::input_hash`: the `name=value` lines sorted and hashed, so the order of the inputs
does not matter and any changed name or value gives another hash. Name every value the verdict rests on, and only
those: a value that does not change the verdict would make waivers stale for nothing.
