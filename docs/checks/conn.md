# CONN checks, explained

Detail page for the `CONN` category of the [check catalogue](../checks.md#-conn---connector-protection-and-pinout). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs
to find it, how hard that is to implement, the usual fix, and where to read more. The one-line entries in the catalogue
stay the source of truth for ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to implement in better_erc without an LLM: `easy` is a graph query on
the netlist alone, `mid` needs part data or a project setting first, `hard` needs analysis the code does not have
yet, `judgement` needs a person (better_erc can at most list candidates). The IDs after it are the
[FEATURES.md](../../FEATURES.md) items the check depends on.

References used throughout:

- **SSZB130** · TI, *System-Level ESD Protection Guide*, rev. E (August 2025). §2.1 working voltage, §2.2
  polarity (uni- vs bidirectional), §3 devices per interface (§3.3 CAN, §3.13 RS-485).
  [ti.com/lit/sg/sszb130e](https://www.ti.com/lit/sg/sszb130e/sszb130e.pdf)
- **SLVA680** · TI, *ESD Protection Layout Guide*, rev. A (April 2022). §1 (the ESD path to ground, chassis
  connection at the connector), §2.2 (EMI from ESD onto adjacent traces), §2.4 (ground schemes).
  [ti.com/lit/an/slva680a](https://www.ti.com/lit/an/slva680a/slva680a.pdf)
- **DS13560** · ST, *STM32G0B1xB/xC/xE datasheet*, Rev 6 (February 2026), Table 22 (current characteristics:
  injected current per pin and in total). Our B153A MCU; fetched through `tpm fetch`.
  [st.com/resource/en/datasheet/stm32g0b1cb.pdf](https://www.st.com/resource/en/datasheet/stm32g0b1cb.pdf)
- **Cortex-M Debug Connectors** · Arm / Keil, overview of the 10-pin Cortex Debug, 20-pin Cortex Debug+ETM,
  legacy 20-pin IDC JTAG and 38-pin Mictor pinouts.
  [documentation-service.arm.com](https://documentation-service.arm.com/static/5fce6c49e167456a35b36af1)
- **J-Link interface** · SEGGER, *J-Link Interface Description* (20-pin JTAG and SWD pinouts, VTref).
  [segger.com](https://www.segger.com/products/debug-probes/j-link/technology/interface-description/)
- **TE, Specifying Current** · Tyco Electronics (AMP), *Specifying Current for the Real World*: contact current
  ratings, temperature rise, derating for loading density.
  [smtnet.com copy](https://smtnet.com/library/files/upload/Specifying-Current-for-The-Real-World.pdf)
- **RPi 4 schematics** · Raspberry Pi, *Raspberry Pi 4 Model B reduced schematics* (2019): the micro HDMI (type D)
  connectors J1/J13 with their pin names.
  [datasheets.raspberrypi.com](https://datasheets.raspberrypi.com/rpi4/raspberry-pi-4-reduced-schematics.pdf)
- **Wikipedia, HDMI** · connector types A to E and the type A pinout. [en.wikipedia.org/wiki/HDMI](https://en.wikipedia.org/wiki/HDMI)
- **Wikipedia, FFC** · *Flexible flat cable*, "Specifications": type A (contacts on the same side at both ends)
  and type D (opposite sides). [en.wikipedia.org/wiki/Flexible_flat_cable](https://en.wikipedia.org/wiki/Flexible_flat_cable)
- **Wikipedia, CAN bus** · "Layers", physical layer: the de facto DE-9 pinout (2 CAN-L, 3 GND, 7 CAN-H, 9 V+) and
  the 0 V rail along the bus. [en.wikipedia.org/wiki/CAN_bus](https://en.wikipedia.org/wiki/CAN_bus)
- **Ott** · Henry W. Ott, *Electromagnetic Compatibility Engineering* (Wiley, 2009): §2.15 shield terminations,
  §2.16 ribbon cables, chapter 15 (ESD: §15.5 preventing ESD entry, §15.7 ESD grounding), §16.2 PCB-to-chassis
  ground connection.
- **Johnson & Graham** · *High-Speed Digital Design: A Handbook of Black Magic* (Prentice Hall, 1993): chapter 9
  (connectors: §9.1 mutual inductance and crosstalk, §9.5 ground under a connector, §9.9 power handling), chapter
  10 (ribbon cables).
- **IEC 61000-4-2** · *Electromagnetic compatibility (EMC) - Part 4-2: Testing and measurement techniques -
  Electrostatic discharge immunity test* (by title).
- **IEC 62368-1** · *Audio/video, information and communication technology equipment - Part 1: Safety
  requirements* (by title; clearance and creepage).
- **IPC-2221** · *Generic Standard on Printed Board Design* (by title; conductor spacing by voltage).

## `CONN-1` No protection on lines going to connectors <a id="conn-1"></a>

⚠️ warning · needs: connector pins, protection parts on the net (TVS, ESD array, clamp) · [catalogue entry](../checks.md#conn-1)

**Impl.** mid · depends on CFG-2, NET-2

**What it catches.** A net that leaves the board through a connector pin with no protection part on it: no TVS,
no ESD array, no clamp diode pair, no series element that limits a fault.

**Why it matters.** A connector is where the outside world touches the board: a charged person or cable
(IEC 61000-4-2: 8 kV contact discharge at the top level), a cable plugged in live, a wrong cable, a short to a
neighbouring supply. On-chip ESD structures are rated for handling during assembly (a few hundred volts to 2 kV
HBM), not for a system-level strike; the strike either kills the pin outright or weakens it so it leaks and fails
months later. The board passes every bench test and fails in the field.

**Example.** An STM32 UART on a 4-pin JST to an external GPS module, TX and RX wired straight from the MCU to the
connector: the first touch of the cable end by a user in a dry office takes out the RX pin. On our B200A every line
of J102 has a 12 V TVS and a spark gap at the connector, a 255 Ω series resistor and a 5V6 Zener behind it; that
is what the check looks for.

**How to check.** For each connector pin, the net and the nets one series resistor or ferrite away
(`find_chains`): is there a part of a protection class (TVS, ESD array, Zener, steering diodes) with its other end
on a ground or rail? The class comes from the library or the MPN (`CFG-2` maps unknown parts); grounds, shields
and internal board-to-board links can be waived per connector.

**Fix.** A TVS or ESD array per line right at the connector, its ground short to the connector's ground or shield
(SLVA680); working voltage above the signal's normal range, capacitance within what the interface allows
(SSZB130 §3); a series resistor behind it for slow lines (`CONN-2`).

**References.** SSZB130 §1 (why external ESD protection), §2.1, §2.4 (IEC 61000-4-2 rating), §3 (devices per
interface); SLVA680 §1, §2.1; Ott chapter 15 (§15.5 preventing ESD entry); IEC 61000-4-2.

## `CONN-2` GPIOs straight to connectors without series protection <a id="conn-2"></a>

💡 suggestion · needs: connector pins, IC pins on the same net, series resistors · [catalogue entry](../checks.md#conn-2)

**Impl.** easy · depends on netlist

**What it catches.** An MCU or logic pin on the same net as a connector pin with no series resistor between them
(protection diodes alone do not count); on long or off-board lines a series resistor of about 1 kΩ, or a buffer, is
the usual minimum.

**Why it matters.** A TVS clamps fast transients, but it clamps at its breakdown voltage, above the pin's
supply: a sustained overvoltage (a short to a neighbouring 12 V pin, a miswired cable, the other board powered
first) then drives current straight into the pin's own clamp diodes. MCUs allow only a few milliamps of injected
current: the STM32G0B1 allows −5 mA per pin, no positive injection at all on its FT pins and 25 mA summed over
the chip (DS13560 Table 22). A series resistor turns a volt of overdrive into a small, legal current and also
slows the edges that leave the board.

**Example.** On B153A the USB-C plug pins A2 and B2 (TX1+, TX2+) go straight to PB13 and PB5 with an ESD array
only, no series resistor and no OVP. A damaged cable or a bit of debris that bridges them to VBUS (up to 20 V once
PD has negotiated) puts that voltage on the MCU pin with only the ESD array to stop it. On the same board the
header lines IO7-IO10 have 255 Ω, a 5V6 Zener on the MCU side and a 12 V TVS at the connector: a 12 V fault there
drives about 25 mA into the Zener, not into the MCU.

**How to check.** Pins of ICs (MCU, logic, transceiver logic side) on a net that also has a connector pin, with no
resistor, ferrite or buffer between them (`find_chains`). Pins of interfaces that cannot take a series resistor
(USB, Ethernet, high-speed lanes) are exempt by net class.

**Fix.** 100 Ω-1 kΩ in series near the connector, behind the TVS; size it from the worst fault voltage and the
pin's injection limit, and check it against the signal's speed and the receiver's input current. A buffer or a
translator with Ioff for lines that need drive.

**References.** DS13560 Table 22 (IINJ(PIN), ∑IINJ(PIN)); SSZB130 §2.1; Ott §15.6 (hardening sensitive
circuits); B153A hardware notes §5 (plug A2/B2).

## `CONN-3` Unipolar protector on a bipolar signal <a id="conn-3"></a>

🛑 error · needs: protector polarity (part data), signal range of the net · [catalogue entry](../checks.md#conn-3)

**Impl.** mid · depends on CFG-2

**What it catches.** A unidirectional TVS or ESD diode on a line whose normal signal goes below ground: CAN, RS-485,
RS-232, analog audio, a ±10 V analog input, an AC sense line.

**Why it matters.** A unidirectional diode is forward biased below about −0.7 V. On a line that swings negative in
normal operation it clamps every negative half: an RS-485 receiver must work from −7 V to +12 V common mode, CAN
sees ground offsets between nodes, RS-232 idles at −5 to −15 V. The result is a distorted or dead bus, or a diode
that carries the full driver current and burns.

**Example.** An SMF12A (unidirectional, 12 V) on each line of an RS-485 port: fine on the bench with both nodes on
one supply, but a remote node with a 3 V ground offset pulls the A line below −0.7 V on every low bit; the diode
conducts, the differential signal collapses and the bus sees framing errors. The bidirectional SMF12CA, or a
two-channel RS-485 or CAN ESD part (TI ESD2CAN24-Q1, ±24 V), is the right part.

**How to check.** Protection parts on interface nets whose range includes negative voltages: known from the
transceiver class (RS-485, CAN, RS-232 parts), a net class, or a project voltage range (`CFG-2`). Polarity from the
MPN (the `A` vs `CA` suffix of the SMx families) or the library.

**Fix.** A bidirectional part with a working voltage above the signal's range in both directions (SSZB130 §2.1:
"must not exceed the working voltage ... in either the negative or positive direction").

**References.** SSZB130 §2.1, §2.2, §3.3 (CAN: ±24 V parts), §3.13 (RS-485: "must be bidirectional", −7 V to 12 V);
Wikipedia, CAN bus "Layers" (common-mode range, the 0 V rail).

## `CONN-4` Connector without a ground pin <a id="conn-4"></a>

⚠️ warning · needs: connector pins, ground nets · [catalogue entry](../checks.md#conn-4)

**Impl.** easy · depends on NET-5

**What it catches.** A connector carrying signals or power with no pin on a ground net.

**Why it matters.** Every signal current needs a return path. Without a ground pin the return takes whatever path
is left: the shield, the mains earth of two power supplies, the USB cable of the laptop that is also plugged in.
The loop is large, the ground offset is undefined, and single-ended signals are read against a reference that
moves. Even differential buses need it: CAN and RS-485 receivers have a limited common-mode range, and the 0 V
wire is what keeps the nodes inside it.

**Example.** A CAN node with a 2-pin terminal for CAN_H and CAN_L only, on a machine where the nodes are powered
from different supplies: the common mode drifts by volts as motors start, and the node drops off the bus. The
de facto DE-9 pinout carries CAN_GND on pin 3 for this reason (our B135B uses DE-9 for CAN).

**How to check.** A connector (by designator class) whose pins carry nets but none of them a ground
(`NET-5`); connectors that are isolated by design (relay contacts, a photo-MOS output, an isolated sensor input) are
waived.

**Fix.** A ground pin next to the signals; for a differential bus the reference ground of the transceiver, or a
documented reason why the other end provides it.

**References.** Wikipedia, CAN bus "Layers" (DE-9 pin 3 GND, the 0 V rail); Ott chapter 3 (grounding, §3.4 ground
loops); Johnson & Graham §9.1 (return current through connector pins).

## `CONN-5` High speed on a connector: a ground for each pair <a id="conn-5"></a>

⚠️ warning · needs: high-speed nets, connector pin geometry · [catalogue entry](../checks.md#conn-5)

**Impl.** mid · depends on CFG-2, VIEW-10

**What it catches.** A high-speed differential pair or fast single-ended line on a connector or ribbon cable without
a ground pin next to it: no ground between neighbouring pairs, or between a fast clock and a sensitive line.

**Why it matters.** In a connector the return current of a signal flows on the nearest ground pin. With grounds
far away, the loop is large: it has inductance (ringing, ground bounce for all signals sharing that ground), it
couples into the neighbours (crosstalk scales with the shared loop area), and it radiates. Interleaving signals and
grounds is the cheapest fix there is and only possible while the pinout is still open.

**Example.** On B200A the 2×15 IDC header J102 puts a ground next to each of IO0-IO3 in the ribbon (pins 3/4,
5/6, 7/8, 9/10) but SCK on pin 16 runs between TX (15) and MOSI (17) with grounds two conductors away on 14 and 18.
At 20-30 MHz SPI over a 30 cm ribbon that is where crosstalk shows up first.

**How to check.** Nets marked as high speed (net classes, `_P`/`_N` pairs, interface classes, `CFG-2`) on a
connector; neighbours from the pin geometry (rows, pitch, ribbon order) of the connector type: is a ground pin
adjacent to each pair or fast line?

**Fix.** Signal-ground-signal or pair-ground-pair pinouts (G-S-S-G for differential); for ribbon cables every other
conductor ground for fast lines; a connector made for the data rate when the pin count runs out.

**References.** Johnson & Graham §9.1 (mutual inductance and crosstalk in connectors), §9.8 (differential
signalling through a connector), chapter 10 (§10.2 ribbon cable crosstalk); Ott §2.16 (ribbon cables).

## `CONN-6` Spare pins: spread power over more pins <a id="conn-6"></a>

💡 suggestion · needs: unconnected connector pins, pin current rating, currents on power nets · [catalogue entry](../checks.md#conn-6)

**Impl.** mid · depends on CFG-2

**What it catches.** A connector with unused pins while a power or ground net sits on one or two pins near (or
above) their current rating.

**Why it matters.** Contact resistance is milliohms per pin and grows with wear and corrosion; a pin near its
rating heats up, and the heat ages the contact further. A spare pin costs nothing at design time and halves the
current per contact. More ground pins also lower the return impedance for all signals (`CONN-10`).

**Example.** A 0.5 mm FPC to a display module, 0.5 A per contact in its datasheet: backlight supply 1 A on two
pins, its ground on two pins, and four pins left NC. Doubling both to four pins puts each contact at 0.25 A.

**How to check.** Connectors with unconnected pins and power or ground nets whose current (from the project,
`CFG-2`) per pin is above a fraction of the contact rating (part data).

**Fix.** Give the spare pins to the power and ground nets that carry the most current; keep them evenly spread so
the current shares (TE, Specifying Current).

**References.** TE, Specifying Current (contact temperature rise, loading density); Johnson & Graham §9.9 (power
handling features of connectors).

## `CONN-7` Reverse or flipped insertion: a pinout without power shorts <a id="conn-7"></a>

💡 suggestion · needs: connector geometry (rows, keying), nets per pin · [catalogue entry](../checks.md#conn-7)

**Impl.** mid · depends on VIEW-11, CFG-2

**What it catches.** A connector that can be mated the wrong way round (unkeyed pin headers, symmetric
housings, a cable turned over) where the reversed mapping puts a supply onto a signal or onto ground.

**Why it matters.** Unkeyed 2.54 mm headers, jumper wires and home-made cables get plugged in reversed sooner or
later. With a symmetric pinout the reversed plug is harmless; with a supply at one end and a logic pin at the
other the first wrong insertion drives the supply into the logic pin's protection, or shorts it to ground.

**Example.** B153A's 1×12 header J101: pin 10 is VBUS_SW (5 V, up to 20 V after PD) and pin 5 is +3V3. Reversed,
pin n lands on pin 13 − n: VBUS_SW reaches IO9 (pin 3) and 3.3 V reaches the USB D+ of pin 8. IO9's 255 Ω and
5V6 Zener then dissipate about 0.8 W at 20 V ((20 − 5.6)² / 255), more than a small-signal Zener survives. The
two grounds on 6 and 7 map onto each other, which is the right idea for the rest of the pinout too.

**How to check.** For each connector without a key, compute the reversed (and for dual rows, the row-swapped)
mapping from the connector geometry (`VIEW-11`), and report pairs where a power net lands on a signal or a
ground, or two different rails meet.

**Fix.** A keyed connector or a shrouded header; otherwise a pinout that is symmetric under reversal (ground in
the middle or at both ends, supply mirrored onto supply or onto a pin that tolerates it).

**References.** Ott §2.16 (ribbon cable pinouts with grounds); catalogue `CONN-18`, `CONN-17`.

## `CONN-8` MicroHDMI wired with the full size HDMI pinout <a id="conn-8"></a>

🛑 error · needs: connector type (HDMI type D), nets per pin · [catalogue entry](../checks.md#conn-8)

**Impl.** easy · depends on netlist

**What it catches.** A micro HDMI (type D) receptacle whose pins carry the signals in the order of the full size
type A connector.

**Why it matters.** Both have 19 pins, but in a different order: type A starts with TMDS data 2+ on pin 1 and ends
with +5 V on 18 and hot-plug detect on 19; type D starts with hot-plug detect on pin 1 and utility on 2, puts the
TMDS pairs with their shields on 3-14, CEC on 15, ground 16, SCL/SDA 17/18 and +5 V on 19. A symbol copied from a
type A design, or a type A symbol with a type D footprint, puts +5 V and DDC onto TMDS lanes: no picture, and the
5 V can damage the sink's TMDS inputs.

**Example.** Raspberry Pi 4's micro HDMI connectors J1/J13: pin 1 HPD, 2 UTILITY, 3 D2+, 4 D2 shield, 5 D2−, ...,
19 +5V. The same nets on a type A connector: 1 D2+, 2 D2 shield, 3 D2−, ..., 18 +5V, 19 HPD.

**How to check.** For a connector whose library or footprint says type D, the net on pin 1 and pin 19: HPD-like on
1 and +5 V on 19 is type D; a TMDS net on pin 1 or +5 V on 18 means the type A pinout.

**Fix.** The type D pin table from the connector's datasheet; a library symbol per connector type, not one HDMI
symbol for all.

**References.** Wikipedia, HDMI ("Connectors" for types A-E, the infobox for the type A pinout); RPi 4 schematics (micro HDMI J1/J13 pin names).

## `CONN-9` Bipolar protector on a unipolar signal <a id="conn-9"></a>

⚠️ warning · needs: protector polarity (part data), signal range of the net · [catalogue entry](../checks.md#conn-9)

**Impl.** mid · depends on CFG-2

**What it catches.** A bidirectional TVS on a line that never goes below ground in normal operation (logic, a
0-5 V sensor, a 5 V USB VBUS).

**Why it matters.** A unidirectional diode clamps negative transients at a forward drop, about −0.7 V; a
bidirectional one only at its reverse breakdown, −6 V or more for a 5 V part. Many inputs tolerate −0.3 V and a
few milliamps (`CONN-2`): a negative ESD or a reversed supply then reaches the protected pin at volts below ground.

**Example.** An SMF5V0CA (bidirectional) on a 5 V VBUS input to a buck converter whose VIN pin is rated −0.3 V:
a reversed bench supply or a negative transient is clamped only around −6 V and the converter dies; the SMF5V0A
(unidirectional) clamps the same event at a diode drop.

**How to check.** Bidirectional protectors (MPN suffix `CA`, library) on nets whose range (net class, rail,
logic level, `CFG-2`) is non-negative.

**Fix.** The unidirectional variant of the same family, working voltage above the signal's maximum.

**References.** SSZB130 §2.2 (polarity: a unidirectional diode "helps protect downstream devices that cannot
tolerate a negative voltage"); DS13560 Table 22 (negative injection limit).

## `CONN-10` Many pins and few grounds on a connector <a id="conn-10"></a>

⚠️ warning · needs: connector pins, ground nets · [catalogue entry](../checks.md#conn-10)

**Impl.** easy · depends on NET-5

**What it catches.** A connector with many signal pins and only one or two grounds, or all grounds bunched at one
end.

**Why it matters.** All return currents crowd onto the few ground pins: their shared inductance becomes common
ground impedance, so every edge on one signal shows up as noise on all others (ground bounce through the
connector), and the far signals have a large loop. It also leaves little current capacity on the return
(`CONN-6`).

**Example.** A 40-pin 0.5 mm FPC to an RGB parallel display with one ground pin, the pixel clock at 30 MHz on the
far end: the display shows sparkles on fast transitions. B200A's J102 has 9 grounds for 30 pins, spread across the
ribbon.

**How to check.** Count ground pins against the signal pins per connector (`NET-5`), and the largest distance from
a signal pin to the nearest ground in pin order; report ratios below about 1 ground per 4-6 signals for fast
interfaces.

**Fix.** More grounds, spread between the signal groups; for fast interfaces one per pair or per two to three
single-ended lines (`CONN-5`).

**References.** Johnson & Graham §9.1, §9.5 (continuity of ground underneath a connector), chapter 10; Ott §2.16.

## `CONN-11` Connector pin current rating below the current on that pin <a id="conn-11"></a>

⚠️ warning · needs: contact current rating (part data), current per net (project) · [catalogue entry](../checks.md#conn-11)

**Impl.** mid · depends on CFG-2

**What it catches.** A power or ground net whose current, divided over the pins that carry it, exceeds the contact
rating of the connector.

**Why it matters.** The rating is the current at which the contact rises a set temperature (usually 30 °C) above
ambient, often measured with a single contact in free air. In a housing full of loaded contacts, in a warm box,
and after years of mating cycles the usable current is lower. A pin above its rating heats, oxidises, its
resistance rises, and it heats more: a melted housing months after shipping.

**Example.** A 1.0 mm-pitch wire-to-board connector whose series datasheet rates 1 A per contact, carrying a 2 A
motor supply on one pin and its return on one more: both pins run at twice their rating.

**How to check.** Per connector, the current of each power and ground net (project data, `CFG-2`, or a regulator's
output rating as an upper bound) divided by its pin count, against the contact rating derated for the number of
loaded contacts and the ambient.

**Fix.** More pins in parallel (`CONN-6`), a connector series with higher-rated contacts, or a separate power
connector.

**References.** TE, Specifying Current (single-contact ratings, t-rise, loading density, end of design life);
Johnson & Graham §9.9.

## `CONN-12` Connector voltage rating below the net it carries <a id="conn-12"></a>

⚠️ warning · needs: connector voltage rating (part data), net voltages · [catalogue entry](../checks.md#conn-12)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** A connector whose rated voltage is below the highest voltage of any net on it.

**Why it matters.** Fine-pitch connectors are rated for low voltages because their contacts sit a fraction of a
millimetre apart: above the rating the insulation, creepage and clearance inside the housing are not qualified,
and with humidity, dust or a transient the gap can track or arc. It is also a safety question once the voltage
leaves the extra-low range.

**Example.** A 48 V PoE supply (a PSE output reaches 57 V) routed to a daughterboard over a 0.5 mm FPC connector
rated 50 V AC/DC.

**How to check.** Net voltages from the rail names (`NET-5`) or the project (`CFG-2`), the connector's rated
voltage from part data; flag nets above it.

**Fix.** A connector rated for the voltage, or the high-voltage net moved to its own connector.

**References.** The connector's datasheet (rated voltage); IEC 62368-1 (clearance and creepage); IPC-2221
(conductor spacing).

## `CONN-13` Mating pair pinout not mirrored <a id="conn-13"></a>

🛑 error · needs: both boards' netlists, the mating connector pair · [catalogue entry](../checks.md#conn-13)

**Impl.** hard · depends on NET-13, VIEW-10

**What it catches.** Two boards joined by a board-to-board plug and receptacle, or by an FPC, where pin n on one
side does not meet the net the other side expects on pin n: typically because both were drawn as seen from the
component side, or because one connector sits on the bottom of its board.

**Why it matters.** Each board's netlist is consistent on its own; the error exists only in the pair. Plug and
receptacle footprints mirror each other, and a connector placed on the bottom side mirrors again: a pinout copied
from one board to the other without thinking about which way the parts face lands every signal on its neighbour
row or its mirror pin. The boards are made, and the first mating shorts supply to ground.

**Example.** A mezzanine on a Hirose DF40-class plug and receptacle, the plug on the bottom of the top board:
the designer copied the receptacle's pin table from the base board onto the plug's symbol, so rows A and B are
swapped through the mated pair and 3.3 V meets a ground pin.

**How to check.** Join the netlists through the mating pair (`NET-13`) using the connector family's mating map
(pin n to pin n for most B2B pairs, with the side each part sits on from the PnP file) and compare net names or net
classes per mated pin.

**Fix.** Define the pinout once, as a signal table for the interface, and generate both symbols from it; check the
mated pair in a drawing with both parts in their real orientation.

**References.** The connector pair's datasheets (pin 1 position on plug and receptacle); Wikipedia, FFC
"Specifications" (type A/D cables for the FPC case, `CONN-14`).

## `CONN-14` FPC / FFC contact side and pin order vs the cable <a id="conn-14"></a>

🛑 error · needs: both ends' connectors (contact side), cable type, board orientation · [catalogue entry](../checks.md#conn-14)

**Impl.** hard · depends on NET-13, VIEW-10

**What it catches.** An FPC or FFC link where the contact side of each connector (top or bottom), the cable type
and the board orientation do not give the pin order the schematics assume.

**Why it matters.** A type A cable (Molex naming; Würth type 1) has its contacts on the same side at both ends, a
type D (type 2) on opposite sides. With two top-contact connectors facing each other on boards that face the same
way, a type A cable lands pin 1 on pin N: the order is reversed. Changing the cable type, a connector to
bottom-contact, or one board's orientation each flip it again. Get one of the three wrong and every signal is on
the wrong pin, often with power onto a signal.

**Example.** A camera board joined to a main board by a 15-pin 1 mm FFC, both connectors top-contact, both
boards face up: with a type A cable the camera's 3.3 V on pin 1 lands on the main board's pin 15. Ordering a
type D cable instead, or a bottom-contact connector on one side, puts pin 1 on pin 1.

**How to check.** For an FPC/FFC link between two netlists (`NET-13`): the contact side of each connector
(part data), the cable type and the side each board is mounted (project, `CFG-2`); derive the pin mapping and
compare nets per pin as in `CONN-13`.

**Fix.** Pick the cable type with the connectors, write it into both schematics and the BOM (the cable has an MPN
too), and draw the link once with both ends.

**References.** Wikipedia, FFC "Specifications" (type A / type D, type 1 / type 2); the connector datasheets
(contact position).

## `CONN-15` Debug header pinout against the standard it claims <a id="conn-15"></a>

⚠️ warning · needs: header nets, the standard's pin table · [catalogue entry](../checks.md#conn-15)

**Impl.** mid · depends on CFG-2

**What it catches.** A debug header (ARM 10-pin Cortex Debug, 20-pin Cortex Debug+ETM, legacy 20-pin IDC JTAG,
Tag-Connect) whose pins do not match the standard pinout, or a custom pinout on a footprint that looks like a
standard one.

**Why it matters.** The probe's cable assumes the standard: on the 10-pin Cortex Debug connector pin 1 is VCC
(the probe's VTref), 2 SWDIO/TMS, 4 SWCLK/TCK, 6 SWO/TDO, 8 TDI, 10 nRESET, 3 and 5 ground, 7 the key and 9
GNDDetect; on the legacy 20-pin IDC, 1 is VCC, 3 TRST, 5 TDI, 7 TMS/SWDIO, 9 TCK/SWCLK, 11 RTCK, 13 TDO/SWO,
15 nRESET and every even pin from 4 to 20 ground. A pinout off by one row puts the probe's ground on a signal, or
VTref on SWDIO; at best the probe sees "no target", at worst it drives a rail.

**Example.** B200A's SWD header J101 is our own 2×3 "Just Screw It" pinout (1 SWDIO, 3 SWCLK, 5 RST, 6 +3V3,
2 and 4 GND). Correct with our adapters (B198 J-Link to Tag-Connect is one of them), but read as the 10-pin Cortex
standard it would put the probe's VTref on SWDIO and the probe's SWDIO on a ground. The check should know that J101 is a declared custom pinout, not flag
it every run.

**How to check.** A header whose nets are SWDIO/SWCLK/TMS/TCK/TDO/nRESET-like; match its pin-to-net map against
the built-in tables of the standard pinouts, report the closest one and the differences; a pinout declared as
custom in the project (`CFG-2`) is accepted.

**Fix.** The standard pinout of the probe you use; if a custom one is needed, a different connector type so no
standard cable fits, and the pinout in silk.

**References.** Cortex-M Debug Connectors (10-pin, 20-pin and legacy pin tables); J-Link interface (20-pin JTAG and
SWD pinouts); catalogue `LIB-2`.

## `CONN-16` Connector shell or mounting pins left floating <a id="conn-16"></a>

⚠️ warning · needs: shell / mounting pins of connectors · [catalogue entry](../checks.md#conn-16)

**Impl.** easy · depends on netlist

**What it catches.** Shell, shield or mounting pins of a connector that are unconnected or on a net of their own,
with no recorded decision (ground, chassis, through R/C, or isolated on purpose).

**Why it matters.** A metal shell is the first thing an ESD strike or a cable shield touches; floating, it
discharges into whatever trace is closest instead of a defined path, and a cable shield ending on it shields
nothing. Some connectors also use the shell as part of a circuit: microSD sockets whose card-detect switch closes
against the shell.

**Example.** B125's change list for rev B: the microSD card detect goes from both side pins to the shell, not to
GND, so the shell has to be tied to ground for detect to read at all. USB and DE-9 shells (B135B's CAN shield)
need the same decision for ESD and shielding.

**How to check.** Connector pins named or typed as shield, shell, mounting or `MP` that are unconnected or alone on
their net; a waiver records "isolated on purpose".

**Fix.** Tie the shell to ground or chassis, directly or through the R/C network the design calls for
(`GND-7`, `USB-4`); put the decision in a schematic note.

**References.** Ott §2.15 (shield terminations), §15.7 (ESD grounding), §16.2 (PCB-to-chassis ground connection);
SLVA680 §1 (chassis connection next to the TVS and the connector shield).

## `CONN-17` Two identical connectors with incompatible pinouts <a id="conn-17"></a>

⚠️ warning · needs: connector types, nets per pin · [catalogue entry](../checks.md#conn-17)

**Impl.** easy · depends on NET-5

**What it catches.** Two or more connectors of the same type (same footprint or MPN) on one board whose pins put
power and ground in different places, so a cable or module meant for one fits the other and meets the wrong
voltages.

**Why it matters.** Users and assemblers go by the plug that fits. Two 4-pin JST connectors on a board, one for an
I2C sensor and one for a UART module, look alike in the enclosure; if one has 5 V on pin 1 and the other GND, the
first module plugged into the wrong one is back-powered through its ground or burns.

**Example.** A controller with two 4-pin JST-PH: `J5` sensor (1 GND, 2 3V3, 3 SDA, 4 SCL) and `J6` GPS
(1 5V, 2 TX, 3 RX, 4 GND). The sensor plugged into J6 gets 5 V on its ground pin.

**How to check.** Group connectors by footprint or MPN; within a group compare where the power and ground nets
sit (`NET-5`); flag groups where a power pin of one sits on a ground or a signal of another.

**Fix.** The same power and ground positions on every connector of a type, or different connector types (or
keyings, or colours) for different functions.

**References.** Catalogue `CONN-7`, `CONN-18`; Johnson & Graham §9.9.

## `CONN-18` Power and ground on adjacent pins <a id="conn-18"></a>

💡 suggestion · needs: connector pin geometry, power and ground nets · [catalogue entry](../checks.md#conn-18)

**Impl.** mid · depends on VIEW-11, NET-5

**What it catches.** A supply pin directly next to a ground pin, where a plug inserted at an angle or shifted by
one pin, a probe that slips, or a solder bridge shorts the supply.

**Why it matters.** A tilted insertion into an unshrouded header, or a jumper wire one pin off, shorts the two
nearest pins. Supply next to ground makes that short a dead short of the supply; a signal or a spare pin between
them turns it into a recoverable fault.

**Example.** B153A's J101: +3V3 on pin 5 right next to GND on pin 6. A jumper wire one pin off, or a probe slipping
while measuring 3.3 V, shorts the board's only logic rail. On B200A's J102, +5V0_OUT on 29 faces GND on 30; the
polyfuse limits it, but only after the rail has dipped.

**How to check.** From the connector geometry (`VIEW-11`), neighbours of each power pin in the same row and across
rows; report a ground (or another rail) next to a power net (`NET-5`).

**Fix.** A signal or a spare pin between supply and ground, a shrouded or keyed connector, or a fuse or current
limit on the supply.

**References.** Catalogue `CONN-7`; Ott §2.16 (pin assignment in ribbon cables).

## `CONN-19` High voltage and logic on adjacent pins <a id="conn-19"></a>

⚠️ warning · needs: net voltages, connector pin geometry · [catalogue entry](../checks.md#conn-19)

**Impl.** mid · depends on NET-5, VIEW-11

**What it catches.** A net well above the logic level (a battery, 12-48 V, a VBUS that can reach 20 V, a motor
or relay output) on a pin next to a logic or analog signal of the same connector.

**Why it matters.** Two failure modes: creepage and clearance inside the housing and on the board between the two
pads (the rating of `CONN-12` assumes a voltage between any neighbours), and the far more common wrong-pin fault,
where a shifted plug, a bent pin or a probe bridges the high voltage onto the logic pin, which survives at most
a few volts above its rail.

**Example.** B153A's J101: VBUS_SW on pin 10 (5 V, up to 20 V once the host negotiated PD) sits between USB D− on
pin 9 (hub port 3) and the relay output RL1 on pin 11. A probe slipping from pin 10 to pin 9 puts up to 20 V on
the hub's downstream D−.

**How to check.** Net voltages (`NET-5`, project) and pin neighbours from the connector geometry; report a pair
where one net is above a threshold (project, or 2× the other net's level) and the other is logic or analog.

**Fix.** A ground or a spare pin between them, the high-voltage net at the end of the row, or on its own connector;
creepage per the voltage (IEC 62368-1, IPC-2221) on the board under the connector.

**References.** IEC 62368-1; IPC-2221; DS13560 Table 22 (what a logic pin may take); catalogue `CONN-12`,
`CONN-18`.

## `CONN-20` Exposed metal with no ESD path <a id="conn-20"></a>

⚠️ warning · needs: parts with touchable metal and their frame pins, mechanical intent · [catalogue entry](../checks.md#conn-20)

**Impl.** judgement · depends on CFG-2

**What it catches.** Metal a user can touch (connector shells, button and switch frames, card housings, rotary
encoder and potentiometer shafts, display bezels) with no path to chassis or ground for an ESD strike.

**Why it matters.** A finger at 8 kV discharges into the first metal it reaches. With no path from that metal to
ground the discharge jumps across to the closest trace or pin inside, past all the TVS diodes that sit on the
connector lines. Shafts and frames are the classic hole in an otherwise protected product, because they are not
"connectors" in the schematic.

**Example.** An EC11 rotary encoder on a plastic front panel, its metal shaft and bushing touchable, the frame
pins left unconnected in the schematic: the IEC 61000-4-2 test on the knob resets the MCU through the encoder's
A/B lines. Grounding the frame pins (or the bushing to the panel's chassis) gives the strike its path.

**How to check.** better_erc can list candidates: parts with frame, shell, mount or shield pins (encoders, pots,
switches, card sockets, connectors) whose those pins are unconnected or not on a ground or chassis net; whether
the metal is touchable in the product is the designer's call, recorded per part (`CFG-2`).

**Fix.** Tie the frame or shell to chassis or ground, close to where the strike enters; for a shaft through an
insulated panel, a grounded bushing or a plastic knob.

**References.** Ott chapter 15 (§15.5 preventing ESD entry, §15.7 ESD grounding); SLVA680 §1, §2.4 (grounding
schemes for ESD); IEC 61000-4-2.
