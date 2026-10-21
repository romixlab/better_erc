# DRV checks, explained

Detail page for the `DRV` category of the [check catalogue](../checks.md#drv-1). Each section explains one check:
what it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the usual
fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID, severity and
wording; this page is the commentary.

Most checks here need two facts the netlist does not carry: the transistor's type and ratings (N or P channel,
VGS(th), VGS(max), RDS(on) and the gate voltage it is specified at), and the voltage that drives it. The first comes
from the MPN or the library, the second from the power structure and the pin model.

References used throughout:

- **SLUA618** · TI (L. Balogh), *Fundamentals of MOSFET and IGBT Gate Driver Circuits*, SLUA618A (2017, rev. 2018).
  [ti.com/lit/an/slua618a](https://www.ti.com/lit/an/slua618a/slua618a.pdf)
- **SLUA887** · TI, *Bootstrap Circuitry Selection for Half-Bridge Configurations*, SLUA887A (2018, rev. 2023).
  [ti.com/lit/an/slua887](https://www.ti.com/lit/an/slua887/slua887.pdf)
- **AN11158** · Nexperia, *Understanding power MOSFET data sheet parameters*, Rev. 7.0 (2025; fetched through
  `tpm fetch`). [assets.nexperia.com/documents/application-note/AN11158.pdf](https://assets.nexperia.com/documents/application-note/AN11158.pdf)
- **AN11160** · Nexperia, *Designing RC snubbers*, Rev. 3.1 (2024; fetched through `tpm fetch`).
  [assets.nexperia.com/documents/application-note/AN11160.pdf](https://assets.nexperia.com/documents/application-note/AN11160.pdf)
- **Toshiba gate drive** · Toshiba, *MOSFET Gate Drive Circuit*, application note (2018; fetched through
  `tpm fetch`). [toshiba.semicon-storage.com](https://toshiba.semicon-storage.com/info/docget.jsp?did=59460)
- **AN2007-04** · Infineon, *How to calculate and minimize the dead time requirement for IGBTs properly*, AN2007-04
  V1.0 (2007; Mouser's copy of the English edition). [mouser.com/pdfDocs](https://www.mouser.com/pdfDocs/Infineon-Deadtime_calculation_for_IGBT_modules-ApplicationNotes-v01_00-EN.pdf)
- **SLVAE57** · TI, *Basics of Ideal Diodes*, SLVAE57B (2021). [ti.com/lit/an/slvae57](https://www.ti.com/lit/an/slvae57/slvae57.pdf)
- **SLVA716** · TI, *Integrated Load Switches Versus Discrete MOSFETs*, SLVA716A (2015, rev. 2019).
  [ti.com/lit/an/slva716](https://www.ti.com/lit/an/slva716/slva716.pdf)
- **SLVA139** · TI, *Reverse Current/Battery Protection Circuits*, SLVA139 (2003).
  [ti.com/lit/an/slva139](https://www.ti.com/lit/an/slva139/slva139.pdf)
- **AN10909** · Nexperia, *Low VCEsat transistors in medium power load switch applications*, Rev. 2 (2013; fetched
  through `tpm fetch`). [assets.nexperia.com/documents/application-note/AN10909.pdf](https://assets.nexperia.com/documents/application-note/AN10909.pdf)
- **SN6501** · TI, *SN6501 Transformer Driver for Isolated Power Supplies*, SLLSEA0I (2012, rev. 2021).
  [ti.com/lit/ds/symlink/sn6501.pdf](https://www.ti.com/lit/ds/symlink/sn6501.pdf)
- **TE coil suppression** · TE Connectivity, *Relay Coil Suppression with DC Relays* (application note, web page;
  fetched through `tpm fetch`). [te.com](https://www.te.com/en/products/relays-and-contactors/electromechanical-relays/intersection/relay-coil-suppression-dc-relays.html)
- **Wikipedia** · *Flyback diode*, *Snubber*, *H-bridge*, *Flyback converter*, *LED circuit*: readable overviews.
  [en.wikipedia.org/wiki/Flyback_diode](https://en.wikipedia.org/wiki/Flyback_diode)
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 2 (bipolar transistors),
  chapter 3 (field-effect transistors).

## `DRV-1` MOSFET gate threshold vs driving level <a id="drv-1"></a>

🛑 error · needs: VGS(th) of the FET (part data), the drive voltage · [catalogue entry](../checks.md#drv-1)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** A MOSFET whose gate is driven by a level that does not reach well above its threshold voltage
VGS(th) (maximum value, over temperature), so the FET may not turn on at all.

**Why it matters.** VGS(th) is the voltage where the FET just starts to conduct (a drain current of 250 µA or 1 mA in
most datasheets), not where it is fully on. A 3.3 V GPIO on a FET with VGS(th) up to 4 V leaves some parts off and
others half-on: the sample on the bench works, the production lot does not, and a half-on FET in a load path heats until
it fails.

**Example.** An IRF540N (VGS(th) 2-4 V) switching a 12 V fan directly from a 3.3 V MCU pin: it works on one board and
not on the next. An AO3400 class part, specified at 2.5 V, does the same job from 3.3 V.

**How to check.** For each FET, the high level of the gate net (the driving pin's VOH, `NET-7`, or the rail the gate is
pulled to, `NET-5`) against VGS(th) max from the part data, with margin; `DRV-20` then checks RDS(on).

**Fix.** A logic-level FET specified at the available drive, or a gate driver or level shifter.

**References.** AN11158 §2.6.1 "Static characteristics" (Table 5, Fig. 6 VGS(th) over temperature, Fig. 7 subthreshold
current); SLVA716 §2.3 "Vth Voltage Disadvantage"; AoE3 chapter 3.

## `DRV-2` High-side MOSFETs driven incorrectly <a id="drv-2"></a>

🛑 error · needs: FET channel type, rails, the drive level · [catalogue entry](../checks.md#drv-2)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A FET between a rail and a load (high side) whose gate drive cannot switch it: an N-channel FET
with its gate driven from logic at or below the rail voltage, or a P-channel FET whose gate is driven by a logic pin
whose high level is below its source (rail) voltage.

**Why it matters.** An N-channel high-side FET needs its gate a few volts *above* the rail to be fully on; driven from
logic it becomes a source follower and the load sees the gate voltage minus VGS(th), with the FET burning the rest. A
P-channel FET turns off only when its gate reaches the source voltage: a 3.3 V GPIO on the gate of a PMOS on 12 V leaves
VGS at −8.7 V, on for good, and the 12 V flows into the GPIO through any gate-source resistor.

**Example.** A PMOS load switch on 12 V with its gate wired straight to a 3.3 V MCU pin: the load is always on, and the
MCU pin clamps the gate through its protection diode. The working form is the one on B153A, where the MCU drives Q101
and Q101 pulls the gate of the VBUS switch Q102 down.

**How to check.** FETs whose source is on a rail and whose drain feeds a load; compare the gate net's levels with the
source voltage per channel type.

**Fix.** PMOS: a small NMOS or NPN as level shifter, with a gate-source pull-up. NMOS high side: a driver with a
bootstrap or charge pump (`DRV-16`), or turn it into a low-side switch where the load allows (`DRV-13`).

**References.** SLUA618 §5.1 "High-Side Drivers for P-Channel Devices" (§5.1.1 direct drive, §5.1.2 level-shifted drive,
Figure 20), §5.2 "High-Side Direct Drivers for N-Channel Devices"; SLVA716 §2-5 (PMOS discrete circuits); Toshiba gate
drive §2.4 "High-side drive from a half or full bridge".

## `DRV-3` PWM from MCU or SoC pins driving transistors directly <a id="drv-3"></a>

⚠️ warning · needs: pin functions (PWM), the FET's gate charge · [catalogue entry](../checks.md#drv-3)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** A timer or PWM output of an MCU or SoC wired to the gate of a power MOSFET (or base of a power
transistor) with no driver in between; the suggestion is a gate driver.

**Why it matters.** A GPIO sources and sinks a few milliamps to a few tens of milliamps; a power FET's gate charge is
tens of nanocoulombs. The edges become microseconds long, the FET spends that time in its linear region at every
switching event and heats, and the drain's dV/dt couples back into the pin through the Miller capacitance. A fault in
the FET (gate-drain short) puts the load voltage on the MCU pin.

**Example.** A 20 kHz PWM from a timer pin into an IRLZ44N driving a 12 V heater: the FET runs hot at half load, the
switching losses rather than RDS(on) dominate; a gate driver (UCC27517 class) fixes it.

**How to check.** Pins with a PWM function (`NET-12`) whose net reaches a transistor's gate or base through at most a
resistor; flag with the FET's gate charge where known.

**Fix.** A gate driver, or a series resistor plus a gate-source pull-down for slow PWM; keep the pin off the gate
directly.

**References.** SLUA618 §3.1 "PWM Direct Drive", §3.1.2 "Driver Protection", §2.3 "MOSFET Critical Parameters" (Miller
effect); AN11158 §2.6.2.1 "Gate charge".

## `DRV-4` Several gates on one driver: charge-pump capacity <a id="drv-4"></a>

⚠️ warning · needs: driver type and its charge-pump current, gate charge · [catalogue entry](../checks.md#drv-4)

**Impl.** mid · depends on NET-7, CFG-5

**What it catches.** A gate driver with a built-in charge pump (ideal-diode controllers, hot-swap and high-side drivers)
whose output drives several FET gates in parallel.

**Why it matters.** A charge pump supplies microamps to a few hundred microamps. Each extra gate adds its charge and its
leakage (and that of any gate TVS or pull-down): turn-on slows from microseconds to milliseconds, the FETs sit in their
linear region meanwhile, and a gate-source resistor sized for one FET may hold the gate below full enhancement.

**Example.** An ideal-diode controller whose datasheet assumes one FET, given two in parallel for current: the turn-on
time doubles, and a 100 kΩ gate pull-down per FET (two in parallel, 50 kΩ) takes enough of the charge pump's current
that the gates never reach their full drive.

**How to check.** Driver outputs (by part role) whose net reaches more than one gate; report the gate count, the sum of
gate charge and the DC load on the gate net.

**Fix.** Check the driver's charge-pump current against the total; use a driver with a stronger pump or a separate
supply, or fewer, larger FETs.

**References.** AN11158 §2.6.2.1 "Gate charge"; SLVAE57 §6.6.1 "Back-to-Back FET Driving Ideal Diode Controllers"; the
driver's datasheet (charge-pump current).

## `DRV-5` MOSFET inverter: gate resistor against Cgd coupling <a id="drv-5"></a>

⚠️ warning · needs: the FET inverter topology · [catalogue entry](../checks.md#drv-5)

**Impl.** easy · depends on netlist

**What it catches.** A MOSFET used as a logic inverter (source to ground, drain pulled up by a resistor and feeding an
input) whose gate is driven straight from a fast logic pin with no series resistor.

**Why it matters.** The gate-drain capacitance Cgd couples the gate edge into the drain node. While the FET is on the
drain is held; once it turns off, the rest of the falling gate edge is coupled through Cgd into a drain node held only
by the pull-up, and drives it below ground for a moment. The receiving input sees a negative spike (into its clamp
diode) and a glitch at the edge, worse with large FETs and weak pull-ups.

**Example.** A BSS138 inverting an MCU reset output into an open-drain enable with a 100 kΩ pull-up: on the fast falling
edge from the MCU the enable dips below ground and the downstream part sees an extra edge.

**How to check.** N-channel FETs with source on a ground, a resistor from drain to a rail, gate on a net driven by a
logic output with no series resistor.

**Fix.** A gate series resistor (100 Ω to 1 kΩ) to slow the gate edge, a stronger pull-up, or a logic inverter.

**References.** SLUA618 §2.3 (Miller capacitance), §3.5 "dv/dt Protection"; Toshiba gate drive §4.4.3 "Gate malfunction
prevention" (Figure 4.9, coupling through Cgd).

## `DRV-6` Q-Q chain not back to back <a id="drv-6"></a>

🛑 error · needs: FET orientation (source and drain), channel type · [catalogue entry](../checks.md#drv-6)

**Impl.** easy · depends on netlist (FET pin names, channel type from the symbol)

**What it catches.** Two MOSFETs in series meant to block current in both directions (a bidirectional switch, pogo-pin
or connector VBUS protection, a power selector) whose body diodes point the same way, so one direction is not blocked at
all; the thresholds of both FETs are checked along with it.

**Why it matters.** A single MOSFET blocks only one direction; its body diode conducts in the other. Two in series block
both only when they face each other (source to source, or drain to drain). Drawn in the same orientation the pair
behaves like one FET: the "switch" leaks the full current through both body diodes in one direction.

**Example.** On B200A three back-to-back PMOS pairs (Q1701 + Q1709, Q1702 + Q1707, Q1703 + Q1708) select 5 V, 3.3 V or
1.8 V for `+VREF`, and with all three off a level fed on J103 pin 7 is blocked from all three rails. With one pair drawn
in series instead of back to back, an external 5 V on pin 7 would flow straight into the 3.3 V rail.

**How to check.** Chains of two FETs between two nets with a shared middle node; the body diodes, from pin names and
channel type, must point in opposite directions.

**Fix.** Turn one FET around so the sources (or drains) meet in the middle, and drive both gates together.

**References.** SLVAE57 §4 "Reverse Battery Protection using MOSFETs", §5 "Reverse Polarity Protection vs Reverse
Current Blocking", §6.6.1 "Back-to-Back FET Driving Ideal Diode Controllers"; AN11158 §2.2 "Pinning information" (the
body diode); B200A review notes §5 (VREF selector).

## `DRV-7` No snubber on a half-bridge output <a id="drv-7"></a>

⚠️ warning · needs: switching nodes (`NET-6`) · [catalogue entry](../checks.md#drv-7)

**Impl.** easy · depends on NET-6

**What it catches.** A half-bridge switching node (the midpoint of two FETs in a DC-DC converter, motor driver or
class-D stage) with no RC snubber to ground or across the low-side FET, and no footprint for one.

**Why it matters.** The node rings at every edge: the FET's output capacitance resonates with the loop inductance of the
layout. The overshoot can exceed the FET's drain rating, and the ringing radiates at tens to hundreds of megahertz. An
RC snubber damps it; without at least a footprint the fix needs a rework.

**Example.** A 48 V motor half bridge built with 60 V FETs: a switch-node overshoot of 15-20 V at turn-off, ringing at
around 100 MHz, already takes the FETs into avalanche. A snubber sized from the measured ring (AN11160: a capacitor of
1-2 times the node's parasitic capacitance, then the resistor tuned for the lowest peak) damps it to a single overshoot,
provided the footprint is there.

**How to check.** Switching nodes (`NET-6`) without a series RC to ground or to the opposite rail.

**Fix.** Place a snubber footprint at each low-side FET (or across the high side); size it from the measured ring
frequency (AN11160).

**References.** AN11160 §2 "Designing the snubber - theory", §4 "Designing the snubber - in practice"; SLUA618 §2.8
"Effects of Parasitic Components"; Wikipedia, Snubber "Resistor-capacitor (RC)".

## `DRV-8` Open-loop flyback with no minimum load on its output <a id="drv-8"></a>

🛑 error · needs: converter type (no feedback), the output load · [catalogue entry](../checks.md#drv-8)

**Impl.** mid · depends on NET-5, CFG-5

**What it catches.** A flyback or transformer converter without feedback from the output (primary-side regulated, or a
fixed-ratio transformer driver) whose output has no resistive load or zener clamp to keep it in range at no load.

**Why it matters.** Without feedback the output voltage depends on the load. At no load the leakage energy charges the
output capacitor with nothing to discharge it, and the voltage rises far above nominal: the next part on that rail (an
LDO, an isolated transceiver) sees more than its rating as soon as the load is light or disconnected.

**Example.** An SN6501 transformer driver making an isolated supply for a CAN transceiver with no LDO after it: the
voltage that is right under load rises towards VIN × n (the turns ratio) when the bus idles. The datasheet strongly
advises an LDO for this reason, and that LDO's input rating must cover the no-load voltage.

**How to check.** Converters of an open-loop class (library), or a flyback controller with no feedback divider or
optocoupler from the output; look for a resistor or zener on the output.

**Fix.** A minimum-load resistor or a zener clamp at the output, sized from the datasheet's no-load voltage; or a
regulated topology.

**References.** SN6501 §8.2.2 "Detailed Design Procedure" (LDO strongly advised), §8.2.2.2 "LDO Selection" (no-load
output VIN-max × n); Wikipedia, Flyback converter "Operations" (primary-side sensing needs a minimum load).

## `DRV-9` Relays without flyback diode <a id="drv-9"></a>

🛑 error · needs: relay coils (part class), the switch driving them · [catalogue entry](../checks.md#drv-9)

**Impl.** easy · depends on netlist

**What it catches.** An electromechanical relay coil switched by a transistor or IC with no diode (or diode and zener,
TVS, RC) across the coil.

**Why it matters.** The coil's inductance keeps the current flowing when the switch opens: the voltage at the switch
rises until something conducts, usually the transistor in avalanche at hundreds of volts. It dies after a number of
cycles, or at once. A plain diode across the coil fixes that but slows the contact release, which wears contacts
switching heavy loads; a diode plus zener releases fast and still clamps.

**Example.** A 12 V relay driven by a BC817 (45 V) with no diode: the collector flies to well over 100 V at every
turn-off, and the transistor fails after days. On B153A U101 is a photo-MOS relay (ISOM8600): its LED input needs a
resistor (R106, 1 kΩ) and no flyback diode, since there is no coil.

**How to check.** Relays by class (symbol, designator `K`, library) whose coil pins have no diode, TVS or RC across them
(or from the switched pin to the coil supply).

**Fix.** A diode across the coil (cathode to the supply side), with a zener in series for fast release; a TVS where the
driver has the voltage headroom.

**References.** TE coil suppression ("Effects of Coil Suppression on Relay Dynamics and Life", "Suggested Methods for
Relay Coil Suppression": diode plus zener); Wikipedia, Flyback diode "Operation"; B153A review notes §10 (photo-MOS
relay U101).

## `DRV-10` Gate drive beyond VGS(max) <a id="drv-10"></a>

🛑 error · needs: VGS(max) (part data), gate and source voltages · [catalogue entry](../checks.md#drv-10)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A MOSFET whose gate-source voltage can exceed its VGS(max) rating (typically ±8, ±12 or ±20 V):
most often a high-side PMOS on a rail above 20 V with its gate pulled to ground to turn it on.

**Why it matters.** The gate oxide is thin; above VGS(max) it breaks down, the gate shorts to the source or drain, and
the FET fails on or off, often taking the driver with it. It may survive the first power-up and fail weeks later.

**Example.** A PMOS (±20 V gate) on a 24 V input with its gate pulled to ground by an NPN: VGS = −24 V. On B153A the
PMOS Q102 switches VBUS, which can reach 20 V after USB PD, and its gate is clamped by a 5V6 zener D101 so VGS stays
within rating whatever VBUS is.

**How to check.** For each FET, the extreme gate and source voltages from the rails and drive levels (`NET-5`, `NET-7`)
against VGS(max) from the part data.

**Fix.** A zener (and a resistor) from gate to source, a divider in the gate drive, or a FET with a higher gate rating.

**References.** AN11158 §2.4 "Limiting values" (Table 3, VGS); SLUA618 §5.1.1 "P-Channel Direct Drive"; B153A review
notes §10 (Q102, D101).

## `DRV-11` MOSFET body diode conducting in the off state <a id="drv-11"></a>

🛑 error · needs: FET orientation, channel type, which side is the source · [catalogue entry](../checks.md#drv-11)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A MOSFET used as a switch whose body diode is forward-biased in normal operation, so it conducts
when the FET is meant to be off: source and drain swapped in the symbol or the wiring, or a load switch oriented so the
load is fed through the diode.

**Why it matters.** The FET then switches nothing. The load gets power through the diode, a diode drop low; the "off"
state does not exist, and the FET dissipates the diode drop times the load current when it should be fully on.

**Example.** A PMOS high-side switch drawn with its drain on the battery and its source on the load: the load sits at
VBAT − 0.7 V whether the gate is driven or not. On B200A the I2C low-path bus switch Q302/Q303 is a single PMOS with its
source on the bus side: a low driven from the bus reaches the TXS0102 through the body diode at about 0.6 V, which the
review flagged for a V_OL margin check at 1.8 V.

**How to check.** For each FET, compare the voltages of the source and drain nets in the off state; for a PMOS the body
diode conducts when the drain is above the source, for an NMOS when the source is above the drain.

**Fix.** Swap source and drain (PMOS high-side switch: source on the supply), or use a back-to-back pair (`DRV-6`).

**References.** AN11158 §2.2 "Pinning information", §2.6.2.4 "Diode characteristics"; SLVA139 Figure 3 "PMOS FET in the
Power Path"; SLVAE57 §4.1; B200A review notes §8 (Q302/Q303).

## `DRV-12` BJT base driven from a logic pin without a base resistor <a id="drv-12"></a>

🛑 error · needs: bipolar transistors (part class), the driving pin · [catalogue entry](../checks.md#drv-12)

**Impl.** easy · depends on netlist

**What it catches.** The base of a bipolar transistor connected straight to a logic output with no resistor (and the
transistor not a digital transistor with built-in resistors).

**Why it matters.** The base-emitter junction is a diode: it clamps the pin at about 0.7 V, and the only current limit
is the pin's own output resistance. The pin sources tens of milliamps into the base, beyond its rating; the pin and the
transistor both degrade, and the MCU's IO budget is spent on one pin.

**Example.** A BC817 switching a buzzer with its base on an STM32 GPIO directly: the pin sits at 0.8 V instead of 3.3 V,
delivers its short-circuit current, and other pins on the same port droop.

**How to check.** BJTs (designator `Q`, symbol or MPN) whose base net reaches an IC output with no resistor in series;
skip pre-biased (digital) transistors by MPN.

**Fix.** A base resistor sized for the collector current over the transistor's minimum gain with margin (1-10 kΩ for
small loads), or a pre-biased transistor (DTC, PDTC series).

**References.** AN10909 §2.1 "Important parameter for BJTs in a load switch application" (base resistor and IC/IB); AoE3
chapter 2.

## `DRV-13` Low-side switch under a load with ground-referenced signals <a id="drv-13"></a>

⚠️ warning · needs: the load's ground pin, its other connections · [catalogue entry](../checks.md#drv-13)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** An N-channel FET or NPN switching the ground of a load (a module, a sensor, a display) that also
has signal lines to the rest of the board, referenced to the main ground.

**Why it matters.** With the switch off, the load's ground is no longer ground: it floats up towards its supply through
the load's own circuitry. Its signal pins then sit far above their ground, the signals from the board drive its clamps
(`BKF-2`), and its outputs push current back into the board's inputs. Low-side switching is fine for a plain load (an
LED, a motor); it does not work for anything that talks to the board.

**Example.** A GPS module whose GND is switched by an NMOS to save power, with its UART on the MCU: with the switch off,
the module's ground rises to about 3 V through its internals, its TX pin drives the MCU's RX near 3.3 V, and the MCU's
TX powers the module through its RX clamp.

**How to check.** Ground pins of parts whose net is the drain of a low-side switch (not a ground), where the same part
has other pins on nets shared with the board.

**Fix.** Switch the supply on the high side, or use a load switch with power-off-safe IO isolation.

**References.** SLVA716 §6 "NMOS Discrete Circuits"; catalogue `BKF-2`, `DRV-2`.

## `DRV-14` Inductive load switched without a freewheel path <a id="drv-14"></a>

⚠️ warning · needs: inductive loads (part class, project data for connectors) · [catalogue entry](../checks.md#drv-14)

**Impl.** easy · depends on netlist (CFG-2 for loads behind connectors)

**What it catches.** A solenoid, motor, coil, valve or transformer switched by a transistor with no freewheel diode, TVS
or snubber across the load or the switch. The relay case is `DRV-9`.

**Why it matters.** Same physics as the relay: interrupting an inductor's current makes the voltage rise until something
conducts. Motors and solenoids store far more energy than a relay coil, so the switch fails faster, and the spike
couples into everything sharing its ground.

**Example.** A 12 V solenoid valve on a connector, switched by a low-side NMOS with no diode: the FET avalanches at
every turn-off; it survives only if its avalanche energy rating covers the solenoid's stored energy.

**How to check.** Switches whose load is an inductive part (by class) or a connector declared as inductive in the
project; look for a diode, TVS or RC across the load or the switch.

**Fix.** A freewheel diode (fast or Schottky) across the load, or a TVS across the switch where a fast decay is needed.

**References.** Wikipedia, Flyback diode "Operation", "Applications"; AN11158 §2.4 (avalanche ratings); catalogue
`DRV-9`.

## `DRV-15` Half bridge driven by one signal and its inverter, no dead time <a id="drv-15"></a>

⚠️ warning · needs: half-bridge topology, logic between PWM and gates · [catalogue entry](../checks.md#drv-15)

**Impl.** hard · depends on NET-6, NET-10, RPT-14

**What it catches.** A half bridge whose high-side and low-side gates are driven by one PWM signal and its logic inverse
(an inverter gate, a transistor inverter), with nothing that inserts a delay between one FET turning off and the other
turning on.

**Why it matters.** FETs turn off more slowly than they turn on. With complementary drive and no dead time, both are on
at once during every edge: shoot-through, a short from the rail to ground through both FETs, limited only by their
resistance. The FETs heat, the supply sees current spikes, and at higher voltages they fail.

**Example.** A motor half bridge with the low side driven through a 74LVC1G04 from the high side's PWM: the inverter's
few nanoseconds of delay are far shorter than the FET's turn-off time, both conduct at every edge.

**How to check.** Half bridges (`NET-6`) whose two gate nets are derived from one signal through an inverter (`NET-10`,
logic data `RPT-14`) with no dead-time circuit or driver with built-in dead time.

**Fix.** A half-bridge driver with dead time, two PWM channels with dead-time insertion (complementary timer outputs),
or an RC-diode delay on each turn-on.

**References.** AN2007-04 §1.1 "Reason of IGBT bridge shoot through", §2 "Calculate proper dead time"; Wikipedia,
H-bridge "General" (shoot-through).

## `DRV-16` Gate driver bootstrap diode or capacitor missing <a id="drv-16"></a>

🛑 error · needs: driver pin roles (HB/HS, VB/VS), whether the diode is internal · [catalogue entry](../checks.md#drv-16)

**Impl.** mid · depends on NET-7, CFG-5

**What it catches.** A half-bridge or high-side gate driver whose bootstrap capacitor (between its bootstrap pin and the
switch node) is missing, or whose bootstrap diode (from the driver supply to the bootstrap pin) is missing on a part
without an internal one.

**Why it matters.** The high-side gate is driven from the bootstrap capacitor, which floats on the switch node and is
recharged through the diode each time the low side is on. Without the capacitor the high side never turns on; without
the diode it turns on once and then decays.

**Example.** An IR2104 half-bridge driver with C between VB and VS but no external diode from VCC to VB: the high side
switches for a few cycles from the initial charge and then stays off.

**How to check.** Driver parts by pin roles (`NET-7`): a capacitor between the bootstrap pin and the switch node, and a
diode from the driver supply to the bootstrap pin unless the part data says it is internal.

**Fix.** Add both; size the capacitor for the high-side gate charge and the longest on time (SLUA887).

**References.** SLUA887 §2 "Basic Operation of Bootstrap Circuit", §3.1 "Bootstrap Capacitor", §3.3 "External Bootstrap
Diode"; SLUA618 §5.2.2 "Bootstrap Gate-Drive Technique"; Toshiba gate drive §3.2 "Bootstrap circuit".

## `DRV-17` LED without a series resistor or current limit <a id="drv-17"></a>

🛑 error · needs: LEDs (part class), the path to a rail or pin · [catalogue entry](../checks.md#drv-17)

**Impl.** easy · depends on NET-5

**What it catches.** An LED whose chain from a rail or GPIO to ground (or to a GPIO sinking it) contains no resistor and
no constant-current driver.

**Why it matters.** An LED has a steep exponential I-V curve: on a voltage source the current is set only by the
source's internal resistance and the LED's own, both small and unspecified. The LED or the pin driving it is overloaded,
and the brightness varies wildly between parts.

**Example.** A red LED from a 3.3 V GPIO straight to ground: the pin sources its short-circuit current into a 2 V LED.
On B200A the status LED D102 has 33 Ω in series to GPIO38 (active low); on B153A the two status LEDs have 1 kΩ each.

**How to check.** LEDs (by class) whose chain has no resistor and no part of a current-regulator class.

**Fix.** A series resistor (V − Vf) / I, or an LED driver.

**References.** Wikipedia, LED circuit "Basic circuit", "Series resistor"; B200A review notes §11 (D102), B153A review
notes §11 (LED pins).

## `DRV-18` LED forward voltage above its rail <a id="drv-18"></a>

🛑 error · needs: LED forward voltage (colour or part data), the supply · [catalogue entry](../checks.md#drv-18)

**Impl.** mid · depends on NET-5, CFG-5

**What it catches.** An LED (or several in series) whose total forward voltage is at or above the voltage driving it, so
it never lights, or lights only faintly on the best part.

**Why it matters.** Forward voltage depends on the colour: red about 1.8-2.2 V, green 2-3.3 V, blue and white 2.8-3.6 V.
A white LED on 1.8 V does not light at all; on 3.3 V with a GPIO and a resistor it lights dimly and varies part to part;
two reds in series on 3.3 V barely glow.

**Example.** A white indicator LED on a 1.8 V IO rail of an FPGA bank: it never lights, and nobody notices until
assembly.

**How to check.** For each LED chain, the Vf from the part data (or the colour in the value) times the number in series,
against the rail or the driving pin's VOH.

**Fix.** Power the LED from a higher rail and switch it with a transistor, choose a lower-Vf colour, or use a driver
with a boost.

**References.** Wikipedia, LED circuit "Power source considerations"; the LED datasheet (VF at the operating current).

## `DRV-19` No gate resistor between a gate driver and a MOSFET <a id="drv-19"></a>

💡 suggestion · needs: gate driver outputs (pin roles), FET gates · [catalogue entry](../checks.md#drv-19)

**Impl.** mid · depends on NET-7

**What it catches.** A gate driver output connected straight to a MOSFET gate with no series resistor.

**Why it matters.** The gate loop (driver output, trace, gate capacitance, return) is a series LC; without resistance it
rings at every edge, which shows as EMI, gate overshoot beyond VGS(max) (`DRV-10`) and false turn-on. The gate resistor
also sets the switching speed, the one knob for trading losses against EMI. A footprint with 0 Ω costs nothing and saves
a rework.

**Example.** A UCC27517 driving a power FET 3 cm away with no gate resistor: the gate rings at 50 MHz with a 3 V
overshoot, the board fails radiated emissions at that frequency.

**How to check.** Nets between a driver output (by pin role) and a gate with no resistor in series.

**Fix.** A gate resistor (typically 2-20 Ω) close to the gate, optionally with a diode across it for a faster turn-off.

**References.** SLUA618 §2.8 "Effects of Parasitic Components", §3.1 "PWM Direct Drive"; Toshiba gate drive §4.3 "Gate
resistors and switching characteristics", §4.4.2 "Optimal gate resistor".

## `DRV-20` RDS(on) only specified at a gate voltage above the drive available <a id="drv-20"></a>

⚠️ warning · needs: VGS of the RDS(on) spec (part data), the drive level · [catalogue entry](../checks.md#drv-20)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** A MOSFET whose datasheet gives RDS(on) only at a gate voltage higher than the drive the board
provides (a 10 V-rated FET driven at 3.3 or 5 V).

**Why it matters.** Above VGS(th) the FET conducts, but RDS(on) falls steeply with VGS until it reaches the specified
value. With only 10 V figures in the datasheet, the resistance at 3.3 V is not guaranteed at all: the typical curve may
show a few times the rated value, and the worst-case part is worse. The FET heats in proportion. `DRV-1` asks whether it
turns on; this asks whether it turns on enough.

**Example.** An IRF540N, RDS(on) specified only at VGS = 10 V, as the load switch for 3 A from a 5 V gate drive: on the
typical curve it carries the current, on a part at the edge of VGS(th) it dissipates watts.

**How to check.** The lowest VGS at which the part data specifies RDS(on) against the gate drive; report the current
through the FET where known.

**Fix.** A logic-level part specified at the drive voltage (RDS(on) at 2.5 V or 4.5 V), or more gate drive.

**References.** AN11158 §2.6.1 (Fig. 8, RDS(on) as a function of VGS); SLVA716 §2.3; catalogue `DRV-1`.
