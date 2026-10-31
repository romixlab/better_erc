# DRAW checks, explained

Detail page for the `DRAW` category of the [check catalogue](../checks.md#draw-1). The one-line entries in the
catalogue stay the source of truth for ID, severity and wording; this page is the commentary. The rules behind them
are in the [schematic drawing guide](../drawing-guide.md). Most checks here are obvious and get one line on why they
matter; the ones that are not (a measure with a setting, a KiCad quirk, a judgement) say how better_erc finds them.

These checks read the drawing, not the netlist: positions, angles, fields, text, the title block. The drawing facts
(`NET-21`) carry labels and flags today; the geometry they need (wires with their ends, field positions and angles,
text, text boxes, tables and groups, paper size, title block, symbol body extents) is `NET-26`, planned. The
**Impl.** line says how hard the check is for better_erc in pure Rust: `mid` once `NET-26` is there, `hard` where
symbol and text extents must be computed, `judgement` where better_erc can only list candidates.

**Boxes.** Several checks compare item boxes: a symbol's body with its pins (from the library graphics, placed by its
rotation and mirror), a field's or label's text box (from font size, text length, justification and the angle
relative to its symbol), a power symbol with its name, a note box, a wire as a thin box. A prototype of this
measure (Python, run on the io_protection sheets) is kept with the P2503 house-style notes; it is the reference for
`NET-26`.

