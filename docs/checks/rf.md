# RF checks, explained

Detail page for the `RF` category of the [check catalogue](../checks.md#-rf---rf). Each section explains one
check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the
usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **ESP32-C3 HDG** · Espressif, *ESP32-C3 Hardware Design Guidelines*, release master (12 Oct 2026). §1.3.5 clock
  source, §1.3.6 RF, §1.4.4 RF layout.
  [docs.espressif.com/…/esp32c3](https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32c3/esp-hardware-design-guidelines-en-master-esp32c3.pdf)
- **ESP32-S3-WROOM-1 DS** · Espressif, *ESP32-S3-WROOM-1 / ESP32-S3-WROOM-1U Datasheet*, v1.8. The PCB-antenna and
  the connector variant of one module.
  [espressif.com](https://www.espressif.com/sites/default/files/documentation/esp32-s3-wroom-1_wroom-1u_datasheet_en.pdf)
- **NINA-W10 DS** · u-blox, *NINA-W10 series data sheet*, UBX-17065507 R17 (May 2025).
  [content.u-blox.com](https://content.u-blox.com/sites/default/files/NINA-W10_DataSheet_UBX-17065507.pdf)
- **SX1261/2 DS** · Semtech, *SX1261/2 Datasheet*, Rev 2.2 (December 2024), linked from the product page
  (fetched 21 Oct 2026 through `tpm fetch`, semtech.com refuses plain HTTP clients).
  [semtech.com/products/wireless-rf/lora-connect/sx1262](https://www.semtech.com/products/wireless-rf/lora-connect/sx1262)
- **MAX-M10S IM** · u-blox, *MAX-M10S Integration manual*, UBX-20053088 R05 (April 2026). Active antenna supply
  and supervisor.
  [content.u-blox.com](https://content.u-blox.com/sites/default/files/MAX-M10S_IntegrationManual_UBX-20053088.pdf)
- **CC2592** · TI, *CC2592 2.4-GHz Range Extender*, SWRS159 (2014). [ti.com/lit/ds/symlink/cc2592](https://www.ti.com/lit/ds/symlink/cc2592.pdf)
- **PE4259** · pSemi, *PE4259 UltraCMOS High-power SPDT RF Switch*, DOC-03694-5.01 (07/2026).
  [psemi.com](https://www.psemi.com/pdf/datasheets/pe4259ds.pdf)
- **AN058** · TI, *Antenna Selection Guide*, SWRA161B. [ti.com/lit/an/swra161b](https://www.ti.com/lit/an/swra161b/swra161b.pdf)
- **SWRU120D** · TI, *2.4-GHz Inverted F Antenna*, SWRU120D (2019). [ti.com/lit/an/swru120d](https://www.ti.com/lit/an/swru120d/swru120d.pdf)
- **SWRA495L** · TI, *Crystal Oscillator and Crystal Selection for the CC13xx, CC26xx, CC23xx, and CC27xx Family
  of Wireless MCUs*, SWRA495L (April 2025). [ti.com/lit/an/swra495](https://www.ti.com/lit/an/swra495/swra495.pdf)
- **2450AT18A100** · Johanson Technology, *Mini 2.45 GHz Antenna, AEC-Q200 Qualified, P/N 2450AT18A100E-AEC*
  (detail specification 12/22/2017). A typical 2.4 GHz chip antenna.
  [johansontechnology.com](https://www.johansontechnology.com/datasheets/2450AT18A100/2450AT18A100.pdf)
- **Wikipedia** · readable overviews: [Impedance matching](https://en.wikipedia.org/wiki/Impedance_matching),
  [Bias tee](https://en.wikipedia.org/wiki/Bias_tee), [Inverted-F antenna](https://en.wikipedia.org/wiki/Inverted-F_antenna).

## `RF-1` RF port without tuning network <a id="rf-1"></a>

⚠️ warning · needs: RF pin roles of the radio (library), parts on the RF net · [catalogue entry](../checks.md#rf-1)

**Impl.** mid · depends on NET-7

**What it catches.** The RF pin of a radio chip (ANT, RF_P/RF_N, LNA_IN, RFIO) wired straight to a trace, an
antenna or a connector with no matching network next to the pin: no series inductor or capacitor and no shunt
footprints between the pin and the rest of the RF path.

**Why it matters.** A radio's RF port is rarely 50 Ω. Its optimum load is a complex impedance set by the PA and
LNA and the package, and the vendor's reference design gets there with a small LC network that also filters the
harmonics. Without it the transmitter sees a mismatch: less output power, more harmonics (a failed emissions
test), worse sensitivity. The values depend on the board, so even a correct network needs footprints to tune.
Modules have the network inside; chip-down designs copied from a module's pinout lose it.

**Example.** An ESP32-C3 chip-down design routes LNA_IN over a 50 Ω trace straight to a 2450AT18A100 chip
antenna. Espressif asks for a CLC network close to the chip (C11 1.2-1.8 pF, L2 2.0-3.0 nH, C12 in 0201) tuned
until S11 sits near the chip's conjugate point 35+j0 and S21 is below −35 dB at the second and third harmonics
(4.8 and 7.2 GHz). Without it there is nothing to tune and nothing that suppresses the harmonics.

**How to check.** Find the pins with an RF role (`NET-7`); on each RF net, the first parts from the pin must be
series and shunt L/C (or 0 Ω and DNP footprints for them) before any connector, antenna or other RF part. A
balun or integrated passive device listed as the radio's matching part counts.

**Fix.** Copy the vendor's matching topology with its footprints next to the pin, even if some start as 0 Ω or
DNP, and tune it on the real board with a network analyser.

**References.** ESP32-C3 HDG §1.3.6 "RF Circuit" and "RF Tuning", Table 2, §1.4.4; Wikipedia, Impedance matching.

## `RF-2` RF net to connector without PI network <a id="rf-2"></a>

⚠️ warning · needs: RF nets, connector and antenna classes, parts on the net · [catalogue entry](../checks.md#rf-2)

**Impl.** mid · depends on NET-7

**What it catches.** An RF net that reaches an antenna feed or an RF connector (u.FL, SMA, MHF4) without a
shunt-series-shunt (π) footprint group just before it.

**Why it matters.** The antenna is the part whose impedance the designer controls least: ground-plane size,
housing, battery, PCB thickness and nearby parts all detune it, and so does a cable to an external antenna.
Antenna vendors give matching values for their evaluation board only and tell you to expect different ones on
yours. A π network costs three 0402 footprints; filled with one 0 Ω or a DC-block capacitor it is invisible when
the antenna happens to be matched, and it is the only way to fix a mismatch without a new board spin. A VSWR
above 2 (return loss under 9.5 dB) already costs link budget.

**Example.** A 2450AT18A100 chip antenna fed straight from a 50 Ω trace. Johanson's own evaluation board needs a
1.0 pF / 2.7 nH / 3.9 nH network to match it, and the datasheet notes that the values and topology on a client's
PCB will be different. TI says the same for its inverted-F PCB antenna: add a π network close to the feed point.

**How to check.** For each RF net ending at a part of class antenna or RF connector (`LIB-2` class data), look
for a shunt part, a series part and another shunt part (caps, inductors, 0 Ω, DNP allowed) on the path between
the RF source and that part.

**Fix.** Three footprints right at the feed point: shunt, series, shunt. Fit the series one with 0 Ω or a DC-block
capacitor and leave the shunts DNP until the board is measured.

**References.** AN058 §3.4 (VSWR, pi-matching recommendation), §6.2.5 (antenna matching); SWRU120D §1;
2450AT18A100 "Mounting Considerations"; ESP32-C3 HDG §1.3.6 (CLC near the antenna).

## `RF-3` Suggest DC block caps between RF parts <a id="rf-3"></a>

💡 suggestion · needs: RF nets, pin roles with DC level (library) · [catalogue entry](../checks.md#rf-3)

**Impl.** mid · depends on NET-7

**What it catches.** Two RF parts (radio, switch, LNA, filter, connector, antenna) joined by an RF net with no
series capacitor between them.

**Why it matters.** Many RF pins carry a DC level: a radio's PA output biased through its choke, an RF switch that
needs its ports at 0 V or at a set bias, an LNA input on its own bias, an active-antenna feed. Joining two of them
directly lets one part's bias flow into the other, shifts operating points, or shorts the bias through an antenna
that is a DC short (an inverted-F is grounded at one end). A series capacitor of a few tens of pF is close to a
short at GHz and an open at DC, and is the usual series element of the π network anyway.

**Example.** u-blox's component list for MAX-M10S has a 47 pF C0G "DC block" (C18) next to the 10 nF bias-T
capacitor (C14) and the DC infeed inductor (L3) that put the active-antenna supply on the RF line. The module's
RF_IN is DC-blocked inside; an external SAW filter or LNA added in front of it, as u-blox suggests for cellular
products, may not be, and then needs its own series capacitor.

**How to check.** On each RF net between two RF-role pins, look for a series capacitor; skip pins the part data
marks as DC-blocked internally (`NET-7`).

**Fix.** A series C0G capacitor (tens of pF at 2.4 GHz, more at sub-GHz) as the series element of the matching
network, or confirm from both datasheets that the pins are DC-blocked internally.

**References.** MAX-M10S IM §1.3 pin assignment (RF_IN "DC blocked internally"), Table 52 (C18 DC block);
AN058 §3.4 (DC-block cap in the π network); Wikipedia, Bias tee.

## `RF-4` Suggest an inline RF test connector <a id="rf-4"></a>

💡 suggestion · needs: RF nets, connector classes · [catalogue entry](../checks.md#rf-4)

**Impl.** mid · depends on NET-7

**What it catches.** An RF path from radio to on-board antenna with no switched coaxial test connector (Murata
MM8130 "SWF" class, Hirose MS-156 class) or other conducted test access.

**Why it matters.** Conducted measurements (output power, frequency offset, harmonics, sensitivity) are what
certification labs and bring-up need, and Espressif's own tuning procedure measures the frequency offset of a TX
tone with a spectrum analyser. Without a test port the only access is soldering a pigtail onto the matching
network, which changes what you measure. A switch connector disconnects the antenna when the probe is inserted and
is nearly transparent otherwise.

**Example.** An ESP32-C3 board with a PCB antenna: tuning the 40 MHz crystal load capacitors to ±10 ppm, as
Espressif asks, needs the RF signal on an analyser. With a switch connector between the matching network and the
antenna that is one probe; without it, it is rework on every board measured.

**How to check.** For each RF path ending at an antenna part, look for an RF connector or test connector on the
path; if none, suggest one. Paths that already end at an RF connector need nothing.

**Fix.** A switched test connector in series between the chip matching network and the antenna matching network,
on the 50 Ω section. On space-limited boards, at least a footprint for a pigtail (a 0 Ω that can be turned).

**References.** ESP32-C3 HDG §1.3.5 (frequency-offset tuning with an analyser), §1.3.6 (RF tuning with a network
analyser); ESP32-S3-WROOM-1 DS §7 "RF Characteristics" (RF data measured at the antenna port, where the RF cable is
connected).

## `RF-5` RF switch, LNA or PA control pins floating <a id="rf-5"></a>

🛑 error · needs: control pin roles of RF parts (library), MCU reset state · [catalogue entry](../checks.md#rf-5)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** The control inputs of an RF switch, front-end module, LNA or PA (CTRL, V1/V2, PA_EN, LNA_EN,
HGM, TX/RX) left unconnected, or driven only by MCU pins that are high-impedance in reset, with no pull resistor.

**Why it matters.** These are plain CMOS inputs with no defined default. Floating, they pick a random mode: the
switch connects the antenna to the wrong port, the PA is enabled while the radio receives, or the LNA sits in the
wrong gain. Since the MCU's pins float during reset and until firmware configures them, the front end can
transmit noise or draw PA current at every boot, and a radio that drives the switch itself (SX1262 DIO2) only
does so after firmware has enabled that function.

**Example.** A CC2592 range extender on a CC2530: PA_EN and LNA_EN select power down (both 0), RX (LNA_EN 1) or TX
(PA_EN 1), HGM selects 11 dB or 6 dB of RX gain. With HGM left open the gain is undefined; with PA_EN and LNA_EN on
GPIOs without pull-downs the part is not in power down while the CC2530 is in reset. A PE4259 switch with CTRL
floating connects RFC to RF1 or RF2 at random.

**How to check.** Find control-role pins of RF parts (`NET-7`); report them when unconnected, or when the only
drivers are MCU pins that tristate in reset (`NET-9`) and no resistor to a rail or ground is on the net.

**Fix.** A pull to the safe state on every control line (usually receive or power down: 10-100 kΩ to ground),
or tie unused controls to a fixed level per the truth table.

**References.** CC2592 §8 (control terminal levels), Table 9-1 (control logic); PE4259 Table 5 and Table 6
(control logic truth tables); SX1261/2 DS §13.3.5 (SetDIO2AsRfSwitchCtrl).

## `RF-6` Radio crystal or TCXO tolerance looser than the radio requires <a id="rf-6"></a>

⚠️ warning · needs: crystal tolerance (MPN), the radio's frequency budget · [catalogue entry](../checks.md#rf-6)

**Impl.** mid · depends on CFG-5

**What it catches.** The reference crystal or TCXO of a radio whose total frequency error (initial tolerance plus
temperature drift plus ageing plus load pulling) is wider than what that radio and its protocol allow.

**Why it matters.** The radio's carrier is synthesised from the crystal, so a 20 ppm crystal error is a 20 ppm
carrier error: 48 kHz at 2.4 GHz. Every protocol bounds that (TI quotes ±40 ppm for Bluetooth LE and 802.15.4),
and some chips ask for much tighter: Espressif wants ±10 ppm on the ESP32-C3's 40 MHz crystal. Narrow-band LoRa at
high spreading factor tolerates only ±50 ppm between transmitter and receiver at SF12, and Semtech expects about
±30 ppm on the end device over all conditions and lifetime. Out of budget, links work on the bench at 25 °C and
drop in the cold or after a year. The generic `CLK-19` checks crystals in general; this one uses the radio's
budget.

**Example.** A 40 MHz 3225 crystal with ±20 ppm initial tolerance and ±30 ppm over temperature on an ESP32-C3: in
total far outside the ±10 ppm Espressif asks for, and Wi-Fi connections fail at temperature extremes. On an
SX1262 LoRaWAN node running SF12, a cheap crystal instead of the TCXO the reference design uses.

**How to check.** Decode the crystal's tolerance and stability from its MPN or part data, sum them as
SWRA495L Equation 9 does, and compare with a per-radio limit table (ESP32-C3 ±10 ppm, BLE ±40 ppm, ...).

**Fix.** A tighter crystal, a TCXO (SX126x drives one from DIO3), or load capacitors trimmed on the real board to
take the initial offset out.

**References.** ESP32-C3 HDG §1.3.5; SWRA495L §3.2 (Equation 9), §3.2.1 (±40 ppm for BLE and 802.15.4); SX1261/2 DS
§3.4 (±30 ppm LoRaWAN), Table 3-8 (FERR_L), §4.1.4 (TCXO control block).

## `RF-7` Module ANT pin wired to an external antenna while the onboard antenna is selected <a id="rf-7"></a>

🛑 error · needs: module variant data (which MPN has which antenna) · [catalogue entry](../checks.md#rf-7)

**Impl.** mid · depends on CFG-5

**What it catches.** A radio module variant with an onboard antenna whose ANT pin (or RF pad) is wired to a
connector or external antenna on the base board, or a connector variant whose ANT pin goes nowhere; also both
paths active at once on a module that selects its antenna with a 0 Ω.

**Why it matters.** Module families share one footprint across antenna variants, and purchasing swaps them. On
the internal-antenna variant the RF signal usually is not on any pin, so the base board's u.FL connector and
cable do nothing, and the PCB antenna radiates into whatever the board designer put around it, often a metal
case chosen because an external antenna was planned. The opposite swap leaves the radio with no antenna at all
(`RF-9`).

**Example.** u-blox NINA-W101 has no internal antenna and brings RF out on its ANT pin; NINA-W102 has a PIFA and
its RF signal is not connected to any module pin. The two share a pin assignment. A board laid out for W101 with
a u.FL to a panel antenna, built with W102 because it was in stock, has a dead connector and an internal
antenna inside the metal enclosure u-blox warns against (it says to use W101 with an external antenna there).
ESP32-S3-WROOM-1 (PCB antenna) vs -1U (connector on the module) is the same trap in another form.

**How to check.** Look up the module's MPN in the part data: antenna type and whether the ANT pin carries RF. If
the variant has an onboard antenna and the ANT pin net reaches anything beyond a no-connect, or the variant has no
antenna and the ANT pin is unconnected, report it.

**Fix.** Pin the MPN in the BOM to the variant the layout needs (no "or equivalent"), and mark the ANT pin
no-connect on internal-antenna variants.

**References.** NINA-W10 DS §1.4.1-§1.4.3 (variants), §2.5 and §2.5.1 (antenna interface, metal enclosures), §3.2
(NINA-W101/W102 pin assignment); ESP32-S3-WROOM-1 DS §1 "Module Overview" (antenna options).

## `RF-8` DC present on an RF port that cannot take it <a id="rf-8"></a>

🛑 error · needs: DC paths on RF nets (bias feeds, DC-short antennas), pin data · [catalogue entry](../checks.md#rf-8)

**Impl.** hard · depends on NET-7, NET-10

**What it catches.** A DC voltage reaching an RF port or part not built for it: a bias tee or antenna supply fed
onto the RF line with no blocking capacitor towards the radio, or a biased line meeting a DC-short antenna or a
DC-coupled input.

**Why it matters.** RF inputs are often DC-coupled to the gate of an LNA or to a switch whose ports must sit at 0 V;
a few volts there shift or destroy the stage. The reverse matters too: an inverted-F or PIFA antenna is grounded at
one end, a DC short, so an active-antenna supply on that line is shorted to ground and only a current limiter
saves the regulator. Swapping an active antenna for a passive one is enough to create this.

**Example.** A MAX-M10S board built for an active GNSS antenna: VCC_RF feeds the antenna through L3 (DC infeed) and
R8 (short-circuit current limiter). Fitted later with a passive inverted-F antenna, the 3.3 V supply sees a short
through the antenna; without R8 and the antenna supervisor that is VCC_RF shorted at every power-up. If the board
adds an external SAW filter or LNA between bias feed and RF_IN, its input sees the antenna DC unless a DC-block
capacitor (C18) is in line.

**How to check.** Follow DC through inductors, resistors and 0 Ω parts on RF nets (`NET-10`) from any rail to every
RF-role pin and antenna; report a DC path that reaches a pin the part data does not mark DC-tolerant, or an
antenna marked DC-short.

**Fix.** A series capacitor between the bias feed and the RF part; a current limiter (resistor or switch with
limit) on any antenna supply; a supervisor or a variant rule that removes the bias when a passive antenna is fitted.

**References.** MAX-M10S IM §3.3.1 (antenna supervisor, R8), §4.3.4 (antenna power supply), Table 52 (C18 DC block);
Wikipedia, Bias tee and Inverted-F antenna.

## `RF-9` Radio with nothing on its RF port <a id="rf-9"></a>

⚠️ warning · needs: RF pin roles, RF path to an antenna, connector or load · [catalogue entry](../checks.md#rf-9)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A radio or PA whose RF port, followed through its matching network, ends at no antenna, RF
connector or termination: an unconnected pin, a DNP 0 Ω, or a matching network that stops at nothing.

**Why it matters.** A transmitter into an open reflects all of its power back into the PA. Datasheets limit the
mismatch the PA survives: Semtech allows at most 10:1 VSWR at the SX126x antenna port, and Espressif warns that
running with RF enabled and no antenna "may result in unstable behavior or potential damage to the RF circuit".
Firmware that starts the radio stack by default makes this a real risk on a board where the antenna was left off.

**Example.** An SX1262 board whose antenna is a PCB trace drawn only in the layout: the schematic RF net ends at
the last matching capacitor, and nothing tells a reviewer (or better_erc) whether an antenna is there. Or a
variant where the u.FL is DNP to save cost while the firmware still transmits.

**How to check.** From each RF-role pin follow the series parts (`NET-10`); the path must end at a part of class
antenna, RF connector or a 50 Ω load. A layout-only antenna should be in the schematic as a part (an `AE`
designator, net-tie style), so the check can see it.

**Fix.** Put every antenna in the schematic as a part, fit a 50 Ω load or connector on variants without one, and
keep the radio's transmitter off in firmware when no antenna is fitted.

**References.** SX1261/2 DS Table 3-3 (VSWR 10:1 at the antenna port); ESP32-C3 HDG §1.3.6 (note on running without
an antenna).
