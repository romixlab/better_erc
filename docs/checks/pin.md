# PIN checks, explained

Detail page for the `PIN` category of the [check catalogue](../checks.md#️-pin---power-input-and-output-protection).
Each section explains one check: what it catches, why the board fails without it, a concrete example, what
better_erc needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the
source of truth for ID, severity and wording; this page is the commentary. The `Impl.` line says how hard the check
is to implement in better_erc and which [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **SLVA139** · TI, *Reverse Current/Battery Protection Circuits* (2003): series diode, low-side NMOS and high-side
  PMOS. [ti.com/lit/an/slva139](https://www.ti.com/lit/an/slva139/slva139.pdf)
- **SLVAE57** · TI, *Basics of Ideal Diodes*, rev. B (2021): Schottky vs PMOS vs ideal-diode controllers for
  reverse battery protection and ORing. [ti.com/lit/an/slvae57](https://www.ti.com/lit/an/slvae57/slvae57.pdf)
- **SLVA862A** · TI, *Basics of eFuses* (2016, rev. 2018): fuses, PTCs and eFuses compared; inrush, overcurrent,
  overvoltage and reverse polarity protection. [ti.com/lit/an/slva862a](https://www.ti.com/lit/an/slva862a/slva862a.pdf)
- **LM74700-Q1** · TI, *Low IQ Reverse Battery Protection Ideal Diode Controller* datasheet, SNOSD17G (2020): TVS
  selection and ORing. [ti.com/lit/ds/symlink/lm74700-q1](https://www.ti.com/lit/ds/symlink/lm74700-q1.pdf)
- **Fuseology** · Littelfuse, *Fuseology Selection Guide: Fuse Characteristics, Terms and Consideration Factors*
  (fetched 21 Oct 2026 through `tpm fetch`, littelfuse.com refuses plain HTTP clients): the eleven "Selection
  Factors", derating, I²t, and the PTC terms.
  [littelfuse.com](https://www.littelfuse.com/~/media/electronics/product_catalogs/littelfuse_fuseology_selection_guide.pdf.pdf)
- **SMF** · Littelfuse, *TVS Diodes SMF Series, Surface Mount 200 W* datasheet (fetched through `tpm fetch`): the
  standoff, breakdown and clamping table quoted below.
  [littelfuse.com](https://www.littelfuse.com/assetdocs/tvs-diodes-smf-datasheet?assetguid=7eb8a5b6-bdd0-4561-8f19-0c3cc6f9b2af)
- **AN88** · Linear Technology (now ADI), Application Note 88, *Ceramic Input Capacitors Can Cause Overvoltage
  Transients* (2001) (fetched through `tpm fetch`).
  [analog.com](https://www.analog.com/media/en/technical-documentation/application-notes/an88f.pdf)
- **SNVA538** · National Semiconductor (TI), *Input Filter Design for Switching Power Supplies*.
  [ti.com/lit/an/snva538](https://www.ti.com/lit/an/snva538/snva538.pdf)
- **SLVA769A** · TI, *Understanding Undervoltage Lockout in Power Devices* (2016, rev. 2018).
  [ti.com/lit/an/slva769](https://www.ti.com/lit/an/slva769/slva769.pdf)
- **Wikipedia** · readable overviews: [Transient-voltage-suppression diode](https://en.wikipedia.org/wiki/Transient-voltage-suppression_diode),
  [Resettable fuse](https://en.wikipedia.org/wiki/Resettable_fuse), [Fuse (electrical)](https://en.wikipedia.org/wiki/Fuse_%28electrical%29),
  [USB hardware](https://en.wikipedia.org/wiki/USB_hardware) ("Voltage tolerance and limits": VBUS up to 5.5 V
  since USB Type-C).
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 9 (voltage regulation
  and power conversion).

## `PIN-1` No reverse polarity protection on a power input <a id="pin-1"></a>

⚠️ warning · needs: power inputs (connector pins on rails), the parts in series with them · [catalogue entry](../checks.md#pin-1)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A supply that enters the board on a connector a person can wire or plug the wrong way round
(terminal block, barrel jack, battery holder, pin header) and reaches the first regulator or load with no series
diode, PMOS, ideal-diode controller or eFuse with reverse blocking in between.

**Why it matters.** Reversed, every IC on the rail sees its ground above its supply: the ESD and body diodes of
the regulator conduct and the only current limit is the source. Electrolytic and tantalum capacitors on the input
are reverse biased and can vent. One swapped field wire destroys the board.

**Example.** A 24 V terminal-block input wired straight to a buck regulator and a 100 µF electrolytic. Swapped
on site, the regulator's VIN-to-GND diode carries the supply's full short-circuit current and the capacitor sees
−24 V. Our B200A (USB-C only, so no reversal through the receptacle) has "reverse power: add a check and document
it" on its rev B change list: the case is common enough to want a rule.

**How to check.** Find the inputs: connector pins on a rail of the power structure (`NET-5`) that are a source,
not an output. Walk from the pin to the first powered part; accept a diode with its anode towards the connector, a
PMOS with its body diode in the forward direction, an NMOS in the return, or a part the library marks as
ideal-diode, eFuse or reverse-blocking (`NET-7`). Keyed connectors that cannot be reversed (USB, a polarised
locking header with a matching cable) are exempt by library class or project setting.

**Fix.** A series Schottky for small currents; a PMOS (or a low-side NMOS) for anything over about an ampere
(`PIN-13`, `PIN-17`); an ideal-diode controller (LM74700-Q1 class) or an eFuse with reverse protection where the
drop or the dissipation matters.

**References.** SLVA139 (Figures 1-3: diode, NMOS, PMOS); SLVAE57 §2 (reverse battery protection), §4 (with
MOSFETs); SLVA862A §3.8 (input reverse polarity protection).

## `PIN-2` Fast blow fuse where inrush is expected <a id="pin-2"></a>

⚠️ warning · needs: fuse type (fast-acting or time-lag) from the MPN, the capacitance and loads behind it · [catalogue entry](../checks.md#pin-2)

**Impl.** mid · depends on NET-5, RPT-1

**What it catches.** A fast-acting or very-fast-acting fuse ahead of a load that draws a large current pulse in
normal operation: bulk capacitance charged on plug-in, a motor start, a hot-plugged downstream board.

**Why it matters.** A fuse opens on the energy it absorbs (I²t), not on the current alone. A fast fuse sized for
the steady current melts on a few milliseconds of inrush, or survives it partly melted and opens weeks later after
enough plug cycles: a nuisance trip that looks like a random field failure.

**Example.** A 1 A very-fast-acting chip fuse on a hot-plugged 24 V input followed by 2 × 220 µF. Only the cable
and the capacitors' ESR limit the plug-in current: tens of amperes for a few hundred microseconds, an I²t in the
range of the fuse's own melting I²t.

**How to check.** Identify the fuse type from the MPN or the library (fast, very fast, time-lag); sum the
capacitance downstream up to the next switch or regulator with soft start (`RPT-1`); flag a fast fuse with more
than a project threshold of bulk capacitance or a motor, relay or connector-to-another-board load behind it.

**Fix.** A time-lag (Slo-Blo) fuse whose I²t covers the inrush pulse with margin, a soft-start or inrush limiter
in front of the bulk capacitance (`SEQ-6`), or an eFuse with a set inrush slew.

**References.** Fuseology "Selection Factors" 6 (pulses, surge, inrush and start-up currents) and "Pulses" with
the I²t worked example; SLVA862A §1 (fuse blowup during inrush), §3.1 (inrush current control).

## `PIN-3` Power outputs on connectors from nets used on board <a id="pin-3"></a>

⚠️ warning · needs: connector pins on rails, the loads on the same rail · [catalogue entry](../checks.md#pin-3)

**Impl.** easy · depends on NET-5

**What it catches.** A connector pin that hands a board rail to the outside world (a header, an expansion
connector, a sensor port) with nothing between the pin and the rail that the board's own parts run from, or only a
slow polyfuse.

**Why it matters.** Whatever is plugged in can short the pin: a miswired cable, a dead adapter card, a probe slip.
The short pulls down the rail the MCU runs on, so the board browns out instead of reporting the fault, and a
polyfuse takes from milliseconds to seconds to trip, which is long enough to reset everything and sometimes to
latch a regulator off.

**Example.** B200A J102 pin 1 `+3V3_OUT` is the `+3V3_DCDC` rail through polyfuse F102, pin 29 `+5V0_OUT` is
VBUS through F101 (SMD1206B010, 100 mA hold, 300 mA trip). A short on pin 1 drags the MCU's own 3.3 V down until
F102 heats up. Load switches were considered for rev B and dropped for lack of space; switching and current sense
go on an optional power adapter instead (change list, 14 Oct 2026). That is a decision, and the check exists so it
is made on purpose.

**How to check.** A connector pin on a rail of the power structure (`NET-5`) that also feeds on-board loads,
where the path from the pin to the rail contains no IC (load switch, eFuse, current-limited switch): a graph query.
Report the polyfuse-only case at a lower severity.

**Fix.** A current-limited load switch or eFuse per output (TPS2553, TPS25947 class), with its fault output read
by the MCU; or a separate regulator for the outputs so a short does not touch the board's own rail.

**References.** SLVA862A §3.2 (overcurrent protection), §3.3 (short-circuit protection), §1 (PTC reaction time);
Wikipedia, Resettable fuse.

## `PIN-4` No overcurrent protection on a power input <a id="pin-4"></a>

⚠️ warning · needs: power inputs, the parts in series with them · [catalogue entry](../checks.md#pin-4)

**Impl.** easy · depends on NET-5

**What it catches.** A power input with no fuse, polyfuse, eFuse or current-limited switch between the connector
and the board.

**Why it matters.** When a part on the board fails short (a ceramic capacitor cracked in assembly, a regulator
whose switch fused), the source delivers whatever it can: a lab supply its limit, a battery or a 24 V industrial
supply tens of amperes. Traces fuse, connectors melt, the battery heats. A fuse turns that into a dead board with a
known cause.

**Example.** A board running from a 2S Li-ion pack through a buck regulator, no fuse: a cracked 1206 input
capacitor shorts the pack through a 0.3 mm trace, which glows and lifts before anything else stops the current.

**How to check.** Walk from each input pin (`NET-5`) to the first load; a part of class fuse (`F` designator,
library class) or an IC the library marks as eFuse or current-limited switch must be on the way. Inputs from a
source that limits by itself (a USB host port, a project setting) may be exempted.

**Fix.** A fuse or polyfuse sized per `PIN-7`, `PIN-10`, `PIN-15`, or an eFuse that also covers inrush and
overvoltage.

**References.** SLVA862A §1 (fuses, PTCs and active protection), §2 (discrete protection circuits); Fuseology
"Selection Factors" 4 and 5 (overload current, maximum available fault current).

## `PIN-5` No overvoltage protection on a power input <a id="pin-5"></a>

⚠️ warning · needs: power inputs, TVS and clamp parts identified from the MPN or library · [catalogue entry](../checks.md#pin-5)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A power input without a TVS, clamp or overvoltage-protection switch near the connector.

**Why it matters.** Inputs see more than their nominal voltage: inductive spikes when a cable is unplugged under
load, hot-plug ringing (`PIN-14`), a wrong adapter, ESD, automotive load dump. The regulator behind the connector
is usually rated with little margin above the highest normal input, and its silicon fails on the first spike over
its absolute maximum.

**Example.** A 12 V vehicle input straight into a 40 V buck: an unsuppressed load dump (ISO 16750-2) rises far
above 40 V for hundreds of milliseconds and destroys it. On B153A and B200A, VBUS has an SMF-series TVS at the
receptacle; the cases where its values are wrong are `PIN-8` and `PIN-9`.

**How to check.** On each input (`NET-5`), look for a part the library classes as TVS, zener clamp or OVP switch
from the input net to ground before the first regulator. A plain diode or zener needs the MPN to tell it from a
TVS, so this needs the part class, not just the `D` designator.

**Fix.** A TVS whose standoff is above the input's maximum and whose clamp is below the protected part's absolute
maximum (`PIN-8`, `PIN-9`); for slow overvoltage (a wrong adapter) an OVP switch or eFuse that disconnects.

**References.** SLVA862A §3.5 (overvoltage protection), §3.9 (surge protection); LM74700-Q1 §10.1.1.3 (selection
of TVS diodes for 12 V battery protection); Wikipedia, Transient-voltage-suppression diode; ISO 7637-2 (road
vehicles, electrical transient conduction along supply lines) and ISO 16750-2 (road vehicles, electrical loads:
load dump), by title.

## `PIN-6` No input filter on a power input <a id="pin-6"></a>

💡 suggestion · needs: power inputs, capacitors and ferrites on them · [catalogue entry](../checks.md#pin-6)

**Impl.** easy · depends on NET-5

**What it catches.** A power input that reaches the first switching regulator with no bulk capacitor, ferrite or
LC filter of its own, only the regulator's local ceramic.

**Why it matters.** A switching regulator draws its input current in pulses at the switching frequency. Without a
filter those pulses run back through the cable, which radiates and fails conducted-emission tests, and they disturb
whatever shares the source. In the other direction the cable's noise reaches the regulator.

**Example.** A 2 MHz buck on a 1 m USB cable with only its 10 µF input ceramic: the input ripple current flows in
the cable, and a conducted-emission scan of the board shows the switching frequency and its harmonics on the
supply lines.

**How to check.** On each input up to the first switching regulator (`NET-5`, switching nodes `NET-6`): is there a
capacitor beyond the regulator's own, or a ferrite or inductor in series? Report as a suggestion; the filter that
is needed depends on the emission limits.

**Fix.** A ferrite or small inductor with a bulk capacitor, damped so the LC does not ring (an electrolytic or a
series R), sized so its output impedance stays below the regulator's negative input impedance.

**References.** SNVA538 (second-order input filter, damping, filter output impedance vs the regulator's input
impedance); AN88 (damping the input with a capacitor that has ESR).

## `PIN-7` Fuse rating vs the expected current <a id="pin-7"></a>

⚠️ warning · needs: fuse rating and curve from the MPN, the worst-case normal load, the fault current · [catalogue entry](../checks.md#pin-7)

**Impl.** hard · depends on NET-8, RPT-2, CFG-2

**What it catches.** A fuse that does not sit between the two currents it has to tell apart: rated so low that the
worst-case normal load (all loads on, hot, low input voltage, inrush) ages or blows it, or so high that the fault
it is there for (a shorted output, a shorted cable) does not open it in time.

**Why it matters.** Fuses are derated: a fuse carries about 75 % of its rating indefinitely at 25 °C and less when
hot. Above that it runs hot and fails early; far above the fault current it never opens and the cable becomes the
fuse. The right value comes from two currents, both of which the designer has to know.

**Example.** On B135B the bring-up list has "fuse: blows when it has to, doesn't blow in the worst-case
connection" as an open check for rev C: the value was chosen without both currents written down. A 1 A fuse
ahead of a board that draws 0.8 A at the minimum input voltage has no margin (1 A × 0.75 = 0.75 A).

**How to check.** Needs the worst-case load per input from the power states (`NET-8`, `RPT-2`) or the project
(`CFG-2`), the fuse rating and derating from the MPN, and the available fault current (source limit, cable
resistance). Report both margins.

**Fix.** Rating ≥ worst-case normal current / (0.75 × temperature rerating), and an opening time at the expected
fault current short enough for the cable and connector (`PIN-11`).

**References.** Fuseology "Selection Factors" 1 (normal operating current, 25 % derating), 3 (ambient
temperature), 4 and 5 (overload and maximum fault current); B135B rev C change list.

## `PIN-8` TVS standoff voltage below the input's operating maximum <a id="pin-8"></a>

🛑 error · needs: TVS standoff and breakdown voltage from the MPN, the input's maximum operating voltage · [catalogue entry](../checks.md#pin-8)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** A TVS on an input whose reverse standoff voltage VR (and in the worst case even its minimum
breakdown voltage VBR) is below the highest voltage the input sees in normal operation.

**Why it matters.** VR is the voltage up to which the datasheet guarantees the diode stays off (below a stated
leakage). Above VR its leakage climbs; above VBR it is in avalanche and conducts continuously, at a dissipation it
is not rated for: it heats, fails short, and the input is shorted to ground.

**Example.** An SMF12A on a 12 V automotive rail that sits at 14.4 V while the alternator charges: VR 12 V and
VBR min 13.3 V are both below 14.4 V, so the TVS conducts whenever the engine runs. Our B200A has the milder case:
D1605 on VBUS has the value SMF5V0A (VR 5.0 V, VBR min 6.4 V, 400 µA leakage at VR) while USB Type-C VBUS may
reach 5.5 V; above 5 V the leakage is no longer guaranteed. (The symbol says SMF22A, a second finding: `BOM`.)

**How to check.** TVS parts on rails (`NET-5`); VR and VBR(min) from the MPN; the rail's maximum from the project
(`CFG-2`) or from its name plus a tolerance per source class (USB 5.5 V, 12 V automotive 14.4-16 V). Error below
VBR(min), warning between VBR(min) and VR.

**Fix.** VR at or above the highest normal input, with margin: SMF15A or SMF16A on 12 V automotive, SMF5.0A only
where the rail cannot exceed 5 V, a 6 V part on USB VBUS. Then check the clamp (`PIN-9`).

**References.** SMF datasheet, Electrical Characteristics table (VBR, VR, IR, VC); LM74700-Q1 §10.1.1.3
(breakdown above the highest system voltage, clamp below the protected part's rating); Wikipedia,
Transient-voltage-suppression diode; Wikipedia, USB hardware "Voltage tolerance and limits".

## `PIN-9` TVS clamping voltage above the absolute maximum of what it protects <a id="pin-9"></a>

⚠️ warning · needs: TVS clamping voltage from the MPN, absolute maximum of the parts on the protected rail · [catalogue entry](../checks.md#pin-9)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A TVS whose clamping voltage VC at its rated pulse current is above the absolute maximum
input voltage of the regulator or IC it sits in front of.

**Why it matters.** VC is far above VR: an SMF22A has VR 22 V but clamps at 35.5 V at its 5.6 A rated pulse. The
TVS only limits the spike to VC; if the part behind it is rated for less, the part breaks down first and the TVS
protects nothing.

**Example.** B200A's VBUS TVS D1605 carries the symbol SMF22A while its value says SMF5V0A. If the SMF22A is what
gets fitted, it clamps at up to 35.5 V in front of the RT5760 buck, which the review notes rate at 6.5 V absolute
maximum. The SMF5.0A (VC 9.2 V) is the right family member for a 5 V-only input, and still above 6.5 V at full
pulse current: the review note "fine for 5 V only" holds only for pulses well below the rated Ipp.

**How to check.** For each TVS on a rail (`NET-5`): VC from the MPN; the lowest absolute maximum of the input pins
on the same rail up to the next series element (`NET-7`). Flag VC above that.

**Fix.** A TVS with a lower VC (often the lower VR is impossible because of `PIN-8`, which leaves a
higher-power TVS with a flatter clamp), a series element that limits the pulse current (ferrite, resistor, PTC), a
regulator with a higher input rating, or an active OVP switch.

**References.** SMF datasheet, Electrical Characteristics table (VC at Ipp, 10/1000 µs); LM74700-Q1 §10.1.1.3;
SLVA862A §3.5 (output voltage clamp during transient overvoltage).

## `PIN-10` Polyfuse hold current below the load, or its resistance in an accurate supply <a id="pin-10"></a>

⚠️ warning · needs: polyfuse hold current and resistance from the MPN, the load current, the rail's tolerance · [catalogue entry](../checks.md#pin-10)

**Impl.** mid · depends on CFG-2, RPT-2

**What it catches.** A polymeric PTC whose hold current is below the current the load draws in normal operation,
or whose resistance (which rises after each trip) sits in series with a rail that has to be accurate.

**Why it matters.** The hold current is the highest current at which the PTC is guaranteed not to trip, at 25 °C;
it falls with temperature. Above it the PTC trips during normal use and the load browns out. Its resistance is
not small (ohms for 100 mA parts) and after a trip it settles higher than new, so a PTC in front of a sensor
supply or an ADC reference adds a load-dependent error.

**Example.** B200A J102 pin 1 `+3V3_OUT` goes through F102, a 100 mA hold, 300 mA trip polyfuse. An adapter card
drawing 120 mA works at the bench and trips after a few minutes in a warm enclosure; one drawing 80 mA sees the
3.3 V output sag by the PTC's resistance times its current.

**How to check.** The load per output from the project or a consumption report (`CFG-2`, `RPT-2`) against the
hold current, rerated for the ambient temperature; for rails the project marks as accurate, flag any PTC in series.

**Fix.** A PTC with a hold current above the worst-case load at the hottest ambient, or an eFuse whose current
limit is accurate and whose on-resistance is milliohms.

**References.** Fuseology "PTC Characteristics and Terms" (hold and trip current, rerating with temperature);
Wikipedia, Resettable fuse (hold current, resistance after a trip); SLVA862A §1 (PTC resistance after resets).

## `PIN-11` Fuse or polyfuse rated above the connector, cable or trace it protects <a id="pin-11"></a>

⚠️ warning · needs: fuse rating, connector contact rating, cable and trace current capacity · [catalogue entry](../checks.md#pin-11)

**Impl.** mid · depends on CFG-2

**What it catches.** A fuse whose rating is above the current the connector contacts, the cable or the trace
behind it can carry.

**Why it matters.** A fuse protects the wiring, not the load. If it is rated above the weakest conductor in its
path, an overload just below the fuse's opening current heats that conductor instead: a header's contacts
discolour and loosen, a thin trace delaminates, a cable's insulation softens, and the fuse never notices.

**Example.** A 3 A fuse feeding an output on a 2.54 mm pin header whose contacts the datasheet rates at 1 A: a 2 A
overload on that output runs for hours, well under the fuse's opening current and twice the contact rating.

**How to check.** Fuse rating from the MPN against the contact rating of connectors on the same path (library
data) and cable or trace limits from the project (`CFG-2`); the trace part needs layout data and stays a report
item until then.

**Fix.** A fuse rated at or below the weakest conductor, or a heavier connector and cable. Size traces with
IPC-2152 (by title: *Standard for Determining Current Carrying Capacity in Printed Board Design*).

**References.** Fuseology "Selection Factors" 1 and 4; Wikipedia, Fuse (electrical); IPC-2152.

## `PIN-12` Two power sources on one net without ORing <a id="pin-12"></a>

🛑 error · needs: power sources (connectors, batteries, regulator outputs) and what joins them · [catalogue entry](../checks.md#pin-12)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A net fed from two sources (battery and USB, two connectors, two regulator outputs) that are
joined directly, without ORing diodes, an ideal-diode controller, a power multiplexer or a switch.

**Why it matters.** Two sources tied together are never at exactly the same voltage: the higher one drives
current into the lower. USB VBUS pushed into a battery charges it with no control; one supply pushed into
another's output drives it backwards; a host's VBUS pushed into a second host's port is a USB compliance failure
and can damage either.

**Example.** B153A runs from either of two VBUS (receptacle and plug) and does it right: `+VIN_SUM` is `+V1`
through D801 or `+V2` through D802, both 1N5819W Schottkys, so neither port ever feeds the other. Drop the diodes
and the computer's VBUS is wired straight to the device under test's VBUS.

**How to check.** Find the sources on each rail (`NET-5`: connector inputs, battery terminals, regulator outputs)
and follow from each to the shared net; the path must contain a diode in the forward direction, a part the
library classes as ideal diode, power mux or load switch (`NET-7`), or end at a controlled switch.

**Fix.** Schottky diodes for small currents (as on B153A), an ideal-diode controller (LM74700-Q1, LM5050) or a
power-mux IC where the drop matters, a charger with power path for battery plus external supply (`BAT-3`).

**References.** SLVAE57 §3 (ORing power supplies), §8 (ORing with ideal-diode controllers); LM74700-Q1 §10.2
(OR-ing application configuration); `RAIL-18` for two regulator outputs.

## `PIN-13` Reverse protection by a series diode on an input over about 1 A <a id="pin-13"></a>

💡 suggestion · needs: the series diode on the input, the input current · [catalogue entry](../checks.md#pin-13)

**Impl.** mid · depends on CFG-2, RPT-2

**What it catches.** A series diode used as reverse polarity (or ORing) protection on an input that carries more
than about an ampere.

**Why it matters.** The diode drops its forward voltage at the full load current: 0.4-0.7 V at a few amperes is
watts of heat in a small package and that much headroom taken from the regulator behind it, which on a low input
(a cranking battery, 5 V USB) can be the difference between regulating and not.

**Example.** TI's comparison: a 2 A load through a series diode dissipates about 1.4 W. The same 2 A through a
PMOS of 20 mΩ drops 40 mV and dissipates 80 mW. B153A's diode-OR (D801/D802, 1N5819W) is fine as it is: it only
carries the input current of the 3.3 V buck, a few hundred milliamperes at most.

**How to check.** A series diode on an input (`PIN-1`), its forward voltage from the MPN, the input current from
the project or a consumption report (`CFG-2`, `RPT-2`); suggest above 1 A or a project limit on dissipation.

**Fix.** A PMOS with its gate pulled to ground through a resistor and clamped by a zener (`PIN-17`), or an
ideal-diode controller with an NMOS (LM74700-Q1).

**References.** SLVA862A §2 (2 A gives about 1.4 W in the diode); SLVAE57 §2 (drawbacks of Schottky diodes), §4.1
(PMOS); SLVA139 Table 1.

## `PIN-14` No bulk capacitance where a supply arrives through a long cable or is hot-plugged <a id="pin-14"></a>

⚠️ warning · needs: inputs marked as cable-fed or hot-plugged, capacitance on them · [catalogue entry](../checks.md#pin-14)

**Impl.** mid · depends on CFG-2, RPT-1

**What it catches.** An input fed through a long cable or plugged in live that has only low-ESR ceramic
capacitors on it, no bulk capacitor with some series resistance.

**Why it matters.** The cable's inductance and a ceramic capacitor form an almost undamped LC circuit. When the
plug makes contact the voltage overshoots: up to twice the source voltage. A 24 V supply then puts close to 48 V
on a regulator rated for 40 V. Nothing in the steady state shows it; the first hot plug does.

**Example.** A board fed with 24 V over 2 m of cable with 2 × 4.7 µF ceramics at a 40 V buck: the plug-in spike
can approach 48 V. An aluminium electrolytic of 47 µF across the input damps it to a clean rise.

**How to check.** Which inputs are cable-fed or hot-plugged is a project fact (`CFG-2`, or connector class from the
library); sum the capacitance on the input and check that at least one part has ESR in the damping range
(electrolytic, polymer, or a ceramic with a series resistor) (`RPT-1`).

**Fix.** A bulk electrolytic or polymer capacitor at the input, an RC snubber, a TVS (`PIN-5`), or an inrush
limiter that ramps the input.

**References.** AN88 (spikes twice the input voltage, damping with a high-ESR capacitor, comparison of snubbers);
SLVA862A §3.9 (surge protection).

## `PIN-15` Fuse voltage rating below the supply voltage <a id="pin-15"></a>

⚠️ warning · needs: fuse voltage rating (and AC or DC) from the MPN, the rail voltage · [catalogue entry](../checks.md#pin-15)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** A fuse whose voltage rating (for DC, the DC rating) is below the voltage of the rail it is
on.

**Why it matters.** The voltage rating is the voltage at which the fuse is guaranteed to clear the fault: when the
element melts, the arc across the gap must go out. At a higher voltage, and in DC where there is no zero crossing,
the arc can keep burning, the fuse body ruptures and the circuit is not interrupted. The fuse "works" in every
test until the fault it is there for.

**Example.** A 32 V DC automotive blade fuse on a 48 V telecom input: it carries the load fine and fails to break
a short.

**How to check.** Voltage rating and AC/DC from the MPN; the rail voltage from the net name or the project
(`NET-5`, `CFG-2`), with its maximum, not its nominal.

**Fix.** A fuse rated at or above the maximum supply voltage, with a DC rating for DC rails.

**References.** Fuseology "Selection Factors" 2 (application voltage: the fuse rating must be equal to or greater
than the available circuit voltage) and "Voltage Rating"; Wikipedia, Fuse (electrical).

## `PIN-16` No undervoltage lockout on a power input whose source can sag <a id="pin-16"></a>

⚠️ warning · needs: inputs from a source that can sag (battery, long cable), the UVLO of the regulators on them · [catalogue entry](../checks.md#pin-16)

**Impl.** mid · depends on CFG-2, NET-7

**What it catches.** An input from a battery or through a long cable where no UVLO (the regulator's own, an
EN-pin divider, a supervisor, an eFuse) stops the board at a defined voltage with hysteresis.

**Why it matters.** As the source sags, the regulator runs out of headroom and its output follows the input down;
the MCU sees a slow brown-out and may write corrupt flash. With a high-impedance source the regulator turning on
pulls the input below its threshold, turns off, lets it recover and turns on again: a restart loop that looks like
a firmware crash. Hysteresis bigger than the drop at turn-on is what stops it.

**Example.** A board on 3 m of thin cable from a 12 V supply: at turn-on the load current drops 1.5 V across the
cable, the buck's UVLO hysteresis is only 0.3 V, and the board chatters on and off until the bulk capacitors happen
to carry it through. B153A's AP64060 has its EN tied to VIN, so only its internal UVLO (about 3.8 V) defines when it
starts: enough for USB, a decision to revisit if the board ever ran from a cable or a battery.

**How to check.** Which sources can sag is a project setting (battery, cable length: `CFG-2`); the regulator's
UVLO threshold and hysteresis come from part data (`NET-7`), an EN divider from the netlist. Flag inputs where the
threshold is below the lowest safe input or the hysteresis below the expected drop.

**Fix.** An EN-pin divider (with hysteresis resistor) to set the start and stop voltages, a supervisor driving EN,
or an eFuse with adjustable UVLO.

**References.** SLVA769A §1 (the need for UVLO, battery applications), §3 (hysteresis and UVLO "bouncing", long
cables); `BAT-6` for the battery-drain side.

## `PIN-17` PMOS reverse-polarity switch beyond Vgs(max), or not fully on at low input <a id="pin-17"></a>

🛑 error · needs: the PMOS gate network, Vgs(max), threshold and Rds(on) conditions from the MPN, the input voltage · [catalogue entry](../checks.md#pin-17)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A PMOS reverse-polarity switch whose gate is pulled to ground on an input above the FET's
maximum gate-source voltage, with no zener from gate to source and no divider; or, on an input of 5 V or less, a
standard-threshold FET that the available gate voltage never turns fully on. ≈ `DRV-10`.

**Why it matters.** With the gate tied to ground, Vgs equals the input voltage. Above Vgs(max) the gate oxide
breaks down, usually the first time the input spikes, and the FET fails short (or open). At low input the opposite
fails: a FET specified at Vgs = −10 V carries the load on its body diode or in its linear region, drops a volt and
overheats.

**Example.** An AO3401A (VDS −30 V, VGS ±12 V) as reverse protection on a 24 V input with its gate to ground through
10 kΩ sees −24 V on the gate. On a 3.3 V input, a PMOS whose Rds(on) is given only at −10 V and whose threshold
can be −4 V never fully conducts; the same AO3401A is specified down to −2.5 V and is the right choice there.

**How to check.** For a PMOS in an input path (`NET-5`) with its drain towards the connector: follow the gate
network; compute the worst-case Vgs from the input maximum and any divider or zener; compare with Vgs(max) from
the MPN (`NET-7`). At low inputs compare the minimum input with the Vgs at which Rds(on) is specified.

**Fix.** A zener from gate to source (10-12 V for a ±20 V FET, lower for ±12 V) with a gate resistor, or a
gate divider; at low inputs a logic-level FET with Rds(on) specified at 2.5 V or less, or an ideal-diode
controller with a charge pump.

**References.** SLVA862A §3.8 (P-MOSFET with Zener diode and resistor, Figure 14(b)); SLVAE57 §4.1; AO3401A
datasheet, Alpha & Omega (absolute maximum ratings, RDS(ON) conditions)
[aosmd.com](https://www.aosmd.com/res/datasheets/AO3401A.pdf).
