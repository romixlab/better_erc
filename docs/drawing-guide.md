# Schematic drawing guide

How to draw a schematic sheet that reads at a glance: where the signal enters and leaves, which way power flows, what
every part is and why it was chosen. better_erc checks what a schematic does; this page is about how it looks, because
a reviewer who cannot read a sheet quickly misses what is wrong on it. The rules that better_erc can check from the
drawing are in the [check catalogue](checks.md#draw-1) as `DRAW` checks, explained on
[docs/checks/draw.md](checks/draw.md); connectivity hygiene (labels, flags, buses) is `WIRE`, on
[docs/checks/wire.md](checks/wire.md).

Most rules are obvious once said: they get the rule and a one-line reason, with the check that measures it. Rules
that are not obvious (a KiCad quirk, a house convention with exact values, a setting) get a sketch, the reasons and
the source. The tag on those says how firm they are:

- **standard**: a published standard, convention or tool default says so (the source is named).
- **house**: what our own sheets do, measured over a private set of 157 production sheets (`N/M` sheets, see
  [House style measured](#house-style-measured)), or set by Roman (dated).
- **new**: our recommendation where neither of the above settles it, with the reason.

The sketches use `>` and `<` for label and pin direction, `*` for a junction, `┴` for a ground symbol and `┬` for a
supply symbol. KiCad names follow KiCad 10.0; most of it holds for KiCad 8 and 9.

## Contents

- [The sheet](#the-sheet): paper, one function per sheet, sections with headings, fill, title block, names
- [Notes, boxes and tables](#notes-boxes-and-tables): design intent, coloured boxes by kind, tables
- [Grid, alignment and spacing](#grid-alignment-and-spacing): 1.27 mm grid, aligned neighbours, clearance margins,
  no pin on pin
- [Signal flow](#signal-flow): left to right, connectors at the edge, symbol orientation, small parts to a rail
  vertical
- [Power and ground](#power-and-ground): ground down, supplies up, power net names, one symbol per rail per part,
  the vhrd_power symbols and rail colours, decoupling
- [Wires and junctions](#wires-and-junctions)
- [Labels and hierarchy](#labels-and-hierarchy): labels lying on their wire, scopes, sheet pins, buses for groups
- [Text](#text): fields, sizes, overlaps
- [Parts](#parts): unused pins, DNP, multi-unit parts, simulation-only parts and directives
- [Colour, print and export](#colour-print-and-export)
- [KiCad settings and shortcuts](#kicad-settings-and-shortcuts)
- [What other tools automate](#what-other-tools-automate)
- [House style measured](#house-style-measured)
- [Sources](#sources)

## The sheet

### Paper size: A4 or A3, one size per project

A4 or A3 landscape, one size for the whole project; split a function that does not fit an A3. Why: anything bigger
does not print readably on a desk printer. `DRAW-21`

### One function per sheet

Each sheet holds one function (power input, a regulator group, the MCU, one interface); split it before it gets
crowded. Why: a reviewer checks a sheet against one datasheet in one pass. `DRAW-18`

### Sections with a heading, key numbers under it

**Rule.** Group the parts of a sheet into sections by function, with a clear gap between sections (at least 10 mm,
more than the gaps inside a section). Each section carries a heading above its top-left corner that names the
function (`DC-DC 3V3`, `Low-pass filter`, `Clamp bus`). Under the heading, and only when the section has numbers
that matter for review (output voltage, corner frequency, trip current), a small parameter box lists them, one per
line. Everything else stays off the sheet or goes in a note box ([below](#notes-live-in-boxes-by-kind)).

**Why.** The heading says what a group of parts is before the reader has to work it out from the parts; the gap
says where it ends. The numbers next to the heading are what a reviewer checks first. Text sprinkled everywhere
hides both.

**Example.**

```
bad:   R1 C1 R2 C2 Q1 U1 U2 spaced evenly over the sheet, "fc = 1.6 kHz" next to C1, "Vout 3.3 V" under U2

good:  Low-pass filter              DC-DC 3V3
       ┌──────────────┐             ┌──────────────┐
       │ fc = 1.6 kHz │             │ Vout = 3.30 V│
       └──────────────┘             │ Iout ≤ 1.5 A │
       ──[R1]──*──[R2]──*──         └──────────────┘
               C1       C2          [U2] [L1] [C5]
```

Heading: free text, KiCad default font, **2.54 mm, bold**, theme colour, justified left bottom, 2.54 mm above the
section's top edge. Parameter box: a KiCad text box, 1.27 mm normal text, stroke 0.1524 mm solid in the note grey
(`rgb(110, 110, 110)`), no fill, right under the heading. Heading and box are a KiCad group, so they move together.

**Source.** house (Roman, 31 Oct 2026); EMA and Altium for blocks by function. Check: `DRAW-31` (sections without a
heading, candidates).

### The drawing is centred and fills the sheet

Centre the circuit with even margins and pick a paper size it fills about half of; nothing touches the frame or the
title block. Why: a circuit in one corner prints small and gets lost. `DRAW-16`, `DRAW-17`

### Title block on every sheet

Title, revision, date and company on every sheet, the revision set once for the project and shown through
`${REVISION}`. Why: a printed page is read alone. `DRAW-19`, `DRAW-20`

### Sheet names say the function

`USB-C input`, not `Sheet1`; the file named after the sheet; an index sheet first in a project of more than three or
four sheets. Why: the name is the navigator entry, the PDF bookmark and the page title. `DRAW-25`

## Notes, boxes and tables

### Notes give the design intent

Write down why a part is there and how its value was found: the divider formula with its result, the filter corner,
why this TVS. Why: a value without its calculation gets "fixed" in the next revision. `DRAW-26`

Where it goes: the numbers of one section in its parameter box ([sections](#sections-with-a-heading-key-numbers-under-it));
the reasoning that spans sections, open questions and instructions in a note box by kind (below), next to the
section it is about, or in one column of boxes at the edge of the sheet when it concerns the whole sheet.

### Notes live in boxes by kind

**Rule.** No free-standing text on a sheet apart from section headings: every note is a KiCad text box of one of six
kinds, each with its colour, and a bold heading naming the kind. Long notes are split into several boxes, one per
topic.

| Kind | Use for | Stroke colour | b125 original | Stroke |
|---|---|---|---|---|
| TODO | open work on the sheet | `rgb(0, 131, 143)` teal | `rgb(0, 194, 194)` | 0.1524 mm dash |
| Information | design intent, how a value was found, how the circuit works | `rgb(110, 110, 110)` grey | `rgb(72, 72, 72)` | 0.1524 mm dash |
| Caution | a trap for the next designer: do not change to X5R, keep this order | `rgb(167, 118, 1)` amber | `rgb(194, 194, 0)` | 0.1524 mm dash |
| Debug | test points, jumpers, bring-up and simulation-only setup | `rgb(232, 113, 10)` orange | `rgb(221, 133, 0)` | 0.1524 mm dash |
| Critical | a mistake here breaks or damages the board | `rgb(210, 40, 70)` crimson | `rgb(194, 0, 0)` | 0.254 mm dash |
| Layout | placement and routing instructions for the layout | `rgb(21, 101, 192)` blue | `rgb(0, 0, 194)` | 0.254 mm dash |

The six kinds and their dashed, unfilled boxes come from the front page of b125 (`vb125_eth_fdcan_pro.kicad_sch`).
The colours are moved onto our KiCad theme (`vhrd.json`, `vhrd_dark.json`): TODO is the theme's current-probe teal,
Information its note grey, Caution its ERC-warning amber, Debug its voltage-probe orange, Critical its ERC-error
crimson (lightened from `rgb(169, 4, 62)`, which drops to 2.5:1 on the dark theme), Layout its bus blue, so a box
reads as the same thing as the theme item it borrows from. Every colour keeps at least 3:1 contrast on white (PDF)
and on the VHRD Dark background `rgb(15, 15, 16)`.

Each box is a group of two items:

- **heading**: free text, the kind's name (`Caution`, optionally `Caution: back-feed`), KiCad default font, 1.27 mm,
  **bold**, in the kind's colour, justified left top, inset 0.95 mm from the box's top-left corner;
- **body**: a text box, KiCad default font, 1.27 mm, normal, theme text colour, justified left top, no fill, the
  kind's stroke, margins 0.95 mm with 3.2 mm at the top to leave room for the heading; width and height on the
  1.27 mm grid.

KiCad's default stroke font is used everywhere (no `face`): it plots the same on every PC and in every PDF; the b125
title font (Orbitron) stays on the front page logo.

**Why.** Loose text has no owner and no weight: a reviewer cannot tell a design reason from a forgotten TODO or a
layout instruction that must not be missed. A box by kind says at a glance what the text is and how much it
matters, and the layout engineer reads the blue boxes and nothing else. The colour only repeats what the heading
says, so a black and white print loses nothing ([colour](#colour-adds-never-carries-meaning-alone)).

**Source.** house (Roman, 31 Oct 2026; the b125 front page). Checks: `DRAW-32` (free text, or a box not of the six
kinds).

### A note about a part points at it

A note box that is about a particular part or block draws a leader to it: a dotted line in leader purple
`rgb(142, 92, 205)` (4.6:1 on white, 4.2:1 on VHRD Dark), 0.1524 mm, with an arrowhead at the target, bent in
orthogonal or 45° segments around wires, parts, labels and text, never through them. A block pointed at this way
gets its own rectangle (same purple, dotted, 0.1524 mm, no fill) around it. KiCad 10 has no arrowheads on lines:
the head is a small filled triangle (a closed polyline, 1.27 mm long, 0.9 mm wide, filled with the stroke colour)
grouped with the line and the box. Why: the reader sees which part a note is about without matching references
by eye. `DRAW-37`, `DRAW-38`

### Tables, not lumps of text

Values that line up in rows and columns (pin options, a parameter sweep, a configuration matrix) go in a KiCad table
(Place > Table, KiCad 8 and later), not in a text block padded with spaces. Why: a table stays aligned in every font
and every PDF. `DRAW-33`

## Grid, alignment and spacing

### Everything that connects sits on the 1.27 mm grid

Symbols, wires, labels and sheet pins on 1.27 mm (50 mil); a finer grid only for text and graphics. Why: KiCad
connects only coinciding points, and an item half a step off looks connected and is not. `DRAW-3`, `WIRE-3`

### Neighbours line up exactly, or clearly not

Alignable things near each other share a column (x) or a row (y) *exactly*: label anchors, power and ground
symbols, the vertical wires of ties, part edges and centre lines. "Near" is on the same side of the same part, or
within `drawing.align.window` (default 20.32 mm) of each other. An offset that is not zero but under
`drawing.align.snap` (default 5.08 mm, two major grid steps) is a near miss and gets fixed; a larger offset is
deliberate and fine. Select them and use **Align** (KiCad 10). Why: a step off reads as careless and hides what
belongs together, while a clear offset reads as a different group. Source: house (Roman, 31 Oct 2026: `+3V3_LDO`
and a GND on the same side of the TMUX1511 in different columns, a label start off its tie column). `DRAW-36`,
`DRAW-10` (label columns)

### Room around everything: clearance margins

**Rule.** Every item keeps a clear margin around it that nothing else enters: symbol bodies, power and ground
symbols, fields and labels. Between sections the gap is bigger ([sections](#sections-with-a-heading-key-numbers-under-it)).

**Why.** A sheet where every gap is the minimum reads as one block, and a field 0.3 mm from the next part's pin is
read as belonging to it. Dense sheets (our io_protection sheets) look tidy at the editor's zoom and become a puzzle
at page width.

**Settings.** The margins are named drawing settings with defaults, used by every spacing check (`DRAW-9`,
`DRAW-34`); a project sets its own in its better_erc project file:

| Setting | Default | Margin around |
|---|---|---|
| `drawing.clearance.part` | 1.27 mm | a symbol's body and its pins |
| `drawing.clearance.power` | 1.27 mm | a power or ground symbol and its name |
| `drawing.clearance.text` | 0.635 mm | a field, a label, a pin name or number, a note box |

Two items are too close when their boxes, each grown by its own margin, overlap: two parts closer than 2.54 mm (one
major grid), a field closer than 1.905 mm to another part, two labels closer than 1.27 mm. A part's own fields and
pin names against its own body are exempt (they sit next to it on purpose); a label against the wire it lies on is
exempt.

```
bad:    +3V3 ┬                      good:        ┬ +3V3
        ─────*──[R5 10k]──              ───*─────┘
             (name over R5: the power       │         (stub long enough
              symbol on a one-step stub     ──[R5 10k]──  that the name clears R5)
              beside a series part)
```

**Source.** new (Roman, 31 Oct 2026: io_protection is too dense, make it configurable, do not overdo the default);
standard: AoE (space around symbols). Checks: `DRAW-34`, `DRAW-9`.

### Pins never join pins

Two pins meet through at least one major grid step (2.54 mm) of wire, never pin end on pin end. Why: a pin-on-pin
joint is invisible, has nowhere to add a branch, and opens silently when one part is moved. `DRAW-29`

### Straight out of a pin, straight into a power symbol

A wire leaves a pin along the pin's own axis for at least one major grid step (2.54 mm) before its first bend, and
enters a pin or a power or ground symbol the same way: no bend right at a pin end. Why: the direction of the pin
and of the symbol stays readable, and a corner at a pin end is not mistaken for a junction. Source: house (Roman,
31 Oct 2026, the VDD pin of the io_protection TMUX1511 bending straight up into `+3V3_LDO`). `DRAW-41`
(`drawing.pin_lead`, default 2.54 mm)

## Signal flow

### Signals run left to right

Inputs on the left, outputs on the right, feedback above or below the forward path. Why: the sheet reads like a
sentence and a missing part in the path stands out. `DRAW-12`

### Power flows left to right, high to low voltage

Input on the left, the rails it makes to the right in falling voltage. Why: the sheet reads as the power tree.

### Connectors at the sheet edge, with their protection beside them

Connector at the edge its signals come from, its TVS, fuse or series resistor right next to it, between it and the
rest. Why: the reviewer sees at once that every line passes its protection first. `DRAW-14`

### ICs as the library drew them

Never mirror an IC or a multi-pin part; rotate two-pin parts freely; mirror only symmetric parts and transistors.
Why: library symbols put inputs left and outputs right, and a mirrored IC runs the flow backwards. `DRAW-12`

### Small parts to a rail or ground stand vertical

Diodes, TVS, capacitors, resistors and other 2- to 4-pin parts with a pin on a supply or ground (a BAT54S clamp to a
rail and to ground, a pull-down, a TVS to ground) are drawn vertical: rail end up, ground end down. Why: the part
then sits on the vertical voltage axis and reads as a shunt, not as a part in the signal path. `DRAW-35`

## Power and ground

### Ground symbols point down, supply symbols point up

Ground below its pin pointing down, positive supplies above pointing up, negative supplies down. Why: the vertical
axis reads as voltage. `DRAW-1`, `DRAW-2`

### Power net names: +VOLTAGE_SOURCE

**Rule.** A power net is named `+<voltage>_<group or source>`, the voltage with `V` as decimal point: `+3V3_LDO`,
`+1V8_REF`, `+5V_USB`, `+12V_IN`, `+3V3_MCU`; negative rails `-5V_<source>`. The suffix says where the rail comes
from (`LDO`, `BUCK`, `USB`, `IN`, `REF`) or which group it feeds (`MCU`, `ANA`, `ETH`). A plain `+3V3` is fine on
a board with one 3.3 V rail. Grounds are `GND`, `AGND`, `PGND`, `GND_<group>`.

**Why.** The name is all a reader sees at a power symbol. Two 3.3 V rails both called `+3V3_x` show they are
different nets with the same voltage, and the suffix says which regulator to look at; a name without the voltage
(`VCC`, `V_MCU`) sends the reader to the power tree for every pin. The `+` and the `V` as decimal point are what our
tools and KiCad's simulator parse as a voltage (`NAME-7`, `NAME-11`).

**Example.**

```
bad:   VCC, V3V3, 3.3V_A, VDD_LDO
good:  +3V3_LDO, +3V3_BUCK, +1V8_REF, +5V_USB, -5V_CP
```

**Source.** house (Roman, 31 Oct 2026). Check: `NAME-13`; `NAME-3` (no voltage in the name), `NAME-11` (negative
rail without its sign).

### One power symbol per rail per pin group

**Rule.** Several pins of one part on the same rail (the four GND pins of a connector, the VDD pins of an IC) are
wired together with short stubs and reach that rail through one power or ground symbol, not one symbol per pin.
The unit is a *pin group*: pins of that rail next to each other on one side of the body. Two groups on different
sides get a symbol each when one shared symbol would need a detour that costs more than the second symbol
([routing is a weighed compromise](#routing-is-a-weighed-compromise)). Other parts keep their own symbols; a
decoupling capacitor keeps its own.

**Why.** Twenty ground symbols mashed under one IC are noise: the reader counts symbols instead of seeing that
"all ground pins go to ground". One wire collecting the pins says it once and leaves room for the pins that matter.

**Example.**

```
bad:    GND ┤1   ├                  good:   ┌┤1   ├
         ┴  │    │                          *┤2   ├
        GND ┤2   ├                          *┤3   ├
         ┴  │    │                          ││    │
        GND ┤3   ├                          ┴ GND
```

**Source.** house (Roman, 31 Oct 2026, on the io_protection TMUX1511 and its four grounded selects); new for the
exception of decoupling. Checks: `DRAW-30`, `DRAW-40`.

### Power symbols, not long wires, to reach a rail

Reach a rail through a power symbol at the part, not a wire across the sheet. Why: a rail wired across a sheet
crosses every signal.

### Power symbols from vhrd_power, rails coloured by net class

**Rule.** Power and ground symbols come from our library `vhrd_power` (`~/git/kicad-library`): `GND` for ground
and the board rails, `PWR_LOCAL` (a KiCad 10 *local* power symbol, its value is the net name) for a rail that must
stay on its sheet: a simulation bench, a sheet that stands for a board of its own. The project's net classes colour
the rails, not the symbols one by one:

| Net class | Patterns | Wire colour | Symbol colour |
|---|---|---|---|
| Ground | `GND` | `rgb(0, 115, 30)` dark green | `GND` in a lighter green `rgb(60, 170, 24)` |
| Power | `+*`, `*/+*` (a local rail is `/<sheet>/+3V3_LDO`) | `rgb(175, 50, 10)` burnt orange | `PWR_LOCAL` and the other `vhrd_power` supplies in orange `rgb(232, 113, 10)` |

The symbol orange is the Debug box orange and the theme's voltage-probe colour, the one the `vhrd_power` supply
symbols already had. A symbol and its wires differ by more than
`drawing.colour.pair_min` ([neighbours in different colours](#neighbours-in-different-colours)), and all four keep
about 3:1 on both the white and the VHRD Dark background, since net class and library colours do not change with
the theme.

**Why.** A reader finds the rails of a sheet at a glance, and a power net named after the rule above
([power net names](#power-net-names-voltage_source)) is coloured without anyone setting a colour by hand. The
colour only repeats what the `+` in the name says ([colour](#colour-adds-never-carries-meaning-alone)). A local
power symbol keeps the per-sheet rails of a bench apart without hierarchical labels.

**Source.** house (Roman, 31 Oct 2026: the io_protection project's net classes, the `vhrd_power` symbols).

### Decoupling next to the pin it serves

Each decoupling capacitor beside its power pin, in the order of the pins. Why: the layout is placed from the
schematic, and caps drawn in a pile end up in a pile. `DRAW-15`, `LAY-17`

## Wires and junctions

### Wires are horizontal or vertical

90° line mode only. Why: diagonals cross more and miss the grid at joints. `DRAW-5`

### No 4-way junctions

Stagger four wires into two T junctions, each with a dot; turn on KiCad's `four_way_junction` ERC check. Why: a
4-way joint and a crossing differ only by a dot that a print can lose. `WIRE-15`, `DRAW-4`

### Crossings: a plain cross never connects

Avoid crossings; on busy sheets set a hop-over size (KiCad 10, Schematic Setup > Formatting). Why: each crossing is a
place the eye jumps to the wrong wire.

### Wires never run through a symbol, a label or text

Why: a wire over a body looks like a pin connection, over text it makes both unreadable. `DRAW-7`

### Routing is a weighed compromise

**Rule.** Where a tie can be drawn more than one way (pins to a power symbol, a few pins tied together), the
cheaper drawing wins, by a cost the guide fixes and better_erc scores:

| Term | Setting | Default | Counts |
|---|---|---|---|
| length | `drawing.route.w_len` | 1 | per major grid step (2.54 mm) of wire |
| bend | `drawing.route.w_bend` | 2 | per bend |
| crossing | `drawing.route.w_cross` | 4 | per crossing with another net: a crossing is worth about four grid steps of detour |
| crossing near a pin or junction | `drawing.route.w_cross_near` | 8 | instead of `w_cross` when the crossing is within one grid step of a pin end or a junction, where it reads as a connection |
| extra symbol | `drawing.route.w_symbol` | 6 | per power or ground symbol beyond the first of that rail on the part |
| detour | `drawing.route.w_detour` | 4 | per unit of (drawn length / Manhattan length − 1) of the tie |

The Manhattan length of a tie is the rectilinear distance it must cover at least (the half-perimeter of the box
around its pins and symbol). So a straight drop to ground that crosses one stub costs less than a loop around the
part, and a second ground symbol costs less than a long way round.

**Why.** "Avoid crossings", "one symbol per part" and "short wires" pull against each other; stated as costs, the
trade is made the same way on every sheet and by a tool, instead of by whichever rule the drawer remembered.

**Example.** Grounded switch pins D2..D4 on the right of a part whose GND pin 7 is at the bottom left:

```
bad: around the part to the GND pin      good: straight down, across VDD, a second symbol
  ┌──────┐                                 ┌──────┐
  │   D2 ├──┐                              │   D2 ├──┐
  │   D3 ├──┤                              │   D3 ├──┤
  │   D4 ├──┤                              │   D4 ├──┤
  │  VDD ├──│──── +3V3                     │  VDD ├──┼──── +3V3
  │ GND  │  │                              │ GND  │  │
  └──┬───┘  │                              └──┬───┘  ┴ GND
     ├──────┘                                 ┴ GND
     ┴ GND
cost: 12 grid + 2 bends + detour 3 = 19     cost: 4 grid + 1 bend + 1 crossing + 1 symbol = 16
```

**Source.** house (Roman, 31 Oct 2026, the ties of the io_protection TMUX1511); the weights are new. Check:
`DRAW-40`.

### No stray wire pieces

No zero-length or overlapping wires. Why: they make junctions nobody drew. `DRAW-6`

## Labels and hierarchy

### Wire within a section, label between sections

Wires inside a section, labels for nets that would cross the sheet. Why: short wires show topology, labels keep long
nets from crossing everything.

### The narrowest scope that works

Local labels on one sheet, hierarchical labels and sheet pins between sheets, global labels only for nets that are
truly everywhere. Why: the scope says where to look for the other end. `WIRE-6`, `WIRE-12`, `WIRE-4`

### Labels lie on their wire, justified by side

**Rule.** A label sits on a horizontal wire stub, its text lying along that stub, never off its end in the air. On
the left side of a block the label is at the left end of its stub and **left-justified**, the text running right
along the wire toward the block; on the right side it is at the right end and **right-justified**, the text running
left toward the block. The stub is at least as long as the text plus one grid step, so the text never reaches the
pin. A label wanted on a pin or on a joint gets its own stub, joined with a junction dot.

**Why.** Text over its wire says which wire it names; text sticking out past the wire end floats next to nothing and
can be read as naming the wire below. Left side left-justified and right side right-justified line the label ends
up with the sheet's outer edge of each block, so the columns of labels read like a pin list.

**KiCad 10.** The side a local label's text runs to comes from its justification, not from its angle: a label at
0° with `justify right` runs left. Set the justification (label properties) rather than rotating the label, and
check it after copying labels from an older sheet. In the file: left side `(at x y 0)` with `(justify left bottom)`,
right side `(at x y 180)` with `(justify right bottom)`.

```
bad:          SEL ──────┤SEL1        good:  SEL──────────┤SEL1     V3V3───*
           (text left of the wire end,       (text along the stub)  ├VDD   (pin)
            hanging in the air)
```

**Source.** house (Roman, 31 Oct 2026); standard: Hack Club (labels sit on a wire). Checks: `DRAW-28`, `DRAW-11`
(vertical labels), `WIRE-3` (dangling labels).

### Sheet pins: inputs left, outputs right

Inputs on the left edge of the sheet symbol, outputs on the right, power at the top. Why: the parent sheet reads as
signal flow. `DRAW-13`

### Buses for groups of signals that go to many places

A group of related nets that each reach three or more places (an IO expander's port, an I2C or SPI bus shared by
several parts, a parallel data bus, DDR) is drawn as a KiCad bus: a bus wire with entries, a vector name `D[0..7]`
or a group name with an alias `{SDA SCL}`, every entry labelled. Signals that do not travel together get no bus.
Why: one bus line says "these go together to all of these places" where a dozen scattered labels make the reader
collect the group by name, and a bus of unrelated signals hides them. Source: house (Roman, 31 Oct 2026); KiCad
bus aliases. `DRAW-39` (a hint), `WIRE-16`

## Text

### Reference and value horizontal, next to the part

Shown on every part, horizontal, beside or above the body; `O` (Autoplace Fields) after rotating. A field's angle is
relative to its symbol: horizontal text on a part rotated by 90° has a field angle of 90°. Why: vertical text is read
with a tilted head and collides with wires first. `DRAW-8`, `DRAW-22`

### Text sizes

| Text | Size | Weight |
|---|---|---|
| fields, labels, pin names, note and parameter box bodies, table cells | 1.27 mm | normal |
| note box headings | 1.27 mm | bold |
| section headings | 2.54 mm | bold |
| group headings inside a section (a block of parameters, a sub-circuit) | 1.905 mm | bold |

Nothing below 1.27 mm. Why: 1.27 mm is the smallest that survives an A3 printed on A4. `DRAW-24`

### Sentences start with a capital letter

Prose in text boxes, notes, headings and table cells starts each sentence with a capital letter; a line that starts
with a net or part name, a unit, a value or code (`.param`, `mA`, `i(V1290)`, `nX`) stays as it is. Why: a note
reads as a sentence, not as a fragment cut from somewhere. Source: house (Roman, 31 Oct 2026). `DRAW-43`

### No text over text, a body or a wire

Why: overlapping text is unreadable, the most common reason a sheet looks messy. `DRAW-9`

## Parts

### Unused pins get a no-connect flag

Only after the datasheet allows it. Why: the flag records that the pin is open on purpose. `WIRE-8`

### DNP is an attribute

Set the DNP attribute and keep the real value. Why: a value of "DNP" is still bought and placed. `DRAW-23`, `BOM-13`

### Multi-unit parts: units near their circuit, power unit placed

Why: an unplaced unit leaves inputs floating and hides the rail. `LIB-14`, `LIB-13`

### Simulation-only parts are drawn apart and marked

Ammeters (0 V sources), fault sources, switches and behavioural models that exist only for the simulator use their
own symbols (an ammeter is `vhrd_sim:AMMETER`, a teal circle with an `A`, not a battery), sit together at the edge
of their section or in a Debug box's area, and are excluded from the BOM (`in_bom no`). They stay on the board
(`on_board yes`): the XML netlist drops off-board parts, and a simulation driver that maps references through it
(spicejob) then loses them. Why: a reader must see at a glance which parts are the circuit and which are the test
bench around it.

### Simulation directives grouped, each explained beside it

SPICE directives (`.param`, `.tran`, `.options`) are text boxes with *exclude from simulation* off, one box per
group of meaning (fault source, target, rails, controls, timing) under a group heading, with a second box flush to
its right that explains each line on the same line. KiCad reads directives from text and text boxes, not from table
cells, so the explanation is a box beside it rather than a table column. Why: forty `.param` lines in one text
block hide which parameters belong together, and an explanation in a separate block is matched line by line by
eye. Source: house (Roman, 31 Oct 2026, the io_protection index sheet).

## Colour, print and export

### Colour adds, never carries meaning alone

Keep the theme's colours for wires, labels and text; hand-set colour only where a name or a heading already says the
same (the note box kinds). Why: black and white prints, other themes and colour-blind readers lose colour. `DRAW-27`

### Neighbours in different colours

Every pair of items that touch is told apart at a glance: a wire and the pin it ends on, a wire and its power or
ground symbol, a wire and its label. The two colours differ by at least `drawing.colour.pair_min` (CIEDE2000,
default 20) on both the light and the dark theme, and each keeps 3:1 on its background where the colour is the
same for both themes (net classes and library symbols are). Our set:

| Item | Where it is set | Light (`vhrd.json`) | Dark (`vhrd_dark.json`) |
|---|---|---|---|
| wire | theme `wire` | `rgb(74, 74, 74)` | `rgb(168, 168, 168)` |
| pin | theme `pin` | `rgb(180, 20, 30)` (was `rgb(46, 46, 46)`, ΔE 9 to the wire) | `rgb(240, 100, 92)` (was `rgb(208, 208, 208)`, ΔE 10.5) |
| ground wire | net class Ground | `rgb(0, 115, 30)` | same |
| ground symbol | `vhrd_power:GND` | `rgb(60, 170, 24)` (ΔE 20.5 to its wire) | same |
| power wire | net class Power | `rgb(175, 50, 10)` | same |
| power symbol | `vhrd_power` supplies, `PWR_LOCAL` | `rgb(232, 113, 10)` (ΔE 22.9 to its wire) | same |

The pin takes the red family of the symbol outline, as in KiCad's own default theme; the symbol of a rail is the
lighter member of its family and its wires the darker one, so the long runs stay calm and the symbols stand out. Why: where a pin and its wire are the
same grey, the reader cannot see where the part ends, and a green ground symbol on a green wire melts into it.
Source: house (Roman, 31 Oct 2026). `DRAW-42`

### Check the PDF before a review

Plot to PDF with the drawing sheet and read it at 100 % and printed in black and white. Why: the PDF is what
reviewers and assemblers see.

## KiCad settings and shortcuts

Command names and default hotkeys of KiCad 10.0 (from its source); the same in 9.0 except where marked.

| Want | KiCad 10 |
|---|---|
| Snap a group to the grid | select, right-click > **Align Items to Grid** (`eeschema.AlignToGrid`, no default hotkey: assign one in Preferences > Hotkeys) |
| Line up labels or parts | select several, right-click > **Align** > Align to Left / Right / Top / Bottom / Horizontal Center / Vertical Center (new in 10, no hotkeys, no distribute) |
| Fields back in place, horizontal | **Autoplace Fields** `O`; Preferences > Editing Options: "Automatically place symbol fields", "Always align autoplaced fields to the 50 mil grid" (both on by default) |
| Label side | label properties > justification (left side: left, right side: right); the angle no longer sets the side |
| Grid | 50 mil (1.27 mm) default; `N` / `Shift+N` next / previous grid, `Alt+1` / `Alt+2` fast grids; grid overrides (`Ctrl+Shift+G`): connected items and wires 50 mil, text 10 mil |
| Rotate, mirror | `R` (counter-clockwise), `Shift+R` (clockwise), `X` mirror left-right, `Y` mirror top-bottom |
| Move or drag | `M` move (breaks connections), `G` drag (keeps them) |
| Wires straight | line mode 90° (`Shift+Space` cycles free / 90° / 45°) |
| Labels in a column | `L` label, then `Insert` repeats the last item one step down with its number incremented (`SDA0`, `SDA1`, ...) |
| No-connect, junction | `Q`, `J` |
| Note box, table | Place > Text Box; Place > Table (KiCad 8 and later) |
| Group a heading with its box | select both, right-click > Grouping > Group (KiCad 10 schematic groups) |
| Edit fields | `E` properties, `U` reference, `V` value, `F` footprint |
| Follow a net | `` ` `` highlight net |
| DNP | right-click > Attributes > Do not Populate; variants in Schematic Setup (10) |
| Formatting | Schematic Setup > Formatting: text size 50 mil, junction dot size, label offsets, **hop-over size** (10, default None) |
| Title block | File > Page Settings (or double-click the frame), "Export to other sheets"; text variables `${REVISION}`, `${#}`, `${##}`, `${COMMENT1}` |
| 4-way junction check | Schematic Setup > Electrical Rules > Violation Severity: "Four connection points are joined together" (`four_way_junction`), off by default: set it to warning |
| Print, export | File > Plot: PDF or SVG, colour or black and white, with or without the drawing sheet |

## What other tools automate

- **Altium Designer**: Align Left / Right / Top / Bottom (`Shift+Ctrl+L`, `R`, `T`, `B`), equal spacing
  (`Shift+Ctrl+H`), Align To Grid (`Shift+Ctrl+D`), and an "Off-Grid Object" compile check. [altium.com
  shortcuts](https://altium.com/documentation/node/304627/printable/print), [Off-Grid
  Object](https://www.altium.com/documentation/altium-designer/workspacemanager-err-offgridobjectoff-grid-object-ad)
- **OrCAD X Capture**: an Align toolbar with Distribute Horizontally / Vertically.
  [cadence.com](https://resources.pcb.cadence.com/orcad-x-capture-user-guide/20-managing-toolbars)
- **KiCad**: ERC catches off-grid ends, dangling labels, labels on several wires and 4-way junctions (when enabled);
  nothing checks orientation, spacing, fill or text overlaps.
- **Third-party KiCad tools** (KiBot, kicad-happy, kicad-tools, kicad-skip) run ERC or electrical reviews, or edit
  schematics from Python; none checks drawing style. That is what the `DRAW` checks are for.

## House style measured

What our own sheets do today, measured on 30 Oct 2026 over a private set of 157 production sheets (133 of them with
parts; KiCad 7 to 10 files) by reading the files themselves (positions, angles, fields, title blocks), not the
netlist. Numbers are over the sheets with parts unless the line says otherwise. The rules Roman set on 31 Oct 2026
(power net names, label justification, one symbol per rail per part, sections, note boxes, tables, clearance, small
parts vertical) are not measured yet.

| Habit | Measured | Verdict |
|---|---|---|
| Ground symbols down | 1837/1845 symbols, all down on 113/121 sheets | house style, keep |
| Supply symbols up | 535/535 | house style, keep |
| On the 1.27 mm grid | 121/133 sheets clean; 8 symbols, 66 wire ends, 32 labels off | house style; fix the rest |
| Diagonal or zero-length wires | 0 | house style |
| 4-way junctions | 17 on 16 sheets | rare; remove |
| Stacked labels aligned | 2289/3124 (73 %) | mostly; one command fixes the rest |
| Vertical labels | 200/3642 (5 %) | rare |
| Label scopes | 2339 local, 822 hierarchical, 481 global | hierarchy is the habit |
| Wires and labels per part | 4.3 wire segments, 1.0 label | balanced |
| Vertical reference / value | 298/3695 and 285/3268 (8 %) | mostly horizontal; `O` after rotating |
| Hidden references | 2 | house style |
| Text size | 10595 of 10605 texts at 1.27 mm | house style |
| Paper | A3 99, A4 54, A2 3, A0 1 of 157 | A3 is the house size |
| Fill of the usable area | median 46 %, 39 sheets under 30 % | room to use a smaller paper or centre |
| Centred within 10 % | 69/133 | half; new rule |
| Title, revision, date | 127, 108, 113 of 157 | mostly; revision missing on a third |
| Notes on the sheet | 102/133 any, 77/133 a calculation or reason | house style |
| Connectors at the edge | 158/417 in the outer fifth of the drawing | not yet a habit; new rule |
| Protection near the connector | 19/28 recognised TVS/ESD parts within 50 mm | mostly |
| DNP | 109 by attribute, 26 by text only | attribute is the habit; fix the text ones |
| Hand-set colours | 75/133 sheets | common; new rule says B&W must still read |
| Density | median 1.9 parts per dm², max 9.3 | fine |

Measuring this again is planned as a `berc` command, `TOOL-7`. The `DRAW` checks will report the same habits per sheet
once `NET-26` reads the drawing geometry.

## Sources

Cited by the rules above that are not obvious; the obvious ones are common to all of them.

- **KiCad** · KiCad, *Schematic Editor* manual, 10.0: grid ("Snapping"), aligning, formatting, hop-overs, text
  variables, plotting.
  [docs.kicad.org/10.0/en/eeschema/eeschema.html](https://docs.kicad.org/10.0/en/eeschema/eeschema.html)
- **KiCad source** · KiCad 10.0 source, `eeschema/tools/sch_actions.cpp` (command names and default hotkeys),
  `eeschema/tools/sch_align_tool.cpp`, `common/settings/app_settings.cpp` (grids), `eeschema/default_values.h`
  (formatting defaults), `eeschema/erc/erc_settings.cpp` (ERC defaults).
  [gitlab.com/kicad/code/kicad](https://gitlab.com/kicad/code/kicad/-/tree/10.0/eeschema)
- **KiCad 10 release** · *KiCad 10.0.0 Released* (hop-overs, variants, local power symbols, groups).
  [kicad.org/blog/2026/03/Version-10.0.0-Released](https://www.kicad.org/blog/2026/03/Version-10.0.0-Released/)
- **KLC** · KiCad Library Conventions, symbol rules S3 (general), S4 (pins), S6 (fields), S7 (power symbols).
  [klc.kicad.org](https://klc.kicad.org/), source
  [gitlab.com/kicad/libraries/klc](https://gitlab.com/kicad/libraries/klc)
- **IEC 61082-1** · *Preparation of documents used in electrotechnology, Part 1: Rules*, ed. 3 (2014): flow of energy
  and signal (7.1.1), connecting lines, joining and crossing (7.1.3), spacing of lines, reference designations at the
  symbol (7.1.6), title block (annex B). Paywalled; the section list is from the publisher's preview.
  [webstore.iec.ch/publication/10551](https://webstore.iec.ch/publication/10551)
- **IEEE 315** · IEEE 315-1975 / ANSI Y32.2-1975, *Graphic Symbols for Electrical and Electronics Diagrams*; IEC
  60617 is the international symbol set. [standards.ieee.org/ieee/315/6396](https://standards.ieee.org/ieee/315/6396/)
- **Wikipedia** · *Circuit diagram*: left to right and top to bottom, positive supplies at the top, grounds and
  negative supplies at the bottom, 4-way connections staggered into two T junctions.
  [en.wikipedia.org/wiki/Circuit_diagram](https://en.wikipedia.org/wiki/Circuit_diagram)
- **AoE** · Horowitz and Hill, *The Art of Electronics*, rules for good schematics (as kept by the xcircuit project).
  [xcircuit.sourceforge.net/goodschem/goodschem.html](https://xcircuit.sourceforge.net/goodschem/goodschem.html)
- **EMA** · EMA Design Automation, *Schematic Guidelines Checklist* (2021). [ema-eda.com
  (PDF)](https://ema-eda.com/wp-content/uploads/2025/04/2021SchematicGuidelines_Checklist.pdf)
- **Altium** · Altium, *Easy Schematics Creation for Elegance and Readability*.
  [resources.altium.com](https://resources.altium.com/p/creating-elegant-and-readable-schematics)
- **Hack Club** · Hack Club Hardware Codex, *Schematic practices*.
  [hwdocs.hackclub.dev](https://hwdocs.hackclub.dev/user-contrib-guides/schematic-practices/)
- **KiCad forum** · *Best practices: pin-on-pin connection, junction dots*.
  [forum.kicad.info/t/13882](https://forum.kicad.info/t/best-practices-pin-on-pin-connection-junctions-dots/13882)