**Drawing settings.** The spacing checks share named settings with defaults, set per project in its better_erc
project file (`CFG-2`); see [clearance margins](../drawing-guide.md#room-around-everything-clearance-margins):

| Setting | Default | Meaning |
|---|---|---|
| `drawing.clearance.part` | 1.27 mm | margin around a symbol's body and its pins |
| `drawing.clearance.power` | 1.27 mm | margin around a power or ground symbol and its name |
| `drawing.clearance.text` | 0.635 mm | margin around a field, a label, a pin name or number, a note box |
| `drawing.section_gap` | 10 mm | gap that separates two sections (`DRAW-31`, `DRAW-16`) |
| `drawing.bus_hint.places` | 3 | places each net of a group must reach before `DRAW-39` suggests a bus |
| `drawing.bus_hint.members` | 4 | nets a group needs before `DRAW-39` suggests a bus |
| `drawing.route.w_len`, `.w_bend`, `.w_cross`, `.w_cross_near`, `.w_symbol`, `.w_detour` | 1, 2, 4, 8, 6, 4 | the routing cost weights ([routing is a weighed compromise](../drawing-guide.md#routing-is-a-weighed-compromise)), `DRAW-40` |
| `drawing.route.detour_max` | 1.5 | drawn length over Manhattan length of a tie above which `DRAW-40` looks for a cheaper drawing |
| `drawing.pin_lead` | 2.54 mm | straight wire at a pin or power symbol before the first bend (`DRAW-41`) |
| `drawing.align.window` | 20.32 mm | neighbourhood within which items must align exactly (`DRAW-36`) |
| `drawing.align.snap` | 5.08 mm | offsets below this (and above zero) are near misses (`DRAW-36`) |
| `drawing.label.stub_extra` | 1.27 mm | stub a local label needs beyond its text length (`DRAW-28`) |
| `drawing.heading_size` | 2.54 mm | text size of a section heading, bold (`DRAW-31`, `DRAW-32`) |
| `drawing.group_heading_size` | 1.905 mm | text size of a group heading (a table's title, a group of directives), bold (`DRAW-32`) |
| `drawing.leader.clear` | 0.635 mm | gap from a leader's tip to the part's outline (`DRAW-46`) |
| `drawing.note.item_words` | 15 | most words in a sentence of a note item (`DRAW-47`) |
| `drawing.note.item_marks` | 3 | most `;`, `:` and `(` in one sentence of a note (`DRAW-47`) |
| `drawing.note.colour_slack` | 8 | counts per RGB channel a note box colour may be off its kind's colour (`DRAW-32`) |
| `drawing.route.k_power`, `.k_ground`, `.k_signal` | 2, 2, 1 | factor on the bends and detours of power, ground and signal ties (`DRAW-40`) |
| `drawing.route.min_saving` | 2 | cost a redraw must save to be reported (`DRAW-40`) |
| `drawing.route.step` | 2.54 mm | the grid step wire is counted in (`DRAW-40`) |
| `drawing.colour.pair_min` | 20 | least CIEDE2000 difference between touching items (`DRAW-42`) |

## `DRAW-1` Ground symbol not pointing down (rotated or mirrored) <a id="draw-1"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-1)

**Impl.** mid · depends on NET-21, NET-26

A ground symbol on its side reads as a signal; with ground down, the vertical axis reads as voltage.

## `DRAW-2` Positive supply symbol not pointing up (negative rails pointing down are right) <a id="draw-2"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-2)

**Impl.** mid · depends on NET-5, NET-26

A supply hanging below a part reads as a ground.

## `DRAW-3` Symbol pin, wire end, label anchor or sheet pin off the 1.27 mm connection grid ≈ WIRE-3 <a id="draw-3"></a>

⚠️ warning · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-3)

**Impl.** mid · depends on NET-26

KiCad connects only coinciding points: an item half a step off looks connected and is not (KiCad ERC
`endpoint_off_grid`).

## `DRAW-4` Wire end or pin on the middle of another wire without a junction: drawn as a T, not connected ≈ WIRE-15 <a id="draw-4"></a>

⚠️ warning · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-4)

**Impl.** mid · depends on NET-26

**What it catches.** A wire end or a pin that lies on the interior of another wire with no junction there.

**Why it matters.** The eye reads a T as a joint whether the dot is there or not; KiCad joins a wire end to the
middle of a wire only through a junction, so the net splits silently. ERC reports at most a dangling end.

**How to check.** Each wire end and pin against the interior of every wire segment of its sheet; no junction at that
point is a finding.

## `DRAW-5` Diagonal wire segment <a id="draw-5"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-5)

**Impl.** mid · depends on NET-26

Diagonal wires cross more and are harder to follow.

## `DRAW-6` Wire drawn on top of another wire, or of zero length (left over after a drag) <a id="draw-6"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-6)

**Impl.** mid · depends on NET-26

Overlapping and zero-length pieces make junctions nobody drew.

## `DRAW-7` Wire running through a symbol body, a field or a label <a id="draw-7"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-7)

**Impl.** hard · depends on NET-26

A wire over a body looks like a pin connection; over text it makes both unreadable.

**Scope.** A wire through a symbol body, a power symbol or a pin of another net; a wire over text (fields, labels, notes) is `DRAW-9`.

## `DRAW-8` Reference or value reading vertically: the fields of a rotated part turned with it instead of being placed horizontally <a id="draw-8"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-8)

**Impl.** mid · depends on NET-26

Vertical text is read with a tilted head. The field angle in the file is relative to the symbol: the effective angle
is field angle plus symbol rotation, so horizontal text on a part rotated by 90° has a field angle of 90°.

## `DRAW-9` Text overlapping other text, a symbol body or a wire (fields, labels, notes) <a id="draw-9"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-9)

**Impl.** hard · depends on NET-26

**What it catches.** A field, label, pin name or number, or note box whose box overlaps another text, a symbol body
or a wire, with no margin: the hard overlap. Closeness within the clearance margins is `DRAW-34`.

**Why it matters.** Overlapping text cannot be read in print or PDF; it is the most common reason a sheet looks
messy.

**How to check.** The item boxes described at the top of this page, pairwise; exempt a label against the wire it lies
on and a part's own pin names against its own body. Text extents from the KiCad stroke font: width about 0.9 × the
font size per character is close enough for overlap, exact widths come from the font's glyph table.

## `DRAW-10` Labels stacked next to each other but not aligned (anchors a grid step or two apart) <a id="draw-10"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-10)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** Horizontal labels of the same justification in one column whose anchors are a grid step or two
apart in x.

**Why it matters.** A column of labels reads like a pin list; a step off looks careless.

**How to check.** Cluster labels into columns first: same justification, anchors within 5 mm in x, and each within
7.62 mm in y of the next one in the column, so labels far apart on the sheet that merely share a y band are never
paired. A column whose anchors differ but by less than 5 mm is a finding. The aligned edge is the anchor: the left
end of left-justified labels, the right end of right-justified ones (`DRAW-28`).

## `DRAW-11` Net label written vertically where a horizontal wire stub would carry it <a id="draw-11"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-11)

**Impl.** mid · depends on NET-21, NET-26

Labels are the most read text on a sheet; vertical ones slow every reading, and a horizontal stub almost always fits.

## `DRAW-12` Symbol placed so its inputs face right and its outputs left (pin types as drawn, after rotation and mirror), against the sheet's left-to-right flow <a id="draw-12"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-12)

**Impl.** mid · depends on NET-7, NET-26

A mirrored or rotated IC runs the flow backwards. Found from each pin's side after the placement transform and its
electrical type: more inputs on the right than the left and more outputs on the left than the right.

## `DRAW-13` Hierarchical sheet symbol with input pins on its right side or output pins on its left <a id="draw-13"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-13)

**Impl.** mid · depends on NET-21, NET-26

On the parent sheet the sheet symbols are the block diagram; inputs left, outputs right make it read as signal flow.

## `DRAW-14` Connector drawn in the middle of the circuit instead of at the sheet's edge, with its protection beside it <a id="draw-14"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-14)

**Impl.** mid · depends on NET-19, NET-26

With the connector at the edge and its protection beside it, a reviewer sees at once that every line passes its
protection first. Found from connectors (`NET-19`) outside the outer fifth of the circuit's bounding box on both
sides.

## `DRAW-15` Decoupling capacitor drawn far from the power pin it serves, or on another sheet ≈ LAY-17 <a id="draw-15"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-15)

**Impl.** mid · depends on NET-22, NET-26

The layout is placed from the schematic: caps drawn in a pile end up in a pile. Found as decoupling caps (`NET-22`)
more than about 25 mm from the nearest power pin on their rail.

## `DRAW-16` Drawing fills too little of the sheet (paper too big for it) or runs into the frame or the title block <a id="draw-16"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-16)

**Impl.** mid · depends on NET-26

**What it catches.** A circuit covering under about 30 % of the usable area inside the frame, or any item outside
the frame or over the title block.

**Why it matters.** A small drawing on a big sheet prints small; one under the title block loses parts in print.

**How to check.** The fill is measured on the circuit alone: symbols, wires, labels, section headings and parameter
boxes, without note boxes, tables and free text, which are often parked in a column and would make a cramped circuit
look like a full sheet. The frame and title block test takes every item.

## `DRAW-17` Drawing not centred on the sheet <a id="draw-17"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-17)

**Impl.** mid · depends on NET-26

A drawing in a corner leaves the reader looking for it. Measured like `DRAW-16`, on the circuit without the note
boxes: its bounding box centre more than 10 % of the usable width or height from the centre of the area above the
title block.

## `DRAW-18` Sheet too dense to read at its paper size: split it <a id="draw-18"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-18)

**Impl.** mid · depends on NET-26

A crowded sheet means long wires and colliding text. Threshold `max_density`, default 4 parts per dm² of paper.

## `DRAW-19` Title block without a title, revision or date, or with a text variable that does not resolve <a id="draw-19"></a>

💡 suggestion · needs: the title block · [catalogue entry](../checks.md#draw-19)

**Impl.** mid · depends on NET-26

A printed page is read alone: without a title and revision nobody knows what it shows.

## `DRAW-20` Sheets of one project with different revisions in their title blocks <a id="draw-20"></a>

⚠️ warning · needs: the title blocks of all sheets · [catalogue entry](../checks.md#draw-20)

**Impl.** mid · depends on NET-26

One of them is wrong, and a reader with only that page takes it for another revision.

## `DRAW-21` Sheets of one project on mixed paper sizes, or a sheet larger than A3 <a id="draw-21"></a>

💡 suggestion · needs: the paper size of all sheets · [catalogue entry](../checks.md#draw-21)

**Impl.** mid · depends on NET-26

Zoom and print settings change from page to page; A2 and up does not print readably on a desk printer.

## `DRAW-22` Reference designator hidden on a placed part (power symbols, logos and mounting holes aside) <a id="draw-22"></a>

💡 suggestion · needs: symbol fields · [catalogue entry](../checks.md#draw-22)

**Impl.** mid · depends on NET-26

A part without a visible reference cannot be named in review, the BOM or on the board.

## `DRAW-23` Value or a field says DNP, NF or DNF while the part's DNP attribute is not set: the sheet, the BOM and the placement files disagree ≈ BOM-13 <a id="draw-23"></a>

⚠️ warning · needs: symbol fields and attributes · [catalogue entry](../checks.md#draw-23)

**Impl.** mid · depends on NET-20

**What it catches.** A part whose value or another field says DNP, DNF, NF or DNM while its DNP attribute (in the
variant view, `NET-20`) is not set.

**Why it matters.** The text marks nothing: the part stays in the BOM and placement files and is bought and placed
with a value of "DNP"; the sheet does not cross it out.

## `DRAW-24` Text smaller than 1.27 mm (fields, labels, pin names, notes): unreadable on a print of the sheet's paper size <a id="draw-24"></a>

💡 suggestion · needs: text sizes · [catalogue entry](../checks.md#draw-24)

**Impl.** mid · depends on NET-26

Below 1.27 mm text disappears when an A3 is printed on A4.

## `DRAW-25` Hierarchical sheet without a meaningful name (`Sheet1`, `Untitled`), or two sheets with the same title <a id="draw-25"></a>

💡 suggestion · needs: sheet names · [catalogue entry](../checks.md#draw-25)

**Impl.** mid · depends on NET-26

The sheet name is the navigator entry, the PDF bookmark and the page title.

## `DRAW-26` Sheet with calculated parts (feedback divider, current-set resistor, filter, timing) and no note that gives the calculation or the reason <a id="draw-26"></a>

💡 suggestion · needs: text notes and part roles · [catalogue entry](../checks.md#draw-26)

**Impl.** judgement · depends on NET-23, NET-26

**What it catches.** A sheet with parts whose value is calculated (`NET-23` passive roles) and no note, parameter box
or table that names them or their values.

**Why it matters.** A value without its calculation gets "fixed" in the next revision.

**How to check.** Notes, parameter boxes and table cells are searched for the parts' references or values; whether
a note really explains is judgement, so better_erc lists candidates only.

## `DRAW-27` Colour set by hand on wires, labels or text: it carries meaning a black and white print loses <a id="draw-27"></a>

💡 suggestion · needs: colours of drawing items · [catalogue entry](../checks.md#draw-27)

**Impl.** mid · depends on NET-26

Off by default: it only matters for a black and white PDF, so a project that prints in black and white turns it on.
Hand-set colours vanish there. Not reported: net class colours (the class is in the data) and the note box kinds of
`DRAW-32` (their heading names the kind).

## `DRAW-28` Label not lying on its wire: text hanging off the stub's end, or justified against its side <a id="draw-28"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-28)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** A horizontal label whose text does not lie along the wire it is anchored on: the text runs off
the end of the stub into free space, or past the far end of a stub too short for it into the pin.

**Why it matters.** Text over its wire says which wire it names; text in the air next to a wire end can be read as
naming the wire below, and a label on the left of a block that runs away from it breaks the column of label ends.
House style (Roman, 31 Oct 2026): left side left-justified, right side right-justified.

**Example.** `SEL` anchored at the left end of the stub to `SEL1` with `justify right`: the text sits left of the
wire end, over nothing.

**How to check.** For each local, global or hierarchical label at 0° or 180°: the direction its text runs comes
from the justification (KiCad 10 ignores the angle for this: `justify left` runs right, `justify right` runs left),
the direction of its stub from the wire attached at the anchor. They must agree, and the stub must be at least as
long as the text box plus one grid step. A label on a pin or a joint without a stub of its own is reported here too.

**Fix.** Set the justification in the label's properties; lengthen the stub.

**Scope.** Local labels only: a global or hierarchical label's flag always starts at the wire end. The stub must be as long as the text plus `drawing.label.stub_extra`.

## `DRAW-29` Pin joined directly to another pin, with no wire between them <a id="draw-29"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-29)

**Impl.** mid · depends on NET-26

A pin-on-pin joint is invisible, cannot take a branch, and opens silently when a part is moved; at least one major
grid step (2.54 mm) of wire goes between two pins.

**Scope.** A power symbol on a pin is normal and not reported; two pins facing each other over less than `drawing.pin_lead` of wire count as joined.

## `DRAW-30` Several power or ground symbols of one rail on one pin group of a part, instead of the pins wired together to one symbol <a id="draw-30"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-30)

**Impl.** mid · depends on NET-21, NET-26

**What it catches.** A pin group (pins of one net next to each other on one side of a part's body) with more than
`max_symbols_per_rail` (default 1) power or ground symbols of that net attached, each directly or through a stub
that touches no other part. Two groups of one part with a symbol each are fine: whether they should share one is
the routing cost's call (`DRAW-40`).

**Why it matters.** A row of identical ground symbols under one IC is noise; wiring the pins together says "all
ground pins to ground" once.

**How to check.** Walk from each power symbol along its wires to the pins it reaches; count, per part and net, the
symbols whose wire reaches only that part. Decoupling capacitors and other two-pin parts are not counted against
the IC (they keep their own symbols).

**Scope.** At most `DRAW-30.max_symbols_per_rail` (default 1) symbols of one rail per pin group of a part.

## `DRAW-31` Section of a sheet without a heading naming its function <a id="draw-31"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-31)

**Impl.** judgement · depends on NET-26

**What it catches.** A sheet with two or more sections (groups of parts and wires separated by gaps of at least
`drawing.section_gap`) where a section has no heading text above it: bold, larger than the body text, within one
section gap of its top edge.

**Why it matters.** A heading says what a group of parts is before the reader works it out; without it the sheet is
a puzzle. House style (Roman, 31 Oct 2026).

**How to check.** Sections are clusters of symbol and wire boxes joined when closer than the section gap; whether
the heading names the function is judgement, so better_erc lists sections without any heading.

## `DRAW-32` Free-standing text, or a note box that is not one of the six kinds (TODO, Information, Caution, Debug, Critical, Layout) <a id="draw-32"></a>

💡 suggestion · needs: text and text boxes · [catalogue entry](../checks.md#draw-32)

**Impl.** mid · depends on NET-26

**What it catches.** A free text item that is not a section heading (bold, 2.54 mm) or a note box heading (bold,
grouped with its box), or that starts with `.` but is not a simulator directive; and a text box whose stroke colour
and heading are not one of the six kinds of the [drawing guide](../drawing-guide.md#notes-live-in-boxes-by-kind)
or a parameter box.

**Why it matters.** Loose text has no owner and no weight: a design reason, a forgotten TODO and a layout
instruction look the same. House style (Roman, 31 Oct 2026).

**How to check.** Free text: not bold, or bold but neither 2.54 mm nor grouped with a text box. Boxes: the stroke
colour against the six kinds' colours and the parameter box grey (within a few counts per channel), the heading
word against the kind's name.

**Scope.** Accepted outside a note box: section headings (`drawing.heading_size`, bold), group headings (`drawing.group_heading_size`, bold), titles (bold above 2.54 mm) and the captions in a sheet box.

## `DRAW-33` Note laid out as a table in plain text (aligned columns, one `name: value` per line) <a id="draw-33"></a>

💡 suggestion · needs: text and text boxes · [catalogue entry](../checks.md#draw-33)

**Impl.** judgement · depends on NET-26

Space-padded columns break in any other font or in a PDF; a KiCad table stays aligned. Candidates: notes with three
or more lines of `name: value` or `name = value`, or runs of two or more spaces in the same column on several lines.

## `DRAW-34` Items closer than their clearance margins (symbol bodies, power symbols, fields, labels, note boxes) <a id="draw-34"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-34)

**Impl.** hard · depends on NET-26

**What it catches.** Two items whose boxes, each grown by its own margin (`drawing.clearance.part`, `.power`,
`.text`), overlap, when the bare boxes do not (that is `DRAW-9`). With the defaults: two parts less than 2.54 mm
apart, a field less than 1.905 mm from another part, two labels less than 1.27 mm apart.

**Why it matters.** A sheet where every gap is the minimum reads as one block, and a field close to the next part is
read as belonging to it. The margins are settings because how much air a sheet needs is taste: Roman's io_protection
sheets were too dense at zero, the defaults stay small.

**Example.** A `+3V3` power symbol on a one-step stub beside a series resistor: its name lies 0.5 mm from the
resistor's body.

**How to check.** The item boxes at the top of this page, grown by their margins; exempt a part's own fields and pin
names against its own body, a label against its own wire, a heading against its parameter box. Report per sheet,
closest pairs first.

## `DRAW-35` Two- to four-pin part tied to a supply or ground drawn horizontal <a id="draw-35"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-35)

**Impl.** mid · depends on NET-5, NET-26

A diode, TVS, resistor or capacitor with a pin on a rail or ground (a BAT54S clamp, a pull-down, a TVS) reads as a
shunt when it stands on the vertical voltage axis, and as a part in the signal path when it lies horizontal. Found
from the direction of its rail or ground pins after the placement transform: pointing left or right is a finding.

**Scope.** Discrete classes only (R, C, L, ferrite bead, diode, fuse) with up to `DRAW-35.max_pins` (default 4) pins, not ICs; a part between two rails with no ground (a series ferrite or shunt) may lie horizontal.

## `DRAW-36` Near-miss alignment <a id="draw-36"></a>

⚠️ warning · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-36)

**Impl.** mid · depends on NET-26

**What it catches.** Two alignable items (label anchors, power and ground symbol pins, vertical or horizontal tie
wires, part body edges and centre lines) that are neighbours (on the same side of the same part, or within
`drawing.align.window` of each other) and whose x (or y) differs by more than zero and less than
`drawing.align.snap`. Each finding names the pair and the offset ("+3V3_LDO at x 152.4, GND at x 149.86: 2.54 mm").
Larger offsets are deliberate and not reported. Within a section the ladders are compared on their own: its ground
symbols, its supplies, its input ports and its output ports ([ladders](../drawing-guide.md#ladders-one-row-for-ground-one-for-supplies-ports-in-columns)).
Wires that end on pins of one symbol are not compared (their pitch is the symbol's), and bodies of different size
compare centres only (a shared pin line or edge clears them).

## `DRAW-37` Note box that names a part or block and has no leader to it <a id="draw-37"></a>

💡 suggestion · needs: text boxes, graphic lines · [catalogue entry](../checks.md#draw-37)

**Impl.** judgement · depends on NET-26

A note about a part that does not point at it makes the reader match references by eye. Candidates: a note box
whose text names a reference on its sheet (or a section heading) with no leader line starting at the box and ending
at that part or its block's rectangle.

## `DRAW-38` Leader line crossing a wire, a part, a label or text <a id="draw-38"></a>

💡 suggestion · needs: the schematic drawing (geometry), graphic lines · [catalogue entry](../checks.md#draw-38)

**Impl.** hard · depends on NET-26

A leader drawn through things reads as a wire or hides what it crosses. Found as leader segments (dotted lines in
the leader colour) against the item boxes at the top of this page, minus its own target and box.

## `DRAW-46` Leader into a small part off its centre, not diagonal, or clipped by the outline <a id="draw-46"></a>

💡 suggestion · needs: the schematic drawing (geometry), graphic lines · [catalogue entry](../checks.md#draw-46)

**Impl.** mid · depends on NET-26

Rule: [a note about a part points at it](../drawing-guide.md#a-note-about-a-part-points-at-it). For a leader ending
at a small part (R, C, diode, TVS, up to 4 pins): the last segment must be a 45° diagonal, its line must pass through
the centre of the body within one grid step, and the tip must stop `drawing.leader.clear` from the outline, not on or
inside it. Leaders to a block's rectangle and to larger parts are out of scope.

## `DRAW-47` Run-on note text <a id="draw-47"></a>

💡 suggestion · needs: text boxes · [catalogue entry](../checks.md#draw-47)

**Impl.** easy · depends on NET-26

Rule: [note boxes are lists and tables](../drawing-guide.md#note-boxes-are-lists-and-tables-not-paragraphs). In a note
box, a sentence longer than `drawing.note.item_words` words, or with more than `drawing.note.item_marks` of `;`, `:`
and `(`. A sentence runs over line ends until a full stop or a new item. Skipped: SPICE directives, the explanation
box beside them, and table cells.

## `DRAW-39` Group of related nets labelled in many places, with no bus <a id="draw-39"></a>

💡 suggestion · needs: labels, wires and buses (drawing) · [catalogue entry](../checks.md#draw-39)

**Impl.** mid · depends on NET-21, NET-26

Rule: [buses for groups of signals that go to many places](../drawing-guide.md#buses-for-groups-of-signals-that-go-to-many-places).
A dozen labels of one group scattered over a sheet make the reader collect the group by name; a bus says it once.
Found as nets whose names share a stem with indices (`D0`..`D7`, `P0_3`, `DQ[n]`) or one prefix (`I2C1_SDA`,
`I2C1_SCL`), at least `drawing.bus_hint.members` of them, each with labels or pins in at least
`drawing.bus_hint.places` places (parts or label positions), none of them on a bus. Only a hint: a group whose
members go to different places each is not a bus.

## `DRAW-40` Power or tie wiring much longer than it needs, with a cheaper drawing at hand <a id="draw-40"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-40)

**Impl.** hard · depends on NET-26

Rule: [routing is a weighed compromise](../drawing-guide.md#routing-is-a-weighed-compromise).

**What it catches.** A tie (the wires of one power net, or of a few pins tied together, between its pins and its
power symbols) whose drawn length is more than `drawing.route.detour_max` times its Manhattan length (the
half-perimeter of the box around its pins and symbols), where a cheaper drawing exists by the routing cost.

**How to check.** Score the drawn tie with the `drawing.route.*` weights (length in grid steps, bends, crossings
with other nets, crossings within one grid step of a pin end or junction, extra symbols, detour ratio). Score the
candidates: each pin group dropped straight along its pin axis to a symbol of its own, and the pin groups joined
by the shortest L-shaped path to the existing symbol, counting the crossings each would make with the wires
already drawn. Power and ground ties weigh their bends and detours `drawing.route.k_power` / `k_ground` times; ties
are read where a power symbol ends them, or among pins of one part tied without a label; `PWR_FLAG` stubs are not
part of the tie. Candidates are drawn the way the guide asks: each starts with the pin lead (`drawing.pin_lead`,
its bend counted) and enters the symbol along its axis with a lead; a straight drop ends level with the tie's own
symbol, so it stays on the section's power or ground row; a candidate touching another net's wire is dropped. Report the drawn cost and
the cheapest candidate with its cost ("straight drop from D2..D4 to a
second GND: 16 instead of 19"). A suggestion only: the candidates ignore parts that would have to move.
A shared rail wire (several parts joined on one power wire with one symbol, as on a clamp or LDO rail) is weighed
against a straight drop per part like any other tie: both drawings are fine, the cheaper wins (Roman, 31 Oct 2026).

## `DRAW-41` Wire bending right at a pin or a power symbol <a id="draw-41"></a>

💡 suggestion · needs: the schematic drawing (geometry) · [catalogue entry](../checks.md#draw-41)

**Impl.** mid · depends on NET-26

A wire leaves a pin, and enters a pin or a power symbol's pin, along that pin's axis for at least
`drawing.pin_lead` (default one major grid step, 2.54 mm) before it bends: the pin's direction stays readable and
the corner is not mistaken for a junction. Found from the wire segment that ends at the pin: shorter than the
setting, or perpendicular to the pin's axis (the pin's direction after the symbol's rotation and mirror). The pin's
end point decides, whatever its length; a pin ending on the middle of a wire is a tee, not a bend (no finding),
but a tee right at the pin tip counts.

## `DRAW-42` Touching items too close in colour <a id="draw-42"></a>

💡 suggestion · needs: the schematic drawing, the project's net class colours, the colour theme · [catalogue entry](../checks.md#draw-42)

**Impl.** mid · depends on NET-26

Rule: [neighbours in different colours](../drawing-guide.md#neighbours-in-different-colours). For each touching
pair (a wire end on a pin, a wire on a power symbol's pin, a label on its wire), the effective colours (the item's
own, else its net class colour, else the theme's) on the light and the dark theme, and their CIEDE2000 difference;
below `drawing.colour.pair_min` (default 20) is a finding, reported once per colour pair with a count. The themes
come from the KiCad colour settings named in the better_erc project file.

## `DRAW-43` Sentence starting with a lower-case letter <a id="draw-43"></a>

💡 suggestion · needs: text, text boxes, table cells · [catalogue entry](../checks.md#draw-43)

**Impl.** easy · depends on NET-26

The first word of a text item, text box or table cell, of each paragraph in it, and of each sentence after a full
stop starts lower case (a line that continues a sentence from the line before is not a start). Skipped: lines starting with a net name or reference known on the sheet, a unit or value (`mA`, `20u`,
`nX`), code (a `.` directive, `i(`, `v(`) or a parameter name followed by `:` or `=`.

**Scope.** A single word (a table value, a case name) is no sentence.
