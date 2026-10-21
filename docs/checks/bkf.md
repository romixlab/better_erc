# BKF checks, explained

Detail page for the `BKF` category of the [check catalogue](../checks.md#bkf-1). Each section explains one check:
what it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the usual
fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID, severity and
wording; this page is the commentary.

Backfeed is current flowing into a power domain that is meant to be off, through a path that was never drawn as a
supply: input clamp diodes, MOSFET body diodes, pull-up resistors, a regulator run backwards. The off domain then
sits at some undefined voltage, its parts start half-way and latch up or hold a bus, and the domain that feeds it
carries a load it was not designed for. Nearly every check here needs to know which rails can be off while others
are on: the power states of `NET-8`.

References used throughout:

- **SZZA036** · TI, *Understanding and Interpreting Standard-Logic Data Sheets*, SZZA036C (2002, rev. 2016).
  [ti.com/lit/an/szza036c](https://www.ti.com/lit/an/szza036c/szza036c.pdf)
- **SSZTAP0** · TI, *Logic Gates and Switches with Ioff or Powered-off Protection: Empowering You to Power Down*,
  technical article SSZTAP0 (2016). [ti.com/lit/pdf/ssztap0](https://www.ti.com/lit/pdf/ssztap0)
- **SLAA689** · TI, *ESD Diode Current Specification*, SLAA689B (2015, rev. 2021).
  [ti.com/lit/pdf/slaa689](https://www.ti.com/lit/pdf/slaa689)
- **AN10441** · NXP, *Level shifting techniques in I²C-bus design*, Rev. 2 (2020). nxp.com no longer serves it at
  its old address; fetched from Mouser's copy through `tpm fetch`. [mouser.com/pdfDocs/AN10441.pdf](https://www.mouser.com/pdfDocs/AN10441.pdf)
- **TXS0102** · TI, *TXS0102 2-Bit Bidirectional Voltage-Level Translator for Open-Drain and Push-Pull
  Applications*, SCES640L (2007, rev. 2026). [ti.com/lit/ds/symlink/txs0102.pdf](https://www.ti.com/lit/ds/symlink/txs0102.pdf)
- **SLUA962** · TI, *Methods to Eliminate Damage Caused by Reverse Current in Synchronous Buck Converters*, SLUA962
  (2019). [ti.com/lit/pdf/slua962](https://www.ti.com/lit/pdf/slua962)
- **SSZT658** · TI, *LDO Basics: Preventing Reverse Current*, technical article SSZT658 (2018).
  [ti.com/lit/pdf/sszt658](https://www.ti.com/lit/pdf/sszt658)
- **TPS7A02** · TI, *TPS7A02 datasheet*, SBVS277C (2022). [ti.com/lit/ds/symlink/tps7a02.pdf](https://www.ti.com/lit/ds/symlink/tps7a02.pdf)
- **SLVAE57** · TI, *Basics of Ideal Diodes*, SLVAE57B (2021). [ti.com/lit/an/slvae57](https://www.ti.com/lit/an/slvae57/slvae57.pdf)
- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, AN4899 Rev 4
  (2026; fetched through `tpm fetch`). [st.com/resource/en/application_note/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-configuration-for-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **USB 2.0** · USB-IF, *Universal Serial Bus Specification*, Revision 2.0 (`usb_20.pdf` in the specification zip).
  [usb.org/document-library/usb-20-specification](https://www.usb.org/document-library/usb-20-specification)

## `BKF-1` Power backfeed through IO <a id="bkf-1"></a>

🛑 error · needs: power states (`NET-8`), pin data (Ioff) · [catalogue entry](../checks.md#bkf-1)

**Impl.** hard · depends on NET-8, NET-7

**What it catches.** An output of a powered domain driving an input of a domain that can be off, where the input has no
power-off protection; the suggestion is a buffer whose inputs and outputs are rated for that (Ioff, partial power down).

**Why it matters.** Most CMOS inputs have a clamp diode to their own VCC. With that VCC off, a high on the input
forward-biases the diode and the driver supplies the whole dead domain through it: the rail rises to the high level
minus a diode drop, parts on it start in an undefined state, and the driving pin sources milliamps it was never sized
for. When the rail is switched on again, a part that started half-way may not reset.

**Example.** An MCU's UART TX into a GPS module on a switched `+3V3_GPS` rail: with the GPS off, TX idles high and lifts
the GPS rail to up to a diode drop below 3.3 V through the module's RX clamp. The module never sees a clean power-on and
comes up without a fix until the whole board is power-cycled.

**How to check.** For each power state (`NET-8`), nets that join a driving pin in an on domain to an input pin in an off
domain, where the input's part has no Ioff rating (`NET-7`, library).

**Fix.** A buffer with Ioff (74LVC1G125, 74LVC1G07 class) powered from the receiving side's rail, or drive the pin low
(or high-impedance) before the rail goes off (`LEAK-2`).

**References.** SZZA036 §4.6.20 "Ioff Input/Output Power-Off Leakage Current" (and the families that have it), §4.4.2
"Input Voltage, VI" (clamp diodes to VCC); SSZTAP0; SLAA689 §1 "Important Side Effects of the ESD Diodes During Power
Cycling".

## `BKF-2` General case: IO across two power domains <a id="bkf-2"></a>

🛑 error · needs: power states (`NET-8`), each pin's supply domain · [catalogue entry](../checks.md#bkf-2)

**Impl.** hard · depends on NET-8, NET-7

**What it catches.** Any net that joins pins of two power domains, in any power state where one domain is on and the
other off, regardless of which side drives: the general form of `BKF-1`, including bidirectional pins, pulls and
passives.

**Why it matters.** The protection diodes of the off side do not care about pin direction. Whatever holds the net high
(an output, a pull-up, a bus) feeds the off rail through them, limited only by the series resistance. On the off side
the rail creeps up to a level where some parts run and others do not.

**Example.** On B200A the analog buffer's supply `+5V0_FLT` had no source at all (a ferrite from `+5V0_USBIN` went
missing). The eight followers' inputs sit on the IO, UART RX and I2C lines through 255 Ω to 1.25 kΩ, so whatever the
device under test drove high powered the two TLV9004s through their input clamps.

**How to check.** For each power state, nets with pins of an on domain and an off domain; report the path (pin, series
resistance) and the current it allows.

**Fix.** Keep the two domains on together, isolate with Ioff buffers or switches, or add enough series resistance that
the clamp current stays below what the off rail's leakage drains (and below the part's injection limit).

**References.** SLAA689 §1, §3.1.2 "Power Supplies With Limited Current-Sinking Capability" (injected current lifts the
rail); AN4899 §6.2 "Cross-voltage domains leakage", §6.3 "Voltage protection when no VDD is supplied"; B200A review
notes §0 (`+5V0_FLT` without a source).

## `BKF-3` Backfeed through a MOSFET body diode in a translator <a id="bkf-3"></a>

🛑 error · needs: power states, translators and their FET orientation · [catalogue entry](../checks.md#bkf-3)

**Impl.** hard · depends on NET-8, NET-4, NET-10

**What it catches.** A discrete or integrated pass-FET translator (BSS138 pair, PCA9306 class) whose body diode conducts
from the powered side into the off side's rail through that side's pull-ups.

**Why it matters.** The BSS138 shifter isolates a powered-down *low* side well: gate at 0 V, FET off, body diode
reverse-biased. It does not isolate a powered-down *high* side: the low side's pull-up drives the source high, the body
diode (source to drain in an N-channel FET) conducts into the high-side bus, and the high-side pull-up carries that
current into the dead rail.

**Example.** A 3.3 V MCU with a BSS138 shifter to a 5 V sensor bus whose `+5V_SENS` rail is switched off: SDA idles high
on the 3.3 V side, so the 5 V bus and, through its 4.7 kΩ pull-ups, the sensor rail are lifted towards 3 V.

**How to check.** For each translator (`NET-4`) and each power state, the side whose rail is off; with an N-channel pass
FET, a backfeed exists when the off side is the drain (high) side and the source side has a pull-up to an on rail.

**Fix.** Switch the pull-ups with the rail they belong to, use a translator with Ioff on both ports, or keep the
high-side rail on whenever the low side is.

**References.** AN10441 §2.1 "Connecting devices with different logic levels" (the drain-substrate diode), §2.1.1
"Operation of the level shifter"; catalogue `I2C-18`, `I2C-19`.

## `BKF-4` Backfeed through MOSFETs in DC-DC converters <a id="bkf-4"></a>

🛑 error · needs: power states, the regulators and what else drives their outputs · [catalogue entry](../checks.md#bkf-4)

**Impl.** hard · depends on NET-8, NET-5

**What it catches.** A synchronous converter whose output can be held up by another source (a battery, a second supply
ORed in, a large capacitor) while its input is off or falling.

**Why it matters.** The high-side FET's body diode connects the output to the input: a buck with a live output and a
dead input powers its own input rail, and everything else on that rail, through the diode. With the controller still
switching as VIN falls, the converter can also run backwards (current flows from output to input through the low-side
FET) and the reverse current can destroy the low-side FET.

**Example.** A 12 V → 5 V synchronous buck whose 5 V output is joined straight to a USB 5 V input, with no diode or
switch between them. Unplug the 12 V supply with USB connected, and the 12 V rail (with its fan, its relay coil and the
input of another regulator) rises to about 4.3 V through the buck's high-side body diode.

**How to check.** For each regulator, other sources that reach its output net (`NET-5`); a finding when a power state
has the output fed and the input off, unless the regulator's data says it blocks reverse current.

**Fix.** An ideal diode or a diode on the shared output, a converter with reverse-current blocking, or an input UVLO (EN
divider) set above the output voltage so the converter stops before VIN falls to VOUT.

**References.** SLUA962 §2 "Reverse Current Forming Mechanism", §3 "Low-side MOSFET Failure Mechanism", §4 "Solutions"
(input UVLO above VOUT); SLVAE57 §8 "ORing Power Supplies with Ideal Diode Controllers".

## `BKF-5` Backfeed through LDOs <a id="bkf-5"></a>

⚠️ warning · needs: power states, regulators, reverse-current data · [catalogue entry](../checks.md#bkf-5)

**Impl.** hard · depends on NET-8, NET-5

**What it catches.** An LDO whose output can be higher than its input in some state: the output is held up by a backup
source, by an external supply on a connector, or by a large output capacitance while the input collapses.

**Why it matters.** A PMOS pass element has a body diode from output to input; with VOUT above VIN it conducts and the
output feeds the input rail. TI limits the reverse current of parts like the TPS7A02 to 5 % of the rated current; many
LDOs have no blocking at all. The rail upstream sees the LDO's load, and the LDO can be damaged.

**Example.** On B200A the `+VREF` rail can be fed from outside on J103 pin 7 (the "Ext" level) while the board's own
sources are off. The rev B idea of an adjustable LDO for `+VREF` therefore needs either the existing back-to-back PMOS
pair behind it or an LDO that blocks reverse current with its enable off: a plain LDO would let the external level feed
back into the board's supply.

**How to check.** LDOs whose output net has another source or a connector pin (`NET-5`, project data); report the states
where VOUT can exceed VIN, unless the part has reverse-current blocking.

**Fix.** An LDO with reverse-current protection, a Schottky in series with the output or input, or a back-to-back
switch.

**References.** SSZT658 (reverse current through the pass FET's body diode; Schottky, second FET); TPS7A02 §7.3.2
"Active Discharge" (reverse current limited to 5 % of the rated current); B200 rev B change list (adjustable LDO for
`+VREF`).

## `BKF-6` Backfeed through internal IC structure <a id="bkf-6"></a>

⚠️ warning · needs: per-part knowledge of internal paths between supply pins · [catalogue entry](../checks.md#bkf-6)

**Impl.** judgement · depends on CFG-5

**What it catches.** Paths inside a part between two of its supply or IO domains that the datasheet documents beyond
plain IO protection diodes: supply ordering rules, internal diodes between VCCA and VCCB, a VIO that powers part of the
core, a battery pin that feeds the main supply.

**Why it matters.** These are written in a datasheet's power-supply or layout section in prose, not in a table, so they
are invisible to a generic check. When the board breaks such a rule, the part feeds one supply from the other or starts
in an undefined state, and the failure looks like a part fault.

**Example.** A TXS0102 requires VCCA ≤ VCCB and asks for OE to be held low until both rails are stable. Wired the other
way round (VCCA = 3.3 V, VCCB = 1.8 V, because the MCU happened to sit on the A side), the part is outside its rules and
nothing in the netlist says so.

**How to check.** Part rules in the database (`CFG-5`), each a condition on the supply nets of that part; better_erc can
list parts with several supply domains as candidates, the rule itself comes from reading the datasheet.

**Fix.** Wire the part as its datasheet requires; where it cannot be, isolate the domain or sequence the rails.

**References.** TXS0102 §8.3 "Power Supply Recommendations" (VCCA ≤ VCCB, OE held low until both supplies are stable).

## `BKF-7` Power through IO or I2C lines <a id="bkf-7"></a>

⚠️ warning · needs: pin roles (supply vs signal), power states · [catalogue entry](../checks.md#bkf-7)

**Impl.** hard · depends on NET-8, NET-10

**What it catches.** A part that can only get its power through its signal pins: its supply pin unconnected, on a net
with no source, or on a rail that is off while its signals are pulled up. 1-Wire parts in parasite-power mode do this on
purpose and are excluded.

**Why it matters.** A part powered through its clamps works, mostly: the bench test passes because the pull-ups deliver
enough current at 25 °C. It fails when the bus is low for long (the part browns out mid-transfer), when it draws a burst
of current, or at temperature.

**Example.** A 24LC32 EEPROM whose VCC pin was left on a net named `VCC` that nothing else drives (`WIRE-2`): it reads
back fine on the bench through the 4.7 kΩ SDA/SCL pull-ups and corrupts writes in the field. A DS18B20 with its VDD tied
to GND is the intended exception (parasite power).

**How to check.** Parts whose supply pins reach no source in some power state while one of their signal pins is pulled
or driven high; exclude parts flagged as parasite-powered.

**Fix.** Connect the supply; or for 1-Wire, decide on parasite power on purpose with a strong pull-up.

**References.** SLAA689 §3.1.2; SZZA036 §4.4.6 "Input Clamp Current, IIK"; catalogue `WIRE-2`.

## `BKF-8` I2C device on a rail that can be off <a id="bkf-8"></a>

⚠️ warning · needs: power states, the I2C buses · [catalogue entry](../checks.md#bkf-8)

**Impl.** hard · depends on NET-8, NET-4

**What it catches.** A device on an I2C bus whose supply rail can be off while the bus is on. The backfeed checks
(`BKF-2`) find the same path in general; this is the bus-specific form with the I2C consequences.

**Why it matters.** The off device's SCL/SDA clamps hold both lines at about one diode drop above its dead rail, so the
whole bus stays low and every other device becomes unreachable: an off sensor kills the always-on RTC. The I2C page has
the full story under `I2C-11`.

**Example.** A board-ID EEPROM on a switched `+3V3_PERIPH` rail sharing the bus with the always-on RTC and PMIC: with
peripherals off to save power, the RTC alarm can no longer be read or set.

**How to check.** Devices on each bus (`NET-4`) whose rail is off in a state where the pull-up rail is on, unless their
bus pins are fail-safe.

**Fix.** Move the device to an always-on rail, or behind a bus switch or buffer that opens with its rail.

**References.** Catalogue `I2C-11` and its [detail section](i2c.md#i2c-11); SZZA036 §4.6.20 (Ioff parts are the
exception).

## `BKF-9` Connector pins an external device can drive while the board is unpowered <a id="bkf-9"></a>

🛑 error · needs: connector pins facing other devices, paths behind them · [catalogue entry](../checks.md#bkf-9)

**Impl.** hard · depends on NET-10, NET-7, CFG-2

**What it catches.** Pins on a connector that the device on the other side can drive while this board has no power, with
a path from the pin into one of this board's rails: op-amp and MCU input clamps, the body diode of a pull-up PMOS, ESD
diodes to a rail.

**Why it matters.** A test adapter, a debug probe or a sensor box is powered on its own and does not know whether the
board it connects to is. Each path powers part of the board through a pin: the board half-starts, the external device
sees a load on its signals, and the clamps may carry more current than they are rated for.

**Example.** B200A, the rePort, must not be powered by the device under test (a datasheet target). The schematic review
found these paths: the analog buffer's input clamps on IO0-IO3 (255 Ω), RX, SDA and SCL (1.25 kΩ) into `+5V0_FLT`; the
body diode of each IO pull-up PMOS into `+VREF_IO` (through 10 kΩ); and SYNC and ADPT_KIND straight to MCU pins through
255 Ω into 3.3 V.

**How to check.** From each external connector pin (project data says which connectors face other devices), follow
series resistors and switches (`NET-10`) to pins with clamp diodes or FET body diodes to a rail (`NET-7`); report each
path with its series resistance.

**Fix.** Op-amps and buffers with powered-off-safe inputs (Ioff, over-voltage tolerant), large series resistance,
pull-up switches built from back-to-back FETs so no body diode reaches the rail.

**References.** SSZTAP0; SZZA036 §4.6.20, §4.7 "Live-Insertion Specifications"; SLVAE57 §6.6.1 "Back-to-Back FET Driving
Ideal Diode Controllers"; B200A review notes §6, §7 and the B200 rev B change list ("no backfeed while unpowered").

## `BKF-10` Level shifter whose pull-ups feed an unpowered target <a id="bkf-10"></a>

⚠️ warning · needs: translator type (internal pull-ups), the target side · [catalogue entry](../checks.md#bkf-10)

**Impl.** mid · depends on NET-4, NET-7, CFG-2

**What it catches.** A translator or buffer with internal pull-ups (TXS010x class: 10 kΩ per line, on while OE is high)
whose target side faces a device that may be off while the translator is powered and enabled.

**Why it matters.** The pull-ups go to the translator's own supply for that side, which this board powers. With the
target off, each line is held high through 10 kΩ into the target's input clamps: the target is fed through its signal
pins.

**Example.** On B153A the TXS0102 U203 connects the MCU to the device under test on the Type-C SBU lines, A side on
`V_SBU`. With OE high, its internal pull-ups hold the target's SWD lines high and feed an unpowered target (and they
load the ADC inputs on the same lines). The rev B change list proposes push-pull translators with a DIR per line
instead.

**How to check.** Translators with internal pull-ups (`NET-7`, library) whose side towards an external connector
(project data) is powered by this board; report with the OE net and its default level.

**Fix.** Keep OE low while the target is off (the TXS0102 disables its pull-ups with OE low), or use a translator
without internal pull-ups and with Ioff.

**References.** TXS0102 §7.3.5 "Pullup or Pulldown Resistors on I/O Lines" (internal 10 kΩ, disabled with OE low),
§7.3.4 "Enable and Disable", §5.5 (Ioff); B153 rev B change list ("Rethink the SBU level shifter").

## `BKF-11` USB VBUS joined to a system rail <a id="bkf-11"></a>

⚠️ warning · needs: power sources of the rail, USB connector roles · [catalogue entry](../checks.md#bkf-11)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** A USB connector's VBUS tied straight to a system rail that has another source (a barrel jack, a
battery boost, a second USB port), with no diode, ideal diode or switch between them, or a board that can act as host on
a port wired as a device.

**Why it matters.** USB 2.0 forbids a device from sourcing current on VBUS at its upstream port at any time. A board
that feeds its own 5 V into the host's VBUS fights the host's supply when both are on and back-powers the host port when
the host is off; some hosts detect it and shut the port down, others are damaged. The D+ pull-up must also come from
VBUS, so a self-powered device does not signal "attached" to a host that is off.

**Example.** A data logger powered from a 5 V barrel jack with the USB-C receptacle's VBUS tied to the same `+5V`:
plugged into a laptop, both supplies drive VBUS; with the laptop asleep, the logger powers the laptop's port.

**How to check.** VBUS pins of USB connectors (`NET-5`) whose net has another source; ports declared as host or
dual-role in the project need a switch either way.

**Fix.** An ideal-diode or load switch between VBUS and the rail (or a power-path controller), VBUS sensing through a
divider for "attached", and the D+ pull-up derived from VBUS.

**References.** USB 2.0 §7.2.1 "Classes of Devices" ("No device shall supply (source) current on VBUS at its upstream
facing port at any time"), §7.1.5.1 (D+/D− pull-up powered from VBUS); SLVAE57 §5 "Reverse Polarity Protection vs
Reverse Current Blocking".

## `BKF-12` Pull-up from a rail that can be off onto a bus that stays on <a id="bkf-12"></a>

⚠️ warning · needs: power states, pull-ups and their rails · [catalogue entry](../checks.md#bkf-12)

**Impl.** hard · depends on NET-8

**What it catches.** A pull-up from a switchable rail onto a net that stays in use while that rail is off: a shared
interrupt or reset line, an I2C bus, a chip select.

**Why it matters.** With its rail off, the pull-up turns into a load: the line's high feeds the dead rail through the
resistor and whatever is on that rail, so the line drags towards the rail's resting voltage. An open-drain line with its
only pull-up on the dead rail simply reads low.

**Example.** An open-drain `INT_N` shared by an always-on PMIC and a sensor, pulled up with 10 kΩ to the sensor's
switched `+3V3_SENS`: with sensors off, the line reads low, and the MCU, which wakes on `INT_N` low, never sleeps.

**How to check.** For each power state, nets used by on domains whose pull-ups (or only pull-up) go to an off rail.

**Fix.** Pull up to an always-on rail (the one of the line's receiver), and isolate the switched devices.

**References.** AN4899 §6.2 "Cross-voltage domains leakage"; catalogue `I2C-19` and its [detail section](i2c.md#i2c-19).
