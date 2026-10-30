# Schematic drawing guide

How to draw a schematic sheet that reads at a glance: where the signal enters and leaves, which way power flows, what
every part is and why it was chosen. better_erc checks what a schematic does; this page is about how it looks, because
a reviewer who cannot read a sheet quickly misses what is wrong on it. The rules that better_erc can check from the
drawing are in the [check catalogue](checks.md#draw-1) as `DRAW` checks, explained on
[docs/checks/draw.md](checks/draw.md); connectivity hygiene (labels, flags, buses) is `WIRE`, on
[docs/checks/wire.md](checks/wire.md).

Every rule has the same shape: the rule in one line, why it matters, a small bad/good sketch, and where it comes from.
The tag says how firm it is:

- **standard**: a published standard, convention or tool default says so (the source is named).
- **house**: what our own sheets already do (`N/M` sheets), measured over a private set of 157 production sheets; see
  [House style measured](#house-style-measured).
- **new**: our recommendation where neither of the above settles it, with the reason.

The sketches use `>` and `<` for label and pin direction, `*` for a junction, `┴` for a ground symbol and `┬` for a
supply symbol.
KiCad names follow KiCad 10.0; most of it holds for KiCad 8 and 9.

## Contents

- [Sources](#sources)
- [The sheet](#the-sheet): size and splitting, one function per block, fill and centring, title block, sheet names,
  notes
- [Grid and alignment](#grid-and-alignment): 1.27 mm grid, aligning, spacing
- [Signal flow](#signal-flow): left to right, connectors at the edge, symbol orientation
- [Power and ground](#power-and-ground): ground down, supplies up, decoupling next to its pin
- [Wires and junctions](#wires-and-junctions): straight wires, no 4-way junctions, crossings, no wire through a part
- [Labels and hierarchy](#labels-and-hierarchy): wire or label, local or global or hierarchical, alignment, sheet
  pins, buses
- [Text](#text): reference and value, text size, overlaps, design-intent notes
- [Parts](#parts): unused pins, DNP, multi-unit parts
- [Colour, print and export](#colour-print-and-export)
- [KiCad settings and shortcuts](#kicad-settings-and-shortcuts)
- [What other tools automate](#what-other-tools-automate)
- [House style measured](#house-style-measured)

## Sources

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
- **IEEE 315** · IEEE 315-1975 / ANSI Y32.2-1975, *Graphic Symbols for Electrical and Electronics Diagrams*, with
  class letters for reference designations; IEC 60617 is the international symbol set.
  [standards.ieee.org/ieee/315/6396](https://standards.ieee.org/ieee/315/6396/)
- **ASME Y14.44** · *Reference Designations for Electrical and Electronics Parts and Equipment* (2008); IPC-2612
  covers schematic documentation. Paywalled, cited for the designator rules only.
- **Wikipedia** · *Circuit diagram*: left to right and top to bottom, positive supplies at the top, grounds and
  negative supplies at the bottom, 4-way connections staggered into two T junctions.
  [en.wikipedia.org/wiki/Circuit_diagram](https://en.wikipedia.org/wiki/Circuit_diagram)
- **AoE** · Horowitz and Hill, *The Art of Electronics*, rules for good schematics (as kept by the xcircuit project):
  dot means connect, never four wires at a point, wires horizontal or vertical, signals left to right, positive supply
  at the top.
  [xcircuit.sourceforge.net/goodschem/goodschem.html](https://xcircuit.sourceforge.net/goodschem/goodschem.html)
- **EMA** · EMA Design Automation, *Schematic Guidelines Checklist* (2021): left input to right output, sheets by
  function in a hierarchy, decoupling grouped, notes for currents, impedance and pull-up purpose. [ema-eda.com
  (PDF)](https://ema-eda.com/wp-content/uploads/2025/04/2021SchematicGuidelines_Checklist.pdf)
- **Altium** · Altium, *Easy Schematics Creation for Elegance and Readability*: signals left to right, voltage falling
  left to right through the regulation stages, hierarchy that mirrors the block diagram.
  [resources.altium.com](https://resources.altium.com/p/creating-elegant-and-readable-schematics)
- **Hack Club** · Hack Club Hardware Codex, *Schematic practices*: ground symbols down and power symbols up, data left
  to right, labels on a wire, decoupling next to what it decouples, pull-ups above the line.
  [hwdocs.hackclub.dev](https://hwdocs.hackclub.dev/user-contrib-guides/schematic-practices/)
- **KiCad forum** · *Best practices: pin-on-pin connection, junction dots*: no pin-on-pin connections, keep a short
  wire.
  [forum.kicad.info/t/13882](https://forum.kicad.info/t/best-practices-pin-on-pin-connection-junctions-dots/13882)

## The sheet

### Paper size: A4 or A3, the same for every sheet of a project

**Rule.** Draw on A4 or A3 landscape, one size for the whole project; when a function does not fit an A3, split it
into sheets instead of taking A2.

**Why.** Sheets are read on a laptop screen, printed for a review and exported to PDF. A3 prints readably on A4 (1.27
mm text becomes 0.9 mm, still legible); an A2 or A1 sheet printed on A4 becomes 0.6 mm text, and on screen it needs
constant zooming. One size keeps zoom and print settings the same from page to page.

**Example.**

```
bad:  p1 A4, p2 A2 (the whole MCU with 200 pins), p3 A0 (everything else)
good: p1 A3 index, p2 A3 MCU power and clocks, p3 A3 MCU IO, p4 A3 Ethernet
```

**Source.** new (our print and screen habit); house: A3 99/157, A4 54/157, A2 3, A0 1 (a full-board overview). Check:
`DRAW-21`.

### One function per sheet, one block per function

**Rule.** Give each sheet one function (power input, a regulator group, the MCU, one interface), and within a sheet
one block per sub-function with whitespace between blocks; split a sheet before it gets crowded, not when it is full.

**Why.** A reviewer goes through a board function by function; a sheet that holds one function can be checked against
its datasheet in one pass. A dense sheet (over about 4 parts per dm² of paper) forces long wires and labels between
unrelated parts. Hierarchy then mirrors the block diagram.

**Example.**

```
bad:  one A3 "Power" with input protection, 3 bucks, 2 LDOs, sequencing and the fan driver packed edge to edge
good: "Power input" (connector, fuse, TVS, reverse protection), "Rails" (3 bucks in three framed blocks),
      "LDOs and sequencing", fan driver on the "Cooling" sheet
```

**Source.** standard: EMA (sheets by function, hierarchy), Altium (hierarchy mirrors the block diagram); house: median
1.9 parts per dm², densest sheet 9.3. Check: `DRAW-18` (too dense).

### The drawing is centred and fills the sheet

**Rule.** Centre the drawing on the sheet with even margins, and size the paper so the drawing fills roughly half of
the frame or more; nothing touches the frame or the title block.

**Why.** A circuit squeezed into one corner of an A3 prints small and leaves the reader hunting; a drawing that runs
into the frame or under the title block loses parts on print. Centring is cheap once a block is done: select all,
move.

**Example.**

```
bad:                                   good:
+-----------------------------+        +-----------------------------+
| [circuit]                   |        |                             |
|                             |        |      [    circuit    ]      |
|                             |        |                             |
|                   [title]   |        |                   [title]   |
+-----------------------------+        +-----------------------------+
```

**Source.** new (our preference; IEC 61082-1 sets out the layout of a document but its wording is not checked here);
house: median fill 46 % of the usable area, 39/133 sheets under 30 %; centre within 10 % of the sheet's width and
height on 69/133. Checks: `DRAW-16` (fill, frame), `DRAW-17` (centring).

### Title block: title, revision, date, on every sheet

**Rule.** Fill the title block on every sheet: title, revision, date, company; put the revision once in the project
(KiCad: Page Settings, "Export to other sheets") and use text variables (`${REVISION}`, `${ISSUE_DATE}`, `${#}` page
number, `${##}` page count) instead of retyping.

**Why.** A printout or a PDF page gets separated from the rest; without a title and revision on the page nobody knows
which board and which iteration it shows. Retyped revisions drift: page 7 still says rev A when the board is rev B.

**Example.**

```
bad:  Title "", Rev "", Date "" on p4; Rev "B" on p1 and "A" on p5
good: Title "USB-C serial adapter - Power input", Rev "${REVISION}" = B, Date 2026-10-12, Page 4/9
```

**Source.** standard: IEC 61082-1 annex B (title block), KiCad (Page Settings, text variables); house: title 127/157,
revision 108/157, date 113/157, company 105/157. Checks: `DRAW-19` (missing fields, unresolved variable), `DRAW-20`
(revisions differ).

### Sheet names say the function

**Rule.** Name each hierarchical sheet after its function (`USB-C input`, `CAN 1`), and the file after the sheet
(`usb_c_input.kicad_sch`); start with an index or block-diagram sheet when the project has more than three or four
sheets.

**Why.** The sheet name is the hierarchy navigator's entry, the PDF bookmark and the page title; `Sheet1` and
`Untitled` tell nothing. An index sheet with the sheet symbols laid out as the block diagram is the map of the board.

**Example.**

```
bad:  Sheet1 (file1.kicad_sch), Sheet2 (new.kicad_sch)
good: 1 Index / 2 Power input / 3 Rails / 4 MCU / 5 CAN 1 / 6 CAN 2
```

**Source.** standard: Altium (hierarchy mirrors the block diagram); new (names). Check: `DRAW-25`.

### Notes give the design intent: why this part, how this value

**Rule.** Put the reason and the calculation next to the parts they explain: the feedback divider formula with the
result, the current-set resistor, the filter corner, why this TVS and not a smaller one, "do not change to X5R".

**Why.** The schematic outlives the designer's memory. A value without its calculation gets "fixed" in the next
revision, a deliberate odd part gets replaced by the standard one. Reviewers check a calculation in seconds when it is
on the sheet, and not at all when it is not.

**Example.**

```
bad:  R12 261k, R13 1M, nothing else
good: R12 261k, R13 1M
      note: "Vout = 0.8 V × (1 + 261k/1M) = 1.01 V"
```

**Source.** standard: EMA (notes for currents, impedance, pull-up purpose); house: 102/133 sheets carry text notes,
77/133 one with a calculation or a reason. Check: `DRAW-26` (candidates only: a
calculated part with no note on its sheet).

## Grid and alignment

### Everything that connects sits on the 1.27 mm grid

**Rule.** Keep the schematic grid at 1.27 mm (50 mil) for symbols, wires, labels and sheet pins; use a finer grid only
for text and graphics (KiCad's grid overrides do exactly that).

**Why.** KiCad connects items only where their points coincide exactly. A pin or label half a grid step off looks
connected and is not, and from then on every wire drawn to it is off too. Library symbols have their pins on a 2.54 mm
(100 mil) grid, so a 1.27 mm sheet grid always meets them.

**Example.**

```
bad:   ──────┐ 0.635 mm gap        good:  ──────>SDA
             >SDA                                 (label anchor on the wire end)
```

**Source.** standard: KiCad ("recommended to always use a 50 mil grid"; ERC `endpoint_off_grid`), KLC S4.1 (pins on
100 mil); house: clean on 121/133 sheets, 106 off-grid items in total. Checks: `DRAW-3`, `WIRE-3`.

### Align with one command, not by eye

**Rule.** After placing or moving a group, select it and run **Align Items to Grid** (right-click on the selection;
KiCad 9 and 10); in KiCad 10 line up labels and parts with **Align** (right-click with several items selected: Align
to Left, Right, Top, Bottom, Horizontal Center, Vertical Center).

**Why.** Labels a grid step apart, parts a little off a row and fields at odd places are what makes a sheet look
untidy, and all of it is one command away. Neither command has a default hotkey: assign one in Preferences > Hotkeys
(`eeschema.AlignToGrid`, `eeschema.Align.alignLeft`, ...) and the habit costs nothing.

**Example.**

```
bad:     >SCL            good:  >SCL
       >SDA                     >SDA
          >INT_N                >INT_N
```

**Source.** standard: KiCad source (`eeschema.AlignToGrid` "Align Items to Grid", KiCad 9 and 10; `eeschema.Align.*`,
KiCad 10; no distribute command); house: 2289 of 3124 stacked labels (73 %) share their column with a neighbour.
Check: `DRAW-10`.

### Leave room: whitespace between blocks, space around parts

**Rule.** Leave at least 5 mm (4 grid steps) around every symbol and its fields and 15-25 mm between blocks; never let
two symbols touch pin to pin.

**Why.** Whitespace is what groups parts into blocks for the eye; without it a sheet reads as one block. A pin-to-pin
connection has no wire to add a branch to later and is invisible when the parts are moved apart; the fields of
touching parts overlap.

**Example.**

```
bad:   ─[R1]─[C1]─          good:   ─[R1]──*──[C1]─
        pin on pin                         │
```

**Source.** standard: AoE (space around symbols), Hack Club and KiCad forum (no pin-on-pin, keep a short wire); new
(the distances). Check: `DRAW-9` (overlaps it leaves).

## Signal flow

### Signals run left to right, inputs on the left, outputs on the right

**Rule.** Lay each sheet and each block out so the main signal enters on the left and leaves on the right, and
feedback runs back above or below the forward path; where the flow is not obvious, an arrow in a label shape (input,
output) says it.

**Why.** Western readers scan left to right; a sheet that follows that order reads like a sentence. When a reviewer
knows where to look for the input, a missing part in the path stands out.

**Example.**

```
bad:   MCU ◄──[R]──◄ J1          good:  J1 >──[TVS]──[R]──> MCU
       (connector right, MCU left, arrows against the reading direction)
```

**Source.** standard: IEC 61082-1 7.1.1, Wikipedia, AoE, EMA, Altium, Hack Club; house: no measure yet (DRAW-12 will
count it). Check: `DRAW-12` (symbols placed against the flow).

### Power flows left to right and from high to low voltage

**Rule.** On power sheets draw the input on the left and the rails it produces to the right in falling voltage (24 V
input, then 5 V, then 3.3 V, then 1.8 V); current returns at the bottom.

**Why.** It reads as the power tree: what feeds what is visible without following labels, and a regulator fed from the
wrong rail looks out of place.

**Example.**

```
bad:   [LDO 1V8]  [J1 24V]  [buck 3V3]  [buck 5V]
good:  [J1 24V] → [buck 5V] → [buck 3V3] → [LDO 1V8]
```

**Source.** standard: Altium (voltage falls left to right through the regulation stages, currents top to bottom).

### Connectors at the sheet edge, with their protection beside them

**Rule.** Put each connector at the edge of its sheet, on the side its signals come from (inputs left, outputs right),
and its protection (TVS, ESD diode, fuse, common-mode choke, series resistor) right next to it, between the connector
and the rest of the circuit.

**Why.** The board edge is where the outside world comes in, and the protection only works when it is the first thing
a surge meets. When the schematic shows the same order, a reviewer sees at once whether every line of a connector
passes its protection first; a TVS drawn next to the MCU invites a layout next to the MCU too.

**Example.**

```
bad:   ... MCU ──*── J3 (middle of the sheet)      good:  J3 >──*──[R 1k]── ... MCU
                 │                                          │
                TVS (on another sheet)                     TVS
                                                            ┴
```

**Source.** new (our preference; the checks `CONN-1`, `CONN-2` need the protection, this is where to draw it); house:
158/417 connectors (38 %) sit in the outer fifth of their sheet's drawing, and 19/28 recognised protection parts are
within 50 mm of a connector. Check: `DRAW-14`.

### Symbol orientation: never mirror an IC, keep text upright

**Rule.** Place ICs and multi-pin parts unrotated and unmirrored, as the library drew them; rotate two-pin parts
freely (90° steps); mirror only symmetric parts (a resistor, a switch) and transistors to point the right way.

**Why.** Library symbols put inputs left, outputs right, power up and ground down (KLC S4.2); a mirrored IC turns that
around and its pin numbers read in an odd order. KiCad never mirrors text, but a rotated IC turns all its pin names
vertical.

**Example.**

```
bad:  U3 mirrored: OUT on the left, IN on the right, VCC on top still, the flow runs backwards
good: U3 as drawn in the library: IN left, OUT right
```

**Source.** standard: KLC S4.2 (pin groups: positive power top, ground bottom, inputs left, outputs right); house:
393/3697 parts mirrored, 904 rotated. Check: `DRAW-12`.

## Power and ground

### Ground symbols point down, supply symbols point up

**Rule.** Draw ground symbols below the pin, pointing down, and positive supply symbols above the pin, pointing up;
negative supplies point down like grounds.

**Why.** The vertical axis then reads as voltage: a reader sees at once which pins go to a rail and which to ground,
and a ground symbol on its side or a supply hanging down reads as a signal. It also keeps power off the left-to-right
signal path.

**Example.**

```
bad:     ┬ GND (turned upside down)      good:     ┬ +3V3 (points up)
         │                                         │
       [C1]                                      [C1]
         │                                         │
         ┴ +3V3 (hanging down)                     ┴ GND (points down)
```

**Source.** standard: Wikipedia, AoE, Hack Club ("Always point ground symbols down, and power symbols up!"), KLC S4.2
(the same in the symbol); house: 1837/1845 ground symbols down (113/121 sheets all down), 535/535 supply symbols up.
Checks: `DRAW-1`, `DRAW-2`.

### Power symbols, not wires, to reach a rail

**Rule.** Connect rails with power symbols at each pin (or a short wire to one), not with long wires across the sheet;
one ground symbol per pin or per small group is fine.

**Why.** A rail wired across a sheet crosses every signal; power symbols keep the signal paths clean and make each
part's supply readable where the part is. A ground symbol per pin is not clutter, it is a statement per pin.

**Example.**

```
bad:   a +3V3 wire from the regulator snakes 30 cm round the sheet to 8 decoupling caps
good:  each cap has its own ┬ +3V3 above and ┴ GND below
```

**Source.** standard: KiCad (power symbols are global labels); house: 1845 ground and 535 supply symbols on 133
sheets, about 18 per sheet. Check: none (`WIRE-5` and `WIRE-14` check their names).

### Decoupling capacitors next to the pin they serve

**Rule.** Draw each decoupling capacitor next to the power pin it decouples (or in a row beside the IC, in the order
of its power pins), with its value and voltage; a note says where it goes in the layout when that matters.

**Why.** The schematic is what the layout engineer places from; caps drawn in a pile in a corner end up in a pile on
the board. Next to the pin, a reviewer can count one cap per pin and check the value the datasheet asks for (`DEC-1`,
`DEC-7`).

**Example.**

```
bad:   C1..C12 100n in a grid in the bottom right, labels +3V3 / GND
good:     VDD1 ─┬─ C4 100n      VDD2 ─┬─ C5 100n      VDDA ─┬─ C6 1u ─ C7 100n
                ┴                      ┴                     ┴
```

**Source.** standard: Hack Club (decoupling next to what it decouples), EMA (decoupling grouped, critical placement in
a note). Checks: `DRAW-15`, `LAY-17` (on the board).

## Wires and junctions

### Wires are horizontal or vertical

**Rule.** Draw wires horizontal and vertical only (KiCad line mode 90°, Shift+Space cycles it).

**Why.** Diagonal wires cross more, are hard to follow and do not meet the grid at junctions. Orthogonal wires read as
a table.

**Example.**

```
bad:   ─╲          good:  ──┐
          ╲                 │
           ─                └──
```

**Source.** standard: AoE (wires horizontal or vertical), KiCad (line modes); house: 0 diagonal wires on 133 sheets.
Check: `DRAW-5`.

### No 4-way junctions: two T junctions instead

**Rule.** Never join four wires at one point; stagger them into two T junctions a few grid steps apart; every T gets a
junction dot.

**Why.** A crossing and a 4-way junction look the same when the dot is small, missing or lost in a print or a scan.
Two T junctions are unambiguous whatever the print quality. A wire ending on the middle of another wire without a dot
looks connected; in KiCad it only connects through the junction.

**Example.**

```
bad:      │             good:      │
       ───*───                  ───*
          │                        │
                                   *───
                                   │
```

**Source.** standard: AoE ("four wires must not connect at a point"), Wikipedia, KiCad (ERC `four_way_junction`, off
by default: turn it on); house: 17 4-way junctions on 16/133 sheets . Checks: `WIRE-15`, `DRAW-4` (T
without a junction).

### Crossings: a plain cross is no connection; hop-overs where it helps

**Rule.** Let wires cross only where it cannot be avoided; a plain cross never connects. In KiCad 10, set Schematic
Setup > Formatting > Hop-over size (Small) on sheets with many crossings so a crossing can never be read as a joint.

**Why.** Each crossing is a place the eye can jump to the wrong wire. Hop-overs (new in KiCad 10, off by default,
plotted too) make the "not connected" explicit.

**Example.**

```
bad:   ───┼───  next to ───*───  (only the dot differs)
good:  ───╮╭───  (hop-over)  or rearranged so they do not cross
```

**Source.** standard: KiCad 10 (hop-overs), Wikipedia (hop symbols for insulated crossings).

### Wires never run through a symbol, a label or text

**Rule.** Route wires around symbols, fields and labels, never over them.

**Why.** A wire across a symbol body looks like a connection to a pin under it, or hides one; a wire through a
reference makes both unreadable.

**Example.**

```
bad:   ───[R5]──────   wire passes over the body of U2 on its way to R5
good:  the wire goes round U2 with a clear gap
```

**Source.** new (common sense; no published rule found). Check: `DRAW-7`.

### No stray wire pieces

**Rule.** Remove zero-length wires and wires drawn on top of each other (they are left behind by drags).

**Why.** Overlapping wires create junctions nobody meant and make moving a part drag odd pieces along; zero-length
wires show as junction dots.

**Example.**

```
bad:   ───────  two wires on top of each other from x=10 to x=40 and x=20 to x=60
good:  one wire from x=10 to x=60
```

**Source.** new; house: 0 zero-length wires on 133 sheets. Check: `DRAW-6`.

## Labels and hierarchy

### Wire or label: wire within a block, label between blocks

**Rule.** Inside a block, connect with wires; between blocks on the same sheet, and for any net that would cross the
sheet, use a local net label at both ends; never draw a wire across the sheet to save a label.

**Why.** Short wires show topology at a glance; labels keep long nets from crossing everything. A sheet that is all
labels (every pin with a stub and a label) hides the circuit just as badly as one that is all wires: the reader has to
search the sheet for each name.

**Example.**

```
bad:   every resistor of a divider with its own label (FB_TOP, FB_MID, FB_BOT), no wire between them
good:  divider drawn with wires; only VOUT and FB leave the block as labels
```

**Source.** new (our balance); house: about 1 label per part, 4.3 wire segments per part. Checks: none (`WIRE-1` and
`WIRE-11` catch the extremes).

### Local, global, hierarchical: the narrowest scope that works

**Rule.** Use a local label for a net on one sheet, a hierarchical label and sheet pin for a net that leaves the
sheet, and a global label only for nets that are truly everywhere (a reset, an I2C bus used on many sheets); power
symbols only for rails.

**Why.** The scope says where to look for the other end. A global label hides which sheets a net reaches, and two
sheets that use the same global name by chance are silently joined. Hierarchical sheet pins make every connection
between sheets visible on the parent sheet.

**Example.**

```
bad:   global label SPI_MOSI on 2 of 9 sheets
good:  hierarchical label SPI_MOSI in the flash sheet, a sheet pin on its symbol, a wire to the MCU sheet's pin
```

**Source.** standard: KiCad (label scopes); house: 2339 local, 822 hierarchical, 481 global labels. Checks: `WIRE-6`
(global on one sheet), `WIRE-12` (one name, two scopes), `WIRE-4` (sheet pins).

### Labels sit on a wire end, horizontal, aligned in a column

**Rule.** Put a label on the end of a short horizontal wire stub (not on a pin, not across several wires), write it
horizontally, and line up labels that stack into one column.

**Why.** A label on a stub shows clearly which wire it names; a vertical label has to be read with a tilted head; a
column of aligned labels reads like a pin list.

**Example.**

```
bad:   │                      good:  ──>SCL
       S                             ──>SDA
       C                             ──>INT_N
       L
```

**Source.** standard: Hack Club (labels sit on a wire), KiCad (ERC `label_multiple_wires`); house: 200 of 3642 labels
vertical (5 %). Checks: `DRAW-10`, `DRAW-11`, `WIRE-3`, `WIRE-15`.

### Sheet pins: inputs on the left of the sheet symbol, outputs on the right

**Rule.** Place sheet pins by direction: inputs on the left edge of the sheet symbol, outputs on the right, power at
the top, bidirectional where the partner is; set their shape (input, output, bidirectional) to match.

**Why.** On the parent sheet, the sheet symbols then wire up left to right like parts, and the block diagram reads as
signal flow.

**Example.**

```
bad:   ┌ Flash ──────┐         good:  ┌ Flash ──────┐
  MISO<┤             ├<MOSI      MOSI>┤             ├>MISO
       └─────────────┘                └─────────────┘
```

**Source.** standard: KLC S4.2 (the same rule for symbol pins); new (for sheet symbols). Check: `DRAW-13`.

### Buses for real groups only

**Rule.** Use a bus for a group of signals that travel together (a parallel bus, an RGMII group); give it a bus alias
or a `{...}` group name, and label every entry.

**Why.** A bus drawn for two unrelated signals hides them; a bus whose members are not labelled connects nothing.

**Example.**

```
bad:   bus "IO" carrying LED1, BTN2 and UART_TX
good:  bus RGMII{TXD[0..3] TX_CTL TXC}, each entry labelled
```

**Source.** standard: KiCad (buses, bus aliases). Check: `WIRE-16`.

## Text

### Reference and value next to the part, horizontal

**Rule.** Show the reference and the value (plus voltage, tolerance or dielectric where it matters) next to every
part, horizontal, to the right of or above it; press `O` (Autoplace Fields) after rotating a part.

**Why.** A vertical reference is read with a tilted head and is the first thing to collide with a wire. KiCad keeps
fields upright but turns them with a rotated part unless they are placed again; autoplace puts them horizontal, beside
the body, on the grid.

**Example.**

```
bad:   ──[R7]──        good:  ──[    ]──  R7
         7                                10k
        R                                 1 %
```

**Source.** standard: KLC S6.2 (reference and value visible), KiCad (Autoplace Fields `O`; Preferences > Editing
Options "Automatically place symbol fields", on by default); house: 298/3695 references and 285/3268 values vertical
(8 %); 2 hidden references. Checks: `DRAW-8`, `DRAW-22`.

### Text at 1.27 mm, one size

**Rule.** Use KiCad's 1.27 mm (50 mil) for all text: fields, labels, pin names and notes; bigger only for a sheet
heading.

**Why.** 1.27 mm is the smallest size that survives an A3 sheet printed on A4; smaller text disappears in print and in
a low-resolution PDF view. One size makes the sheet calm.

**Example.**

```
bad:   pin names at 0.8 mm on a dense connector symbol, labels at 2 mm
good:  everything at 1.27 mm
```

**Source.** standard: KLC S3.2 (50 mil, pin names down to 20 mil only on small symbols), KiCad default text size 50
mil; house: 10595 of 10605 measured texts at 1.27 mm. Check: `DRAW-24`.

### No text over text, a body or a wire

**Rule.** No field, label or note may overlap another text, a symbol body, a pin name or a wire.

**Why.** Overlapping text is unreadable, and it is the most common reason a sheet "looks messy". In a PDF it is worse
than on screen, where at least the editor can highlight.

**Example.**

```
bad:   C6R6 / 10u330  (two parts' fields on top of each other)
good:  C6  10u      R6  330
```

**Source.** new (no rule found, everybody agrees); our own sheets: 56 overlapping field pairs on 16 sheets of one
study. Check: `DRAW-9`.

## Parts

### Unused pins get a no-connect flag, deliberately

**Rule.** Mark every pin that is left open with a no-connect flag (`Q`), and only after checking the datasheet allows
it; tie the ones the datasheet says to tie.

**Why.** The flag is a decision written down: "this pin is open on purpose". Without it, an open pin may be a
forgotten one, and ERC reports it every time.

**Example.**

```
bad:   PA3 ─  (nothing)            good:  PA3 ─x   (no-connect flag)
```

**Source.** standard: KiCad (no-connect flag, ERC `pin_not_connected`). Checks: `WIRE-8`, `WIRE-7`, `WIRE-9`.

### DNP is an attribute, and it shows

**Rule.** Mark parts that are not fitted with the DNP attribute (KiCad: Do not Populate, or a variant), never only by
writing "DNP" in the value; keep the real value so the part can be fitted later.

**Why.** KiCad crosses a DNP part out on the sheet and leaves it out of the BOM and placement files; a value that says
"DNP" does none of that, so the part is bought and placed with a value of "DNP". The real value is what rework needs.

**Example.**

```
bad:   R20 value "DNP"               good:  R20 10k, DNP attribute set (drawn crossed out)
```

**Source.** standard: KiCad (DNP attribute, variants in 10); house: 109 parts with the DNP attribute, 26 with DNP-like
text and no attribute. Checks: `DRAW-23`, `BOM-13`.

### Multi-unit parts: power unit apart, units near their circuit

**Rule.** Draw the power unit of a multi-unit part (op-amps, logic) with its decoupling on the sheet of its circuit,
and each unit where its circuit is; unused units placed and tied off.

**Why.** A unit drawn away from the power unit hides which rail the part runs on; an unplaced unit has floating
inputs.

**Example.**

```
bad:   U5A and U5B used, U5C (power) never placed: no supply on the sheet
good:  U5C with +5V, GND and 100n next to U5A
```

**Source.** standard: KLC S3.8 (separate power unit). Checks: `LIB-14`, `LIB-13`.

## Colour, print and export

### Colour adds, never carries meaning alone

**Rule.** Keep KiCad's default colours for wires, labels and text; if you colour (net classes, hand-set colours), the
sheet must still read in black and white.

**Why.** Sheets are printed in black and white, viewed in other colour themes and read by colour-blind reviewers; a
meaning that lives only in a colour is lost there. Net class colours are fine as a screen aid because the net class
itself is still in the data.

**Example.**

```
bad:   "red wires are 24 V" (nothing else says so)
good:  24 V nets labelled +24V and in net class POWER_24V; red is a bonus
```

**Source.** new (our preference); house: hand-set colours on 75/133 sheets. Check:
`DRAW-27`.

### Check the print and the PDF before a review

**Rule.** Before a review, plot the schematic to PDF (File > Plot, PDF, with the drawing sheet) and read it at 100 %
on screen and printed in black and white on A4.

**Why.** The PDF is what reviewers, assemblers and the next designer see. Small text, light colours and overlaps that
look fine at the editor's zoom show up there. KiCad's PDF adds bookmarks per sheet, hierarchical links and property
popups, so a reviewer can work in the PDF alone.

**Example.**

```
bad:   review from the editor only; the PDF has grey text on a white background after a theme change
good:  PDF, colour, drawing sheet on, checked at 100 %; SVG per sheet for the docs
```

**Source.** standard: KiCad (Plot: PDF and SVG, colour or black and white, drawing sheet, background colour, minimum
line width; hop-overs plotted). Checks: `DRAW-24`, `DRAW-27`.

## KiCad settings and shortcuts

Command names and default hotkeys of KiCad 10.0 (from its source); the same in 9.0 except where marked.

| Want | KiCad 10 |
|---|---|
| Snap a group to the grid | select, right-click > **Align Items to Grid** (`eeschema.AlignToGrid`, no default hotkey: assign one in Preferences > Hotkeys) |
| Line up labels or parts | select several, right-click > **Align** > Align to Left / Right / Top / Bottom / Horizontal Center / Vertical Center (new in 10, no hotkeys, no distribute) |
| Fields back in place, horizontal | **Autoplace Fields** `O`; Preferences > Editing Options: "Automatically place symbol fields", "Always align autoplaced fields to the 50 mil grid" (both on by default) |
| Grid | 50 mil (1.27 mm) default; `N` / `Shift+N` next / previous grid, `Alt+1` / `Alt+2` fast grids; grid overrides (`Ctrl+Shift+G`): connected items and wires 50 mil, text 10 mil |
| Rotate, mirror | `R` (counter-clockwise), `Shift+R` (clockwise), `X` mirror left-right, `Y` mirror top-bottom |
| Move or drag | `M` move (breaks connections), `G` drag (keeps them) |
| Wires straight | line mode 90° (`Shift+Space` cycles free / 90° / 45°) |
| Labels in a column | `L` label, then `Insert` repeats the last item one step down with its number incremented (`SDA0`, `SDA1`, ...) |
| No-connect, junction | `Q`, `J` |
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
  nothing checks orientation, fill or text overlaps.
- **Third-party KiCad tools** (KiBot, kicad-happy, kicad-tools, kicad-skip) run ERC or electrical reviews, or edit
  schematics from Python; none checks drawing style. That is what the `DRAW` checks are for.

## House style measured

What our own sheets do today, measured on 30 Oct 2026 over a private set of 157 production sheets (133 of them with
parts; KiCad 7 to 10 files) by reading the files themselves (positions, angles, fields, title blocks), not the
netlist. Numbers are over the sheets with parts unless the line says otherwise.

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