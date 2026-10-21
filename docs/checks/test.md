# TEST checks, explained

Detail page for the `TEST` category of the [check catalogue](../checks.md#-test---test-points-and-testability). Each
section explains one check: what it catches, why the board is harder to bring up or test in production without it, a
concrete example, what better_erc needs to find it, the usual fix, and where to read more. The one-line entries in
the catalogue stay the source of truth for ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is for better_erc in pure Rust: `easy` (a query on the netlist), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet), `judgement` (intent, so
better_erc can at most list candidates).

References used throughout:

- **PCBCart TP** · PCBCart, *Test Point Design Guidelines for ICT & Flying Probe Testing* (pad diameter, spacing,
  clearance to tall parts). [pcbcart.com/article/content/test-point-design-ict-fp](https://www.pcbcart.com/article/content/test-point-design-ict-fp)
- **XJTAG DFT** · XJTAG, *Design for Test (DFT) Guidelines for improving JTAG testability* (testing through connectors,
  loopbacks). [xjtag.com/about-jtag/design-for-test-guidelines](https://www.xjtag.com/about-jtag/design-for-test-guidelines/)
- **Altium testpoint rules** · Altium, *Testpoint Rule Types* (fabrication and assembly test point style and usage
  rules, an example of what a layout tool can enforce).
  [altium.com/documentation/altium-designer/pcb/design-rule-types/testpoint](https://www.altium.com/documentation/altium-designer/pcb/design-rule-types/testpoint)
- **Wikipedia, Test point** and **Wikipedia, In-circuit testing** · readable overviews of test points, bed-of-nails
  and flying-probe testers. [en.wikipedia.org/wiki/Test_point](https://en.wikipedia.org/wiki/Test_point),
  [en.wikipedia.org/wiki/In-circuit_test](https://en.wikipedia.org/wiki/In-circuit_test)
- Our test plans: B200A `doc/test_plan_b200a.md` (loopback harness, test points per block), B135B `doc/test_plan.md`
  (signals per test point).

## `TEST-1` Unconnected pins without test points (optional report table) <a id="test-1"></a>

💡 suggestion · needs: unconnected pins (one-pin nets, no-connect flags), test points · [catalogue entry](../checks.md#test-1)

**Impl.** easy · depends on netlist

**What it catches.** Pins that connect to nothing and have no test point: a report table of outputs and spare pins
a reviewer may want to reach later.

**Why it matters.** An unconnected output is a signal nobody can observe once the board is built: the last stage of a
chain, a clock output, a status pin. A pad costs nothing at layout; adding one later means a wire on a fine-pitch pin.
The table is optional because most unconnected pins really are unused; the point is that someone decided.

**Example.** On B200A the serial output of the last 74HC595 in the 32-bit chain (`EX4_QH_CHK`) goes to TP1401 only:
the MCU cannot read it, but the test plan shifts a walking one through the chain and reads it back on TP1401 after
32 clocks. Without the test point the chain could only be checked through its effects on the outputs.

**How to check.** Pins on a net of their own (or with a no-connect flag) and no test point on that net; list them by
part and pin type, outputs first.

**Fix.** A test point on outputs and spare pins worth observing; an explicit no-connect flag on the rest (`WIRE-8`).

**References.** B200A test plan P3 (chain readback on TP1401); B200A schematic review §4 (shift-register chain).

## `TEST-2` Several test points on one net (incl. through 0R), unless top and bottom <a id="test-2"></a>

💡 suggestion · needs: test points per net, 0 Ω links, board side (PnP, `IMP-5`) · [catalogue entry](../checks.md#test-2)

**Impl.** easy · depends on netlist, `IMP-5`

**What it catches.** Two or more test points on the same net, counting nets joined by a 0 Ω resistor or a net tie,
unless they sit on opposite sides of the board.

**Why it matters.** Duplicate test points waste board area and fixture probes, and they often hide a copy-paste: two
pads labelled the same while a neighbouring net got none. One per side is legitimate: a bed-of-nails fixture probes
the bottom, a person at the bench the top.

**Example.** `TP3` and `TP7` both on `+3V3` (one copied with a power block from another sheet), while `+1V8` has no
test point at all (`TEST-4`).

**How to check.** Group test points by net, merging nets joined by 0 Ω parts; more than one per group and side is a
finding. The side comes from the PnP file (`IMP-5`) or the footprint layer.

**Fix.** Keep one per side; move the spare to a net that has none.

**References.** Wikipedia, In-circuit testing (bed of nails probes from one side); PCBCart TP.

## `TEST-3` All harness connector pins self-testable <a id="test-3"></a>

💡 suggestion · needs: harness connectors (project), what each pin reaches (`NET-10`), MCU pin functions (`NET-12`) · [catalogue entry](../checks.md#test-3)

**Impl.** hard · depends on `NET-10`, `NET-12`, `CFG-2`

**What it catches.** Pins of a harness or field connector that the board itself cannot drive or read: no path from
the pin to an MCU GPIO, ADC input or transceiver, so a loopback plug cannot test it.

**Why it matters.** Connector pins are where boards fail: bent contacts, cracked solder joints, ESD-damaged
protection. If every pin reaches something the firmware can drive or read, a passive loopback plug turns production
test into a firmware run, and field units can self-test. A pin the board cannot see needs a fixture with
instruments, or is never tested.

**Example.** B200A's test plan uses a loopback harness on J102: TX to RX, RTS to CTS, MOSI to MISO, IO0 to IO4 and so
on, each pair driven at every level and read back by the partner. Pins that only feed a passive divider or a
connector-to-connector pass-through would fall out of that test.

**How to check.** Mark harness connectors in the project (`CFG-2`), then follow each pin through series resistors,
protection, translators and buffers (`NET-10`) to a pin the MCU can use as input or output (`NET-12`); report pins
with no such path.

**Fix.** Route an observing path (a GPIO through a resistor, an ADC divider) to pins that lack one, or pair them in
the pinout so a loopback plug joins an output to an input.

**References.** XJTAG DFT "Extend 1149.x testing through connectors" (loopbacks for signals the tester cannot
reach); B200A test plan §C (bench and loopback harness), P5 GIO-10.

## `TEST-4` Power rail or ground without a test point <a id="test-4"></a>

💡 suggestion · needs: rails and grounds (`NET-5`), test points · [catalogue entry](../checks.md#test-4)

**Impl.** easy · depends on `NET-5`

**What it catches.** A power rail, or a ground net, with no test point.

**Why it matters.** The first bring-up step is measuring every rail; on a dense board the only other places to touch
are capacitor ends the size of a grain of sand. A ground test point next to the rail ones (or a loop for the scope
clip) is just as important: it makes the measurement possible with one hand. Production test measures rails on
every unit.

**Example.** B153A and B200A both had a "1.8 V" LDO that was really a TLV70233 (3.3 V); both boards have a test point
on `+1V8_LDO` (TP801, TP1605), so the wrong voltage is one probe touch away, not a search for a capacitor pad.

**How to check.** For each rail and ground of the power structure, look for a test point on the net (or on a net
joined to it by a 0 Ω); report those without.

**Fix.** A test point per rail and at least one ground pad (or a ground loop) per board area.

**References.** B153A schematic review §2 (TP801-TP803); B200A schematic review §2 (TP1601-TP1605); PCBCart TP.

## `TEST-5` Bring-up bus (UART, I2C, SPI, SWD) without test points <a id="test-5"></a>

💡 suggestion · needs: bus nets (by pin names, `NET-4`, `NET-12`), test points · [catalogue entry](../checks.md#test-5)

**Impl.** easy · depends on `NET-4` (`NET-12` for pins named by port only)

**What it catches.** The lines of a bus used at bring-up (a console UART, I2C, SPI, SWD) with no test point and no
header pin.

**Why it matters.** These are the lines one probes in the first hours: is the console sending, does the sensor ACK,
does the flash answer. With test points a logic analyser or a probe connects without soldering; on a production
fixture the same pads flash the firmware (SWD) and read the self-test result (UART). `DBG-18` is the same question
from the debug side.

**Example.** B153A brings SWDIO and SWCLK to TP201 and TP202 besides the receptacle SBU pins; its rev B change list
adds silk labels on those test points, because unlabelled pads were hard to find (`LAY-7`).

**How to check.** For each bus line (I2C from `NET-4`, UART/SPI/SWD by pin names), a test point or connector pin on
the net.

**Fix.** A test point per line plus a nearby ground; on SWD, also NRST (`DBG-17`).

**References.** B153 rev B change list (silk on the test points); B153A schematic review §4 (SWD on TP201, TP202).

## `TEST-6` Clock, reset or enable net without a test point <a id="test-6"></a>

💡 suggestion · needs: clock, reset and enable pins (by name or role, `NET-7`), test points · [catalogue entry](../checks.md#test-6)

**Impl.** mid · depends on `NET-7`

**What it catches.** Clock nets (crystal outputs, clock buffers, MCO), reset nets and enable or power-good nets with
no test point.

**Why it matters.** "Does the oscillator run, is reset released, is the regulator enabled, is power-good high" are
the questions of a board that does nothing; each is one probe touch with a test point and a hunt without one. A
fixture can also hold a part in reset through the reset test point.

**Example.** B135B's test plan measures XOUT on TP305 with a frequency counter and triggers a reset through TP209 on
`~H7_RST` to check that the firmware reboots; B200A puts the buck's power-good on TP1602.

**How to check.** Nets on pins of clock, reset or enable role (names like `XOUT`, `CLKOUT`, `MCO`, `RST`, `EN`, `PG`,
or pin roles from `NET-7`) with no test point.

**Fix.** A test point on each; for crystals, on the output side through a small series resistor or on a clock
output the MCU can enable, so the probe's capacitance does not stop the oscillator.

**References.** B135B test plan (TP305 XOUT, TP209 reset); B200A schematic review §2 (TP1602 PG).

## `TEST-7` Test points too small or too close for the probe or fixture they are meant for (layout) <a id="test-7"></a>

💡 suggestion · needs: layout geometry (pad size, spacing, clearance to tall parts), fixture type from the project · [catalogue entry](../checks.md#test-7)

**Impl.** hard · depends on a layout import (none yet), `CFG-2`

**What it catches.** Test pads smaller than the probe needs, closer to each other than the probe pitch allows,
under solder mask, or next to tall parts the probe would hit.

**Why it matters.** A test point the fixture cannot hit reliably is worse than none: it gives intermittent failures
on good boards. Spring probes on a bed of nails need pad size and pitch matched to the probe type; flying probes
allow less but still have limits. Hand probing at the bench needs a pad large enough not to slip onto the
neighbour.

**Example.** 0.6 mm test pads at 1.27 mm pitch next to a 6 mm tall electrolytic on a board meant for a bed-of-nails
fixture with standard 2.54 mm probes: half the pads need the expensive fine-pitch probes and two cannot be reached.

**How to check.** From the layout: pad diameter, centre-to-centre spacing, mask opening, distance to parts taller than
a limit; compare with the fixture class set in the project. PCBCart's guideline is a usable default: 1.0 mm pads
preferred (0.9 mm acceptable), 2.54 mm spacing, clear of parts taller than about 5 mm.

**Fix.** Bigger pads, more spacing, test points away from tall parts, mask open; agree the fixture type with the
assembler before layout.

**References.** PCBCart TP (pad diameter, spacing, tall-part clearance); Altium testpoint rules; Wikipedia, In-circuit
testing (bed of nails and flying probe).
