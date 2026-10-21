# DEC checks, explained

Detail page for the `DEC` category of the [check catalogue](../checks.md#-dec---decoupling-and-datasheet-passives).
Each section explains one check: what it catches, why the board fails without it, a concrete example, what
better_erc needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the
source of truth for ID, severity and wording; this page is the commentary.

References used throughout:

- **MT-101** · Analog Devices, *Decoupling Techniques*, tutorial MT-101 (Rev. 0, March 2009). Capacitor parasitics,
  capacitor types, local decoupling, LC resonance with ferrites (fetched through `tpm fetch`, analog.com refuses plain
  clients). [analog.com MT-101.pdf](https://www.analog.com/media/en/training-seminars/tutorials/MT-101.pdf)
- **Ferrite Beads Demystified** · J. Eco, A. Limjoco, Analog Devices, *Analog Dialogue* 50-02 (February 2016): bead
  model, DC bias current, LC resonance and damping (fetched through `tpm fetch`).
  [analog.com ferrite-beads-demystified.pdf](https://www.analog.com/media/en/analog-dialogue/volume-50/number-1/articles/ferrite-beads-demystified.pdf)
- **AN4488** · ST, *Getting started with STM32F4xxxx MCU hardware development*, Rev 7 (October 2018); §2.2 power
  supply schemes (VCAP), §8.3 power supply decoupling, §9.3 "MCU does not work properly" (fetched through
  `tpm fetch`).
  [st.com AN4488](https://www.st.com/resource/en/application_note/an4488-getting-started-with-stm32f4xxxx-mcu-hardware-development-stmicroelectronics.pdf)
- **AN2586** · ST, *Getting started with STM32F10xxx hardware development*, Rev 8 (December 2022); §6.4 decoupling
  (fetched through `tpm fetch`).
  [st.com AN2586](https://www.st.com/resource/en/application_note/an2586-getting-started-with-stm32f10xxx-hardware-development-stmicroelectronics.pdf)
- **DS00001692C** · Microchip, *USB251xB/xBi USB 2.0 Hi-Speed Hub Controller* datasheet, rev C (2015); Table 3-2 pin
  descriptions (CRFILT, PLLFILT, RBIAS).
  [ww1.microchip.com 00001692C.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf)
- **Wikipedia** · [Decoupling capacitor](https://en.wikipedia.org/wiki/Decoupling_capacitor),
  [Ferrite bead](https://en.wikipedia.org/wiki/Ferrite_bead), [Tantalum capacitor](https://en.wikipedia.org/wiki/Tantalum_capacitor)
  (reverse voltage), [Electrolytic capacitor](https://en.wikipedia.org/wiki/Electrolytic_capacitor).
- **Ott** · H. W. Ott, *Electromagnetic Compatibility Engineering* (Wiley, 2009), chapter 11 (digital circuit power
  distribution: decoupling, multiple capacitors, anti-resonance).
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 9 (voltage regulation and
  power conversion).

## `DEC-1` No decoupling capacitors on power nets <a id="dec-1"></a>

⚠️ warning · needs: power rails, capacitors to ground · [catalogue entry](../checks.md#dec-1)

**Impl.** easy · depends on NET-5

**What it catches.** A power rail that feeds IC power pins and has no capacitor from it to a ground anywhere on the
board.

**Why it matters.** A digital IC draws its current in spikes at every clock edge; the trace and plane inductance
between it and the regulator cannot deliver them, so the local supply dips and rings. Without a capacitor at the
pins the part sees that noise as logic errors, PLL jitter, ADC noise or brown-out resets, and the rail radiates.
A rail with *no* capacitor at all is nearly always an omission: a sheet copied without its passives, or caps placed on
the wrong net name.

**Example.** A sub-sheet with an SN74LVC595 shift register on `+3V3_IO`, its 100 nF drawn on `+3V3` from the copy
source: the net `+3V3_IO` (fed by a 0 Ω from `+3V3`) has four power pins and no capacitor.

**How to check.** For each rail of the power structure (`NET-5`) with at least one power-input pin, look for a
capacitor whose two pins sit on that rail and on a ground. Rails behind a 0 Ω or ferrite count separately
(`DEC-8`).

**Fix.** At least one 100 nF ceramic per IC power pin close to the pin, plus bulk on the rail (`DEC-6`).

**References.** MT-101 "What is proper decoupling and why is it necessary?", "Localized high frequency decoupling
recommendations"; AN4488 §8.3; Wikipedia, Decoupling capacitor.

## `DEC-2` Recommended decoupling values <a id="dec-2"></a>

💡 suggestion · needs: decoupling rule per part (library), capacitors per pin · [catalogue entry](../checks.md#dec-2)

**Impl.** mid · depends on NET-7, RPT-1

**What it catches.** Decoupling that differs from what the part's datasheet or hardware guide recommends: fewer or
smaller capacitors, a missing bulk capacitor, or a value outside the stated range.

**Why it matters.** Vendors size decoupling for the part's current profile and internal regulators; boards that
follow a generic "100 nF per pin" rule miss the bulk capacitor the part needs for load steps, or the specific value
an internal regulator was characterised with. The result is marginal behaviour that shows up only at temperature
or with a busy firmware.

**Example.** An STM32F4 board with 100 nF on each VDD pin but no 4.7 µF on the rail: AN4488 §8.3 asks for 100 nF per
supply pair *and* one tantalum or ceramic of at least 4.7 µF (typically 10 µF).

**How to check.** Needs a per-part rule (pin group → capacitors and values) in the library; sum the capacitors on
each pin group's net (`RPT-1`) and compare.

**Fix.** Match the datasheet; where it is silent, 100 nF per pin plus 1-10 µF per rail is the usual default.

**References.** AN4488 §8.3; AN2586 §6.4; MT-101 "Types of decoupling capacitors".

## `DEC-3` STM32 VCAP pins need 2.2 µF ceramic caps <a id="dec-3"></a>

🛑 error · needs: pin names (VCAP), MCU family, capacitor values · [catalogue entry](../checks.md#dec-3)

**Impl.** mid · depends on NET-7

**What it catches.** An STM32 VCAP pin (VCAP, VCAP1, VCAP2) without its ceramic capacitor to ground, with a value
other than the family asks for, or wired to VDD.

**Why it matters.** VCAP is the output of the MCU's internal core regulator. The regulator is stable only with the
specified low-ESR capacitor; without it the core supply oscillates or collapses, and the MCU does not start, resets
at random or cannot be programmed. Treating VCAP as "another VDD" and putting 100 nF on it is the classic mistake.

**Example.** STM32F407 in LQFP100 with VCAP1 and VCAP2 each on 100 nF: the core regulator is unstable. AN4488 asks
for 2 × 2.2 µF, ESR below 2 Ω (or one 4.7 µF below 1 Ω where only VCAP1 exists); with BYPASS_REG tied high the VCAP
pins instead take the external core supply with 100 nF each.

**How to check.** Pins named VCAP* on a part recognised as an STM32 (MPN prefix); the net must carry exactly the
capacitor the family asks for (2.2 µF per pin, 4.7 µF for a single VCAP on F4) to ground and nothing else. The
value table per family lives in the library.

**Fix.** The datasheet value, X5R/X7R ceramic, next to the pin.

**References.** AN4488 §2.2 (VCAP with the regulator on), §9.3 (VCAPs in the troubleshooting list).

## `DEC-4` Caution against ferrite beads without measurements <a id="dec-4"></a>

💡 suggestion · needs: ferrites in power paths · [catalogue entry](../checks.md#dec-4)

**Impl.** easy · depends on NET-5

**What it catches.** A ferrite bead in series with a supply (between two rails, or between a rail and a part's power
pin).

**Why it matters.** A bead is an inductor at low frequency and a resistor only in a band; with the decoupling
capacitor behind it it forms an LC low-pass with a resonant peak, often in the hundreds of kHz to a few MHz, where it
*amplifies* noise instead of filtering it. Its impedance also drops with DC bias current (`DEC-9`). A bead added "for
cleanliness" without looking at the impedance curve at the operating current or measuring the result can make the
rail worse.

**Example.** An 0603 bead feeding an ADC's AVDD with 10 µF behind it: a low-frequency inductance of about 1 µH with
10 µF resonates near 50 kHz, in the band of a switching regulator's ripple and its harmonics, and the AVDD ripple at
that frequency is higher than without the bead.

**How to check.** Ferrites (designator `FB`/`L` with a bead value or library class) in series on a rail; report
each as a suggestion with the LC resonance computed from the bead inductance and the capacitance behind it.

**Fix.** Check the bead's impedance at the rail current, damp the resonance (a series resistor or a lossy capacitor,
as in the ADI article), or use a 0 Ω or an RC filter; measure the ripple either way.

**References.** Ferrite Beads Demystified ("DC bias current considerations", LC resonance and damping); MT-101
"Resonant circuits formed by LC decoupling networks"; Wikipedia, Ferrite bead.

## `DEC-5` Datasheet-specified filter or bypass capacitors missing or off value <a id="dec-5"></a>

🛑 error · needs: pin roles (PLL, CRFILT, VCAP...), datasheet values · [catalogue entry](../checks.md#dec-5)

**Impl.** mid · depends on NET-7

**What it catches.** A pin the datasheet says must carry a filter or bypass capacitor (an internal regulator output,
a PLL filter, a core supply, an analog filter) with no capacitor, or with a value other than the datasheet's.

**Why it matters.** These pins are outputs of on-chip regulators or nodes of on-chip filters; the part was
characterised with a specific capacitor and may not regulate, lock its PLL or meet its jitter without it. The
failure ranges from "does not start" to "USB fails eye tests". The netlist shows a pin with nothing on it, which an
ERC treats as a valid unconnected pin.

**Example.** On B153A the USB2514B hub's CRFILT (C306) and PLLFILT (C310) carry 1 µF in the Main variant and are DNP
in Plus, as on the Microchip EVK; 100 nF C304/C308 sit on the supplies. The datasheet DS00001692C (Table 3-2) says
each pin "can have up to a 0.1 µF low-ESR capacitor to VSS, or be left unconnected", while the review checklist used
for B153A asks 1 µF on both. Three sources, three answers: the check makes the board follow one documented value per
variant and names it.

**How to check.** Pin roles and required capacitors come from the library (`NET-7`): for each such pin, the capacitor
on its net to ground and its value, compared with the library entry and the variant's DNP flags.

**Fix.** Fit what the datasheet specifies for the exact MPN; where vendor documents disagree, pick one, note it on the
schematic and keep it the same in every variant.

**References.** DS00001692C Table 3-2 (CRFILT, PLLFILT); AN4488 §2.2 (VCAP); B153 rev B change list (C306/C310).

## `DEC-6` Bulk capacitance on a rail below what its regulator or load needs <a id="dec-6"></a>

⚠️ warning · needs: capacitance per rail, derating, regulator data · [catalogue entry](../checks.md#dec-6)

**Impl.** mid · depends on RPT-1, NET-5, CFG-2

**What it catches.** A rail whose total capacitance, derated for DC bias at the rail voltage, is below the minimum the
regulator needs at its output or the load needs for its current steps.

**Why it matters.** Regulators are stable and meet their transient spec only with a minimum output capacitance; loads
that switch large currents (radios, motors, SoCs leaving sleep) need charge nearby so the rail stays above their
brown-out level until the regulator responds. Ceramics lose much of their capacitance under DC bias, so "22 µF" on the
schematic can be 8 µF on the board.

**Example.** A Wi-Fi module on a 3.3 V buck with 2 × 10 µF 6.3 V 0603 X5R: well under 10 µF effective at 3.3 V. Its
transmit bursts draw several hundred mA within microseconds, faster than the buck's loop answers, and the rail sags
below the module's brown-out threshold at every burst.

**How to check.** Sum capacitance per rail (`RPT-1`) with a DC-bias derating per package and dielectric, compare with
the regulator's minimum (library) and a load requirement from the project (`CFG-2`).

**Fix.** More or larger-case capacitors, a polymer or tantalum bulk capacitor; check the regulator's maximum too
(`RAIL-21`).

**References.** AN4488 §8.3 (bulk 4.7-10 µF per rail); MT-101 "Types of decoupling capacitors"; AoE3 chapter 9.

## `DEC-7` Fewer decoupling capacitors than power pins <a id="dec-7"></a>

⚠️ warning · needs: power pins per part and rail, capacitors on that rail · [catalogue entry](../checks.md#dec-7)

**Impl.** easy · depends on NET-5

**What it catches.** A part with N power-input pins on one rail and fewer than N small capacitors on that rail, when
the part's datasheet (or the default rule) wants one per pin.

**Why it matters.** A shared capacitor serves one pin well and the others through trace inductance; high-pin-count
parts (MCUs, FPGAs, hubs) have their supply pins around the package so each edge of the die needs its own local
charge. Too few capacitors is the most common sign of a symbol whose power pins were drawn stacked and decoupled as
one.

**Example.** An STM32F4 in LQFP100 with its VDD/VSS pairs on all four sides of the package, and four 100 nF on `+3V3`
shared with a flash chip and a CAN transceiver: fewer capacitors than the MCU's supply pairs alone. AN4488 §8.3 asks
one 100 nF per supply pair.

**How to check.** Count power-input pins per part per rail and the capacitors of ≤ 1 µF on that rail; capacitors
already counted for another part on the same rail reduce the pool. Report the deficit.

**Fix.** One capacitor per power pin, placed at the pin.

**References.** AN4488 §8.3; AN2586 §6.4; MT-101 "Localized high frequency decoupling recommendations".

## `DEC-8` Decoupling only on the source side of a ferrite, filter or series resistor <a id="dec-8"></a>

⚠️ warning · needs: rails split by series parts, capacitors per sub-net · [catalogue entry](../checks.md#dec-8)

**Impl.** easy · depends on NET-5

**What it catches.** A power pin on a net reached through a ferrite, filter inductor or series resistor, where all
the capacitors sit on the source side and none on the pin's side.

**Why it matters.** The series part is there to stop current spikes from crossing; with the capacitors on the wrong
side the pin's spikes now go *through* the filter impedance, so the pin sees more noise than with no filter at all.
It happens when a filter is inserted into an existing net and the capacitors keep the old net name.

**Example.** B200A does it right: `+3V3_VDDA` is `+3V3_DCDC` through R1802 0 Ω, with 4.7 µF and 100 nF on the VDDA
side. Rename the capacitors' net back to `+3V3_DCDC` and VDDA has nothing local: that is the case the check flags.

**How to check.** For each two-pin series part between two rail nets (`NET-5`), the net that holds power-input pins
must also hold at least one capacitor to ground.

**Fix.** Put the capacitors on the load side, at the pin; keep a capacitor on the source side only if the filter
design needs it.

**References.** MT-101 "Localized high frequency decoupling recommendations" (correct vs incorrect ferrite placement
figure); Ferrite Beads Demystified.

## `DEC-9` Ferrite bead in series with a high-current rail <a id="dec-9"></a>

⚠️ warning · needs: bead rated current and DCR (library), rail current (project) · [catalogue entry](../checks.md#dec-9)

**Impl.** mid · depends on CFG-2, RPT-2

**What it catches.** A ferrite bead on a rail whose current is near or above the bead's rating, or whose DC
resistance drops the rail by more than the load tolerates.

**Why it matters.** Bead impedance falls sharply with DC current as the ferrite saturates: a 600 Ω bead at 0 A can be
under 100 Ω at half its rated current, so the filter does nothing exactly when the load is busy. Its DCR (0.1-0.5 Ω
for small beads) also drops the rail: 0.3 Ω at 1 A is 0.3 V, a tenth of a 3.3 V rail.

**Example.** An 0603 bead labelled 600 Ω at 100 MHz, rated 1.3 A with 0.15 Ω DCR, feeding a 1 A camera module: at 1 A
its impedance is a fraction of the label value, and the module sees 150 mV less.

**How to check.** Bead rated current and DCR from the library; rail current from the project or a power-state report
(`CFG-2`, `RPT-2`); flag above ~50 % of rating or a drop beyond the load's tolerance.

**Fix.** A bead with a much higher current rating, an LC filter with a power inductor, or no filter on that rail.

**References.** Ferrite Beads Demystified ("DC bias current considerations"); Wikipedia, Ferrite bead.

## `DEC-10` Bypass capacitor on the IC's power pin but not on the rail <a id="dec-10"></a>

🛑 error · needs: nets with only passives and power pins · [catalogue entry](../checks.md#dec-10)

**Impl.** easy · depends on netlist

**What it catches.** A net that holds only power-input pins and capacitors (often one pin and one capacitor), with no
regulator output, connector, switch or series part bringing power in.

**Why it matters.** The schematic looks complete at the IC: a supply pin with its 100 nF to ground. But the net name
was mistyped or the label never joins the rail, so the IC is unpowered. A normal ERC is satisfied, because a power
input "driven" by a capacitor's passive pin passes the pin-type matrix in most tools.

**Example.** B200A: `+5V0_FLT`, the V+ of the two TLV9004 op-amps U201/U202 with C201/C202, had no source in the
schematic or on the PCB; the tracks joined the four pads to each other only. All eight analog channels read garbage
and the op-amp inputs were back-powered through the IO pins' ESD diodes until rework 1 added the feed from
`+5V0_USBIN`. (`RAIL-5` catches the same board from the rail side.)

**How to check.** Every net whose members are power-input pins and capacitor pins only (the capacitors' other pins on
ground) is a finding.

**Fix.** Connect the net to its rail (label, ferrite or 0 Ω as intended).

**References.** B200 rev B change list (`+5V0_FLT` has no source); MT-101 for what the capacitor is for.

## `DEC-11` Parallel ceramics a decade apart <a id="dec-11"></a>

💡 suggestion · needs: capacitor values per rail · [catalogue entry](../checks.md#dec-11)

**Impl.** easy · depends on IMP-6, NET-5

**What it catches.** Capacitors in parallel on one rail whose values are a decade or more apart (10 µF with 10 nF,
100 nF with 1 nF), without values in between.

**Why it matters.** Each capacitor is a series RLC; between the self-resonances of a large and a small one, the large
one is already inductive and the small one still capacitive, and the two form a parallel resonance with an impedance
*peak* higher than either part alone. Modern MLCCs in the same package have nearly the same ESL, so a 10 nF next to a
100 nF adds little at high frequency and creates a peak; the "decade ladder" comes from through-hole days.

**Example.** `+1V2_CORE` on an FPGA with 10 µF, 100 nF and 1 nF, all 0402: anti-resonance peaks around tens of MHz
where the core current has energy; three 100 nF and one 10 µF give a flatter impedance.

**How to check.** Parse the values (`IMP-6`), sort per rail, flag gaps of ≥ 10× between neighbours; a suggestion only.

**Fix.** Several of one value, or a graded set (10 µF, 2.2 µF, 470 nF, 100 nF); for fast rails, a PDN impedance
target and a simulation.

**References.** Ott chapter 11 (multiple decoupling capacitors and anti-resonance); MT-101 "Real capacitors and their
parasitics", "Types of decoupling capacitors".

## `DEC-12` Polarized capacitor with its negative terminal on the higher-potential net <a id="dec-12"></a>

🛑 error · needs: capacitor polarity (pin names), net voltages · [catalogue entry](../checks.md#dec-12)

**Impl.** easy · depends on NET-5

**What it catches.** An electrolytic or tantalum capacitor whose `+` pin sits on the lower of its two nets: on ground
with `−` on a positive rail, or on a negative rail with `−` on ground.

**Why it matters.** Under reverse voltage the oxide dielectric of a polarized capacitor is reduced; leakage rises,
the part heats, and a tantalum capacitor can fail short and burn, an aluminium one vent. A reversed symbol or a
negative rail (−12 V, where `+` belongs on ground) is enough to do it, and nothing in an ERC notices.

**Example.** A 47 µF tantalum on `-5V` drawn with its `+` pin on `-5V` "because it is the supply": reverse-biased by
5 V, it fails short at power-on.

**How to check.** Capacitors with polarized symbols (pins named `+`/`−` or 1/2 with a polarized library class); compare
the net voltages from the power structure (`NET-5`): `+` must be the higher one.

**Fix.** Flip the capacitor; on negative rails the `+` goes to ground.

**References.** Wikipedia, Tantalum capacitor "Reverse voltage"; Wikipedia, Electrolytic capacitor.

## `DEC-13` Datasheet-specified bias or set resistors missing, off value, or too loose <a id="dec-13"></a>

⚠️ warning · needs: pin roles (RBIAS, RZQ...), datasheet value and tolerance · [catalogue entry](../checks.md#dec-13)

**Impl.** mid · depends on NET-7

**What it catches.** A pin that must carry a precision resistor (to ground or to a rail) with no resistor, a different
value, or a looser tolerance than the datasheet asks.

**Why it matters.** These resistors set internal currents: a USB PHY's transceiver bias, an Ethernet PHY's output
amplitude, a DDR ZQ calibration (240 Ω 1 %), a charger's current. Off value means out-of-spec eye diagrams, wrong
termination after calibration, or the wrong charge current, and the board works on the bench and fails compliance.
A 5 % jellybean of the right value passes a value check and still misses the spec.

**Example.** The USB2514B on B153A has RBIAS 12K; DS00001692C Table 3-2 asks "12.0 kΩ (±1 %)" from RBIAS to ground. The
check reads the tolerance from the BOM line: a 12K 5 % from the generic library would be flagged.

**How to check.** Pin roles and the required value and tolerance from the library (`NET-7`); compare with the
resistor on the pin's net (`IMP-6` for the value, the BOM field for the tolerance).

**Fix.** The exact value and tolerance (1 % or better), placed next to the pin with nothing else on the net.

**References.** DS00001692C Table 3-2 (RBIAS 12.0 kΩ ±1 %).
