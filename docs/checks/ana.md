# ANA checks, explained

Detail page for the `ANA` category of the [check catalogue](../checks.md#-ana---analog-and-sensing). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find
it, how hard that is to implement, the usual fix, and where to read more. The one-line entries in the catalogue stay the
source of truth for ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to implement in better_erc without an LLM: `easy` is a graph query on
the netlist alone, `mid` needs part data or a project setting first, `hard` needs analysis the code does not have
yet, `judgement` needs a person (better_erc can at most list candidates). The IDs after it are the
[FEATURES.md](../../FEATURES.md) items the check depends on.

References used throughout:

- **AN2834** · ST, *How to optimize the ADC accuracy in the STM32 MCUs*, AN2834 Rev 10 (October 2024). Section
  numbers below refer to it (fetched 22 Oct 2026 through `tpm fetch`, st.com refuses plain HTTP clients).
  [st.com/resource/en/application_note/an2834](https://www.st.com/resource/en/application_note/an2834-how-to-get-the-best-adc-accuracy-in-stm32-microcontrollers-stmicroelectronics.pdf)
- **DS13560** · ST, *STM32G0B1xB/xC/xE* datasheet, DS13560 Rev 6 (February 2026): the MCU of B153A.
  [st.com/resource/en/datasheet/stm32g0b1ce.pdf](https://www.st.com/resource/en/datasheet/stm32g0b1ce.pdf)
- **RP2350** · Raspberry Pi, *RP2350 Datasheet*: the MCU of B200A.
  [datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- **INA181** · TI, *INAx181 Bidirectional, Low- and High-Side Voltage Output, Current-Sense Amplifiers*, SBOS793H
  (November 2023): the sense amplifier of B153A. [ti.com/lit/ds/symlink/ina181.pdf](https://www.ti.com/lit/ds/symlink/ina181.pdf)
- **TLV9004** · TI, *TLV900x Low-Power, RRIO, 1-MHz Operational Amplifier*, SBOS833R (November 2021): the buffer
  op-amp of B200A. [ti.com/lit/ds/symlink/tlv9004.pdf](https://www.ti.com/lit/ds/symlink/tlv9004.pdf)
- **LM358** · TI, *LMx58, LM2904x* datasheet, SLOS068AB (October 2024): the classic non-rail-to-rail op-amp.
  [ti.com/lit/ds/symlink/lm358.pdf](https://www.ti.com/lit/ds/symlink/lm358.pdf)
- **Kitchin** · Charles Kitchin (ADI), *Avoid Common Problems When Designing Amplifier Circuits*, Analog Dialogue
  41-08 (August 2007). DC return paths, in-amp and op-amp references, dividers, decoupling of bias points.
  [analog.com/media/en/analog-dialogue/volume-41/number-3](https://www.analog.com/media/en/analog-dialogue/volume-41/number-3/articles/common-problems-when-designing-amplifier-circuits.pdf)
- **MT-xxx** · Walt Kester (ADI) tutorials, Rev. 0 / Rev. A (October 2008): MT-002 *What the Nyquist Criterion
  Means to Your Sampled Data System Design*, MT-031 *Grounding Data Converters and Solving the Mystery of "AGND" and
  "DGND"*, MT-035 *Op Amp Inputs, Outputs, Single-Supply, and Rail-to-Rail Issues*, MT-036 *Op Amp Output
  Phase-Reversal and Input Over-Voltage Protection*, MT-038 *Op Amp Input Bias Current*, MT-068 *Difference and
  Current Sense Amplifiers*. URLs follow one pattern,
  [analog.com/media/en/training-seminars/tutorials/MT-035.pdf](https://www.analog.com/media/en/training-seminars/tutorials/MT-035.pdf)
  (each fetched through `tpm fetch`):
  [MT-002](https://www.analog.com/media/en/training-seminars/tutorials/MT-002.pdf),
  [MT-031](https://www.analog.com/media/en/training-seminars/tutorials/MT-031.pdf),
  [MT-036](https://www.analog.com/media/en/training-seminars/tutorials/MT-036.pdf),
  [MT-038](https://www.analog.com/media/en/training-seminars/tutorials/MT-038.pdf),
  [MT-068](https://www.analog.com/media/en/training-seminars/tutorials/MT-068.pdf).
- **SBOA204** · TI, *How to Properly Configure Unused Operational Amplifiers*, SBOA204A (2018).
  [ti.com/lit/an/sboa204](https://www.ti.com/lit/an/sboa204/sboa204.pdf)
- **SBOA313** · TI, *Non-Inverting Comparator with Hysteresis Circuit*, SBOA313A (2019).
  [ti.com/lit/an/sboa313](https://www.ti.com/lit/an/sboa313/sboa313.pdf)
- **SBOA418** · TI, *Determine Optimal Isolation Resistance for Driving Capacitive Load*, SBOA418 (July 2023).
  [ti.com/lit/an/sboa418](https://www.ti.com/lit/an/sboa418/sboa418.pdf)
- **SLOA020** · TI, *Stability Analysis of Voltage-Feedback Op Amps Including Compensation Techniques*, SLOA020A
  (March 2001). [ti.com/lit/an/sloa020a](https://www.ti.com/lit/an/sloa020a/sloa020a.pdf)
- **SBAA275** · TI, *A Basic Guide to RTD Measurements*, SBAA275A (2023).
  [ti.com/lit/an/sbaa275](https://www.ti.com/lit/an/sbaa275/sbaa275.pdf)
- **Wikipedia** · readable overviews: [Operational amplifier](https://en.wikipedia.org/wiki/Operational_amplifier),
  [Schmitt trigger](https://en.wikipedia.org/wiki/Schmitt_trigger),
  [Anti-aliasing filter](https://en.wikipedia.org/wiki/Anti-aliasing_filter),
  [Four-terminal sensing](https://en.wikipedia.org/wiki/Four-terminal_sensing),
  [Electrical resistivity and conductivity](https://en.wikipedia.org/wiki/Electrical_resistivity_and_conductivity).
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 4 (operational
  amplifiers), chapter 5 (precision circuits), chapter 13 (digital meets analog: ADCs and DACs).

## `ANA-1` ADC input from a high-impedance source without a buffer <a id="ana-1"></a>

⚠️ warning · needs: ADC pins (pin functions), source impedance through the resistor network · [catalogue entry](../checks.md#ana-1)

**Impl.** mid · depends on NET-7, NET-12, RPT-6

**What it catches.** An MCU or ADC input whose source impedance (the Thevenin resistance of the divider, sensor or
filter in front of it) is above what the ADC can settle in its sampling time, with no buffer and no reservoir
capacitor at the pin.

**Why it matters.** A SAR ADC samples by connecting a small capacitor (a few pF) to the pin for a short time. The
source has to recharge it through its own resistance; when that takes longer than the sampling window the
conversion reads low, and the error depends on what the previous channel was. Input leakage into a large source
resistance adds an offset on top. The error is silent: plausible numbers, a few percent off, worse at fast scan
rates.

**Example.** A battery monitor with a 1 MΩ / 100 kΩ divider (chosen for a tiny standby current) straight into an
STM32G0B1 ADC pin: the Thevenin resistance is 91 kΩ, while the datasheet allows at most 50 kΩ for 12 bits even at
the longest sampling time of 160.5 cycles (DS13560 Table 64). B153A's VBUS sense on PA3 (10 kΩ / 1.62 kΩ, about
1.4 kΩ) is fine, and it carries a 100 nF capacitor at the pin as well.

**How to check.** Find the ADC-capable pins in use (pin functions, `NET-12`), compute the source resistance seen
from the pin through the resistor network (`RPT-6`), and compare it with the part's limit per resolution and
sampling time (library or project data). A capacitor to ground at the pin, large against the sampling capacitor,
or an op-amp output on the net exempts it.

**Fix.** A follower in front of the ADC, a capacitor at the pin (≥ 1000 × the sampling capacitance, and accept the
slower signal), lower divider values, or a longer sampling time where the part allows it.

**References.** AN2834 §3.2.7 (effect of the analog signal source resistance), §4.2.7 (external analog buffer
usage), §4.4 (high impedance source measurement); DS13560 Table 64 (maximum ADC RAIN); AoE3 chapter 13.

## `ANA-2` Sensitive pins next to high-speed pins on a part <a id="ana-2"></a>

⚠️ warning · needs: pin numbers, pin functions, net classes · [catalogue entry](../checks.md#ana-2)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** An ADC input or other high-impedance analog pin whose neighbour by pin number (or ball, on a
BGA) carries a clock, a fast edge or a switching node: SPI clock, PWM, a UART at high rate, a crystal output.

**Why it matters.** Adjacent pins and the tracks that leave them are coupled by a few hundred fF to a few pF.
Every edge on the neighbour injects a charge into the high-impedance node; the ADC sees spikes or an offset that
correlates with the digital activity, and no amount of filtering behind the pin removes what couples in at the pin.

**Example.** On B153A (STM32G0B1, LQFP48) the INA181 output `AN0` is on PB0, pin 19, between PA7 (the button
ladder, pin 18) and PB1 (pin 20), which is tied to header pin 1 and can run a UART or SPI on PB3. With a fast
signal on PB1 the current reading picks up its edges; with the button ladder next to it the neighbour is quiet.

**How to check.** For each analog pin in use, look up its neighbours by pin number in the package (and the
diagonal ones on a BGA, which needs the ball map) and classify their nets: clocks, PWM, buses by name or net class,
switching nodes (`NET-6`). The adjacency is in the netlist; the function of the pins needs part data.

**Fix.** Swap pins so the analog input has quiet neighbours (a ground, a static GPIO, another slow analog
signal), slow the edges of the neighbour, or route the two apart right at the package.

**References.** AN2834 §3.2.11 (I/O pin crosstalk), §4.2.11 (minimizing I/O pin crosstalk); MT-031 "Treat the ADC
digital outputs with care".

## `ANA-3` Shunt lines to the amplifier without a filter <a id="ana-3"></a>

💡 suggestion · needs: shunt (low-value resistor), current-sense amplifier inputs, capacitors · [catalogue entry](../checks.md#ana-3)

**Impl.** easy · depends on netlist

**What it catches.** A current-sense amplifier whose inputs come straight from the shunt, with no differential
capacitor (and small series resistors) across the inputs and no RC at the output either.

**Why it matters.** A shunt in a switching converter, a motor bridge or a PWM-driven load carries ripple and
fast edges far above the bandwidth anyone wants to measure. The sense amplifier rectifies or aliases part of it
(`ANA-18`) and the ADC samples random points of the ripple; common-mode steps on the bus couple in too. A filter
at the output is usually best; a filter at the inputs is the second choice and must keep its series resistors
small, because the amplifier's input bias currents flow through them and turn into a gain error.

**Example.** An INA181 on a 10 mΩ shunt in a 100 kHz buck output: the reading jumps by tens of mA from sample to
sample. 10 Ω + 10 Ω with 1 µF across the inputs puts the corner at about 8 kHz; on the A2 gain version these 10 Ω
already add 0.89 % of gain error (the datasheet's own figure), and 100 Ω would add about 8 %.

**How to check.** Find low-value resistors (below about 1 Ω) whose two ends go to the two inputs of a part with
current-sense pin names (IN+/IN−, VIN+/VIN−, SENSE+/−); look for a capacitor between the input nets, or an RC on
the output net before the ADC.

**Fix.** RC on the output into the ADC; where the input must be filtered, ≤ 10 Ω in each line and a capacitor
across the inputs (plus small ones to ground, matched).

**References.** INA181 §8.1.3 (signal filtering, Figure 8-2, series resistors ≤ 10 Ω, Tables 8-1 and 8-2);
MT-068 (applications of current sensing).

## `ANA-4` Current sense resistor straight to ground or power <a id="ana-4"></a>

💡 suggestion · needs: shunt, power structure · [catalogue entry](../checks.md#ana-4)

**Impl.** easy · depends on NET-5

**What it catches.** A shunt one of whose ends is the ground or rail net itself, so the amplifier's sense input is
on that net too: the netlist cannot tell the sense connection from any other connection to the plane.

**Why it matters.** A 5 mΩ shunt with 1 A across it gives 5 mV. Ten millimetres of 1 oz copper are already a few
milliohms: if the sense line joins the ground net anywhere on the plane instead of at the shunt's own pad, the
copper drop between the two points becomes part of the measurement and the reading is off by tens of percent. A
net tie (a separate net for the sense line, joined to the power net only at the pad) makes the Kelvin connection
explicit, so the layout tool routes it as its own track and the DRC keeps it.

**Example.** A low-side shunt R10 10 mΩ from the motor bridge source to `GND`, with the amplifier's IN− also on
`GND`: the router connects IN− to the nearest ground via, 20 mm away, and the 3 A motor current through that
stretch of plane adds a few mV, a 10 % error that changes with layout revisions.

**How to check.** Low-value resistors (below about 1 Ω) with one end on a rail or ground of the power structure
(`NET-5`) and a sense-amplifier input on the same net.

**Fix.** Separate sense nets (`ISENSE_P`, `ISENSE_N`) joined to the power nets by net-tie footprints at the shunt
pads; route them as a pair.

**References.** INA181 §8.4.1 (Kelvin or 4-wire connection to the sense resistor); Wikipedia, Four-terminal
sensing; `ANA-23` for the layout side.

## `ANA-5` Reference or bias input fed from a divider <a id="ana-5"></a>

⚠️ warning · needs: REF/VBIAS pins, divider impedance, pin input current (part data) · [catalogue entry](../checks.md#ana-5)

**Impl.** mid · depends on NET-7, RPT-6

**What it catches.** A REF, VREF, VBIAS or VOCM pin of an amplifier, in-amp or ADC fed from a plain resistor
divider, without a buffer, where the pin's input current or input resistance is not negligible against the
divider.

**Why it matters.** In a difference amplifier or current-sense amplifier the REF pin is the bottom of an internal
resistor network, so current flows in or out of it, and that current depends on the input common-mode voltage. A
divider's Thevenin resistance turns it into a reference shift, which appears 1:1 at the output as an offset; the
resistance in series with REF also unbalances the internal network and degrades common-mode rejection. A
capacitor on the divider removes noise, not this DC error.

**Example.** On B153A the INA181A2 REF comes from R603/R604 1 kΩ / 1 kΩ off 3.3 V with 4.7 µF: 500 Ω source
resistance. The datasheet gives more than 500 kΩ from each input to REF; with the inputs on a USB VBUS of 5 to 20 V,
the current into REF is roughly 7 to 37 µA, a shift of a few mV to about 18 mV, which at 0.25 V/A reads as 15 to
70 mA of current that is not there and moves with the PD voltage. The INA181 datasheet asks for a buffered
reference for exactly this reason.

**How to check.** Pins of the reference class (by name and part data) whose net is the midpoint of two resistors
between a rail and ground, with no op-amp output on it; compute the Thevenin resistance (`RPT-6`) and multiply by
the pin's input current from the part data.

**Fix.** Buffer the divider with an op-amp follower, use a reference IC, or drive REF from a low-impedance source
(the ADC's own reference buffer, where it is offered); at least lower the divider values and recompute.

**References.** INA181 §7.4.2 ("buffer the reference voltage connected to the REF pin"), §7.4.5 (500 kΩ input
network to OUT and REF); Kitchin, "Correctly providing in-amp reference voltage" and "Supplying reference
voltages for in-amps, op amps, and ADCs" (Figures 8, 9); MT-068.

## `ANA-6` Op-amp input common-mode range vs the signal <a id="ana-6"></a>

🛑 error · needs: op-amp common-mode range (part data), signal range on the inputs · [catalogue entry](../checks.md#ana-6)

**Impl.** hard · depends on NET-7, NET-10, CFG-2, RPT-10

**What it catches.** An op-amp whose inputs sit at a voltage outside its input common-mode range: typically a
signal at or near the positive rail on a part whose input stage stops 1.5-2 V below it, or a signal at ground on a
part that does not include the negative rail.

**Why it matters.** Outside the common-mode range the input stage runs out of headroom: the gain collapses, the
offset grows by orders of magnitude, and some parts invert their output (`ANA-8`). The circuit looks right on paper
because the op-amp symbol has no range on it.

**Example.** An LM358 on 3.3 V as a high-side current-sense difference amplifier with the inputs at 3.3 V: the LM358
is specified for a common-mode voltage from V− to V+ − 2 V, so 1.3 V is the top of its range and the amplifier
reads garbage. On B200A the TLV9004 followers are rail-to-rail input (V− − 0.1 V to V+ + 0.1 V), so their 0-5 V
inputs are in range on a 5 V supply, once the supply is there.

**How to check.** For each op-amp input, the possible voltage range of its net (rails, dividers, signals from
connectors with ranges from the project, `CFG-2`), against the part's common-mode range from the library. Getting the
range of an arbitrary signal needs signal following and transfer functions (`NET-10`, `RPT-10`); the clear cases
(an input on a rail net, or on the high side of a shunt on a rail) come out of the netlist.

**Fix.** A rail-to-rail input part, an input divider that brings the signal into range, a higher supply for the
op-amp, or a current-sense amplifier made for the common-mode range.

**References.** MT-035 "Op amp input stages" (single-supply and rail-to-rail inputs); LM358 §5.3 (recommended
VCM V− to V+ − 2 V); TLV9004 §7.10 (VCM), §8.3.2 (rail-to-rail input); AoE3 chapter 4.

## `ANA-7` Op-amp output swing vs what the next stage needs <a id="ana-7"></a>

⚠️ warning · needs: op-amp output swing (part data), load, range the next stage needs · [catalogue entry](../checks.md#ana-7)

**Impl.** hard · depends on NET-7, NET-10, RPT-10

**What it catches.** An op-amp output expected to reach a level it cannot: 0 V on a single-supply part, or the
positive rail, when its output stops tens or hundreds of mV short under the actual load.

**Why it matters.** "Rail-to-rail output" means within 10-50 mV at light load, more as the load current rises;
older outputs stop 1-1.5 V short. A single-supply current or sensor amplifier then cannot show "zero": the low end
of the range is lost, and a reading that should be 0 sits at 20 mV forever. Downstream comparators and ADCs inherit
the dead zone.

**Example.** A unidirectional current monitor with REF at ground: the INA181's output swing to ground is
specified at 5 mV, but with a +40 µV offset at gain 200 the output cannot go below 8 mV (its own datasheet example),
so the first 40 µV of shunt voltage (4 mA on a 10 mΩ shunt) never shows. B200A's TLV9004 followers on 5 V drive
13.7 kΩ / 26.7 kΩ dividers and swing to within about 20 mV of the rails at 10 kΩ, so 0 V and 5 V read as about
13 mV and 3.29 V at the ADC.

**How to check.** For each op-amp output, the range the next stage expects (ADC full scale, comparator threshold,
a project setting), against the output swing of the part at the load the netlist shows. Needs the transfer
function of the stage (`RPT-10`).

**Fix.** Offset the signal off the rail (REF slightly above ground, a mid-rail bias), a negative supply or charge
pump for true zero, a part with a better output, or accept and calibrate the dead zone.

**References.** INA181 §7.3.5 (rail-to-rail output swing, offset × gain), §8.1.2 (Equations 3 and 4); TLV9004
§8.3.3 (rail-to-rail output); MT-035 "Output stages".

## `ANA-8` Op-amp input driven beyond its supply with no series resistance <a id="ana-8"></a>

⚠️ warning · needs: input source range (connectors, other domains), series parts, op-amp supply · [catalogue entry](../checks.md#ana-8)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** An op-amp input that can see a voltage beyond its supply rails (from a connector, from another
power domain, or because the op-amp's own supply can be off) with no resistor between the source and the pin to limit
the current.

**Why it matters.** The inputs are diode-clamped to the rails; above V+ + 0.5 V or below V− − 0.5 V those diodes
conduct and only the source impedance limits the current. Datasheets allow about 10 mA; a stiff source delivers
amps. Before damage, some parts show phase reversal: the output slams to the opposite rail, which in a control loop
is positive feedback. An unpowered op-amp with signals on its inputs is the same case: the clamps then back-power
its supply rail.

**Example.** B200A rev A: `+5V0_FLT`, the supply of both TLV9004s, has no source, so every IO pin driven by the
device under test feeds the op-amp supply through the input clamps. IO0-IO3 have only 255 Ω in series, which a 5 V
logic output pushes up to about 17 mA through, above the TLV9004's 10 mA input current rating; RX, SCL and SDA
have 1.25 kΩ.

**How to check.** For each op-amp input, the path back to a connector pin or a net of another rail (`NET-5`):
flag it when no resistor is in the path, or when the resistor cannot hold the current below the part's rating for the
voltage the project allows on that pin (`CFG-2`).

**Fix.** A series resistor sized for the fault voltage (1-10 kΩ, within the noise budget), external clamps to the
rails or a Zener, a part with over-voltage-protected inputs; keep the op-amp powered whenever its inputs can be.

**References.** MT-036 (output phase reversal, input over-voltage protection); TLV9004 §7.1 (input current
±10 mA, note 2: inputs diode-clamped to the rails); AoE3 chapter 4.

## `ANA-9` Op-amp driving a capacitive load without an isolation resistor <a id="ana-9"></a>

⚠️ warning · needs: op-amp output pins, capacitance on the output net, part data (stable load) · [catalogue entry](../checks.md#ana-9)

**Impl.** mid · depends on NET-7

**What it catches.** An op-amp output connected straight to a capacitor, an ADC input, a long cable or a large
gate, with no series resistor between the output and that load.

**Why it matters.** The op-amp's open-loop output resistance and the load capacitance add a pole inside the
feedback loop; the phase margin drops and the stage rings or oscillates. Followers are the worst because their loop
gain is highest. An ADC input is a switched capacitor that kicks the output at every sample; the amplifier then has
to recover before the sample ends. The fix is a resistor of a few tens of ohms that isolates the capacitance from the
loop, sized from the op-amp's output impedance.

**Example.** A follower with a 1 µF reservoir capacitor directly on its output feeding an ADC: the step response
rings, and at certain loads it oscillates at hundreds of kHz; with 47 Ω in series the same circuit is stable. TI's
note works the case of a 10 µF load on an OPA392.

**How to check.** For each op-amp output (by pin name and the library), sum the capacitors to ground on the net
and note ADC pins and connectors there; flag them when no resistor separates the output from them, and compare the sum
with the part's stable capacitive load from the library.

**Fix.** An isolation resistor (RISO, 10-100 Ω) between the output and the capacitance, with the feedback taken
before it (or a second feedback path after it for DC accuracy); a part made for capacitive loads.

**References.** SBOA418 §3 (calculating the optimal isolation resistance), §4 (RO, RISO and capacitive load drive);
SLOA020 §5 (capacitively loaded op amp, Figure 13); TLV9004 Figures 7-24, 7-25 (overshoot vs capacitive load).

## `ANA-10` Op-amp with no feedback path, or feedback into the non-inverting input <a id="ana-10"></a>

🛑 error · needs: op-amp pins, connectivity · [catalogue entry](../checks.md#ana-10)

**Impl.** easy · depends on netlist

**What it catches.** An op-amp whose output has no path (direct, resistor, capacitor) back to its inverting input,
or whose only feedback goes to the non-inverting input.

**Why it matters.** Without negative feedback an op-amp is a comparator with no hysteresis: its output sits at a rail
and flips with microvolts of noise. Feedback to the non-inverting input is positive feedback: a latch, or a Schmitt
trigger when that is intended. Both are typical drawing mistakes: swapped input pins on a symbol, a missing resistor,
a feedback net labelled with a typo.

**Example.** A non-inverting gain stage whose gain resistor lands on pin 3 (IN+) instead of pin 2 (IN−) after a symbol
change: the output latches to one rail at power-up and stays there.

**How to check.** For each op-amp unit (library class, pin names IN+, IN−, OUT), walk from OUT through resistors and
capacitors and see whether IN− is reached; flag none, or a path that reaches only IN+. Parts used as comparators on
purpose are exempt when the project says so.

**Fix.** Connect the feedback network to the inverting input; if a comparator was meant, use a comparator.

**References.** Wikipedia, Operational amplifier ("Open-loop amplifier", "Closed-loop amplifier"); AoE3 chapter 4.

## `ANA-11` Unused op-amp channel left floating <a id="ana-11"></a>

⚠️ warning · needs: op-amp units and their pins · [catalogue entry](../checks.md#ana-11)

**Impl.** easy · depends on netlist

**What it catches.** A channel of a dual or quad op-amp whose inputs (and output) are not connected, or are
connected in a way that drives it into saturation.

**Why it matters.** A floating channel picks up whatever is around it and its output swings between the rails,
drawing extra supply current and coupling into the used channels in the same package. Tying both inputs to a rail is
no better: the input stage may sit outside its common-mode range or see a large differential voltage. TI's guidance
is a follower whose input is held inside the common-mode and output ranges, ideally at mid-supply.

**Example.** B200A: the eighth follower U7 (second TLV9004) has its input grounded on a single 5 V supply. The TLV9004
input range includes ground, but the output cannot reach 0 V, so the loop sits in saturation at the bottom rail: the
configuration TI's note warns about. A 10 kΩ / 10 kΩ divider to mid-supply, or the input tied to any in-range
voltage already on the board, is the clean version.

**How to check.** For each op-amp unit, inputs with no connection, or a follower whose non-inverting input is on the
negative rail while the part's output cannot reach it.

**Fix.** Wire the channel as a follower with its input at mid-supply or any voltage inside the input and output
ranges.

**References.** SBOA204 (single-supply implementation, Figure 2; grounded follower input saturates the output);
TLV9004 §8.3.2, §8.3.3.

## `ANA-12` Comparator on a slow or noisy signal without hysteresis <a id="ana-12"></a>

💡 suggestion · needs: comparator parts and their feedback · [catalogue entry](../checks.md#ana-12)

**Impl.** easy · depends on netlist

**What it catches.** A comparator (or an op-amp used as one) with no positive feedback resistor from the output to
the non-inverting input, no built-in hysteresis, fed from a slow or noisy signal.

**Why it matters.** Near the threshold the input spends a long time within millivolts of the reference; noise and
the comparator's own output edge coupling back through the supply make it toggle many times per crossing. A
battery-low flag chatters, an interrupt fires a thousand times, a relay buzzes. A few millivolts of hysteresis cost
one resistor.

**Example.** A thermostat comparator on a thermistor divider with a 10 nF filter: the temperature crosses the
threshold over seconds and the output oscillates at tens of kHz for the whole crossing, until a 1 MΩ positive
feedback resistor gives 30 mV of hysteresis.

**How to check.** For each comparator unit (library class), look for a resistor between OUT and IN+ (or a datasheet
hysteresis pin). Which signals are slow or noisy is not in the netlist, so this lists every comparator without one.

**Fix.** A positive feedback resistor sized per SBOA313 (and a small capacitor across it if the edge is noisy), or a
comparator with built-in hysteresis.

**References.** SBOA313 (non-inverting comparator with hysteresis, design steps); Wikipedia, Schmitt trigger; AoE3
chapter 4.

## `ANA-13` ADC input range exceeded without scaling <a id="ana-13"></a>

🛑 error · needs: ADC pins, voltage range of their nets, divider ratios · [catalogue entry](../checks.md#ana-13)

**Impl.** mid · depends on NET-5, NET-12, RPT-6, CFG-2

**What it catches.** A signal whose maximum is above the ADC's full scale (its reference, or VDDA) reaching an ADC
pin directly or through a divider that does not bring it under.

**Why it matters.** Above full scale the ADC saturates, so the top of the range is lost; above the pin's rating the
clamp diodes conduct, the input injects current into the neighbouring channels (`ANA-20`), and the reference and
supply rise with it. A 5 V sensor on a 3.3 V MCU is the classic case, often hidden behind a module or connector whose
level nobody wrote down.

**Example.** A 0-10 V industrial input with a 10 kΩ / 10 kΩ divider into an STM32: 5 V at the pin, far above the 3.3 V
full scale. B153A's VBUS sense is sized right: × 0.139 gives 2.79 V at 20 V and reaches the 3.3 V full scale only above
23.7 V, just under the 24 V over-voltage limit of its load switch.

**How to check.** For each ADC pin, the maximum voltage of the net it is on: rail voltages from the power structure
(`NET-5`), divider ratios (`RPT-6`), and connector pin ranges from the project (`CFG-2`); compare with the ADC
reference.

**Fix.** A divider (with the source impedance of `ANA-1` in mind), an op-amp scaling stage, or an external reference
that covers the range.

**References.** AN2834 §3.2.6 and §4.2.5 (matching the ADC range to the signal amplitude); DS13560 Table 21 (VIN);
AoE3 chapter 13.

## `ANA-14` Current sense amplifier common-mode rating below the bus <a id="ana-14"></a>

🛑 error · needs: sense amplifier common-mode rating (part data), bus voltage · [catalogue entry](../checks.md#ana-14)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A current-sense amplifier on a shunt whose bus voltage (including transients and the highest
negotiated or charging voltage) exceeds the amplifier's input common-mode range or its absolute maximum.

**Why it matters.** High-side sense amplifiers are sold in voltage classes: 26 V, 36 V, 80 V, 100 V and more. Above the
operating range the reading is wrong; above the absolute maximum the inputs are destroyed, and with them often the
ADC behind them. A family member at a lower class is a drop-in footprint, so a BOM substitution or a design reused on a
higher bus gets there without anyone touching the schematic.

**Example.** An INA181 (common-mode −0.2 V to +26 V) copied from a 12 V design onto a 48 V battery bus. Inside its
class it can still be marginal: on a USB PD port the steady 20 V fits, but the datasheet has a section on common-mode
transients above 26 V, which a hot-plug of a 20 V source with long cables can produce.

**How to check.** For each current-sense amplifier, the voltage of the shunt's nets (rail voltage from `NET-5` or the
project) against the part's common-mode range and absolute maximum from the library.

**Fix.** A part of the right class, or a sense topology that keeps the amplifier at low common mode (low-side shunt,
a level-shifting high-voltage sense part); transient protection where the bus rings.

**References.** INA181 §7.3.3 (wide input common-mode range), §8.3.1 (common-mode transients greater than 26 V);
MT-068 (high common-mode current sensing).

## `ANA-15` Sense gain times shunt drop saturates or wastes the range <a id="ana-15"></a>

⚠️ warning · needs: shunt value, amplifier gain (MPN), current range (project), ADC full scale · [catalogue entry](../checks.md#ana-15)

**Impl.** mid · depends on RPT-11, CFG-2

**What it catches.** Shunt × gain × maximum current above the output swing or ADC full scale (the output clips below
the current to measure), or so far below it that most of the ADC range is never used.

**Why it matters.** Clipping loses exactly the overload currents a protection function needs. The opposite case looks
harmless but costs resolution and makes the amplifier's offset a large share of every reading: a 0.5 V swing on a
3.3 V ADC throws away nearly three bits.

**Example.** B153A: 5 mΩ × gain 50 (INA181A2) = 0.25 V/A, centred on REF = 1.65 V, so the output covers about ±6.6 A
against a ±5.5 A goal: about 20 % headroom, one LSB about 3.2 mA. Swap in the A4 (gain 200) by BOM mistake and the
output clips at ±1.65 A.

**How to check.** Shunt value (parsed), gain from the amplifier's MPN suffix, REF voltage and output swing from the
part, current range from the project (`CFG-2`), ADC full scale; compute both ends (`RPT-11`).

**Fix.** Pick shunt and gain variant together so the full current lands at 70-90 % of the output range; move REF for
asymmetric bidirectional currents.

**References.** INA181 §8.1.2 (RSENSE and device gain selection, Equations 2-4), §7.4.4 (input differential
overload); AN2834 §3.2.6.

## `ANA-16` Gain or filter resistors far outside the sane range <a id="ana-16"></a>

⚠️ warning · needs: resistor values around op-amps · [catalogue entry](../checks.md#ana-16)

**Impl.** easy · depends on netlist

**What it catches.** Resistors in an op-amp's feedback or input network below about 1 kΩ (loading the output) or above
about 1 MΩ (where bias current and leakage dominate), or a filter capacitor so small that board parasitics set the
response.

**Why it matters.** The feedback network is a load on the output too: 10 Ω in a gain stage asks the output for
hundreds of mA, which it cannot give, so the gain sags and the part heats. At the other end bias current times the
resistance is an offset (1 nA × 10 MΩ = 10 mV), resistor noise grows, and leakage across a dirty board changes the gain
with humidity. Values like these usually come from a misplaced unit (10R for 10k, 10M for 10k).

**Example.** A gain-of-11 stage with Rf = 100 Ω and Rg = 10 Ω, a unit slip from 100 kΩ / 10 kΩ: on a 5 V supply
the output must sink or source 45 mA into the network at full swing, more than the TLV9004's typical 40 mA short-circuit
current, and the stage clips at a fraction of the expected amplitude.

**How to check.** Resistors with a pin on an op-amp's IN− or OUT net, compared with a window from the config (and the
part's bias current and output current from the library, for the finer version).

**Fix.** Keep feedback networks in the 1 kΩ to a few hundred kΩ range for general-purpose parts; use FET-input parts
when high values are unavoidable.

**References.** MT-038 (input bias current and its effects); TLV9004 §7.10 (output swing at 10 kΩ and 2 kΩ, ISC ±40 mA);
AoE3 chapter 5.

## `ANA-17` Sensor divider and ADC reference on different sources <a id="ana-17"></a>

💡 suggestion · needs: divider top, ADC reference source · [catalogue entry](../checks.md#ana-17)

**Impl.** mid · depends on NET-5, NET-12, IMP-9

**What it catches.** A resistive sensor divider (thermistor, pot, RTD, resistor ladder) excited from a rail while the
ADC uses a different reference, or an absolute signal (a divided supply, a sensor with an absolute output) measured
against the ADC's own supply as reference.

**Why it matters.** A divider's output is a fraction of its excitation; an ADC reports the input as a fraction of its
reference. When both are the same voltage the rail drops out and only the ratio is measured (ratiometric). When they
are different, every percent the rail moves becomes a percent of error: a 3.3 V regulator with 2 % tolerance and
ripple is a 2 % error on a temperature that should be exact.

**Example.** B200A measures 0-5 V signals through a × 0.661 divider into the RP2350 ADC, whose reference is
`+3V3_VDDA`, the buck output through a 0 Ω resistor: the absolute reading inherits the buck's tolerance and ripple. A
thermistor divider from the same 3.3 V would cancel it; an absolute measurement needs a reference IC. On B153A the
INA181 REF divider runs from the same 3.3 V as VDDA, so its mid-scale offset is ratiometric, as the board notes say.

**How to check.** For each ADC pin, the top of the divider in front of it (`NET-5`) against the ADC's reference
source: the VREF+ pin's net, or VDDA for parts that use it (part data, `IMP-9`).

**Fix.** Excite the sensor from the ADC reference (or measure the excitation on a second channel), or give absolute
signals a reference IC.

**References.** SBAA275 §1.4 (ratiometric measurements); AN2834 §3.2.2 (reference voltage / power supply regulation);
RP2350 §6.1.5 (ADC supply ADC_AVDD).

## `ANA-18` No anti-aliasing filter before an ADC <a id="ana-18"></a>

💡 suggestion · needs: ADC pins, filters, knowledge of the signal · [catalogue entry](../checks.md#ana-18)

**Impl.** judgement · depends on NET-12

**What it catches.** An ADC input fed from a noisy or fast source (a switching converter's current, a motor, a long
cable, a sensor with RF pickup) with no low-pass filter in front of it.

**Why it matters.** A sampled system folds every frequency above half the sample rate back into the band below it.
Ripple at 100 kHz sampled at 10 kHz does not average out: it appears as a slow wander or a fixed offset, depending on
the beat frequency, and no digital filter after the ADC can tell it from the signal.

**Example.** A motor current sampled at 1 kHz from a shunt amplifier while the bridge switches at 20 kHz: the reading
wobbles at a few hertz from the beat of the PWM against the sampling clock. An RC at 300 Hz in front of the ADC, or
sampling synchronised to the PWM, removes it.

**How to check.** better_erc can list ADC pins without a capacitor to ground or an RC in front; whether the source is
noisy, and what bandwidth is wanted, is the designer's knowledge.

**Fix.** An RC (or an active filter) with its corner below half the sample rate, sized for the attenuation needed at
the noise frequency; synchronise sampling to known switching where possible.

**References.** MT-002 "Baseband antialiasing filters"; AN2834 §4.2.3 (analog-input signal noise elimination);
Wikipedia, Anti-aliasing filter; AoE3 chapter 13.

## `ANA-19` DAC output into a load it cannot drive <a id="ana-19"></a>

⚠️ warning · needs: DAC pins, load on the net, DAC drive limits (part data) · [catalogue entry](../checks.md#ana-19)

**Impl.** mid · depends on NET-12, IMP-9

**What it catches.** An MCU or standalone DAC output connected to a resistive load below its minimum, a capacitance
above its maximum, or a pin with its own current (an LDO feedback pin, a reference input), with no buffer.

**Why it matters.** On-chip DAC buffers are weak: the output sags under resistive load, rings or oscillates with
capacitance, and stops a few hundred mV from the rails. With the buffer off the output is a resistor ladder of
around 10 kΩ, and any load divides it.

**Example.** STM32G0B1: with the DAC buffer on, the load must be at least 5 kΩ to ground and at most 50 pF, and the
output stays 0.2 V away from the rails; with it off, the output impedance is 9.6-13.8 kΩ. B153B considers setting
V_SBU with the DAC on PA4 through an LDO's feedback network: that network and the LDO's feedback pin are the DAC's load
and must be checked against these numbers, or buffered.

**How to check.** DAC-capable pins in use (`NET-12`), the resistance and capacitance to ground on their nets, against
the DAC's drive limits from the part data (`IMP-9`).

**Fix.** An op-amp follower after the DAC, a higher-impedance load, or the DAC buffer enabled with the load inside its
limits.

**References.** DS13560 §5.3.19, Table 66 (DAC characteristics: RL, CL, output range, RO with buffer off); AoE3
chapter 13.

## `ANA-20` Analog input above VDDA + 0.3 V, also on a 5 V-tolerant pin <a id="ana-20"></a>

🛑 error · needs: analog pin functions, pin tolerance per mode (part data), voltage of the nets · [catalogue entry](../checks.md#ana-20)

**Impl.** mid · depends on NET-5, NET-7, NET-12, CFG-2

**What it catches.** An ADC, comparator or op-amp input on an MCU pin that can see more than its analog limit (VDDA +
0.3 V, IOVDD + 10 %, or a fixed limit), trusted because the pin is listed as 5 V tolerant.

**Why it matters.** "5 V tolerant" describes the digital input buffer, with the pull resistors off. Once the pin is
switched to the analog mux, the analog switch has its own protection and the limit drops; above it current flows into
the supply, the reading is wrong, and the injection disturbs the neighbouring channels. The datasheet says so in a
footnote, not in the pin table.

**Example.** STM32G0B1 FT_a pins take VDD + 4 V as digital inputs, but "when an FT_a pin is used by an analog
peripheral such as ADC, the maximum VIN is 4 V" (DS13560 Table 21, note 3). On the RP2350 the plain GPIOs are fault
tolerant up to 5.5 V, while the ADC pins (GPIO40-47 on QFN-80) are a different pin type, and the ADC input must not
exceed IOVDD + 10 %; above IOVDD current leaks through the ESD diodes.

**How to check.** For each pin used as an analog input (`NET-12`), the maximum voltage on its net (`NET-5`, `CFG-2`)
against the analog limit of the pin from the part data (`NET-7`), not its digital tolerance.

**Fix.** Scale or clamp the signal into the analog range (`ANA-13`), with a series resistor to limit injection.

**References.** DS13560 Table 21 (voltage characteristics, note 3); RP2350 §12.4 (maximum ADC input voltage set by
IOVDD), §14.8.2.1 (pin types: fault tolerant digital), §14.8.2.2 Table 1674 (ADC GPIOs are not of the FT type);
AN2834 §3.2.9 (injection current effect).

## `ANA-21` AC-coupled input with no DC return path <a id="ana-21"></a>

🛑 error · needs: amplifier inputs, capacitors, transformers, resistive paths · [catalogue entry](../checks.md#ana-21)

**Impl.** easy · depends on netlist

**What it catches.** An op-amp or in-amp input that reaches the rest of the circuit only through a series capacitor or
a floating transformer winding, with no resistor (or other DC path) to ground or a bias voltage.

**Why it matters.** Every input draws a bias current, from picoamps to microamps. With only a capacitor in the path the
current has nowhere to go but onto the capacitor, which charges until the input reaches a rail and the amplifier
saturates. With FET inputs this takes seconds to hours, so the circuit passes a quick bench test and fails in the
field. A transformer secondary that floats has the same problem.

**Example.** A microphone preamp with 1 µF coupling straight into a CMOS op-amp's non-inverting input, biased
nowhere: it works for minutes, then the output drifts to a rail. A 100 kΩ to the mid-rail bias fixes it.

**How to check.** For each amplifier input, walk the nets through resistors, inductors and transformer windings (not
capacitors) and see whether a ground, rail or driven node is reached.

**Fix.** A resistor from the input to ground (dual supply) or to the bias voltage (single supply), sized against the
bias current and the coupling capacitor's corner; a centre tap or resistors to ground on a transformer secondary.

**References.** Kitchin, "Missing DC bias current return path when ac-coupled" (Figures 1-6); MT-038; AoE3 chapter 4.

## `ANA-22` Mid-rail bias divider without a capacitor to ground <a id="ana-22"></a>

💡 suggestion · needs: dividers from a rail, op-amp inputs, capacitors · [catalogue entry](../checks.md#ana-22)

**Impl.** easy · depends on NET-5

**What it catches.** A divider from a supply rail to ground that sets the bias of a single-supply stage (often two
equal resistors to mid-rail), with no capacitor from its midpoint to ground.

**Why it matters.** The divider passes half of all supply noise straight into the signal path: the stage's
power-supply rejection is now that of a resistor divider, 6 dB, instead of the op-amp's 80 dB or more. In
single-supply audio and sensor stages this is the usual source of hum, digital whine and "motor-boating".

**Example.** A 3.3 V microphone stage biased by 100 kΩ / 100 kΩ from the same rail as a BLE radio: the radio's
current pulses at 1.6 kHz appear in the audio at half their supply amplitude. A 10 µF capacitor at the midpoint puts the
corner at 0.3 Hz.

**How to check.** Two resistors between a rail and ground (`NET-5`) whose midpoint drives an op-amp input (directly or
through a resistor) with no capacitor from that midpoint to ground.

**Fix.** A capacitor from the midpoint to ground (corner well below the signal band), or a buffered reference shared by
several stages.

**References.** Kitchin, "Preserving power-supply rejection (PSR) when amplifiers are referenced from the supply rail
using voltage dividers" (Figure 10), "Decoupling single-supply op-amp circuits" (Figures 12, 13); MT-035 "Circuit
design considerations for single-supply systems".

## `ANA-23` Sense path through copper or a high-TCR part, or not from Kelvin points <a id="ana-23"></a>

⚠️ warning · needs: sense path parts, layout geometry · [catalogue entry](../checks.md#ana-23)

**Impl.** hard · depends on NET-10, TOOL-2

**What it catches.** A current measurement whose sense voltage includes copper (a trace used as the shunt, or a stretch
of trace or plane between the shunt pad and the sense connection) or a part with a high temperature coefficient (a
ferrite, a 0 Ω jumper, a fuse), or whose sense lines leave from the power pads instead of the shunt's Kelvin terminals.

**Why it matters.** Copper changes its resistance by about 0.39 % per °C: a sense path with copper in it reads 20 %
high after a 50 °C rise. A 0 Ω jumper is tens of milliohms with a loose tolerance, comparable to the shunt itself.
Where the sense lines attach to the pad matters too: at 5 mΩ, the solder joint and the pad edge are a measurable share.

**Example.** A 1 mΩ shunt whose sense lines connect to the wide power copper 3 mm from the pads: 0.3 mΩ of copper in
series, a 30 % error that grows 0.39 % per °C as the board warms. On a four-terminal shunt the sense pads are separate
and the error is gone.

**How to check.** In the netlist: parts other than the shunt between the shunt and the amplifier inputs (`NET-10`).
The copper part needs the layout: where the sense tracks attach and whether they share copper with the current path
(`TOOL-2`).

**Fix.** Sense from the shunt's Kelvin pads or the inner edge of its pads with dedicated tracks, use four-terminal
shunts at low values, keep jumpers and ferrites out of the sense path, and avoid copper shunts where accuracy matters.

**References.** INA181 §8.4.1 (Kelvin connection); Wikipedia, Four-terminal sensing; Wikipedia, Electrical
resistivity and conductivity (temperature coefficient of annealed copper 3.93 × 10⁻³ /K); `ANA-4`.
