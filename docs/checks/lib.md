# LIB checks, explained

Detail page for the `LIB` category of the [check catalogue](../checks.md#-lib---library-symbols-and-footprints). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs
to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for
ID, severity and wording; this page is the commentary.

The **Impl.** line rates how hard the check is to build in better_erc, in pure Rust and without an LLM: `easy` (the
netlist alone: connectivity, pin types, part classes and values), `mid` (part data or a project setting must be built or
read first), `hard` (analysis or layout geometry the code does not have yet), `judgement` (not a rule: a human or an LLM
reads intent or datasheet prose, better_erc can at most list candidates). The IDs after "depends on" are
[FEATURES.md](../../FEATURES.md) items.

References used throughout:

- **KLC** · KiCad Library Conventions, the rule set of the official KiCad libraries; rule numbers below (F4.4, S4.4,
  ...) are its own. Read from the source repository (klc.kicad.org sits behind a bot check).
  [gitlab.com/kicad/libraries/klc](https://gitlab.com/kicad/libraries/klc)
- **KiCad ERC** · KiCad 9 Schematic Editor manual, "List of ERC checks".
  [docs.kicad.org/9.0/en/eeschema](https://docs.kicad.org/9.0/en/eeschema/eeschema.html#list-of-erc-checks)
- **KiCad PCB** · KiCad 9 PCB Editor manual.
  [docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html)
- **IPC-7351B** · IPC, *Generic Requirements for Surface Mount Design and Land Pattern Standard*: land pattern sizes
  from the terminal dimensions and tolerances, courtyard excess.
- **SLUA271** · TI, *QFN and SON PCB Attachment*, Rev. C (2023).
  [ti.com/lit/an/slua271](https://www.ti.com/lit/an/slua271/slua271.pdf)
- **SLMA002** · TI, *PowerPAD Thermally Enhanced Package*, Rev. H (2018).
  [ti.com/lit/an/slma002](https://www.ti.com/lit/an/slma002/slma002.pdf)
- **Wikipedia, Footprint** · a short overview of footprints and land patterns.
  [en.wikipedia.org/wiki/Footprint_(electronics)](https://en.wikipedia.org/wiki/Footprint_%28electronics%29)

## `LIB-1` Pin names differ from the same part elsewhere <a id="lib-1"></a>

⚠️ warning · needs: a part database keyed by MPN, pin names per symbol · [catalogue entry](../checks.md#lib-1)

**Impl.** mid · depends on CFG-5, NET-7

**What it catches.** One part (same MPN or same library part) whose symbol names its pins differently from another
symbol of the same part in the database: `PG` in one project, `PGOOD` in the next, `PA13` here and `SWDIO` there.

**Why it matters.** Most checks find pins by name or role: the pull-up check looks for `SDA`, the reset checks for
`NRST`, the regulator checks for `EN` and `FB`. A symbol with its own private names silently drops out of those checks,
and a reviewer who knows the part from another board reads the wrong function into a pin. Two symbols for one part also
mean two places where a pinout error can hide, and they drift apart with every fix applied to only one.

**Example.** A TPS62130 drawn once from the KiCad library (pins `PG`, `FB`, `SS/TR`) and once by hand for an older
project (`PGOOD`, `VFB`, `SS`). The power-good pull-up check works on the first board and reports nothing on the second,
although both have the same missing pull-up.

**How to check.** Group the symbols of the netlist by MPN (or library part) against the database; compare pin number to
name per group and report the pins whose names differ. Normalise case and common separators first (`PA13/SWDIO` vs
`PA13`) so only real differences remain.

**Fix.** One symbol per part in the library, the vendor's pin names, alternate functions in the pin name or as KiCad
alternate pin functions; replace the private copies.

**References.** KLC S4.1 (general pin requirements), S4.2 (pins grouped by function); KiCad ERC "Symbol doesn't match
copy in library".

## `LIB-2` Connector pinout against its standard <a id="lib-2"></a>

⚠️ warning · needs: connector standard per footprint or field, a pinout table per standard · [catalogue
entry](../checks.md#lib-2)

**Impl.** mid · depends on CFG-5, CFG-2

**What it catches.** A connector that claims a standard (USB-C, CAN on DB-9, ARM 10-pin SWD, RJ45 Ethernet) whose nets
do not land on the pins that standard assigns.

**Why it matters.** Standard connectors mate with things the designer does not control: cables, programmers, other
vendors' boards. A swapped pair or a ground on the wrong pin is invisible in the schematic, passes every connectivity
check, and fails only when the first real cable is plugged in, sometimes by shorting a supply. Pinouts are also drawn
from "looking into the connector" or "from the solder side" views, which mirrors them (`LIB-18`).

**Example.** The B135 CAN board uses a DB-9 for the bus. CiA 303-1 puts CAN_L on pin 2, CAN_GND on pin 3 and CAN_H on
pin 7; a symbol drawn with CAN_H on pin 2 still passes ERC and every CAN check, and talks to nothing on a standard
cable. The same for a 10-pin SWD header with SWDIO and SWCLK exchanged (`CONN-15`).

**How to check.** Tag the connector with the standard it implements (footprint name, a field, or the project file); look
up the table, map each pin's net to the role it should carry (by net name and by what it connects to: a CAN
transceiver's CANH pin, a ground), and report pins whose net plays another role.

**Fix.** Rewire the symbol to the standard's table, or mark the connector as non-standard so nobody plugs a standard
cable into it.

**References.** CiA 303-1, *Cabling and connector pin assignment* (CAN in Automation), by title; Wikipedia, USB hardware
"Pin assignments" ([en.wikipedia.org/wiki/USB_hardware](https://en.wikipedia.org/wiki/USB_hardware)).

## `LIB-3` MOSFET pins not named G/D/S <a id="lib-3"></a>

⚠️ warning · needs: part class (MOSFET), pin names · [catalogue entry](../checks.md#lib-3)

**Impl.** easy · depends on netlist

**What it catches.** A transistor recognised as a MOSFET (by reference prefix `Q`, library part or value) whose pins are
not named `G`, `D` and `S` (or the obvious long forms), for example pins named `1`, `2`, `3`.

**Why it matters.** Many checks need to know which pin is which: gate drive and gate pull-down checks (`IO-3`),
reverse-polarity and load-switch checks, the body diode direction, Vds rating (`RATE-4`). With numbered pins the code
cannot tell gate from drain, so those checks either skip the part or guess. A numbered symbol also hides a wrong pin
mapping between symbol and footprint (`LIB-11`), since SOT-23 MOSFETs do not all share one pinout.

**Example.** A generic `Q_NMOS` symbol whose pins are `1`, `2`, `3`, used for a 2N7002 (SOT-23: 1 gate, 2 source, 3
drain). The gate pull-down check of the load switch next to it cannot find the gate and reports nothing.

**How to check.** Parts classed as MOSFETs (by library or description) whose pins are not named G/GATE, D/DRAIN
and S/SOURCE (`erc_checks/src/lib/lib_3.rs`, test schematic `lib-3_mosfet_pin_names`).

**Fix.** Use a symbol with named pins whose order matches the package, or rename the pins in the library symbol.

**References.** KLC S4.1 (general pin requirements); KLC S2.3 (a separate symbol per footprint pin order where packages
differ).

## `LIB-4` Wrong pin electrical type <a id="lib-4"></a>

⚠️ warning · needs: pin electrical types, pin roles from part data · [catalogue entry](../checks.md#lib-4)

**Impl.** mid · depends on NET-7, IMP-9, IMP-10

**What it catches.** A symbol pin whose electrical type does not match its function: a supply input typed passive, a
regulator output typed passive or input, an open-drain output typed push-pull output, an MCU GPIO typed input.

**Why it matters.** ERC and most of better_erc's checks reason over pin types. "Input power pin not driven" only fires
when the pin is typed power input; "two outputs on one net" only when both are typed output; an open-drain pin typed
output makes a wired-OR bus look like a conflict, and the reviewer starts ignoring the noise. A wrong type is therefore
a hole in every check that depends on it, not just a cosmetic error.

**Example.** An LDO symbol with its `OUT` pin typed passive: the rail it feeds has no power output on it, so every part
on that rail reports "power input not driven" and the designer adds `PWR_FLAG` symbols until it is quiet, which also
hides the real undriven rail next to it. The opposite case: a sensor's `INT` open-drain output typed output, shared with
a second sensor's `INT`: ERC reports an output conflict on a correct circuit.

**How to check.** Candidates from the netlist alone: pins whose names say power (`VDD`, `VCC`, `VIN`, `GND`, `VSS`) but
whose type is not power; outputs named `INT`, `ALERT`, `PG` typed push-pull. The full check compares the type with the
pin role from part data (`NET-7`, pin tables per `IMP-9`/`IMP-10`).

**Fix.** Correct the type in the library symbol: power input and power output for supply pins, open collector/open
emitter for open-drain outputs, bidirectional for programmable MCU IOs.

**References.** KLC S4.4 (pin electrical type); KiCad ERC "Input Power pin not driven by any Output Power pins" and the
pin conflicts map.

## `LIB-5` Pins missing or duplicated against the package <a id="lib-5"></a>

⚠️ warning · needs: symbol pins (library part), pin count from the footprint or package · [catalogue
entry](../checks.md#lib-5)

**Impl.** mid · depends on netlist, CFG-5

**What it catches.** A symbol whose pin numbers do not cover the package: pins of a QFN-32 numbered 1-31, an exposed pad
missing, a number used twice for two different functions, or numbers beyond the package's count.

**Why it matters.** A missing pin has no net: a ground or supply pin left out of the symbol stays unconnected on the
board, and nothing reports it because the schematic has no pin to leave unconnected. A duplicated number joins two
functions on one pad. Both usually come from a symbol drawn from the wrong package variant of the part (the TSSOP pinout
on a QFN footprint).

**Example.** A TPS62130 (VQFN-16 with exposed pad) drawn with pins 1-16 and no pin 17 for the pad, which the datasheet
requires on ground: the pad is unconnected on the board (`LIB-10`). Or a symbol with two pins numbered 7, one `VDD` and
one `NC`.

**How to check.** The KiCad netlist lists every pin of each library part (`libparts`), connected or not; compare the set
of numbers with the pin count decoded from the footprint name (`QFN-24-1EP_4x4mm`, `SOIC-8`) or from the part database.
Missing numbers, duplicates with different names, and numbers outside the range are findings; skipped numbers in
packages with removed pins (KLC F2.4, `MSOP-12-16`) are not.

**Fix.** Redraw the symbol from the datasheet's pin table for the package actually used; number the exposed pad one
above the pin count (KLC S3.7).

**References.** KLC S3.7 (pin numbering for exposed pads), S4.5 (omitted pins), F2.4 (footprints with removed pins);
KiCad ERC "Duplicate pins with different nets".

## `LIB-6` Footprint does not match the MPN's package <a id="lib-6"></a>

🛑 error · needs: package per MPN (part database or MPN decoding), the footprint name · [catalogue
entry](../checks.md#lib-6)

**Impl.** mid · depends on CFG-5

**What it catches.** A part whose footprint is not the package of the MPN in its BOM fields: a 4x4 mm inductor footprint
for a part that is 5x5 mm, a SOD-123 footprint for a SOD-323 diode, a 0603 pattern for an 0805 capacitor.

**Why it matters.** The board is made and the parts are bought, and only at assembly do they turn out not to fit. Small
mismatches still solder, badly (tombstoning, opens, a part sitting on one pad); large ones cannot be placed at all and
the batch needs rework or a respin. Neither ERC nor DRC can see it: the schematic and the layout agree with each other,
both are wrong against the real part.

**Example.** On B153A the buck inductor's footprint was "completely wrong" for the part in the BOM: found at assembly,
the board had to be reworked by hand and rev B fixes the footprint. The diode case: a BOM line for a SOD-323 Schottky
placed on a SOD-123 footprint, whose pads are farther apart than the SOD-323's terminals.

**How to check.** Decode the package from the MPN (resistor and capacitor part numbers encode the case size; for others
the part database holds it) and compare with the footprint name or, better, with the footprint's pad geometry. Report
mismatches; report unknown MPNs as not checked (`DIAG-2`).

**Fix.** Pick the footprint from the MPN's datasheet package drawing, or change the MPN to one in the footprint's
package. Keep the package in the library part so the two cannot drift.

**References.** KLC F4.1 (datasheet recommendations take priority); IPC-7351B; B153 rev B change list (inductor
footprint).

## `LIB-7` Pin name hides a shared special function <a id="lib-7"></a>

⚠️ warning · needs: the part's pin list with shared functions (stm32-data, vendor pin tables) · [catalogue
entry](../checks.md#lib-7)

**Impl.** mid · depends on NET-12, IMP-9

**What it catches.** A pin whose name shows only one of the functions it has when the others are not optional alternate
functions but are always there or there by default: a reset input, a boot strap, an oscillator pin, a debug pin after
reset.

**Why it matters.** The schematic is what the reviewer reads. If the symbol says `PF2`, nobody suspects that pulling the
pin low resets the MCU. The check `MUX-4` (shared functions in conflict) cannot work either if the name carries no hint
and no pin data is loaded. This class of bug passes every review and shows up at bring-up as "the board resets itself"
or "the hub never comes out of reset".

**Example.** On B153A the STM32G0B1's pin 10 is drawn as `PF2` and drives the USB hub's reset with a 10 kΩ pull-down;
but on this package the pin is `PF2-NRST`, the MCU's reset. The pull-down held the MCU in reset; rev A was reworked
(trace cut, hub reset moved to PC13) and the rev B symbol names the pin `PF2-NRST`.

**How to check.** For MCUs with known pin data (stm32-data for STM32, vendor tables per `IMP-9`), list the pins whose
function list contains a reset, boot, oscillator or default debug function, and check that the symbol's pin name
mentions it.

**Fix.** Name the pin with all its fixed functions (`PF2-NRST`, `PA14-BOOT0`), or use KiCad alternate pin functions so
the special one is visible.

**References.** B153 rev B change list ("PF2 is NRST", MCU symbol); KLC S4.1; KiCad 9 Schematic Editor manual,
"Alternate pin functions"
([docs.kicad.org/9.0/en/eeschema](https://docs.kicad.org/9.0/en/eeschema/eeschema.html#alternate_pin_functions)).

## `LIB-8` Pads too small or too large for the terminals <a id="lib-8"></a>

⚠️ warning · needs: pad geometry from the footprint, terminal dimensions from the package drawing · [catalogue
entry](../checks.md#lib-8)

**Impl.** hard · depends on TOOL-3, CFG-5

**What it catches.** A footprint whose pads do not fit the terminals of the part: too short to form a heel and toe
fillet, too narrow for the terminal, too long or too wide so neighbours bridge, or so asymmetric that surface tension
pulls the part up on one end.

**Why it matters.** The footprint is drawn once and reused for years; a marginal pattern costs yield on every batch. Too
small: poor wetting, cracked joints after thermal cycling, failed visual inspection. Too large on small passives:
tombstoning, because one end wets before the other. Too large between fine-pitch pins: bridges. It is found at assembly,
and often only as "a few percent of boards have a bad joint".

**Example.** On B153A the footprints of D801 and D802 had pads too small for the diodes' terminals and were listed as
wrong in the assembly notes; on B200A the long thin pins of a connector needed longer pads. The general case: a 0402
resistor on a pattern made for 0402 capacitors (taller terminals) with pads 0.2 mm longer than needed tombstones in
reflow.

**How to check.** Read the pads from the footprint (the `.kicad_pcb` reader of `TOOL-3` already parses footprints) and
compare them with the land pattern computed from the package's terminal dimensions and tolerances per IPC-7351B, or with
the vendor's recommended land pattern stored in the part database. Report pads outside a tolerance band.

**Fix.** Use the vendor's recommended land pattern, or one generated per IPC-7351B from the package drawing (KiCad's
footprint generators do this); check with the assembler for their stencil and process.

**References.** IPC-7351B; SLUA271 §3.2 (land pad design), §3.3 (lead finger pad PCB design); KLC F4.1, F6.3 (pad
requirements for SMD footprints); B153 rev B change list (D801, D802).

## `LIB-9` Footprint without a pin-1 or polarity mark <a id="lib-9"></a>

💡 suggestion · needs: footprint graphics (silk and fab layers) · [catalogue entry](../checks.md#lib-9)

**Impl.** hard · depends on TOOL-3

**What it catches.** A footprint of a polarized or orientation-sensitive part (IC, diode, LED, electrolytic, connector)
with no pin-1 or polarity mark on the silkscreen, or no bevel or arrow on the fabrication layer that assembly drawings
are made from.

**Why it matters.** Without a mark, the person or the inspection camera that checks orientation has nothing to compare
against: rotated ICs and reversed diodes pass inspection and the board is dead or burns at first power. Marks hidden
under the part body are just as useless after placement. Since the mark lives in the library footprint, every board that
uses the footprint inherits the gap; `LAY-6` checks the board, this check fixes it at the source.

**Example.** On B125A U3 and U12 had their pin-1 text on a mechanical layer only, so the assembled board showed nothing;
on B135B D201 and D601/D603 needed a clear and consistent pin-1 mark; on B153A the ISOM optocoupler had no pin-1 mark at
all, and the rev B change list asks for bigger marks with text across the board.

**How to check.** For footprints of polarized parts, look for silk graphics near pad 1 (or the cathode pad) outside the
part body, and a pin-1 feature on the fab layer. A heuristic, so it reports candidates for review.

**Fix.** Add the mark per KLC F5.1 (a filled chevron for SMD, a bar for diodes, `+` for polarized capacitors, placed so
it stays visible after assembly) and the bevel or arrow on the fab layer per F5.2.

**References.** KLC F5.1 (silkscreen: polarity marking / pin-1 designator), F5.2 (fabrication layer: pin-1 location);
B125 rev B, B135 rev C and B153 rev B change lists.

## `LIB-10` Exposed pad connected wrongly <a id="lib-10"></a>

🛑 error · needs: the exposed pad pin, its required connection from part data · [catalogue entry](../checks.md#lib-10)

**Impl.** mid · depends on NET-7, CFG-5

**What it catches.** An exposed pad (`EP`, `PAD`, the pin numbered one above the package's count) left unconnected
although the part needs it on ground or another net, or connected to ground although the datasheet asks to leave it
floating or tie it to another supply.

**Why it matters.** On many QFN and PowerPAD parts the exposed pad is a real electrical connection: the only ground, or
the power ground, or the substrate. Unconnected, the part does not work or works with ground through its bond wires
only, and overheats because the pad is also the thermal path. The opposite fault hurts too: on parts whose pad is the
negative supply or a separate substrate, a ground connection shorts that supply.

**Example.** The TPS62130 (3 A buck in a 16-pin VQFN) requires the exposed thermal pad to be connected to AGND, PGND and
the ground plane, and to be soldered for heat and mechanical strength. A symbol without pin 17 leaves the pad floating
on the board: the converter switches its 3 A with no proper ground return and runs hot. The other way round: the TMP117
precision temperature sensor asks not to solder its thermal pad at all on a rigid board, and if it is soldered, to leave
it floating or on ground, never on another net.

**How to check.** From the netlist: pins named like an exposed pad (or numbered above the package count) that are
unconnected are candidates (`easy` part). The rest needs part data: the required net of the pad per MPN (ground, a
supply, floating) compared with what it is connected to.

**Fix.** Give the symbol a pin for the pad (KLC S3.7 numbers it one above the pin count) and connect it as the datasheet
says; add thermal vias where the datasheet asks (KLC F4.4).

**References.** TPS6213x datasheet (SLVSAG7F) §6 "Pin Configuration and Functions"
([ti.com/lit/ds/symlink/tps62130](https://www.ti.com/lit/ds/symlink/tps62130.pdf)); TMP117 datasheet (SNOSD82D) §10.1
"Layout Guidelines" ([ti.com/lit/ds/symlink/tmp117](https://www.ti.com/lit/ds/symlink/tmp117.pdf)); SLUA271 §3.4
(exposed pad PCB design), §4.4 ("the exposed die must be soldered to the PCB"); SLMA002 §2; KLC S3.7, F4.4.

## `LIB-11` Pin to pad mapping wrong on a polarized part <a id="lib-11"></a>

🛑 error · needs: pin to pad mapping per symbol and footprint, the MPN's pinout · [catalogue entry](../checks.md#lib-11)

**Impl.** mid · depends on CFG-5, NET-7

**What it catches.** A diode, LED, transistor or polarized capacitor whose symbol pins map to the wrong footprint pads:
the schematic's cathode lands on the package's anode, the gate on the drain.

**Why it matters.** The schematic is right and the layout matches the schematic, so ERC, DRC and the schematic review
all pass, yet the part is mounted reversed by design. A reversed TVS or Schottky conducts at power-up; a reversed LED
stays dark; a reversed electrolytic vents. It happens most where pin numbering differs between packages of the same
function: diode symbols number cathode 1 in KiCad, but SOT-23 diodes put the cathode on pin 3.

**Example.** A BAT54 Schottky in SOT-23 (pin 1 anode, pin 3 cathode) drawn with KiCad's two-pin diode symbol, whose pin
1 is the cathode, on a three-pad SOT-23 footprint: pad 1 gets the cathode net and pad 2 (not connected inside the part)
the anode. The diode is out of circuit; in a reverse-polarity protection the board is unprotected. The B135B and B153A
change lists show how often the polarity question comes up at assembly (pin-1 marks, LED orientation `BOM-8`).

**How to check.** Compare the symbol's pin-to-pad mapping with the MPN's pinout from the part database (pad number of
anode and cathode, of G/D/S). Without part data, flag polarized two-pin symbols placed on footprints with more than two
pads, and symbols whose pin names (`A`, `K`) disagree with the footprint's pad names.

**Fix.** Use a symbol whose pin numbering matches the package (a three-pin diode symbol for SOT-23 diodes), or a
footprint with the matching pad numbering; keep the pairing in the library part so it is chosen once.

**References.** Diodes Inc., BAT54 datasheet DS11005 Rev. 34-2, page 1 (SOT23 pinout diagram)
([diodes.com](https://www.diodes.com/assets/Datasheets/ds11005.pdf)); KLC S2.3 (a separate symbol per footprint option),
S4.1; KiCad ERC "Assigned footprint doesn't match footprint filters" (catches some mismatches).

## `LIB-12` Footprint without a courtyard <a id="lib-12"></a>

💡 suggestion · needs: footprint graphics (courtyard layer) · [catalogue entry](../checks.md#lib-12)

**Impl.** mid · depends on TOOL-3

**What it catches.** A footprint with nothing on the courtyard layers (`F.CrtYd`/`B.CrtYd`), or an open outline that
does not enclose the part.

**Why it matters.** The courtyard is what keeps parts apart: KiCad's DRC checks overlapping courtyards, and the
assembler relies on the spacing for placement nozzles, rework and inspection. A footprint without one never triggers the
overlap check, so a part can be placed touching or under its neighbour and nobody notices until assembly. Imported
footprints (from vendors' download portals or older libraries) are the usual source.

**Example.** A footprint downloaded for a 5x5 mm inductor has copper and silk but no courtyard; a 0402 capacitor ends up
0.1 mm from the inductor body. DRC is clean, the pick-and-place nozzle hits the inductor.

**How to check.** For each footprint in the board file, look for courtyard graphics and check that they form a closed
outline. A presence check, no geometry analysis needed beyond that; `LAY-3` checks the size of the courtyard.

**Fix.** Add a closed courtyard per KLC F5.3: 0.25 mm around body and pads by default, 0.15 mm for parts under 1.5 mm,
0.5 mm for connectors, canned capacitors and crystals, 1.0 mm for BGAs.

**References.** KLC F5.3 (courtyard layer requirements); IPC-7351B (courtyard excess); KiCad PCB "Checking footprints"
and "Design rules checking".

## `LIB-13` Units of one part disagree <a id="lib-13"></a>

⚠️ warning · needs: per-unit fields and connections from the schematic · [catalogue entry](../checks.md#lib-13)

**Impl.** mid · depends on TOOL-3, IMP-1

**What it catches.** A multi-unit part (a quad op-amp, a hex inverter, a dual MOSFET) whose units carry different
footprints or values, or whose shared power pins are connected to different nets in different units.

**Why it matters.** The units are one physical part. If unit A says `TLV9064` and unit C says `TLV9062`, the BOM gets
one of them and half the schematic was designed for the other; if the footprint differs, the netlist exporter picks one
and the other unit's assumption is silently lost. Shared power pins on two nets short the nets through the part (or,
with hidden pins, connect two rails by name).

**Example.** A 74HC14 drawn with its power on every unit by hidden pins: units A to E on `+3V3`, unit F copied from
another sheet where the hidden pin's net was `+5V`. The netlist joins `+3V3` and `+5V` through the part's VCC pin.

**How to check.** Read the per-unit fields from the schematic (the `.kicad_sch` reader of `TOOL-3`) and compare
footprint and value across units; from the netlist, check that every shared pin has one net. KiCad's own ERC checks all
three ("Units of same symbol have different values", "Different footprint assigned in another unit of the symbol",
"Different net assigned to a shared pin in another unit of the symbol"); better_erc repeats them for netlists from other
tools.

**Fix.** Set fields on the symbol, not per unit; draw shared power pins in a separate power unit (KLC S3.8) instead of
hiding them in each unit.

**References.** KiCad ERC "Conflicts ERC checks"; KLC S3.8 (multi unit symbols), S4.6 (hidden pins).

## `LIB-14` Unit of a multi-unit part not placed <a id="lib-14"></a>

⚠️ warning · needs: units per part (library part), placed units · [catalogue entry](../checks.md#lib-14)

**Impl.** easy · depends on IMP-1

**What it catches.** A multi-unit part with one or more units never placed in the schematic, so their pins exist on the
board but connect to nothing.

**Why it matters.** An unplaced unit is not "unused", it is floating. CMOS gate inputs left open drift to mid-supply,
where both output transistors conduct: extra supply current, heat and oscillation that couples into the neighbours. An
op-amp channel with floating inputs can rail and oscillate, adding noise and current to the used channels of the same
package. ERC says nothing, because there is no pin in the schematic to complain about.

**Example.** A dual op-amp (TLV9062) with only unit A placed for a sensor buffer: unit B's inputs float on the board,
the output sits at one rail or oscillates, and the quiescent current of the package doubles. The fix is a placed unit B
wired as a buffer to a mid-rail or ground reference (`ANA-11`).

**How to check.** The KiCad netlist lists every pin of the library part and the nets of the placed ones; pins of a unit
that appear in no net at all, while other units of the same part are placed, are the finding. Parts whose unused units
are explicitly allowed (no-connect flags on a placed unit) are not.

**Fix.** Place every unit; tie unused gate inputs to a rail and unused op-amps as followers to a valid common-mode
voltage; or choose a part with fewer channels.

**References.** TI SCBA004, *Implications of Slow or Floating CMOS Inputs*, Rev. E
([ti.com/lit/an/scba004](https://www.ti.com/lit/an/scba004/scba004.pdf)); KLC S3.8.

## `LIB-15` Symbol without power pins <a id="lib-15"></a>

⚠️ warning · needs: pin types per symbol, part class · [catalogue entry](../checks.md#lib-15)

**Impl.** easy · depends on netlist

**What it catches.** An active part (IC, oscillator, module) whose symbol has no pin typed power input or power output,
and no pin whose name says power or ground.

**Why it matters.** Without power pins in the netlist, the part has no supply as far as any tool knows. Every power
check (rail present, voltage in range `RATE-11`, decoupling, power states) skips it, and if the symbol simply forgot the
pins, the part has no supply on the board either. Old symbol libraries and some vendors' symbols hide power pins and
connect them implicitly by name, which works until a project's ground is called `GNDD` instead of `GND`.

**Example.** A logic gate symbol drawn without its `VCC` and `GND` pins because "power is implicit": in this library
they are not hidden pins but simply absent, so the footprint's VCC and GND pads get no net and the gate is unpowered on
the board. DRC is clean, since the pads have no net to connect.

**How to check.** From the netlist: parts of active classes (U, IC, oscillators, modules; not passives, connectors, test
points or mechanical parts) whose pins include no power type and no power-like name. Hidden power pins that the netlist
resolves to a net count as power pins.

**Fix.** Draw the power pins, visible, in the symbol (or in a power unit for multi-unit parts, KLC S3.8); avoid hidden
power input pins outside power symbols (KLC S4.6).

**References.** KLC S4.6 (hidden pins: power input pins must not be invisible unless used in power symbols), S3.8; KiCad
9 Schematic Editor manual, "Hidden Power Pins"
([docs.kicad.org/9.0/en/eeschema](https://docs.kicad.org/9.0/en/eeschema/eeschema.html#hidden-power-pins)).

## `LIB-16` Pin without a pad, pad without a pin <a id="lib-16"></a>

🛑 error · needs: symbol pin numbers, footprint pad numbers · [catalogue entry](../checks.md#lib-16)

**Impl.** mid · depends on TOOL-3, IMP-1

**What it catches.** A mismatch between the symbol's pin numbers and the footprint's pad numbers: a pin whose number has
no pad (its net never reaches copper), a pad with no pin (it gets no net), or two pins on the same pad with different
nets.

**Why it matters.** A pin without a pad is the worst case: the schematic shows a connection that does not exist on the
board, and nothing downstream knows. A pad without a pin is often an exposed pad or a mounting tab that should be ground
and is left floating (`LIB-10`). Both typically come from a symbol and a footprint made for different packages or with
different numbering conventions (letters vs numbers for BGAs, `EP` vs `17` for exposed pads).

**Example.** A TLV702 LDO symbol for the 5-pin SOT-23 (1 IN, 2 GND, 3 EN, 4 NC, 5 OUT) placed on a 3-pad SOT-23
footprint: pins 4 and 5 have no pad, so the output net never reaches copper and the rail it feeds is unconnected on the
board. "Update PCB from schematic" warns about the missing pads, but only in a dialog nobody re-reads.

**How to check.** Read the pad numbers of each footprint (board file or library, `TOOL-3`) and the pin numbers of the
symbol (netlist `libparts`, `IMP-1`); report pins without pads, pads without pins that are not mechanical (`MP`, `SH`
per KLC S3.7), and duplicated pin numbers with different nets.

**Fix.** Pick the footprint that matches the symbol's package, or fix the numbering in one of them; give the exposed pad
a pin.

**References.** KLC S3.7 (pin numbering for specialized pads), S4.3 (pin stacking), F4.3 (connected copper elements
share a pad number); KiCad PCB "Update PCB From Schematic (forward annotation)"
([docs.kicad.org/9.0/en/pcbnew](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html#forward-annotation)).

## `LIB-17` Symbol or footprint link out of date <a id="lib-17"></a>

⚠️ warning · needs: the schematic's embedded symbols, the libraries, footprint filters · [catalogue
entry](../checks.md#lib-17)

**Impl.** mid · depends on TOOL-3

**What it catches.** A symbol in the schematic that is no longer the same as the symbol in its library (the library was
fixed, the schematic still has the old copy), a footprint field pointing to a footprint that does not exist, or a
footprint that the symbol's own footprint filter would not offer.

**Why it matters.** KiCad embeds a copy of each symbol in the schematic. When a library symbol is corrected (a pin type,
a pin number, the `PF2-NRST` name of `LIB-7`), older schematics keep the wrong copy until someone updates them. A broken
footprint link means the board was laid out with whatever footprint happened to be loaded, and the next update may swap
it; a footprint outside the filter is a strong hint that the package does not match (`LIB-6`).

**Example.** The B153 rev B change list fixes the MCU symbol's pin name in the library; any other board that placed the
old symbol keeps `PF2` until its symbols are updated from the library. ERC reports "Symbol doesn't match copy in
library" only when the library is installed on the reviewer's PC.

**How to check.** Compare the embedded symbol in the `.kicad_sch` with the library version, resolve each footprint link
against the footprint libraries, and match the footprint name against the symbol's filter patterns. KiCad's ERC has all
three ("Symbol doesn't match copy in library", "Footprint link issue", "Assigned footprint doesn't match footprint
filters"); better_erc runs them in batch and in CI where the GUI does not.

**Fix.** Update symbols from the library (and review the changes), fix the footprint link, or widen the filter when the
footprint is really right.

**References.** KiCad ERC "Miscellaneous ERC checks"; KLC S5.1 (default footprint links to a valid footprint), S5.2
(footprint filters).

## `LIB-18` Footprint mirrored from a bottom view <a id="lib-18"></a>

🛑 error · needs: the footprint's pad positions, the datasheet's package drawing and its view · [catalogue
entry](../checks.md#lib-18)

**Impl.** judgement · depends on TOOL-3

**What it catches.** A footprint whose pad arrangement is the mirror image of the real part, because its author read a
bottom-view (or solder-side, or mating-face) drawing as a top view.

**Why it matters.** The schematic, the netlist and the layout all agree, so no automatic check sees anything; the part
simply does not fit the way it is drawn, or fits rotated and connects its pins to the wrong nets. For a regulator that
means input and output swapped; for a connector, every signal on the wrong pin of the mating cable. It is found at
assembly or first power, and the fix is a respin.

**Example.** A TO-92 transistor footprint drawn from a pinout seen from below: E-B-C becomes C-B-E, and the 2N3904
(E-B-C) is placed reversed. Board connectors are the other classic: a D-sub's pin numbering seen from the mating face is
the mirror of the solder side, so a footprint drawn from the wrong view swaps pins 1 and 5 of a DB-9.

**How to check.** Not decidable from data the tool has: it needs reading the datasheet drawing and its view. better_erc
can list candidates: hand-made footprints (not from the official library) of transistors, regulators and connectors, and
footprints whose pad 1 position differs from the library footprint of the same package, for a human to check against the
drawing.

**Fix.** Redraw from the top-view land pattern (most datasheets give a "recommended land pattern, top view"); print the
layout 1:1 and place the real part on it before ordering.

**References.** KLC F4.1 (datasheet recommendations take priority), F4.2 (pin 1 at the top left); IPC-7351B.
