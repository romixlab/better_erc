# LAY checks, explained

Detail page for the `LAY` category of the [check catalogue](../checks.md#-lay---layout-placement-and-fabrication). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs
to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for
ID, severity and wording; this page is the commentary.

The **Impl.** line rates how hard the check is to build in better_erc, in pure Rust and without an LLM: `easy` (the
netlist alone: connectivity, pin types, part classes and values), `mid` (part data, a project setting or data read from
the board file is needed first), `hard` (analysis or layout geometry the code does not have yet), `judgement` (not a
rule: a human or an LLM reads intent or a document, better_erc can at most list candidates). The IDs after "depends on"
are [FEATURES.md](../../FEATURES.md) items. Nearly every LAY check needs the board file: `berc textconv` already parses
`.kicad_pcb` (`TOOL-3`), but no check reads layout data yet.

References used throughout:

- **KiCad PCB** · KiCad 9 PCB Editor manual: forward annotation, design rules checking, rule areas, zones, board
  stackup, fabrication outputs. [docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html)
- **KLC** · KiCad Library Conventions, rules F5.1 (silkscreen) and F5.3 (courtyard). Read from the source repository
  (klc.kicad.org sits behind a bot check). [gitlab.com/kicad/libraries/klc](https://gitlab.com/kicad/libraries/klc)
- **JLCPCB capabilities** · a typical low-cost fab's published limits: edge clearances, V-cut, silkscreen, panel rails
  and fiducials (fetched 22 Oct 2026).
  [jlcpcb.com/capabilities/pcb-capabilities](https://jlcpcb.com/capabilities/pcb-capabilities)
- **SLUA271** · TI, *QFN and SON PCB Attachment*, Rev. C (2023): land patterns, stencil design for exposed pads.
  [ti.com/lit/an/slua271](https://www.ti.com/lit/an/slua271/slua271.pdf)
- **MT-101** · Analog Devices, *Decoupling Techniques*, tutorial MT-101 (fetched through `tpm fetch`, analog.com refuses
  plain HTTP clients).
  [analog.com/.../MT-101.pdf](https://www.analog.com/media/en/training-seminars/tutorials/MT-101.pdf)
- **AN2867** · ST, *Guidelines for oscillator design on STM8AF/AL/S and STM32 MCUs/MPUs*, Rev. 24 (February 2026),
  fetched through `tpm fetch`. [st.com
  AN2867](https://www.st.com/resource/en/application_note/an2867-guidelines-for-oscillator-design-on-stm8afals-and-stm32-mcusmpus-stmicroelectronics.pdf)
- **ESP32 HW guide** · Espressif, *ESP32 Hardware Design Guidelines*, "PCB Layout Design".
  [docs.espressif.com/.../pcb-layout-design.html](https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32/pcb-layout-design.html)
- **IPC-2221B** · IPC, *Generic Standard on Printed Board Design*: conductor spacing by voltage, conductor sizing.
  **IPC-2152** · IPC, *Standard for Determining Current Carrying Capacity in Printed Board Design*. **IPC-D-356A** ·
  IPC, *Bare Substrate Electrical Test Data Format*. **IEC 62368-1** · *Audio/video, information and communication
  technology equipment, Part 1: Safety requirements* (creepage and clearance for insulation). By title.
- **Ott** · Henry W. Ott, *Electromagnetic Compatibility Engineering* (Wiley, 2009), chapter 11 (digital circuit power
  distribution) and chapter 16 (PCB layout and stackup).

## `LAY-1` Schematic and layout match <a id="lay-1"></a>

🛑 error · needs: the netlist and the board file's pads and nets · [catalogue entry](../checks.md#lay-1)

**Impl.** mid · depends on TOOL-2, TOOL-3

**What it catches.** Differences between the schematic's netlist and the board: a part in one and not the other, a pad
on a different net than its pin, a footprint or value changed on one side only.

**Why it matters.** Every other check reviews the schematic; the board is what gets built. A late fix made in the
schematic and never pushed to the layout (or a net edited by hand in the layout) means the reviewed design is not the
manufactured one. KiCad warns when the update is run, but nothing forces anyone to run it before the Gerbers are
plotted, and Altium and OrCAD flows have the same gap when the PCB comes from another person.

**Example.** A pull-up changed from 10 kΩ to 4.7 kΩ in the schematic after layout: the BOM (from the schematic) says 4.7
kΩ, the placement file and the assembly drawing (from the board) say 10 kΩ, and the assembler asks which one is right,
or does not ask. The same for a net rename such as B153's `+5V` to `VBUS_SW` (rev B): the silk keeps the old name unless
the board is updated.

**How to check.** Read pads, nets, footprints and values from the board file (`TOOL-3`) and compare them with the
netlist per reference designator (`TOOL-2`): missing and extra parts, pin-to-net differences, footprint and value
differences. Report them grouped by part.

**Fix.** Run "Update PCB from Schematic" and review its list; make all changes in the schematic first; plot fabrication
files only from a board with a clean update.

**References.** KiCad PCB "Update PCB From Schematic (forward annotation)"
([docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html#forward-annotation)) and the DRC option
"Test for parity between PCB and schematic" under "Design rules checking".

## `LAY-2` Parts in their areas, nothing in forbidden ones <a id="lay-2"></a>

⚠️ warning · needs: part positions, rule areas or a placement plan from the project · [catalogue
entry](../checks.md#lay-2)

**Impl.** hard · depends on TOOL-3, CFG-2

**What it catches.** Parts outside the area meant for them (components in the enclosure's keep-out, a connector not at
the edge it must reach), test points outside the area the bed-of-nails fixture can probe, and anything placed in a
forbidden zone (under a heatsink, in the clamping strip, under a label).

**Why it matters.** Mechanical constraints are invisible in the schematic and often only half known to the layout tool:
a part 1 mm into the screw boss area is found when the board does not go into its housing. Test points outside the probe
grid make the test fixture more expensive or impossible.

**Example.** A 6.5 mm tall electrolytic placed under the area where the enclosure lid has a 4 mm rib: the board passes
DRC, the lid does not close. Or a test point placed on the bottom side under a connector body where no probe can reach
it.

**How to check.** Read part positions and courtyards (`TOOL-3`) and the areas from the project (KiCad rule areas with
placement restrictions, or a mechanical outline per height class in the project data, `CFG-2`); report parts and test
points outside or inside the wrong areas.

**Fix.** Draw the mechanical constraints into the board as rule areas (keep-outs with height or component restrictions)
before placement, so DRC enforces them.

**References.** KiCad PCB "Rule areas (keepouts)"
([docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html#pcb-rule-areas)).

## `LAY-3` Auto courtyard check and fix <a id="lay-3"></a>

⚠️ warning · needs: courtyards, pads and 3D body outlines per footprint · [catalogue entry](../checks.md#lay-3)

**Impl.** hard · depends on TOOL-3

**What it catches.** A courtyard that is not an even margin around the part: tight on one side, generous on another,
smaller than the 3D body, or missing (`LIB-12`). The catalogue's note asks that the courtyard sit equidistant from the
3D body and pad boundary, and that better_erc can add a proper one on request.

**Why it matters.** Courtyard overlap is what DRC uses to keep parts apart; a courtyard that is too small lets parts
touch, one that is too large forces needless space or gets ignored. Courtyards drawn by hand around imported footprints
are often just the pad outline, which ignores an overhanging body (an inductor wider than its pads, a connector with a
latch).

**Example.** A 5x5 mm shielded inductor on a footprint whose courtyard was drawn around its 4.5 mm pad span only: a 0402
decoupling capacitor placed against the courtyard ends up under the inductor's body edge. KLC asks for 0.25 mm around
the combined body and pads.

**How to check.** Compute the union of the pads and the 3D model's outline (from the footprint's model reference) and
compare it, grown by the margin from KLC F5.3, with the drawn courtyard; report a margin outside tolerance on any side.
The fix mode generates the courtyard from the same union.

**Fix.** Redraw the courtyard: 0.25 mm around body and pads (0.15 mm for parts under 1.5 mm, 0.5 mm for connectors,
canned capacitors and crystals, 1.0 mm for BGAs), plus mating space for connectors.

**References.** KLC F5.3 (courtyard layer requirements); IPC-7351B, *Generic Requirements for Surface Mount Design and
Land Pattern Standard* (courtyard excess), by title.

## `LAY-4` Check stackup <a id="lay-4"></a>

💡 suggestion · needs: the board's layer stack and which nets fill each layer · [catalogue entry](../checks.md#lay-4)

**Impl.** mid · depends on TOOL-3

**What it catches.** A stackup that gives signals a poor return path: a 4-layer board with a power plane between ground
and the bottom signals, or signals routed on a layer whose neighbour is a split power plane. The catalogue recommends
signal, ground, ground, signal (SGGS) for 4-layer boards.

**Why it matters.** A signal's return current flows in the plane next to it. If that plane is a power plane, or a plane
cut into several rails, the return path detours around the splits and through decoupling capacitors, and the loop
radiates and couples. Two ground planes give every signal layer a solid adjacent reference; power is routed as wide
traces or pours on the signal layers. It costs nothing at fabrication and saves EMC respins.

**Example.** A 4-layer board stacked signal / ground / 3.3 V / signal, with the 3.3 V plane split into 3.3 V, 1.8 V and
5 V areas: bottom-layer signals that cross the splits have no continuous return. With SGGS the same board has a solid
reference under every layer.

**How to check.** Read the stackup and the zones per inner layer from the board file; classify each inner layer by the
nets filling it (ground, power, signals, split); compare with the recommended pattern for the layer count and report
deviations as suggestions.

**Fix.** For 4 layers, SGGS with power on the outer layers as pours and wide traces; for more layers, every signal layer
next to a ground plane.

**References.** Eric Bogatin interviewed by Robert Feranec, *Do You Really Need Power Planes? Are you sure?* (video,
linked in the catalogue: [youtu.be/kdCJxdR7L_I](https://youtu.be/kdCJxdR7L_I)); Ott chapter 16; KiCad PCB "Configuring
board stackup and physical parameters".

## `LAY-5` Stitching vias <a id="lay-5"></a>

⚠️ warning · needs: ground vias and pours per layer, the highest frequency on the board · [catalogue
entry](../checks.md#lay-5)

**Impl.** hard · depends on TOOL-3, CFG-2

**What it catches.** Ground planes and pours on different layers joined by too few vias: spacing larger than the
catalogue's rule of at least six vias per wavelength of the highest frequency signal, so that the plane pair cannot be
excited as a cavity.

**Why it matters.** Two planes form a parallel-plate cavity with resonances whose wavelength is set by the board size.
Fast edges switching between layers (a signal via changing reference planes) inject current into that cavity; without
stitching vias nearby, it rings and radiates from the board edge. Pours on outer layers connected at one point are
worse: they become patch antennas.

**Example.** A board with 0.35 ns clock edges has content up to about 1 GHz. In FR-4 (εr around 4.3) the wavelength at 1
GHz is about 300 mm / √4.3 ≈ 145 mm, so six vias per wavelength means a ground via at least every 24 mm across the plane
pair, and next to every signal via that changes reference planes.

**How to check.** Read ground vias and pours (`TOOL-3`), take the highest frequency from the project (`CFG-2`) or from
the fastest interface, and find areas of overlapping ground pours where the largest via-free circle exceeds the spacing.
Also flag outer-layer pours connected by one or two vias.

**Fix.** Stitch ground pours on a grid at the computed spacing, along board edges, and next to every via where a fast
signal changes layers.

**References.** The catalogue's video ([youtu.be/kdCJxdR7L_I](https://youtu.be/kdCJxdR7L_I)); Ott chapter 16.

## `LAY-6` Polarized part without a pin-1 mark on the silk <a id="lay-6"></a>

⚠️ warning · needs: footprint silk graphics and their position on the board · [catalogue entry](../checks.md#lay-6)

**Impl.** hard · depends on TOOL-3

**What it catches.** A polarized part or an IC on the board with no visible pin-1 or polarity mark on the silkscreen:
missing in the footprint, hidden under the part body, covered by another part, or removed by silk clipping at the solder
mask.

**Why it matters.** The mark is what assembly operators and inspection compare the part against. Without it, a rotated
IC or reversed diode is only found at power-up, sometimes with damage. `LIB-9` fixes the footprint; this check looks at
the finished board, where a mark can still vanish under a neighbour or get clipped away.

**Example.** B125A: U3 and U12 had pin-1 text only on a mechanical layer. B135B: D201 needed a clear pin-1 mark, D601
and D603 used two different dot styles. B153A: the ISOM optocoupler had none, and rev B asks for bigger pin-1 marks with
text across the board.

**How to check.** For each polarized footprint, find silk graphics near pad 1 (or the cathode) that are outside the
part's body outline and not covered by another courtyard, and not clipped by the solder mask opening.

**Fix.** Add or move the mark outside the body, one style per board; a `1` next to it on dense boards.

**References.** KLC F5.1 (polarity marking / pin-1 designator; "the marking must be visible after board assembly"); B125
rev B, B135 rev C and B153 rev B change lists.

## `LAY-7` Test points without their net name on the silk <a id="lay-7"></a>

💡 suggestion · needs: test point footprints, silk text near them · [catalogue entry](../checks.md#lay-7)

**Impl.** mid · depends on TOOL-3

**What it catches.** A test point whose net name (or a short label for it) is not printed next to it.

**Why it matters.** At bring-up and repair, a test point labelled `TP12` sends everyone to the schematic for every
probe. With `SWCLK`, `3V3` or `VBUS` printed beside it, measurements go faster and fewer probes land on the wrong point
(a 20 V rail next to a 1.8 V one is how parts get killed).

**Example.** On B153A the SWD test points (SWCLK, ...) carried no names; the rev B change list adds silk on the test
points.

**How to check.** For each test point footprint, look for a silk text item in the footprint or near it that contains its
net name (or a value field shown on silk, which KiCad's test point footprints can carry).

**Fix.** Show the net name as silk text next to each test point (a text variable such as `${NET_NAME(1)}` in the
footprint keeps it in sync).

**References.** B153 rev B change list (silk on the test points); KiCad PCB "Editing footprint fields".

## `LAY-8` No fiducials on a machine-assembled board <a id="lay-8"></a>

⚠️ warning · needs: fiducial footprints, fine-pitch parts and their positions, assembly mode from the project ·
[catalogue entry](../checks.md#lay-8)

**Impl.** mid · depends on TOOL-3, CFG-2

**What it catches.** A board for machine assembly with fewer than three global fiducials, or without local fiducials
near fine-pitch parts (0.5 mm pitch QFPs and smaller, BGAs).

**Why it matters.** The pick-and-place machine locates the board by its fiducials, correcting for offset, rotation and
stretch of the panel. Without them it relies on the board edges or holes and places with less accuracy; fine-pitch parts
end up bridged or off pad. Many assemblers add fiducials on the panel rails, but local fiducials near a fine-pitch part
can only be on the board.

**Example.** A 0.4 mm pitch QFN-56 MCU on a board with no fiducials, panelised by the fab with rail fiducials only: the
panel's stretch over 200 mm is enough to move the MCU's pads a fraction of the pitch.

**How to check.** Count fiducial footprints (by footprint name or attribute) on the board and their spread (not all
three in one corner, not symmetric so the board can be told apart from its rotation); for each fine-pitch part, look for
local fiducials near it. Only for boards the project marks as machine assembled.

**Fix.** Three global fiducials in an asymmetric pattern, 1 mm copper with a mask opening around them, and local ones at
fine-pitch parts; or agree with the assembler on rail fiducials.

**References.** Wikipedia, Fiducial marker "Printed circuit boards"
([en.wikipedia.org/wiki/Fiducial_marker](https://en.wikipedia.org/wiki/Fiducial_marker)); JLCPCB capabilities (1 mm
fiducials on the 5 mm tooling edges for its own assembly).

## `LAY-9` No tooling holes or panel rails planned <a id="lay-9"></a>

💡 suggestion · needs: holes and outline of the board, panel data or the project's assembly plan · [catalogue
entry](../checks.md#lay-9)

**Impl.** mid · depends on TOOL-3, CFG-2

**What it catches.** A board for machine assembly with no tooling holes and no plan for rails: parts close to all edges
and nothing in the project saying the fab adds the panel.

**Why it matters.** The assembly line transports and clamps the board by its edges; a board with parts within a few
millimetres of every edge needs rails added, otherwise it cannot go through the machine. Tooling holes locate the panel
in fixtures for printing, testing and depaneling. Finding this when the assembler sends back the order delays the build.

**Example.** A 40 x 25 mm board with connectors overhanging two edges and parts 1 mm from the others, ordered
"assembled" without a panel: the assembler adds 5 mm rails with mouse bites, which the connectors' overhang does not
allow on those edges, and the order waits for a new panel drawing.

**How to check.** Look for NPTH holes usable as tooling holes and for a panel (a separate panel board file or panel
settings in the project); where none, check whether parts sit closer to the edges than the assembler's clamping strip
and report it.

**Fix.** Plan the panel with the board: rails on the edges where parts allow it, tooling holes and fiducials on the
rails, or keep a clamping strip free on two opposite edges.

**References.** JLCPCB capabilities ("use 5 mm tooling edges, 2 mm tooling holes, and 1 mm fiducials" for its SMT
assembly).

## `LAY-10` Large pads with a full paste aperture <a id="lay-10"></a>

💡 suggestion · needs: pad sizes, paste layer apertures and margins per pad · [catalogue entry](../checks.md#lay-10)

**Impl.** mid · depends on TOOL-3

**What it catches.** Large pads (exposed pads, screw and shield pads, big SMD tabs) whose solder paste aperture covers
the whole pad instead of being split into a window-pane pattern.

**Why it matters.** A full aperture on a large pad prints far more solder than the joint needs. In reflow the excess
floats the part (opens on the small pins around an exposed pad), voids, and squeezes out to bridge neighbours. TI's
guidance for QFN exposed pads is 50 to 70 % paste coverage; a 1:1 aperture "results in excessive metal volume that can
float the part".

**Example.** B156A's screw pads printed with full paste wetted up and almost shorted to their neighbours (rev B: less
paste). On B135B the big pad of D604 needed its solder amount checked, and B153A's hub wanted a little more paste on its
pad: the paste on large pads is a choice to make per part, not a default.

**How to check.** For pads above an area threshold (a few mm²), read the paste layer: a paste margin of zero with no
custom paste shapes means a full aperture. Report them, with the exposed pads of QFN/SON parts first.

**Fix.** Window-pane apertures for 50 to 70 % coverage on exposed pads (per the part's datasheet when it gives one),
reduced and split apertures on screw and shield pads; no paste over thermal vias of 0.3 mm or larger unless the
datasheet says so.

**References.** SLUA271 §4.4 (exposed-pad stencil design); KLC F4.4 (no paste over thermal vias of 0.3 mm or larger);
B156 rev B and B135 rev C change lists.

## `LAY-11` Small parts too close for hand assembly <a id="lay-11"></a>

💡 suggestion · needs: part positions and sizes, the assembly method from the project · [catalogue
entry](../checks.md#lay-11)

**Impl.** hard · depends on TOOL-3, CFG-2

**What it catches.** 0402 and smaller parts placed closer together than a soldering iron or tweezers can work, on a
board where the project plans hand assembly (prototypes, small series, rework).

**Why it matters.** Machine placement is happy with 0.2 mm gaps; a person with an iron is not. Dense clusters on a
hand-built prototype take hours, bridge, and get damaged in rework. The extra space costs little on most boards.

**Example.** The B125A assembly notes: spread out 0402s for hand assembly where possible, and add it to the layout
guidelines.

**How to check.** For boards marked as hand-assembled, find pairs of small passives whose courtyard gap is below a set
distance (around 0.5 mm) and report clusters.

**Fix.** Spread the parts where the layout allows; use 0603 where density is not the constraint.

**References.** B125 rev B change list (0402 spacing for hand assembly).

## `LAY-12` Big THT pads into a plane without thermal relief <a id="lay-12"></a>

⚠️ warning · needs: zone connection settings per zone and pad, pad sizes · [catalogue entry](../checks.md#lay-12)

**Impl.** mid · depends on TOOL-3

**What it catches.** Large through-hole pads (connector pins, screw terminals, power THT parts) connected solidly to a
copper plane on one or more layers.

**Why it matters.** The plane draws heat out of the joint faster than the iron or the wave can put it in: the solder
does not wet the barrel, the joint is cold or needs so much heat that the part or the board is damaged. Thermal spokes
limit the heat flow while keeping the electrical connection.

**Example.** On B135B the DB-9 connector's pins needed thermal relief: soldering them into the plane was hard (rev C
change list).

**How to check.** For each THT pad above a size threshold and connected to a zone, read the effective zone connection
(zone setting, footprint or pad override); report "solid" connections, with the number of planes involved.

**Fix.** Thermal relief connections on THT pads; solid connection only for high-current or RF pads where it is needed,
with the extra soldering effort planned.

**References.** Wikipedia, Thermal relief
([en.wikipedia.org/wiki/Thermal_relief](https://en.wikipedia.org/wiki/Thermal_relief)); KiCad PCB "Working with zones";
B135 rev C change list (DB-9 thermal relief).

## `LAY-13` Track barely touching a via or pad <a id="lay-13"></a>

🛑 error · needs: track, via and pad geometry · [catalogue entry](../checks.md#lay-13)

**Impl.** hard · depends on TOOL-3

**What it catches.** A track that connects to a via or pad only by a sliver: its end overlaps the copper by a small
fraction of its width, so the connectivity check passes but the real connection is thin.

**Why it matters.** DRC asks only whether two copper items touch. A track end that clips the edge of a via ring by 0.02
mm passes, and after etch tolerance and drill wander the connection may be broken or a hairline that opens under current
or thermal cycling. It happens when a track is dragged and snaps to the wrong point, or when a via is moved after
routing.

**Example.** A 0.2 mm track ending at the edge of a via's 0.6 mm annular ring, overlapping it by 0.03 mm after the via
was nudged to make room: DRC clean, the board works on some units and not on others.

**How to check.** For each track end on a via or pad, compute the overlap of the track's end cap with the copper; report
overlaps below a fraction of the track width (for example under half), and track ends that touch without reaching the
pad or via centre region.

**Fix.** Re-route the track to the centre of the via or pad; teardrops add margin at the junction.

**References.** KiCad PCB "Teardrops"
([docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html#editing-teardrops)); IPC-2221B (annular
ring and conductor geometry), by title.

## `LAY-14` Creepage and clearance across high voltage and isolation barriers <a id="lay-14"></a>

🛑 error · needs: copper geometry, net voltages and the isolation requirement per barrier · [catalogue
entry](../checks.md#lay-14)

**Impl.** hard · depends on TOOL-3, CFG-2

**What it catches.** Copper on two sides of an isolation barrier (digital isolator, optocoupler, transformer, isolated
DC-DC) or between high-voltage nets closer than the required clearance (through air) or creepage (along the surface).

**Why it matters.** An isolated interface is only as isolated as its narrowest copper gap. A ground pour that creeps
under an isolator, a via placed in the barrier, or a trace routed across it reduces the creepage to whatever the layout
left, and the board fails its hipot test or, worse, passes it new and fails in a dirty, humid installation. The barrier
is also a safety feature on mains- or high-voltage-connected products.

**Example.** An isolated CAN interface (the B125 and B135 boards have them) with a digital isolator whose wide-body
package is specified with about 8 mm creepage pin to pin: a ground pour on the bus side extended under the package to
within 1.5 mm of a logic-side pad reduces the board's creepage to 1.5 mm, whatever the isolator is rated for.

**How to check.** Take the barrier from the parts that form it and the net sets on each side (the project gives the
required clearance and creepage per barrier, `CFG-2`); compute the minimum copper-to-copper distance between the sets on
each layer and along the surface (slots count). Report distances below the requirement.

**Fix.** A copper keep-out across the full barrier on all layers, a slot under the part where creepage is short, parts
with the right package rating; the required distances from the applicable safety standard.

**References.** IPC-2221B (conductor spacing by voltage); IEC 62368-1 (creepage and clearance for insulation), by title;
KiCad PCB "Rule areas (keepouts)".

## `LAY-15` Mounting holes without a decision <a id="lay-15"></a>

⚠️ warning · needs: mounting hole footprints, their nets, keep-outs around them · [catalogue entry](../checks.md#lay-15)

**Impl.** hard · depends on TOOL-3

**What it catches.** A mounting hole whose connection was never decided (plated but on no net, so neither grounded nor
isolated on purpose), or whose surroundings leave no room for the screw head and washer (copper, parts or tracks under
the head's footprint).

**Why it matters.** A screw connects the board to the chassis whether intended or not. A plated hole with copper around
it but no net is an unplanned antenna or an unplanned ground path; a track under the washer is shorted to the chassis
when the screw is tightened or cut through over time. The grounding decision (chassis ground, through an RC, or
isolated) belongs in the design, not to whoever assembles the box.

**Example.** An M3 screw (washer 7 mm in diameter) in a hole with a 6 mm keep-out: the washer overlaps a ground-adjacent
3.3 V track on the top layer; the solder mask is the only insulation and the screw's rotation scrapes it.

**How to check.** For each mounting hole, read its net and pad (plated or not) and check that copper and parts keep out
of a circle sized for the screw head and washer (from the hole size, M2.5, M3, M4); report holes without a net and
without an isolating keep-out.

**Fix.** Decide per hole: on the chassis or ground net with a pad for the washer, or NPTH with a keep-out sized for the
washer plus tolerance.

**References.** Ott chapter 3 (grounding) and chapter 16; KiCad PCB "Rule areas (keepouts)".

## `LAY-16` Trace width below the current it carries <a id="lay-16"></a>

⚠️ warning · needs: track widths per net and layer, copper weight, per-net current from the project · [catalogue
entry](../checks.md#lay-16)

**Impl.** hard · depends on TOOL-3, CFG-2

**What it catches.** A track on a power or high-current net whose width (for its copper weight and layer) is below what
the net's current needs for the allowed temperature rise.

**Why it matters.** Narrow tracks heat up, drop voltage and, at the extreme, fuse. The neck-down where a wide trace
meets a small pad or a via is the usual weak spot. The netlist knows nothing about current; the project data has to say
how much flows in each net.

**Example.** A 0.25 mm track on an outer layer of 35 µm (1 oz) copper carrying 2 A to a motor driver: the IPC-2221
formula for external conductors (I = 0.048 · ΔT^0.44 · A^0.725, A in mil²) gives a rise of about 65 °C; a 10 °C rise
needs about 0.8 mm. IPC-2152 refines these charts with the board's own heat spreading.

**How to check.** For each net with a current in the project (`CFG-2`), find the narrowest track segment and the number
of vias in series; compute the width needed per IPC-2152 for the layer and copper weight, and report shortfalls.

**Fix.** Wider tracks or pours, more vias in parallel where the current changes layers, thicker copper.

**References.** IPC-2152 (current carrying capacity), IPC-2221B (conductor sizing, the external-conductor formula
above), by title.

## `LAY-17` Decoupling capacitor far from its pin <a id="lay-17"></a>

⚠️ warning · needs: which capacitor serves which pin, positions and the routing between them · [catalogue
entry](../checks.md#lay-17)

**Impl.** hard · depends on TOOL-3, NET-7

**What it catches.** A decoupling capacitor placed or routed far from the supply pin it is meant for: centimetres away,
connected through a long thin track, or with its ground via far from the IC's ground.

**Why it matters.** A decoupling capacitor works by providing current with a small loop inductance; every millimetre of
track adds inductance and makes the capacitor useless at the frequencies it was meant for. A schematic with a 100 nF on
every pin and a layout with all of them in a row at the board edge is the classic result.

**Example.** An MCU with four VDD pins and four 100 nF capacitors placed together 20 mm away near the regulator: at 100
MHz the tracks' inductance dominates and the pins see almost no decoupling; MT-101 asks for the small capacitor "as
physically close to the power pins" as possible, with short connections to the ground plane.

**How to check.** Pair each decoupling capacitor with a supply pin (from the schematic grouping, `NET-7`, or the nearest
pin on the same net); measure the routed path from capacitor pad to pin and from the capacitor's ground pad to a ground
via; report pairs above a threshold.

**Fix.** Place the small capacitor next to its pin on the same side, power path through the capacitor pad to the pin, a
short via to ground at the capacitor.

**References.** MT-101 (local decoupling placement, Figure 6); Ott chapter 11.

## `LAY-18` No keep-out under the antenna or around the crystal <a id="lay-18"></a>

💡 suggestion · needs: antenna and crystal footprints, copper and parts around them · [catalogue
entry](../checks.md#lay-18)

**Impl.** hard · depends on TOOL-3

**What it catches.** Copper, tracks or parts under or near a PCB or chip antenna (or a module's antenna area), and
signals routed under or next to a crystal.

**Why it matters.** Copper near an antenna detunes it and takes most of the range. A crystal is a high-impedance,
low-level oscillator: fast signals near it couple into the oscillation (jitter, frequency shift, or failure to start),
and the crystal's own pins radiate if the loop is large.

**Example.** An ESP32 module with its PCB antenna over the base board's ground pour: range drops by a large factor.
Espressif recommends placing the antenna outside the base board or cutting the board away under it, and a clearance of
at least 15 mm in all directions inside the housing; for the crystal, a gap of at least 2.7 mm to the chip's clock pin
and no signal traces under it.

**How to check.** For antenna footprints (and module footprints with an antenna area), look for copper on any layer
inside the keep-out area; for crystals, look for tracks of other nets under the crystal and its load capacitors and
within a set distance of the crystal traces.

**Fix.** Keep-out rule areas on all layers under the antenna; a ground guard ring around the crystal and its capacitors,
nothing routed under them.

**References.** ESP32 HW guide "PCB Layout Design", sections "Crystal", "RF" and "General Principles of PCB Layout for
Modules"; AN2867 §7.1 (PCB design guidelines: guard ring, dedicated ground under the oscillator, no signals near it).

## `LAY-19` Assembly drawing without marking codes <a id="lay-19"></a>

💡 suggestion · needs: parts that look alike (same package, different MPN), their marking codes, the assembly drawing ·
[catalogue entry](../checks.md#lay-19)

**Impl.** judgement · depends on CFG-5

**What it catches.** An assembly drawing that does not give the top marking of parts that are hard to tell apart:
several SOT-23 or SOT-23-5 parts with different functions, diodes in one package with different ratings.

**Why it matters.** A SOT-23 carries a two- or three-letter code, not the part number. If three different SOT-23 ICs sit
next to each other, the person who checks the assembly (or reworks a board) cannot tell from the drawing which one
belongs where, and swapped parts survive inspection.

**Example.** B135B: the assembly drawing needed the marking codes of U301 (`LHV`), U302 (`LFAT`) and U402 (`AOT`) so the
board could be checked by eye (rev C change list).

**How to check.** better_erc can list candidates: parts that share a package but have different MPNs, with the marking
codes from the part database. Whether the drawing shows them is a document review.

**Fix.** Put the marking codes on the assembly drawing's fab layer next to the parts, or in a table on the drawing.

**References.** B135 rev C change list (marking codes on the assembly drawing); KLC F5.2 (fabrication layer content).

## `LAY-20` Pin-1 orientation differs between connectors of one type <a id="lay-20"></a>

💡 suggestion · needs: connector footprints and their rotation on the board · [catalogue entry](../checks.md#lay-20)

**Impl.** mid · depends on TOOL-3

**What it catches.** Several connectors of the same type on one board placed with different pin-1 orientations (one with
pin 1 left, the next with pin 1 right) without a reason.

**Why it matters.** People plug cables by habit: if the first header has pin 1 at the top left, they expect it on the
next one. Mixed orientations lead to reversed cables, and for power headers to damage. It also makes the harness drawing
harder to read.

**Example.** A board with four identical 6-pin sensor headers along one edge, three rotated 0° and one rotated 180° to
ease routing: the fourth sensor gets its cable reversed at the first installation.

**How to check.** Group connectors by footprint; compare their rotation and mirroring (side of the board); report the
odd ones out.

**Fix.** One orientation per connector type, or keyed connectors where routing forces a different orientation.

**References.** KLC F5.1 (pin-1 marks visible after mating); KiCad PCB "Placing and moving footprints".

## `LAY-21` Silk without revision, date code or serial blank <a id="lay-21"></a>

💡 suggestion · needs: silk text on the board · [catalogue entry](../checks.md#lay-21)

**Impl.** mid · depends on TOOL-3

**What it catches.** A board without its revision on the silk, without a place for a date code, and without a blank area
to write a serial number and the assembly revision.

**Why it matters.** Boards outlive their documentation. A board on the bench with no revision on it makes every bug
report ambiguous; a board with no field for a serial number cannot be traced to its test results or rework history. Rev
A, rev B and a reworked rev A look the same without it.

**Example.** A B-series board whose silk shows the project name only: two revisions in the lab, a rework on some rev A
units, and no way to tell which board a measurement came from without reading the BOM differences.

**How to check.** Search the silk text for a revision (`rev`, `${REVISION}` text variable, `A`/`B` after the board name)
and a date; look for an unfilled rectangle or a labelled field for the serial. Report what is missing.

**Fix.** Board name and revision on the silk (from the project's title block variables so they stay in sync), a date
code, and a blank rectangle for serial number and assembly revision.

**References.** KiCad PCB "Board setup" (text variables)
([docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html#board_setup)).

## `LAY-22` Copper or parts too close to the board edge or a V-score <a id="lay-22"></a>

⚠️ warning · needs: board outline, V-score lines, copper and part positions · [catalogue entry](../checks.md#lay-22)

**Impl.** hard · depends on TOOL-3

**What it catches.** Copper closer than 0.5 mm to the board edge or a V-score line, and parts closer than 3 mm (the
catalogue's limits).

**Why it matters.** Routing and scoring tolerance moves the edge; copper too close is exposed, cut, or shorted to the
next board in a panel. Depaneling bends the board along the score: ceramic capacitors near it crack (often only later,
as a short), and parts near any edge can be knocked off in handling or by the clamping strip.

**Example.** An 0805 MLCC 1 mm from a V-score: depaneling by bending cracks it; the crack does not show at test and
turns into a leakage path or short on the supply weeks later.

**How to check.** Compute the distance from copper and courtyards to the board outline and to V-score lines; report
items below the limits (the fab's own minimums, for example 0.2 mm from routed edges and 0.4 mm from V-cut edges at
JLCPCB, are a floor, not a target).

**Fix.** Pull copper and parts back from edges and scores; orient MLCCs parallel to the score line when they must be
near it; use routed tabs instead of V-scores near sensitive parts.

**References.** JLCPCB capabilities (outline: copper clearance from routed and V-cut edges); KiCad PCB "Design rules
checking" ("Board edge clearance violation").

## `LAY-23` Silk over pads, text too small, bottom text not mirrored <a id="lay-23"></a>

💡 suggestion · needs: silk items, pads and mask openings, text sizes and mirroring · [catalogue
entry](../checks.md#lay-23)

**Impl.** hard · depends on TOOL-3

**What it catches.** Silkscreen on pads or vias (clipped by the fab, or printed on solderable copper), text below the
fab's minimum height or line width, and bottom-side text that is not mirrored (and so reads backwards).

**Why it matters.** Silk on a pad hinders soldering; the fab clips it, so the reference designator or pin-1 mark the
designer counted on disappears. Text below the minimum becomes an illegible smear. Unmirrored bottom text is a sign the
item is on the wrong layer or was flipped by hand.

**Example.** Reference designators at 0.6 mm height with 0.1 mm lines, sent to a fab whose minimum is 1.0 mm text and
0.15 mm lines: the assembly house sees blurred blobs and works from the drawing only.

**How to check.** Check silk against pad and mask openings (overlap), text height and stroke width against the fab's
limits from the project, and the mirror flag of text on bottom layers.

**Fix.** Keep silk clear of pads (KLC asks for 0.2 mm), text at or above the fab's minimum, bottom text mirrored.

**References.** KLC F5.1 (silkscreen line width 0.10 to 0.15 mm, never over pads, 0.2 mm recommended clearance); JLCPCB
capabilities (legend: minimum line width 0.15 mm, minimum text height 1.0 mm, pad to silkscreen 0.15 mm).

## `LAY-24` Fab notes without stackup, impedance targets or IPC-356 netlist <a id="lay-24"></a>

💡 suggestion · needs: the fabrication drawing and outputs · [catalogue entry](../checks.md#lay-24)

**Impl.** judgement · depends on CFG-2

**What it catches.** A fabrication package without the stackup (layer order, thicknesses, materials), without the
controlled-impedance nets and their targets, or without an IPC-D-356 netlist for electrical test.

**Why it matters.** Without a stackup the fab picks its default, which may not give the impedances the design assumed.
Without impedance targets per net class the fab does not tune the line widths. Without an IPC-356 netlist the fab's
electrical test compares the board against its own Gerbers only, so an inverted inner layer (the files say one thing,
the stack was built in another order) or a missing connection that the Gerbers also miss passes test.

**Example.** A 4-layer board with USB (90 Ω differential) and Ethernet (100 Ω differential) ordered with Gerbers only:
the fab builds its standard stack, the actual impedance is 110 Ω, and an inner layer swap would go unnoticed at test
because there is no netlist to compare against.

**How to check.** Check that the fabrication outputs include a stackup table, an impedance table for the net classes
with impedance targets, and an IPC-D-356 file. Whether the notes say enough is a document review; better_erc can list
what is missing from the outputs it can see.

**Fix.** Add the stackup and impedance tables to the fab drawing (KiCad can place both as board items), export an
IPC-D-356 netlist, and ask the fab to confirm the stack before production.

**References.** IPC-D-356A (bare substrate electrical test data format), by title; KiCad PCB "Fabrication outputs and
plotting" and "Configuring board stackup and physical parameters".

## `LAY-25` Hot parts next to sensitive ones <a id="lay-25"></a>

💡 suggestion · needs: dissipation per part, positions, the sensitive parts (oscillators, references, ADCs, temperature
sensors) · [catalogue entry](../checks.md#lay-25)

**Impl.** hard · depends on TOOL-3, RPT-2, NET-7

**What it catches.** Parts that dissipate noticeable power (regulators, power resistors, FETs, LED drivers) placed next
to parts whose accuracy depends on temperature: crystals and oscillators, voltage references, ADC front ends,
temperature sensors.

**Why it matters.** Heat spreads through the board's copper. A temperature sensor next to an LDO measures the LDO; a
reference next to a power resistor drifts with the load; a crystal's frequency shifts with its temperature. Errors like
these show up as "the reading depends on what the board is doing".

**Example.** A TMP117 (±0.1 °C sensor) placed 3 mm from an LDO dissipating 0.3 W: the sensor reads the LDO's temperature
rise. The TMP117 datasheet warns that even the I2C pull-up resistor can be a heat source and asks for some distance
between it and the sensor.

**How to check.** Estimate dissipation per part (from currents and voltages in the project, `RPT-2`), mark the
temperature-sensitive parts (`NET-7` roles, part classes), and report pairs closer than a distance that scales with the
dissipation.

**Fix.** Move the sensitive part away, cut the copper between them (a slot or a thermal gap in the pour), or move the
hot part to the far side of the board.

**References.** TMP117 datasheet (SNOSD82D) §10.1 "Layout Guidelines"
([ti.com/lit/ds/symlink/tmp117](https://www.ti.com/lit/ds/symlink/tmp117.pdf)); TI SPRA953, *Semiconductor and IC
Package Thermal Metrics*, Rev. D, §1 (RθJA depends mostly on the PCB, Table 1-1)
([ti.com/lit/an/spra953](https://www.ti.com/lit/an/spra953/spra953.pdf)).
