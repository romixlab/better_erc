# LVL checks, explained

Detail page for the `LVL` category of the [check catalogue](../checks.md#-lvl---logic-levels-and-io-banks). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find
it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to build in better_erc without an LLM: `easy` (the netlist alone), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet) or `judgement` (a human or an LLM
decides, better_erc lists candidates), followed by the [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **SCEA035A** · TI, *Selecting the Right Level-Translation Solution* (2004). Translator families, why a pull-up on a
  CMOS output is no translator (§3.2), overvoltage-tolerant inputs (§5), TTL-threshold inputs (§6).
  [ti.com/lit/an/scea035a](https://www.ti.com/lit/an/scea035a/scea035a.pdf)
- **SCEA044** · TI, *A Guide to Voltage Translation With TXS-Type Translators* (2010). Driver requirements (§3),
  external loads and the one-shot (§4), output enable (§5).
  [ti.com/lit/an/scea044](https://www.ti.com/lit/an/scea044/scea044.pdf)
- **SCEA043** · TI, *A Guide to Voltage Translation With TXB-Type Translators* (2010), the push-pull sibling: external
  loads (§3), output enable (§4). [ti.com/lit/an/scea043](https://www.ti.com/lit/an/scea043/scea043.pdf)
- **AN10441** · NXP, *Level shifting techniques in I²C-bus design*, Rev. 2 (2020). The BSS138-pair shifter (§2).
  [mouser.com/pdfDocs/AN10441.pdf](https://www.mouser.com/pdfDocs/AN10441.pdf) (the nxp.com copy answers 404 today)
- **UM10204** · NXP, *I²C-bus specification and user manual*, Rev. 7.0 (2021). Table 10 (VIL 0.3·VDD, VIH 0.7·VDD, note
  1 on legacy fixed levels).
  [nxp.com/docs/en/user-guide/UM10204.pdf](https://www.nxp.com/docs/en/user-guide/UM10204.pdf)
- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, Rev 4 (2026).
  Five-volt tolerant pins (§5.2.2), cross-domain leakage (§6.2), no tolerance without VDD (§6.3).
  [st.com/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **TXS0102** · TI datasheet SCES640L (2026): internal 10 kΩ pull-ups (§7.3.5), OE pull-down and power-up (§8.3).
  [ti.com/lit/ds/symlink/txs0102.pdf](https://www.ti.com/lit/ds/symlink/txs0102.pdf)
- **SN74LVC2T45** · TI datasheet SCES516N (2024): DIR referenced to VCCA, application information (§8.1).
  [ti.com/lit/ds/symlink/sn74lvc2t45.pdf](https://www.ti.com/lit/ds/symlink/sn74lvc2t45.pdf)
- **ADuM1201** · Analog Devices, *ADuM1200/ADuM1201 Dual-Channel Digital Isolators*, Rev. L (2025): pin function tables
  15 and 16 (fixed channel directions).
  [analog.com/ADuM1200_1201.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/ADuM1200_1201.pdf)
- **Wikipedia, Logic level** and **Level shifter** · readable overviews.
  [en.wikipedia.org/wiki/Logic_level](https://en.wikipedia.org/wiki/Logic_level),
  [en.wikipedia.org/wiki/Level_shifter](https://en.wikipedia.org/wiki/Level_shifter)
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 12 (logic interfacing).

## `LVL-1` Pull-up, divider or other circuitry to a different voltage level <a id="lvl-1"></a>

🛑 error · needs: IO bank voltage per pin (easy for single-supply parts) · [catalogue entry](../checks.md#lvl-1)

**Impl.** mid · depends on NET-5, NET-7, IMP-10

**What it catches.** A pull-up, pull-down divider, LED or other passive network on a pin that ends on a rail other than
the one that powers that pin's IO bank: a 3.3 V pull-up on a 1.8 V pin, a divider from 5 V into a 3.3 V ADC input, a
pull-up to the far side's rail on the near side of a translator.

**Why it matters.** The pin's ESD diode to its own supply clamps the higher level, so current flows from the higher rail
through the resistor into the lower rail. That back-powers the lower domain when it is off, lifts it when it is lightly
loaded, and puts the pin above its absolute maximum. A pull-up on a push-pull output to a higher rail also drives
current back through the output's P-channel FET whenever the output is high (SCEA035A §3.2). The level itself "works" on
the bench, which is why it survives review.

**Example.** A LAN8720A PHY run with VDDIO = 1.8 V, its `nRST` pulled up with 10 kΩ to `+3V3` because the reference
schematic used a 3.3 V VDDIO: 150 µA flow into the 1.8 V rail through the input clamp, and the pin sits above VDDIO +
0.3 V for its whole life.

**How to check.** For every resistor (or chain, `NET-2`) from a pin to a rail, compare the rail voltage (`NET-5`) with
the supply of that pin's bank. For a single-supply part the bank is the one VDD net; for multi-bank parts (FPGAs, SoCs,
MCUs with VDDIO2) the bank of each pin must come from part data (`NET-7`, `IMP-10`).

**Fix.** Pull to the pin's own bank rail; across domains use a translator, an open-drain buffer with the pull-up on the
receiving side, or a divider sized for the lower side.

**References.** SCEA035A §3.2; AN4899 §6.2; Wikipedia, Logic level.

## `LVL-2` 1V8 output to 3V3 input and similar: compare thresholds <a id="lvl-2"></a>

🛑 error · needs: output VOH and input VIH per pin, bank voltages · [catalogue entry](../checks.md#lvl-2)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A push-pull output from a lower-voltage domain driving an input of a higher-voltage domain whose
VIH is above the driver's VOH(min).

**Why it matters.** CMOS inputs typically switch at a fraction of their own supply: 0.7 · VDD for VIH on many parts,
2.31 V at 3.3 V. A 1.8 V output never gets there; the input sits in the undefined region, reads randomly, and draws
shoot-through current (both input FETs half on). Some parts pass on the bench because their real threshold is near
VDD/2, and fail on the next lot or at temperature.

**Example.** A 1.8 V SoC's UART TX into a 3.3 V USB-UART bridge whose RXD needs VIH = 2.0 V: characters drop at cold and
on some boards only. A 3.3 V MCU into a 5 V 74HC input (VIH 3.5 V at 5 V) is the same mistake one level up; a 74HCT
input (TTL thresholds, VIH 2.0 V) would accept it (SCEA035A §6).

**How to check.** For each net with one driver and one or more inputs in different domains, compare VOH(min) of the
driver at its bank voltage with VIH(min) of each input at its own (`NET-7`). Without part data, fall back on the 0.7 ·
VDD rule and report it as an estimate.

**Fix.** A translator (LVC1T45 class for one direction), a receiver with TTL-compatible or fixed thresholds (74HCT,
74LVC at the higher supply with 5 V tolerant inputs driven from 3.3 V), or move the signal to a bank at the right
voltage (`LVL-6`).

**References.** SCEA035A §2 and §6; Wikipedia, Logic level; AoE3 chapter 12.

## `LVL-3` I2C thresholds may allow skipping a translator <a id="lvl-3"></a>

💡 suggestion · needs: VIH of every device on the bus, bus voltage · [catalogue entry](../checks.md#lvl-3)

**Impl.** mid · depends on NET-4, NET-7

**What it catches.** An I2C translator between two domains where every device on the higher side would accept the lower
side's levels, so one bus at the lower voltage would do.

**Why it matters.** Translators cost a part, board space and a failure mode (`LVL-4`, `LVL-9`, `LVL-11`). Plain I2C
inputs use VIH = 0.7 · VDD, which a 1.8 V pull-up never meets on a 3.3 V device, but many parts state a fixed VIH
(SMBus-style 1.35 V, or "1.26 V independent of VDD" in sensors with a separate VDDIO) or have a VDDIO pin that can sit
on the 1.8 V rail. With those, the translator is dead weight.

**Example.** A 1.8 V SoC talks to a 3.3 V PMIC whose SDA/SCL are specified with VIH(min) = 1.3 V and an open-drain
output: pull the bus up to 1.8 V and drop the PCA9306. Before doing so, check every other device on that segment: one
plain 0.7 · VDD device at 3.3 V keeps the translator.

**How to check.** For each I2C translator found by `NET-4`, read VIH of every device on the high side (`NET-7`) and
compare with the low-side pull-up voltage minus margin; also check that no high-side device needs its pull-up at its own
VDD for some other reason (a 5 V tolerance limit on the low side is then irrelevant, as the bus never exceeds the low
rail).

**Fix.** Remove the translator, pull the whole bus to the lower rail, and note the threshold assumption in the schematic
so a later part swap does not break it.

**References.** UM10204 Table 10 (VIH, note 1 on fixed levels); SMBus thresholds as in [I2C-12](i2c.md#i2c-12); AN10441
§1.

## `LVL-4` I2C discrete translator pull-up to the wrong voltage <a id="lvl-4"></a>

🛑 error · needs: the translator (NET-4), rails of the devices on each side · [catalogue entry](../checks.md#lvl-4)

**Impl.** easy · depends on NET-4, NET-5

**What it catches.** In a BSS138-pair shifter, the pull-ups of one side going to the other side's rail: both pairs to
the high rail, or the low side pulled to the high rail.

**Why it matters.** The pass-FET shifter works because each side has its own pull-up to its own rail (AN10441 §2). A
low-side pull-up to 3.3 V drives 3.3 V into the 1.8 V devices' pins through their clamps (`LVL-1`); a high-side pull-up
to 1.8 V gives a high level of 1.8 V on the 3.3 V or 5 V side, below its VIH.

**Example.** A schematic copied from a 3.3 V/5 V design onto a 1.8 V/3.3 V board: the "LV" pull-ups R1/R2 still go to
`+3V3`, so the 1.8 V sensor's SDA sits at 3.3 V whenever the bus is idle.

**How to check.** For each discrete shifter that `NET-4` recognises, the pull-ups on the source-side nets must go to the
same rail as the gate (`LVL-9`) and to the supply of the devices on that side; the drain-side pull-ups to the supply of
the devices there (`NET-5`).

**Fix.** Each side's pull-up to that side's IO rail; or replace the pair with a PCA9306 / TCA9406 class part whose pins
name the sides.

**References.** AN10441 §2; [I2C-1](i2c.md#i2c-1); UM10204 §7.1.

## `LVL-5` Bidirectional level shifter used for unidirectional signals <a id="lvl-5"></a>

⚠️ warning · needs: translator class, signal direction per pin · [catalogue entry](../checks.md#lvl-5)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** An auto-direction translator (TXS, TXB, BSS138 pair) carrying signals that only ever go one way:
UART TX/RX, SPI, an enable, a boot-mode strap.

**Why it matters.** Auto-direction parts guess the direction from which side moves first. They need strong push-pull
drivers and hold a static level only weakly: a TXB through a 4 kΩ output buffer, so any pull-up or pull-down well below
50 kΩ forms a divider with it (SCEA043 §3); a TXS only through its 10 kΩ pull-up after the one-shot (SCEA044 §4). A
strap pin behind one is not at a defined level until both rails and OE are up. A fixed-direction translator has none of
these problems.

**Example.** A USART plus a BOOT0 strap to a target processor through one TXB0104: the 10 kΩ BOOT0 pull-down on the
target holds the TXB's B side, the output never quite reaches the high level, and the boot mode is random.

**How to check.** For each auto-direction translator, look up what drives each channel on both sides (`NET-7`, pin
functions from `NET-12`); a channel whose two ends are an output and an input, or that carries a strap or has a resistor
to a rail, is unidirectional.

**Fix.** A fixed-direction translator (TXU0204 class with per-channel directions, or 2T45/1T45 with DIR tied), keeping
the auto-direction part only for true bidirectional lines (I2C, 1-Wire, SDIO data).

**References.** SCEA043 §2-3; SCEA044 §3-4; SCEA035A §2.

## `LVL-6` A better suited bank exists for the net voltage or target bank voltage <a id="lvl-6"></a>

💡 suggestion · needs: bank of every pin, pin-swap freedom, net voltages · [catalogue entry](../checks.md#lvl-6)

**Impl.** hard · depends on NET-7, NET-12, IMP-10

**What it catches.** A signal routed to an IO bank at the wrong voltage (and so through a translator or a divider) while
a free, function-compatible pin exists on a bank already at the right voltage.

**Why it matters.** FPGAs, CPLDs/SPLDs and SoCs often have banks at 1.8 V and 3.3 V on the same package side. Moving a
signal to the matching bank removes a translator, its delay and its failure modes, and frees space.

**Example.** On an SPLD the `+3V3` side is used for a 1.8 V sensor interface through a TXS0104, while the `+1V8` bank on
the same side as the connector has free pins: move the four lines and drop the translator.

**How to check.** For every translated or divided signal, list free pins on banks at the signal's voltage that can carry
its function (`NET-12`, bank data from `IMP-10`), preferring the same package side.

**Fix.** Re-assign the pins (and re-check `MUX-1`/`MUX-2` for the new ones), then delete the translator.

**References.** SCEA035A §1-2 (cost of translation); vendor bank tables of the part.

## `LVL-7` Several power rails on a part and no banks defined: warn <a id="lvl-7"></a>

⚠️ warning · needs: rails on each part's power pins · [catalogue entry](../checks.md#lvl-7)

**Impl.** easy · depends on NET-5

**What it catches.** A part whose supply pins land on two or more different rails (VDD and VDDIO, VCCA and VCCB, VDDIO2)
while the library gives no bank membership for its signal pins.

**Why it matters.** Every level check in this category needs to know which supply a pin's thresholds follow. Without
banks better_erc would have to guess, and the guess is wrong exactly on the parts where it matters: level translators,
MCUs with a separate VDDIO2, sensors with a VDDIO.

**Example.** A TXS0102 symbol with VCCA on `+1V8` and VCCB on `+3V3` but no A/B side on its pins: `LVL-1` and `LVL-2`
cannot tell which side SDA_A is on.

**How to check.** Count distinct rails (`NET-5`) on the power pins of each part; with more than one and no bank data for
the part, report it, and report the level checks on its pins as not run (`DIAG-2`).

**Fix.** Add bank data to the library part (a pin property, a units-per-bank symbol, or a pin file via `IMP-10`).

**References.** AN4899 §5.2.2 (pins supplied from several rails); SN74LVC2T45 (VCCA/VCCB referenced pins).

## `LVL-8` Signal not referenced to GND <a id="lvl-8"></a>

🛑 error · needs: each part's ground reference, rails · [catalogue entry](../checks.md#lvl-8)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A logic signal crossing between parts whose ground pins are on different nets: an inverting
buck-boost (a buck IC whose ground pin sits on the negative output), a high-side gate driver referenced to the switch
node, a part on an isolated ground.

**Why it matters.** A logic level is a voltage against the receiver's own ground. A buck IC used as an inverting
buck-boost has its GND pin at −Vout, so an EN driven by a 3.3 V MCU against board ground arrives as 3.3 V + |Vout| at
the IC: it may exceed EN's absolute maximum, and "low" (0 V) is still |Vout| above the IC's ground, above its VIH, so
the converter can never be turned off.

**Example.** A buck regulator wired as an inverting buck-boost to make −5 V: its GND pin on `-5V`, EN from an MCU GPIO.
With the GPIO low, EN sees +5 V against the IC's ground and the rail stays on; with it high, EN sees 8.3 V.

**How to check.** For every net that connects pins of two parts, compare the nets on their ground pins (`NET-5`, ground
pins from `NET-7`); report when they differ and no level-shifting part (isolator, PNP/NPN shifter, optocoupler) sits
between.

**Fix.** A level shift referenced to the IC's ground (a PNP or a small FET pair, an optocoupler), or a part with an
enable referenced to board ground.

**References.** The regulator's datasheet section on inverting operation; AoE3 chapter 12.

## `LVL-9` Discrete translator with a gate reference that gives the wrong threshold for one side <a id="lvl-9"></a>

🛑 error · needs: the translator (NET-4), rails · [catalogue entry](../checks.md#lvl-9)

**Impl.** easy · depends on NET-4, NET-5

**What it catches.** A BSS138-pair shifter whose FET gates go to the higher rail (or to anything but the lower side's
rail).

**Why it matters.** The gate sets where the FET turns off: with the gate at the lower rail, the source (low side) can
rise to about Vgate − Vth before the FET opens, which is the low side's own pull-up level. Gate at the high rail and the
FET never opens: the low side follows the high side up to the high rail minus Vth, well above its supply (AN10441 §2
describes the gate on the lower supply).

**Example.** Gates tied to `+5V` in a 3.3 V/5 V shifter: the 3.3 V side is pulled to about 3.5-4 V through the FETs
whenever the bus is idle, through the 3.3 V devices' clamps.

**How to check.** The gate net of each FET in a recognised shifter must be the rail of the source-side pull-ups and the
lower of the two rails (`NET-5`).

**Fix.** Gate to the lower rail, sources on the lower side, drains on the higher side.

**References.** AN10441 §2; Wikipedia, Level shifter.

## `LVL-10` Higher-voltage output into a lower-voltage input above its absolute maximum <a id="lvl-10"></a>

🛑 error · needs: real rail voltages, per-pin tolerance (FT) · [catalogue entry](../checks.md#lvl-10)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** An output on a higher rail driving an input whose absolute maximum is its own VDD + 0.3 V (or
similar): 5 V into a 3.3 V pin that is not 5 V tolerant, 3.3 V into a 1.8 V pin.

**Why it matters.** The input clamp diode conducts; the driver's output current flows into the lower rail and the pin,
which is damage in the long run and back-powering in the short run (the lower domain stays half alive when it should be
off). "5 V tolerant" is not a property of the whole MCU: on STM32 it holds only for FT pins, only in input mode, only
with VDD present (below 3.6 V otherwise) and only with no analog function enabled (AN4899 §5.2.2, §6.3).

**Example.** B200A's "1V8" rail `+1V8_LDO` is made by a TLV70233, a fixed 3.3 V LDO, so the I2C low path and every "1.8
V" IO option put about 3.2 V on a device that expects 1.8 V; the firmware had to stop offering 1.8 V until the part is
swapped (rev B change list). A check that trusts the net name passes it.

**How to check.** Take the real voltage of each rail from its source (the regulator's part or feedback divider, not the
net name; `NET-5` sources), then compare a driver's VOH(max) with the receiver pin's absolute maximum and tolerance
class (`NET-7`).

**Fix.** A translator or a series resistor plus clamp sized for the injected current; or move to a tolerant pin
(`MUX-8`); or fix the rail.

**References.** AN4899 §5.2.2, §6.3; SCEA035A §5 (overvoltage-tolerant inputs); B200 rev B change list (U1602).

## `LVL-11` Edge-accelerated auto-direction shifter driving push-pull signals into a load <a id="lvl-11"></a>

⚠️ warning · needs: translator class, loads on its outputs · [catalogue entry](../checks.md#lvl-11)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A TXS01xx-class translator (pass FET plus one-shot edge accelerators plus 10 kΩ pull-ups) whose
outputs drive a pull-down, a long line or a capacitive load, used for push-pull signals such as SWD, UART or SPI.

**Why it matters.** The TXS drives an edge hard only for its one-shot (10-30 ns); after that the high is held by its
internal 10 kΩ pull-up alone (SCEA044 §4, TXS0102 §7.3.5). A 10 kΩ pull-down on the far side makes a divider at half the
rail; more than about 70 pF and the one-shot times out before the edge completes, leaving an RC ramp.

**Example.** B153A routes the target's SWD over the Type-C SBU lines through a TXS0102 (U203). Into B129A's 10 kΩ SWCLK
pull-down and its 200 Ω + 5V6 zener protection, SWCLK highs reached only 2.3-2.5 V at 1 MHz and about 2.0 V at 4 MHz,
and SWDIO highs ramped over microseconds (P2625 bench, 12-13 Oct 2026). The rev B plan is a direction-controlled
2T45-class translator per line.

**How to check.** For every TXS-class part (`NET-7`), follow each channel's output side to its loads (`NET-10`): a
resistor to ground or a rail, a connector (a cable), or a sum of input capacitances above the one-shot budget.

**Fix.** A direction-controlled translator (SN74AXC2T45, SN74LVC2T45, 1T45 per line) for push-pull signals; keep
auto-direction parts for open-drain lines without heavy loads.

**References.** SCEA044 §4; TXS0102 §7.3.5; B153 rev B change list ("Rethink the SBU level shifter").

## `LVL-12` Level shifter direction pin (DIR) floating, or fixed against the signal's direction <a id="lvl-12"></a>

🛑 error · needs: DIR pin role, signal direction · [catalogue entry](../checks.md#lvl-12)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** A direction-controlled translator (LVC1T45, LVC2T45, 74LVC245) whose DIR input is unconnected,
driven by something that is undefined at reset, or tied to the level that points the channel against the actual signal.

**Why it matters.** A floating DIR is a floating CMOS input: random direction, possibly oscillating, and two outputs
fighting on the side that should have been an input. A DIR tied the wrong way turns the translator's output against the
real driver: both drive the line, one of them at the wrong voltage.

**Example.** B200A's IO channels use one SN74LVC1T45 each with DIR pulled down by 10 kΩ, so every channel starts as an
input (the safe state). On rev A, DIR of IO0-IO3 come from the 595 chain, whose outputs are undefined while `EXP.~OE`
floats low at power-up (`IO-12`): until the firmware loads the chain, those four channels may drive the user's
connector.

**How to check.** DIR pins (`NET-7`) must have a driver that is defined at reset or a pull; for a tied DIR, compare the
selected direction with the functions of the pins on both sides (`NET-12`).

**Fix.** A pull on DIR to the "input" direction (B → A towards the controller), driven by a pin that is valid at reset;
tie DIR only when the direction never changes.

**References.** SN74LVC2T45 (DIR referenced to VCCA, §8.1 "no floating I/Os when changing direction"); SCEA035A §2;
B200A hardware reference §4 and §6.

## `LVL-13` Level shifter output enable floating, or enabled before both of its rails are up <a id="lvl-13"></a>

⚠️ warning · needs: OE pin role, power-up order of the two rails · [catalogue entry](../checks.md#lvl-13)

**Impl.** mid · depends on NET-7, NET-8

**What it catches.** A translator's OE (or ~OE) with no pull to the disabled state, or tied enabled while its two
supplies can come up in either order.

**Why it matters.** With OE floating the outputs are enabled or not by chance, and a floating OE draws extra supply
current (SCEA044 §5). Enabled while one rail is still ramping, a translator can glitch its outputs or pass the other
side's state as a wrong level. TI asks for OE pulled to GND and enabled only once VCCA and VCCB are stable (TXS0102
§8.3).

**Example.** B153A does it right: the TXS0102's OE is on PB2 with a 10 kΩ pull-down (R213), so the shifter is off
through reset and while the MCU is halted. The same part with OE tied to VCCA would enable the instant 3.3 V is up,
before V_SBU is selected.

**How to check.** The OE net needs a pull to the disabled level or a driver valid at reset (`NET-7`); a tied OE is a
finding when the two rails have no guaranteed order (`NET-8`). The floating half runs without power states.

**Fix.** Pull OE to its disabled level; drive it from a GPIO after both rails are up, or from the later rail's
power-good.

**References.** TXS0102 §8.3; SCEA044 §5; SCEA043 §4.

## `LVL-14` Digital isolator or transceiver channel direction against the signal <a id="lvl-14"></a>

🛑 error · needs: channel directions of the part, signal direction · [catalogue entry](../checks.md#lvl-14)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** A fixed-direction channel wired backwards: an MCU TX into an isolator's output pin, two outputs
facing each other through a transceiver, the wrong variant of a multi-channel isolator.

**Why it matters.** Isolators and fixed-direction translators come in variants that differ only in which channels go
which way: the ADuM1200 has both channels side 1 → side 2, the ADuM1201 one each way, on the same footprint (ADuM1201
tables 15 and 16). Swap the variant in the BOM and two outputs drive each other, while the two inputs float.

**Example.** A UART across isolation designed for an ADuM1201 (TX one way, RX back) is built with an ADuM1200 because
the BOM line only said "ADuM120x": RX is an output on the MCU side, fighting the MCU's TX.

**How to check.** Per channel, the pin roles of the part (`NET-7`, from the exact MPN) against the functions of the pins
on both sides (`NET-12`): every channel input must face a driver and every channel output an input.

**Fix.** The variant with the right channel split, or swap the connections; give the BOM the full MPN.

**References.** ADuM1201 tables 15-16; TXU0304 datasheet (fixed per-channel directions)
[ti.com/lit/ds/symlink/txu0304.pdf](https://www.ti.com/lit/ds/symlink/txu0304.pdf).

## `LVL-15` Single-MOSFET open-drain shifter on a fast push-pull line <a id="lvl-15"></a>

⚠️ warning · needs: the shifter, signal speed or interface type · [catalogue entry](../checks.md#lvl-15)

**Impl.** mid · depends on NET-10, NET-12

**What it catches.** A BSS138-pair shifter (or PCA9306-style pass gate without acceleration) on SPI above a few MHz,
WS2812 data, a fast clock or any push-pull line that needs sharp edges.

**Why it matters.** Rising edges on the side behind the FET come only from that side's pull-up into the line
capacitance: 10 kΩ and 20 pF is a 200 ns time constant, so a 4 MHz SPI clock never reaches the high level. Falling edges
are fine, which makes the failure look like a duty-cycle or timing problem.

**Example.** A 3.3 V MCU driving a 5 V WS2812 chain through a BSS138 with 10 kΩ pull-ups: the 800 kHz bit timing needs a
0.4 µs high pulse, and the edge alone eats most of it; the first LED reads garbage.

**How to check.** Recognise the shifter, then classify the signal: a pin function of SPI SCK/MOSI, a timer/PWM output,
an LED data input (`NET-12`, following the line with `NET-10`); flag anything that is not I2C, 1-Wire or a slow GPIO.

**Fix.** A buffer with the higher supply and TTL thresholds (74AHCT1G125) or a direction-controlled translator.

**References.** AN10441 §2 (intended for I2C); SCEA035A §3-4; Wikipedia, Level shifter.
