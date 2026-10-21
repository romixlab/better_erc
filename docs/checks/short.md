# SHORT checks, explained

Detail page for the `SHORT` category of the [check catalogue](../checks.md#-short---shorts-and-net-collisions).
Each section explains one check: what it catches, why the board fails without it, a concrete example, what
better_erc needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the
source of truth for ID, severity and wording; this page is the commentary.

References used throughout:

- **KiCad** · KiCad, *Schematic Editor* manual, 10.0, "List of ERC checks" (`#list-of-erc-checks`) and "ERC
  violation severity and pin conflicts map".
  [docs.kicad.org/10.0/en/eeschema/eeschema.html](https://docs.kicad.org/10.0/en/eeschema/eeschema.html)
- **Altium ERC** · Altium, *Validating Your Design Project* (Altium Designer documentation).
  [altium.com/documentation/altium-designer/schematic/design-validation](https://www.altium.com/documentation/altium-designer/schematic/design-validation)
- **SDYA009** · TI, *Designing With Logic*, SDYA009C (1997). [ti.com/lit/pdf/sdya009](https://www.ti.com/lit/pdf/sdya009)
- **SCBA004** · TI, *Implications of Slow or Floating CMOS Inputs*, SCBA004E (1994, rev. 2021).
  [ti.com/lit/pdf/scba004](https://www.ti.com/lit/pdf/scba004)
- **Toshiba unused inputs** · Toshiba, *Handling of Unused Input Pins* (CMOS logic usage considerations,
  e-learning; fetched through `tpm fetch`).
  [toshiba.semicon-storage.com](https://toshiba.semicon-storage.com/us/semiconductor/knowledge/e-learning/cmos-logic-usage-considerations/usage-1.html)
- **SSZB130** · TI, *System-Level ESD Protection Guide*, SSZB130E (2025). [ti.com/lit/pdf/sszb130](https://www.ti.com/lit/pdf/sszb130)
- **ROHM TVS** · ROHM, *Selection Method and Usage of TVS Diodes*, 66AN046E Rev. 001 (2023).
  [fscdn.rohm.com](https://fscdn.rohm.com/en/products/databook/applinote/discrete/diodes/selection_method_and_usage_of_tvs_diodes_an-e.pdf)
- **SLVA139** · TI, *Reverse Current/Battery Protection Circuits*, SLVA139 (2003).
  [ti.com/lit/an/slva139](https://www.ti.com/lit/an/slva139/slva139.pdf)
- **Wikipedia** · *Bus contention*, *Open collector*, *Transient-voltage-suppression diode*: readable overviews.
  [en.wikipedia.org/wiki/Bus_contention](https://en.wikipedia.org/wiki/Bus_contention)

## `SHORT-1` Net shunting a known part <a id="short-1"></a>

🛑 error · needs: the netlist · [catalogue entry](../checks.md#short-1)

**Impl.** easy · depends on netlist

**What it catches.** A two-terminal part (resistor, capacitor, inductor, ferrite, diode) or two terminals of a
transistor that sit on the same net, so the part does nothing.

**Why it matters.** Nobody places a resistor to short it out. Both ends on one net nearly always means two labels that
collided: a port and a net label with the same name, a label copied with its neighbour's text, or two sheets that both
used a generic name like `OUT`. The part is harmless; what the collision joined elsewhere usually is not.

**Example.** A current-sense shunt R12 between `VBAT` and `VBAT_SENSE`, where an ADC divider on another sheet also
labelled its top `VBAT`: fine so far. A later edit renames the shunt's load side `VBAT` too, and R12 now has both pads
on `VBAT`: the sense amplifier reads zero forever, and the load is joined to the battery through nothing.

**How to check.** For every part, compare the nets of its terminals; two terminals of a passive, or gate-source,
drain-source, base-emitter on one net, is a finding (FET G-S on one net can be deliberate in a diode-connected stage,
which the waiver covers).

**Fix.** Find the label that merged the two nets (`WIRE-2`, `WIRE-12`) and rename it.

**References.** KiCad "More than one name given to this bus or net" (`multiple_net_names`); Altium ERC "Nets with
multiple names"; catalogue `WIRE-2`, `WIRE-12`.

## `SHORT-2` Accidental net name collision <a id="short-2"></a>

🛑 error · needs: the netlist, resistor chains · [catalogue entry](../checks.md#short-2)

**Impl.** easy · depends on netlist (NET-2 chains)

**What it catches.** A chain of parts that starts and ends on the same net where it should bridge two: the pull-ups of
both sides of a translator landing on one rail (`VCCA`-R-R-`VCCA` where `VCCA`-R-R-`VCCB` was drawn), or two 0 Ω links
from one net into the same other net.

**Why it matters.** It is `SHORT-1` one step removed: the collision is not across one part but across a chain, so each
part looks fine on its own. On a translator the two sides become one net and the translator is bypassed, the low-voltage
device gets the high-voltage bus.

**Example.** A PCA9306 between a 1.8 V sensor and a 3.3 V MCU, with both sides labelled `SDA`/`SCL` because the sheet
was copied from a board without translator: SDA1 and SDA2 are one net, the 1.8 V sensor sees 3.3 V highs through the
MCU-side pull-up, and its pull-up to 1.8 V fights the 3.3 V one.

**How to check.** Find chains of passives and switches (`NET-2` `find_chains`) whose two ends are the same net, and pin
pairs of the same part that should be on different nets (A/B of a translator, IN/OUT of a switch) but are not.

**Fix.** Rename one side (`SDA_1V8`); a translator's two sides never share a name.

**References.** KiCad `multiple_net_names`; Altium ERC "Nets with multiple names"; catalogue `SHORT-1`.

## `SHORT-3` Non-DNM part shunting another mounted part <a id="short-3"></a>

🛑 error · needs: the netlist, fitted / not fitted per variant · [catalogue entry](../checks.md#short-3)

**Impl.** easy · depends on netlist (DNP flags per variant)

**What it catches.** Two mounted parts in parallel where one of them only makes sense when the other is not fitted: a 0
Ω link across a resistor, a jumper across a ferrite, two option resistors that select two sources and are both fitted.

**Why it matters.** Assembly options are drawn in parallel on purpose, with one of them DNP. When both are fitted, the
option collapses: the 0 Ω shorts the filter resistor, or two selector resistors tie two rails or two signals together.
It happens when variants are kept by hand and one BOM forgets the DNP.

**Example.** On B153A R211/R212 (0 Ω, bypass of the TXS0102 shifter U203) are DNP in both KiCad variants, but an old BOM
export and an importer that ignored variants showed them as fitted. Fitted, they short the shifter's A and B sides:
`V_SBU` at 1.8 V tied to 3.3 V MCU pins.

**How to check.** For parts whose terminal sets are on the same pair of nets, count the ones fitted in the current
variant; a 0 Ω, jumper or switch parallel to another fitted part is a finding.

**Fix.** Mark the option DNP in every variant that does not use it, and generate BOMs from the variants, not from the
base flags.

**References.** Catalogue `BOM-13` (variant BOMs from the base DNP flags); B153 rev B change list (R211/R212 shown as
fitted by the old BOM export).

## `SHORT-4` Power to ground through low-value resistors <a id="short-4"></a>

🛑 error · needs: the power structure, resistor values · [catalogue entry](../checks.md#short-4)

**Impl.** easy · depends on NET-5, IMP-6

**What it catches.** A resistor or a chain of resistors from a rail to a ground whose total resistance draws more
current than a set limit or burns more than its rating.

**Why it matters.** A 0 Ω link from `+3V3` to `GND` is a dead short; a 10 Ω "load" is 330 mA and 1 W in an 0603 rated
for 0.1 W. Most come from a pull-down placed on the wrong side of a label, a value typo (10 instead of 10k), or a 0 Ω
that tied a rail to ground in a variant.

**Example.** A divider for an ADC drawn as 100k / 10k, entered as `100k` and `10` (the `k` lost): 33 mA through the
lower resistor, still a working rail, and the ADC reads 0.

**How to check.** Sum the resistance along chains from each rail to each ground (`NET-2`, `IMP-6`); report currents
above a project threshold and dissipation above the package's rating.

**Fix.** Correct the value or the wiring; if a real load is meant, size it.

**References.** Catalogue `RATE-2` (resistor power rating); FEATURES `IMP-6` (value parsing).

## `SHORT-5` Supply pins shorted to ground, or ground pins to a rail <a id="short-5"></a>

🛑 error · needs: pin names or types, the power structure · [catalogue entry](../checks.md#short-5)

**Impl.** easy · depends on NET-5

**What it catches.** A power-input pin (VDD, VCC, VIN, AVDD) on a ground net, or a ground pin (GND, VSS, VEE) on a
positive rail.

**Why it matters.** Either the symbol's pins are swapped against the part, a power symbol was placed upside down, or a
label joined a rail to ground. At power-up the part has no supply, and if both a rail and ground meet on it the
regulator sees a short.

**Example.** A sensor's VDD pin connected to a `GND` power symbol that was meant for the neighbouring VSS pin, both pins
one grid step apart: the sensor is unpowered, its IO pins back-feed it (`BKF-2`).

**How to check.** Pin names and types against the power structure (`NET-5`); combine with `WIRE-14` for the ground side.

**Fix.** Rewire the pin; check the symbol pin numbers against the datasheet pinout.

**References.** KiCad "Ground pin not connected to ground net" (`ground_pin_not_ground`, 10.0), section "Pin electrical
types"; catalogue `WIRE-14`.

## `SHORT-6` Inputs shorted to ground or power without a resistor <a id="short-6"></a>

🛑 error · needs: pin types, which pins can become outputs · [catalogue entry](../checks.md#short-6)

**Impl.** easy · depends on netlist (NET-7 to tell configurable pins)

**What it catches.** An input or IO pin tied straight to a rail or ground with no resistor in between.

**Why it matters.** For a fixed-function CMOS input, a hard tie is fine. For a pin that can be an output (an MCU GPIO,
an FPGA user IO, a boundary-scan pin, a configuration pin that is driven during boot), a hard tie becomes a short the
moment the pin drives the opposite level: firmware sets it as output, the boot ROM toggles it, a JTAG EXTEST drives it.
The pin's driver then sources 30-100 mA into the rail until it fails.

**Example.** An unused STM32 GPIO tied to GND "so it does not float". A later firmware maps an LED to that pin and
drives it high: the pin sinks its short-circuit current, the port's IO current budget is gone, and the pin is degraded.

**How to check.** Pins of IO or bidirectional type, or input pins of parts with configurable IO (`NET-7`), whose net is
a rail or ground with no series resistor (`NET-10`).

**Fix.** Tie through 1-10 kΩ, or leave the GPIO unconnected and configure it as analog or output-low in firmware
(`LEAK-3`).

**References.** Toshiba unused inputs (pins that can become outputs go through a resistor, never directly); TI SN74ACT17
datasheet SCASE32 §8.2.1.2 "Input Considerations"
([ti.com](https://www.ti.com/document-viewer/lit/html/SCASE32/GUID-A567F4CC-73D6-49DC-9BFF-14FD2C01A872)); SDYA009 §3
"Unused Inputs"; SCBA004 §5.

## `SHORT-7` Shorted IOs <a id="short-7"></a>

⚠️ warning · needs: pin types · [catalogue entry](../checks.md#short-7)

**Impl.** easy · depends on netlist

**What it catches.** Two IO pins of programmable parts on the same net, with no other driver: two MCU GPIOs tied
together, or a GPIO tied to an FPGA IO.

**Why it matters.** It is a legitimate trick (two pins to reach an alternate function on either, a rework that needs a
pin from another port, a PIO and a hardware peripheral on one line), but only if firmware never drives the two pins
against each other. The schematic should say which pin is the driver and which must stay an input.

**Example.** On B200A SPI0 SCK (GPIO34) and SPI1 SCK (GPIO26) are one net, so SPI1 can listen to the bus SPI0 drives
(the SPI monitor): fine while GPIO26 stays an input, a short if SPI1 is ever set up as a master.

**How to check.** Nets with two or more IO-type pins of programmable parts and nothing else driving; report them for a
firmware note rather than as a fault.

**Fix.** Note the rule on the schematic and in the firmware's pin map, or put a series resistor (≥ 330 Ω) so a conflict
is a logic error, not a short.

**References.** SDYA009 §7 "Bus Contention", Table 4; Wikipedia, Bus contention.

## `SHORT-8` Direct connections to battery and other high-energy nets <a id="short-8"></a>

⚠️ warning · needs: battery and high-energy nets (project data) · [catalogue entry](../checks.md#short-8)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** Parts connected straight to a battery or another source that can deliver a large current (a bench
supply input, a vehicle 12 V, a supercap), with no fuse, PTC, current limit or series resistance in between.

**Why it matters.** A battery does not current-limit. A short anywhere on a net tied straight to it (a cracked MLCC, a
solder bridge, a probe slip) has hundreds of amps available until something burns open, usually a trace or the cell.
Pull-ups, dividers and decoupling caps straight on the battery net are each such a point.

**Example.** A Li-ion pack with a 10 µF 0805 MLCC straight across the cell terminals and no protection IC in front: a
flex crack in that capacitor shorts the cell, the board chars before the cell's own protection reacts.

**How to check.** From the battery net (named, a battery or connector part, or a project setting) list all parts
connected before the first fuse, PTC or protection switch.

**Fix.** A fuse or protection IC right at the source, and everything else after it; high-value series resistors for
sense lines.

**References.** SLVA139 (protection right at the battery); catalogue `PIN-4` (no overcurrent protection on an input).

## `SHORT-9` Power to ground through a forward-biased diode or a body diode <a id="short-9"></a>

🛑 error · needs: diode and FET orientation, rail polarity · [catalogue entry](../checks.md#short-9)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A diode with its anode on a rail and its cathode on ground (or on a lower rail by more than a diode
drop), and a MOSFET whose body diode points the same way, from a higher rail to a lower one.

**Why it matters.** A diode that should be reverse-biased (a clamp, a reverse-polarity diode, a TVS drawn as a diode)
and is drawn forward is a short at power-up: the rail supplies whatever the diode can take until it fails, then usually
shorted. The same with a FET whose source and drain were swapped in the symbol: the body diode conducts no matter the
gate.

**Example.** A reverse-polarity diode across a 12 V input, drawn with the anode on `+12V` and the cathode on `GND`: the
bench supply goes into current limit at turn-on, on a battery the diode explodes.

**How to check.** Diodes from pin names (A/K) and FETs from pin names and channel type (`LIB-3`); for each, the voltages
of the two nets (`NET-5`) tell whether the diode is forward-biased in normal operation.

**Fix.** Turn the diode around; swap source and drain of the FET (`DRV-11`).

**References.** SLVA139 Figures 1-3 (series diode, NMOS in the ground return, PMOS in the power path: which way the
diode must point); catalogue `DRV-11`.

## `SHORT-10` TVS or zener with standoff voltage below the net voltage <a id="short-10"></a>

🛑 error · needs: TVS standoff / zener voltage (part data), net voltage · [catalogue entry](../checks.md#short-10)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** A TVS whose reverse standoff voltage (VRWM) or a zener whose zener voltage is below the highest
normal voltage of the net it protects.

**Why it matters.** A TVS is meant to be an open circuit up to VRWM and conduct only during a transient. Below the net's
working voltage it conducts all the time: it heats, drags the rail down, or burns open and leaves the line unprotected.
A zener clamp below the signal's high level clips every high.

**Example.** A 5 V TVS (SMAJ5.0A, VRWM 5 V) on a USB-C VBUS that negotiates 9 V or 20 V with USB PD: at 9 V it conducts
heavily, the source sees a short and drops the contract, or the TVS fails short. On B153A the header IO lines clamp 3.3
V signals with 5V6 zeners behind 255 Ω: above the working level, so they stay open in normal use.

**How to check.** Standoff or zener voltage from the part (MPN decoding or library data) against the net's maximum
voltage (`NET-5`, or the project setting for connector nets and PD rails).

**Fix.** Pick VRWM at or above the highest working voltage (and the clamp voltage below what the protected part
survives); for PD VBUS a 24 V TVS class part.

**References.** SSZB130 §2.1 "Working Voltage (VRWM)"; ROHM TVS, point 1 "Selection with consideration of the voltage on
the wiring"; Wikipedia, Transient-voltage-suppression diode "Characterization".

## `SHORT-11` Two push-pull outputs on one net <a id="short-11"></a>

🛑 error · needs: pin types (output vs open-drain, tri-state) · [catalogue entry](../checks.md#short-11)

**Impl.** easy · depends on netlist (NET-7 for pins whose type the symbol gets wrong)

**What it catches.** A net with two or more pins that are push-pull outputs, with no series resistor between them.

**Why it matters.** When they disagree, one pulls high and the other low: tens of milliamps flow from rail to ground
through both output stages, the net sits at an undefined voltage, and both drivers heat or degrade. Tri-state and
open-drain outputs are made to share a net; push-pull ones are not.

**Example.** An Ethernet PHY's 50 MHz `REF_CLK` drawn with both a crystal oscillator and the MCU's MCO pin on it, one of
the two meant as an assembly option and neither marked DNP: two clocks of slightly different frequency beat against each
other, the net spends part of every cycle in contention, and the PHY loses link at random.

**How to check.** Pin types per net from the netlist; outputs of type "output" (not tri-state, not open collector)
counted per net, with series resistors breaking the count (`NET-10`).

**Fix.** Make one of them open-drain or tri-state, combine through a gate, or separate them with a series resistor so a
conflict is limited.

**References.** SDYA009 §7 "Bus Contention" (Figure 26, Table 4); KiCad "ERC violation severity and pin conflicts map"
(output to output is an error by default); Wikipedia, Bus contention; Wikipedia, Open collector "Wired logic".

## `SHORT-12` Differential pair with P tied to N <a id="short-12"></a>

🛑 error · needs: the netlist (diff pair names or pin roles) · [catalogue entry](../checks.md#short-12)

**Impl.** easy · depends on netlist

**What it catches.** The positive and negative halves of a differential pair on the same net: a `_P` and `_N` label pair
where both ends were labelled `_P`, a bus entry that took the wrong member, or a part's D+ and D− pins on one net.

**Why it matters.** A shorted pair carries no differential signal at all: USB does not enumerate, Ethernet has no link,
LVDS outputs drive into each other. It is easy to miss because each pair looks complete in the drawing, two lines going
from one part to the other.

**Example.** USB `USB_D_P` and `USB_D_N` from a connector to the MCU, where the label on the MCU side was copied and
both read `USB_D_P`: D+ and D− of the MCU on one net, D− of the connector a one-pin net (`WIRE-1`).

**How to check.** Pins whose names mark the two halves of a pair (D+/D−, `_P`/`_N`, `+`/`−` on transceivers) on one net;
nets named as one half that reach both halves' pins.

**Fix.** Relabel the pair.

**References.** Altium ERC "Differential Pair Net Connection Polarity Inversed"; catalogue `HS-10` (P and N swapped).
