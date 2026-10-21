# CLK checks, explained

Detail page for the `CLK` category of the [check catalogue](../checks.md#%EF%B8%8F-clk---clocks-and-reset). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find
it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to build in better_erc without an LLM: `easy` (the netlist alone), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet) or `judgement` (a human or an LLM
decides, better_erc lists candidates), followed by the [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **AN2867** · ST, *Guidelines for oscillator design on STM8AF/AL/S and STM32 MCUs/MPUs*, Rev 24 (2026). Load
  capacitance (§3.3), gain margin gm/gmcrit > 5 (§3.4), drive level and the external resistor (§3.5), pullability and
  C0G load capacitors (§3.7), low-speed oscillators (§4.1), crystal selection (§4.2), PCB guidelines (§7.1), LSE and
  PC13 (§7.4), MEMS oscillators and bypass mode (§10).
  [st.com/an2867](https://www.st.com/resource/en/application_note/an2867-guidelines-for-oscillator-design-on-stm8afals-and-stm32-mcusmpus-stmicroelectronics.pdf)
- **AN5096** · ST, *Getting started with STM32G0 MCUs hardware development*, Rev 4 (2025). VBAT (§1.1.1), NRST modes and
  the reset pulse generator (§1.3.2), HSE and HSE bypass (§2.1), HSI16 accuracy (§2.2), HSI48 with CRS for USB (§2.3),
  reset in the reference design (§6.1.2).
  [st.com/an5096](https://www.st.com/resource/en/application_note/an5096-getting-started-with-stm32g0-series-hardware-development-stmicroelectronics.pdf)
- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, Rev 4 (2026): NRST
  internal pull-up (§6.7), VBAT-domain GPIOs limited to 3 mA (§6.8).
  [st.com/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **STM32G0B1** · ST datasheet DS13560 Rev 6 (2026): HSE 4-48 MHz (Table 42), LSE drive settings and Gmcritmax (Table
  43), NRST input (§5.3.15). [st.com/stm32g0b1re.pdf](https://www.st.com/resource/en/datasheet/stm32g0b1re.pdf)
- **RM0008** · ST, *STM32F101xx ... STM32F107xx reference manual*, Rev 21 (2021): HSI 1% (§7.2.2), PLL for the 48 MHz
  USB clock (§7.2.3).
  [st.com/rm0008](https://www.st.com/resource/en/reference_manual/rm0008-stm32f101xx-stm32f102xx-stm32f103xx-stm32f105xx-and-stm32f107xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
- **RP2350** · Raspberry Pi, *RP2350 Datasheet*: XOSC 1-50 MHz, 5 MHz minimum for the PLL, external clock into XIN
  (§8.2.1).
  [datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- **RP2350 HW** · Raspberry Pi, *Hardware design with RP2350*: crystal, 15 pF load capacitors, 1 kΩ series resistor (§4,
  §4.1).
  [datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf)
- **USB 2.0** · USB-IF, *Universal Serial Bus Specification*, Revision 2.0 (2000), §7.1.11 Data Signaling Rate (high
  speed ±500 ppm, full speed ±0.25 %).
  [usb.org/document-library/usb-20-specification](https://www.usb.org/document-library/usb-20-specification)
- **LAN8720A** · Microchip, *LAN8720A/LAN8720AI* datasheet, DS00002165B: clock circuit, 25 MHz crystal with a ±50 ppm
  budget (§5.6, Table 5-13).
  [ww1.microchip.com/…/00002165B.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00002165B.pdf)
- **ESP32 checklist** · Espressif, *ESP Hardware Design Guidelines*, Schematic Checklist (ESP32): Clock Source (40 MHz
  crystal within ±10 ppm, RTC crystal), Chip Power-up and Reset Timing.
  [docs.espressif.com/…/schematic-checklist.html](https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32/schematic-checklist.html)
- **ABM8** · Abracon, *ABM8 ceramic SMD crystal* datasheet (rev. 2020): outline drawing, pads 2 and 4 are GND.
  [abracon.com/Resonators/ABM8.pdf](https://abracon.com/Resonators/ABM8.pdf)
- **SiT8008** · SiTime, *SiT8008B Low Power Programmable Oscillator* datasheet: Table 2 (OE/ST/NC pin, pull-up note).
  [sitime.com/datasheet/SiT8008](https://www.sitime.com/datasheet/SiT8008)
- **TPS3823** · TI, *TPS382x Voltage Monitor With Watchdog Timer*, SLVS165O (2025): push-pull and open-drain outputs
  (§7.3.3), watchdog input (§7.3.4). [ti.com/lit/ds/symlink/tps3823.pdf](https://www.ti.com/lit/ds/symlink/tps3823.pdf)
- **CDCLVC1102** · TI, *CDCLVC11xx 3.3-V and 2.5-V LVCMOS High-Performance Clock Buffer Family*, SCAS895B (2017): series
  resistor near the driver (§8.1, §9.2.1).
  [ti.com/lit/ds/symlink/cdclvc1102.pdf](https://www.ti.com/lit/ds/symlink/cdclvc1102.pdf)
- **Wikipedia, Crystal oscillator**, **Pierce oscillator**, **Power-on reset**, **Watchdog timer**, **Real-time clock**
  · readable overviews. [en.wikipedia.org/wiki/Crystal_oscillator](https://en.wikipedia.org/wiki/Crystal_oscillator),
  [en.wikipedia.org/wiki/Pierce_oscillator](https://en.wikipedia.org/wiki/Pierce_oscillator),
  [en.wikipedia.org/wiki/Power-on_reset](https://en.wikipedia.org/wiki/Power-on_reset),
  [en.wikipedia.org/wiki/Watchdog_timer](https://en.wikipedia.org/wiki/Watchdog_timer),
  [en.wikipedia.org/wiki/Real-time_clock](https://en.wikipedia.org/wiki/Real-time_clock)
- **Johnson & Graham** · *High-Speed Digital Design: A Handbook of Black Magic* (Prentice Hall, 1993), chapters 11
  (clock distribution) and 12 (clock oscillators).

## `CLK-1` MCU with CAN or USB and no crystal <a id="clk-1"></a>

🛑 error · needs: interfaces in use, the part's clock options · [catalogue entry](../checks.md#clk-1)

**Impl.** mid · depends on NET-12

**What it catches.** A controller that uses USB or CAN while it has no crystal, crystal oscillator or other accurate
clock, and no internal clock that is good enough for that interface.

**Why it matters.** Full-speed USB needs ±0.25 % and high speed ±500 ppm (USB 2.0 §7.1.11); CAN needs a few tenths of a
percent at higher bit rates. Internal RC oscillators are trimmed to about 1 % at 25 °C and drift with temperature
(RM0008 §7.2.2, AN5096 §2.2): enumeration works on the bench and fails in the cold. Some parts have a USB-grade internal
clock: the STM32G0B1's HSI48 trimmed by the clock recovery system from the USB SOF packets (AN5096 §2.3). That exception
covers USB device mode only, never CAN.

**Example.** An STM32F103 USB device without a crystal: the USB clock comes from the PLL (RM0008 §7.2.3), and with the
PLL on the 1 % HSI the board enumerates on some hosts and not others.

**How to check.** For each controller with USB or CAN pins in use (`NET-12`), require a crystal, oscillator or clock
input on its HSE/XIN pins, unless part data marks a crystal-less mode for exactly that interface.

**Fix.** A crystal or oscillator; for USB device only on parts with CRS-style trimming, document the choice.

**References.** USB 2.0 §7.1.11; RM0008 §7.2.2-7.2.3; AN5096 §2.2-2.3; RP2350 HW §4 (USB needs the external crystal).

## `CLK-2` HS crystal capacitors should be high frequency rated <a id="clk-2"></a>

⚠️ warning · needs: dielectric and tolerance of the load capacitors · [catalogue entry](../checks.md#clk-2)

**Impl.** mid · depends on netlist (MPN field)

**What it catches.** Crystal load capacitors that are not class 1 (C0G/NP0) ceramics: X7R, X5R or Y5V parts, or
tolerances worse than ±5 %.

**Why it matters.** The load capacitance sets the crystal's frequency (pullability is tens of ppm per pF, AN2867 §3.7).
Class 2 dielectrics change capacitance with temperature, voltage and age by percent, not ppm, and have more loss at high
frequency: the frequency drifts and the oscillator's margin shrinks.

**Example.** 12 pF X7R capacitors (±10 %, −15 % over temperature) on a 25 MHz Ethernet crystal with 30 ppm/pF
pullability: a few pF of drift is enough to push the clock outside the ±50 ppm budget (`CLK-19`).

**How to check.** Decode the dielectric and tolerance from the capacitors' MPNs (the value field rarely says it).

**Fix.** C0G/NP0, ±5 % or better; AN2867's example and Raspberry Pi's reference design both use C0G.

**References.** AN2867 §3.7; RP2350 HW appendix A (C0G load capacitors); Wikipedia, Ceramic capacitor (class 1)
[en.wikipedia.org/wiki/Ceramic_capacitor](https://en.wikipedia.org/wiki/Ceramic_capacitor).

## `CLK-3` Crystal NC pins may need to go to ground <a id="clk-3"></a>

⚠️ warning · needs: crystal parts and their pin roles · [catalogue entry](../checks.md#clk-3)

**Impl.** easy · depends on IMP-1

**What it catches.** A four-pad crystal whose two non-crystal pads (lid or case pads) are left unconnected.

**Why it matters.** In 4-pad packages like the 3.2 × 2.5 mm ABM8, pads 2 and 4 are GND (connected to the lid; the
crystal sits between 1 and 3, ABM8 outline drawing). Left open, the lid floats and couples noise into the high-impedance
oscillator nodes; footprints that leave them unconnected also lose mechanical support.

**Example.** A KiCad 2-pin crystal symbol on a 4-pad footprint: pads 2 and 4 have no net, the DRC is happy, and the
metal lid floats next to XIN.

**How to check.** Crystals with four pins (`IMP-1`) where the non-crystal pins are unconnected.

**Fix.** Use a 4-pin symbol and connect the GND pads to the oscillator's ground, as the datasheet shows.

**References.** ABM8 outline drawing; AN2867 §7.1 (ground under the oscillator).

## `CLK-4` Crystal pins used as IO: warn about low drive strength <a id="clk-4"></a>

⚠️ warning · needs: which pins are oscillator pins, their drive limits · [catalogue entry](../checks.md#clk-4)

**Impl.** mid · depends on NET-12

**What it catches.** Oscillator pins (OSC32_IN/OUT, PC14/PC15, XIN/XOUT) used as GPIO for a load that needs current or
speed.

**Why it matters.** On STM32 the LSE pins and PC13 sit in the backup (VBAT) domain, supplied through a switch that
limits them to 3 mA and low speed even with VDD present (AN4899 §6.8). Toggling PC13 also disturbs the LSE (AN2867
§7.4). An LED or a fast signal on these pins gives a weak level and a drifting RTC.

**Example.** An LED at 10 mA on PC14 because the board has no 32 kHz crystal: the LED is dim, and the pin's supply
switch is overloaded.

**How to check.** Pins with an oscillator function (`NET-12`) used as GPIO; estimate the load (LED, pull to the other
rail) against the pin's limit.

**Fix.** Use these pins for slow inputs; move LEDs and fast signals elsewhere.

**References.** AN4899 §6.8; AN2867 §7.4.

## `CLK-5` No pull-up on a reset_n input <a id="clk-5"></a>

⚠️ warning · needs: reset pins, internal pulls · [catalogue entry](../checks.md#clk-5)

**Impl.** easy · depends on IMP-1, NET-7

**What it catches.** An active-low reset input (`~RST`, `NRST`, `RESET_N`, `RUN`, `CHIP_PU`) with no pull-up, where the
part has no internal pull or only a weak one.

**Why it matters.** A floating reset puts the part in and out of reset with noise. Even where an internal pull-up exists
(STM32 NRST, about 40 kΩ, AN4899 §6.7), a long trace to a debug connector or a button picks up enough to glitch it;
ESP32's CHIP_PU must not float at all (ESP32 checklist).

**Example.** An ESP32 module's EN pin routed to a header for an auto-reset circuit and nothing else: without the
programmer attached, the module resets when the cable next to it moves.

**How to check.** Reset-role pins (`IMP-1` names, part data `NET-7`) with no resistor to a rail; with a known internal
pull-up, downgrade to a note.

**Fix.** 10 kΩ pull-up to the part's IO rail (with `CLK-7`'s capacitor).

**References.** AN4899 §6.7; ESP32 checklist (Chip Power-up and Reset Timing); AN5096 §6.1.2.

## `CLK-6` Suggest a cap on buttons to reset inputs; note hw debounce in the programmer report <a id="clk-6"></a>

💡 suggestion · needs: switch parts on reset nets · [catalogue entry](../checks.md#clk-6)

**Impl.** easy · depends on IMP-1, RPT-13

**What it catches.** A reset button with no capacitor on the reset net.

**Why it matters.** A bouncing reset button gives a train of short resets; parts with a minimum reset pulse width (the
USB251xB needs RESET_N low for at least 1 µs after its supplies are valid) may see a pulse too short to be a clean
reset, and ESD from a finger lands directly on the pin.

**Example.** A reset button straight to `NRST` on a board with a 1 m cable to the button panel: every press gives a
burst of resets and occasionally a half-reset peripheral.

**How to check.** Switches on reset nets (`IMP-1`) with no capacitor to ground on that net; report "hw debounce: yes/no"
for the reset input in the programmer report (`RPT-13`).

**Fix.** 100 nF to ground on the reset net (with the pull-up, a ms-range time constant).

**References.** USB251xB §5.5.1
[ww1.microchip.com/…/00001692C.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf); AN5096 §6.1.2.

## `CLK-7` No capacitor on a reset_n input <a id="clk-7"></a>

⚠️ warning · needs: reset pins · [catalogue entry](../checks.md#clk-7)

**Impl.** easy · depends on IMP-1

**What it catches.** An active-low reset input with no capacitor to ground.

**Why it matters.** The capacitor filters noise and ESD on a high-impedance node and, with the pull-up, holds the part
in reset until the supply is up. Espressif recommends R = 10 kΩ and C = 1 µF on CHIP_PU for the power-up timing (ESP32
checklist). On STM32G0 the internal reset holder pulls NRST low until the capacitor is discharged (AN5096 §6.1.2), so a
capacitor there is expected.

**Example.** B200A's RP2350 `RUN` has 10 kΩ and 100 nF (B200A hardware reference §3), and the pin also goes to the SWD
header: without the capacitor, that header and whatever cable hangs on it feed noise straight into the reset.

**How to check.** Reset-role pins (`IMP-1`) whose net has no capacitor to ground.

**Fix.** 10-100 nF (1 µF where a power-up delay is wanted), within the driver's capability to pull it low.

**References.** ESP32 checklist; AN5096 §1.3.2, §6.1.2; Wikipedia, Power-on reset.

## `CLK-8` Crystal load capacitors vs the crystal's CL <a id="clk-8"></a>

⚠️ warning · needs: crystal CL, capacitor values, stray estimate · [catalogue entry](../checks.md#clk-8)

**Impl.** mid · depends on NET-7

**What it catches.** Load capacitors that do not give the crystal its specified load capacitance CL.

**Why it matters.** A crystal oscillates at its nominal frequency only with the load it was cut for; a wrong load pulls
it by tens of ppm per pF, and a much too large one lowers the oscillator's gain margin until it does not start (AN2867
§3.3-3.4). The catalogue's rule of thumb C1 = C2 ≈ 2·CL − Cstray uses the stray per pin; with AN2867's total stray Cs it
reads CL = C1·C2/(C1 + C2) + Cs, so C1 = C2 = 2·(CL − Cs).

**Example.** B200A's 12 MHz RP2350 crystal follows Raspberry Pi's design: CL = 10 pF, 15 pF capacitors, about 3 pF
stray: 7.5 + 3 = 10.5 pF (RP2350 HW §4). Fitting 22 pF "because the other crystal had them" gives 14 pF, 4 pF too much,
which pulls the crystal tens of ppm low.

**How to check.** CL from the crystal's part data (`NET-7`), capacitor values from the netlist, stray from the part (pin
capacitance) plus a default for the PCB; report the computed CL and the deviation.

**Fix.** Recompute the capacitors; confirm on the board by measuring the frequency (AN2867 §4.2 step 2).

**References.** AN2867 §3.3, §4.2; RP2350 HW §4; Wikipedia, Pierce oscillator.

## `CLK-9` Crystal ESR and drive level vs the oscillator <a id="clk-9"></a>

⚠️ warning · needs: crystal ESR, C0, CL, drive level; oscillator gm · [catalogue entry](../checks.md#clk-9)

**Impl.** mid · depends on NET-7

**What it catches.** A crystal whose gain margin with the oscillator is under 5 (gm / gmcrit, with gmcrit = 4 · ESR ·
(2πF)² · (C0 + CL)²), or a crystal driven above its rated drive level with no series resistor.

**Why it matters.** Below a gain margin of 5 the oscillator may start in the lab and not in production or at temperature
(AN2867 §3.4); where the datasheet gives Gm_crit_max instead, gmcrit must stay below it. Above the drive level the
crystal ages fast or cracks; the fix is a series resistor at the oscillator output (AN2867 §3.5).

**Example.** Raspberry Pi pairs the RP2350's oscillator with a 50 Ω ESR crystal and a 1 kΩ series resistor so a 3.3 V
IOVDD does not overdrive it; at a lower IOVDD the same resistor leaves too little amplitude (RP2350 HW §4). B200A uses
that circuit. A cheap 12 MHz crystal with 100 Ω ESR and the same 1 kΩ risks a slow or failed start.

**How to check.** Compute gmcrit from the crystal's part data and compare with the oscillator's gm or Gm_crit_max (part
data of the controller, `NET-7`); check that a series resistor exists where the oscillator's drive exceeds the crystal's
drive level.

**Fix.** A crystal with lower ESR or CL, the resistor value from AN2867 §3.5.3, or the vendor's recommended crystal.

**References.** AN2867 §3.4-3.5, §10; RP2350 HW §4-4.1.

## `CLK-10` Crystal frequency outside the range the part's oscillator supports <a id="clk-10"></a>

🛑 error · needs: oscillator range per part, crystal frequency · [catalogue entry](../checks.md#clk-10)

**Impl.** mid · depends on NET-7

**What it catches.** A crystal or oscillator frequency outside the oscillator input's specified range, or outside the
range the firmware's PLL or boot ROM can use.

**Why it matters.** Oscillator amplifiers are designed for a range: the STM32G0B1 HSE takes 4-48 MHz (STM32G0B1 Table
42); the RP2350 XOSC 1-50 MHz but its PLL needs at least 5 MHz (RP2350 §8.2.1); some boot ROMs assume one frequency (the
ESP32 firmware supports only a 40 MHz crystal, ESP32 checklist). Outside, the oscillator does not start, or the ROM's
UART and USB run at the wrong rate.

**Example.** A 3.6864 MHz UART-friendly crystal on an STM32G0's HSE: below the 4 MHz minimum, it may or may not start.

**How to check.** The crystal frequency from its value or MPN against the range in the controller's part data (`NET-7`)
and any ROM restriction.

**Fix.** A crystal inside the range; for odd frequencies, derive them with the PLL.

**References.** STM32G0B1 Table 42; RP2350 §8.2.1; ESP32 checklist (Clock Source); AN2867 §3.4 (note on the range).

## `CLK-11` 32.768 kHz crystal with a CL or ESR the low-power oscillator cannot drive <a id="clk-11"></a>

⚠️ warning · needs: crystal CL and ESR, LSE drive settings · [catalogue entry](../checks.md#clk-11)

**Impl.** mid · depends on NET-7

**What it catches.** A 32 kHz crystal whose CL (12.5 pF types) or ESR needs more oscillator gain than the part's
low-speed oscillator gives at the drive setting the firmware uses.

**Why it matters.** Low-speed oscillators have gm of a few µA/V to tens of µA/V (AN2867 §3.4) and selectable drive
levels (STM32G0B1 Table 43, LSEDRV). A 12.5 pF crystal is robust but needs a higher drive; a 6-7 pF one is low power but
sensitive to noise (AN2867 §4.1, Figure 12). Picking the crystal without checking means an RTC that stops in the cold,
or more battery current than planned.

**Example.** A 12.5 pF, 70 kΩ ESR watch crystal on an STM32 with the LSE left at its lowest drive: gmcrit exceeds the
low-drive Gm_crit_max, the LSE starts on most boards and not on some.

**How to check.** As `CLK-9` with the LSE's Gm_crit_max per drive setting (part data, `NET-7`) and the drive setting
from the project.

**Fix.** A crystal from the vendor's compatible list (AN2867 Table 7), or a higher drive setting.

**References.** AN2867 §4.1, Table 5, Table 7; STM32G0B1 Table 43.

## `CLK-12` Active oscillator output into a crystal pin without bypass mode or attenuation <a id="clk-12"></a>

⚠️ warning · needs: oscillator parts, the pin's bypass capability · [catalogue entry](../checks.md#clk-12)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** A clock oscillator (or another chip's clock output) driving an MCU's crystal input while the
crystal network is still there, the firmware does not select bypass mode, or the level is not what the input takes.

**Why it matters.** With an external clock the oscillator amplifier must be off: STM32 calls it HSE bypass (AN5096 §2.1,
AN2867 §10), the RP2350 drives XIN directly with the oscillator disabled (RP2350 §8.2.1). Leaving the amplifier on, or
leaving load capacitors on the line, distorts the clock; some inputs accept only a limited swing and need a divider or a
capacitor.

**Example.** A 25 MHz oscillator into an STM32's OSC_IN with the old 18 pF load capacitors still fitted and the firmware
configuring HSE as a crystal: the clock works at room temperature with a distorted edge and jitter.

**How to check.** An oscillator part's output (`NET-7`) on a crystal input pin; check for leftover load capacitors and
the project's clock configuration (`CFG-2`).

**Fix.** Bypass mode in firmware, remove the load capacitors, OSC_OUT free or used as allowed, level per the datasheet.

**References.** AN5096 §2.1; AN2867 §10; RP2350 §8.2.1.

## `CLK-13` Oscillator or clock buffer output-enable / standby pin floating <a id="clk-13"></a>

🛑 error · needs: OE/ST pin roles of oscillators and buffers · [catalogue entry](../checks.md#clk-13)

**Impl.** mid · depends on NET-7

**What it catches.** The OE, ~ST or EN pin of a crystal oscillator or clock buffer left unconnected where the part needs
it driven or pulled.

**Why it matters.** Many oscillators share pin 1 between output-enable, standby and no-connect, chosen by the ordering
code. The SiT8008 in OE or standby mode asks for a pull-up of 10 kΩ or less if the pin is not driven; only the NC
variant may float (SiT8008 Table 2). A floating OE gives a clock that stops at random or never starts.

**Example.** A SiT8008 ordered as the standby variant on a footprint that left pin 1 open (drawn for an NC-pin
oscillator): the clock sleeps on some boards.

**How to check.** OE/ST pins of oscillator and buffer parts (`NET-7`) with no driver and no pull; read the variant from
the MPN where the pin's mode depends on it.

**Fix.** Tie or pull to the enabling level, or order the NC variant.

**References.** SiT8008 Table 2; Johnson & Graham chapter 12.

## `CLK-14` Reset input driven by a push-pull output <a id="clk-14"></a>

⚠️ warning · needs: pin types of the drivers on reset nets · [catalogue entry](../checks.md#clk-14)

**Impl.** easy · depends on IMP-1, NET-7

**What it catches.** A reset net driven by a push-pull output (a GPIO, a supervisor with push-pull output) together with
any other driver: a debugger, a button, the MCU's own NRST, a second supervisor.

**Why it matters.** Reset lines are wired-OR: several sources pull low, one pull-up lets go. The STM32's NRST is itself
an output: internal reset sources drive the pin low through a pulse generator for at least 20 µs (AN5096 §1.3.2). A
push-pull output holding the line high fights that, the debugger's reset and the button, and the reset either does not
happen or the drivers heat.

**Example.** A supervisor with push-pull `RESET` output (a TPS3823; only the TPS3828 of that family is open drain,
TPS3823 §7.3.3) on the MCU's NRST and the SWD connector's nRESET: the probe's reset never gets the line low, and every
watchdog reset of the MCU shorts its NRST driver into the supervisor's high output.

**How to check.** Reset nets (`IMP-1` names) with an `output` (push-pull) pin and any other driver or a button; open
drain is `open_collector` (`IMP-1`, `NET-7` where the library is vague).

**Fix.** Open-drain sources only (supervisors with open-drain output, GPIOs in open-drain mode), or a series resistor
where a push-pull source cannot be avoided.

**References.** AN5096 §1.3.2, §6.1.2; TPS3823 §7.3.3.

## `CLK-15` One reset net shared by the MCU and its peripherals <a id="clk-15"></a>

⚠️ warning · needs: reset pins of all parts on the net · [catalogue entry](../checks.md#clk-15)

**Impl.** easy · depends on IMP-1

**What it catches.** The MCU's reset pin and the reset inputs of peripherals on one net.

**Why it matters.** Any reset of the MCU (debugger, watchdog, software reset, which on STM32 all drive NRST low, AN5096
§1.3.2) then resets every peripheral: configurations are lost on each firmware restart, a debugger reset during bring-up
takes the whole board down, and a peripheral the MCU needs during recovery is held in reset (the same trap as `IO-13`).

**Example.** An MCU and an Ethernet PHY on one `NRST` net: every software reset re-runs the PHY's strap latch and
auto-negotiation, dropping the link for two seconds on each firmware update.

**How to check.** Nets that carry the reset pin of a controller and reset pins of other parts (`IMP-1`).

**Fix.** Give peripherals their own reset from a GPIO (with a pull, `IO-13`) or from the power supervisor, and keep the
MCU's NRST for the MCU and the debugger.

**References.** AN5096 §1.3.2; USB251xB §5.5.1 (what a hub loses on reset).

## `CLK-16` RTC in use with VBAT floating, no backup source, or no 32 kHz crystal <a id="clk-16"></a>

⚠️ warning · needs: whether the RTC is used, VBAT and LSE pins · [catalogue entry](../checks.md#clk-16)

**Impl.** mid · depends on NET-12, CFG-2

**What it catches.** A design that uses the RTC (timestamps, wake-up, calendar) while VBAT is unconnected, tied to
nothing that holds it across power loss, or the RTC has no 32.768 kHz crystal.

**Why it matters.** VBAT powers the RTC and backup registers when VDD is off; ST recommends tying VBAT to VDD when no
battery is used (AN5096 §1.1.1), and a floating VBAT leaves the backup domain undefined. Without an LSE crystal the RTC
runs from an internal RC oscillator that is off by percent: minutes to hours per day.

**Example.** A data logger that timestamps on an STM32 RTC with VBAT on a test point only: every power cycle resets the
clock to 2000-01-01, and on the LSI the time drifts by minutes or more a day while powered.

**How to check.** VBAT pins unconnected or on a net with no source; RTC use from the project (`CFG-2`) without a crystal
on the LSE pins (`NET-12`).

**Fix.** A coin cell, a supercap or VDD on VBAT, and a 32.768 kHz crystal (`CLK-11`).

**References.** AN5096 §1.1.1, §2.5; Wikipedia, Real-time clock.

## `CLK-17` External watchdog with no way to disable it for debugging <a id="clk-17"></a>

💡 suggestion · needs: watchdog parts, their WDI pins · [catalogue entry](../checks.md#clk-17)

**Impl.** mid · depends on NET-7

**What it catches.** An external watchdog (supervisor with WDI) whose kick input cannot be disconnected or overridden
with a jumper, a DNP resistor or a switch.

**Why it matters.** A debugger halt stops the kicks; the watchdog resets the target mid-debug session, and step-through
debugging is impossible. Some parts disable the watchdog when WDI floats (TPS3823 §7.3.4), which makes a DNP series
resistor on WDI a clean switch.

**Example.** A TPS3823 with WDI hard-wired to an MCU GPIO: every breakpoint longer than its 1.6 s timeout resets the
board. A 0 Ω in series, DNP for debug builds, disables the watchdog.

**How to check.** Watchdog-role parts (`NET-7`) whose WDI connects to the controller without a resistor, jumper or
connector on the way.

**Fix.** A 0 Ω or jumper in the WDI line (on parts that disable on high-Z), or an enable pin on a jumper.

**References.** TPS3823 §7.3.4; Wikipedia, Watchdog timer.

## `CLK-18` Clock output fanned out to several loads without a buffer or series termination <a id="clk-18"></a>

💡 suggestion · needs: clock nets, loads per net · [catalogue entry](../checks.md#clk-18)

**Impl.** easy · depends on IMP-1, NET-12

**What it catches.** An oscillator or MCO/clock output driving two or more inputs directly, with no series resistor at
the driver and no buffer.

**Why it matters.** Clock edges are fast; a branched route rings, and each load sees the reflections of the others.
Double-clocking and jitter follow, and the oscillator's output stage may not be specified for the combined load. A clock
buffer gives each load its own output, with a series resistor near the driver (CDCLVC1102 §8.1).

**Example.** A 25 MHz oscillator feeding an Ethernet PHY and an MCU's HSE input over a T-shaped route: the PHY links
fine, the MCU's PLL occasionally loses lock.

**How to check.** Clock nets (oscillator outputs, MCO, `*CLK*` names, `NET-12`) with more than one input and no resistor
in series at the driver (`IMP-1`).

**Fix.** A clock buffer (CDCLVC11xx class) or one series resistor per branch at the driver; daisy-chain only for low
frequencies.

**References.** CDCLVC1102 §8.1, §9.2.1; Johnson & Graham chapter 11.

## `CLK-19` Crystal tolerance vs the interface that uses it <a id="clk-19"></a>

⚠️ warning · needs: crystal ppm (tolerance, stability, aging), interfaces · [catalogue entry](../checks.md#clk-19)

**Impl.** mid · depends on NET-12, CFG-2

**What it catches.** A crystal whose total error (tolerance at 25 °C + stability over temperature + aging) exceeds what
the interface clocked from it allows.

**Why it matters.** USB high speed allows ±500 ppm and full speed ±0.25 % (USB 2.0 §7.1.11); Ethernet's PHY budget is
±50 ppm in practice (the LAN8720A asks for about ±45 ppm tolerance plus stability, Table 5-13, under IEEE 802.3's ±100
ppm); radios are tighter (Espressif wants the ESP32's 40 MHz within ±10 ppm, ESP32 checklist); CAN FD needs more
precision as the data phase gets faster (ISO 11898-1). A crystal picked for "MHz and footprint" often has ±30 ppm
tolerance plus ±50 ppm over temperature.

**Example.** A 25 MHz crystal with ±30 ppm tolerance and ±50 ppm stability on a LAN8720A: up to 80 ppm plus aging,
outside the PHY's budget; links work with most switches and fail with some at temperature.

**How to check.** The crystal's ppm figures from its MPN, the interfaces it clocks (`NET-12`) and the project's
temperature range (`CFG-2`); sum and compare.

**Fix.** A tighter crystal, or a TCXO for radios and wide temperature.

**References.** USB 2.0 §7.1.11; LAN8720A §5.6, Table 5-13; ESP32 checklist (Clock Source); ISO 11898-1, *Road vehicles
— Controller area network (CAN) — Part 1: Data link layer and physical signalling* (by title).
