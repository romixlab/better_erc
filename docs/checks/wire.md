# WIRE checks, explained

Detail page for the `WIRE` category of the [check catalogue](../checks.md#-wire---connectivity-hygiene). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to
find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for
ID, severity and wording; this page is the commentary.

Most of these checks are about how a schematic is drawn rather than what it does: labels, flags, sheet pins and
buses. A netlist has already resolved all of that into nets, so several checks here read the schematic itself
(`.kicad_sch`, through the s-expression reader `berc textconv` uses, `TOOL-3`) instead of the netlist.

References used throughout:

- **KiCad** · KiCad, *Schematic Editor* manual, 10.0. ERC checks are named below as the manual lists them in "List of
  ERC checks" (`#list-of-erc-checks`), with the key that `.kicad_pro` uses in brackets.
  [docs.kicad.org/10.0/en/eeschema/eeschema.html](https://docs.kicad.org/10.0/en/eeschema/eeschema.html)
- **KiCad ERC keys** · KiCad source, `eeschema/erc/erc_item.cpp` (branch 10.0): the rule keys and texts; some
  exist only from 10.0 on (`ground_pin_not_ground`, `isolated_pin_label`).
  [gitlab.com/kicad/code/kicad](https://gitlab.com/kicad/code/kicad/-/raw/10.0/eeschema/erc/erc_item.cpp)
- **Altium ERC** · Altium, *Validating Your Design Project* (Altium Designer documentation): the same checks under
  Altium's names. [altium.com/documentation/altium-designer/schematic/design-validation](https://www.altium.com/documentation/altium-designer/schematic/design-validation)
- **SDYA009** · TI, *Designing With Logic*, SDYA009C (1997). [ti.com/lit/pdf/sdya009](https://www.ti.com/lit/pdf/sdya009)
- **SCBA004** · TI, *Implications of Slow or Floating CMOS Inputs*, SCBA004E (1994, rev. 2021).
  [ti.com/lit/pdf/scba004](https://www.ti.com/lit/pdf/scba004)

## `WIRE-1` Net with a single pin <a id="wire-1"></a>

⚠️ warning · needs: the netlist · [catalogue entry](../checks.md#wire-1)

**Impl.** easy · depends on netlist

**What it catches.** A named net that reaches exactly one pin: a label with no partner, or two labels meant to be one
net but spelled differently, so each half is a net of its own.

**Why it matters.** A label is a promise that the signal goes somewhere else. When the other end is missing or
misspelled, the schematic still looks connected on both sheets, and nothing on the PCB joins the two pins. The ratsnest
shows no airwire, so the layout is "complete" with the net open.

**Example.** `UART2_TX` on the MCU sheet and `UART_2_TX` on the connector sheet: two one-pin nets, and a debug UART that
never prints. Auto-named nets of unconnected pins (`unconnected-(U1-PA3)`) are one-pin nets on purpose and are not
reported.

**How to check.** Count the pins per net; report named nets with one pin, and point at `WIRE-2` when another net with a
near-identical name exists.

**Fix.** Rename one label to match, or delete it and mark the pin no-connect (`WIRE-8`).

**References.** KiCad "Label connected to only one pin" (`isolated_pin_label`, 10.0); Altium ERC "Nets with Only One
Pin".

## `WIRE-2` Net names that differ only in case, separator or suffix <a id="wire-2"></a>

🛑 error · needs: the netlist · [catalogue entry](../checks.md#wire-2)

**Impl.** easy · depends on netlist

**What it catches.** Two or more nets whose names become equal once case, `_`/`-`/`.` separators, a leading `+` and
polarity suffixes (`_N`, `_n`, `#`, `~`) are normalised: `RESET_N` and `RESET_n`, `3V3` and `+3V3`, `SPI.MOSI` and
`SPI_MOSI`.

**Why it matters.** EDA tools compare names exactly (KiCad is case-sensitive), so these are separate nets, and in
practice the designer almost always meant one. The usual source is a sheet copied from another project with its own
naming habits. It is silent: both nets look fine on their own, each with a few pins.

**Example.** A power sheet that produces `+3V3` and a sensor sheet copied from an older board that uses `3V3`: the
sensor's VDD and its decoupling cap sit on a net with no source, the sensor is dead, and ERC is clean because the `3V3`
power symbol counts as a driver.

**How to check.** Normalise every net name, group, and report groups with more than one original name; rails get the
extra rule that a voltage must match (`+3V3` and `3V3A` are not the same).

**Fix.** Pick one name and rename the others; for power, use one set of power symbols across the project.

**References.** KiCad "Labels are similar (lower/upper case difference only)" (`similar_labels`), "Power pin and label
are similar" (`similar_label_and_power`): KiCad compares case only, not separators, `+` or suffixes.

## `WIRE-3` Label not attached to a wire or pin <a id="wire-3"></a>

⚠️ warning · needs: the schematic (labels and wire geometry) · [catalogue entry](../checks.md#wire-3)

**Impl.** mid · depends on TOOL-3

**What it catches.** A net label whose connection point touches no wire end, wire segment or pin: placed beside the
wire, or one grid step off after a move.

**Why it matters.** A label that touches nothing names nothing. The wire it was meant for gets an auto name, the other
labels of the same name form a net without it, and the drawing still reads as connected. The netlist loses the label
entirely, which is why this needs the schematic.

**Example.** `CAN_STB` dragged together with a block and left 1.27 mm (half a grid) off its wire: the transceiver's STB
pin floats on an auto-named net, a TCAN1042 stays in standby (its STB has an internal pull-up), CAN is silent.

**How to check.** For every label in the schematic, test its anchor against wire segments and pin ends on the same
sheet; KiCad reports the same as a dangling label.

**Fix.** Snap the label onto the wire; keep the schematic grid at 1.27 mm (50 mil) so it cannot sit between points.

**References.** KiCad "Label not connected" (`label_dangling`), "Symbol pin or wire end off connection grid"
(`endpoint_off_grid`); Altium ERC "Floating net labels".

## `WIRE-4` Hierarchical sheet pin without a matching label <a id="wire-4"></a>

🛑 error · needs: the schematic (sheet pins and hierarchical labels) · [catalogue entry](../checks.md#wire-4)

**Impl.** mid · depends on TOOL-3

**What it catches.** A sheet pin on a hierarchical sheet symbol with no hierarchical label of the same name inside the
sheet, a hierarchical label inside with no sheet pin outside, or a sheet pin with no wire on the parent.

**Why it matters.** The sheet pin is the only path from the parent into the child sheet. If either side is missing or
unwired, the signal stops at the sheet border while both sheets look complete when read alone. It typically happens
after renaming a label inside the sheet without updating the sheet symbol.

**Example.** A motor driver sheet used twice (`MOTOR_L`, `MOTOR_R`) whose inner label was renamed from `FAULT` to
`nFAULT`: both sheet symbols still show `FAULT`, the MCU never sees either driver's fault output.

**How to check.** Match sheet pins against hierarchical labels per sheet by exact name; check each sheet pin has a wire
or label on the parent.

**Fix.** Re-import the sheet pins from the child (KiCad: "Import Sheet Pin") and wire them; delete stale ones.

**References.** KiCad "Mismatch between hierarchical labels and sheet pins" (`hier_label_mismatch`), sections
"Hierarchical sheet pins" and "Syncing sheet pins"; Altium ERC "Port not linked to parent sheet symbol", "Sheet Entry
not linked to child sheet".

## `WIRE-5` Power symbol and a signal label with the same name <a id="wire-5"></a>

🛑 error · needs: the schematic (power symbols, labels), the power structure · [catalogue entry](../checks.md#wire-5)

**Impl.** mid · depends on TOOL-3, NET-5

**What it catches.** A net that carries both a power symbol and an ordinary label of the same text, so a signal someone
named like a rail is in fact joined to the rail.

**Why it matters.** In KiCad a power symbol is a global label: every net that touches a `VBUS` power symbol is one net
project-wide. A designer who labels a sense line `VBUS` (meaning "the VBUS measurement") has just tied the ADC pin to
the 5 V rail, or worse, to 20 V after USB PD. The netlist shows one large net and nothing odd about it.

**Example.** `VBAT` as the label of the divider output into the ADC, next to the `VBAT` power symbol of the battery: the
ADC pin sits on the battery, 4.2 V into a 3.3 V MCU; the divider resistors are now shorted at one end (`SHORT-1`).

**How to check.** In the schematic, nets where a power symbol and a label share a name; in the netlist alone, a rail net
(`NET-5`) with a pin that is an output or an analog input of a signal role is the trace to follow.

**Fix.** Name the signal for what it is (`VBAT_SENSE`) and keep rail names for power symbols only.

**References.** KiCad sections "Power symbols" and "Net name assignment rules" (power symbols are global; global labels
outrank power symbols when one net has both); catalogue `SHORT-1`.

## `WIRE-6` Global label used on one sheet only <a id="wire-6"></a>

⚠️ warning · needs: the schematic (label scope, sheet) · [catalogue entry](../checks.md#wire-6)

**Impl.** mid · depends on TOOL-3

**What it catches.** A global label whose name appears on a single sheet and nowhere else in the project.

**Why it matters.** Global labels exist to cross sheets. One that does not is either a local label in the wrong scope
(harmless but misleading, and a collision waiting to happen when another sheet reuses the name), or the other end is
misspelled and the signal is open (`WIRE-1`, `WIRE-2`).

**Example.** `LED_STATUS` as a global label on the MCU sheet, `LED_STAT` on the front-panel sheet: each is alone, the
status LED never lights.

**How to check.** Group global labels by name and sheet; report names that occur on one sheet only, with the closest
name elsewhere.

**Fix.** Fix the spelling on the other sheet, or make it a local label if it is meant to stay on one sheet.

**References.** KiCad "Global label only appears once in the schematic" (`single_global_label`, off by default: severity
Ignore).

## `WIRE-7` No-connect flag on a pin that also has a wire <a id="wire-7"></a>

🛑 error · needs: the schematic (no-connect flags, wires) · [catalogue entry](../checks.md#wire-7)

**Impl.** mid · depends on TOOL-3

**What it catches.** A pin with a no-connect flag that is also connected to a wire, a label or another pin.

**Why it matters.** The flag says "deliberately unconnected", the wire says the opposite. One of the two is stale:
either the pin was later put to use and the flag forgotten (harmless to the copper, but it hides the next real open pin
from review), or the flag is right and the wire joins something that must not be there.

**Example.** An unused comparator output flagged no-connect, later wired to a spare GPIO for a test: the flag stays and
the review checklist "all NC pins are intended" passes over a pin that now drives a net.

**How to check.** No-connect flag positions against pin ends that also touch a wire or label; KiCad reports it too.

**Fix.** Delete the flag or the wire, whichever is stale.

**References.** KiCad "A pin with a "no connection" flag is connected" (`no_connect_connected`), section "No-connection
flags".

## `WIRE-8` Unconnected pin without a no-connect flag <a id="wire-8"></a>

⚠️ warning · needs: the schematic (no-connect flags), pin types · [catalogue entry](../checks.md#wire-8)

**Impl.** mid · depends on TOOL-3

**What it catches.** A pin connected to nothing that carries no no-connect flag.

**Why it matters.** The flag is the designer's signature: "I read this pin and decided". An open pin without it is
undecided, and a review cannot tell an intended spare from a forgotten input. Forgotten inputs float (`IO-1`), forgotten
supply and ground pins leave a part half-powered.

**Example.** A second VDD pin of an MCU in LQFP48 hidden on another unit of the symbol and never placed: the pin is
open, there is no flag, and the core runs from one supply pin until it brown-outs under load.

**How to check.** Pins on one-pin nets without a flag at their end in the schematic. The netlist alone sees the pin as
unconnected but cannot tell whether a flag was placed, so this reads the schematic.

**Fix.** Connect the pin, or flag it after checking the datasheet for `WIRE-9`.

**References.** KiCad "Pin not connected" (`pin_not_connected`), section "No-connection flags"; catalogue `IO-1`.

## `WIRE-9` No-connect flag on a pin that must be tied <a id="wire-9"></a>

🛑 error · needs: pin data ("must connect" pins, exposed pads), no-connect flags · [catalogue entry](../checks.md#wire-9)

**Impl.** mid · depends on NET-7, TOOL-3

**What it catches.** A no-connect flag on a pin that the datasheet requires to be connected: an unused CMOS input, a
mode or test pin that must be tied to GND, a second supply pin, an exposed pad that is the ground connection.

**Why it matters.** The flag makes ERC quiet, so the open pin passes every automated check. An unused logic input floats
and draws shoot-through current or oscillates; a TEST pin left open may enter a factory mode; an exposed pad that
carries ground (many QFN regulators and RF parts) leaves the part with no ground or no heat path.

**Example.** A 74LVC2G08 using one gate, the other gate's inputs flagged no-connect: they float, the unused gate toggles
with noise and adds milliamps of supply current. A DRV8833 with its exposed pad flagged: it is the ground and thermal
pad, the part overheats at a fraction of its rating.

**How to check.** Pins flagged no-connect whose pin data says "must connect" (unused inputs of logic, supply and ground
pins, EP of parts where the datasheet requires it); the pin data comes from the library or `NET-7`.

**Fix.** Tie unused inputs to GND or VCC (the datasheet's recommendation), tie mode pins as specified, solder the pad to
ground.

**References.** SDYA009 §3 "Unused Inputs"; SCBA004 §1 (floating CMOS inputs), §5 (pull-up or pull-down resistors); the
device datasheets (pin descriptions, "thermal pad" notes).

## `WIRE-10` Hidden power pins connected by name <a id="wire-10"></a>

⚠️ warning · needs: the schematic (symbol pins marked hidden) · [catalogue entry](../checks.md#wire-10)

**Impl.** mid · depends on TOOL-3

**What it catches.** Symbols with hidden power-input pins, which connect to the net of the same name without a wire on
the sheet (KiCad joins them as if a power symbol were there).

**Why it matters.** The connection exists but is invisible: a reader cannot see which rail a part runs from, and the pin
goes to whatever net has that name, which for an old 74-series symbol is `VCC` and `GND`. A 3.3 V design with a `+5V`
rail and a `VCC` net from some other sheet gets the gate powered from the wrong rail, or from a net nobody drives.

**Example.** A 74HC14 from an old library with hidden `VCC`/`GND` pins in a design whose 3.3 V rail is `+3V3`: the pins
join a `VCC` net that only exists through these hidden pins, with no source. The inverter is unpowered, and the
schematic shows nothing.

**How to check.** Symbols with pins of type power input that are marked hidden; report the net they joined and whether
it is a rail with a source.

**Fix.** Use symbols with visible power pins, or place a separate power unit and wire it.

**References.** KiCad section "Hidden power pins" ("not recommended in new designs"); Altium ERC "Mismatched Hidden Pin
Connections".

## `WIRE-11` Unnamed net with many pins or crossing sheets <a id="wire-11"></a>

💡 suggestion · needs: the netlist (net names, component sheet paths) · [catalogue entry](../checks.md#wire-11)

**Impl.** easy · depends on netlist

**What it catches.** An auto-named net (`Net-(R12-Pad2)`) with more than a few pins, or one that reaches parts on more
than one sheet.

**Why it matters.** Auto names change when the schematic changes, so the net cannot be found again in a review, a layout
comment, a test plan or a scope screenshot. A net that crosses sheets without a name is often an unintended join through
a hierarchical pin or a power symbol.

**Example.** The feedback node of a buck converter, `Net-(U4-FB)`, after a part renumbering becomes `Net-(R31-Pad1)`:
the bring-up note "probe Net-(U4-FB)" points at nothing.

**How to check.** Nets with auto-name patterns per tool and more than N pins (a project setting, 4 is a good default),
or pins on more than one sheet path.

**Fix.** Give the net a label with a name that says what it carries.

**References.** KiCad sections "Labels" and "Net name assignment rules".

## `WIRE-12` Same label text in different scopes <a id="wire-12"></a>

🛑 error · needs: the schematic (label scopes) · [catalogue entry](../checks.md#wire-12)

**Impl.** mid · depends on TOOL-3

**What it catches.** One label text used in two scopes: a local label and a global label, or a hierarchical sheet pin
and a global label, of the same name.

**Why it matters.** Whether such labels join is a tool rule, and the rules differ between tools and versions: a designer
from one tool assumes they are one net, the tool keeps them apart (or the other way round). Either the signal is open,
or two signals that were meant to be separate are shorted. Moving the design between tools changes the netlist with no
visible edit.

**Example.** A local `SDA` on a sensor sheet and a global `SDA` from the MCU sheet: in KiCad they are separate nets and
the sensor has no bus; renamed to a global label it joins, but so does the unrelated local `SDA` of a second, isolated
bus.

**How to check.** Collect label texts with their scope per sheet; report a text that appears with two scopes.

**Fix.** One scope per name: global for cross-sheet signals, local with a sheet-specific name for the rest.

**References.** KiCad "Local and global labels have the same name" (`same_local_global_label`), section "Net name
assignment rules"; Altium ERC "Nets with multiple names".

## `WIRE-13` Output pin whose net reaches no input <a id="wire-13"></a>

⚠️ warning · needs: pin types, signal following through series parts · [catalogue entry](../checks.md#wire-13)

**Impl.** easy · depends on netlist (NET-10 to look through series resistors)

**What it catches.** A net driven by an output pin with no input, bidirectional or passive load other than passives to a
rail, test points or a connector-free dead end.

**Why it matters.** An output that drives nothing is a function that was designed and then lost on the way: the consumer
was deleted, renamed (`WIRE-1`), or sits on another sheet behind a broken sheet pin (`WIRE-4`). Sometimes it is on
purpose (a spare kept for rework), and then the review should say so.

**Example.** On B200A the last shift register's serial output `EX4_QH_CHK` (U1404 QH') goes to TP1401 only: the MCU
cannot read the 32-bit chain back to verify it loaded. A deliberate choice there, and exactly the kind of net this check
lists for a decision.

**How to check.** For each output pin, follow the net through series resistors (`NET-10`) and list nets where no other
pin is an input, bidirectional or an IC pin of unknown type.

**Fix.** Connect the consumer, or note why the output is spare (and add a test point if it should be measurable).

**References.** KiCad sections "Pin electrical types" and "ERC violation severity and pin conflicts map"; catalogue
`WIRE-1`, `WIRE-4`.

## `WIRE-14` Ground-named pin on a net that is not a ground <a id="wire-14"></a>

🛑 error · needs: pin names, the power structure · [catalogue entry](../checks.md#wire-14)

**Impl.** easy · depends on NET-5

**What it catches.** A pin named GND, VSS, AGND, DGND, PGND or similar whose net is not one of the project's grounds.

**Why it matters.** Ground pins wired to a signal or a rail are nearly always a symbol whose pin numbers do not match
the part, a label typo, or a ground net with a non-ground name (`0V_ISO` not recognised). When the part has a real
negative supply on that pin, the check is a reminder to confirm it, then waive.

**Example.** A boost converter symbol drawn for the SOT-23-6 variant used on the SOT-563 footprint: the "GND" pin lands
on the SW net in the netlist and the part shorts its own switch node at power-up. The legitimate case: an LM358 run from
±12 V, whose pin 4 is named GND in the datasheet and is wired to `-12V`.

**How to check.** Ground-like pin names against the grounds of the power structure (`NET-5`).

**Fix.** Correct the symbol or the wiring; declare isolated grounds as grounds in the project; waive negative supplies.

**References.** KiCad "Ground pin not connected to ground net" (`ground_pin_not_ground`, new in 10.0); catalogue
`SHORT-5`.

## `WIRE-15` Four-way junction or a label over several wires <a id="wire-15"></a>

💡 suggestion · needs: the schematic (wire geometry, junctions) · [catalogue entry](../checks.md#wire-15)

**Impl.** mid · depends on TOOL-3

**What it catches.** A junction where four wires meet at one point, and a label whose anchor sits where several wires
meet so it is not clear which it names.

**Why it matters.** On paper and in PDF prints, a four-way junction with a small dot is easy to misread as two crossing
wires (and a crossing as a junction, when the dot is lost in printing). Reviewers then argue about a net the netlist is
already sure about. Offsetting the joins into two T-junctions makes the drawing say one thing.

**Example.** The ground net of an op-amp stage drawn as a cross with a 0.5 mm dot: in a printed review the dot
disappears and the reviewer marks "feedback not grounded".

**How to check.** Count wire ends per junction point; four or more is a finding. Labels whose anchor touches more than
one wire segment are reported with it.

**Fix.** Split into two T-junctions; move the label onto a single segment.

**References.** KiCad "Four connection points are joined together" (`four_way_junction`, off by default), "Label
connects more than one wire" (`label_multiple_wires`), section "Wire junctions".

## `WIRE-16` Bus mismatch <a id="wire-16"></a>

🛑 error · needs: the schematic (buses, bus entries, member names) · [catalogue entry](../checks.md#wire-16)

**Impl.** mid · depends on TOOL-3

**What it catches.** A net joined to a bus it is not a member of, buses of different widths connected together, and
members with indices outside the bus range (`D[0..7]` with an entry labelled `D8`).

**Why it matters.** A bus is drawing shorthand: what actually connects is each member by name. A member outside the bus
definition connects to nothing through it, and two buses of different widths joined together leave the extra members
open. The drawing shows a thick line going to the right place.

**Example.** A bus `A[0..15]` from the MCU sheet joined through a sheet pin to `A[0..16]` on the memory sheet: `A16` is
open on the SRAM side and floats, so the upper half of the memory appears at random.

**How to check.** Expand buses to members, compare names and widths at every bus connection and entry.

**Fix.** One bus definition per group, entries labelled with members that exist.

**References.** KiCad sections "Buses" and "Bus members"; checks "Net is graphically connected to a bus but not a bus
member" (`net_not_bus_member`), "Buses are graphically connected but share no bus members", "Invalid connection between
bus and net items" (`bus_to_net_conflict`).

## `WIRE-17` ERC or DRC rules silenced in the project file <a id="wire-17"></a>

💡 suggestion · needs: the project file · [catalogue entry](../checks.md#wire-17)

**Impl.** mid · depends on TOOL-3

**What it catches.** ERC or DRC rules whose severity the project lowered to "ignore" (or warning), and ERC exclusions,
listed in the report.

**Why it matters.** Silencing a rule is sometimes right (a library that has no power-flag habit), but it applies to the
whole project and to every later edit: a rule ignored once for one false positive hides every true positive after it. A
review should know which nets were never checked.

**Example.** `pin_not_connected` set to ignore in `.kicad_pro` to quiet a connector with many unused pins: the next
revision adds an MCU whose VDDA is left open, and KiCad says nothing.

**How to check.** Read the rule severities and exclusions from the project file (`berc textconv` already prints them per
section) and report the ones that differ from KiCad's defaults.

**Fix.** Restore the default severity and waive the individual case with an exclusion that names the item.

**References.** KiCad section "ERC violation severity and pin conflicts map"; KiCad source
`eeschema/erc/erc_settings.cpp` (the `erc` object of `.kicad_pro`: `rule_severities`, `erc_exclusions`, `pin_map`);
Altium ERC "Suppressing ERC Violations".
