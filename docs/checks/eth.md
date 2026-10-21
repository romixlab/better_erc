# Ethernet checks, explained

Detail page for the `ETH` category of the [check catalogue](../checks.md#-eth---ethernet). Each section explains
one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it,
the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **SNLA079** · TI, *AN-1469 PHYTER Design & Layout Guide*, SNLA079D (rev. April 2013). Magnetics, Bob Smith
  termination, MII series termination, clock and strap requirements for the DP838xx family; section and table
  numbers below refer to it. [ti.com/lit/an/snla079e](https://www.ti.com/lit/an/snla079e/snla079e.pdf)
- **SNLA076** · TI, *AN-1405 DP83848 Single 10/100 Mb/s Ethernet Transceiver RMII Mode*, SNLA076A (rev. April
  2013). RMII reference clock and the elasticity buffer. [ti.com/lit/an/snla076a](https://www.ti.com/lit/an/snla076a/snla076a.pdf)
- **DP83848C** · TI, *DP83848C/I/VYB/YB PHYTER* datasheet, SNLS266E. Current-mode 10/100 PHY: straps, PHY address,
  MDIO. [ti.com/lit/ds/symlink/dp83848c](https://www.ti.com/lit/ds/symlink/dp83848c.pdf)
- **DP83867** · TI, *DP83867IR/CR Robust, High Immunity 10/100/1000 Ethernet PHY* datasheet, SNLS484J (rev. June
  2026). Voltage-mode gigabit PHY: 4-level straps, RGMII delay, reset timing.
  [ti.com/lit/ds/symlink/dp83867ir](https://www.ti.com/lit/ds/symlink/dp83867ir.pdf)
- **KSZ9031** · Microchip, *KSZ9031RNX Gigabit Ethernet Transceiver with RGMII Support* datasheet, DS00002117F.
  [ww1.microchip.com/…/00002117F.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00002117F.pdf)
- **LAN8720A** · SMSC/Microchip, *LAN8720A/LAN8720Ai Small Footprint RMII 10/100 Ethernet Transceiver* datasheet,
  rev. 1.4 (Aug 2012). LED/strap sharing, REF_CLK modes, reset.
  [ww1.microchip.com/…/8720a.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/8720a.pdf)
- **TPS2378** · TI, *TPS2378 IEEE 802.3at PoE High-Power PD Interface* datasheet, SLVSB99C (rev. July 2015).
  [ti.com/lit/ds/symlink/tps2378](https://www.ti.com/lit/ds/symlink/tps2378.pdf)
- **RGMII 2.0** · Hewlett-Packard, *Reduced Gigabit Media Independent Interface (RGMII)*, version 2.0 (1 April
  2002), archived copy.
  [web.archive.org/…/RGMIIv2_0_final_hp.pdf](https://web.archive.org/web/20160303171328/http://www.hp.com/rnd/pdfs/RGMIIv2_0_final_hp.pdf)
- **Linux ethernet-controller binding** · the devicetree schema whose comment block explains who adds the RGMII
  delay and how `phy-mode` says it.
  [kernel.org/…/ethernet-controller.yaml](https://www.kernel.org/doc/Documentation/devicetree/bindings/net/ethernet-controller.yaml)
- **IEEE 802.3** · *IEEE Standard for Ethernet*, by title: Clause 22 (MII and the MDIO management interface),
  Clause 33 (DTE power via MDI, i.e. PoE).
- **Wikipedia** · readable overviews: [Media-independent interface](https://en.wikipedia.org/wiki/Media-independent_interface)
  (MII, RMII, RGMII), [Medium-dependent interface](https://en.wikipedia.org/wiki/Medium-dependent_interface)
  (pinouts, Auto MDI-X), [Ethernet over twisted pair](https://en.wikipedia.org/wiki/Ethernet_over_twisted_pair),
  [Power over Ethernet](https://en.wikipedia.org/wiki/Power_over_Ethernet).
- **Ott** · Henry W. Ott, *Electromagnetic Compatibility Engineering* (Wiley, 2009), chapters 3 (grounding) and 12
  (digital circuit radiation, common-mode cable radiation).

## `ETH-1` Magnetics centre taps terminated for the wrong PHY type <a id="eth-1"></a>

🛑 error · needs: PHY driver type (library), magnetics pin roles (centre taps) · [catalogue entry](../checks.md#eth-1)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** The PHY-side centre taps of the Ethernet transformer wired the way a different kind of PHY
wants them, or only two of four taps handled on a gigabit design.

**Why it matters.** The two driver families need opposite things. A *current-mode* driver (DP83848, LAN8720A,
LAN8740A) only sinks current: the line is biased from outside, through the centre tap tied to the PHY's analog
supply (DP83848C §7.2, typical application: "center tap is pulled to VDD"). A *voltage-mode* driver (KSZ9031, DP83867)
sets the common-mode voltage itself: each centre tap must go to ground through its own capacitor and to nothing
else, because the four pairs can sit at different common-mode voltages depending on the link speed (KSZ9031 §12.0).
Swap them and a current-mode PHY transmits nothing (no bias), while a voltage-mode PHY fights a supply on its
outputs. Taps tied together on a gigabit board copied from a 10/100 one short the pairs' common modes.

**Example.** A design moves from a LAN8720A (10/100, current mode) to a KSZ9031 (gigabit, voltage mode) and keeps
the old magnetics block: centre taps joined and fed from 3.3 V through a ferrite. Gigabit link fails or flaps;
100 Mbit/s may even come up, which hides the fault.

**How to check.** Classify the PHY by driver type from its library data; find the transformer's PHY-side centre
tap pins (pin roles of the magnetics or magjack, `NET-7`); check that they are separate and capacitor-to-ground
for voltage mode, or on the PHY's analog rail (`NET-5`) for current mode, and that all four are handled for a
1000BASE-T PHY.

**Fix.** Follow the PHY's datasheet: separate 100 nF per tap to ground for voltage-mode PHYs, a common tap to the
analog supply with its decoupling for current-mode ones; pick magnetics with separate tap pins for gigabit.

**References.** KSZ9031 §12.0 (voltage-mode drivers, separate taps through 0.1 µF to ground); DP83867 §9.2.1.1
(Figure 9-2 note A: each PHY-side tap isolated, decoupled to ground); DP83848C §7.2 (typical application: centre tap
pulled to VDD).

## `ETH-2` RGMII delay missing or doubled <a id="eth-2"></a>

⚠️ warning · needs: PHY delay straps (library), MAC delay setting (project), clock trace delay · [catalogue entry](../checks.md#eth-2)

**Impl.** hard · depends on NET-7, CFG-2

**What it catches.** An RGMII link where nobody adds the ~2 ns clock-to-data skew (no PHY internal delay, no MAC
delay, no long clock trace), or where two of them do.

**Why it matters.** RGMII transmits clock and data edge-aligned at the source; the receiver needs the clock
shifted by about 1.5-2 ns to sample in the middle of the data eye (RGMII 2.0 §3.3, Table 2: TskewR 1-2.6 ns at the
receiver). Version 2.0 allows the delay inside the transmitter ("RGMII-ID"). With no delay the receiver samples on
the edge; with two delays (PHY strap and MAC both on) it samples 4 ns late, which at 125 MHz is the next bit's
edge. Either way the link comes up and drops packets or only works at 100 Mbit/s, where the timing is ten times
looser.

**Example.** A DP83867 strapped for RX and TX clock shift (Table 7-9) on an SoC whose devicetree says
`phy-mode = "rgmii-id"`: Linux asks the PHY driver for internal delay too, or the MAC adds its own, and the delays
add. The Linux binding says outright that these modes "are frequently used wrongly".

**How to check.** Decode the PHY's delay straps (pins, levels, `NET-7`); take the MAC side and any PCB clock delay
from the project (`CFG-2`); exactly one of them must provide the skew in each direction.

**Fix.** Decide once per direction who adds the delay, write it on the schematic next to the PHY, and make the
straps, the MAC setting and the devicetree `phy-mode` agree.

**References.** RGMII 2.0 §3.3 and Table 2 (TskewR, note 1: clock trace delay 1.5-2.0 ns; note 4: RGMII-ID);
DP83867 §6.9 note 2, §7.4.1.1, Table 7-9 (RGMII clock skew straps); KSZ9031 features (on-chip delay per RGMII
v2.0); Linux ethernet-controller binding (`rgmii`, `rgmii-id`, `rgmii-rxid`, `rgmii-txid`).

## `ETH-3` MDIO without a pull-up <a id="eth-3"></a>

🛑 error · needs: the MDIO net (pin names), resistors to rails · [catalogue entry](../checks.md#eth-3)

**Impl.** easy · depends on NET-5

**What it catches.** An MDIO net with no resistor to a rail.

**Why it matters.** MDIO (IEEE 802.3 Clause 22) is a shared bidirectional line that both the MAC and the PHY
release during the turnaround and between frames; the idle state and the 32-bit preamble of ones come from the
pull-up alone. Without it the line floats during turnaround, reads garbage, and some PHYs ignore frames whose
preamble is not clean. The DP83848 asks for 1.5 kΩ; the KSZ9031 says the pin "requires an external pull-up" and
specifies its fastest MDC with 1.0 kΩ.

**Example.** An STM32H7 to KSZ9031 board relying on the MCU's internal ~40 kΩ pull-up: register reads return
0xFFFF on one boot and plausible values on the next, and the link-up interrupt works only because the firmware
retries.

**How to check.** Find the net on a pin named MDIO (PHY or MAC side) and look for a resistor to a rail (`NET-5`);
value outside 1-10 kΩ is worth a note.

**Fix.** One 1.5 kΩ (1-4.7 kΩ) pull-up to the PHY's IO rail per MDIO bus.

**References.** DP83848C §4.3 and §6.4.3 (1.5 kΩ pull-up); KSZ9031 pin 37 description, §3.10 (MIIM, Clause 22.2.4);
DP83867 §6.6 (MDIO pulled high for the 32-bit preamble).

## `ETH-4` PHY address straps unset, duplicate or 0 <a id="eth-4"></a>

⚠️ warning · needs: PHY address straps (library pin data, levels), MDIO bus membership · [catalogue entry](../checks.md#eth-4)

**Impl.** mid · depends on NET-7

**What it catches.** A PHY whose address straps are left to the internal pulls without saying so, two PHYs on one
MDIO bus with the same address, or address 0 on a PHY that gives 0 a special meaning.

**Why it matters.** Every PHY on an MDIO bus answers to its strapped address; two at the same address drive MDIO
together and the driver configures both as one. Address 0 differs by family: on the DP83848 a strapped 0 puts the
part into MII isolate mode, so it links on the cable but passes no data to the MAC; on the KSZ9031 0 is an ordinary
address and explicitly *not* broadcast; other PHYs answer 0 as a broadcast in addition to their own address. The
straps sit on data pins (RXD, COL, LED), so whatever else hangs on those pins at reset also sets the address.

**Example.** Two DP83848 on one STM32 MDIO, both with straps left at their defaults (PHYAD0 internal pull-up,
PHYAD[4:1] pull-downs: address 1): both answer, reads are the wired combination of two PHYs.

**How to check.** Resolve each PHY's address from its strap pins, the external resistors and the internal pulls
(the same machinery as I2C addresses, `NET-11`, on `NET-7` pin data); then look for duplicates per MDIO net and
for 0 on families where it is special.

**Fix.** Strap every PHY explicitly with 2.2 kΩ resistors (SNLA079 §7), give each a distinct non-zero address,
write the address on the schematic.

**References.** DP83848C §4.9 (PHYAD straps, address 0 = MII isolate); KSZ9031 §3.10 (PHYAD[2:0], 0h not
broadcast); DP83867 §7.5.4 (PHY address configuration); SNLA079 §7.

## `ETH-5` Unused pairs and centre taps without Bob Smith termination <a id="eth-5"></a>

💡 suggestion · needs: magjack / magnetics cable-side pins, chassis net · [catalogue entry](../checks.md#eth-5)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** Cable-side centre taps and unused pairs (4/5 and 7/8 on 10/100) left open, or terminated
wrongly: returned to signal ground instead of chassis, placed on the PHY side of the transformer, or through a
capacitor rated below 2 kV.

**Why it matters.** The Bob Smith network (75 Ω per tap or pair into one 1 nF capacitor to chassis) gives the
cable's common-mode currents a defined path to the shield and damps the resonance of unused pairs; without it
they radiate and pick up noise (Ott, common-mode cable radiation). Returned to signal ground it carries that noise
into the logic and defeats the transformer's isolation; a capacitor below 2 kV fails the 1500 V isolation test.
TI notes it does not apply to PoE designs, where the pairs carry power.

**Example.** A 10/100 board with discrete Pulse H1102 magnetics: RJ45 pins 4/5 and 7/8 open, cable-side taps
unconnected. It passes bring-up and fails radiated emissions around 100 MHz.

**How to check.** Find the magnetics' cable-side tap pins and the jack's unused pair pins (`NET-7`); they should
reach a chassis net (`NET-5`, named chassis/shield) through ~75 Ω and a capacitor; flag a path to signal ground, a
network on the PHY side, or a capacitor whose rating (from the value or MPN) is under 2 kV.

**Fix.** 75 Ω per tap and pair into a 1 nF 2 kV capacitor to chassis ground; keep chassis and circuit ground
separate except at the defined tie.

**References.** SNLA079 §2.3 (Bob Smith: 75 Ω, 1000 pF, at least 2 kV; not for PoE), §2.4 (EMI, chassis ground);
DP83867 §9.2.1.1 (Figure 9-2: 75 Ω, 1 nF 2 kV); Ott, *Electromagnetic Compatibility Engineering*.

## `ETH-6` RMII REF_CLK direction wrong <a id="eth-6"></a>

🛑 error · needs: REF_CLK pins of MAC and PHY, PHY clock-mode strap, oscillators · [catalogue entry](../checks.md#eth-6)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** An RMII link where both the MAC and the PHY are set to output the 50 MHz reference clock, or
neither is and no oscillator feeds the net.

**Why it matters.** RMII times everything, both directions, from one 50 MHz ±50 ppm clock that comes from the MAC,
the PHY or a separate oscillator (SNLA076 §3.1). Two outputs on the net fight; no source means no Ethernet at all.
Several PHYs choose the direction with a strap that shares a pin with something else, so the direction is easy to
get wrong without anyone drawing it.

**Example.** STM32F4 with a LAN8720A and a 25 MHz crystal on the PHY. The design intends REF_CLK Out mode (the
PHY makes 50 MHz from the crystal), which needs nINTSEL strapped low; the strap is left to its internal pull-up, so
the PHY is in REF_CLK In mode and waits for a 50 MHz clock that nobody drives. Its pin is nINT, an interrupt
output, wired to the MCU's ETH_RMII_REF_CLK.

**How to check.** Find the REF_CLK net from the MAC's alternate function (`NET-12`) and the PHY's pin; decode the
PHY's clock-mode strap (`NET-7`); count sources on the net (PHY out, MAC out, oscillator): exactly one.

**Fix.** Pick one source, strap the PHY accordingly (LAN8720A: nINTSEL low for REF_CLK Out), and write the
direction on the schematic.

**References.** SNLA076 §3.1 (REF_CLK 50 MHz ±50 ppm, sourced by MAC or external); LAN8720A §3.7.4 and Table 3.6
(nINTSEL, REF_CLK In / Out mode), §5.3.1-5.3.2; Wikipedia, Media-independent interface "Reduced
media-independent interface".

## `ETH-7` PHY LED pins that double as straps <a id="eth-7"></a>

🛑 error · needs: PHY pin roles (LED and strap on one pin), parts on the pin · [catalogue entry](../checks.md#eth-7)

**Impl.** mid · depends on NET-7

**What it catches.** An LED (with its resistor) on a PHY pin that is also a configuration strap, with no strap
resistor that dominates it, or wired with a polarity that sets an unintended mode.

**Why it matters.** At reset the PHY reads the pin's level as a strap; afterwards it drives the LED and picks the
LED polarity from what it read. An LED and its resistor to a rail or to ground load the strap, so the latched value
depends on the diode's forward voltage against the internal pull. On the LAN8720A, LED1 is REGOFF: read high, the
internal 1.2 V regulator is switched off and the PHY has no core supply; LED2 is nINTSEL: read low, the PHY starts
driving a 50 MHz REF_CLK (`ETH-6`).

**Example.** LAN8720A with both LEDs wired from the pins to 3.3 V (active-low, the usual MCU habit) and no strap
resistors: LED1 can latch REGOFF = 1 and the board has no Ethernet until the LED is lifted. The datasheet's own
note: a strap pin connected to a load "must be augmented with an external resistor".

**How to check.** From the PHY's pin data (`NET-7`) find pins that are both LED and strap; list the parts on each
pin and the level they set; require a strap resistor and an LED polarity that matches the intended strap value.

**Fix.** A 2.2 kΩ (or the datasheet's value) strap resistor on every LED/strap pin, the LED wired for the polarity
that strap gives, both written on the schematic.

**References.** LAN8720A Note 2.2, §3.7.3 (REGOFF), §3.8.1.1 (REGOFF and LED1 polarity), §3.8.1.2 (nINTSEL and
LED2 polarity); SNLA079 §7 (LED and non-LED strap pins); DP83867 §7.5.2 (LED configuration), Table 7-6 note 3.

## `ETH-8` MDI pair polarity or order against the jack <a id="eth-8"></a>

⚠️ warning · needs: PHY MDI pins, magnetics internal mapping, jack pinout · [catalogue entry](../checks.md#eth-8)

**Impl.** hard · depends on NET-10, NET-7

**What it catches.** MDI pairs that reach the RJ45 on the wrong pins, in the wrong pair order, or with P and N
swapped, after following them through the magnetics.

**Why it matters.** Auto MDI-X fixes the classic crossover (pairs A and B swapped), and many PHYs correct the
polarity of 10BASE-T link pulses automatically, but neither covers every case: a 10/100 PHY with MDI-X disabled,
a pair landing on 4/5 or 7/8, or a split pair (one pair's P and N on two different twisted pairs of the cable) does
not link at all or links only on short cables. Magjacks hide the mapping inside the part, and pin-compatible
looking magjacks differ.

**Example.** On B125A the PoE and non-PoE magjack versions turned out to have incompatible pinouts (assembly log,
22 Apr 2024): a footprint shared by two magjack MPNs only works for one of them.

**How to check.** Follow each PHY TX/RX pair through the transformer to the jack pins (`NET-10`, with the
magnetics' winding map in the part data `NET-7`); compare to the standard pair assignment (1/2, 3/6, 4/5, 7/8);
report pairs swapped (fine with Auto MDI-X), P/N swapped (fine only where the PHY corrects it) and split pairs
(never fine).

**Fix.** Route pairs to the standard pins; check every alternate magjack MPN against the footprint pin by pin.

**References.** DP83848C §6.5.1.3.7 (automatic 10BASE-T link polarity correction); LAN8720A §3.3 (HP Auto-MDIX);
Wikipedia, Medium-dependent interface "Automatic MDI-X"; B125A assembly log.

## `ETH-9` PHY crystal tolerance above ±50 ppm <a id="eth-9"></a>

⚠️ warning · needs: crystal / oscillator ppm (value field or MPN), PHY clock pins · [catalogue entry](../checks.md#eth-9)

**Impl.** mid · depends on CFG-5, NET-7

**What it catches.** A crystal or oscillator on a PHY's clock input (or on the RMII REF_CLK) whose frequency
tolerance plus stability is looser than ±50 ppm.

**Why it matters.** The PHY receives data clocked by the far end's crystal and hands it over in its own clock
domain through an elasticity buffer; the buffer is sized for both ends within ±50 ppm. At ±100 ppm the longest
frame that crosses without overrun or underrun halves (SNLA076 Table 5: 2400 bytes at ±50 ppm, 1200 at ±100 ppm
with the shortest buffer), so full-size and jumbo frames get dropped with CRC errors against a partner at the
other end of the tolerance, and nothing fails on the bench with two boards of the same batch.

**Example.** B125A uses a 50 MHz ±50 ppm oscillator for its RMII LAN8740A, inside the budget. A cheap 25 MHz
crystal "±30 ppm tolerance, ±50 ppm over temperature" adds up to ±80 ppm before ageing.

**How to check.** Find the crystal or oscillator on the PHY clock pins or the REF_CLK net (`NET-7`), take
tolerance, stability and ageing from the value or MPN (part data, `CFG-5`), and compare their sum to ±50 ppm.

**Fix.** A crystal or oscillator specified at ±50 ppm total over temperature and ageing, with the load
capacitance the PHY datasheet asks for.

**References.** SNLA079 §10.1, Tables 1-2 (±50 ppm crystal and oscillator); SNLA076 §4.3, Table 5 (elasticity
buffer packet size at ±50 / ±100 ppm); LAN8720A Table 5.13 (crystal specifications), CLKIN ±50 ppm.

## `ETH-10` PoE PD without isolation or classification resistor <a id="eth-10"></a>

🛑 error · needs: PD controller pins, converter topology, grounds and other ports · [catalogue entry](../checks.md#eth-10)

**Impl.** hard · depends on NET-5, NET-7

**What it catches.** A PoE powered device whose PD side is not isolated from everything a user or another cable
can touch, or whose PD controller has no classification resistor (or no detection resistor).

**Why it matters.** IEEE 802.3 Clause 33 keeps the cable floating: the PSE's up to 57 V and the cable pairs must stay
isolated from ground and every other system potential, tested at 1500 Vac. A PD converter that ties the cable side
to the board ground is only acceptable when nothing else on the board reaches the outside; with a USB port, a
debug header or a metal case it makes a ground loop through the PSE or a shock path. Without the class resistor
the PSE sees class 0 and budgets 13 W, or a wrong resistor claims too little power and the PSE cuts the PD under
load; without the 24.9 kΩ detection signature the PSE never turns power on.

**Example.** A TPS2378 PD followed by a non-isolated buck on a board that also has a USB-C port and two isolated
CAN channels: the USB host's ground is now tied to a PoE pair. Same board with RCLS left out: a 25 W design
negotiates class 0 and the PSE allows 13 W.

**How to check.** Find the PD controller's CLS and DEN pins (`NET-7`) and their resistors to VSS / VDD; check the
values against the class table. For isolation, check that the PD-side ground (`NET-5`) reaches the board ground
only through a transformer or a rated isolation barrier, or that the board has no other external port.

**Fix.** RDEN 24.9 kΩ, RCLS from the class table, and an isolated converter (flyback, active clamp forward) unless
the product is fully enclosed with no other port.

**References.** TPS2378 §7.3.3 and Table 1 (class resistor), §7.3.4 (DEN 24.9 kΩ), §7.4.1 (PoE overview, IEEE
802.3at / Clause 33), §7.4.11 (cable isolated from ground, 1500 Vac); IEEE 802.3 Clause 33; Wikipedia, Power over
Ethernet.

## `ETH-11` RGMII / RMII lines without series resistors <a id="eth-11"></a>

💡 suggestion · needs: MAC interface pins (outputs), series parts · [catalogue entry](../checks.md#eth-11)

**Impl.** mid · depends on NET-7

**What it catches.** RMII, MII or RGMII outputs (clocks and data, both directions) connected straight from driver
to receiver where the PHY's reference design puts a series resistor at the source.

**Why it matters.** These are fast CMOS edges on traces of several centimetres; without source termination they
ring and overshoot, which costs timing margin on the clock and radiates. TI recommends 50 Ω series termination on
all MII outputs, including the clocks and the receive data. PHYs with integrated output termination do not need
them, hence a suggestion, not a warning.

**Example.** An STM32H7 RMII bus to a LAN8742A routed over 8 cm with no series parts: the 50 MHz REF_CLK and
the data lines ring well past the 3.3 V rail on every edge, and their harmonics show up in the emissions scan;
22-33 Ω at each driver, close to the pin, damps both.

**How to check.** Identify the interface's output pins on both sides (`NET-7`) and look for a resistor in series
close to the driver; skip PHYs known to terminate internally.

**Fix.** 22-50 Ω series resistors at each output (the PHY's reference design value), placed at the driver; 0 Ω
footprints if unsure.

**References.** SNLA079 §5.1 (50 Ω series termination on MII outputs), §5.2 (trace length); Johnson & Graham,
*High-Speed Digital Design* (Prentice Hall, 1993), chapter 6 (terminations).

## `ETH-12` PHY reset released before its clock is stable <a id="eth-12"></a>

⚠️ warning · needs: PHY reset pin, its pulls and driver, PHY clock source and its enable · [catalogue entry](../checks.md#eth-12)

**Impl.** hard · depends on NET-8, NET-10

**What it catches.** A PHY reset that can rise before the PHY's clock runs (clock from an MCU output or an
oscillator with its own enable), or a reset pin with no pull at all.

**Why it matters.** The PHY samples its straps and starts its PLL when reset is released; without a clock it
latches garbage or never starts. The LAN8720A needs a hardware reset after power-up, held for at least 100 µs,
with the clock already present on XTAL1/CLKIN; the DP83867 wants XI stable for at least 1 µs during the reset
pulse. A floating reset follows whatever the MCU pin does in its own reset and boot.

**Example.** LAN8720A clocked from an STM32's MCO output, nRST wired to a GPIO with no pull-down: the PHY comes
out of power-on reset with no clock, MCO starts only when firmware configures it, and link works only after a
soft reset of the PHY that the firmware happens to do.

**How to check.** Find the PHY's reset pin: a pull must exist (easy). Then follow the clock source back
(`NET-10`): if it is an MCU output or a gated oscillator, the reset must be held low until firmware or a
supervisor releases it (`NET-8` power states and start-up order).

**Fix.** Pull the PHY reset low and release it from firmware after the clock runs, or use a free-running crystal
on the PHY; add an RC or supervisor if no GPIO is free.

**References.** LAN8720A §3.8.5.1 (hardware reset required after power-up, clock during reset), §5.5.3 (trstia
100 µs); DP83867 §6.6 (power-up timing), §6.7 (reset timing, T4: XI stable during reset); SNLA079 §11.1.

## `ETH-13` Magnetics not rated for the design <a id="eth-13"></a>

⚠️ warning · needs: magnetics / magjack ratings (MPN data), PoE presence · [catalogue entry](../checks.md#eth-13)

**Impl.** mid · depends on CFG-5

**What it catches.** Magnetics or a magjack with an isolation rating below 1500 Vrms, or not specified for PoE on
a board that carries PoE current through the centre taps.

**Why it matters.** The 1500 V isolation is what separates a building's cabling from the board; below it the
product fails the hipot test of its safety approval. PoE puts up to several hundred milliamps of DC through each
centre tap; magnetics not designed for it saturate when the two halves of a winding carry unequal current, and
the data link degrades exactly when the device draws power.

**Example.** A PoE design built on Pulse H1102, the part SNLA079 Table 5 lists for typical applications, where
the same table points to H2019 for PoE applications.

**How to check.** Take the isolation voltage and PoE rating of the magnetics or magjack from the part data
(`CFG-5`, MPN); require ≥ 1500 Vrms and PoE rating when a PD or PSE controller is on the board.

**Fix.** PoE-rated magnetics (or magjack) with 1500 Vrms isolation, matched to the PHY type (`ETH-1`).

**References.** SNLA079 §10.2, Table 4 (isolation 1500 Vrms HPOT), Table 5 (H1102 typical, H2019 PoE); TPS2378
§7.4.11 (1500 Vac); Wikipedia, Power over Ethernet.

## `ETH-14` PHY multi-level strap divider computed wrongly <a id="eth-14"></a>

⚠️ warning · needs: strap divider values, the pin's internal pull and level table (library) · [catalogue entry](../checks.md#eth-14)

**Impl.** mid · depends on NET-7, IMP-6

**What it catches.** A divider on a multi-level strap pin computed without the pin's internal pull, built with 5 %
resistors, or with a capacitor straight on the strap.

**Why it matters.** Multi-level straps (DP83867: four levels per pin) read a voltage window, and the windows are
narrow. The pin has an internal pull of its own (DP83867: 9 kΩ ±25 % to ground) in parallel with the external
low-side resistor; a divider computed without it lands between two windows, and with ±25 % on the internal part
the level moves across the boundary from board to board. A capacitor on the strap delays the voltage past the
moment it is latched.

**Example.** DP83867 strap aimed at mode 3 (0.225-0.284 × VDDIO) with 10 kΩ / 3.3 kΩ, which alone gives 0.248.
With the internal 9 kΩ in parallel the low side is 2.41 kΩ and the ratio 0.195: just above mode 2's 0.191 limit
and below mode 3. With the internal pull at its −25 % end it reads mode 2, at +25 % it reads nothing defined.
TI's table gives 5.76 kΩ / 2.49 kΩ for mode 3.

**How to check.** Compute each strap divider with the internal pull and its tolerance and the resistor
tolerances (`IMP-6` values, `NET-7` for the pull and level table); every corner must stay inside one window. Flag
a capacitor on the strap net.

**Fix.** Use the datasheet's recommended pairs with 1 % resistors, no capacitor on strap pins.

**References.** DP83867 §7.5.1, Table 7-5 (4-level strap ratios, 9 kΩ ±25 % internal pull-down, 1 % resistors),
§6.6-6.7 (strap latch-in timing).
