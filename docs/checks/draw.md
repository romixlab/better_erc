# DRAW checks, explained

Detail page for the `DRAW` category of the [check catalogue](../checks.md#draw-1). Each section explains one check:
what it catches, why it makes a sheet hard to read, a concrete example, what better_erc needs to find it, the usual
fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID, severity and
wording; this page is the commentary. The rules behind them, with good and bad sketches and how our own sheets measure
against them, are in the [schematic drawing guide](../drawing-guide.md).

These checks read the drawing, not the netlist: positions, angles, fields, text, the title block. The drawing facts
(`NET-21`) carry labels and flags today; the geometry they need (wires with their ends, field positions and angles,
text, paper size, title block, symbol body extents) is `NET-26`, planned. The **Impl.** line says how hard the check
is for better_erc in pure Rust: `mid` once `NET-26` is there, `hard` where symbol and text extents must be computed,
`judgement` where better_erc can only list candidates.

References used throughout:

- **KiCad** · KiCad, *Schematic Editor* manual, 10.0: "Snapping", "Aligning objects", "Junctions", text variables,
  plotting. [docs.kicad.org/10.0/en/eeschema/eeschema.html](https://docs.kicad.org/10.0/en/eeschema/eeschema.html)
- **KLC** · KiCad Library Conventions, symbol rules S3, S4, S6. [klc.kicad.org](https://klc.kicad.org/)
- **IEC 61082-1** · *Preparation of documents used in electrotechnology, Part 1: Rules*, ed. 3 (2014).
  [webstore.iec.ch/publication/10551](https://webstore.iec.ch/publication/10551)
- **Wikipedia, Circuit diagram** · drawing conventions.
  [en.wikipedia.org/wiki/Circuit_diagram](https://en.wikipedia.org/wiki/Circuit_diagram)
- **AoE** · Horowitz and Hill, rules for good schematics.
  [xcircuit.sourceforge.net/goodschem/goodschem.html](https://xcircuit.sourceforge.net/goodschem/goodschem.html)
- **Hack Club Hardware Codex** · *Schematic practices*.
  [hwdocs.hackclub.dev](https://hwdocs.hackclub.dev/user-contrib-guides/schematic-practices/)
- **EMA checklist** · EMA Design Automation, *Schematic Guidelines Checklist* (2021). [ema-eda.com
  (PDF)](https://ema-eda.com/wp-content/uploads/2025/04/2021SchematicGuidelines_Checklist.pdf)
- **Altium** · *Off-Grid Object* compile check.
[altium.com](https://www.altium.com/documentation/altium-designer/workspacemanager-err-offgridobjectoff-grid-object-ad)

## `DRAW-1` Ground symbol not pointing down (rotated or mirrored) <a id="draw-1"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-1)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** A ground power symbol (GND, AGND, PGND, chassis, earth) placed rotated or flipped so its bars do
not point down.

**Why it matters.** On a sheet where ground points down, the vertical axis reads as voltage. A ground symbol on its
side reads as a signal at first glance, and one pointing up sits where a supply is expected.

**Example.** `#PWR014` GND under C12 rotated 90° to save a grid row: the cap looks like it hangs on a net named GND
going right.

**How to check.** For every power symbol whose net is a ground (`NET-5`), read its rotation and mirror: anything but
0° unmirrored (or mirrored left-right) is a finding.

**Fix.** Rotate it back (`R`) and move it below the pin; a short wire stub is better than a sideways symbol.

**References.** Wikipedia, Circuit diagram; Horowitz and Hill (AoE); Hack Club Hardware Codex; KLC S4.2. Guide: [power
and ground](../drawing-guide.md#power-and-ground).

## `DRAW-2` Positive supply symbol not pointing up (negative rails pointing down are right) <a id="draw-2"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-2)

**Impl.** mid · depends on NET-5, NET-26

**What it catches.** A positive supply power symbol (`+3V3`, `VBUS`) rotated or flipped so it does not point up.
Negative rails (`-5V`) point down like grounds and are not reported.

**Why it matters.** Same reason as DRAW-1: supplies at the top, grounds at the bottom is the convention every reader
carries; a supply hanging below a part reads as a ground.

**Example.** `+5V` drawn under the pull-up of a reset line: the reader takes the line for pulled down.

**How to check.** For every power symbol on a positive rail (`NET-5` voltage above 0), read rotation and mirror; not
0° unflipped is a finding.

**Fix.** Rotate it and put it above the pin.

**References.** Wikipedia, Circuit diagram; AoE; Hack Club Hardware Codex. Guide: [power and
ground](../drawing-guide.md#power-and-ground).

## `DRAW-3` Symbol pin, wire end, label anchor or sheet pin off the 1.27 mm connection grid ≈ WIRE-3 <a id="draw-3"></a>

⚠️ warning · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-3)

**Impl.** mid · depends on NET-26

**What it catches.** A symbol pin, wire end, label anchor, junction or sheet pin whose position is not on the 1.27 mm
(50 mil) grid (or the project's connection grid).

**Why it matters.** KiCad joins items only where their points coincide exactly. Off-grid items are where
half-connected wires and dangling labels come from (`WIRE-3`), and everything later drawn to them is off too.

**Example.** A block pasted from an inch-grid project lands 0.635 mm off: its labels look attached and two of them are
not.

**How to check.** Every connection point modulo 1.27 mm (pins through the symbol's placement); report per sheet with
the count, the worst items first. KiCad's ERC has the same rule (`endpoint_off_grid`).

**Fix.** Set the grid to 50 mil, select the block, right-click > Align Items to Grid.

**References.** KiCad Schematic Editor, "Snapping"; KiCad ERC `endpoint_off_grid`; KLC S4.1; Altium "Off-Grid Object".
Guide: [grid and alignment](../drawing-guide.md#grid-and-alignment).

## `DRAW-4` Wire end or pin on the middle of another wire without a junction: drawn as a T, not connected ≈ WIRE-15 <a id="draw-4"></a>

⚠️ warning · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-4)

**Impl.** mid · depends on NET-26

**What it catches.** A wire end or a pin that lies on the middle of another wire, with no junction there: drawn as a
T, not connected.

**Why it matters.** The eye reads a T as a joint whether the dot is there or not; KiCad joins a wire end to the middle
of a wire only through a junction. The net splits silently into two.

**Example.** A pull-up's wire dragged onto the middle of `I2C_SCL` after the junction was deleted: the bus has no
pull-up, ERC reports only a dangling wire end.

**How to check.** For each wire end and pin, test it against the interior of every wire segment of its sheet; report
the ones with no junction at that point.

**Fix.** Add the junction (`J`), or end the wire on a pin or a wire end.

**References.** KiCad Schematic Editor, "Junctions"; AoE (dot means connect). Guide: [wires and
junctions](../drawing-guide.md#wires-and-junctions).

## `DRAW-5` Diagonal wire segment <a id="draw-5"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-5)

**Impl.** mid · depends on NET-26

**What it catches.** A wire segment that is neither horizontal nor vertical.

**Why it matters.** Diagonal wires cross more and are harder to follow; they usually come from a free-angle line mode
left on.

**Example.** A crystal's load cap wired at 45° across its neighbour's value.

**How to check.** Wire segments whose ends differ in both x and y.

**Fix.** Redraw it in 90° line mode (`Shift+Space` cycles the modes).

**References.** AoE (wires horizontal or vertical); KiCad line modes. Guide: [wires and
junctions](../drawing-guide.md#wires-and-junctions).

## `DRAW-6` Wire drawn on top of another wire, or of zero length (left over after a drag) <a id="draw-6"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-6)

**Impl.** mid · depends on NET-26

**What it catches.** Two collinear wire segments that overlap, or a segment whose two ends are the same point.

**Why it matters.** Overlaps and zero-length pieces are left by drags; they make junctions nobody drew and drag odd
pieces along on the next move.

**Example.** A wire from 10 to 40 mm and one from 20 to 60 mm on the same line: KiCad shows one wire, the file has
two, and deleting one leaves a gap.

**How to check.** Pairwise collinear segments with overlapping ranges; segments with equal ends.

**Fix.** Delete the extra piece and redraw one wire.

**References.** Our own rule. Guide: [wires and junctions](../drawing-guide.md#wires-and-junctions).

## `DRAW-7` Wire running through a symbol body, a field or a label <a id="draw-7"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-7)

**Impl.** hard · depends on NET-26

**What it catches.** A wire whose path crosses the body outline of a symbol, a visible field or a label it is not
attached to.

**Why it matters.** A wire over a symbol looks like a connection to a pin under it; over text it makes both
unreadable.

**Example.** A long `+3V3` wire drawn straight across a connector symbol to reach a cap on the other side.

**How to check.** Needs each placed symbol's body extents from its library graphics and each text's extents
(`NET-26`), then a segment-rectangle test.

**Fix.** Route the wire round the part, or use a label.

**References.** Our own rule. Guide: [wires and junctions](../drawing-guide.md#wires-and-junctions).

## `DRAW-8` Reference or value reading vertically: the fields of a rotated part turned with it instead of being placed horizontally <a id="draw-8"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-8)

**Impl.** mid · depends on NET-26

**What it catches.** A visible reference or value whose effective angle (field angle plus symbol rotation) is
vertical: the fields of a rotated part turned with it.

**Why it matters.** Vertical text is read with a tilted head, and a vertical field next to a vertical part is the
first thing to collide with a wire. KiCad places them horizontally when asked (Autoplace Fields).

**Example.** Series resistors on horizontal lines with `R13` and `1.5k` written upwards along the wire.

**How to check.** For each visible reference and value: field angle plus the symbol's rotation; 90° or 270° is a
finding.

**Fix.** Select the parts and press `O` (Autoplace Fields); keep "Automatically place symbol fields" on.

**References.** KLC S6.2; KiCad Autoplace Fields. Guide: [text](../drawing-guide.md#text).

## `DRAW-9` Text overlapping other text, a symbol body or a wire (fields, labels, notes) <a id="draw-9"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-9)

**Impl.** hard · depends on NET-26

**What it catches.** A field, label, pin name or note whose box overlaps another text, a symbol body or a wire.

**Why it matters.** Overlapping text cannot be read in print or PDF, and it is the most common reason a sheet looks
messy.

**Example.** Two parts placed close together, their references and values on top of each other (`C6R6`); a
reference over an IC's pin names.

**How to check.** Text extents from font size, length and justification, symbol body extents from the library
(`NET-26`); rectangle overlap.

**Fix.** Move the fields (`O` autoplaces them) or the parts apart.

**References.** Our own rule. Guide: [text](../drawing-guide.md#text).

## `DRAW-10` Labels stacked next to each other but not aligned (anchors a grid step or two apart) <a id="draw-10"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-10)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** Horizontal labels of the same direction stacked within a few grid steps of each other vertically,
whose anchors are a grid step or two apart horizontally.

**Why it matters.** A column of labels reads like a pin list; labels a step off look careless and hide which one
belongs to which wire.

**Example.** `SCL`, `SDA` and `INT_N` at x = 101.6, 102.87 and 104.14 mm.

**How to check.** Group horizontal labels by angle and nearby y; report groups whose x positions differ by under 5 mm
but are not equal.

**Fix.** Select them, right-click > Align > Align to Left (KiCad 10), or move the stubs.

**References.** KiCad Schematic Editor, "Aligning objects" (KiCad 10). Guide: [labels and
hierarchy](../drawing-guide.md#labels-and-hierarchy).

## `DRAW-11` Net label written vertically where a horizontal wire stub would carry it <a id="draw-11"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-11)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** A local, global or hierarchical label at 90° or 270°.

**Why it matters.** Labels are read most often of all texts on a sheet; vertical ones slow every reading. A short
horizontal stub almost always fits.

**Example.** `VREF` written upwards on the vertical wire to a reference.

**How to check.** Labels whose angle is 90° or 270°; suggestion only.

**Fix.** Add a short horizontal stub at the wire end and put the label on it.

**References.** Our own rule; Hack Club (labels on a wire). Guide: [labels and
hierarchy](../drawing-guide.md#labels-and-hierarchy).

## `DRAW-12` Symbol placed so its inputs face right and its outputs left (pin types as drawn, after rotation and mirror), against the sheet's left-to-right flow <a id="draw-12"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-12)

**Impl.** mid · depends on NET-7, NET-26

**What it catches.** A placed symbol whose input pins (by pin type, after rotation and mirror) sit on its right side
and whose output pins sit on its left.

**Why it matters.** Library symbols put inputs left and outputs right so the sheet flows left to right; a mirrored or
rotated IC runs the flow backwards and its pin names read the wrong way.

**Example.** A level shifter mirrored left-right to save a wire crossing: `A` on the right, `B` on the left, the
signal now runs right to left through it.

**How to check.** For each multi-pin symbol: the side of each pin after the placement transform, by electrical type
(`NET-7` when the types are wrong); more inputs on the right than the left and more outputs on the left than the right
is a finding.

**Fix.** Undo the mirror (`X`) and reroute; fix the library symbol if it is drawn that way.

**References.** KLC S4.2; IEC 61082-1 7.1.1. Guide: [signal flow](../drawing-guide.md#signal-flow).

## `DRAW-13` Hierarchical sheet symbol with input pins on its right side or output pins on its left <a id="draw-13"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-13)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** A hierarchical sheet symbol with input-shaped sheet pins on its right edge or output-shaped pins
on its left edge.

**Why it matters.** On the parent sheet the sheet symbols are the block diagram; inputs left and outputs right make it
read as signal flow.

**Example.** A `Flash` sheet symbol with `MOSI` (input) on its right edge, wired back across the symbol to the MCU on
the left.

**How to check.** Sheet pin side from its position on the sheet box, shape from the pin; inputs on the right or
outputs on the left are findings.

**Fix.** Drag the sheet pins to the other edge.

**References.** Our own rule, after KLC S4.2. Guide: [labels and hierarchy](../drawing-guide.md#labels-and-hierarchy).

## `DRAW-14` Connector drawn in the middle of the circuit instead of at the sheet's edge, with its protection beside it <a id="draw-14"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-14)

**Impl.** mid · depends on NET-19, NET-26

**What it catches.** A connector symbol placed well inside the drawing's bounding box rather than at its left or right
side.

**Why it matters.** Connectors are where the outside world enters; with the connector at the edge and its protection
beside it, a reviewer sees at once whether every line passes its protection first.

**Example.** A USB-C receptacle in the middle of the sheet, its ESD array at the top right next to the hub.

**How to check.** Connectors by part class (`NET-19`); position relative to the sheet's drawing bounding box: outside
the outer fifth on both sides is a finding.

**Fix.** Move the connector to the edge its signals come from and its protection next to it.

**References.** Our own rule. Guide: [signal flow](../drawing-guide.md#signal-flow).

## `DRAW-15` Decoupling capacitor drawn far from the power pin it serves, or on another sheet ≈ LAY-17 <a id="draw-15"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-15)

**Impl.** mid · depends on NET-22, NET-26

**What it catches.** A capacitor from a supply pin's rail to ground (`NET-22`) drawn far from the part it decouples
(more than about 25 mm, or on another sheet).

**Why it matters.** The layout is placed from the schematic; caps drawn in a pile end up in a pile, and a reviewer
cannot count one cap per pin.

**Example.** Twelve 100 nF caps in a grid in a corner of the MCU sheet, each with `+3V3` and `GND` symbols.

**How to check.** For each decoupling cap, the distance on the sheet to the nearest power pin of a part on its rail;
report the far ones per part.

**Fix.** Draw each cap next to its power pin.

**References.** Hack Club Hardware Codex; EMA checklist. Guide: [power and
ground](../drawing-guide.md#power-and-ground).

## `DRAW-16` Drawing fills too little of the sheet (paper too big for it) or runs into the frame or the title block <a id="draw-16"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-16)

**Impl.** mid · depends on NET-26

**What it catches.** A sheet whose drawing covers under about 30 % of the usable area inside the frame, or any item
outside the frame or over the title block.

**Why it matters.** A small drawing on a big sheet prints small; one under the title block or over the frame loses
parts in print.

**Example.** An A3 with one LDO in the top left corner; a label half under the title block.

**How to check.** Bounding box of all items (with symbol and text extents where known) against the paper size minus
frame and title block.

**Fix.** Use a smaller paper size, merge the sheet with another, or move the drawing in.

**References.** Our own rule. Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-17` Drawing not centred on the sheet <a id="draw-17"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-17)

**Impl.** mid · depends on NET-26

**What it catches.** The centre of the drawing's bounding box more than about 10 % of the usable width or height away
from the centre of the area above the title block.

**Why it matters.** A centred drawing looks deliberate and prints with even margins; a drawing in a corner leaves the
reader looking for it.

**Example.** Every circuit in the top half of an A3, the lower half empty but for the notes.

**How to check.** Bounding box centre against the usable area's centre.

**Fix.** Select all and move the drawing to the middle.

**References.** Our own rule. Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-18` Sheet too dense to read at its paper size: split it <a id="draw-18"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-18)

**Impl.** mid · depends on NET-26

**What it catches.** More parts (or wire segments) per dm² of paper than a reader can take in: above about 4 parts per
dm².

**Why it matters.** A crowded sheet means long wires and labels between unrelated parts and text that collides; a
split sheet is cheaper than a slow review.

**Example.** An A4 with 60 parts: a whole power tree squeezed onto one page.

**How to check.** Parts per paper area; threshold `max_density` per project.

**Fix.** Split the sheet by function, or move to A3.

**References.** EMA checklist (sheets by function). Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-19` Title block without a title, revision or date, or with a text variable that does not resolve <a id="draw-19"></a>

💡 suggestion · needs: the title block · [catalogue entry](../checks.md#draw-19)

**Impl.** mid · depends on NET-26

**What it catches.** A sheet whose title block has no title, revision or date, or shows a text variable that resolves
to nothing (`${REVISION}` with no revision set).

**Why it matters.** A printed page or a PDF page is read alone; without a title and revision nobody knows what it
shows.

**Example.** Page 5 with an empty title block, printed for a bring-up and taken for the previous revision.

**How to check.** Title block fields per sheet, text variables resolved the way KiCad does.

**Fix.** Fill Page Settings once and export to all sheets; use `${REVISION}` and friends.

**References.** IEC 61082-1 annex B; KiCad text variables. Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-20` Sheets of one project with different revisions in their title blocks <a id="draw-20"></a>

⚠️ warning · needs: the title blocks of all sheets · [catalogue entry](../checks.md#draw-20)

**Impl.** mid · depends on NET-26

**What it catches.** Two sheets of one project whose title blocks give different revisions.

**Why it matters.** One of them is wrong, and a reader who has only that page takes it for another board revision.

**Example.** Rev B on the index, rev A on the four sheets copied from the previous board.

**How to check.** Revision per sheet; more than one distinct value is a finding naming the odd sheets.

**Fix.** Set the revision on one sheet and export it to all (Page Settings), or use `${REVISION}`.

**References.** KiCad Page Settings. Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-21` Sheets of one project on mixed paper sizes, or a sheet larger than A3 <a id="draw-21"></a>

💡 suggestion · needs: the paper size of all sheets · [catalogue entry](../checks.md#draw-21)

**Impl.** mid · depends on NET-26

**What it catches.** Sheets of one project on different paper sizes, or any sheet larger than A3 (or ANSI B).

**Why it matters.** Zoom and print settings change from page to page; an A2 or larger sheet does not print readably on
a desk printer.

**Example.** An A0 overview sheet beside A4 detail sheets.

**How to check.** Paper size per sheet.

**Fix.** One size for the project; split a big sheet.

**References.** Our own rule. Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-22` Reference designator hidden on a placed part (power symbols, logos and mounting holes aside) <a id="draw-22"></a>

💡 suggestion · needs: symbol fields · [catalogue entry](../checks.md#draw-22)

**Impl.** mid · depends on NET-26

**What it catches.** A placed symbol (not a power symbol, logo, mounting hole or fiducial) whose reference field is
hidden.

**Why it matters.** The reference is how a part is talked about in review, in the BOM and on the board; a hidden one
makes the sheet useless for all three.

**Example.** A test point symbol with its reference hidden to save space: `TP7` cannot be found on the sheet.

**How to check.** Visibility of the Reference field per placed symbol, minus the classes that are exempt.

**Fix.** Show it and autoplace it (`O`).

**References.** KLC S6.2. Guide: [text](../drawing-guide.md#text).

## `DRAW-23` Value or a field says DNP, NF or DNF while the part's DNP attribute is not set: the sheet, the BOM and the placement files disagree ≈ BOM-13 <a id="draw-23"></a>

⚠️ warning · needs: symbol fields and attributes · [catalogue entry](../checks.md#draw-23)

**Impl.** mid · depends on NET-20

**What it catches.** A part whose value or another field says DNP, DNF, NF or DNM while its DNP attribute is not set.

**Why it matters.** The text marks nothing: the part is still in the BOM and placement files, bought and placed with a
value of "DNP"; the sheet does not show it crossed out.

**Example.** `R20` with value `DNP`: the assembler asks what a DNP resistor is.

**How to check.** Value and fields against the DNP flag of the variant view (`NET-20`).

**Fix.** Set the DNP attribute (or the variant) and put the real value back.

**References.** KiCad DNP attribute and variants. Guide: [parts](../drawing-guide.md#parts).

## `DRAW-24` Text smaller than 1.27 mm (fields, labels, pin names, notes): unreadable on a print of the sheet's paper size <a id="draw-24"></a>

💡 suggestion · needs: text sizes · [catalogue entry](../checks.md#draw-24)

**Impl.** mid · depends on NET-26

**What it catches.** A field, label, pin name or note with a font height below 1.27 mm (50 mil).

**Why it matters.** Below 1.27 mm text disappears when an A3 sheet is printed on A4 or viewed as a PDF at page width.

**Example.** Pin names at 0.76 mm on a 100-pin connector symbol.

**How to check.** Font size of every text item, field and pin name.

**Fix.** Set the text to 1.27 mm; split a crowded symbol.

**References.** KLC S3.2; KiCad default text size. Guide: [text](../drawing-guide.md#text).

## `DRAW-25` Hierarchical sheet without a meaningful name (`Sheet1`, `Untitled`), or two sheets with the same title <a id="draw-25"></a>

💡 suggestion · needs: sheet names · [catalogue entry](../checks.md#draw-25)

**Impl.** mid · depends on NET-26

**What it catches.** A sheet named `Sheet1`, `Untitled` or after its UUID, or two sheets whose title block titles are
the same.

**Why it matters.** The sheet name is the hierarchy navigator's entry, the PDF bookmark and the page title.

**Example.** `Sheet2` holding the Ethernet PHY.

**How to check.** Sheet names against KiCad's default patterns; title duplicates across sheets.

**Fix.** Name the sheet after its function.

**References.** Our own rule. Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-26` Sheet with calculated parts (feedback divider, current-set resistor, filter, timing) and no note that gives the calculation or the reason <a id="draw-26"></a>

💡 suggestion · needs: text notes and part roles · [catalogue entry](../checks.md#draw-26)

**Impl.** judgement · depends on NET-23, NET-26

**What it catches.** A sheet with parts whose value is calculated (a feedback divider, a current-set or timing
resistor, a filter) and no text note that gives the calculation or the reason.

**Why it matters.** A value without its calculation gets "fixed" in the next revision; a reviewer checks a written
calculation in seconds.

**Example.** A buck's 261k/1M divider with no note of the output voltage it sets.

**How to check.** Passive roles (`NET-23`) name the calculated parts; the sheet's notes are searched for their
references or values. better_erc lists candidates only: whether a note explains is judgement.

**Fix.** Add a note next to the parts with the formula and the result.

**References.** EMA checklist (notes). Guide: [the sheet](../drawing-guide.md#the-sheet).

## `DRAW-27` Colour set by hand on wires, labels or text: it carries meaning a black and white print loses <a id="draw-27"></a>

💡 suggestion · needs: colours of drawing items · [catalogue entry](../checks.md#draw-27)

**Impl.** mid · depends on NET-26

**What it catches.** Wires, labels, fields or text with a colour set on the item instead of the theme's.

**Why it matters.** Hand-set colours vanish in a black and white print and in another colour theme; if they carry
meaning, the meaning is lost.

**Example.** Green, cyan and magenta hierarchical labels whose colour means the voltage domain.

**How to check.** Item colours that are not "default" (alpha 0) in the file.

**Fix.** Put the meaning in names or net classes; colour net classes for the screen if wanted.

**References.** Our own rule. Guide: [colour print and export](../drawing-guide.md#colour-print-and-export).
