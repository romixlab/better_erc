# SEQ checks, explained

Detail page for the `SEQ` category of the [check catalogue](../checks.md#️-seq---sequencing-enables-inrush-discharge).
Each section explains one check: what it catches, why the board fails without it, a concrete example, what
better_erc needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the
source of truth for ID, severity and wording; this page is the commentary.

References used throughout:

- **SLVA670** · TI, *Managing Inrush Current*, SLVA670A (August 2014, revised May 2015): what inrush is, the problems it
  causes, discrete and load-switch solutions, slew-rate control. [ti.com/lit/an/slva670a](https://www.ti.com/lit/an/slva670a/slva670a.pdf)
- **SLVA716** · TI, *Integrated Load Switches Versus Discrete MOSFETs*, SLVA716A (2015, revised 2019): PMOS discrete
  circuits, adding a resistor to slow the output rise time. [ti.com/lit/an/slva716](https://www.ti.com/lit/an/slva716/slva716.pdf)
- **SLVA883** · TI, *Timing of Load Switches*, SLVA883 (April 2017); §3.1 slew rate, §3.3 quick output discharge.
  [ti.com/lit/an/slva883](https://www.ti.com/lit/an/slva883/slva883.pdf)
- **SLVA470** · TI, *Sequencing and Tracking With the TPS621-Family and TPS821-Family*, SLVA470A (2011, revised 2017):
  sequencing with EN and PG, ratiometric and simultaneous start-up. [ti.com/lit/an/slva470](https://www.ti.com/lit/an/slva470/slva470.pdf)
- **AM335x** · TI, *AM335x Sitara Processors* datasheet, SPRS717L (revised March 2020); §6.1 power supplies (power-up
  sequencing figures with PWRONRSTn), §6.1.2 power-down sequencing. [ti.com am3358.pdf](https://www.ti.com/lit/ds/symlink/am3358.pdf)
- **TPS22918** · TI, *TPS22918 5.5-V, 2-A, 52-mΩ On-Resistance Load Switch* datasheet, SLVSD76C (2017); §8.3.2 quick
  output discharge (QOD). [ti.com tps22918.pdf](https://www.ti.com/lit/ds/symlink/tps22918.pdf)
- **Wikipedia** · [Inrush current](https://en.wikipedia.org/wiki/Inrush_current),
  [Power-on reset](https://en.wikipedia.org/wiki/Power-on_reset), [Bleeder resistor](https://en.wikipedia.org/wiki/Bleeder_resistor).
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 9 (voltage regulation and
  power conversion).

## `SEQ-1` Power sequencing: intermittent problems on power on or off <a id="seq-1"></a>

⚠️ warning · needs: power states and ramps per rail, part requirements · [catalogue entry](../checks.md#seq-1)

**Impl.** judgement · depends on NET-8

**What it catches.** A design where the order or timing in which rails come up and go down is not controlled, and
some part on the board is sensitive to it: the umbrella entry for start-up and shutdown problems that `SEQ-2`,
`SEQ-7`, `SEQ-8` and `SEQ-9` name one by one.

**Why it matters.** Rails ramp at different speeds depending on load, input voltage and temperature. A part powered
before its IO rail, an IO bank driven before its supply, a reset released while a rail is still ramping: each works
most of the time and fails one power-on in a hundred, the worst kind of field bug.

**Example.** A sensor on an always-on 3.3 V LDO and an MCU on a buck that starts 5 ms later: during those 5 ms the
sensor's interrupt output (push-pull, high) drives the unpowered MCU pin and back-powers its rail through the ESD
diode (`BKF`), so the MCU sometimes starts from a half-powered state and hangs.

**How to check.** Not a single rule: list the rails, their sources and enables, and the parts with sequencing
requirements; better_erc can draw the start-up order from EN/PG chains (`NET-8`), the judgement whether it is enough
stays with the reviewer.

**Fix.** Make the order explicit: EN from the previous rail's PG, a sequencer or PMIC, tracking (SS/TR) where the
datasheet asks for it; scope the ramps at cold and hot.

**References.** SLVA470 §2 (sequential start-up with EN and PG), §3-4 (ratiometric and simultaneous); AM335x §6.1.

## `SEQ-2` Feedback or bootstrap loops: EN from the device's own output <a id="seq-2"></a>

🛑 error · needs: regulator pin roles (EN, VOUT), the EN net's divider or source · [catalogue entry](../checks.md#seq-2)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** A regulator or switch whose enable (or UVLO, or power-good-gated input) is derived from its own
output, directly or through a divider, a transistor or another regulator that itself needs that output.

**Why it matters.** At power-on the output is 0 V, so EN is low, so the output stays at 0 V: the board never starts.
Sometimes it does start, from leakage or noise, which makes it worse: a board that boots on the bench and not in the
field.

**Example.** A 5 V boost converter whose EN comes from a 100k/100k divider on `+5V0`, drawn that way so "EN follows
the battery": the divider sits on the boost's output net instead of the battery net. The converter never switches
on.

**How to check.** For each regulator (pin roles from the library, `NET-7`), follow the EN net back through resistors
and two-pin parts; if it reaches the regulator's own output rail (or a rail only that output feeds, `NET-5`) before
any other source, report it.

**Fix.** Take EN from the input side (a divider on Vin for a UVLO, or tie it to Vin if allowed, `RAIL-15`) or from
an upstream PG.

**References.** SLVA470 §1.2 (EN), §2 (EN and PG chains).

## `SEQ-3` SD card insertion inrush <a id="seq-3"></a>

⚠️ warning · needs: SD socket, its VDD path, capacitors · [catalogue entry](../checks.md#seq-3)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** An SD or microSD socket whose VDD comes straight from a board rail, with no power switch or
series resistor; or a switch with the card's decoupling on the wrong side of it.

**Why it matters.** A card carries its own decoupling capacitance, and an inserted card charges it from the rail in
microseconds. The rail dips, the MCU sharing it browns out or resets, and the card itself may not reach a clean
power-on reset. A power switch also lets firmware power-cycle a card that hangs, which the SD protocol has no other
way to recover.

**Example.** A microSD socket on the MCU's `+3V3` with 100 nF: hot insertion of a card with ~10 µF dips `+3V3` by
hundreds of mV and resets the MCU. With a TPS22918-class load switch with a slow rise time and the 4.7 µF on the
card side, the dip stays in the noise.

**How to check.** Find the socket (library class or symbol), follow its VDD pin: if it reaches a board rail through
nothing but copper, report; if a switch or resistor is there, the capacitors must sit between it and the socket
(card side).

**Fix.** A load switch with slew-rate control (or a PMOS with an RC on the gate, `SEQ-6`) driven by a GPIO, card
decoupling after it.

**References.** SLVA670 §1.1 (effects of load capacitance), §2 (problems caused by inrush current), §3.3 (integrated
load switches); Wikipedia, Inrush current.

## `SEQ-4` Soft-start circuit with a MOSFET or resistor as load <a id="seq-4"></a>

⚠️ warning · needs: soft-start circuit, load capacitance and current · [catalogue entry](../checks.md#seq-4)

**Impl.** hard · depends on NET-10, CFG-2

**What it catches.** A discrete soft-start (a MOSFET with an RC on its gate, or an inrush resistor bypassed later),
whose behaviour with the real load has not been worked out; with a known load, a report of the inrush current, ramp
time and FET dissipation.

**Why it matters.** The ramp of a gate-RC soft start depends on the FET's Miller plateau and the load: a resistive
load and a capacitive one give very different inrush currents and FET dissipation during the ramp. A soft start that
is too slow cooks the FET in its linear region; one that is too fast does nothing. An inrush resistor that is never
bypassed drops the rail under load.

**Example.** A PMOS (AO3401) with 100k gate-source and 1 µF gate capacitor switching 12 V into a board with 470 µF and
1 A of resistive load: the gate time constant is 100 ms, the FET spends tens of milliseconds in its linear region at
several watts, beyond the SOA of a SOT-23 part.

**How to check.** Recognise the structure (FET with a capacitor on gate, resistor to source, `NET-10`), take load
capacitance and current from the project (`CFG-2`), compute ramp time, peak current and energy in the FET, compare
with its SOA.

**Fix.** Size the RC for the load, check the FET's safe operating area, or use a load switch or hot-swap controller
with a specified slew rate.

**References.** SLVA716 §2.2 and §3.3 (adding a resistor to slow the rise time); SLVA670 §3.2 (discrete
implementation), §4 (application examples: rise time from the inrush limit).

## `SEQ-5` High voltage net without self-discharge resistors <a id="seq-5"></a>

⚠️ warning · needs: net voltage, capacitance, paths to ground · [catalogue entry](../checks.md#seq-5)

**Impl.** easy · depends on NET-5

**What it catches.** A net above a safe-to-touch voltage (above about 50 V, or a project threshold) with capacitance
on it and no resistor that discharges it when the supply is removed.

**Why it matters.** Capacitors on a high-voltage rail stay charged for minutes or hours after unplugging: a shock or
burn hazard for whoever opens the device, and an arc when a connector or probe touches it. A bleeder resistor sets a
known discharge time and is required by most safety standards for exposed parts.

**Example.** A 48 V motor bus with 2 × 220 µF and only a gate driver's µA of quiescent current: after unplugging, the
bus is still above 40 V for many minutes. A 47 kΩ bleeder brings it below 10 V in under a minute (τ ≈ 20 s) at
~50 mW in operation.

**How to check.** Rails with a voltage from the name or the project above the threshold (`NET-5`), capacitance on them,
and no resistor (chain) from the rail to ground; compute the RC time constant if one exists.

**Fix.** A bleeder resistor (or a chain rated for the voltage), sized for the discharge time and its dissipation in
operation.

**References.** Wikipedia, Bleeder resistor.

## `SEQ-6` Power switch turning hard on into a capacitive load <a id="seq-6"></a>

⚠️ warning · needs: power switches, their gate drive, downstream capacitance · [catalogue entry](../checks.md#seq-6)

**Impl.** hard · depends on NET-10, CFG-2

**What it catches.** A MOSFET or load switch feeding an off-board load or a hot-plug port, with nothing slowing its
turn-on: gate driven straight from a GPIO or a strong pull, no gate RC, a load switch without soft start.

**Why it matters.** The load's capacitance charges through the switch's on-resistance only: amperes for microseconds.
That current comes out of the board's own input capacitance, so the source dips and the board's own parts brown out or
reset. Slow turn-on with fast turn-off is the right shape: a ramp limits inrush, a quick off still protects in a fault.

**Example.** On B153A the switched receptacle VBUS `VBUS_SW` (Q102, header pin 10) turned hard on into a B129A with
more than 110 µF: the dongle's own supply dipped and the hub reset. A 10 Ω resistor in the wire cured it on the bench;
rev B gets a slow turn-on with a still fast turn-off (a gate RC with a diode across the resistor, or a load switch with
soft start).

**How to check.** For each FET or load switch whose output reaches a connector (`NET-10`), look at the gate network
(capacitor to source, series resistor) or the load switch's soft-start pin; with downstream capacitance from the
project (`CFG-2`), compute the inrush current.

**Fix.** Gate RC (Cgs plus a series resistor, a diode across the resistor for fast turn-off), a load switch with
adjustable rise time, or a hot-swap controller.

**References.** SLVA670 §2, §4.1 (response with and without slew-rate control); SLVA716 §2.2; SLVA883 §3.1;
B153 rev B change list (VBUS_SW soft start).

## `SEQ-7` Multi-rail part whose datasheet orders its supplies, with nothing enforcing the order <a id="seq-7"></a>

⚠️ warning · needs: per-part supply order (library), rail sources and enables · [catalogue entry](../checks.md#seq-7)

**Impl.** hard · depends on NET-8, NET-7

**What it catches.** A SoC, FPGA, DDR memory or PMIC-fed processor whose datasheet specifies the order of its supply
rails (or a maximum difference between them during ramps), on a board where those rails start independently: all ENs
tied to the input, no PG chain, no sequencer.

**Why it matters.** Wrong order forward-biases internal diodes between domains, draws large currents through IO
structures, latches up, or leaves IO pads in undefined states during start-up. The datasheet's power-up diagram is not
a suggestion: parts outside it are out of spec, and the failure may be a slow degradation instead of a dead board.

**Example.** AM335x: Figures 6-2 to 6-6 of the datasheet give the power-up order of the RTC, 1.8 V, DDR, 3.3 V and core
supplies, with PWRONRSTn released last, and note H adds that VDDS must come up before the other 1.8 V supplies when
they come from separate regulators. Four bucks with EN tied to 5 V start in whatever order their soft starts give.

**How to check.** The library carries each part's rail order; from the power structure and the EN/PG chains
(`NET-8`) derive the start-up order on the board, or report that nothing controls it.

**Fix.** A PMIC made for the part, or EN of each regulator from the previous one's PG (SLVA470 §2), or a sequencer;
the power-down order too (AM335x §6.1.2).

**References.** AM335x §6.1 (Figures 6-2 to 6-6, power-up sequencing), §6.1.2 (power-down sequencing); SLVA470 §2.

## `SEQ-8` Reset of a multi-rail part released by an RC alone <a id="seq-8"></a>

⚠️ warning · needs: reset pin role, the reset net's drivers, rail supervision · [catalogue entry](../checks.md#seq-8)

**Impl.** mid · depends on NET-7

**What it catches.** The power-on reset input of a part with several supplies that is held only by a resistor and a
capacitor (or nothing), not by a supervisor or a PGOOD chain that waits until all its rails are valid.

**Why it matters.** An RC times from the rail it is pulled to, not from the last rail to come up; if another rail is
slow (a large load, a low input, cold), reset is released before the part is fully powered and it starts in an
undefined state. A slow input ramp also makes the RC threshold crossing slow and noisy, giving several resets.
Supervisors have a defined threshold and delay.

**Example.** An AM335x PWRONRSTn on a 10k/1 µF RC to `+3V3`, while the core buck starts later from the 5 V input: the
datasheet's figures expect PWRONRSTn low until all supplies are valid. With a soft-starting core rail at cold the RC
releases first.

**How to check.** The reset pin (role from the library, `NET-7`) of a part with more than one supply rail: the drivers
on its net must include a supervisor output, a PG (or a PG-AND), or a PMIC reset; resistors and capacitors only is the
finding.

**Fix.** A voltage supervisor on the last rail (or several), or PGs wired-ANDed into reset; the RC can stay as
glitch filter.

**References.** AM335x §6.1 (PWRONRSTn in the power-up sequencing figures); Wikipedia, Power-on reset.

## `SEQ-9` Output capacitors not discharged at power-off for a part that needs a full power-on reset <a id="seq-9"></a>

💡 suggestion · needs: per-part POR requirement, rail discharge paths · [catalogue entry](../checks.md#seq-9)

**Impl.** judgement · depends on NET-8

**What it catches.** A switched rail feeding a part that needs its supply to fall below a low voltage (often a few
hundred mV) before it powers up again, with no active or resistive discharge of the rail's capacitors.

**Why it matters.** With the switch off and only a sleeping load, the rail decays slowly; a quick off-on (a watchdog
power cycle, a user replugging) starts the part from a half-powered state where its internal POR never fired. It boots
with stale registers or not at all, and the "power cycle fixes it" recovery does not.

**Example.** A cellular modem on a rail switched by a PMOS, 100 µF bulk, a few µA in off state: the firmware's
"power cycle the modem for 100 ms" leaves the rail at 2 V, and the modem stays hung. A load switch with quick output
discharge (TPS22918 QOD) pulls it to 0 V in milliseconds.

**How to check.** Needs the part's POR requirement from its datasheet, often only stated in prose; better_erc can list
switched rails without a discharge path (QOD pin, resistor) for the reviewer.

**Fix.** A load switch with quick output discharge, a discharge FET driven by the inverted enable, or a resistor
sized for the needed time; a long enough off time in firmware.

**References.** SLVA883 §3.3 (quick output discharge); TPS22918 §8.3.2 (QOD).
