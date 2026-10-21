# GND checks, explained

Detail page for the `GND` category of the [check catalogue](../checks.md#-gnd---grounds-and-isolation). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find
it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **MT-031** · Analog Devices, *Grounding Data Converters and Solving the Mystery of "AGND" and "DGND"*, tutorial
  MT-031 (Rev. A, October 2008): ground planes, star ground, separating analog and digital ground planes, AGND and DGND
  pins (fetched through `tpm fetch`, analog.com refuses plain clients).
  [analog.com MT-031.pdf](https://www.analog.com/media/en/training-seminars/tutorials/MT-031.pdf)
- **SLYT499 / SLYT512** · S. Pithadia, S. More, TI, *Grounding in mixed-signal systems demystified*, Part 1 and Part 2,
  Analog Applications Journal (1Q and 2Q 2013): split vs partitioned planes, signals crossing a split.
  [ti.com/lit/an/slyt499](https://www.ti.com/lit/an/slyt499/slyt499.pdf),
  [ti.com/lit/an/slyt512](https://www.ti.com/lit/an/slyt512/slyt512.pdf)
- **SLLA284** · TI, *Digital Isolator Design Guide*, SLLA284G (July 2022, revised September 2023); §4.3 creepage
  distance, §4.5 reference planes. [ti.com/lit/an/slla284a](https://www.ti.com/lit/an/slla284a/slla284a.pdf)
- **SLLA353** · TI, *Isolation Glossary*, SLLA353A (October 2014, revised September 2017): VIOWM, VIORM, VIOTM, VISO,
  VIOSM, CIO, creepage, CTI. [ti.com/lit/an/slla353](https://www.ti.com/lit/an/slla353/slla353.pdf)
- **SLLA368** · TI, *Low-Emission Designs With ISOW7841 Integrated Signal and Power Isolator*, SLLA368C (May 2017,
  revised February 2019); §4 interlayer side 1-to-side 2 capacitor, §5 direct and capacitive connection to protection
  earth. [ti.com/lit/an/slla368](https://www.ti.com/lit/an/slla368/slla368.pdf)
- **Wikipedia** · [Galvanic isolation](https://en.wikipedia.org/wiki/Galvanic_isolation),
  [Ground loop (electricity)](https://en.wikipedia.org/wiki/Ground_loop_%28electricity%29),
  [Chassis ground](https://en.wikipedia.org/wiki/Chassis_ground).
- **Ott** · H. W. Ott, *Electromagnetic Compatibility Engineering* (Wiley, 2009), chapter 3 (grounding), chapter 17
  (mixed-signal PCB layout).
- **IEC 60384-14** · *Fixed capacitors for use in electronic equipment, Part 14: Sectional specification: Fixed
  capacitors for electromagnetic interference suppression and connection to the supply mains* (X and Y classes); by
  title, not fetched.

## `GND-1` Split grounds without an isolated circuit <a id="gnd-1"></a>

⚠️ warning · needs: ground nets, isolators and isolated converters · [catalogue entry](../checks.md#gnd-1)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** More than one ground net (`GND`, `AGND`, `PGND`, `GND_ISO`) where no isolator, isolated
converter or transformer separates them: grounds that are split in the schematic for no functional reason.

**Why it matters.** A split ground only makes sense across a real isolation barrier. Without one, the two grounds are
joined somewhere anyway (a 0 Ω, a regulator's pins, the chassis), and every signal that crosses between them returns
through that one joint instead of right under its trace. The loop that forms radiates, picks up noise, and gives
ground offsets in analog measurements: the split makes things worse than one solid plane.

**Example.** B135B (isolated CAN FD to USB) has a real barrier: the PS501 isolated DC/DC makes `+5V0_ISO` for the CAN
side, and the check stays quiet. A sensor board with an `AGND` for its ADC front-end, joined to `GND` only by a 0 Ω at
the regulator while the ADC's SPI lines run from the MCU across the split, is the case it reports.

**How to check.** Ground nets from the power structure (`NET-5`); for each pair, is there a part with a declared
isolation barrier (digital isolator, isolated DC/DC, transformer, optocoupler, `NET-7`) with one side on each? If not,
report the split with the parts that join them.

**Fix.** One ground plane, partitioned by placement (analog parts over the analog area, digital over the digital
area), as MT-031 and SLYT512 recommend; split only across real isolation.

**References.** MT-031 "Ground and power planes", "Separating analog and digital ground planes"; SLYT512 (split vs
partitioned plane, signals crossing a split); Ott chapters 3 and 17.

## `GND-2` Suggest a Y cap between isolated grounds <a id="gnd-2"></a>

💡 suggestion · needs: isolated ground pairs, capacitors between them · [catalogue entry](../checks.md#gnd-2)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** Two grounds separated by an isolation barrier with no capacitor between them.

**Why it matters.** Isolated DC/DC converters push common-mode current across the barrier through their transformer's
interwinding capacitance at the switching frequency. Without a low-impedance return next to the converter, that current
returns through the cables and the environment: a large antenna, and the usual reason an isolated board fails radiated
emissions. A small safety-rated capacitor across the barrier, or stitching capacitance between planes, closes the loop
locally.

**Example.** B135B has the Y capacitor C508 across its barrier, between the USB side and the isolated CAN side fed by
PS501; the notes from the B135B assembly plan one EMC sample without C508 to measure what it buys. A board with only
the isolators and no capacitor across is the finding.

**How to check.** For each isolated ground pair (`GND-1`), look for a capacitor with one pin on each side; suggest one
if none, and pass it to `GND-6` for its rating.

**Fix.** A Y-rated capacitor (typically hundreds of pF to a few nF), sized for the leakage current the application
allows, or interlayer stitching capacitance in the PCB.

**References.** SLLA368 §4 (side 1-to-side 2 capacitor, stitching capacitance); SLLA353 (CIO); IEC 60384-14 (Y class);
B135 rev C change list (C508).

## `GND-3` Analog and digital grounds joined at more than one point, or not at all <a id="gnd-3"></a>

⚠️ warning · needs: analog and digital ground nets, the parts that join them · [catalogue entry](../checks.md#gnd-3)

**Impl.** easy · depends on NET-5

**What it catches.** A design with separate analog and digital ground nets (`AGND`, `DGND`) that are tied together by
two or more parts (0 Ω, ferrites, net ties, a mixed-signal IC's AGND and DGND pins on different nets), or by none.

**Why it matters.** Split grounds need exactly one joint: with two or more, the plane between the joints carries
digital return current through the analog side, which the split was meant to prevent; with none, the analog side
floats and every signal crossing finds its return through parasitic paths. Both give noise and offsets that move with
digital activity.

**Example.** A board with an ADC that has separate AGND and DGND pins (ADS1256 class), both pins tied at the part as
the datasheet says, plus a 0 Ω `NT1` near the
regulator and a ferrite under the MCU: three joints between `AGND` and `DGND`, three loops.

**How to check.** Parts and net ties with one pin on each of the two ground nets, plus IC pins (AGND/DGND roles) that
join them; exactly one is expected (or zero if the project says the grounds are one plane by naming only).

**Fix.** One joint, usually at the mixed-signal part or the supply entry; or (often better) one plane and no split
(`GND-1`).

**References.** MT-031 "Separating analog and digital ground planes", "Grounding and decoupling mixed-signal ICs
with low digital currents" (AGND and DGND pins joined at the part); SLYT499; SLYT512.

## `GND-4` Net crossing an isolation barrier <a id="gnd-4"></a>

🛑 error · needs: isolator pin sides, the side each net belongs to · [catalogue entry](../checks.md#gnd-4)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** A net that is on both sides of an isolation barrier: a power symbol or label of one side used on
the other, typically `GND` on a pin of the isolated side, or a non-isolated rail feeding an isolated part.

**Why it matters.** One wrong power symbol bridges the barrier in copper: the isolation is gone, the isolated DC/DC
is shorted across, and the first ground-potential difference in the field (or the hipot test) flows through signal
traces. Netlist-wise it looks perfect, the isolated part is "connected to ground".

**Example.** On an isolated CAN design like B135B, the transceiver's GND pin on the `+5V0_ISO` side given the plain
`GND` power symbol instead of the isolated ground: the CAN ground and the USB ground are now the same net.

**How to check.** From the isolators' pin sides (library, `NET-7`), assign each net to side 1 or side 2 by
propagating through non-isolating parts; a net reached from both sides is the finding, with the path.

**Fix.** Give each side its own ground and rail symbols (`GND_ISO`, `+5V0_ISO`) and recheck every pin of the isolated
parts.

**References.** SLLA284 §4.3 (creepage distance), §4.5 (reference planes); Wikipedia, Galvanic isolation.

## `GND-5` Non-isolated part or structure spanning the barrier <a id="gnd-5"></a>

🛑 error · needs: barrier sides per net, the parts with pins on both sides · [catalogue entry](../checks.md#gnd-5)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** A part that is not an isolator with pins on both sides of an isolation barrier: a pull-up, a
resistor, a capacitor without a safety rating, a protection diode, a debug header with one ground for both sides, a
shield or mounting hole bonded to both grounds.

**Why it matters.** The barrier is only as good as its weakest link: a 10 kΩ pull-up from the isolated side to the
logic rail leaks current, defeats the common-mode rejection and, at a few hundred volts, burns. A debug header with one
ground pin that both sides use shorts the barrier as soon as a probe is plugged in, and the probe's ground connects
the lab PC to the field side.

**Example.** On B153A the ISOM8600 photo-MOS relay (U101) gives an isolated contact on header pins RL1/RL2; a TVS from
RL1 to the board `GND` "for protection" would put the relay's isolated side back on board ground.

**How to check.** With each net's barrier side from `GND-4`, every part whose pins are on both sides and which is not
an isolator, an isolated converter, a transformer or a Y-rated capacitor (`GND-6`) is a finding.

**Fix.** Keep each passive on one side; protection to the local ground of its side; separate debug grounds or an
isolated debug probe; shields and mounting holes to one ground, decided (`GND-7`).

**References.** SLLA284 §4.3; SLLA353 (creepage, isolation resistance RIO); Wikipedia, Galvanic isolation.

## `GND-6` Isolation rating vs the voltage the barrier must hold <a id="gnd-6"></a>

💡 suggestion · needs: barrier voltage (project), isolation ratings (library) · [catalogue entry](../checks.md#gnd-6)

**Impl.** mid · depends on CFG-2, NET-7

**What it catches.** Isolators, isolated converters, transformers and capacitors across the barrier whose isolation
rating is below what the project needs: working voltage, transient or surge voltage, or a capacitor without a Y
rating.

**Why it matters.** Isolation ratings come in several kinds (VIOWM working, VIORM repetitive peak, VIOTM transient,
VISO one-minute withstand, VIOSM surge) and the weakest part decides. A 1 kV-rated isolated DC/DC next to a 5 kV
digital isolator makes a 1 kV barrier; a plain X7R capacitor across it may meet no isolation spec at all.

**Example.** On B135B the barrier is PS501, the isolated signal path and C508; the weakest of their ratings is the
board's. C508 has to be a Y-rated part: a generic 1 nF 2 kV ceramic of the same size is not safety rated, and when it
fails short it bridges the barrier.

**How to check.** Ratings from the library (`NET-7`), barrier requirement from the project (`CFG-2`); list each part
across the barrier with its rating and the minimum, as a review table.

**Fix.** Parts with matching ratings (and creepage on the layout, `SLLA284` §4.3); Y1 or Y2 class capacitors as the
application's standard asks.

**References.** SLLA353 (VIOWM, VIORM, VIOTM, VISO, VIOSM definitions); SLLA284 §3.1 (parameters of interest), §4.3;
IEC 60384-14.

## `GND-7` Chassis or shield ground tied to signal ground, or floating, without a decision <a id="gnd-7"></a>

⚠️ warning · needs: chassis and shield nets, what joins them to signal ground · [catalogue entry](../checks.md#gnd-7)

**Impl.** easy · depends on NET-5

**What it catches.** A chassis, shield, earth or mounting-hole net that is joined to signal ground directly with no
note, or connected to nothing, where the board should make an explicit choice (direct, R, C, R‖C).

**Why it matters.** Both extremes can be right and both can be wrong: a direct bond lets ESD and cable-shield
currents flow through signal ground; a floating shield does not shield and charges up until it arcs; a capacitor gives
a high-frequency bond without a DC ground loop. The netlist usually shows whatever the symbol library defaulted to,
not a decision.

**Example.** A USB-C receptacle whose shell pins are on `GND` through the footprint, on a board in a metal enclosure
earthed through the mains: the USB cable shield becomes a ground loop between the PC and the device. The usual
alternative is the shell through 1 MΩ ‖ 4.7 nF to `GND` (`USB-4`).

**How to check.** Nets named `CHASSIS`, `SHIELD`, `EARTH`, `PE`, connector shell pins and mounting holes (`NET-5`);
report what joins them to signal ground: nothing, a direct connection, or the R/C network, for the reviewer to confirm.

**Fix.** Decide per product (enclosure, cable, earth) and draw it: a net tie, a resistor, a capacitor or both, at the
connector.

**References.** SLLA368 §5 (direct and capacitive connection to protection earth); Ott chapter 3; Wikipedia, Chassis
ground; Wikipedia, Ground loop (electricity).
