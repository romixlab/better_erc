# BOM checks, explained

Detail page for the `BOM` category of the [check catalogue](../checks.md#-bom---bom-hygiene). Each section explains
one check: what it catches, why it costs money, time or a working board, a concrete example, what better_erc needs to
find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for
ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is for better_erc in pure Rust: `easy` (a query on the netlist and the
values we already parse), `mid` (part data such as an MPN decoder, a part database or variant data first), `hard`
(analysis the code does not have yet), `judgement` (intent, so better_erc can at most list candidates).

References used throughout:

- **E series** · Wikipedia, *E series of preferred numbers* (E6 to E192, how the steps are built); the standard is
  IEC 60063, *Preferred number series for resistors and capacitors*, by title.
  [en.wikipedia.org/wiki/E_series_of_preferred_numbers](https://en.wikipedia.org/wiki/E_series_of_preferred_numbers)
- **Yageo RC_L** · Yageo, *General purpose chip resistors RC_L series*, product specification V.11 (May 2020),
  "Ordering information - global part number" (an MPN that encodes size, tolerance and value; fetched through `tpm
  fetch`, Mouser refuses plain HTTP clients).
  [mouser.com PYu_RC_Group_51_RoHS_L_11](https://www.mouser.com/datasheet/2/447/PYu_RC_Group_51_RoHS_L_11-1984063.pdf)
- **EPCI DC bias** · T. Zednicek, EPCI, *High CV MLCC DC/AC bias ageing capacitance loss explained* (2019).
  [epci.eu](https://epci.eu/wp-content/uploads/2019/10/HIGH-CV-MLCC-DCAC-BIAS-AGEING-CAPACITANCE-LOSS-EXPLAINED.pdf)
- **KiCad variants** · KiCad, *Schematic Editor* 10.0 documentation, "Design variants" and the Symbol Fields Table.
  [docs.kicad.org/10.0/en/eeschema/eeschema.html](https://docs.kicad.org/10.0/en/eeschema/eeschema.html)
- **SNVA112A** · TI, *AN-1378 Method for Calculating Output Voltage Tolerances in Adjustable Regulators* (2013).
  [ti.com/lit/an/snva112a](https://www.ti.com/lit/an/snva112a/snva112a.pdf)
- **SLVAFE6** · TI, *Adjusting the Output Voltage of a Fixed Output Voltage DC-DC Converter* (2022).
  [ti.com/lit/an/slvafe6](https://www.ti.com/lit/an/slvafe6/slvafe6.pdf)
- **JESD48** · JEDEC, *Product Discontinuance*, by title (how vendors announce end of life).

## `BOM-1` Resistors (or capacitors) with values a step apart that could be one value <a id="bom-1"></a>

💡 suggestion · needs: parsed values (`IMP-6`), the role of each part · [catalogue entry](../checks.md#bom-1)

**Impl.** easy · depends on `IMP-6`

**What it catches.** Two values one E-series step apart (33 Ω and 39 Ω, 4.7 kΩ and 5.1 kΩ, 1 µF and 2.2 µF) in roles
where either would do.

**Why it matters.** Every distinct value is a BOM line, a feeder on the pick-and-place machine, a reel to stock and a
chance for the wrong reel. Where the exact value does not matter (pull-ups, LED resistors, series terminations,
bulk decoupling), merging them is free. Where it does matter (dividers, filters) the finding is noise and is waived.

**Example.** B125A's assembly notes ask to consolidate the 39 Ω and 33 Ω of R48 and R49, and C40, C52 and C53; B135B's
change list asks the same for C301/C307, C506/C510 and a lone 49.9 Ω.

**How to check.** Collect values per class (`IMP-6`), find pairs one step apart in the E24 or E96 grid and report them
with their designators; a role filter (pull-up, series, decoupling) cuts the noise. better_erc
(`erc_checks/src/bom/bom_1.rs`) takes the distinct values of the fitted resistors and of the fitted capacitors and
reports two values one step apart in the smallest series that has both (`ESeries::steps`): 33 Ω and 39 Ω in E12,
4.7 kΩ and 5.1 kΩ in E24, 4.99 kΩ and 5.11 kΩ in E96, 1 µF and 2.2 µF in E3, with the parts of both. The role filter
waits for pin roles (`NET-7`); until then every pair is listed and the ones in dividers and filters are waived.

**Fix.** Pick one value where the circuit allows; leave dividers and filters alone.

**References.** E series (step sizes per series); B125 rev B change list ("From the VB125A assembly"); B135 rev C
change list.

## `BOM-2` Capacitor voltage, size and dielectric report: find odd ones <a id="bom-2"></a>

💡 suggestion · needs: capacitance, voltage, size and dielectric (fields or MPN decoding), net voltage (`NET-5`) · [catalogue entry](../checks.md#bom-2)

**Impl.** mid · depends on `NET-5`, MPN decoding (no FEATURES id yet)

**What it catches.** A table of capacitors by value, voltage rating, case size and dielectric, with the odd ones
marked: one 100 nF in 0603 among 0402s, one 16 V part where all others are 50 V, an X5R among X7Rs, a Y5V anywhere.

**Why it matters.** Odd ones are usually unintended: a library part copied from another project, or a rating chosen
without the net in mind. Class II ceramics also lose capacitance with DC bias, more in small cases and at a larger
fraction of their rating: a 10 µF 6.3 V X7R 0805 can lose 35-65 % at rated voltage and 12-32 % at 3.3 V, depending on
the vendor. A "10 µF" on a 5 V rail in 0603 may be 3 µF in the circuit (`RATE-16`).

**Example.** B200A's change list asks whether the input caps need a 50 V rating at USB and whether a single 10 nF and
the only 100 kΩ on the board can be replaced: both came out of a BOM read line by line, which this report automates.

**How to check.** Collect the fields (or decode the MPN), group by value, and report outliers in voltage, size and
dielectric; join with the net's voltage to flag ratings close to it.

**Fix.** Harmonise ratings and sizes where nothing speaks against it; size voltage and case for the DC-bias derating
the circuit needs.

**References.** EPCI DC bias §2 (technical background), §3 (impact on designs); catalogue `RATE-1`, `RATE-16`; B200 rev B
change list.

## `BOM-3` Big and small connectors, big and small parts, SMT and THT mixed and similar <a id="bom-3"></a>

💡 suggestion · needs: footprints (package, mount type) · [catalogue entry](../checks.md#bom-3)

**Impl.** easy · depends on netlist

**What it catches.** A report of assembly mixes that cost money: through-hole parts on an otherwise SMT board,
one 0201 among 0603s, one connector family different from the rest, parts on both sides.

**Why it matters.** Each mix can add a process step: a single THT part means wave or selective soldering or a hand
step; a second passive size means more nozzles and feeders; a part on the bottom side means a second reflow pass.
Often the odd part is there by accident and has an SMT or same-size twin.

**Example.** B125A's assembly notes: a Y-rated capacitor footprint far bigger than needed, and a wish to spread out
0402s for hand assembly.

**How to check.** Group footprints by package size and mount type from the footprint name or library data; report
minorities. better_erc (`erc_checks/src/bom/bom_3.rs`, `erc_analysis::package`) reads each fitted part's footprint:
the chip case size (`R_0402_1005Metric` is 0402), SMD or through-hole (from the name, or the board file's attribute
when it was read) and the side (board file). Among the fitted parts it reports the minority mount type, each minority
case size, each minority connector family (the footprint name up to its part number, with the pitch: `JST_PH
P2.00mm`, `PinHeader P2.54mm`; only connectors whose family is a choice, not USB, HDMI, RJ45, card or coax), and the
minority side; a minority is a group of at most `minority` (0.2) of its kind. A part in two minorities (a THT header
that is also the odd family) is one finding.

**Fix.** Pick an SMT or same-size alternative where it exists; if a mix stays, plan the assembly step for it.

**References.** B125 rev B change list ("From the VB125A assembly"); catalogue `LAY-11`.

## `BOM-4` Propose consolidating identical resistors into resistor networks <a id="bom-4"></a>

💡 suggestion · needs: values (`IMP-6`), which resistors share a part or a bus · [catalogue entry](../checks.md#bom-4)

**Impl.** easy · depends on `IMP-6`

**What it catches.** Groups of identical resistors (same value, same package) that sit on pins of one part or one bus
and could be one array.

**Why it matters.** Four 0402 pull-ups are four placements; one 4-element array is one, with a smaller footprint and
fewer joints. It is a trade-off, not a rule: arrays come in fewer values, make single-resistor rework harder, and
couple the elements thermally.

**Example.** Eight 10 kΩ pull-ups on a parallel bus next to an MCU: two Bourns CAT16-103J4LF arrays (4 isolated
elements, 1206 size) replace them.

**How to check.** Identical resistors whose one end goes to pins of the same part (or the same connector) and whose
other end is shared (a rail) or free; groups of 2, 4 or 8. better_erc (`erc_checks/src/bom/bom_4.rs`): fitted
two-pin resistors of one value and case whose one end is on signal pins of the same IC or connector (one resistor per
net, not a supply net) and whose other ends share a net (pull-ups) or are all apart (series resistors); a group of
`min_group` (2) or more is one finding, named with the parts on both sides, and a group inside a bigger one is the
bigger one's.

**Fix.** Use an isolated-element array where values and layout allow; keep discrete parts where rework or precision
matters.

**References.** Bourns, *CAT/CAY16 Series - Chip Resistor Arrays* (2, 4 or 8 isolated elements, "How To Order")
([bourns.com/docs/Product-Datasheets/CATCAY.pdf](https://www.bourns.com/docs/Product-Datasheets/CATCAY.pdf)).

## `BOM-5` No TODOs or "calculate later" values left <a id="bom-5"></a>

🛑 error · needs: the value field · [catalogue entry](../checks.md#bom-5)

**Impl.** easy · depends on `IMP-6`

**What it catches.** Placeholder values in the schematic: `?`, `??k`, `TBD`, `TODO`, `calc`.

**Why it matters.** A placeholder is a decision not taken. It survives into the BOM, the assembler asks (a week
lost) or guesses (a board lost). The second form of the same mistake, the placeholder part being fitted, is `BOM-6`.

**Example.** A feedback resistor marked `?k` "until the output voltage is decided": the BOM goes out with `?k`, the
assembler fits the nearest reel.

**How to check.** Values that start or end with `?`, or contain `TBD`, `TODO` or `calc` as a word, on any part, fitted
or not (`erc_checks/src/bom/bom_5.rs`); a fitted one is reported once, as `BOM-6`.

**Fix.** Decide the value; if it really stays open, mark the part DNP and say so on the sheet.

**References.** Test schematic `bom-10_values` (R3, R9, R10).

## `BOM-6` "Calculate later" part mounted <a id="bom-6"></a>

🛑 error · needs: value field, DNP flag · [catalogue entry](../checks.md#bom-6)

**Impl.** easy · depends on `IMP-6`

**What it catches.** A part with a placeholder value that is not DNP: it will be placed.

**Why it matters.** A placeholder on a DNP part is harmless (an option for later); on a mounted part the assembler
must put something there. This is the version of `BOM-5` that breaks a build.

**Example.** A divider resistor `??` mounted in the base variant: the first article comes back with a 0 Ω, the rail
it sets comes up at the regulator's reference voltage.

**How to check.** `BOM-5` findings filtered by the part's DNP flag (and its per-variant flags, `BOM-13`).

**Fix.** Decide the value, or make the part DNP.

**References.** FEATURES.md (`BOM-5`, `BOM-6`); KiCad variants (DNP per variant).

## `BOM-7` Depopulate spare and debug parts for release <a id="bom-7"></a>

💡 suggestion · needs: which parts are debug or spare (intent) · [catalogue entry](../checks.md#bom-7)

**Impl.** judgement · depends on netlist (candidates: debug headers, bypass 0 Ω, test LEDs)

**What it catches.** Parts that only serve bring-up (debug headers, bypass 0 Ω links, test LEDs, jumper options)
still mounted in the production variant.

**Why it matters.** Each one costs a placement and some are a risk in the field: a debug header invites a probe into
a running product, a bypass link defeats the protection it bypasses, a debug LED drains a battery. Others must stay
(the SWD pads for production programming). Which is which is a decision.

**Example.** B153A's R211/R212 0 Ω level-shifter bypass is DNP in both variants and must stay so: fitted together
with the shifter enabled, the MCU and the target drive each other at different levels. An older BOM CSV and an
importer that ignored the variant showed them as placed (`BOM-13`).

**How to check.** better_erc can list candidates (0 Ω across a part, LEDs on GPIOs, debug headers, jumpers) per
variant; a person confirms.

**Fix.** A production variant with the debug parts DNP, reviewed once per release.

**References.** KiCad variants; B153A schematic review §8 (R211/R212), §9 (variants).

## `BOM-8` LED swapped (polarity) <a id="bom-8"></a>

🛑 error · needs: LED pins (anode, cathode), rails and grounds (`NET-5`) · [catalogue entry](../checks.md#bom-8)

**Impl.** easy · depends on `NET-5`

**What it catches.** An LED whose anode sits on the lower-potential side of its circuit: anode to ground, cathode
to a rail, or cathode toward a high-side driver.

**Why it matters.** A reversed LED stays dark, and if the reverse voltage exceeds its rating (often 5 V) it can be
damaged (`RATE-24`). Most often the symbol is right and the footprint's pad mapping or the assembly orientation is
wrong (`LIB-11`, `LAY-6`); the schematic version of the mistake is the easy one to catch.

**Example.** A status LED drawn from GPIO through 1 kΩ with its anode on GND: the GPIO drives high, the LED is
reverse-biased and dark.

**How to check.** For each LED, find which pin leads (through resistors) to a rail and which to ground or a driver;
anode toward ground is a finding. The footprint side needs `LIB-11`. better_erc (`erc_checks/src/bom/bom_8.rs`) takes
the fitted LEDs with pins named anode and cathode (`A`, `K`), walks from each through fitted two-pin resistors and
ferrites (`max_hops` = 2) to the rails and grounds of `NET-5`, and reports an anode that reaches only ground or a
negative rail, or a cathode that reaches only a rail. A cathode toward a high-side driver needs pin roles (`NET-7`).

**Fix.** Turn the LED in the schematic; check the footprint's pad 1 against the datasheet's cathode mark.

**References.** B135 rev C change list (pin-1 marks on D201, D601, D603); catalogue `LIB-11`, `LAY-6`, `RATE-24`.

## `BOM-9` Part without a value <a id="bom-9"></a>

🛑 error · needs: the value field · [catalogue entry](../checks.md#bom-9)

**Impl.** easy · depends on netlist

**What it catches.** A component whose value field is empty.

**Why it matters.** An empty value gives an empty BOM line: the assembler cannot buy it, and a reviewer cannot check
it. With resistors and capacitors it also blinds every value-based check (`I2C-9`, `RAIL-2`).

**Example.** A capacitor placed from a generic `C` symbol, footprint set, value never typed.

**How to check.** A component (not a power symbol) whose Value field is empty in the variant checked
(`erc_checks/src/bom/bom_9.rs`); `DNP` and `DNM` as values are accepted.

**Fix.** Fill in the value (and the MPN, `BOM-14`).

**References.** Test schematic `bom-10_values` (R2).

## `BOM-10` Value that doesn't parse or isn't a standard series value <a id="bom-10"></a>

⚠️ warning · needs: parsed values (`IMP-6`) · [catalogue entry](../checks.md#bom-10)

**Impl.** easy · depends on `IMP-6`

**What it catches.** A value written in a form the tools cannot read (`4,7 k`, `4k7 Ohm`, `4K7R`) or one that is not in
any E series (`4.8k`).

**Why it matters.** Values that do not parse make every value-based check skip the part; odd notation also confuses
humans (`4R7` vs `4k7`). A value outside the series can still be bought only as a special or a high-precision part,
which is often a typo.

**Example.** `5k` written for 5.1 kΩ (5.00 kΩ is in no E series, E96 has 4.99 kΩ), `10K` vs `10k` mixed, `2.2 K` with a space.

**How to check.** Resistor, capacitor and inductor values are read in the unit their part class gives
(`erc_analysis::value::Values`, `erc_checks/src/bom/bom_10.rs`), on the parts the variant fits: a value that does not
read is reported, and so is odd notation (more than one space before the multiplier, a small `r`, an `R` after a
multiplier, a decimal comma, the unit as a separate word, text after the value that is no tolerance, rating or
dielectric). A value in no E series up to E192 is reported with the nearest E24 and E96 values. Values of other
classes (a crystal's frequency, a fuse's current) are read but not held to a series.

**Fix.** One notation across the project (`4k7`, `100n`); values from the series the parts come from.

**References.** E series (E24, E96 values); FEATURES.md (`BOM-10`, `IMP-6`).

## `BOM-11` Same value in two packages <a id="bom-11"></a>

💡 suggestion · needs: value and footprint · [catalogue entry](../checks.md#bom-11)

**Impl.** easy · depends on netlist

**What it catches.** One value used in two case sizes: 100 nF in 0402 and in 0603, 10 kΩ in 0402 and 0805.

**Why it matters.** Two BOM lines and two reels for one value, and often unintended (a block copied from another
board). Sometimes it is right: power rating, voltage rating, or DC-bias derating ask for the bigger case.

**Example.** A board with 23 × 100 nF 0402 and 2 × 100 nF 0603 (copied with a USB block from an older design).

**How to check.** Group by value and footprint; report values with more than one package, with designators.
better_erc (`erc_checks/src/bom/bom_11.rs`) groups the fitted parts whose value reads in their class's unit
(resistors, capacitors, inductors, ferrites, crystals, fuses) by class and value (`100n` and `100nF` are one), and
reports a value in more than one package: the case size, else the footprint name.

**Fix.** Move the minority to the majority package unless rating or derating needs the bigger one.

**References.** EPCI DC bias §2 (0805 vs 0603 capacitance loss with DC bias); catalogue `BOM-2`.

## `BOM-12` Value field vs the MPN's parameters <a id="bom-12"></a>

🛑 error · needs: MPN decoding per vendor family, the value and footprint fields · [catalogue entry](../checks.md#bom-12)

**Impl.** mid · depends on MPN decoding (no FEATURES id yet), `IMP-6`

**What it catches.** An MPN whose encoded resistance, capacitance, voltage, tolerance or package disagrees with the
part's value or footprint: the schematic says 13.7 kΩ, the MPN orders 13.3 kΩ.

**Why it matters.** The assembler buys by MPN; the reviewer reads the value. A mismatch passes every review that looks
at only one of them, and the assembled board has the wrong part. Passive MPNs encode their parameters in a
documented scheme (Yageo's `RC0603FR-0713K7L` is 0603, ±1 %, 13.7 kΩ), so the check can be mechanical.

**Example.** B200A's analog dividers were drawn 13.7 kΩ and assembled with 13.3 kΩ (R201, R203, ...): the 0-5 V range
scales by 0.668 instead of 0.661 and the ADC clips at ~4.94 V; the change list asks "bad MPN? why wasn't it caught?".
B153A's SBU pull-ups R219/R222 should be 3.3 kΩ and were fitted with 13.3 kΩ ("wrong second resistor choice"); B135B's
C405, C509, C511 and C512 had MPN errors.

**How to check.** Decode the MPN per vendor family (Yageo RC, Panasonic ERJ, Samsung CL, Murata GRM, ...), compare
value, tolerance, voltage and case size with the fields and the footprint; check every alternate MPN of a part too.

**Fix.** Correct the MPN (and its alternates); keep MPNs generated from the value, not typed.

**References.** Yageo RC_L "Ordering information - global part number"; B200 rev B change list (R201, R314); B200A
schematic review §0 and §7; B153 rev B change list (R219, R222); B135 rev C change list (C405, C509).

## `BOM-13` Variant: a part's DNP flag disagrees with its per-variant fields <a id="bom-13"></a>

⚠️ warning · needs: variant data from the schematic (KiCad 10 design variants), BOM per variant · [catalogue entry](../checks.md#bom-13)

**Impl.** mid · depends on `IMP-1` (variant data from the schematic), `IMP-8`

**What it catches.** Two kinds of variant confusion: a part whose base DNP flag says one thing while its per-variant
override or fields say another (a DNP part with a variant MPN, a fitted part whose variant value is empty), and a BOM
or placement file built from the base DNP flags instead of a variant.

**Why it matters.** Variants live as overrides on top of the base design. Tools that read only the base flag (a plain
netlist, an older BOM export, an importer) see neither variant correctly: they list parts that no variant fits, or
miss parts every variant fits. The assembler builds what the file says.

**Example.** B153A has Main and Plus variants; R211/R212 are DNP in both, but the old BOM CSV and the mx3 import
showed them placed. Its hub U301 still carries 18 alternate MPNs mixing the 2-port USB2512B and the 4-port USB2514B in
one line, so a variant BOM could get the wrong hub; the change list asks for per-variant MPNs and per-variant BOM
exports.

**How to check.** Read the variant overrides from the schematic, build each variant's fitted list, compare with the
base flags and with per-variant fields; any BOM or PnP file handed in can be checked against the variant it claims.

**Fix.** Keep variant data in the variant mechanism, not in base flags and comments; export BOM and placement per
variant.

**References.** KiCad variants ("Understanding design variants", "Generating variant-aware outputs"); B153A schematic
review §9; B153 rev B change list (per-variant MPNs and BOM export).

## `BOM-14` Mounted part without an MPN <a id="bom-14"></a>

⚠️ warning · needs: MPN field, DNP flag · [catalogue entry](../checks.md#bom-14)

**Impl.** easy · depends on netlist

**What it catches.** A fitted part with an empty manufacturer part number field (or no such field).

**Why it matters.** Without an MPN the assembler or the buyer picks a part from the value: for a resistor that may be
fine, for a capacitor it decides voltage, dielectric and DC bias, for an inductor saturation current, for a diode
everything. Decisions made by someone who has not seen the schematic.

**Example.** A 4.7 µH inductor with value and footprint but no MPN: the assembler fits a 0.8 A part on a 2 A buck.

**How to check.** Parts not DNP whose MPN field (by name, configurable: `MPN`, `Mfr. Part`, `Manufacturer_Part_Number`)
is empty. Test points, mounting holes and fiducials are exempt. better_erc (`erc_checks/src/bom/bom_14.rs`) reads
every field whose name starts with `MPN` (`Mpn1`, `MPN_2`) or is `Mfr. Part`, `Mfr PN`, `Manufacturer Part Number`
(case and punctuation ignored), in the variant checked; a part with one non-empty is fine. Parts out of the BOM and net
ties are exempt too. When no part of the board has an MPN, that is one finding for the board, not one per part. The
field names become configurable with the project file (`CFG-2`).

**Fix.** Fill in the MPN, with alternates for passives.

**References.** KiCad variants (field name templates, Symbol Fields Table).

## `BOM-15` MPN not recommended for new designs, obsolete or single-source <a id="bom-15"></a>

💡 suggestion · needs: lifecycle and sourcing data per MPN (a part database or distributor API) · [catalogue entry](../checks.md#bom-15)

**Impl.** mid · depends on a part lifecycle source (no FEATURES id yet)

**What it catches.** Parts whose vendor status is NRND, last-time-buy or obsolete, or that only one manufacturer makes
with no pin-compatible second source.

**Why it matters.** A part going end of life forces a redesign at the worst time; a single-source part can stop
production on an allocation. Knowing it at review allows a second footprint, a stock buy or another part.

**Example.** B153A uses the DPO2036 for CC/SBU overvoltage protection; the change list asks to re-evaluate it as
single source, comparing TI TPD4S311A / TPD6S300A, ST TCPP01-M12 / TCPP03-M20 and discrete options.

**How to check.** Look each MPN up in a lifecycle source (cached), report status and the number of manufacturers.

**Fix.** Pick an active, multi-source part, or plan the risk (footprint for an alternative, stock).

**References.** JESD48 (product discontinuance notices); B153 rev B change list (DPO2036).

## `BOM-16` Tolerance missing or loose on a part in a precision role <a id="bom-16"></a>

💡 suggestion · needs: part roles (feedback divider, ADC reference divider, sense resistor; `NET-7`), tolerance (field or MPN) · [catalogue entry](../checks.md#bom-16)

**Impl.** mid · depends on `NET-7`, `RPT-6`, MPN decoding

**What it catches.** Resistors in a role where their tolerance sets an accuracy (regulator feedback, ADC reference or
input divider, current-sense shunt and its gain resistors) with no tolerance given, or 5 % where 1 % or better is
needed.

**Why it matters.** The output of a regulator is its reference times the divider ratio: a ±1 % reference with two ±5 %
resistors can be off by about ±10 % in the worst case. TI's SLVAFE6 shows the same arithmetic in the small: two 1 %
resistors turn a ±1 % fixed-output converter into a ±1.3 % one. Without a tolerance in the BOM the buyer picks the
cheapest.

**Example.** B200A's analog inputs scale 0-5 V into the 3.3 V ADC with 13.7 kΩ / 26.7 kΩ dividers (×0.661); every
percent of resistor tolerance is a percent of measurement error on all eight channels.

**How to check.** Find dividers into regulator FB pins and ADC inputs, and shunts with their amplifiers; read the
tolerance from the field or the MPN; flag missing and loose ones.

**Fix.** 1 % (or 0.1 % for measurement) in precision roles, stated in the value or MPN; compute the error budget per
SNVA112A.

**References.** SNVA112A §3 (worst-case approach); SLVAFE6 §2 (two 1 % resistors: ±1 % to ±1.3 %); B200A schematic review
§7.

## `BOM-17` One MPN with two different values (a copy-paste part) <a id="bom-17"></a>

🛑 error · needs: MPN and value fields · [catalogue entry](../checks.md#bom-17)

**Impl.** easy · depends on netlist

**What it catches.** The same MPN on parts with different values: a 10 kΩ MPN on a part whose value says 4.7 kΩ.

**Why it matters.** A copied part keeps its MPN when only the value is edited. The BOM groups by MPN, so the 4.7 kΩ
silently becomes 10 kΩ on the board, while the schematic still reads 4.7 kΩ. It is the cheapest form of `BOM-12` to
catch: no MPN decoding needed.

**Example.** A pull-up block copied from an I2C bus to a reset line, value changed from 4k7 to 100k, MPN
`RC0402FR-074K7L` left as it was: the reset line gets 4.7 kΩ.

**How to check.** Group parts by MPN; more than one distinct value (after normalising notation, `IMP-6`) is a finding.
better_erc (`erc_checks/src/bom/bom_17.rs`) groups the parts, DNP ones too (another variant may fit them), by every
MPN field they carry, and compares the values read in their class's unit (`4k7` and `4.7k` are one value; a value
that does not read compares as text).

**Fix.** Correct the MPN on the edited parts.

**References.** Yageo RC_L "Ordering information - global part number"; catalogue `BOM-12`.

## `BOM-18` Parts of the vendor's typical application or reference design left out without a stated reason <a id="bom-18"></a>

💡 suggestion · needs: the vendor's typical application per part, intent · [catalogue entry](../checks.md#bom-18)

**Impl.** judgement · depends on part data (typical application circuits; no FEATURES id yet)

**What it catches.** A part whose datasheet's typical application or the vendor's evaluation board has components
(filter capacitors, a bootstrap cap, a snubber, pull-ups on config pins) that the design does not, with no note why.

**Why it matters.** The typical application is what the vendor tested. Leaving a part out can be right (the EVK
covers options the design does not use), but it must be a decision; unexplained omissions are where a part that
"should work" does not.

**Example.** B153A's hub filter capacitors C306 (CRFILT) and C310 (PLLFILT), 1 µF: fitted in the Main variant, DNP in
Plus "as on the Microchip hub EVK", while the vendor's checklist asks for 1 µF; the review left it as an open question
to double-check.

**How to check.** A comparison against the typical application, done by a person or an LLM from the datasheet;
better_erc can list a part's pins and what hangs on them next to the datasheet's circuit.

**Fix.** Fit the part, or write the reason on the sheet next to it.

**References.** The part's datasheet (typical application) and evaluation board schematic; B153A schematic review §6
(C306/C310); catalogue `RAIL-16`.
