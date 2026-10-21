# LOGIC checks, explained

Detail page for the `LOGIC` category of the [check catalogue](../checks.md#-logic---pin-logic-and-fan-out). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find
it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to build in better_erc without an LLM: `easy` (the netlist alone), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet) or `judgement` (a human or an LLM
decides, better_erc lists candidates), followed by the [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, Rev 4 (2026).
  Voltage output and current drive (§5.1.4), pull-up calculation (§5.1.5), open-drain with no load (§6.4).
  [st.com/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **STM32G0B1** · ST datasheet DS13560 Rev 6 (2026): Table 22 *Current characteristics* (15 mA per pin, 80 mA sourced by
  all IOs together). [st.com/stm32g0b1re.pdf](https://www.st.com/resource/en/datasheet/stm32g0b1re.pdf)
- **AN2606** · ST, *Introduction to system memory boot mode on STM32 MCUs*, Rev 70 (2026): the STM32G0B1 bootloader
  (§50.1).
  [st.com/an2606](https://www.st.com/resource/en/application_note/an2606-introduction-to-system-memory-boot-mode-on-stm32-mcus-stmicroelectronics.pdf)
- **USB251xB** · Microchip, *USB251xB/xBi USB 2.0 Hi-Speed Hub Controller*, DS00001692C: configuration options (Table
  5-1), SMBus configuration (§5.3), RESET_N (§5.5.1).
  [ww1.microchip.com/…/00001692C.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf)
- **SCBA004E** · TI, *Implications of Slow or Floating CMOS Inputs* (rev. 2021): slow input edges (§2).
  [ti.com/lit/an/scba004e](https://www.ti.com/lit/an/scba004e/scba004e.pdf)
- **CDCLVC1102** · TI, *CDCLVC11xx 3.3-V and 2.5-V LVCMOS High-Performance Clock Buffer Family* datasheet SCAS895B
  (2017): series resistor near the driver (§8.1, §9.2.1).
  [ti.com/lit/ds/symlink/cdclvc1102.pdf](https://www.ti.com/lit/ds/symlink/cdclvc1102.pdf)
- **Wikipedia, Fan-out** and **Open collector** · readable overviews.
  [en.wikipedia.org/wiki/Fan-out](https://en.wikipedia.org/wiki/Fan-out),
  [en.wikipedia.org/wiki/Open_collector](https://en.wikipedia.org/wiki/Open_collector)
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 10 (digital logic) and
  chapter 12 (logic interfacing).

## `LOGIC-1` Logical contradictions in GPIO or GPIO expander connections <a id="logic-1"></a>

🛑 error · needs: what each GPIO controls, signal following · [catalogue entry](../checks.md#logic-1)

**Impl.** judgement · depends on NET-10, RPT-14

**What it catches.** A GPIO (or an expander or shift-register output) that drives several things at once whose wanted
levels conflict: one level enables part A and must disable part B, but the two are needed together; or two outputs meant
to be mutually exclusive that one bit can set together.

**Why it matters.** Sharing a pin to save one is common, and the conflict shows up only in a mode that nobody tried on
the bench. Since the catalogue's own idea is a per-occurrence table, the check is mostly a review aid: list every GPIO
with more than one load and what each level does at each of them.

**Example.** On B200A one 595 bit (EX30) is both the low-voltage I2C translator's OE and, through Q301, the gate of the
bus switch: fine, as both must follow each other; while EX24 (5 V pull-ups) and EX30 (low path) must never be on
together, which only the firmware guarantees (B200A hardware reference §8). A table makes both cases visible.

**How to check.** For every controller output with more than one load, follow each branch (`NET-10`) to the pins it ends
on and their roles; produce the table (`RPT-14`). Whether a combination is a contradiction needs the intent: a human or
an LLM reads the table.

**Fix.** Separate pins for functions that must not move together; hardware interlocks (an inverter, a NOR) for
combinations that must never happen.

**References.** AoE3 chapter 10; B200 rev B change list (interlock for the VREF sources).

## `LOGIC-2` IO going to two MCUs or SoCs <a id="logic-2"></a>

⚠️ warning · needs: which parts are controllers · [catalogue entry](../checks.md#logic-2)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** A net with IO pins of two different programmable controllers.

**Why it matters.** Two controllers on one line need an agreement: who drives, when, and what each does while the other
is in reset or unpowered (backfeed through the unpowered one's clamp diodes). IRQs and buttons shared by two MCUs are
the classic trap: one MCU sets its pin as an output by mistake, or a button that should wake both also reaches a reset
input of one.

**Example.** A button on a line that goes both to the application MCU's wake-up pin and, via a forgotten net tie, to the
supervisor's `NRST`: pressing it to wake the board resets the supervisor.

**How to check.** Nets with IOs of two or more parts that are controllers (`NET-9`); report the pin roles on both sides
(`NET-7`) for review, and add a backfeed check when the two have different rails.

**Fix.** A series resistor (limits the fight and the backfeed), an open-drain arrangement with a pull-up, or a buffer
owned by one side; document which controller owns the line.

**References.** AN4899 §6.2 (cross-domain leakage); AoE3 chapter 12.

## `LOGIC-3` Suggest buffering high fan-out signals <a id="logic-3"></a>

💡 suggestion · needs: pin types, input count per net · [catalogue entry](../checks.md#logic-3)

**Impl.** easy · depends on IMP-1

**What it catches.** One output driving many inputs: a reset, a clock, a chip select or an enable that reaches a long
list of parts.

**Why it matters.** Each input adds capacitance (3-10 pF) and the trace adds more, so the edge slows: SPI clocks lose
margin, a slow edge on a non-Schmitt input causes multiple transitions and extra current (SCBA004E §2), and reflections
on a long star route double-clock. A buffer splits the load and gives each branch a clean edge.

**Example.** A 3.3 V MCU GPIO drives the latch clock of eight shift registers along a 20 cm board: 8 × 5 pF plus trace
is around 60 pF, the edge is still fine at 1 MHz but rings at the far end and occasionally double-latches.

**How to check.** Count input pins per net with one driver (`IMP-1`); suggest above a threshold (8 inputs, or a clock
net with more than two).

**Fix.** A buffer (74LVC1G125, or a clock buffer like the CDCLVC11xx for clocks), series termination at the driver, or a
daisy-chain route.

**References.** SCBA004E §2; CDCLVC1102 §8.1, §9.2.1 (series resistor near the driver); Wikipedia, Fan-out.

## `LOGIC-4` Total IO current per IC or bank <a id="logic-4"></a>

⚠️ warning · needs: load current per pin, simultaneous states · [catalogue entry](../checks.md#logic-4)

**Impl.** hard · depends on NET-7, NET-8, IMP-10

**What it catches.** The sum of currents that a part's IOs source or sink at the same time exceeding the per-bank or
per-package limit, although each pin stays within its own.

**Why it matters.** Datasheets limit not just each pin but the total through the supply and ground pins: the STM32G0B1
allows 15 mA per pin but 80 mA sourced by all IOs together (STM32G0B1 Table 22). Beyond that, VOH and VOL move out of
spec on every pin of the bank, and the bond wires and on-chip supply rail degrade.

**Example.** Eight LEDs at 10 mA each driven high from one STM32 port plus two 10 mA status LEDs: 100 mA sourced, above
the 80 mA total, although no pin is above 15 mA.

**How to check.** Estimate the load current of each output (LED resistor and forward voltage, pull-ups, input currents;
`NET-7`), group per bank and supply pin (`IMP-10`), and sum for the worst simultaneous state (`NET-8`).

**Fix.** Sink instead of source where the limit is asymmetric, drive LEDs through transistors or an LED driver, lower
the currents, or spread loads over banks.

**References.** STM32G0B1 Table 22; AN4899 §5.1.4.

## `LOGIC-5` Dependency loops in bring-up, update or recovery <a id="logic-5"></a>

🛑 error · needs: what configures what, reset states, boot paths · [catalogue entry](../checks.md#logic-5)

**Impl.** judgement · depends on NET-8, NET-10, NET-12

**What it catches.** Part A only works once part B has set it up, while B needs A to start, update or recover. The board
works while the firmware runs and fails exactly when the firmware is dead, half-written, in reset or in a ROM
bootloader.

**Why it matters.** Recovery paths are tested last, often never: flashing works through the debugger on the bench, so
nobody notices that the field update path depends on the firmware it is meant to replace. The same shape appears with
supervisors powered by what they supervise, PHYs or clocks configured by the chip they serve, configuration memories
written through the bus they configure, and muxes or level shifters on a programming path driven by the target.

**Example.** On B153A the STM32G0B1's only USB path goes through hub U301, strapped for SMBus configuration: out of
reset the hub waits for the MCU to configure it and does not attach before (USB251xB §5.3), and the hub's `RESET_N` has
a pull-down. After any MCU reset the hub is held in reset, so the ROM bootloader's USB DFU (AN2606 §50.1) runs but never
reaches the host. A jump into the ROM without a reset, hub left running, enumerates fine. Rev B: a pull-up on the hub's
reset and a mux that puts the MCU's USB on the connector directly (B153 rev B change list).

**How to check.** For each programmable part, take its boot, update and recovery interfaces (`NET-12`), follow them to
the outside world (`NET-10`), and mark every part on the way that needs configuration or a GPIO level from that same
part; evaluate with each programmable part in reset (`NET-8`). better_erc can list the paths and the suspicious parts;
whether a loop is real (a strap may make the hub work alone) needs a datasheet reading.

**Fix.** Make the recovery path work with the part in reset: straps or pulls that put the in-between parts in a working
default, a direct path for the ROM bootloader, or an independent programming connector.

**References.** USB251xB Table 5-1, §5.3; AN2606 §50.1; B153 rev B change list (ST ROM bootloader test, 21 Oct 2026).

## `LOGIC-6` Current through one pin above its maximum <a id="logic-6"></a>

🛑 error · needs: per-pin current limits, load currents · [catalogue entry](../checks.md#logic-6)

**Impl.** mid · depends on NET-7

**What it catches.** A logic output whose load draws more than the pin's absolute maximum: an LED with no resistor, a
relay or solenoid coil, a buzzer, a low-value pull, a heavy chain of inputs and pulls.

**Why it matters.** Beyond the rated current the output's VOH/VOL is meaningless, the pin's FETs heat and degrade, and
an inductive load's turn-off spike goes straight into the pin's clamp. The part does not fail at once, which is why it
ships.

**Example.** A 5 V relay coil (70 Ω, 70 mA) straight on a 3.3 V MCU pin rated 15 mA (STM32G0B1 Table 22): the relay
clicks weakly on the bench, the pin dies within weeks, and the coil's kickback has no diode.

**How to check.** For each output, the load current from the parts on its net (LED with its series resistor and Vf, coil
resistance from part data, pulls to the opposite rail; `NET-7`) against the pin's IOH/IOL maximum.

**Fix.** A series resistor, a transistor or MOSFET driver (with a flyback diode for coils), or a driver IC.

**References.** STM32G0B1 Table 22; AN4899 §5.1.4; AoE3 chapter 12.

## `LOGIC-7` Open-drain or open-collector output as the only driver of a net with no pull-up <a id="logic-7"></a>

🛑 error · needs: open-drain pin types · [catalogue entry](../checks.md#logic-7)

**Impl.** easy · depends on IMP-1, NET-7

**What it catches.** A net whose only drivers are open-drain or open-collector outputs (including MCU pins configured
open-drain, comparators, optocouplers, transistor stages) and that has no resistor to a rail.

**Why it matters.** Open-drain outputs only pull low; the high level has to come from a pull-up. Without one the net
floats when released: the receiver reads whatever leakage and noise decide (AN4899 §6.4).

**Example.** An LM393 comparator output into an MCU input, no pull-up, the MCU's internal pull-up never enabled: the
input reads low most of the time and "high" when a finger is near.

**How to check.** Nets whose output-capable pins are all `open_collector` / `open_emitter` (KiCad pin types, `IMP-1`) or
open-drain by part data (`NET-7`), with no resistor to a rail. `IO-15` is the same check narrowed to interrupt and fault
outputs.

**Fix.** A pull-up to the receiver's rail, sized per `IO-17`.

**References.** AN4899 §5.1.5, §6.4; Wikipedia, Open collector.

## `LOGIC-8` Polarity mismatch by name <a id="logic-8"></a>

⚠️ warning · needs: pin and net names · [catalogue entry](../checks.md#logic-8)

**Impl.** easy · depends on IMP-1

**What it catches.** An active-low output (`_N`, `#`, `~`, `n` prefix) wired to an active-high input or the reverse,
with no inverter between: `PGOOD` into `~EN`, `~INT` into a pin configured for rising-edge interrupts, `RESET` into
`~RST`.

**Why it matters.** Names are the only polarity information in a netlist, and a mismatch is either a bug (the regulator
is enabled exactly when the other is not good) or a deliberate inversion that deserves a comment. Both are cheap to
list.

**Example.** A sequencer wires rail A's `PG` (active high, "good") to rail B's `~SHDN` (active low, "shut down"): B runs
while A is not good and shuts down when A is good.

**How to check.** Parse active-low markers in pin names (and net names) on both ends of each net (`IMP-1`); report pairs
whose markers differ when no inverting part (logic inverter, transistor stage) sits between.

**Fix.** Fix the wiring, add an inverter, or rename so the polarity is visible.

**References.** AoE3 chapter 10 (active-low signals); the catalogue's NAME category (naming and conventions).
