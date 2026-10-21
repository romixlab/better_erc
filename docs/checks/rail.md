# RAIL checks, explained

Detail page for the `RAIL` category of the [check catalogue](../checks.md#-rail---rails-and-regulators). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc
needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of
truth for ID, severity and wording; this page is the commentary.

References used throughout:

- **SLVA477** · TI, *Basic Calculation of a Buck Converter's Power Stage*, SLVA477C. Duty cycle (§2), inductor
  (§3), rectifier diode (§4), feedback divider (§5), capacitors (§6, §7).
  [ti.com/lit/an/slva477](https://www.ti.com/lit/an/slva477/slva477.pdf)
- **SLVA372** · TI, *Basic Calculation of a Boost Converter's Power Stage*, SLVA372D (2022). The same for boost.
  [ti.com/lit/an/slva372](https://www.ti.com/lit/an/slva372/slva372.pdf)
- **SLVA079** · TI, *Understanding the Terms and Definitions of LDO Voltage Regulators* (1999). Dropout (§1),
  instability (§10). [ti.com/lit/an/slva079](https://www.ti.com/lit/an/slva079/slva079.pdf)
- **SLVA115** · TI, *ESR, Stability, and the LDO Regulator*, SLVA115A (2020).
  [ti.com/lit/an/slva115](https://www.ti.com/lit/an/slva115/slva115.pdf)
- **TLV702** · TI, *TLV702 300-mA, Low-IQ, Low-Dropout Regulator*, SLVSAG6D (2019): the LDO family of our
  TLV70233 / TLV70218. [ti.com/lit/ds/symlink/tlv702.pdf](https://www.ti.com/lit/ds/symlink/tlv702.pdf)
- **TLV743P** · TI, *TLV743P* datasheet, SBVS310C (2019): the family of B200A's 1.1 V core LDO TLV74311.
  [ti.com/lit/ds/symlink/tlv743p.pdf](https://www.ti.com/lit/ds/symlink/tlv743p.pdf)
- **AP64060** · Diodes, *AP64060/AP64062 4.5V to 40V Input, 600mA Synchronous Buck*, DS44123 Rev. 4-2 (May 2025):
  B153A's buck. [diodes.com/datasheet/download/AP64060_AP64062.pdf](https://www.diodes.com/datasheet/download/AP64060_AP64062.pdf)
- **RT5760** · Richtek, *RT5760A/B/C/D 6V 1A ACOT Buck Converter*, DS5760A/B/C/D-04 (November 2020): B200A's buck.
  [richtek.com](https://www.richtek.com/SaveDownload.aspx?specid=RT5760A/RT5760B/RT5760C/RT5760D)
- **AOZ1375DI** · Alpha & Omega, *AOZ1375DI 23 V 18 mΩ Bidirectional Load Switch*, Rev. 2.1 (February 2025): B153A's
  VBUS switch. [aosmd.com/res/datasheets/AOZ1375DI-02.pdf](https://www.aosmd.com/res/datasheets/AOZ1375DI-02.pdf)
- **TPS61099** · TI, *TPS61099x Synchronous Boost Converter With Ultra-Low Quiescent Current*, SLVSD88M: a boost
  with true load disconnect. [ti.com/lit/ds/symlink/tps61099.pdf](https://www.ti.com/lit/ds/symlink/tps61099.pdf)
- **LT3080** · Analog Devices (Linear), *LT3080 Adjustable 1.1A Single Resistor Low Dropout Regulator*, rev. C:
  output capacitance and DC bias (Figure 2), paralleling devices (Figure 4; fetched 21 Oct 2026 through
  `tpm fetch`, analog.com refuses plain HTTP clients).
  [analog.com/…/3080fc.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/3080fc.pdf)
- **AN2834** · ST, *How to optimize the ADC accuracy in the STM32 MCUs*, Rev 10 (October 2024; fetched through
  `tpm fetch`). Reference noise and decoupling (§3.2.1-§3.2.3, §4.2.1).
  [st.com/…/an2834](https://www.st.com/resource/en/application_note/an2834-how-to-get-the-best-adc-accuracy-in-stm32-microcontrollers-stmicroelectronics.pdf)
- **RP2350** · Raspberry Pi, *RP2350 Datasheet* (build 1 Oct 2026), §6.1.5 ADC supply, §12.4.3.7 Supply; and
  *Hardware design with RP2350* (build 20 Aug 2026), §2.1. B200A's MCU.
  [datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf),
  [hardware-design-with-rp2350.pdf](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf)
- **KiCad ERC** · KiCad 9 Schematic Editor manual, "Power pins and power flags".
  [docs.kicad.org/9.0/en/eeschema](https://docs.kicad.org/9.0/en/eeschema/eeschema.html#power-pins-and-power-flags)
- **Wikipedia** · [Low-dropout regulator](https://en.wikipedia.org/wiki/Low-dropout_regulator),
  [Buck converter](https://en.wikipedia.org/wiki/Buck_converter),
  [Boost converter](https://en.wikipedia.org/wiki/Boost_converter): readable overviews.
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 9 (voltage regulation and
  power conversion).

## `RAIL-1` Power input pins of parts not powered <a id="rail-1"></a>

🛑 error · needs: pin types (power in), power structure · [catalogue entry](../checks.md#rail-1)

**Impl.** easy · depends on NET-5

**What it catches.** A pin typed power input (VDD, VCC, VIN, AVDD) on a net that is not a power rail: a net with
only that pin, a signal net, or a net no power source reaches.

**Why it matters.** The part is dead, or half alive through its IO clamp diodes (the back-powering of `BKF`). Hidden
power pins, a renamed rail label and a multi-unit symbol whose power unit was never placed are the usual ways it
happens; the sheet looks complete.

**Example.** A quad op-amp drawn as four units A-D with the power pins in unit E, which nobody placed: V+ and V−
exist only in the library, the netlist has them on auto-named single-pin nets.

**How to check.** For each power-input pin, its net must be a rail of the power structure (`NET-5`); report
single-pin nets and nets without a source (`RAIL-5`) separately so the message says which.

**Fix.** Wire the pin to its rail; place the power unit; make hidden power pins visible in the library.

**References.** KiCad ERC "Power pins and power flags" (KiCad's own check, and why a PWR_FLAG silences it).

## `RAIL-2` Regulator output from its divider vs the net it feeds <a id="rail-2"></a>

🛑 error · needs: regulator reference voltage (part data), feedback divider, net voltage · [catalogue entry](../checks.md#rail-2)

**Impl.** mid · depends on NET-5, NET-7, RPT-6

**What it catches.** An adjustable DC-DC or LDO whose output, computed from its reference voltage and the feedback
divider (Vout = VFB · (1 + Rtop / Rbottom)), differs from the voltage in the output net's name or the one the
project specifies.

**Why it matters.** A divider swapped top for bottom, a value from another design or a 1 % slip in a BOM line
puts the wrong voltage on every part of the rail at the first power-up. Nothing in an ordinary ERC looks at values.

**Example.** On B153A the AP64060 (VFB 0.8 V) has 69K8 over 22K1: 0.8 × (1 + 69.8 / 22.1) = 3.33 V on
`+3V3_DCDC`, right, and the datasheet's Table 1 gives the same pair for 3.3 V. On B200A the RT5760 (VFB 0.6 V) with
45K3 over 10K gives 3.32 V. Either pair with the resistors swapped gives 1.05 V (AP64060) or 0.73 V (RT5760).

**How to check.** Find the regulator's FB pin, follow it to the two resistors to the output and to ground, compute
Vout with VFB from the part data, compare with the net name (`NET-5`) or the project's net voltage (`CFG-2`) with
the tolerance of the reference and the resistors.

**Fix.** Recompute the divider from the datasheet equation; keep the bottom resistor at the datasheet's
recommended value and solve for the top one.

**References.** SLVA477 §5 (output voltage setting); AP64060 "Setting the Output Voltage" (Eq. 4, Table 1); RT5760
pin description (FB, 0.6 V).

## `RAIL-3` Net name carries a voltage that is wrong <a id="rail-3"></a>

⚠️ warning · needs: voltage parsed from the net name, the rail's real voltage · [catalogue entry](../checks.md#rail-3)

**Impl.** mid · depends on NET-5, RPT-6

**What it catches.** A net whose name says one voltage (`+1V8_LDO`, `_3V3`, `5V0_`) while the source that feeds it
makes another: a fixed regulator, a divider, a connector pin with a stated voltage.

**Why it matters.** People and firmware trust the name. A rail named 1.8 V that is 3.3 V gets connected to 1.8 V
parts in the next revision, and the test plan measures against the wrong value. When the name is right and the
part is wrong, the same comparison finds the real bug (`RAIL-4`).

**Example.** On B200A `+1V8_LDO` sits at about 3.2 V (a 3.3 V LDO in dropout from 3.32 V); the firmware offered
"1.8 V" IO levels that were really 3.2 V until the review caught it.

**How to check.** Parse the voltage from the name (`NET-5` already does), take the source's voltage from `RAIL-2`
or `RAIL-4`, report a difference beyond the rail tolerance.

**Fix.** Fix the part (usually) or rename the net; never both without deciding which was meant.

**References.** B200A review notes §0, B200 rev B change list.

## `RAIL-4` Fixed-output regulator whose MPN voltage differs from the net it feeds <a id="rail-4"></a>

🛑 error · needs: output voltage decoded from the MPN, net voltage · [catalogue entry](../checks.md#rail-4)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A fixed-output regulator whose part number encodes a voltage (TLV702**33** = 3.3 V,
TLV702**18** = 1.8 V, TLV743**11** = 1.1 V) different from the voltage of the net on its output.

**Why it matters.** The symbol is usually shared across the voltage options and the value field is copied from
the BOM, so the sheet reads "1V8" next to a 3.3 V part and nothing looks wrong. Every part on the rail gets the
wrong supply.

**Example.** The same miss on two boards: B153A U802 and B200A U1602 are TLV70233 (3.3 V) on `+1V8_LDO`. On B200A
every "1.8 V" IO and I2C level was ~3.2 V; both rev B change lists swap to a TLV70218.

**How to check.** Decode the voltage from the MPN with the family's nomenclature (TLV702xx: "XX is nominal output
voltage"), compare with the output net's voltage (`NET-5`), and check every MPN option of the BOM line, not only
the first.

**Fix.** Order the right voltage option, and put the voltage in the symbol's value so the sheet shows it.

**References.** TLV702 §11.1.2 Device Nomenclature; B153 and B200 rev B change lists.

## `RAIL-5` Power net with loads but no source <a id="rail-5"></a>

🛑 error · needs: power structure with sources (regulators, connectors, switches, ties) · [catalogue entry](../checks.md#rail-5)

**Impl.** easy · depends on NET-5

**What it catches.** A rail with power-input pins and capacitors on it but no source: no regulator output, no
connector power pin, no switch, ferrite or 0 Ω from a sourced net.

**Why it matters.** KiCad's ERC only knows pin types: one PWR_FLAG on the net, or a symbol pin typed power output,
silences it. A ferrite or 0 Ω deleted while tidying the sheet leaves a named rail that looks fine and powers
nothing; the parts on it then back-power through their IO pins.

**Example.** On B200A `+5V0_FLT` (V+ of the two TLV9004 op-amps U201/U202, C201, C202) has no source in the
schematic and none on the PCB: the tracks join the four pads to each other only. All eight analog channels read
garbage and the op-amp inputs are back-powered from the IO pins (rework 1: a ferrite from `+5V0_USBIN`).

**How to check.** Walk each rail back through two-pin series parts (ferrites, 0 Ω, fuses, switches) until a source
is found (`NET-5` needs sources and ties for this); a rail with loads and no source is the finding.

**Fix.** Add the missing link; put a test point on every rail so bring-up measures it first.

**References.** KiCad ERC "Power pins and power flags"; B200A review notes §0; B200 rev B change list.

## `RAIL-6` Current-limit resistor vs the limit the design states <a id="rail-6"></a>

⚠️ warning · needs: the part's limit formula, the resistor value, the stated limit · [catalogue entry](../checks.md#rail-6)

**Impl.** hard · depends on NET-7, CFG-2

**What it catches.** An ILIM, ISET or RLIM resistor whose value, put into the datasheet formula, gives a limit
different from the one the sheet note or the project states, or outside the range the datasheet guarantees.

**Why it matters.** The limit is what protects the cable, the connector and the far board; a factor of two off
means either nuisance trips at normal load or no protection at all. Formulas are per part and often hold only
for a stated sense resistor or direction.

**Example.** B153A U601 AOZ1375DI with R605 14K5 and a 5 mΩ shunt; the sheet says "Ilim = 160/Rlim[kΩ] = 5.5 A".
The datasheet formula is 160 / RLIM for a 10 mΩ RSENSE: 160 / 14.5 = 11 A, already outside the 1-6 A range the
datasheet guarantees, before the 5 mΩ shunt is taken into account. It also limits one direction only: "no current
limiting function when configured as sink".

**How to check.** Find the resistor on the limit pin, evaluate the part's formula (with its sense resistor), compare
with the stated limit (`CFG-2`, or the sheet note) and the datasheet's valid range.

**Fix.** Recompute R from the formula for the actual shunt; when the part's range does not reach the wanted limit,
change the shunt or the part.

**References.** AOZ1375DI Detailed Description, "Programmable Over-Current Protection (OCP)" and Figure 20; B153A
review notes §3; B153 rev B change list.

## `RAIL-7` Feedback divider impedance outside the datasheet range <a id="rail-7"></a>

⚠️ warning · needs: divider values, FB bias current or the datasheet's range · [catalogue entry](../checks.md#rail-7)

**Impl.** mid · depends on NET-7, RPT-6

**What it catches.** A feedback divider whose resistors are so high that the FB bias current and noise pickup
shift the output, or so low that the divider wastes the quiescent budget.

**Why it matters.** A 1 MΩ / 220 kΩ divider is tempting on a low-Iq design: the FB pin's bias current flows through
the bottom resistor and moves Vout, and a high-impedance node next to the switch node picks up noise. A 1 kΩ
divider on a 3.3 V rail burns 3 mA, more than the whole sleeping board.

**Example.** A TPS61099 (VREF 1.0 V, FB leakage up to 50 nA) set to 3.3 V with 4.7 MΩ over 2 MΩ to save
microamps: 0.5 µA through the divider, only 10 times the leakage, which flows through the top resistor and can move
the output by 50 nA × 4.7 MΩ = 0.24 V. The datasheet's own example uses 1 MΩ over 249 kΩ.

**How to check.** Compute the divider current and compare with 100 × IFB, or with the datasheet's recommended
bottom-resistor range.

**Fix.** Keep the divider current at least 100 times the FB bias current; use the datasheet's recommended values
(AP64060 Table 1 uses 22K1 for the bottom resistor) and add the feed-forward capacitor it suggests if the top
resistor is large.

**References.** SLVA477 §5 (Equation 9: divider current ≥ 100 × IFB); TPS61099 §8.2.2.2 (R2 current 100 times
the FB leakage), §6.5 (IFB_LKG); AP64060 "Setting the Output Voltage" (too high values make the output susceptible
to noise).

## `RAIL-8` LDO output capacitor not what the part needs for stability <a id="rail-8"></a>

⚠️ warning · needs: the LDO's output capacitor requirement, capacitor values and dielectric · [catalogue entry](../checks.md#rail-8)

**Impl.** mid · depends on NET-7, RPT-1

**What it catches.** An LDO whose output capacitance is below the datasheet minimum, or of the wrong kind: ceramic
on an old part that needs some ESR to be stable, or a high-ESR electrolytic on a part designed for ceramics.

**Why it matters.** An LDO is a feedback loop; the output capacitor and its ESR place a pole and a zero in it. Out
of the stable window it oscillates, sometimes only at some load or temperature, and the "noisy rail" is found
months later.

**Example.** A TLV702 needs 1 µF ceramic on OUT (stable down to 0.1 µF effective). An LT3080 wants at least 2.2 µF
with ESR ≤ 0.5 Ω; fitted with a 16 V 10 µF Y5V on a 5 V output, the effective value can drop to 1-2 µF.

**How to check.** Sum the capacitors on the LDO output net (`RPT-1`), derated per dielectric and voltage, compare
with the part's minimum and ESR window.

**Fix.** Follow the datasheet's capacitor section; X5R/X7R ceramics for modern LDOs, an ESR-tolerant part or a
small series resistor where the datasheet asks for ESR.

**References.** SLVA115 §1; SLVA079 §10 (instability of the LDO, Figure 12 stable range of CSR); TLV702 §8.2.2.1; LT3080
"Stability and Output Capacitance".

## `RAIL-9` LDO headroom below its dropout at load <a id="rail-9"></a>

🛑 error · needs: input range, output voltage, load current, dropout curve · [catalogue entry](../checks.md#rail-9)

**Impl.** mid · depends on NET-5, NET-7, CFG-2, RPT-5

**What it catches.** An LDO whose minimum input voltage minus its output is less than its dropout at the load
current.

**Why it matters.** In dropout the LDO is a resistor: the output follows the input down, ripple passes straight
through (PSRR collapses) and the "3.3 V" rail is 3.1 V on a half-empty battery. It works on the bench supply and
fails in the field.

**Example.** 3.3 V from a Li-ion cell through a TLV70233: dropout 260 mV typical, 375 mV maximum at 300 mA, so the
rail falls out of regulation below about 3.7 V, which is most of the cell's discharge curve. B200A's TLV70233 fed
from 3.32 V has no headroom at all and sits at ~3.2 V.

**How to check.** Vin(min) from the source rail or the project (`CFG-2`, `RPT-5`), load current from the project,
dropout from the part data at that current and temperature.

**Fix.** A lower output voltage, a part with less dropout, a buck-boost, or a higher input rail.

**References.** SLVA079 §1 (dropout voltage); TLV702 §6.5 (VDO), §7.3.3, §7.4.2; TLV743P §8.1.2; Wikipedia,
Low-dropout regulator.

## `RAIL-10` Switching regulator bootstrap capacitor missing or off value <a id="rail-10"></a>

🛑 error · needs: BST pin role, capacitor between BST and SW · [catalogue entry](../checks.md#rail-10)

**Impl.** mid · depends on NET-7

**What it catches.** A buck with a BST / BOOT pin and no capacitor from it to SW, a capacitor to ground instead of
to SW, or a value far from the datasheet's.

**Why it matters.** The high-side N-MOSFET needs a gate voltage above Vin; the bootstrap capacitor, refreshed while
SW is low, provides it. Without it the high side never turns fully on: no output, or a hot regulator in linear
mode. To ground it is simply wrong; far too small and it sags during long on-times.

**Example.** The AP64060 on B153A: 100 nF from BST (pin 1) to SW (pin 6), as the datasheet asks. A copy of the
symbol with BST to GND would give a buck that never starts.

**How to check.** For each BST pin (`NET-7` pin role), find a capacitor whose other pin is the part's SW net;
compare its value with the part data.

**Fix.** A ceramic of the datasheet value (typically 100 nF) between BST and SW, close to the pins.

**References.** AP64060 pin description (BST) and "Bootstrap Capacitor and Low-Dropout (LDO) Operation".

## `RAIL-11` Feedback divider sensing the wrong node <a id="rail-11"></a>

⚠️ warning · needs: FB pin, the divider's top node, series parts on the rail · [catalogue entry](../checks.md#rail-11)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A feedback divider whose top is connected before a ferrite, filter, current shunt or switch, so
the regulated voltage is not the one the load sees; or a top on a different net than the regulator's output.

**Why it matters.** Everything between the sense point and the load drops voltage with current: a ferrite's DCR, a
shunt, a load switch's Rds(on). At full load the load sees less than the nominal, and a filter inside the loop can
make the regulator unstable.

**Example.** A 1.0 V FPGA core buck sensing before a 0.1 Ω ferrite: at 2 A the core sees 0.8 V, below its 0.95 V
minimum.

**How to check.** Follow FB's top resistor to its net; if that net reaches the load rail only through a series
part (`NET-5` ties), report it with the part's resistance where known.

**Fix.** Sense at the load (after the filter), with the datasheet's guidance for filters inside the loop, or
take the ferrite out of the high-current path (`DEC-9`).

**References.** SLVA477 §5 (the divider sets the voltage at its top node); AoE3 chapter 9 (remote sensing).

## `RAIL-12` Conversion ratio vs switching frequency <a id="rail-12"></a>

⚠️ warning · needs: Vin range, Vout, fSW, minimum on-time and maximum duty cycle · [catalogue entry](../checks.md#rail-12)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** A buck whose duty cycle D ≈ Vout / Vin needs an on-time D / fSW below the part's minimum
on-time at the highest input, or a duty cycle above its maximum at the lowest input (unless it has a 100 % mode).

**Why it matters.** Below the minimum on-time the converter cannot make the pulse it needs: it skips pulses, the
ripple grows and the frequency drops into audible or EMI-sensitive bands. Above the maximum duty cycle it falls
out of regulation.

**Example.** The AP64060 on B153A runs at 2 MHz with a 60 ns minimum on-time; 3.3 V out needs D = 0.165 at 20 V
(82 ns, fine) but only 41 ns at the 40 V on the sheet note: above about 27 V input the part cannot hold its
frequency. B153A's input stops at 20 V (USB PD), so the board is safe; a 24 V industrial input would not be. A
24 V to 1 V buck at 2 MHz needs 21 ns, below almost every part.

**How to check.** From Vin range (`CFG-2`), Vout and fSW, compute the on-time at Vin(max) and duty cycle at
Vin(min); compare with tON_MIN and D(max) from the part data.

**Fix.** A lower-frequency sibling (AP64062, 1.2 MHz), two stages, or a part with a smaller minimum on-time; on the
low end a part with 100 % duty cycle (the RT5760 has it).

**References.** SLVA477 §2 (Equation 1, duty cycle); AP64060 Electrical Characteristics (tON_MIN, fSW); RT5760
"Maximum Duty Cycle Operation"; Wikipedia, Buck converter.

## `RAIL-13` Boost converter output not isolated when disabled <a id="rail-13"></a>

⚠️ warning · needs: boost topology, part's disconnect feature, power states · [catalogue entry](../checks.md#rail-13)

**Impl.** hard · depends on NET-6, NET-7, NET-8

**What it catches.** A boost whose output is meant to be off in some power state, built with a part (or a discrete
inductor and diode) that leaves a DC path from input to output when disabled.

**Why it matters.** In a classic boost the inductor and the rectifier are a straight path from Vin to Vout: "off"
is Vin minus a diode drop. A 5 V USB output from a battery boost stays at 3.5 V when off, a sensor rail meant to
cut power keeps it, and the "disabled" load drains the battery.

**Example.** A 3.7 V cell boosted to 5 V for a USB port with an asynchronous boost and a Schottky: with EN low the
port still has 3.4 V and the attached device draws the battery flat. A boost with true load disconnect
(TPS61099 class: the rectifying PMOS turns fully off) avoids it.

**How to check.** Recognise the boost (inductor from the input to the SW node, rectifier to the output, `NET-6`);
look up whether the part disconnects; flag it when the output rail is off in some power state (`NET-8`).

**Fix.** A boost with load disconnect, or a load switch after the boost.

**References.** SLVA372 §1 (topology); TPS61099 §7.1, §7.3.3; Wikipedia, Boost converter.

## `RAIL-14` PGOOD output without a pull-up, or pulled to a rail that is not up <a id="rail-14"></a>

⚠️ warning · needs: PG pin role (open drain), its pull-up and the pull-up rail's state · [catalogue entry](../checks.md#rail-14)

**Impl.** mid · depends on NET-7, NET-8

**What it catches.** A power-good output (open drain) with no pull-up resistor on its net, or with one to a rail
that is off when the signal is used (to sequence the next regulator, or read by an MCU that is not up yet).

**Why it matters.** An open-drain PG without a pull-up is always low (or floating): a sequencer waiting on it never
releases the next rail, an MCU reads "power bad" forever. Pulled to the rail it is meant to sequence, it can never
go high.

**Example.** B200A's RT5760 PG goes only to a test point TP1602; fine as long as nothing uses it, but the BOM also
allows the RT5760C/D, which have no PG pin at all (pin 6 is NC). Using PG later means both a pull-up and pinning
the BOM to the A/B variants.

**How to check.** For each PG / PGOOD pin (`NET-7`), a resistor to a rail on its net; that rail on in every state
where PG is read (`NET-8`).

**Fix.** A pull-up (10-100 kΩ) to a rail that is up before PG is needed, usually the input or an always-on rail.

**References.** RT5760 pin description (PG: open drain with external pull-up), Table 1 PG Pin Status, Ordering
Information.

## `RAIL-15` Enable pin tied or driven above its maximum, or below its high threshold <a id="rail-15"></a>

🛑 error · needs: EN absolute maximum and VIH, the voltage of what drives it · [catalogue entry](../checks.md#rail-15)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** An EN pin tied straight to a rail above its absolute maximum, or driven from a rail or GPIO
whose high is below the EN high threshold.

**Why it matters.** Many regulators have a low-voltage EN (6 V, or Vin + 0.3 V) even when Vin goes much higher;
tying EN to Vin is a habit copied from parts whose EN is high-voltage. Above the rating the pin breaks down; below
VIH the part never starts or starts at random.

**Example.** The AP64060 on B153A has EN tied to VIN: fine, its EN "is a high-voltage pin and can be directly
connected to VIN". The same tie on a TLV702 is out of rating above 6 V (EN max is VIN + 0.3 V or 6 V, whichever
is smaller); an AOZ1375DI (EN max 6 V) tied to a 20 V VBUS breaks down.

**How to check.** The EN net's voltage (rail from `NET-5`, or the driving pin's bank) against the part's EN
absolute maximum and VIH (`NET-7`).

**Fix.** A divider from Vin (which also sets a UVLO, `RAIL-25`), or drive EN from logic of the right level.

**References.** AP64060 "Enable"; TLV702 §6.1 (EN absolute maximum, note 3), §6.5 (VEN(high)); AOZ1375DI
Absolute Maximum Ratings (EN to GND 6 V).

## `RAIL-16` Soft-start, compensation or timing pins without the required parts <a id="rail-16"></a>

🛑 error · needs: pin roles (SS, COMP, RT, CT) and what the datasheet requires on them · [catalogue entry](../checks.md#rail-16)

**Impl.** mid · depends on NET-7

**What it catches.** A regulator's SS, COMP, RT or CT pin left open or tied where the datasheet requires a
capacitor, an RC network or a resistor.

**Why it matters.** An external-compensation buck with COMP open has no loop compensation: it oscillates or
latches off. RT open sets no frequency (some parts default, others do not start). SS open starts with no soft
start, so the inrush trips the source's current limit (`SEQ-6`).

**Example.** The AOZ1375DI on B153A has its SS pin with 1 nF; drawn from a template that left SS open, the switch
would turn on hard into the plug's capacitance.

**How to check.** For each pin with one of these roles (`NET-7`), the parts the datasheet requires on it (a
capacitor to ground, a resistor to ground, an RC to ground); a single-pin net is the clearest miss.

**Fix.** Add the parts from the datasheet's typical application, with values from its design procedure.

**References.** AOZ1375DI pin description (SS) and Detailed Description; SLVA477 §1 (necessary parameters of the
power stage).

## `RAIL-17` Regulator configuration pins floating where the datasheet says tie them <a id="rail-17"></a>

🛑 error · needs: pin roles (MODE, SYNC, FREQ, FPWM, VSEL), the datasheet rule · [catalogue entry](../checks.md#rail-17)

**Impl.** mid · depends on NET-7

**What it catches.** A MODE, SYNC, FREQ, FPWM or VSEL pin on a single-pin net when the datasheet says to tie it high
or low (or allows floating only for some options).

**Why it matters.** A floating CMOS configuration input picks a mode at random, per board and per temperature: one
unit runs in PFM with its light-load ripple, the next in forced PWM; a VSEL picks the output voltage.

**Example.** A buck with a MODE pin (high = forced PWM, low = PFM) left open because an older sibling had an
internal pull-down: on the part fitted, the pin drifts, the converter changes between PFM and PWM from board to
board, and the light-load ripple on the ADC rail with it.

**How to check.** Single-pin nets on configuration pins (`NET-7` role); report unless the part data marks the pin
as having a defined internal pull.

**Fix.** Tie to the level the design wants, or through a 0 Ω option for both.

**References.** RT5760 Ordering Information (the operating mode is fixed by the part option there, not a pin: one
reason the same check must read the part data).

## `RAIL-18` Two regulator outputs tied together <a id="rail-18"></a>

🛑 error · needs: power output pins on one net, parts that allow paralleling · [catalogue entry](../checks.md#rail-18)

**Impl.** easy · depends on NET-5

**What it catches.** Two regulator outputs (LDO, DC-DC, a module) on the same net without ORing, ballast or a part
designed for paralleling.

**Why it matters.** Two regulators never agree on the voltage: the higher one carries all the load while the
other's loop drives its output to zero current, or one sinks current from the other (a synchronous buck can sink),
and current sharing is anyone's guess. Sometimes it is a leftover of two footprint options both populated.

**Example.** Two TLV70233 in parallel "for 600 mA": the one with the higher output takes nearly all the load and
current-limits, the other idles. The LT3080 is built for paralleling: SET pins tied together and a few milliohms of
trace as ballast to share the current.

**How to check.** Count power-output pins (regulator OUT/VOUT/SW-after-inductor) per rail (`NET-5`); more than one
is a finding unless the part data marks the part as parallel-capable and ballast is present.

**Fix.** One regulator of the needed current, a part made for paralleling with ballast, or ORing diodes /
ideal-diode controllers when two sources are intended (`PIN-12`).

**References.** LT3080 "Paralleling Devices" (Figure 4, Table 1); KiCad ERC "Power pins and power flags" (why two
power outputs on one net are already a pin conflict).

## `RAIL-19` Analog supply fed straight from the digital rail without a filter <a id="rail-19"></a>

💡 suggestion · needs: analog supply pins (VDDA, AVDD, VREF+), the path to the rail · [catalogue entry](../checks.md#rail-19)

**Impl.** easy · depends on NET-5

**What it catches.** An analog supply pin connected to the digital rail with no ferrite, RC or separate regulator
between them, only a 0 Ω or a direct connection.

**Why it matters.** The digital rail carries the switching noise of the MCU core, the buck ripple and the IO edges;
the ADC, PLL or analog front end sees it as noise on its supply and, when the supply is the reference, as noise
on every reading.

**Example.** B200A: `+3V3_VDDA` is `+3V3_DCDC` through R1802 0 Ω with 4.7 µF + 100 nF. The RP2350 ADC uses
ADC_AVDD as its reference (the B200A notes: "the ADC reference is the 3.3 V rail"), so the buck's 2.2 MHz ripple
goes into every conversion. The datasheet separates ADC_AVDD "to allow noise filtering"; its hardware guide uses an
RC filter (33 Ω, 4.7 µF) for the even more sensitive VREG_AVDD.

**How to check.** Analog supply pins by name; walk back to the source rail; a path with only 0 Ω or nothing is the
finding.

**Fix.** A ferrite or a small RC (sized to the pin's current) with the decoupling on the pin side (`DEC-8`), or a
separate LDO; keep a 0 Ω footprint for the option.

**References.** RP2350 datasheet §6.1.5, §12.4.3.7; Hardware design with RP2350 §2.1; AN2834 §4.2.1.

## `RAIL-20` Reference input without its own decoupling, or from a noisy rail <a id="rail-20"></a>

⚠️ warning · needs: reference pins (VREF+, REF, VREFIN), their decoupling and source · [catalogue entry](../checks.md#rail-20)

**Impl.** mid · depends on NET-5, NET-7, RPT-1

**What it catches.** A converter or MCU reference input with no capacitor of its own at the pin, or sourced from a
rail with switching noise and no filter.

**Why it matters.** Every ADC reading is a ratio to the reference: reference noise and droop during the sampling
current spikes are conversion errors, one for one. A shared capacitor three centimetres away does not hold the
reference during the charge-redistribution spikes.

**Example.** An STM32 in a 100-pin package with an external reference on VREF+ and no capacitor at the pin: AN2834
asks for 10 nF and 1 µF on VREF+ (and 10 nF + 1 µF on VDDA). On B200A the RP2350's reference is its ADC_AVDD,
`+3V3_VDDA`, which has decoupling but no filter from the buck (`RAIL-19`).

**How to check.** Reference pins by role; a capacitor directly on the pin net to ground (`RPT-1`); its source rail
traced back for a filter (`RAIL-19`).

**Fix.** The decoupling the datasheet gives at the pin; for accuracy an external reference (`ANA`) or a filtered
rail.

**References.** AN2834 §3.2.1 (reference voltage noise), §3.2.3 (reference decoupling and impedance), §4.2.1 (VDDA and
VREF+ decoupling, Figure 20).

## `RAIL-21` Total output capacitance outside the stable range after DC-bias derating <a id="rail-21"></a>

⚠️ warning · needs: capacitors per rail with case, voltage rating and dielectric, regulator limits · [catalogue entry](../checks.md#rail-21)

**Impl.** mid · depends on NET-7, RPT-1

**What it catches.** A regulator whose output capacitance, summed over the rail and derated for DC bias, falls
outside the range the datasheet gives for stability, or is more than its soft start and current limit can charge.

**Why it matters.** Class II ceramics lose capacitance with applied voltage: a 10 µF 0603 X5R at 5 V is roughly
4 µF, a Y5V far less. The design "has 22 µF" on paper and is at the edge of stability in reality. At the other
end, every downstream decoupling capacitor adds to the regulator's output: a rail with 500 µF on it may never
finish soft start before the current limit or the hiccup timer kicks in.

**Example.** An LT3080 at 5 V with a 16 V 10 µF Y5V as its only output capacitor: 1-2 µF effective, below its
2.2 µF minimum. On the high side, a 1 A buck with 470 µF of downstream bulk and 1 ms soft start needs 1.55 A
(470 µF × 3.3 V / 1 ms) on top of the load.

**How to check.** `RPT-1` (capacitance per net derated at working voltage, from MPN or case and rating), compared
with the datasheet's minimum and maximum output capacitance; C · Vout / tSS against the current limit.

**Fix.** Larger case or higher-rated capacitors, X7R over Y5V; a longer soft start or a load switch to separate
the bulk from the regulator.

**References.** LT3080 "Stability and Output Capacitance" (Figure 2, Figure 3); SLVA477 §7 (output capacitor
selection); AP64060 "Output Capacitor".

## `RAIL-22` Inductor value outside the datasheet range <a id="rail-22"></a>

⚠️ warning · needs: inductor value and ratings, fSW, Vin, Vout, load current · [catalogue entry](../checks.md#rail-22)

**Impl.** mid · depends on NET-7, RPT-3

**What it catches.** A switching regulator's inductor whose value is far from the one the datasheet's equation or
table gives for the switching frequency, voltages and current; or whose saturation current is below the peak.

**Why it matters.** Too small: the ripple current is large, peak current reaches the switch limit before the rated
load, and the output ripple grows. Too large: slow transient response, and for current-mode parts with internal
slope compensation, subharmonic oscillation. A saturating inductor is a short at the peak.

**Example.** The AP64060's Table 1 gives 8.2 µH at 3.3 V for the 2 MHz AP64060 and 10 µH for the 1.2 MHz AP64062;
B153A fits 10 µH on the AP64060, the value of the slower sibling (in range here, but the kind of copy `RAIL-23`
is about). B200A's RT5760 at 2.2 MHz uses 1 µH.

**How to check.** L = Vout · (Vin − Vout) / (Vin · ΔIL · fSW) with ΔIL of 20-40 % of the load, against the fitted
value; peak current Iout + ΔIL / 2 against the inductor's Isat (`RPT-3`).

**Fix.** The datasheet's recommended value; an inductor with Isat above the switch current limit.

**References.** SLVA477 §3 (inductor selection); SLVA372 §3; AP64060 "Inductor" (Eq. 5) and Table 1.

## `RAIL-23` Regulator symbol shared by siblings with different VFB or Vout <a id="rail-23"></a>

⚠️ warning · needs: MPN options of the BOM line, sibling data · [catalogue entry](../checks.md#rail-23)

**Impl.** mid · depends on NET-5, NET-7, RPT-6

**What it catches.** A regulator symbol or BOM line whose orderable variants differ in feedback voltage, fixed
output voltage, frequency or pinout, so the divider or the net was computed for another sibling than the one
ordered.

**Why it matters.** Vendors sell families on one datasheet: 0.6 V and 0.8 V reference options, 1.8 V and 3.3 V
fixed outputs, pins that are PG on one option and NC on another. The symbol says "RT5760" or "TLV702", the
divider matches one sibling, and the BOM orders another.

**Example.** The AP64060 (VFB 800 mV, 2 MHz) and AP64062 (810 mV, 1.2 MHz) share a datasheet; the RT5760A/B have PG
on pin 6, the RT5760C/D have it NC; B200A's BOM allows A, C and D. The `+1V8_LDO` misses of `RAIL-4` are the
same story with fixed outputs.

**How to check.** For each MPN option on the BOM line, decode its VFB / Vout / pin differences from the part data,
recompute `RAIL-2` or `RAIL-4` per option; flag options that disagree.

**Fix.** Pin the BOM to the options that match, and put the deciding letter in the symbol's value.

**References.** AP64060 Electrical Characteristics (VFB per part); RT5760 Ordering Information and pin
description; TLV702 §11.1.2.

## `RAIL-24` Regulator EN pin floating on a part without an internal pull <a id="rail-24"></a>

🛑 error · needs: EN pin role, whether the part pulls it internally · [catalogue entry](../checks.md#rail-24)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** A regulator or load switch EN pin on a single-pin net, or driven only by a GPIO that is
high-impedance at reset with no external pull, on a part without a documented internal pull.

**Why it matters.** A floating CMOS EN picks up whatever couples into it: the regulator starts on some boards,
not on others, or turns on when a finger comes near. Driven by a GPIO that is an input at reset, the rail's state
during boot is undefined.

**Example.** The RT5760: "Do not leave this pin floating". The AOZ1375DI: "The EN pin must be driven to a logic
high or logic low state to guarantee operation". B153A does it right: the AOZ1375's EN on PA6 has a 10K pull-down
R601, so plug VBUS is off at reset.

**How to check.** EN pins (`NET-7`) on single-pin nets, or on nets with only a tristate-at-reset driver and no
resistor to a rail (`NET-9`); exempt parts whose data gives an internal pull.

**Fix.** Tie EN to its rail (within `RAIL-15`), or a pull to the state wanted at reset.

**References.** RT5760 pin description (EN); AOZ1375DI Detailed Description, "Enable"; B153A review notes §3.

## `RAIL-25` Input range below the regulator's minimum Vin or UVLO <a id="rail-25"></a>

⚠️ warning · needs: input voltage range, the regulator's Vin(min) and UVLO · [catalogue entry](../checks.md#rail-25)

**Impl.** mid · depends on NET-5, NET-7, CFG-2

**What it catches.** A regulator whose input, at the low end of its range (a discharged battery, a sagging cable,
the far end of a diode-OR), drops below the part's minimum operating Vin or its UVLO threshold.

**Why it matters.** Below UVLO the part turns off; around it, it may cycle on and off as the load pulls the input
down and releases it. A 4.5 V-minimum buck on a 3-cell alkaline pack (3.0-4.8 V) runs only while the batteries are
fresh.

**Example.** B153A's AP64060 (4.5-40 V, UVLO 3.8 V falling) runs from VBUS through a 1N5819W diode-OR: USB's
4.75 V VBUS minus a Schottky drop at load is near 4.3-4.4 V, below the 4.5 V the datasheet specifies though above
UVLO, so the part keeps running out of its specified range on a weak port.

**How to check.** Vin(min) of the input rail (`CFG-2`, minus series drops of diodes and switches on the way) against
the part's minimum Vin and UVLO (`NET-7`).

**Fix.** A part with a lower minimum Vin, a buck-boost, an ideal diode instead of a Schottky in the OR, or an
adjusted UVLO divider on EN where the source should be cut off (`PIN-16`).

**References.** AP64060 "Adjusting Undervoltage Lockout (UVLO)" and Electrical Characteristics; TLV702 §7.3.4;
B153A review notes §2.

## `RAIL-26` Asynchronous converter rectifying through a slow diode or a body diode <a id="rail-26"></a>

⚠️ warning · needs: switching node, the rectifier part and its type · [catalogue entry](../checks.md#rail-26)

**Impl.** mid · depends on NET-6, NET-7

**What it catches.** A non-synchronous buck or boost whose rectifier is a standard silicon diode (1N4148, 1N400x,
S1x) or relies on a MOSFET body diode instead of a Schottky.

**Why it matters.** The rectifier carries the full inductor current every cycle. A silicon diode's 0.7-1 V forward
drop costs efficiency, and its reverse recovery at hundreds of kHz turns into current spikes, heat and EMI on the
switch node. A 1N400x has microseconds of recovery: at 500 kHz it barely blocks at all.

**Example.** A 12 V to 5 V asynchronous buck at 500 kHz built with an S1M (1 A, 1000 V standard recovery) instead of
an SS34: hot diode, ringing switch node, efficiency down by 10 points.

**How to check.** Find the diode on the switch node (`NET-6`), decode its type from the MPN or part class
(Schottky, ultrafast, standard), report a non-Schottky in a low-voltage converter.

**Fix.** A Schottky with forward current above the output current and a voltage rating above Vin (boost: Vout);
or a synchronous part.

**References.** SLVA477 §4 (rectifier diode selection: use Schottky diodes); SLVA372 §4; AoE3 chapter 9.
