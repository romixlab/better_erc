# DBG checks, explained

Detail page for the `DBG` category of the [check catalogue](../checks.md#-dbg---debug-access-indicators-rework-options).
Each section explains one check: what it catches, why the board is harder to bring up or service without it, a
concrete example, what better_erc needs to find it, the usual fix, and where to read more. The one-line entries in
the catalogue stay the source of truth for ID, severity and wording; this page is the commentary.

Most of these are suggestions: a board without them works, but the first bring-up, the first field return and every
firmware bug that hangs the part cost more. The **Impl.** line says how hard the check is for better_erc in pure Rust:
`easy` (a query on the netlist), `mid` (part data or a project setting first), `hard` (analysis the code does not
have yet), `judgement` (intent, so better_erc can at most list candidates).

References used throughout:

- **SEGGER 20-pin** · SEGGER Knowledge Base, *20-pin J-Link Connector* (pin table: VTref, nTRST, TMS, TCK, nRESET,
  pin 19 5V-Supply). [kb.segger.com/20-pin_J-Link_Connector](https://kb.segger.com/20-pin_J-Link_Connector)
- **SEGGER 9-pin** · SEGGER Knowledge Base, *9-pin JTAG/SWD connector* (the 10-pin Cortex-M header, 1.27 mm).
  [kb.segger.com/9-pin_JTAG/SWD_connector](https://kb.segger.com/9-pin_JTAG/SWD_connector)
- **AN4989** · ST, *Introduction to debug toolbox for STM32 MCUs* (fetched 21 Oct 2026 through `tpm fetch`, st.com
  refuses plain HTTP clients). §4.1 SWD/JTAG pinout, §4.2 Reset and connection mode, §7.2 Printf via UART, §7.3
  Printf via SWO/SWV. [st.com AN4989](https://www.st.com/resource/en/application_note/an4989-stm32-microcontroller-debug-toolbox-stmicroelectronics.pdf)
- **AN5096** · ST, *Getting started with STM32G0 MCUs hardware development*, Rev 4 (fetched through `tpm fetch`).
  §1.2.2 Programmable voltage detector (PVD), §4.3 Pinout and debug port pins.
  [st.com AN5096](https://www.st.com/resource/en/application_note/an5096-getting-started-with-stm32g0-series-hardware-development-stmicroelectronics.pdf)
- **XJTAG DFT** · XJTAG, *Design for Test (DFT) Guidelines for improving JTAG testability* (TAP pull-ups and
  pull-downs, termination, connector design).
  [xjtag.com/about-jtag/design-for-test-guidelines](https://www.xjtag.com/about-jtag/design-for-test-guidelines/)
- **RP2350 HW** · Raspberry Pi, *Hardware design with RP2350*, §5.3 Debug connector.
  [datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf)
- **UM10204** · NXP, *I²C-bus specification and user manual*, Rev. 7.0 (October 2021), §3.1.16 Bus clear (fetched
  through `tpm fetch`). [nxp.com/docs/en/user-guide/UM10204.pdf](https://www.nxp.com/docs/en/user-guide/UM10204.pdf)
- **Wikipedia, JTAG** · a readable overview of the TAP signals and boundary scan.
  [en.wikipedia.org/wiki/JTAG](https://en.wikipedia.org/wiki/JTAG)
- **IEEE 1149.1** · *IEEE Standard for Test Access Port and Boundary-Scan Architecture* (the JTAG standard), by title.
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 9 (power regulation),
  chapter 13 (digital meets analog).

## `DBG-1` SWD/JTAG not connected <a id="dbg-1"></a>

🛑 error · needs: debug pins of each MCU (pin names, `NET-12`), header or test points · [catalogue entry](../checks.md#dbg-1)

**Impl.** easy · depends on netlist (`NET-12` for symbols that name only the port pin)

**What it catches.** An MCU, SoC or FPGA whose SWD or JTAG pins (SWDIO, SWCLK; TCK, TMS, TDI, TDO) reach no
header, connector or test point at all: they are unconnected, or wired only to other on-board functions.

**Why it matters.** The debug port is the one interface that works whatever the firmware does: it flashes a blank
part, recovers a bricked one and shows where a hang sits. Without it the board depends on a ROM bootloader (if the
part has one and its interface is reachable, `BOOT-6`) and every firmware bug is debugged with printf. Many parts
ship blank and can only be programmed through it.

**Example.** An STM32G030 board where PA13 (SWDIO) drives a status LED and PA14 (SWCLK/BOOT0) a button, with no
header and no pads: the factory-blank part can only be programmed by soldering wires to the LED resistor and the
button, and once the firmware turns the pins into GPIO even that stops working (`MUX-5`).

**How to check.** Find each part's debug pins by name (`SWDIO`, `SWCLK`, `TCK`, `TMS`, `TDI`, `TDO`, or the
port pin they share once `NET-12` maps alternate functions) and follow their nets through series resistors; at least
SWDIO, SWCLK and GND (or the four JTAG signals) must reach a connector or test points.

**Fix.** A debug header (ARM 10-pin 1.27 mm, a Tag-Connect footprint, or at least test points for SWDIO, SWCLK,
NRST, VTref and GND); keep the pins free of other functions, or share them only through resistors (`DBG-2`).

**References.** AN4989 §4.1 (SWD/JTAG pinout); AN5096 §4.3 (SWD pins and connector); SEGGER 9-pin; RP2350 HW §5.3.

## `DBG-2` No series resistors on an obvious debug header (e.g. SWD) <a id="dbg-2"></a>

💡 suggestion · needs: debug header recognised from its nets, series resistors · [catalogue entry](../checks.md#dbg-2)

**Impl.** easy · depends on netlist

**What it catches.** SWD or JTAG lines going straight from the MCU pins to a header, with no series resistor in
between.

**Why it matters.** A debug header is the board's most handled connector: probes are plugged in hot, cables are
swapped, ground is sometimes connected last. A 22-100 Ω series resistor limits the current of an ESD event or a
wrong-voltage probe into the pin, damps ringing on a long ribbon cable at several MHz, and gives a footprint to
cut the line or inject a signal during debug. XJTAG recommends a series resistor (typically 22 Ω) on TDO for the
same signal-integrity reason.

**Example.** A 15 cm ribbon cable between a J-Link and a 10-pin header at 4 MHz SWD: overshoot on SWCLK double
clocks the target, the connection drops until the speed is lowered. 33 Ω at the MCU end cures it.

**How to check.** Nets of the debug pins (`DBG-1`) that reach a header pin with no resistor in the path.

**Fix.** 22-100 Ω in SWCLK and SWDIO (TCK, TMS, TDI, TDO) near the MCU; keep NRST and VTref without one
(`DBG-19`).

**References.** XJTAG DFT "TDO Termination", "TCK Termination"; Johnson & Graham, *High-Speed Digital Design*,
chapter 6 (terminations).

## `DBG-3` No LEDs on the board at all: suggest at least one for debug <a id="dbg-3"></a>

💡 suggestion · needs: part classes (LED) · [catalogue entry](../checks.md#dbg-3)

**Impl.** easy · depends on netlist

**What it catches.** A board with an MCU and not a single LED.

**Why it matters.** An LED is the cheapest output a board has: it shows "alive" before any UART, probe or
software on the PC works, it tells a technician on the line whether the firmware booted, and it is the last way to
signal a fault when everything else is down. Leaving it out saves a few cents and costs hours at bring-up.

**Example.** A sensor node whose only interface is a radio: when the radio does not join, there is nothing to say
whether the MCU runs at all, so every field return goes to the bench.

**How to check.** Count parts of the LED class (designator `D`/`LED` with an LED symbol or footprint); none on a
board with a programmable part is a finding.

**Fix.** One LED on a free GPIO (`DBG-16`), 1-2 mA is enough with a modern high-efficiency LED.

**References.** B153A schematic review §11 (LED_RED on PF0, LED_GREEN on PF1).

## `DBG-4` No power rail LEDs <a id="dbg-4"></a>

💡 suggestion · needs: power rails (`NET-5`), LEDs on them · [catalogue entry](../checks.md#dbg-4)

**Impl.** easy · depends on `NET-5`

**What it catches.** Power rails, above all the input rail, with no LED from the rail to ground.

**Why it matters.** A power LED answers the first bring-up question ("is it powered?") without a meter, shows a
shorted rail (LED off or dim) and a missing regulator at a glance, and lets a user see that a cable is in.
Rails that switch on in firmware are the most useful ones to show.

**Example.** A board with a USB input, a 3.3 V buck and a switched 5 V output: with a single LED on 3.3 V, a
blown input fuse and a buck that does not start look the same. One LED on VBUS tells them apart.

**How to check.** For each rail of the power structure, look for an LED (with its resistor) between the rail and a
ground net.

**Fix.** An LED on the input and on the main rail, optionally on switched rails; take the current into the sleep
budget (`RPT-2`), or put a DNP option on battery boards.

**References.** AoE3 chapter 9 (power regulation).

## `DBG-5` No error flag LEDs when flags aren't routed to IOs <a id="dbg-5"></a>

💡 suggestion · needs: fault and status outputs of parts (pin roles, `NET-7`) · [catalogue entry](../checks.md#dbg-5)

**Impl.** mid · depends on `NET-7`

**What it catches.** Fault and status outputs (`FAULT`, `nFLT`, `PG`, `CHG`, `STAT`, `ALERT`) that go neither to an
MCU pin nor to an LED: left open, or only pulled up.

**Why it matters.** A load switch that trips, a charger that stops on a timer fault, a regulator that never reaches
power-good: the part knows and says so on a pin, and nobody listens. Without a GPIO or an LED the failure looks
like "the board does nothing" and is found with a scope after an hour.

**Example.** A TI BQ24075 charger whose open-drain CHG and PGOOD outputs, meant to drive LEDs, are left
unconnected: a battery that never charges cannot be told from a charger that was never powered.

**How to check.** Pins of fault or status role (from the library, `NET-7`, or by name) whose net reaches no MCU
pin, no LED and no connector.

**Fix.** Route the flag to a GPIO; when pins are short, put an LED (open-drain flags can sink it directly) or at
least a test point.

**References.** The part's datasheet (pin description of the flag); catalogue `RAIL-14`, `IO-15`.

## `DBG-6` Power rails not monitored by an ADC <a id="dbg-6"></a>

💡 suggestion · needs: rails (`NET-5`), dividers to ADC-capable pins (`NET-10`, `NET-12`) · [catalogue entry](../checks.md#dbg-6)

**Impl.** mid · depends on `NET-5`, `NET-10`, `NET-12`

**What it catches.** Rails, especially the input and any rail the firmware switches or selects, that no ADC channel
can measure (directly or through a divider).

**Why it matters.** With an ADC on a rail the firmware can log brown-outs, refuse to start a motor on a weak supply,
report a shorted output and tell support what the input voltage really was. It also turns many field returns into
a log reading instead of a bench session.

**Example.** On B200A the IO reference `+VREF` is selected by firmware from three sources (or fed externally) but
cannot be measured, so the firmware has to trust that the level it set is the level on the pins; the review proposed
an ADC measurement or at least a "VREF present" input on the free GPIO24.

**How to check.** For each rail, follow resistors from it to an MCU pin that has an analog function (`NET-12`);
none is a finding for the input rail and for firmware-switched rails.

**Fix.** A divider into a free ADC pin (high-impedance with a small cap at the pin), or a supply monitor with an I2C
interface (INA2xx class) where current is wanted too.

**References.** AoE3 chapter 13 (ADC inputs); B200A schematic review §5 (`+VREF` not measured).

## `DBG-7` Suggest DNM pull-up and down on EN pins <a id="dbg-7"></a>

💡 suggestion · needs: enable pins (by name or `NET-7`), DNP flags · [catalogue entry](../checks.md#dbg-7)

**Impl.** easy · depends on netlist

**What it catches.** Enable pins of regulators, load switches and peripherals that are tied hard to a rail or
driven from one place only, with no footprint to override the level.

**Why it matters.** At bring-up the first thing one wants is to power the board stage by stage: hold a regulator
off to find which load shorts a rail, or force a switch on while the firmware that drives it does not exist yet.
A pair of unpopulated pull-up and pull-down footprints does that with a dab of solder; without them it is a cut trace.

**Example.** A 1.8 V LDO whose EN is tied straight to its VIN: when the 1.8 V rail turns out shorted, the only way to
tell whether the LDO or a load is at fault is lifting a pin.

**How to check.** Pins named `EN`, `ON`, `SHDN` and similar whose net is a rail directly, or a single driver, with
no second resistor footprint on it.

**Fix.** Drive EN through a resistor (or from a GPIO with a pull, `IO-3`) and add a DNM resistor to the other level.

**References.** XJTAG DFT (pull resistors to define a default that a controller can override); the regulator's
datasheet (EN thresholds and input current).

## `DBG-8` Suggest DNM pull-ups on open drain pins; test point detector should highlight those nets <a id="dbg-8"></a>

💡 suggestion · needs: open-drain pins (pin type), pull-ups · [catalogue entry](../checks.md#dbg-8)

**Impl.** easy · depends on netlist

**What it catches.** Open-drain or open-collector outputs (pin type in the library, or by role: `INT`, `ALERT`,
`PG`, `FAULT`) whose net relies on an MCU's internal pull-up only, with no resistor footprint.

**Why it matters.** An internal pull-up exists only once the firmware enables it: in reset, in the ROM bootloader,
and while the MCU is unpowered the line floats. A DNM footprint lets bring-up add a real pull-up when an interrupt
seems to "never fire", and makes the net easy to probe. Nets with no pull at all are `LOGIC-7`.

**Example.** A PMIC's open-drain `nIRQ` read by an MCU pin with its internal 40 kΩ pull-up: during the MCU's reset the
PMIC sees nobody acknowledging and the interrupt line floats; a DNM 10 kΩ footprint lets the cause be ruled out in a
minute.

**How to check.** Nets driven by an open-drain pin with no resistor to a rail, populated or not; list them in the
test point report (`RPT-15`) too.

**Fix.** A DNM (or fitted) 10 kΩ to the IO rail of the reader, plus a test point.

**References.** XJTAG DFT "Do not rely on programmable pull resistors"; catalogue `LOGIC-7`, `IO-15`.

## `DBG-9` Suggest DNM 0R bypasses on power switches <a id="dbg-9"></a>

💡 suggestion · needs: power switches (load switches, eFuses, FET switches; `NET-5` sources and switches) · [catalogue entry](../checks.md#dbg-9)

**Impl.** mid · depends on `NET-5`

**What it catches.** A load switch, eFuse, ideal-diode or discrete FET switch between two rails with no unpopulated
0 Ω footprint across it.

**Why it matters.** Power switches are where bring-up gets stuck: the control signal comes from firmware that does
not run yet, the part trips on inrush, or its current limit is set wrong (`RAIL-6`). A bypass footprint lets the
downstream circuit be brought up while the switch is debugged, and keeps a board usable when the switch was the
mistake.

**Example.** B153A's header supply `VBUS_SW` goes through Q102; a hard turn-on into a capacitive load reset the hub
until a 10 Ω resistor in the wire cured it (`SEQ-6`). A footprint across Q102 would have allowed the bench test of
the load without the switch.

**How to check.** Parts recognised as power switches between two rails (`NET-5`), and whether a resistor footprint
connects the same two nets.

**Fix.** A DNM 0 Ω (sized for the current, 0805 or larger) or a solder jumper across the switch.

**References.** B153 rev B change list (soft start on `VBUS_SW`); the switch's datasheet (current limit and
turn-on slew).

## `DBG-10` Many things on one power net: suggest splitting with a 0R before each power input <a id="dbg-10"></a>

💡 suggestion · needs: rails (`NET-5`), power input pins per part · [catalogue entry](../checks.md#dbg-10)

**Impl.** easy · depends on `NET-5`

**What it catches.** A rail that feeds many parts' supply pins with no 0 Ω, ferrite or jumper between the rail and
any of them.

**Why it matters.** When the rail is shorted or draws too much, there is no way to find the culprit short of
lifting parts; with a 0 Ω per branch each load can be isolated and its current measured. The same footprint also
becomes the place for a ferrite if one branch turns out noisy, and lets one block be left unpowered in a variant.

**Example.** On B200A the analog buffer rail `+5V0_FLT` had no source: probably a ferrite or 0 Ω from `+5V0_USBIN`
went missing, and the rework had to add one between two pads that were never there (`RAIL-5`). A design with a
branch footprint per block makes that both visible in the schematic and trivial to bodge.

**How to check.** Count supply pins of distinct parts on each rail; above a threshold (say five parts, or several
functional blocks) with no series part per branch is a finding.

**Fix.** A 0 Ω (or ferrite, or solder jumper) at the start of each block's branch.

**References.** Ott, *Electromagnetic Compatibility Engineering*, chapter 11 (decoupling and power distribution);
B200A schematic review §0.

## `DBG-11` Suggest 0R on USART TX, RX, CTS, RTS and SPI MOSI, MISO, as they're often wrong <a id="dbg-11"></a>

💡 suggestion · needs: UART and SPI nets (pin names, `NET-12`), series resistors · [catalogue entry](../checks.md#dbg-11)

**Impl.** easy · depends on netlist (`NET-12` for pins named by port only)

**What it catches.** UART (TX, RX, RTS, CTS) and SPI data lines (MOSI, MISO) between two parts, or to a connector,
with no series resistor footprint.

**Why it matters.** These are the signals most often crossed wrong: TX to TX, RTS to RTS, MISO to MOSI, because
"TX" can mean "this pin sends" or "the net to the other side's TX". A pair of 0 Ω footprints (or a crossover
option) turns a respin into a rework. The resistor also isolates a module that back-powers through its RX
(`BKF-1`) and is a probe point.

**Example.** On B153A the SBU lines carry UART in both orientations (SBU1 = TX or RX), and the design reaches both
through net ties and several MCU pins; an MCU-to-GPS link where both sides' TX pins meet on one net is the plain
version of the same mistake.

**How to check.** Nets that join a UART or SPI data pin to another part or a connector without a resistor in the
path; `MUX-1` and `NAME-4` catch the crossing itself.

**Fix.** 0 Ω in each line near the MCU, or a 2×2 footprint that allows straight and crossed fitting.

**References.** SparkFun, *Serial Communication* ("Rules and wiring": TX to RX)
([learn.sparkfun.com/tutorials/serial-communication](https://learn.sparkfun.com/tutorials/serial-communication/all));
B153A schematic review §8 (SBU modes and swapped wiring).

## `DBG-12` Spot potentially useful 0R series resistors (AI?) <a id="dbg-12"></a>

💡 suggestion · needs: intent of each net · [catalogue entry](../checks.md#dbg-12)

**Impl.** judgement · depends on netlist (candidates only)

**What it catches.** Places where a 0 Ω footprint would make a later fix cheap, beyond the fixed rules of `DBG-9`,
`DBG-10` and `DBG-11`: a signal that may need inverting, a strap that may need the other level, a level shifter
that may need bypassing.

**Why it matters.** The cheapest rework is fitting or removing a part that is already on the board. Which nets are
uncertain is a judgement on the design's risk, not a rule: a new part, an unclear datasheet, a protocol the team
has not used.

**Example.** B153A had R211/R212 0 Ω footprints that bypass its TXS0102 level shifter; when the shifter turned out
weak for push-pull signals, the bypass (still DNP on rev A) was the obvious fallback, and SPI-driven SWD on the same
lines needs net ties to reach the right MCU pins.

**How to check.** better_erc can list candidates (nets through a translator, straps, nets between a new part and the
MCU) for a reviewer or an LLM to rank; the decision stays with a person.

**Fix.** Add the footprints the reviewer picks, DNP by default where the straight path is the expected one.

**References.** B153A schematic review §8 (R211/R212 shifter bypass); B153 rev B change list (SBU level shifter).

## `DBG-13` No early power-fail signal on a board that writes storage (SD, eMMC, flash) <a id="dbg-13"></a>

💡 suggestion · needs: storage parts (part class), input rail and regulator (`NET-5` sources), a comparator ahead of it · [catalogue entry](../checks.md#dbg-13)

**Impl.** mid · depends on `NET-5`, `NET-7`

**What it catches.** A board with an SD card, eMMC or a flash the firmware writes to, and no comparator or supervisor
on the input ahead of the regulator that warns the MCU before its own rail drops.

**Why it matters.** A write cut by power loss corrupts a file system or a flash page; an SD card's internal
controller can lose more than the block being written. The MCU's own brown-out reset comes too late: it fires when
the rail is already sagging. Watching the input, upstream of the regulator and its bulk capacitance, gives
milliseconds of warning: enough to finish a block and stop.

**Example.** B125B logs to a microSD card from PoE or bus power; the rev B notes ask for a voltage comparator on both
inputs "to know earlier that power is gone and stop writing to SD".

**How to check.** If storage parts exist, look for a comparator or supervisor whose sense input divides down the
input rail (before the regulator) and whose output reaches an MCU pin with an interrupt. An MCU-internal detector
(STM32 PVD) watching VDD counts only partially: it sees the rail after the regulator.

**Fix.** A comparator or supervisor on the input with its output on an interrupt pin, enough bulk capacitance
behind the regulator to hold up the time a write needs, and a power-loss-tolerant file system in the firmware.

**References.** AN5096 §1.2.2 (PVD, an internal detector on VDD); littlefs, *DESIGN.md* (power-loss resilience)
([github.com/littlefs-project/littlefs/blob/master/DESIGN.md](https://github.com/littlefs-project/littlefs/blob/master/DESIGN.md));
B125 rev B change list.

## `DBG-14` No UART console reachable on a header or test points <a id="dbg-14"></a>

💡 suggestion · needs: UART pins of the MCU (pin names, `NET-12`), connectors and test points · [catalogue entry](../checks.md#dbg-14)

**Impl.** easy · depends on netlist (`NET-12` for pins named by port only)

**What it catches.** A board whose MCU or SoC has no UART TX/RX pair reaching a header, connector or test points.

**Why it matters.** A console is the debug channel that works without a probe, in the field and on a production
tester: boot logs, a shell, crash dumps. SoCs running Linux print their boot loader and kernel messages there; MCUs
use it for logs when SWO or RTT is not available. Adding one later means a respin or wires on QFN pins.

**Example.** A Linux SoM carrier board without its debug UART on a header: when U-Boot stops before the network
comes up, nothing at all is visible.

**How to check.** UART TX/RX pins of each programmable part (by name or alternate function) whose nets reach a
connector or a pair of test points; none is a finding.

**Fix.** Three pads or a 3-pin header (TX, RX, GND) at the IO level, ideally the pinout of a common USB-UART cable;
on SoCs the UART the ROM and boot loader use.

**References.** AN4989 §7.2 (printf via UART); Linux kernel documentation, *Linux Serial Console*
([docs.kernel.org/admin-guide/serial-console.html](https://docs.kernel.org/admin-guide/serial-console.html)).

## `DBG-15` Board revision and variant not readable by firmware <a id="dbg-15"></a>

💡 suggestion · needs: intent (which pins or dividers encode the board) · [catalogue entry](../checks.md#dbg-15)

**Impl.** judgement · depends on `NET-12` (candidates: ADC pins with a fixed divider, strap pins)

**What it catches.** No way for the firmware to tell which revision or assembly variant it runs on: no ID resistor
divider on an ADC pin, no strap pins, no ID EEPROM.

**Why it matters.** One firmware image for every board in the field is the only maintainable way, and it needs to
know whether pin X drives a LED (rev A) or the hub reset (rev B), or whether a 2-port or 4-port hub is fitted.
Without an ID the firmware guesses from side effects, and a guess that fails can drive a pin into a part that
is not there.

**Example.** B153A's Main and Plus variants differ in hub and SBU switch; the firmware tells them apart by R902, a
10 kΩ pull-down on PA10 fitted on Plus only, read at boot. The revision itself is not readable; the change list
suggests an ID divider on PA4 (ADC1_IN4) once that pin is free. B200A has the same need before its shift-register
chain is loaded.

**How to check.** Not a rule: better_erc can list ADC-capable pins that see only a divider between two rails, and
GPIOs tied to a rail through a resistor, as candidate IDs; a reviewer confirms whether one exists.

**Fix.** A resistor divider per board on a free ADC pin (one resistor per revision or variant, values spaced so
tolerances never overlap), or strap resistors on GPIOs, or an ID EEPROM as Raspberry Pi HATs use.

**References.** Raspberry Pi, *HAT specification* (ID EEPROM read at boot)
([github.com/raspberrypi/hats](https://github.com/raspberrypi/hats)); B153 rev B change list ("From the runtime
variant work"); B153A schematic review §9.

## `DBG-16` No firmware-driven LED (heartbeat), only power LEDs <a id="dbg-16"></a>

💡 suggestion · needs: LEDs and what drives them (MCU pin, rail) · [catalogue entry](../checks.md#dbg-16)

**Impl.** easy · depends on netlist (`NET-10` for LEDs driven through a transistor)

**What it catches.** A board whose LEDs all hang on rails: none is driven by an MCU pin.

**Why it matters.** A power LED says the rail is up, not that the firmware runs. A heartbeat LED blinking from the
main loop or a timer shows a hang, a reset loop (blink pattern restarts) and a hard fault at a glance, and is the
first thing a bring-up firmware toggles.

**Example.** B153A has a red and a green LED on PF0 and PF1 (both on timer channels for PWM): the firmware shows
its state without a probe. A board with only a "3V3" LED shows the same glow whether the firmware runs or sits in
a hard fault.

**How to check.** LEDs whose resistor chain ends on an MCU GPIO (directly, or through a transistor with `NET-10`);
none is a finding when a programmable part exists.

**Fix.** One LED on a free GPIO, ideally one with a timer output.

**References.** B153A schematic review §11 (LED pins, timer channels).

## `DBG-17` SWD header without NRST (and SWO): the probe cannot reset a stuck target or trace <a id="dbg-17"></a>

💡 suggestion · needs: debug header (`DBG-1`), reset pin, SWO pin · [catalogue entry](../checks.md#dbg-17)

**Impl.** easy · depends on netlist

**What it catches.** A debug header or test point set that carries SWDIO and SWCLK but not the MCU's reset (and,
on parts that have it, not SWO).

**Why it matters.** When the firmware turns the SWD pins into GPIO, puts the part into a low-power mode with the
debug clock off, or sets every pin analog, the probe can only regain control by holding NRST low while it connects
("connect under reset"). Without NRST on the header that needs a finger on a button or a power cycle at the right
moment. SWO gives printf-style trace without a UART.

**Example.** AN4989 notes that STM32Cube power examples switch every GPIO, SWD included, to analog to save current,
which disconnects the debugger: only connect under reset with NRST recovers. B200A's 2×3 "Just Screw It SWD" header
does carry `~RST` on pin 5.

**How to check.** Find the header carrying the SWD nets; check that the MCU's reset net reaches a pin of the same
header (or a test point beside the SWD ones); for parts with SWO, the same.

**Fix.** Use a standard pinout that includes nRESET (ARM 10-pin: pin 10) and SWO (pin 6); on test-point designs,
add an NRST pad.

**References.** AN4989 §4.1 (SWD pins in reset state, connect under reset), §4.2 (reset and connection mode), §7.3
(SWO); SEGGER 9-pin (pin 10 nRESET, pin 6 SWO); AN5096 §4.3.3 (SWD connector with NRST).

## `DBG-18` I2C or SPI buses not reachable at a header or test points for a bus analyser <a id="dbg-18"></a>

💡 suggestion · needs: buses (`NET-4` for I2C, SPI by pin names), test points · [catalogue entry](../checks.md#dbg-18)

**Impl.** easy · depends on `NET-4`

**What it catches.** I2C (SCL, SDA) and SPI (SCK, MOSI, MISO, CS) buses whose lines cannot be reached by a
logic analyser clip: no test points, no header, only fine-pitch pins.

**Why it matters.** Most "sensor not found" and "reads garbage" bugs end with a logic analyser on the bus. On a board
with QFN and 0402 parts there is nowhere to clip; the probe wires get soldered to a pull-up resistor, which works
once and lifts a pad the second time.

**Example.** An IMU on SPI next to a QFN MCU with its 0402 decoupling: the only reachable metal is the via under the
chip select trace.

**How to check.** For each bus line, look for a test point or connector pin on the net; report buses with none
(same data as `RPT-15`, `TEST-5`).

**Fix.** A test point per line plus GND, or an unpopulated 2.54 mm header (a footprint costs nothing).

**References.** XJTAG DFT (bring control and observation signals to a connector); catalogue `TEST-5`.

## `DBG-19` Debug header VTref not wired straight to the target rail; a header VCC pin that back-powers the board <a id="dbg-19"></a>

⚠️ warning · needs: debug header pin roles (VTref, VCC), the rail of the debugged part, power states · [catalogue entry](../checks.md#dbg-19)

**Impl.** mid · depends on `NET-5`, `NET-7`

**What it catches.** Two faults on the header's supply pins: VTref connected through a series resistor, or not at all,
or to another rail than the debugged part's IO rail; and a header supply pin that the probe can drive into the
board's rail.

**Why it matters.** The probe uses VTref to see whether the target has power and to set its own output level and
input thresholds. SEGGER says it must not have a series resistor: with one the probe sees a sagging or wrong level.
Unconnected, many probes refuse to connect or drive at their default level. A probe with a supply output (J-Link
pin 19 gives 5 V, up to 300 mA) or a header pin wired to the board's main rail can power the board from the probe:
half of it comes up, parts on other rails are back-powered through their IOs (`BKF-2`), and the bug hunt starts on
a board in a state it never sees in use.

**Example.** A board with a 1.8 V MCU and VTref on the 3.3 V rail: the probe drives 3.3 V into 1.8 V pins.
Another with VTref through 1 kΩ "for protection": the probe reads a voltage that sags under its own sense
current.

**How to check.** Recognise the header from its SWD/JTAG nets and its pinout (`CONN-15`); the VTref pin must sit on
the IO rail of the debugged part with no part in between; a header pin that is a rail must be marked as a probe
power input only if the design means it (`CFG-2`).

**Fix.** VTref straight to the target's IO rail; leave the probe's supply pin unconnected unless the board is meant
to be powered from the probe, and then feed it in through a diode or switch.

**References.** SEGGER 20-pin (VTref: "must not have a series resistor"; pin 19 5V-Supply, 300 mA); SEGGER 9-pin
(VTref); catalogue `BKF-2`, `CONN-15`.

## `DBG-20` No hardware way to reset or power-cycle bus peripherals after an MCU-only reset <a id="dbg-20"></a>

💡 suggestion · needs: power states (`NET-8`), reset and enable nets of the peripherals · [catalogue entry](../checks.md#dbg-20)

**Impl.** hard · depends on `NET-8`, `NET-10`

**What it catches.** Peripherals on a bus (I2C above all) that have no reset pin driven by the MCU and sit on a
rail the MCU cannot switch, so only a full power cycle resets them.

**Why it matters.** An I2C slave interrupted mid-byte by an MCU reset keeps holding SDA low, waiting for clocks that
never come; the MCU comes back and finds the bus stuck. UM10204 describes the bus clear: the master sends up to nine
clocks, and if the slave still holds SDA, the device needs a hardware reset or a power cycle. Without either, the
board needs the user to pull the plug.

**Example.** An I2C fuel gauge on an always-on rail with no reset pin: after a watchdog reset during a read it holds
SDA low; the firmware's nine-clock recovery works only if SCL is a GPIO that firmware can toggle, and if not, the
battery has to be removed.

**How to check.** For each bus device, does a reset pin reach an MCU GPIO, or is its rail switched by one? Needs the
power states (`NET-8`) to know which rails a GPIO can turn off.

**Fix.** Wire peripheral resets to a GPIO, or put the peripherals on a load switch the MCU controls; make SCL a pin
the firmware can bit-bang for the nine-clock recovery.

**References.** UM10204 §3.1.16 (bus clear); catalogue `IO-13`; B153 rev B change list (hub reset kept across an MCU
reset, the reverse case).

## `DBG-21` JTAG pulls: TCK without a pull-down, TMS and TDI without pull-ups, nTRST floating <a id="dbg-21"></a>

⚠️ warning · needs: JTAG pins (by name), external pulls, internal pulls of the part (`NET-7`) · [catalogue entry](../checks.md#dbg-21)

**Impl.** mid · depends on `NET-7`

**What it catches.** JTAG TAP pins left to float when no probe is connected: TMS or TDI without a pull-up, TCK without
a defined level (a pull-down), nTRST floating where the header has no nTRST pin.

**Why it matters.** With TMS held high, five TCK edges bring the TAP to Test-Logic-Reset, its quiet state; a floating
TMS and a TCK picking up noise can walk the TAP into other states and, on some parts, into boundary-scan modes
that take over the pins. IEEE 1149.1 requires internal pull-ups on TMS, TDI and nTRST, but not every part follows
it and many are weak. nTRST is the subtle one: XJTAG recommends a pull-down (TAP held in reset when unused, the
boundary-scan view), SEGGER says it is normally pulled high so the debug logic is not reset when no probe is
attached (the debug view). Floating it is wrong in both.

**Example.** An FPGA with its JTAG header unpopulated in production and no pulls: a field unit with a long cable
nearby occasionally loses its configuration.

**How to check.** JTAG pins of each part; for each, an external pull or an internal pull known from the part's data
(`NET-7`); nTRST: pulled to a level that matches whether the header drives it.

**Fix.** 1-10 kΩ pull-ups on TMS and TDI, a pull-down on TCK, nTRST pulled down if a probe or tester drives it,
otherwise up (or tied to the power-on reset).

**References.** XJTAG DFT "Pull-ups / Pull-downs" (TDI, TDO, TMS up, nTRST down, 1-10 kΩ); SEGGER 20-pin (nTRST
"normally pulled HIGH", TMS "should be pulled up", TCK and TDI "pulled to a defined state"); IEEE 1149.1; Wikipedia,
JTAG.

## `DBG-22` SWDIO without a pull-up or SWCLK without a pull-down (10k-100k) <a id="dbg-22"></a>

💡 suggestion · needs: SWD pins, external pulls, internal pulls of the part (`NET-7`) · [catalogue entry](../checks.md#dbg-22)

**Impl.** mid · depends on `NET-7`

**What it catches.** SWDIO with no pull-up, SWCLK with no pull-down, on a part that does not provide them internally,
or where the pins can lose their internal pulls (pins reused as GPIO).

**Why it matters.** The ARM SWD protocol expects a defined idle: SWDIO high, SWCLK low. With an unplugged header and
a few centimetres of trace, noise can toggle the debug port. On parts with a boot function on SWCLK it matters more:
STM32G0's PA14 is also BOOT0, and SWCLK held high at reset enters the ROM bootloader once `nBOOT_SEL` is cleared.

**Example.** B153A's SWCLK carries a 100 kΩ pull-down (R209) because it is also BOOT0; the change list asks whether
to go to 10 kΩ like on CANnify. AN5096 says the STM32G0 has internal pulls on both pins (SWDIO pull-up, SWCLK
pull-down) and needs no external ones, so for that part the check only fires where the pins are reused.

**How to check.** For each SWD pin, an external pull to the right level, or an internal one from the part's data
(`NET-7`); flag parts without either.

**Fix.** 10-100 kΩ pull-up on SWDIO and pull-down on SWCLK at the MCU, unless the datasheet guarantees the internal
ones in every state the board sees.

**References.** AN5096 §4.3.2 (internal pull-up and pull-down on SWD pins); B153 rev B change list (SWCLK
pull-down); B153A schematic review §12 (BOOT0 on PA14).

## `DBG-23` Spare GPIOs not brought to pads or a header for bodges and debug <a id="dbg-23"></a>

💡 suggestion · needs: MCU pins classed as GPIO (`NET-7`), unconnected pins · [catalogue entry](../checks.md#dbg-23)

**Impl.** mid · depends on `NET-7`, `NET-12`

**What it catches.** An MCU with free GPIOs that are left unconnected under the package, with no pad, test point
or header.

**Why it matters.** A free GPIO on a pad is the material of every rework: a wire to a signal the design forgot, a
scope trigger from firmware, a timing measurement, a second UART. On a QFN, an unrouted pin is out of reach.

**Example.** B153A's hub reset sat on PF2, which is the MCU's NRST; the rework cut it and wired it to PC13, the only
free pin, and the review notes "Free pins: none on rev A once PC13 carries the hub reset". With PC13 unrouted under
the QFN the fix would have needed a respin.

**How to check.** GPIO-class pins of programmable parts with no net (or a one-pin net) and no test point.

**Fix.** Route spare GPIOs to test points or a 2.54 mm header footprint, and tie their default in firmware
(`WIRE-8`, `WIRE-9` for unused pins).

**References.** B153A schematic review §11 (MCU pin map, free pins); B153 rev B change list (PF2 is NRST).

## `DBG-24` No non-volatile or backup storage for reset cause and crash logs <a id="dbg-24"></a>

💡 suggestion · needs: memory parts and MCU features, firmware plan · [catalogue entry](../checks.md#dbg-24)

**Impl.** judgement · depends on netlist (candidates: no FRAM/EEPROM, no backup supply)

**What it catches.** A design with no place to keep a crash record across a reset or power cycle: no backup
registers kept alive, no FRAM or EEPROM, no flash pages set aside.

**Why it matters.** A field unit that resets once a day leaves nothing to look at unless the firmware writes the
reset cause, the faulting address and a few words of context somewhere that survives. Most MCUs have reset-cause
flags and a few backup registers kept alive while VBAT or VDD stays up; a power cycle needs FRAM, EEPROM or reserved
flash. The schematic decides which are available: VBAT wired or not, a small FRAM on the I2C bus or not.

**Example.** A motor controller that browns out and resets in the field: without a stored reset cause, support
cannot tell a brown-out from a watchdog from a hard fault.

**How to check.** A judgement on the firmware plan; better_erc can at most report whether a backup supply pin
(VBAT) is wired and whether a small non-volatile memory exists.

**Fix.** Keep VBAT or the backup domain powered, add a small FRAM/EEPROM where a log must survive power cycles,
and reserve flash pages for it.

**References.** AN4989 §6 (exception handling: where the faulting state comes from); the MCU's reference manual
(reset and backup domain); B200A change list (flash sized for A/B images and introspection).
