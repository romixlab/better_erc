# IO checks, explained

Detail page for the `IO` category of the [check catalogue](../checks.md#%EF%B8%8F-io---default-states-and-pulls). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs
to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for
ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to build in better_erc without an LLM: `easy` (the netlist alone), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet) or `judgement` (a human or an LLM
decides, better_erc lists candidates), followed by the [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **SCBA004E** · TI, *Implications of Slow or Floating CMOS Inputs* (rev. 2021). What a floating input does (§1), pull
  resistors (§5), bus-hold (§6). [ti.com/lit/an/scba004e](https://www.ti.com/lit/an/scba004e/scba004e.pdf)
- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, Rev 4 (2026).
  Pull-up calculation (§5.1.5), no floating unused pins (§6.1), open-drain with no load (§6.4), debug pins with pulls by
  default (§6.6), NRST is no enable (§6.7).
  [st.com/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **AN2606** · ST, *Introduction to system memory boot mode on STM32 MCUs*, Rev 70 (2026). Hardware requirements (§4.3),
  the STM32G0B1 bootloader and the pins it configures (§50.1, Table 111).
  [st.com/an2606](https://www.st.com/resource/en/application_note/an2606-introduction-to-system-memory-boot-mode-on-stm32-mcus-stmicroelectronics.pdf)
- **RP2350** · Raspberry Pi, *RP2350 Datasheet*: GPIO reset state (§9.3, pulled low), erratum RP2350-E9 (§D.5.1).
  [datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- **SN74LVC595A** · TI datasheet SCASE93A (2025): power-up state of the registers and the OE pull-up (§8.1).
  [ti.com/lit/ds/symlink/sn74lvc595a.pdf](https://www.ti.com/lit/ds/symlink/sn74lvc595a.pdf)
- **USB251xB** · Microchip, *USB251xB/xBi USB 2.0 Hi-Speed Hub Controller*, DS00001692C: configuration options (Table
  5-1), SMBus configuration (§5.3), RESET_N (§5.5.1).
  [ww1.microchip.com/…/00001692C.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf)
- **LAN8720A** · Microchip, *LAN8720A/LAN8720AI* datasheet, DS00002165B: configuration straps (§3.7).
  [ww1.microchip.com/…/00002165B.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00002165B.pdf)
- **Ganssle** · Jack Ganssle, *A Guide to Debouncing* (two parts).
  [ganssle.com/debouncing.htm](https://www.ganssle.com/debouncing.htm),
  [ganssle.com/debouncing-pt2.htm](https://www.ganssle.com/debouncing-pt2.htm)
- **Wikipedia, Pull-up resistor**, **Open collector** and **Switch** (contact bounce) · readable overviews.
  [en.wikipedia.org/wiki/Pull-up_resistor](https://en.wikipedia.org/wiki/Pull-up_resistor),
  [en.wikipedia.org/wiki/Open_collector](https://en.wikipedia.org/wiki/Open_collector),
  [en.wikipedia.org/wiki/Switch](https://en.wikipedia.org/wiki/Switch#Contact_bounce)
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 12 (logic interfacing).

## `IO-1` Input pin without a drive source <a id="io-1"></a>

⚠️ warning · needs: pin types, smart about resistors and FETs · [catalogue entry](../checks.md#io-1)

**Impl.** easy · depends on IMP-1, NET-10

**What it catches.** A net with one or more input pins and nothing that can set its level: no output, no bidirectional
pin, no resistor to a rail, no FET or switch that connects it to one.

**Why it matters.** A CMOS input left open floats somewhere between the rails, picks up whatever couples into it and can
sit in the threshold region, where both input transistors conduct: random readings, oscillation, and several
milliamperes of shoot-through per input (SCBA004E §1). On an enable or a mode pin the part does something different from
board to board.

**Example.** A comparator's output stage removed in a revision while the MCU's `~ALERT` input stays on the net with
nothing else: the IRQ fires at random and the low-power mode never holds.

**How to check.** From KiCad pin types (`IMP-1`), list nets whose pins are all inputs (and passives). A resistor counts
as a source when its far end is a rail or a driven net; a FET or switch counts when its other side is (`NET-10`); a
connector alone does not (`IO-10`). The code collects such inputs today without reporting them.

**Fix.** A driver or a pull to the safe level.

**References.** SCBA004E §1, §5; AN4899 §6.1.

## `IO-2` IO to an input without pull-up/down <a id="io-2"></a>

⚠️ warning · needs: which drivers are MCU GPIOs (tristate at reset) · [catalogue entry](../checks.md#io-2)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** An input driven only by an MCU or SoC GPIO, with no external pull.

**Why it matters.** A GPIO is high-Z from power-on until the firmware configures it, and again during every reset, in a
ROM bootloader, when the core is halted by a debugger, and when the MCU is unpowered. In those windows the input floats
(`IO-1`). Internal pulls do not help on parts whose reset state is "no pull" (most STM32 pins), and an MCU's pad pull is
often 40-50 kΩ.

**Example.** An MCU GPIO drives the `EN` of a motor driver directly: during the 2 ms between power-on and GPIO init the
driver's EN floats, and on some boards the motor twitches at every reset.

**How to check.** Nets whose only drivers are MCU/SoC IOs (`NET-9` treats them as tristate) and whose loads are inputs,
with no resistor to a rail.

**Fix.** A pull to the level that is safe while the controller is not running.

**References.** SCBA004E §5; AN4899 §6.1; Wikipedia, Pull-up resistor.

## `IO-3` Enable driven by an MCU or SoC GPIO without a pull <a id="io-3"></a>

🛑 error · needs: enable pin roles, GPIO default states, `default_gpio_state` · [catalogue entry](../checks.md#io-3)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** The special case of `IO-2` that hurts: an enable, power-switch control, reset or chip select of
another part driven by a GPIO with no pull, unless the GPIO's reset state is declared with the `default_gpio_state` part
property.

**Why it matters.** Enables decide whether rails, outputs and motors are on. Undefined during boot means a rail may come
up when it should not, a load switch connects VBUS to a port before the firmware has negotiated anything, or a regulator
glitches on every reset. Reboots happen more often than power-ups: watchdog, debugger reset, firmware update.

**Example.** B153A drives every select and enable from the MCU and gives each one a pull-down, on purpose, so that a
reset or a halted MCU means VBUS to the device under test off, the level shifter off and the header VBUS off (B153A
hardware reference §14). B200A rev A missed one: `EXP.~OE` of its shift registers (`IO-12`).

**How to check.** For each pin with an enable or chip-select role (`NET-7`) driven by an MCU GPIO (`NET-9`), require a
resistor to a rail on the net, or a `default_gpio_state` on the driving pin that the project vouches for.

**Fix.** A pull to the safe state: off for power, deselected for chip selects, held in reset only where that is the
intent (`IO-13`).

**References.** SCBA004E §5; RP2350 §9.3 (GPIOs pulled low at reset); AN4899 §6.1.

## `IO-4` Assume IO pins are high/low/Z by default and look for bugs <a id="io-4"></a>

⚠️ warning · needs: per-pin reset state, signal following · [catalogue entry](../checks.md#io-4)

**Impl.** hard · depends on NET-8, NET-9, NET-10

**What it catches.** Consequences that only appear once the controller's pins are taken at a specific default: what
happens downstream if every GPIO is high-Z, or high, or low, at the same time.

**Why it matters.** A board where each enable has a pull can still combine them badly: two pulls that each choose "on"
for a different source of the same rail, a mux whose select and enable defaults route a 5 V line into a 3.3 V port.
Walking the board in each default state finds these before the smoke does.

**Example.** On B200A the shift-register outputs are 32 enables; "all zeros" is safe except EX13, whose zero turns the
3.3 V VREF source on, so a cleared chain plus one wrong bit puts VBUS and 3.3 V on `+VREF` together (B200A hardware
reference §4-5).

**How to check.** Assign each controller IO a default (Z per `NET-9`, then high, then low), propagate through pulls,
FETs and logic (`NET-10`) and evaluate the resulting rail and switch states (`NET-8`) against conflicts: two sources on
one rail, a switch closed into an unpowered domain.

**Fix.** Pulls and logic chosen so that the "nothing is driven" state is a known safe state.

**References.** SCBA004E §4-5 (bus control and pulls); AoE3 chapter 12.

## `IO-5` If IO pin default states are known, derive checks <a id="io-5"></a>

⚠️ warning · needs: per-pin reset states from part data or the project · [catalogue entry](../checks.md#io-5)

**Impl.** hard · depends on NET-9, NET-10, CFG-2

**What it catches.** The precise version of `IO-4`: with the real reset state of each pin (from the datasheet, or
declared in the project), the same propagation without guessing.

**Why it matters.** Reset states are not uniform: the RP2350 pulls every Bank 0 GPIO low with its input buffer off
(RP2350 §9.3); STM32 SWD pins come up with a pull-up (SWDIO) and a pull-down (SWCLK) and everything else floating
(AN4899 §6.6); a ROM bootloader then changes several of them (`IO-9`). A pull-down on an enable is redundant on the
RP2350 and essential on the STM32.

**Example.** RP2350B GPIO27 drives B200A's 595 `~OE`: the pad pull-down holds it low at reset, so the shift registers
are enabled with undefined content (`IO-12`); with the known reset state the check finds this without a guess.

**How to check.** Read reset states per pin (part data or `default_gpio_state` via `CFG-2`), then run the `IO-4`
propagation (`NET-10`).

**Fix.** As `IO-4`.

**References.** RP2350 §9.3; AN4899 §6.6.

## `IO-6` High impedance nets acting as touch buttons <a id="io-6"></a>

⚠️ warning · needs: connectivity, pin types · [catalogue entry](../checks.md#io-6)

**Impl.** easy · depends on IMP-1

**What it catches.** A high-impedance input net that reaches something a finger, a cable or a probe can touch: a
connector, a test pad, a button or a long trace, with no pull and no driver.

**Why it matters.** A floating input with a few pF to the outside world is a capacitive touch sensor: touching the
connector, bringing a hand close, or ESD flips the level. The bug reports read "the board resets when I touch it".

**Example.** A header pin `EXT_TRIG` wired straight to an MCU input with only a 100 Ω series resistor: with no cable
plugged, a finger near the header triggers acquisitions.

**How to check.** `IO-1` nets that reach a connector, test point or switch part (`IMP-1` part classes).

**Fix.** A pull (10-100 kΩ) on the board side, and ESD protection if it leaves the enclosure.

**References.** SCBA004E §1; AN4899 §6.1 (noise on unconnected inputs).

## `IO-7` Button to ground with a pull-down, or button to power with a pull-up <a id="io-7"></a>

🛑 error · needs: switch parts, resistors to rails · [catalogue entry](../checks.md#io-7)

**Impl.** easy · depends on NET-5

**What it catches.** A switch whose closed state pulls the net to the same level the resistor already holds it at.

**Why it matters.** The button does nothing: pressed and released read the same. It is a copy error (a pull-up footprint
wired to GND) that the ERC never sees, because every pin is connected.

**Example.** SW1 between `BTN` and GND, R12 10 kΩ from `BTN` to GND as well: the input reads low forever.

**How to check.** For each switch, the rail on its far side (`NET-5`) must differ from the rail of every pull on the
input side.

**Fix.** Pull towards the other rail.

**References.** Wikipedia, Pull-up resistor; Wikipedia, Switch (contact bounce).

## `IO-8` Mention a needed pull-up or down where it applies <a id="io-8"></a>

💡 suggestion · needs: switch parts, pins with internal pulls · [catalogue entry](../checks.md#io-8)

**Impl.** easy · depends on NET-5, RPT-13

**What it catches.** A button or open-drain source with no external pull, where the firmware must enable an internal
one: report it, so the firmware does.

**Why it matters.** A button to GND on a GPIO with no external pull-up works only with the internal pull-up enabled;
nothing in the schematic tells the firmware writer. Some parts make internal pulls unusable: on the RP2350 A2, an input
with only the pad pull-down can latch at about 2.2 V (erratum E9), so a button to 3.3 V needs an external pull-down of
8.2 kΩ or less.

**Example.** B200A rev A's user button SW101 connects GPIO22 to 3.3 V with no external pull-down: the internal one is
needed, and E9 makes it unreliable; rev A got a 6 kΩ rework resistor, rev B an external pull-down.

**How to check.** Switch to a rail with no opposite pull on the input side: add an entry "needs internal pull-up/down"
to the programmer report (`RPT-13`), and an error where the part's errata rule internal pulls out.

**Fix.** Either an external pull, or a line in the programmer report and the firmware's pin setup.

**References.** RP2350 §D.5.1 (RP2350-E9); B200 rev B change list; Ganssle part 2 (RC debouncer).

## `IO-9` Pins a part drives in its non-firmware modes <a id="io-9"></a>

🛑 error · needs: ROM bootloader pin tables, what each pin controls · [catalogue entry](../checks.md#io-9)

**Impl.** hard · depends on NET-10, NET-12, CFG-2

**What it catches.** Pins that a controller drives or toggles while no application firmware runs (ROM bootloader,
factory test, reset, fault states), where the board uses them for something that reacts to that level.

**Why it matters.** A ROM bootloader configures the interfaces it listens on: the STM32G0B1's enables USART1-3, I2C1-3,
SPI1-2, FDCAN1 and USB, with USART TX pins as push-pull outputs with pull-ups (AN2606 Table 111). The board's firmware
may keep those pins in a safe state, but during a ROM update there is no firmware, and whatever hangs on a TX pin sees a
high.

**Example.** On B153A, PA9 (`DATA_CUT`) selects the USB data switch for the device under test, with a pull-down for
"connected". PA9 is USART1_TX of the G0B1 ROM bootloader, which drives it high while it runs, so every firmware update
through the ROM disconnects the DUT's USB data (typec_dongle FW-50; B153 rev B change list).

**How to check.** For each programmable part, take its ROM and reset-time pin table (part data per family, `NET-12`),
then follow each listed pin to what it controls (`NET-10`) and evaluate the bootloader's level there; the board's wanted
state per mode comes from the project (`CFG-2`).

**Fix.** Move the control to a pin the ROM leaves alone, or make the bootloader's level the harmless one.

**References.** AN2606 §4.3, §50.1 Table 111; STM32G0B1 datasheet §3.5 (bootloader configures GPIOs out of high-Z)
[st.com/stm32g0b1re.pdf](https://www.st.com/resource/en/datasheet/stm32g0b1re.pdf); B153 rev B change list.

## `IO-10` Input driven only from a connector <a id="io-10"></a>

⚠️ warning · needs: connector parts, pin types · [catalogue entry](../checks.md#io-10)

**Impl.** easy · depends on IMP-1

**What it catches.** An input whose only possible driver is off-board: a pin of a connector, with no pull on this board.

**Why it matters.** Unplugged, the input floats (`IO-1`), and it is unplugged during assembly test, on the bench, and
whenever a cable falls out in the field. Firmware then sees random edges on what should be a quiet line.

**Example.** A `REMOTE_EN` input from a 2-pin terminal block meant for a dry contact to GND: without a pull-up, "nothing
connected" reads randomly, and the machine starts by itself.

**How to check.** `IO-1` nets whose only non-input parts are connectors (`IMP-1` part class).

**Fix.** A pull to the inactive level on this board; ESD protection at the connector.

**References.** SCBA004E §5; AN4899 §6.1.

## `IO-11` MOSFET gate without a pull <a id="io-11"></a>

🛑 error · needs: FET parts with named pins (`LIB-3`), resistors · [catalogue entry](../checks.md#io-11)

**Impl.** easy · depends on IMP-1

**What it catches.** A MOSFET whose gate has no resistor to its source (or to the rail that means "off"), driven by
something that can be high-Z: an MCU GPIO, a driver in reset, a connector.

**Why it matters.** A MOSFET gate is a capacitor with gigaohms of leakage. Undriven it keeps whatever charge it last
had, or picks up drain-gate coupling from the switching node: a load switch half on, a power FET in its linear region
getting hot, a reverse-polarity FET that conducts when it should not.

**Example.** A low-side NMOS for a solenoid, gate on an MCU GPIO through 100 Ω and nothing else: during reset the
solenoid clicks, and with the MCU unpowered the drain's dV/dt from another load turns it on.

**How to check.** For each MOSFET (pins named per `LIB-3`), the gate net needs a resistor to the source net or to the
off-rail, unless its driver is a push-pull output that is valid in every state (a gate driver IC).

**Fix.** 10-100 kΩ from gate to source (pull-down for NMOS, pull-up to the source rail for PMOS).

**References.** AoE3 chapter 3 (FET switches) and chapter 12; Wikipedia, Pull-up resistor.

## `IO-12` Output enable of a shift register, driver or buffer without a pull to its disabled state <a id="io-12"></a>

🛑 error · needs: OE pin role and polarity, GPIO default · [catalogue entry](../checks.md#io-12)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** An `~OE`, `~G` or `EN` input of a part with outputs (shift register, buffer, driver, mux) that has
no pull to its disabled state, so the outputs are live before the firmware loads them.

**Why it matters.** The registers of a 74x595 power up in an unknown state (SN74LVC595A §8.1); if `~OE` is low, those
random bits go straight to the outputs. Clearing with `~SRCLR` does not help: it clears the shift register, not the
output register. TI's own application circuit pulls `~OE` up so the power-up mess stays high-Z.

**Example.** B200A drives `~OE` of four SN74LVC595s from RP2350 GPIO27 with no pull-up. The RP2350's pad pull-down holds
it low from reset, so the 32 control outputs (VREF source selection, translator enables, IO directions) show random
content until the firmware runs; a bad pattern can turn on two VREF sources at once, shorting VBUS into the 3.3 V rail.
Rev A was reworked with a 12 kΩ pull-up, rev B gets a 10 kΩ one (B200 rev B change list, B200A assembly report).

**How to check.** For each part with an output-enable role (`NET-7`), the OE net needs a pull to the disabling level
(polarity from the pin name), or a driver whose reset state is that level (`NET-9`).

**Fix.** Pull OE to "disabled" (10 kΩ to VCC for `~OE`); the firmware loads a safe pattern, then enables.

**References.** SN74LVC595A §8.1; RP2350 §9.3; B200A hardware reference §4.

## `IO-13` Peripheral reset pin pulled into reset by default, driven by a controller that resets <a id="io-13"></a>

⚠️ warning · needs: reset pin roles, which controller drives them · [catalogue entry](../checks.md#io-13)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** A peripheral's reset input with a pull into reset (pull-down on `RESET_N`), driven by an MCU or SoC
that itself resets: every controller reset, watchdog, debugger reset or jump into a ROM bootloader puts the peripheral
back into reset.

**Why it matters.** A pull into reset is a safe default for a peripheral the controller must configure first. But the
peripheral then loses its configuration on every controller reset, and if the controller needs that peripheral to reach
the outside world (a USB hub, a PHY, a level shifter) its own recovery path goes dark (`LOGIC-5`, `BOOT-6`).

**Example.** B153A's USB hub (USB2514B/2512B) has `RESET_N` on PC13 with R304 10 kΩ to GND. The hub is strapped for
SMBus configuration and waits for the MCU after every reset (USB251xB §5.3). When the MCU resets into its ROM
bootloader, the pull-down holds the hub in reset and the ROM's USB DFU never reaches the host. The rev B plan is a
pull-up, with PC13 still able to pull the hub low for recovery.

**How to check.** Reset-role pins (`NET-7`) with a pull towards the asserted level and a driver on an MCU/SoC GPIO
(`NET-9`); report them with the question "should the peripheral survive a controller reset?".

**Fix.** Decide on purpose: pull out of reset with the controller able to assert it, or keep the pull-down and make sure
nothing the controller needs for recovery depends on that peripheral.

**References.** USB251xB Table 5-1, §5.3, §5.5.1; B153 rev B change list (ST ROM bootloader test, 21 Oct 2026).

## `IO-14` Configuration, mode or address straps of a non-MCU part floating <a id="io-14"></a>

🛑 error · needs: strap pin roles and their required levels · [catalogue entry](../checks.md#io-14)

**Impl.** mid · depends on NET-7, CFG-5

**What it catches.** A strap pin of a PHY, hub, codec, sensor or regulator left unconnected where the datasheet requires
a level, or loaded so that its internal pull no longer sets it.

**Why it matters.** Straps are latched at power-on or reset; a floating strap is latched at whatever the pin happened to
be. Internal strap pulls are weak (tens of kΩ) and lose against an LED, an input with a pull or a long trace: LAN8720A
asks for an external resistor on any strap that is connected to a load (LAN8720A §3.7).

**Example.** A LAN8720A's `RXER/PHYAD0` routed to an MCU input with a pull-down enabled: the PHY latches address 0 on
one board and 1 on another (when the MCU pull comes up late), and the driver finds no PHY on half the builds.

**How to check.** Pins with a strap role (`NET-7`, from part data or a part rule in `CFG-5`): unconnected, or connected
to anything other than a resistor to a rail without such a resistor.

**Fix.** An explicit pull per strap, strong enough to win against the load (LAN8720A: an external resistor "augmenting"
the internal one), and the latched value written next to it in the schematic.

**References.** LAN8720A §3.7; USB251xB Table 5-1; SCBA004E §5.

## `IO-15` Open-drain interrupt or fault output without a pull-up <a id="io-15"></a>

🛑 error · needs: open-drain pin types · [catalogue entry](../checks.md#io-15)

**Impl.** easy · depends on IMP-1, NET-7

**What it catches.** An `~INT`, `~ALERT`, `~FAULT`, `PGOOD` or similar open-drain output whose net has no pull-up and no
other driver.

**Why it matters.** An open-drain output only pulls low. Without a pull-up the line never goes high: the controller sees
a permanent interrupt or fault, or (with an internal pull-up it forgets to enable) a floating line. A fault signal that
cannot go inactive is a fault signal nobody looks at.

**Example.** B153A's `~PWR_FLT` (load switch FLTB on PC14) has no external pull-up; the firmware must enable the MCU's
pull-up, and with the MCU in reset the fault line floats (B153A hardware reference §3).

**How to check.** KiCad's `open_collector`/`open_emitter` pin type (`IMP-1`), or open-drain roles from part data
(`NET-7`), on a net with no resistor to a rail.

**Fix.** A pull-up to the receiver's IO rail (10-100 kΩ), or a line in the programmer report (`RPT-13`) if the internal
pull is intended.

**References.** AN4899 §6.4; Wikipedia, Open collector.

## `IO-16` Pull-up and pull-down both on one net <a id="io-16"></a>

🛑 error · needs: resistors to rails · [catalogue entry](../checks.md#io-16)

**Impl.** easy · depends on NET-5

**What it catches.** A net with a resistor to a power rail and another to ground (and nothing that makes it a deliberate
divider, such as an ADC input or a feedback pin).

**Why it matters.** The two form a divider; the net sits at a ratio of the rail that may be inside the threshold region
of the input, so neither level is clean. Usually one of them is a leftover (a DNP pull that got fitted, a pull-up added
in rev B next to the old pull-down).

**Example.** 10 kΩ to `+3V3` and 10 kΩ to GND on a boot strap: 1.65 V, read as either level depending on the part.

**How to check.** Resistors from one net to a rail and to a ground (`NET-5`), excluding nets that reach analog inputs or
feedback pins; report the divided voltage.

**Fix.** Remove one, or make the intent explicit (a DNP pair for a strap option).

**References.** SCBA004E §1, §5.

## `IO-17` Pull value too weak for the leakage, or too strong for the driver <a id="io-17"></a>

⚠️ warning · needs: input leakage, driver IOL and VOL per pin · [catalogue entry](../checks.md#io-17)

**Impl.** mid · depends on NET-7

**What it catches.** A pull so weak that the leakage on the net (input leakage, ESD diodes, a capacitor's leakage, a
part's erratum) moves the level across the threshold; or so strong that the driver cannot pull against it to a valid low
(or high).

**Why it matters.** The window is IOL and VOL on one side (Rp ≥ (VDD − VOL) / IOL) and leakage times Rp against the
noise margin on the other (AN4899 §5.1.5). 1 MΩ against a few µA of leakage is volts; 1 kΩ on a 3 mA open-drain output
gives a low above VIL.

**Example.** B200A's button input on an RP2350 A2: erratum E9 lets the pad source about 120 µA around 2.2 V, so only a
pull-down of 8.2 kΩ or less keeps the input low after release (RP2350 §D.5.1); the internal pull-down (tens of kΩ) does
not.

**How to check.** For each pull, the sum of leakage on the net (part data, `NET-7`) times Rp against the noise margin,
and the strongest driver's IOL at VOL(max) against (VDD − VOL) / Rp.

**Fix.** A value inside the window; for slow, high-leakage inputs a stronger pull and a filter capacitor.

**References.** AN4899 §5.1.5; RP2350 §D.5.1; SCBA004E §5 (1-10 kΩ typical).

## `IO-18` Button without a debounce capacitor where the firmware assumes an RC filter <a id="io-18"></a>

💡 suggestion · needs: switch parts, the firmware's assumption · [catalogue entry](../checks.md#io-18)

**Impl.** mid · depends on CFG-2, RPT-13

**What it catches.** A button on a GPIO with no capacitor to ground, where the firmware relies on a hardware filter (no
software debounce, or an edge interrupt that counts presses).

**Why it matters.** Contacts bounce for hundreds of µs to several ms; an interrupt sees each bounce as a press, and an
analog-decoded button (several buttons on one ADC pin through a ladder) reads intermediate values during the bounce.
Firmware and hardware must agree on who filters.

**Example.** B153A's PA7 carries a button (10 kΩ to GND) and a slide switch on one ladder decoded by `button.rs`, which
assumes an RC filter; the board has none. Rev B adds 10-100 nF to GND.

**How to check.** Buttons (switch parts) on GPIO nets without a capacitor to ground; whether the firmware assumes a
filter comes from the project (`CFG-2`), and the programmer report states "hw debounce: none" either way (`RPT-13`).

**Fix.** A capacitor (10-100 nF with the existing pull, giving 0.1-1 ms) and, for slow RC edges, a Schmitt-trigger
input.

**References.** Ganssle part 2 ("An RC Debouncer"); Wikipedia, Switch (contact bounce); B153 rev B change list.
