# RATE checks, explained

Detail page for the `RATE` category of the [check catalogue](../checks.md#rate-1). Each section explains one check: what
it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the usual fix, and
where to read more. The one-line entries in the catalogue stay the source of truth for ID, severity and wording; this
page is the commentary.

The **Impl.** line rates how hard the check is to build in better_erc, in pure Rust and without an LLM: `easy` (the
netlist alone: connectivity, pin types, part classes and values), `mid` (part data or a project setting must be built or
read first), `hard` (analysis the code does not have yet: power states, following signals, per-family datasheet rules),
`judgement` (not a rule: a human or an LLM reads intent or datasheet prose, better_erc can at most list candidates). The
IDs after "depends on" are [FEATURES.md](../../FEATURES.md) items. Every RATE check compares a stress (voltage, current,
power, temperature) with a rating: the stress comes from rail voltages (`NET-5`, from net names today) or the project
file (`CFG-2`), the rating from the part's MPN in the part database (`CFG-5`); a part without data is reported as not
checked (`DIAG-2`).

References used throughout:

- **AVX derating** · T. Zedníček, J. Gill (KYOCERA AVX), *Voltage Derating Rules for Solid Tantalum and Niobium
  Capacitors*, technical paper (hosted on ethertronics.com, an AVX site).
  [ethertronics.com/.../voltaged.pdf](https://ethertronics.com/docs/techinfo/Tantalum-NiobiumCapacitors/voltaged.pdf)
- **SLVA477** · TI, *Basic Calculation of a Buck Converter's Power Stage*, Rev. C.
  [ti.com/lit/an/slva477](https://www.ti.com/lit/an/slva477/slva477.pdf)
- **SLVA118** · TI, *Linear Regulator Design Guide for LDOs* (Digital Designer's Guide to Linear Voltage Regulators and
  Thermal Management), Rev. A (2008). [ti.com/lit/an/slva118](https://www.ti.com/lit/an/slva118/slva118.pdf)
- **SPRA953** · TI, *Semiconductor and IC Package Thermal Metrics*, Rev. D (2024).
  [ti.com/lit/an/spra953](https://www.ti.com/lit/an/spra953/spra953.pdf)
- **TLV702** · TI, *TLV702 300-mA, Low-IQ, Low-Dropout Regulator*, datasheet SLVSAG6D: the LDO used on B153A and B200A.
  [ti.com/lit/ds/symlink/tlv702](https://www.ti.com/lit/ds/symlink/tlv702.pdf)
- **Wikipedia, Ceramic capacitor** · class 2 temperature codes (X7R, X5R, Y5V) and "Voltage dependence of capacitance".
  [en.wikipedia.org/wiki/Ceramic_capacitor](https://en.wikipedia.org/wiki/Ceramic_capacitor)
- **Wikipedia, Derating** · a short overview. [en.wikipedia.org/wiki/Derating](https://en.wikipedia.org/wiki/Derating)
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 9 (voltage regulation and
  power conversion).

## `RATE-1` Capacitor voltage rating margins <a id="rate-1"></a>

⚠️ warning · needs: rail voltage across the capacitor, its voltage rating (value field or MPN) · [catalogue
entry](../checks.md#rate-1)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** A capacitor whose voltage rating leaves too little margin over the highest voltage across it: the
rail's maximum (not its nominal), plus ripple and the transients it is exposed to.

**Why it matters.** A capacitor at or above its rating fails early: ceramics crack and short, electrolytics and
tantalums leak and vent. The nominal rail value is rarely the worst case: a "24 V" industrial supply runs up to about 30
V, a USB-C VBUS can be negotiated to 20 V (`RATE-19`), and hot-plugging a cable with inductance rings well above the
steady voltage. Ceramics also lose capacitance with bias (`RATE-16`), which is a second reason to choose a higher
rating.

**Example.** A 25 V MLCC on a 24 V industrial input: 24 V +20 % is 28.8 V before any surge. The input needs 50 V parts
(or a clamp in front of 35 V parts). On B200A the assembly notes asked whether the USB input capacitors should be 50 V
parts.

**How to check.** Take the rail's maximum voltage (from the project's range for the net, or the name's voltage with a
default tolerance), read the capacitor's rating from its value field (`100n 50V`) or MPN, and report a margin below a
set ratio (for example rating under 1.25 × maximum for ceramics; electrolytics and tantalums per `RATE-17`).

**Fix.** A higher voltage rating; for inputs exposed to hot-plug, a TVS or an input filter that limits the overshoot.

**References.** Wikipedia, Ceramic capacitor "Voltage proof"; AVX derating (why rated voltage is not a safe working
voltage for tantalum); B200 rev B change list ("50V rating at USB?").

## `RATE-2` Resistor power rating <a id="rate-2"></a>

⚠️ warning · needs: voltage across or current through the resistor, its power rating (package or MPN) · [catalogue
entry](../checks.md#rate-2)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** A resistor whose dissipation, V²/R or I²R in the worst case, exceeds its rated power (with derating
for the board temperature).

**Why it matters.** An overloaded resistor drifts, discolours the board, desolders itself or opens. Chip resistor
ratings are small (a 0402 is typically 1/16 W) and quoted at 70 °C ambient, derating linearly to zero at the maximum
temperature; on a warm board a resistor at its nominal rating is already over it. Bleeder resistors, LED series
resistors on high rails and pull-downs on high-voltage nets are the usual cases.

**Example.** A 1 kΩ 0402 bleeder across a 12 V rail dissipates 144 mW, more than twice a 63 mW rating; an 0805 at 125 mW
is still too small, a 1206 (250 mW) has some margin.

**How to check.** For resistors whose both ends are on known rails (or a rail and ground), the voltage across is known
from the netlist and rail voltages; for others the project must give a current. Compare with the rating from the MPN, or
from the package when there is no MPN, derated per the datasheet's curve for the board temperature.

**Fix.** A larger package, a higher value where the function allows, or several resistors in series or parallel to share
the power.

**References.** Vishay, *MCS 0402, MCT 0603, MCU 0805, MCA 1206 Professional Thin Film Chip Resistors* datasheet (rated
dissipation P70 per size, derating) ([vishay.com/docs/28705](https://www.vishay.com/docs/28705/mcx0x0xpro.pdf)); AoE3
chapter 1 (resistor power ratings).

## `RATE-3` Shunt power ratings <a id="rate-3"></a>

⚠️ warning · needs: shunt value, maximum current in its path, its power rating · [catalogue entry](../checks.md#rate-3)

**Impl.** mid · depends on CFG-2, CFG-5

**What it catches.** A current-sense shunt whose I²R at the maximum current (including fault current up to the limit or
fuse) exceeds its rating.

**Why it matters.** Shunts carry the full load current and sit in the power path; an overloaded shunt drifts with
temperature (so the measurement drifts), heats the board under the sense amplifier, and fails open, which removes the
load's path or the ground reference. The fault current matters: a shunt in front of a current limiter carries the limit,
not the nominal load.

**Example.** A 10 mΩ shunt in a 1206 package rated 0.5 W in an 8 A path: 0.64 W, over the rating at room temperature
already; at a 10 A fault limit it is 1 W. A 2512 shunt rated 1 W or more, or 5 mΩ with more amplifier gain (`ANA-15`),
fixes it.

**How to check.** Recognise shunts (low-value resistors in a power path, connected to a sense amplifier's inputs); take
the maximum current of the path from the project (or the limit of the current limiter or fuse in series) and compare I²R
with the rating from the MPN, derated for temperature.

**Fix.** A larger or higher-rated shunt, a lower value with more gain, or a shunt with Kelvin terminals in a package
meant for power.

**References.** TI INA181 datasheet (SBOS793H) §8.1.2 "RSENSE and Device Gain Selection"
([ti.com/lit/ds/symlink/ina181](https://www.ti.com/lit/ds/symlink/ina181.pdf)); Vishay thin film chip resistor datasheet
(derating).

## `RATE-4` MOSFET Vds rating <a id="rate-4"></a>

⚠️ warning · needs: drain and source nets and their voltages, the MOSFET's Vds rating · [catalogue
entry](../checks.md#rate-4)

**Impl.** mid · depends on NET-5, CFG-5, LIB-3

**What it catches.** A MOSFET whose maximum drain-source voltage is below, or too close to, the highest voltage it must
block between drain and source.

**Why it matters.** Above Vds(max) the FET avalanches: it may survive a few events, then fails short, which for a power
switch means the load is always on or the supply is shorted. Inductive loads and hot-plugged inputs add spikes above the
steady voltage. Load switches on USB-C VBUS must block whatever the port can negotiate.

**Example.** On B153A the switch Q102 that puts the receptacle VBUS on header pin 10 is an SLMT330V3CP in the BOM (the
value field says SSM3J35); VBUS can be 20 V after PD negotiation, and the schematic review flagged its Vds rating
against 20 V as a point to check.

**How to check.** With the pins named (`LIB-3`), take the voltages of the drain and source nets (rails, or the project's
range for the net) and compare the largest difference with the Vds rating from the MPN; report margins below a set ratio
(for example 1.2 for resistive loads, more for inductive ones).

**Fix.** A FET with a higher Vds rating; a clamp (TVS, snubber, freewheeling diode) for inductive spikes.

**References.** B153A schematic review notes (Q102, Vds for 20 V); Wikipedia, USB hardware "USB Power Delivery" (fixed
supplies up to 20 V); AoE3 chapter 3 (FETs).

## `RATE-5` MOSFET power dissipation estimate <a id="rate-5"></a>

⚠️ warning · needs: drain current, Rds(on) at the gate drive and temperature, switching losses, the package's thermal
resistance · [catalogue entry](../checks.md#rate-5)

**Impl.** hard · depends on CFG-2, CFG-5, RPT-4

**What it catches.** A MOSFET whose estimated dissipation (conduction I²·Rds(on), plus switching losses in a converter)
raises its junction above the rated temperature with its package and copper.

**Why it matters.** Rds(on) is quoted at 10 V gate drive and 25 °C; at 3.3 V drive and a hot junction it can be several
times higher, and dissipation rises with it. A small SOT-23 FET that switches 2 A "easily" on the datasheet's front page
runs at well above 100 °C on a real board. The FET then fails, or its Rds(on) runs away.

**Example.** An SOT-23 load switch with 50 mΩ at 4.5 V gate drive carrying 2 A: 0.2 W, and with Rds(on) rising about
1.5× at a hot junction, 0.3 W; at a junction-to-ambient resistance around 200 °C/W (typical for SOT-23 on modest copper,
compare SPRA953) that is 60 °C above ambient.

**How to check.** Needs the path current (project), the gate drive voltage (from the driving net, through `NET-10`),
Rds(on) at that drive from the part data, and for switching FETs the frequency and transition times (`RPT-4`). Then P ×
RθJA against the maximum junction temperature at the board's ambient.

**Fix.** A FET with lower Rds(on) at the actual gate drive, more gate drive, a larger package or more copper.

**References.** SPRA953 §1 (RθJA depends strongly on the PCB); SLVA477 §2 (peak switch current in a buck converter);
AoE3 chapter 3 (FET switches) and chapter 9.

## `RATE-6` LDO current rating <a id="rate-6"></a>

⚠️ warning · needs: the rail's total load current, the regulator's rated output current · [catalogue
entry](../checks.md#rate-6)

**Impl.** mid · depends on RPT-2, CFG-2, CFG-5

**What it catches.** A linear regulator whose output rail draws more than its rated output current, when the loads'
consumption is known.

**Why it matters.** Beyond its rating the regulator current-limits: the rail sags, parts brown out at the worst moment
(radio transmit bursts, LED flashes, motor start). Even below the limit, current sets dissipation (`RATE-21`).
Consumption grows during a design as features are added to an "auxiliary" rail.

**Example.** A TLV702 (300 mA) feeding an ESP32 module: Wi-Fi transmit peaks reach several hundred milliamps, so the
rail dips and the module resets during transmission, although the average consumption is well below 300 mA.

**How to check.** Sum the load currents per rail for the chosen power state (`RPT-2`, from part data or the project),
including peaks where they are known; compare with the rated output current of the regulator feeding the rail.

**Fix.** A regulator with a higher rating (or a buck converter), splitting the loads over two rails, or bulk capacitance
sized for short peaks.

**References.** TLV702 datasheet (300 mA rating); SLVA118 §3 (power dissipation in the linear regulator).

## `RATE-7` LED too bright <a id="rate-7"></a>

⚠️ warning · needs: LED current from the series resistor and rail, the LED's luminous intensity per current · [catalogue
entry](../checks.md#rate-7)

**Impl.** mid · depends on NET-5, CFG-5

**What it catches.** An indicator LED driven at a current that makes it far brighter than an indicator needs, typically
because the series resistor was chosen for an old, dim LED type.

**Why it matters.** Modern InGaN and AlGaInP LEDs give at 1-2 mA what old GaP LEDs gave at 20 mA. An indicator at 10-20
mA is glaring in a dark room, wastes current on battery products, lights up a whole enclosure through its light pipe,
and heats up small packages. Bright status LEDs are also a common complaint in user reviews of finished products.

**Example.** Two LEDs in the same 1608 package from one vendor: the APT1608SGC (GaP green) gives 12 mcd typical at 20
mA; the APT1608LZGCK (InGaN green) gives 100 mcd typical at 2 mA. Swapping the second in for the first with the same 150
Ω resistor on 5 V (about 15 mA) makes an indicator several hundred millicandela bright.

**How to check.** Compute the LED current from the rail voltage, a forward voltage per colour (or from the part data)
and the series resistor; with the luminous intensity per current from the part data, estimate the intensity and report
indicators above a set brightness (or simply above a few milliamps for indicator-class parts).

**Fix.** A larger series resistor (1-2 mA for indicators with modern LEDs), or PWM dimming from firmware.

**References.** Kingbright APT1608SGC datasheet
([kingbrightusa.com](https://www.kingbrightusa.com/images/catalog/SPEC/APT1608SGC.pdf)) and APT1608LZGCK datasheet
([kingbrightusa.com](https://www.kingbrightusa.com/images/catalog/SPEC/APT1608LZGCK.pdf)), luminous intensity tables;
Wikipedia, Light-emitting diode "Considerations for use"
([en.wikipedia.org/wiki/Light-emitting_diode](https://en.wikipedia.org/wiki/Light-emitting_diode)).

## `RATE-8` Temperature ranges that make no sense <a id="rate-8"></a>

⚠️ warning · needs: operating temperature range per part (MPN data, capacitor dielectric) · [catalogue
entry](../checks.md#rate-8)

**Impl.** mid · depends on CFG-5, CFG-2

**What it catches.** Parts on one board whose temperature ranges do not fit together or do not fit the product's range:
some parts chosen for −55 to 125 °C while the ICs stop at 85 °C (paying for a range the board cannot use), or the
reverse, a few commercial-range parts (0 to 70 °C, Y5V capacitors) on a board meant for −40 °C.

**Why it matters.** The board's range is the narrowest part's range. A single 0-70 °C part on an industrial board makes
the whole product commercial; a Y5V capacitor rated −30 to +85 °C loses most of its capacitance at the ends of that
range. The other direction costs money and hides the real limit: when most parts are rated to 125 °C, nobody notices the
one 85 °C IC.

**Example.** An outdoor sensor specified for −40 °C with a Y5V (−30/+85 °C) bulk capacitor on the regulator's output: at
−40 °C the capacitor is outside its rating and its capacitance has collapsed, and the regulator oscillates in winter. Or
X7R capacitors (−55/+125 °C) everywhere, next to an MCU and a PHY rated −40 to 85 °C: the board is an 85 °C board, the
X7R choice only buys bias and ageing margin.

**How to check.** Collect the operating range per part (from the MPN; for capacitors from the dielectric code); compute
the board's range as the intersection; report parts that narrow it below the product range from the project, and
outliers far wider than the rest as an informational note.

**Fix.** Parts rated for the product's range; X7R or better for class 2 ceramics on industrial boards; state the product
range in the project so the check has a target.

**References.** Wikipedia, Ceramic capacitor "Class 2 ceramic capacitors" (X7R −55/+125 °C, X5R −55/+85 °C, Y5V −30/+85
°C with +22/−82 % change); SPRA953 §6 (industrial and commercial temperature ranges).

## `RATE-9` Resistor voltage rating <a id="rate-9"></a>

⚠️ warning · needs: voltage across each resistor, its maximum working voltage · [catalogue entry](../checks.md#rate-9)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** A resistor with more voltage across it than its maximum working voltage, typically the top resistor
of a divider from a high-voltage rail, or a bleeder across a high-voltage capacitor.

**Why it matters.** Small chip resistors have a voltage limit independent of their power rating: a high-value 0402
across 100 V dissipates almost nothing and still fails, by arcing across the short body or by slow drift as the film
breaks down. High-value resistors reach the voltage limit long before the power limit, so the power check (`RATE-2`)
alone misses them.

**Example.** A 1 MΩ / 10 kΩ divider measuring a 100 V bus with the 1 MΩ as a single 0402: 99 V across a part whose
maximum operating voltage is 75 V in Vishay's MCS 0402 thin film series (and 50 V for many thick film 0402 series).
Three 330 kΩ 0603 resistors in series share the voltage at 33 V each.

**How to check.** For each resistor, compute the voltage across it from the rails at its ends and the divider ratio
(`RPT-6`); compare with the maximum working voltage from the MPN, or a default per package.

**Fix.** Several resistors in series, larger packages, or parts specified for high voltage.

**References.** Vishay MCS/MCT/MCU/MCA datasheet (operating voltage Umax per size: 75 V for 0402, 100 V for 0603, 150 V
for 0805, 200 V for 1206) ([vishay.com/docs/28705](https://www.vishay.com/docs/28705/mcx0x0xpro.pdf)).

## `RATE-10` LED current above its maximum forward current <a id="rate-10"></a>

⚠️ warning · needs: LED current from rail and series resistor, the LED's maximum forward current · [catalogue
entry](../checks.md#rate-10)

**Impl.** mid · depends on NET-5, CFG-5

**What it catches.** An LED whose current, set by its series resistor (or a driver), exceeds its maximum DC forward
current.

**Why it matters.** Overdriven LEDs dim quickly and fail. Indicator LEDs in small packages are rated only 20-30 mA, and
a resistor sized for 3.3 V moved onto a 5 V or 12 V rail multiplies the current.

**Example.** A green APT1608SGC (DC forward current 25 mA maximum, forward voltage about 2.2 V) on 5 V with a 68 Ω
resistor: (5 − 2.2) / 68 ≈ 41 mA, well above the rating.

**How to check.** Compute the current from the rail, a forward voltage (part data or a default by colour, at the low end
so the current comes out high) and the series resistance; compare with the maximum forward current from the part data,
derated for temperature.

**Fix.** A larger series resistor; a constant-current driver for power LEDs.

**References.** Kingbright APT1608SGC datasheet, absolute maximum ratings
([kingbrightusa.com](https://www.kingbrightusa.com/images/catalog/SPEC/APT1608SGC.pdf)); Wikipedia, Light-emitting diode
"Power sources".

## `RATE-11` Supply pin voltage outside the part's range <a id="rate-11"></a>

🛑 error · needs: rail voltage per supply pin, the part's operating range and absolute maximum per supply pin ·
[catalogue entry](../checks.md#rate-11)

**Impl.** mid · depends on NET-5, NET-7, CFG-5

**What it catches.** A part whose supply pin sits on a rail outside its operating range (it may not work) or above its
absolute maximum (it gets damaged): a 3.3 V rail into a 1.8 V-only part, a 5 V rail into a 3.6 V-maximum part.

**Why it matters.** It is one of the most basic errors, and still common, because the netlist does not carry voltages
and the part's range lives in its datasheet. It hides in parts with many variants (the same flash in 1.8 V and 3.3 V
versions), in nets whose names do not tell the truth, and in reused blocks moved to a board with different rails.

**Example.** B153A and B200A both have a "1V8" LDO that is a TLV70233, a fixed 3.3 V part: the `+1V8_LDO` net is really
about 3.2 V. On B153A it is the low option of the voltage for the target's SBU lines, so a 1.8 V target would see 3.2 V.
On B200A the flash is a 2.7-3.6 V part on 3.3 V, correct, but its BOM datasheet link points at the 1.8 V variant: a
check reading that range would have flagged it, and a buyer following the link could order a part that 3.3 V would
destroy.

**How to check.** For each supply pin (`NET-7` roles), take the rail's voltage range (project or name) and compare it
with the operating range and absolute maximum of that pin from the part data. Use the regulator's real output (from its
MPN) rather than the net name where they differ (`NAME-8`).

**Fix.** The right part variant or the right rail; fix the regulator (TLV70218 for 1.8 V in the B153 and B200 rev B
change lists).

**References.** B153A and B200A schematic review notes (TLV70233 on `+1V8_LDO`, flash datasheet link); B153 and B200 rev
B change lists.

## `RATE-12` Regulator input above its maximum Vin <a id="rate-12"></a>

🛑 error · needs: the input rail's maximum voltage, the regulator's maximum input voltage · [catalogue
entry](../checks.md#rate-12)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** A regulator whose input rail can exceed its maximum input voltage: an LDO rated for 6 V on a 12 V
or 24 V rail, a buck rated for 17 V on a rail that can reach 20 V.

**Why it matters.** Regulators are fed from the board's highest rails, where the voltage is least controlled: a wall
adapter, a battery charger, a USB-C VBUS after PD negotiation, an automotive supply with load dumps. Over the absolute
maximum the regulator fails, usually short, and passes the input voltage to every part on its output.

**Example.** A TLV702 (absolute maximum 6 V on IN) fed from a USB-C VBUS: correct for a 5 V-only sink, destroyed the
first time the port negotiates 9 V or more. On a board like B153, where VBUS can be 20 V, such an LDO must sit behind a
regulator or a switch that limits its input.

**How to check.** Take the maximum voltage of the regulator's input rail (project range, PD capability, battery maximum)
and compare with the regulator's maximum Vin from the part data; report anything above the recommended maximum, as an
error above the absolute maximum.

**Fix.** A regulator rated for the full input range, a pre-regulator, or limiting what the port can negotiate.

**References.** TLV702 datasheet §6.1 "Absolute Maximum Ratings" (IN −0.3 to 6 V); Wikipedia, USB hardware "USB Power
Delivery".

## `RATE-13` Diode current or reverse voltage <a id="rate-13"></a>

⚠️ warning · needs: the diode's path current and reverse voltage, its ratings · [catalogue entry](../checks.md#rate-13)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** A diode whose average forward current exceeds its rating, or whose reverse voltage in the circuit
is above (or close to) its maximum repetitive reverse voltage: a 20 V Schottky on a 24 V rail, a 200 mA diode in a 1 A
path.

**Why it matters.** Schottky diodes in particular have low reverse ratings and a leakage current that rises steeply with
temperature; near the rating they leak, heat, leak more and fail short. Overloaded in the forward direction, small
diodes overheat. Reverse-polarity and OR-ing diodes carry the whole board's current, so a small-signal part chosen
"because it was in the library" is overloaded at once.

**Example.** A BAT54 (Schottky, 30 V VRRM, 200 mA) used as an OR-ing diode between USB and a battery pack on a board
drawing 500 mA: more than twice its rated current. The same diode on a 24 V rail has only 25 % reverse margin, before
transients.

**How to check.** For each diode, take the reverse voltage across it in the worst case from its rails and the path
current from the project (or the downstream loads); compare with VRRM and IF(AV) from the part data, with a margin.

**Fix.** A diode rated for the current and with margin on reverse voltage (Schottkys at 1.5-2× the rail), or an
ideal-diode controller with a FET for higher currents.

**References.** Diodes Inc., BAT54 datasheet DS11005 (VRRM 30 V, IO 200 mA)
([diodes.com](https://www.diodes.com/assets/Datasheets/ds11005.pdf)); SLVA477 §4 (rectifier diode current rating in a
buck converter).

## `RATE-14` TVS clamp above the protected pin's maximum <a id="rate-14"></a>

⚠️ warning · needs: the TVS's clamping voltage, the absolute maximum of the pins on the protected net · [catalogue
entry](../checks.md#rate-14)

**Impl.** mid · depends on NET-7, CFG-5

**What it catches.** A TVS diode whose clamping voltage at the expected surge current is above the absolute maximum of
the parts it is meant to protect.

**Why it matters.** A TVS does not hold the line at its standoff voltage: it starts conducting at its breakdown voltage
and, at the surge current, clamps at a voltage well above it. If that is higher than the protected pins can stand, the
TVS conducts, and the IC fails anyway. A TVS chosen by standoff voltage alone often clamps at one and a half to two
times that voltage.

**Example.** On B200A the VBUS TVS D1605 has the value SMF5V0A but the symbol SMF22A. The 5 V part clamps around 9 V,
acceptable for the 5 V-only input; the 22 V part only starts to conduct above its 22 V standoff, so the 5 V parts on
VBUS behind it would see no protection at all.

**How to check.** For each TVS, take its clamping voltage at the specified surge current from the part data and the
lowest absolute maximum of the pins on its net (`NET-7`); report when the clamp is above it, and value/symbol
disagreements as `BOM-12`.

**Fix.** A TVS with a lower clamping voltage (a standoff just above the rail's maximum), series resistance between the
TVS and the pin to share the surge, or a protected IC with a higher rating.

**References.** Wikipedia, Transient-voltage-suppression diode
([en.wikipedia.org/wiki/Transient-voltage-suppression_diode](https://en.wikipedia.org/wiki/Transient-voltage-suppression_diode));
B200A schematic review notes (D1605 value vs symbol).

## `RATE-15` Inductor saturation current below the converter's peak current <a id="rate-15"></a>

🛑 error · needs: input range, output voltage and current, switching frequency, inductance, the inductor's saturation
current · [catalogue entry](../checks.md#rate-15)

**Impl.** mid · depends on CFG-2, CFG-5, RPT-3

**What it catches.** A switching converter's inductor whose saturation current is below the peak current it must carry:
the maximum output current plus half the ripple, at the maximum input voltage.

**Why it matters.** A saturated inductor loses its inductance, the current rises steeply within one cycle, the converter
hits its current limit or the switch fails; at best the output collapses at full load. The inductor's "rated current"
(thermal) and "saturation current" are different numbers, and the smaller one counts.

**Example.** A TPS62130 buck from 12 V to 3.3 V at 3 A, 2.5 MHz, with 1 µH: per SLVA477 the duty cycle is 3.3 / (12 ×
0.9) ≈ 0.31, the ripple ΔIL = (12 − 3.3) × 0.31 / (2.5 MHz × 1 µH) ≈ 1.06 A, and the peak 3 + 0.53 ≈ 3.5 A. An inductor
with a 3 A saturation current saturates at full load.

**How to check.** Find the converter's input and output rails, output current (project), switching frequency (part data)
and inductor value; compute the peak per SLVA477 equations 1, 2 and 4; compare with the inductor's saturation current
from the MPN (`RPT-3` reports the same numbers).

**Fix.** An inductor with saturation current above the peak with margin (or above the converter's switch current limit,
which also covers faults), or a larger inductance to reduce the ripple.

**References.** SLVA477 §2 (maximum switch current, equations 1-4), §3 (inductor selection); TPS6213x datasheet
(SLVSAG7F) ([ti.com/lit/ds/symlink/tps62130](https://www.ti.com/lit/ds/symlink/tps62130.pdf)); Wikipedia, Saturation
(magnetic) ([en.wikipedia.org/wiki/Saturation_(magnetic)](https://en.wikipedia.org/wiki/Saturation_%28magnetic%29)).

## `RATE-16` Ceramic capacitor DC-bias derating ignored <a id="rate-16"></a>

⚠️ warning · needs: the capacitor's working voltage, its capacitance vs DC bias curve (per MPN), the capacitance the
circuit needs · [catalogue entry](../checks.md#rate-16)

**Impl.** mid · depends on CFG-5, RPT-1

**What it catches.** A class 2 ceramic capacitor (X5R, X7R, Y5V) whose effective capacitance at the working voltage is
below what the circuit needs, because its nominal value was taken at face value.

**Why it matters.** Class 2 dielectrics lose capacitance with DC bias, the more so the smaller the package and the
closer the working voltage is to the rating: drops to −80 % are possible. A regulator output that needs 10 µF for
stability gets 2-3 µF and oscillates; a decoupling bank sized by nominal values is a fraction of what was planned. The
datasheet front page does not show it; the vendor's bias curve does.

**Example.** A 10 µF 6.3 V X5R 0402 on a 5 V rail as an LDO's output capacitor: at 5 V such parts typically keep a small
fraction of their nominal value, so the LDO sees far less than the minimum capacitance in its datasheet. A 10 µF 10 V or
16 V part in 0603 or 0805 keeps much more.

**How to check.** For each class 2 ceramic, take its working voltage (rail) and the bias curve from the part data
(vendors publish them per MPN); compute the effective capacitance; compare it with requirements where they are known
(regulator minimum output capacitance, `RPT-1` totals per rail).

**Fix.** A higher voltage rating or a larger package for the same value, more capacitors, or a C0G or film part where
the value matters.

**References.** Wikipedia, Ceramic capacitor "Voltage dependence of capacitance"; MT-101 (Analog Devices, *Decoupling
Techniques*) on ceramic capacitor choice
([analog.com/.../MT-101.pdf](https://www.analog.com/media/en/training-seminars/tutorials/MT-101.pdf)).

## `RATE-17` Electrolytic or tantalum without derating <a id="rate-17"></a>

⚠️ warning · needs: rail voltage and source impedance, the capacitor's technology and rating · [catalogue
entry](../checks.md#rate-17)

**Impl.** mid · depends on NET-5, CFG-5

**What it catches.** Aluminium electrolytic or tantalum capacitors run close to their rated voltage, and in particular
solid tantalum capacitors without the customary 50 % voltage derating, or placed on a low-impedance rail (a battery, a
strong supply) with no resistance to limit the inrush current.

**Why it matters.** Solid tantalum capacitors fail short, and the failure is often a fire: a defect in the dielectric
heats under current, and on a low-impedance rail the current is limited by nothing. The industry's long-standing rule is
to run them at no more than half their rated voltage; KYOCERA AVX's paper allows 80 % only with lower MTBF and higher
leakage, and recommends limiting the surge current across low-impedance sources. Aluminium electrolytics fail more
gently but age faster near their rating and at high temperature.

**Example.** A 16 V tantalum on a 12 V battery rail: 75 % of rating on a source that can deliver hundreds of amps into a
short. The 50 % rule asks for a 25 V part (or a polymer or ceramic part instead).

**How to check.** Recognise the technology from the MPN (or value field, `TANT`), take the rail's maximum voltage and
report ratings below 2× the rail for solid tantalum (a configurable ratio) and close margins for electrolytics; flag
tantalums directly on battery or connector inputs without series resistance.

**Fix.** Double the voltage rating, use polymer tantalum or ceramic capacitors, or add series resistance or a soft-start
FET in front of tantalums on low-impedance sources.

**References.** AVX derating (abstract: "a minimum of 50% voltage derating should be applied"; conclusion on 80 % use
and low impedance sources); Wikipedia, Tantalum capacitor "Rated and category voltage" and "Surge voltage"
([en.wikipedia.org/wiki/Tantalum_capacitor](https://en.wikipedia.org/wiki/Tantalum_capacitor)).

## `RATE-18` Relay, solenoid or motor current above its driver's rating <a id="rate-18"></a>

⚠️ warning · needs: the load's current (coil resistance and voltage, or project data), the driver's rating, the path
from driver to load · [catalogue entry](../checks.md#rate-18)

**Impl.** mid · depends on CFG-5, CFG-2, NET-10

**What it catches.** An inductive or high-current load (relay coil, solenoid, motor) driven by something rated for less:
a GPIO driving a relay coil directly, a small BJT below the coil current, a driver channel over its per-channel or
package rating.

**Why it matters.** The driver overheats or fails, often slowly: a GPIO driving 70 mA works on the bench and fails in
the field. Motors draw several times their running current at start and stall, and a driver sized for running current
fails at the first blocked rotor.

**Example.** A 5 V relay with a 70 Ω coil (about 71 mA) on an MCU pin rated around 20 mA: the pin's output sags, the
relay may not pull in, and the MCU's IO structure is stressed. A ULN2003A channel (500 mA peak per output) handles it,
but seven channels at 300 mA together exceed what the package can dissipate.

**How to check.** Follow the load back to its driver (`NET-10`: through series resistors and transistors); compute the
load current from the coil resistance and supply or take it from the project; compare with the driver's rating from the
part data (GPIO from `NET-7`, transistor Ic, driver channel and package totals).

**Fix.** A transistor or driver rated for the current (with a freewheeling diode for inductive loads), parallel channels
where the driver allows, or a load that draws less.

**References.** TI ULN200x datasheet (SLRS027T): 500 mA per output, package dissipation
([ti.com/lit/ds/symlink/uln2003a](https://www.ti.com/lit/ds/symlink/uln2003a.pdf)); AoE3 chapter 2 (BJT switches with
inductive loads).

## `RATE-19` Parts on a PD-capable VBUS rated below 22 V <a id="rate-19"></a>

⚠️ warning · needs: VBUS nets of ports that can negotiate PD, the voltage ratings of parts on them · [catalogue
entry](../checks.md#rate-19)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** Capacitors, FETs, TVS diodes, regulators and connectors on a USB-C VBUS that can carry Power
Delivery voltages, rated below 22 V.

**Why it matters.** USB PD's standard power range has fixed supplies up to 20 V; with tolerance and transients VBUS goes
above that. A board that "only uses 5 V" still sees 20 V if its own firmware, a PD controller, a pass-through to another
port or a misbehaving charger negotiates it. Parts chosen for a 5 V VBUS (6.3 V and 10 V capacitors, a 6 V LDO, a 12 V
TVS) fail at the first PD contract.

**Example.** On B153 the receptacle VBUS reaches header pin 10 through Q102; the host can negotiate 9 to 20 V, so the
net was renamed `VBUS_SW` in rev B and Q102's Vds rating was flagged for 20 V. On B200A the assembly notes asked whether
the USB input should have 50 V parts.

**How to check.** Find VBUS nets of USB-C ports whose PD capability (a PD controller on CC, or the project) allows more
than 5 V; list every part on those nets and compare its voltage rating from the part data with 22 V.

**Fix.** Parts rated 25 V or more (capacitors often 35-50 V for margin against bias and transients), or a switch that
disconnects the 5 V-only parts above 5.5 V.

**References.** Wikipedia, USB hardware "USB Power Delivery" (fixed supply voltages up to 20 V); B153A schematic review
notes (pin 10, Q102); B200 rev B change list.

## `RATE-20` Bus termination resistors not rated for a bus fault <a id="rate-20"></a>

⚠️ warning · needs: termination resistors on bus nets, the fault voltage the bus can see, the resistor's power rating ·
[catalogue entry](../checks.md#rate-20)

**Impl.** mid · depends on CFG-2, CFG-5

**What it catches.** RS-485 or CAN termination resistors (120 Ω, or 2 × 60 Ω split) sized for normal operation only,
which a bus line shorted to the supply would overload.

**Why it matters.** In normal operation a termination resistor sees a few volts and a few tens of milliwatts, so 0402 or
0603 parts look fine. Field buses run next to 12 V or 24 V supply wires; a miswire or a crushed cable shorts a bus line
to the supply, and the resistor then carries the transceiver's short-circuit current with the supply voltage across it,
watts for as long as the fault lasts. The transceiver may be rated to survive that; the resistor burns, the node stays
off the bus or the board chars.

**Example.** On B125A the 120 Ω termination R77 was soldered for the PoE setup (assembly report), and the rev B change
list asks for a higher power rating for the termination resistor. TI's CAN guide puts it plainly: a bus-line short to a
24 V supply drives the transceiver's short-circuit current through the termination, and a 1/4 W resistor "would blow up
in this circumstance".

**How to check.** Recognise terminations (a resistor of about 120 Ω or two of about 60 Ω across a differential pair at a
transceiver); take the fault voltage from the project (the highest supply in the same cable) and the transceiver's
short-circuit current from its part data; compare V × I, or V² / R, with the resistor's rating.

**Fix.** A termination in a package rated for the fault (often a 2010 or 2512 part), split termination with two larger
resistors, or a bus-fault-protected transceiver with a fused or switched termination.

**References.** TI SLLA270, *Controller Area Network Physical Layer Requirements*, §4.5 "Line Terminations"
([ti.com/lit/an/slla270](https://www.ti.com/lit/an/slla270/slla270.pdf)); TI SLLA272, *The RS-485 Design Guide*, Rev. D,
§6 "Bus Termination and Stub Length" ([ti.com/lit/an/slla272](https://www.ti.com/lit/an/slla272/slla272.pdf)); B125 rev
B change list.

## `RATE-21` Linear regulator dissipation vs its package <a id="rate-21"></a>

⚠️ warning · needs: input and output voltage, output current, the package's thermal resistance · [catalogue
entry](../checks.md#rate-21)

**Impl.** mid · depends on RPT-2, CFG-2, CFG-5

**What it catches.** A linear regulator whose dissipation, (Vin − Vout) × Iout, raises its junction above the maximum
temperature for its package and board: a SOT-23 at about 250 °C/W cannot dissipate 0.3 W at room temperature, let alone
in a warm box.

**Why it matters.** LDOs are chosen by current and dropout; the heat is forgotten. An LDO dropping 5 V to 1.8 V at 100
mA burns 0.32 W, and in a SOT-23 that is about 80 °C above ambient. The regulator goes into thermal shutdown and the
rail cycles on and off, or the part runs at the edge of its rating for its whole life.

**Example.** For B200A's planned adjustable IO supply (an LDO from VBUS), the worst case of about 14 outputs shorted
through their 255 Ω at 1.8 V is about 90 mA, roughly 0.3 W from 5 V: the TLV702's SOT-23 (RθJA 249.2 °C/W) would rise
about 75 °C, too much; the B200 change list asks for SOT-223 or a DFN with a pad.

**How to check.** Take Vin (the input rail's maximum), Vout and the maximum output current (`RPT-2` or the project);
compute P = (Vin − Vout) × Iout plus Vin × Iq; multiply by RθJA from the part data for the package and add the board's
maximum ambient; report above the maximum junction temperature (with margin).

**Fix.** A package with lower thermal resistance and copper under it, a lower input voltage (a buck first), or a
switching regulator.

**References.** TLV702 datasheet §6.4 "Thermal Information" (RθJA 249.2 °C/W for SOT-23); SLVA118 §3 (power
dissipation), §4 (thermal equations); SPRA953 §1; B200 rev B change list (adjustable LDO, heat).

## `RATE-22` Zener power at the maximum current its series resistor allows <a id="rate-22"></a>

⚠️ warning · needs: the zener's voltage and power rating, the series resistor, the highest voltage at its input ·
[catalogue entry](../checks.md#rate-22)

**Impl.** mid · depends on NET-5, CFG-2, CFG-5

**What it catches.** A zener diode used as a clamp or a shunt regulator whose dissipation, at the highest input voltage
and with the series resistor as the only limit, exceeds its power rating.

**Why it matters.** A zener clamp is sized for the normal signal and forgotten for the fault: when the input goes to the
maximum the connector can see, the zener carries (Vmax − Vz) / R and dissipates Vz times that. Small zeners (SOD-323,
SOD-523) are rated 200-300 mW; beyond that they fail, often short, which pulls the protected line to ground for good.

**Example.** B153A's IO lines go through 255 Ω to a 5.6 V zener at the MCU side, with a 12 V TVS at the connector. If a
line is held at 12 V, the zener carries (12 − 5.6) / 255 ≈ 25 mA and dissipates about 0.14 W, continuously: within a
typical 200 mW SOD-323 zener's rating at room temperature but with little margin when the board is warm, and four lines
faulted at once put their heat into one corner of the board.

**How to check.** For each zener with a series resistor, take the highest voltage the input side can see (the TVS clamp,
the connector's rated maximum, the supply in the same cable) and compute the current and power; compare with the zener's
rating from the part data, derated for temperature.

**Fix.** A larger series resistor, a zener in a bigger package, or a clamp to a rail through a Schottky where the rail
can absorb the current.

**References.** Wikipedia, Zener diode ([en.wikipedia.org/wiki/Zener_diode](https://en.wikipedia.org/wiki/Zener_diode));
B153A schematic review notes (IO protection: 255 Ω, 5V6 zener, 12 V TVS); AoE3 chapter 9.

## `RATE-23` Optocoupler LED current vs its CTR at the load <a id="rate-23"></a>

⚠️ warning · needs: LED current from the drive, the CTR (minimum, over life and temperature), the output's required
current · [catalogue entry](../checks.md#rate-23)

**Impl.** mid · depends on NET-5, NET-7, CFG-5

**What it catches.** An optocoupler whose LED current is too low for its output to pull the load hard enough (output
current = LED current × CTR, at the CTR's minimum), or too high for the LED's maximum forward current.

**Why it matters.** The current transfer ratio of phototransistor optocouplers varies widely from part to part (binned
ranges spanning several times) and falls with LED ageing and temperature. A design that works with a typical-CTR part on
the bench fails with a low-bin part, or after years of operation. Driving the LED harder ages it faster; driving it from
a GPIO limits the current.

**Example.** A phototransistor optocoupler driven from a 3.3 V GPIO through 1 kΩ (LED forward voltage about 1.2 V):
about 2 mA. With a minimum CTR of 50 % the output sinks only 1 mA, so a 1 kΩ pull-up to 5 V on the output (5 mA needed
to pull it fully low) does not give a valid low. Photo-MOS relays such as the ISOM8600 on B153A's relay output (PA5
through R106 1 kΩ) have a trigger current instead of a CTR: the same calculation against the datasheet's maximum trigger
current.

**How to check.** Compute the LED current from the drive voltage (GPIO high level from `NET-7`), forward voltage and
series resistor; the output current needed from the pull-up and supply on the output side; compare with the minimum CTR
(or trigger current) from the part data, and the LED current with its maximum.

**Fix.** More LED current within its rating, a larger output pull-up, a part with a higher CTR bin, or a digital
isolator where speed and lifetime matter.

**References.** Wikipedia, Opto-isolator (current transfer ratio by type)
([en.wikipedia.org/wiki/Opto-isolator](https://en.wikipedia.org/wiki/Opto-isolator)); B153A schematic review notes
(relay: PA5, R106, U101).

## `RATE-24` LED reverse voltage exceeded <a id="rate-24"></a>

⚠️ warning · needs: the voltage across the LED in reverse, its reverse voltage rating · [catalogue
entry](../checks.md#rate-24)

**Impl.** mid · depends on NET-5, CFG-5

**What it catches.** An LED that can see more reverse voltage than its rating: across an AC signal, on a supply that can
be reversed, or in an anti-parallel arrangement where the other branch does not clamp low enough.

**Why it matters.** LEDs have a reverse voltage rating of about 5 V and are not meant to block; above it they leak and
degrade, and on a reversed 12 V or 24 V supply they fail. Indicators on connector power pins (a "power OK" LED on an
input that can be connected backwards) are the usual case.

**Example.** A power indicator, an APT1608SGC (reverse voltage 5 V maximum) with a 2.2 kΩ resistor, across a 24 V input
that is not reverse-protected: wired backwards, the LED sees almost the full 24 V in reverse.

**How to check.** For each LED, find the reverse voltage it can see: the rails at its ends with the supply reversed (for
inputs the project marks as reversible) or an AC signal's peak; compare with the reverse voltage rating from the part
data (default 5 V).

**Fix.** A diode in series or anti-parallel with the LED, reverse protection on the input, or a bicolour/anti-parallel
LED pair for AC indication.

**References.** Kingbright APT1608SGC datasheet, absolute maximum ratings (reverse voltage 5 V)
([kingbrightusa.com](https://www.kingbrightusa.com/images/catalog/SPEC/APT1608SGC.pdf)); Wikipedia, Light-emitting diode
"Electrical polarity".
