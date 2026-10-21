# I2C checks, explained

Detail page for the `I2C` category of the [check catalogue](../checks.md#-i2c---i2c). Each section explains one
check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the
usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **UM10204** · NXP, *I²C-bus specification and user manual*, Rev. 7.0 (October 2021). The bus standard; section and table numbers
  below refer to it (fetched 21 Oct 2026 through `tpm fetch`, nxp.com refuses plain HTTP clients). [nxp.com/docs/en/user-guide/UM10204.pdf](https://www.nxp.com/docs/en/user-guide/UM10204.pdf)
- **SLVA689** · TI, *I2C Bus Pullup Resistor Calculation* (2015). [ti.com/lit/an/slva689](https://www.ti.com/lit/an/slva689/slva689.pdf)
- **SLVA704** · TI, *Understanding the I2C Bus* (2015). [ti.com/lit/an/slva704](https://www.ti.com/lit/an/slva704/slva704.pdf)
- **SCPA054** · TI, *Why, When, and How to use I2C Buffers* (2018). [ti.com/lit/an/scpa054](https://www.ti.com/lit/an/scpa054/scpa054.pdf)
- **SMBus 3.1** · SBS-IF, *System Management Bus (SMBus) Specification*, v3.1 (2018). [smbus.org/specs](http://smbus.org/specs/SMBus_3_1_20180319.pdf)
- **Wikipedia, I²C** · a readable overview with the reserved-address table and the SMBus differences.
  [en.wikipedia.org/wiki/I²C](https://en.wikipedia.org/wiki/I%C2%B2C)
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 14 (serial buses).

## `I2C-1` No pull-ups on an I2C bus <a id="i2c-1"></a>

🛑 error · needs: the bus (SCL/SDA nets), resistors to a rail · [catalogue entry](../checks.md#i2c-1)

**What it catches.** An SCL or SDA net with no resistor to a power rail anywhere on the bus.

**Why it matters.** I2C drivers are open-drain: every device can only pull a line low. Without a pull-up nothing
ever pulls it high, so the bus reads a permanent low (or floats, if a master uses its internal pulls) and no
transfer completes. Boards "with an I2C sensor that is never found" very often have the pull-ups on a
daughterboard that was assumed, or a DNP footprint nobody populated.

**Example.** A sensor board designed to hang off a Raspberry Pi header: the designer relies on the Pi's 1.8 kΩ
pull-ups. Plugged into an MCU board without them, the sensor is dead. The reverse happens too: a module with its
own pull-ups removed "to save power".

**How to check.** Follow each SCL/SDA net through series resistors and switches to the full bus segment; look
for a resistor whose other end is a rail (`NET-5`). Internal pull-ups of masters count only when the project says
so (they are typically 30-50 kΩ and too weak, see `I2C-8`).

**Fix.** One pull-up pair per bus segment, to the IO rail of the devices on it (`LVL-4`); value per `I2C-8`/`I2C-9`.

**References.** UM10204 §7.1 (pull-up resistor sizing); SLVA689; SLVA704 §2; Wikipedia, I²C "Physical layer".

## `I2C-2` Multiple pull-ups <a id="i2c-2"></a>

⚠️ warning · needs: the bus, resistors to rails · [catalogue entry](../checks.md#i2c-2)

**What it catches.** More than one pull-up on the same line of one bus segment.

**Why it matters.** Pull-ups in parallel add up: two 4.7 kΩ make 2.35 kΩ, and three breakout boards with
their own pulls can reach 1 kΩ. Below Rp(min) the drivers cannot sink the current and VOL rises above the input
threshold: the bus "works on the bench, fails on the hot board". It also wastes power on every low bit.

**Example.** An MCU board with 4.7 kΩ, an OLED module with 10 kΩ and an IMU breakout with 4.7 kΩ on one bus:
1.9 kΩ in total, over 1.7 mA at 3.3 V, above the 3 mA IOL only with margin that the 1.8 V variant does not have.

**How to check.** Count resistors to a rail per line and segment; report the parallel value against Rp(min) =
(VDD − VOL(max)) / IOL (3 mA standard and fast mode, 20 mA Fm+).

**Fix.** Keep one pair; make the module ones DNP; if modules must keep theirs, size the main pair so the total
stays inside the window of `I2C-9`.

**References.** SLVA689 §2 (Rp(min)); UM10204 Table 10 (IOL per mode).

## `I2C-3` Pull-up from two series resistors to nowhere <a id="i2c-3"></a>

🛑 error · needs: the bus, resistor chains · [catalogue entry](../checks.md#i2c-3)

**What it catches.** A pull-up resistor whose far end goes through another resistor (or a chain) that ends on an
unconnected pin, a DNP part or a net with nothing else on it, so no current path to a rail exists.

**Why it matters.** The schematic *looks* pulled up (there is a resistor on the line), but the chain never
reaches a rail. It is the typical leftover of a rework: a 0 Ω selector between two rails was deleted and both
options are now gone.

**Example.** `SDA` → R1 4.7 kΩ → net `I2C_PU` → R2 0 Ω (DNP in this variant) → `+3V3`. In the variant with R2
unpopulated the bus has no pull-up at all, and `I2C-1` alone would not notice because R1 is there.

**How to check.** Walk the resistor chain from the line; the chain must end on a rail with every part mounted
in the current variant.

**Fix.** Pull up straight from the rail, or make the selector a solder jumper with a default.

**References.** UM10204 §7.1.

## `I2C-4` Pull-up to a non-rail net <a id="i2c-4"></a>

🛑 error · needs: the bus, power structure · [catalogue entry](../checks.md#i2c-4)

**What it catches.** A pull-up whose far end is a net that is not a power rail (a signal, an enable, a net with
only this resistor on it).

**Why it matters.** Usually a label typo (`+3V3` vs `3V3`) or a pull-up wired to an "I2C_EN" GPIO so the
firmware can switch it: then the bus only works after that GPIO is driven, which the ROM bootloader and early
firmware do not do, and the GPIO's current limit becomes the bus's.

**Example.** On B153A the SDA pull-up R301 went to `I2C_EN` (PA4) while SCL's R302 went to 3.3 V, so the hub was
only reachable once firmware set PA4; the hub in fact works with both lines simply pulled up, and the pin was
freed in rev B.

**How to check.** The far end of each pull-up must be a rail of the power structure (`NET-5`); a two-pin net
there is the clearest sign.

**Fix.** Pull to the rail; if a switched pull-up is really wanted, use a proper bus switch or a FET with a
documented default (`IO-3`).

**References.** UM10204 §7.1; B153 rev B change list (R301).

## `I2C-5` Address collision <a id="i2c-5"></a>

🛑 error · needs: device addresses (library data, straps `NET-11`) · [catalogue entry](../checks.md#i2c-5)

**What it catches.** Two devices on one bus segment that answer to the same 7-bit (or 10-bit) address.

**Why it matters.** Both acknowledge and both drive SDA during reads: the master sees the wired-AND of two
answers, data is garbage, and writes land in both. Nothing in the netlist hints at it, which is why it is
missed so often: a temperature sensor and an EEPROM from different vendors both at 0x48, or two identical
sensors whose address pin was left at the same level.

**Example.** Two BME280 sensors on one bus: the part has only two addresses (0x76/0x77, by SDO level); a third
needs a mux (`I2C-15`) or a second bus.

**How to check.** Resolve each device's address from the library plus its strap pins (A0-A2, SDO, ADDR levels),
then look for duplicates per segment, counting devices that occupy several addresses (24C16: eight).

**Fix.** Change a strap, pick a sibling part with another address, add a mux or address translator, or move
one device to another bus.

**References.** UM10204 §3.1.11 (10-bit addressing), §3.1.12 (reserved addresses); Wikipedia, I²C "Addressing".

## `I2C-6` I2C over a coarse connector <a id="i2c-6"></a>

💡 suggestion · needs: connector type · [catalogue entry](../checks.md#i2c-6)

**What it catches.** An I2C bus leaving the board through a connector that is not a short fine-pitch FPC or
board-to-board link: a pin header, a cable, a terminal block.

**Why it matters.** I2C has no termination, no differential signalling and a hard 400 pF budget for the whole
bus; a metre of cable adds 50-100 pF and picks up noise, a plug inserted mid-transfer glitches the lines, and a
slave left holding SDA low on the far end locks the whole bus. It works on the bench and fails in the field.

**Example.** A sensor on a 2 m cable at 400 kHz: rise times fail (`I2C-8`), the first ESD event hangs the bus
until power cycle.

**How to check.** A bus net reaching a connector pin whose connector is not FPC/B2B class (`LIB-2`).

**Fix.** Keep I2C on the board; across a cable use a buffer with hot-swap and capacitance isolation (PCA9511,
P82B96 class), a differential bus (CAN, RS-485), or 1-Wire/UART for simple sensors.

**References.** SCPA054 (buffers for long buses, capacitance isolation); UM10204 Table 11 (Cb 400 pF).

## `I2C-7` Series tie resistor too high <a id="i2c-7"></a>

⚠️ warning · needs: the bus, series resistors · [catalogue entry](../checks.md#i2c-7)

**What it catches.** A series resistor in an SCL or SDA line (a net tie, a 0 Ω selector, an ESD resistor) above
about 100 Ω.

**Why it matters.** Series resistance adds to the open-drain driver's output impedance: the low on the far side
is VOL plus IOL × R, and with the pull-up it forms a divider. A few hundred ohms with a 3 mA sink already lifts the
low by a volt. It also slows the falling edge together with the far side's capacitance.

**Example.** A 1 kΩ "protection" resistor in SDA towards an off-board sensor: the far-side low sits at ~1.4 V
with a 2.2 kΩ pull-up on 3.3 V, above the 0.3·VDD = 0.99 V threshold; the sensor never sees a low.

**How to check.** Series resistors on the bus lines (`NET-10`) with a value over `MAX_TIE_RESISTANCE`.

**Fix.** 0 Ω ties, or protection with a TVS instead of a resistor; if a resistor is needed keep it ≤ 100 Ω and
recompute VOL.

**References.** UM10204 §7.3 (series protection resistors), Table 10 (VOL, IOL); SLVA704 §4.

## `I2C-8` Pull-up too weak for the bus speed <a id="i2c-8"></a>

⚠️ warning · needs: the bus, pull-up value, estimated capacitance, bus speed · [catalogue entry](../checks.md#i2c-8)

**What it catches.** Rp above Rp(max) = tr(max) / (0.8473 · Cb), where tr(max) is the rise time the mode allows
(1000 ns standard, 300 ns fast, 120 ns Fm+) and Cb the line capacitance.

**Why it matters.** The rising edge is an RC curve through Rp and the bus capacitance; too slow and the receiver
samples a half-high bit, or the master's next falling edge comes before the line reached VIH. Masters with weak
internal pulls (30-50 kΩ) and long traces fail at 400 kHz with no obvious symptom but random NACKs.

**Example.** Measured on our bench with ~135 pF per line: 10 kΩ gives 1170 ns (fails even 100 kHz), 2.2 kΩ
277 ns (meets 400 kHz), 1 MHz needs ≤ 1.05 kΩ.

**How to check.** Estimate Cb from the pins on the net (≈10 pF per device pin, 1-2 pF/cm of trace, or a project
setting), take the bus speed from the project, compute Rp(max), compare.

**Fix.** Lower Rp (down to Rp(min), see `I2C-9`), shorten the bus, lower the speed, or add a buffer that splits
the capacitance (SCPA054).

**References.** SLVA689 §2.1 (tr = 0.8473 · Rp · Cb, Rp(max)); UM10204 Table 11 (tr per mode), §7.1; Wikipedia,
I²C "Physical layer".

## `I2C-9` Pull-up value outside the recommended range <a id="i2c-9"></a>

⚠️ warning · needs: pull-up value · [catalogue entry](../checks.md#i2c-9)

**What it catches.** A pull-up below Rp(min) ≈ (VDD − 0.4 V) / 3 mA (≈ 1 kΩ at 3.3 V, 470 Ω at 1.8 V) or above
the usual 10 kΩ, or a value that does not parse.

**Why it matters.** Below Rp(min) the open-drain drivers cannot pull the line to a valid low (their IOL is
3 mA, 20 mA in Fm+); above ~10 kΩ the edges are too slow for any real bus (`I2C-8`) and the line becomes
sensitive to leakage and noise. 2.2-4.7 kΩ at 3.3 V covers most boards; 1.8 V buses need the low end.

**Example.** A 470 Ω pull-up copied from a 5 V Fm+ design onto a 1.8 V sensor bus: 3.8 mA per line, above the
sensor's 3 mA sink, VOL rises above 0.3·VDD = 0.54 V.

**How to check.** Compare the parallel pull-up value per line with the window for the bus voltage and mode.

**Fix.** Pick within the window; SLVA689 gives the full calculation.

**References.** SLVA689 §2 (Rp(min), Rp(max)); UM10204 Table 10 (VOL(max) 0.4 V, IOL 3 mA / 20 mA).

## `I2C-10` SCL and SDA pull-ups differ <a id="i2c-10"></a>

⚠️ warning · needs: pull-up values · [catalogue entry](../checks.md#i2c-10)

**What it catches.** Different pull-up values on SCL and SDA of one segment.

**Why it matters.** Rarely fatal, nearly always a mistake: a BOM swap, a value changed on one line only, or a
copy from another bus. Different rise times on clock and data shrink the setup and hold margins the standard
assumes, and the weaker line reaches the limit of `I2C-8` first.

**Example.** SCL 2.2 kΩ, SDA 10 kΩ after "fixing" one line during debug.

**How to check.** Compare the two parallel values per segment.

**Fix.** Same value on both lines.

**References.** UM10204 §7.1.

## `I2C-11` Back-powered I2C device <a id="i2c-11"></a>

🛑 error · needs: power states (`NET-8`), device supply rail, fail-safe data of the part · [catalogue entry](../checks.md#i2c-11)

**What it catches.** A device on the bus whose supply can be off in some power state while the bus stays
powered, and whose SCL/SDA pins are not fail-safe (Ioff-rated) inputs.

**Why it matters.** An unpowered CMOS input has its ESD diode to VDD forward-biased by the bus high: the device
powers itself through the diode to ~0.6 V and clamps both lines there, below VIL for everyone else. The entire
bus is dead until that one device gets power. The design "works" when everything is on, and fails exactly in the
low-power state it was meant to support.

**Example.** A display controller on a switchable `+3V3_DISP` rail sharing the bus with the always-on PMIC and
RTC. Display off: no RTC access, the system cannot wake on alarm.

**How to check.** For each power state, devices on the bus whose rail is off while the pull-up rail is on; the
part is exempt only if the library says its bus pins are fail-safe (PCA9306-class switches, buffers with
Ioff, isolators).

**Fix.** Put the device behind a bus switch that is off with its rail, or on an isolated segment with its own
pull-ups from the same rail (`I2C-18`, `I2C-19`), or keep it powered.

**References.** SCPA054 §3 (bus isolation, hot-swap); UM10204 §3.1.16 (bus clear, which does *not* help here);
catalogue `BKF-2` for the general backfeed case; P2623 "I2C in the wild" §1.

## `I2C-12` SMBus device on an I2C bus <a id="i2c-12"></a>

⚠️ warning · needs: part class (I2C vs SMBus) from the library, bus speed · [catalogue entry](../checks.md#i2c-12)

**What it catches.** A device specified for SMBus (power monitors, chargers, fan controllers, many PMICs)
on a bus run as plain I2C.

**Why it matters.** SMBus is a stricter subset: the clock must stay between 10 kHz and 100 kHz (400 kHz in
3.x), a device may time out and reset its interface when SCL stays low for 25-35 ms (tTIMEOUT), and input
thresholds are fixed (SMBus 2.0: 0.8 / 2.1 V; 3.0: 0.8 / 1.35 V) rather than I2C's 0.3 / 0.7 · VDD. An I2C
master that stretches a transfer over a slow interrupt, or bit-bangs below 10 kHz, gets NACKs; on a 5 V bus
the 2.1 V VIH is far from I2C's 3.5 V, on a 1.8 V bus the fixed 1.35 V may never be reached by a weak high.

**Example.** A bit-banged I2C master at 8 kHz talking to a battery gauge: the gauge times out mid-transfer and
answers nothing. The same gauge works on a hardware I2C peripheral at 100 kHz.

**How to check.** Library class of each device vs the bus speed and VDD of the segment.

**Fix.** Run the bus at SMBus rates with a hardware controller; on 1.8 V or 5 V buses check the fixed
thresholds against the master's VOH/VOL; use a translator where they do not meet.

**References.** SMBus 3.1 §4.2 (fSMB 10 kHz min, tTIMEOUT 25-35 ms), §4.2.1 (fixed VIL/VIH); Wikipedia,
System Management Bus; UM10204 §6 (I2C thresholds).

## `I2C-13` Static-offset buffers in series or facing a low-VIL device <a id="i2c-13"></a>

🛑 error · needs: buffer part type (library) · [catalogue entry](../checks.md#i2c-13)

**What it catches.** Two static-offset bus buffers (P82B96, PCA9515, TCA9517 B-side and friends) connected
back-to-back on one segment, or one of them facing a device whose VIL is below the buffer's output low.

**Why it matters.** These buffers tell "a low I drive" from "a low someone else drives" by driving their own
low to about 0.5 V instead of 0 V. Two of them on one segment each see the other's 0.5 V low as not low, so
neither passes it on and the bus locks; a device with VIL below 0.5 V (some 1.8 V parts) never sees the buffer's
low at all.

**Example.** A backplane with a P82B96 on each card plus a PCA9515 on the host: the host side never sees the
cards' ACKs.

**How to check.** Buffers on a segment by library type; two offset types on one segment, or an offset type
whose output low exceeds any device's VIL on that side, is a finding.

**Fix.** Pair an offset buffer with a non-offset side (TCA9517 A-side faces offset types, B-side must not),
or use incremental-offset / level-shifting buffers that are designed to chain (TCA9617B class).

**References.** SCPA054 §2 (buffer types, static vs incremental offset); TI TCA9517 datasheet §8.3 ("B-side
cannot be used in series with devices that use static voltage offset"); P2623 "I2C in the wild" §5.

## `I2C-14` Push-pull auto-direction shifter on I2C <a id="i2c-14"></a>

⚠️ warning · needs: shifter part type (library) · [catalogue entry](../checks.md#i2c-14)

**What it catches.** A TXB010x-class (push-pull, auto-direction) translator in SCL/SDA, or an edge-accelerated
open-drain class (TXS01xx) where a plain open-drain shifter was wanted.

**Why it matters.** TXB parts sense direction from which side moves first and drive the other with a weak
push-pull output; open-drain lines with external pull-ups under ~50 kΩ defeat the sensing, and TI says so in
its own guide. TXS parts are built for open-drain buses, but their one-shot edge accelerators can fire on slow
edges or heavy capacitance and leave a glitch; they also hold the high only through ~10 kΩ internal pull-ups,
which matters on fast push-pull lines (`LVL-11`). A pass-FET shifter with no acceleration (PCA9306, a BSS138
pair) has none of these modes.

**Example.** A TXB0104 between a 1.8 V SoC and a 3.3 V sensor bus: the sensor is found at 100 kHz on a short
bus, lost once a second sensor adds capacitance.

**How to check.** Translator library type on an I2C net.

**Fix.** PCA9306 or a BSS138 pair for I2C; keep TXS for 1-Wire and slow mixed buses; TXB for push-pull only.

**References.** SCEA043 §1 (TXB: not for open-drain, use TXS for I2C); SCEA044 §2 (TXS one-shot edge
acceleration, 10 kΩ pull-ups); TI PCA9306 datasheet (pass-FET translator without acceleration).

## `I2C-15` Mux channels that can join colliding devices <a id="i2c-15"></a>

⚠️ warning · needs: mux type, device addresses · [catalogue entry](../checks.md#i2c-15)

**What it catches.** An I2C switch (TCA9548A class, where several channels can be on at once) with devices of
the same address, or a total capacitance over the limit, on channels the firmware may enable together.

**Why it matters.** A *switch* joins every enabled channel into one bus; the address collision of `I2C-5` is
back the moment two channels with twins are on, and the capacitance of all joined channels adds up (`I2C-8`,
`I2C-21`). A *mux* (one channel at a time) cannot do this, so the part type decides.

**Example.** Eight identical sensors behind a TCA9548A: correct one at a time, garbage if the driver enables
two channels to "save transactions".

**How to check.** Per switch, the address sets and capacitance of each channel; flag duplicates across
channels and sums over 400 pF.

**Fix.** Document single-channel use, or use a mux-type part, or give the twins different straps.

**References.** TI TCA9548A datasheet §8.3 (channel selection, resolving address conflicts); SCPA054 §4.

## `I2C-16` Device at a reserved address <a id="i2c-16"></a>

⚠️ warning · needs: device addresses · [catalogue entry](../checks.md#i2c-16)

**What it catches.** A device whose 7-bit address falls in 0x00-0x07 or 0x78-0x7F.

**Why it matters.** The standard reserves these: 0x00 general call, 0x01 CBUS, 0x02 other bus formats, 0x03
future, 0x04-0x07 Hs-mode master codes, 0x78-0x7B the 10-bit address prefix, 0x7C-0x7F device ID. A device
there is addressed by accident by any master using those features, and many host stacks refuse to talk to it at
all. SMBus adds 0x08 (host) and 0x0C (alert response address).

**Example.** A configurable device strapped to 0x03 to "avoid the sensors": Linux's i2c-core rejects the
address, the driver never binds.

**How to check.** Resolved address against the reserved table.

**Fix.** Another strap value.

**References.** UM10204 §3.1.12, Table 4 (reserved addresses); Wikipedia, I²C "Reserved addresses in 7-bit
address space"; SMBus 3.1 Appendix C (SMBus reserved addresses).

## `I2C-17` Address straps floating <a id="i2c-17"></a>

🛑 error · needs: strap pins (library) · [catalogue entry](../checks.md#i2c-17)

**What it catches.** Address-select pins (A0-A2, ADDR, SA0, SDO) with no pull, tie or driver.

**Why it matters.** Most such pins have no internal pull: a floating one reads a random level, so the address
differs from board to board or changes with humidity and the firmware's probe fails intermittently. Some parts
sample the straps once at power-up, so a GPIO that drives them later has no effect. Others (24C16 class EEPROMs)
use the pins as block selects instead of address bits and must still be tied.

**Example.** An ADS1115's ADDR left open "because the default is 0x48": the pin has no internal pull, half the
boards appear at 0x49.

**How to check.** Pins of the strap class (`NET-7`, library) that are not tied to a rail, a resistor or a driver.

**Fix.** Tie each strap to GND or VDD directly, or through a resistor if a GPIO shares it; note the resulting
address in the schematic.

**References.** UM10204 §3.1.11; the device datasheets (address pin section); P2623 "I2C in the wild" §2.

## `I2C-18` Bus segment without its own pull-ups <a id="i2c-18"></a>

🛑 error · needs: the bus split into segments by translators, buffers, muxes · [catalogue entry](../checks.md#i2c-18)

**What it catches.** A segment separated from the rest of the bus by a translator, buffer, isolator or mux
whose SCL/SDA have no pull-ups of their own.

**Why it matters.** Each such part breaks the bus into electrically separate segments: the pull-ups on the master
side do not reach through a buffer (it only passes lows), through an isolator (nothing passes), or through a
pass-FET translator when the far side's rail is the higher one. A segment without pull-ups is `I2C-1` in
disguise, and the master side looks perfectly fine on a scope.

**Example.** An ISO1540 isolator with 4.7 kΩ on the MCU side only: the isolated side's SCL/SDA never go high,
the sensor behind it is never found.

**How to check.** Segment the bus at each translator/buffer/isolator/mux pin pair (`NET-4` already does this
for translators) and run `I2C-1` per segment.

**Fix.** A pull-up pair per segment, to that segment's rail.

**References.** SCPA054 §2 (buffers: each side needs pull-ups); TI PCA9306 datasheet, application section
(pull-ups on both sides); UM10204 Table 11 (Cb per side).

## `I2C-19` Segment pull-ups to a rail that can be off <a id="i2c-19"></a>

⚠️ warning · needs: power states (`NET-8`) · [catalogue entry](../checks.md#i2c-19)

**What it catches.** Pull-ups of a segment to a rail that is off in a power state where the rest of the bus is
on and that segment is still connected (no switch opens it).

**Why it matters.** With the rail off, the pull-ups become a path *from* the bus *into* the dead rail: the bus
highs feed the rail through the resistors (weakly) and the segment sees no proper high. If a translator still
joins the segments, the whole bus suffers. It is the pull-up variant of the backfeed family (`BKF-12`).

**Example.** A switched `+3V3_SENS` rail feeding sensors and their pull-ups, joined to the always-on bus by a
PCA9306 whose EN stays high: sensors off, bus highs drag to ~1 V through the sensors' ESD diodes and the
pull-ups.

**How to check.** For each power state, segments whose pull-up rail is off while an adjacent segment is on and
the joining part is enabled.

**Fix.** Open the switch/translator together with the rail (EN from the rail), or pull up from an always-on rail
and switch only the devices.

**References.** TI PCA9306 datasheet (EN pin behaviour); catalogue `BKF-12`, `I2C-11`.

## `I2C-20` Clock-stretching device on a master that cannot stretch <a id="i2c-20"></a>

💡 suggestion · needs: device and master data (library) · [catalogue entry](../checks.md#i2c-20)

**What it catches.** A slave known to stretch SCL (sensors that hold the clock during conversion, some MCUs as
slaves) on a master known to mishandle stretching.

**Why it matters.** The standard allows a slave to hold SCL low until it is ready; some masters do not wait
correctly. The best known is the BCM2835 of early Raspberry Pis, whose controller only masks the clock while
SCL is held and does not restore a full clock period afterwards, so a stretch that ends at the wrong phase
corrupts the following bits. Such a pairing works most of the time and fails on the occasional long conversion.

**Example.** A BNO055 IMU (stretches for up to a millisecond) on a Raspberry Pi 3 over hardware I2C: sporadic
bad readings until the bus is slowed to 50 kHz or moved to a software I2C with stretching support.

**How to check.** Both parts flagged in the library; a finding is a reminder, not a fault of the schematic.

**Fix.** A master that supports stretching, a lower clock, or a protocol that avoids it (the device may offer
an interrupt line instead).

**References.** UM10204 §3.1.9 (clock stretching); Advamation, "Raspberry Pi I2C clock-stretching bug"
(advamation.com/knowhow/raspberrypi/rpi-i2c-bug.html); P2623 "I2C in the wild" §2.

## `I2C-21` Bus capacitance over 400 pF <a id="i2c-21"></a>

⚠️ warning · needs: estimated capacitance per segment · [catalogue entry](../checks.md#i2c-21)

**What it catches.** A segment whose estimated line capacitance exceeds 400 pF (550 pF in Fm+).

**Why it matters.** The rise-time budget of `I2C-8` assumes Cb ≤ 400 pF; above it no legal pull-up can meet
the rise time at any speed, and the drivers' VOL rises with the current they must sink. Long cables, many
devices and large ESD arrays get there quickly.

**Example.** Twelve devices at ~10 pF each plus 1.5 m of ribbon cable (~120 pF) plus the master: ~250 pF on
board, 370-400 pF with the cable, no margin.

**How to check.** Sum the pin capacitances (library or a default per pin), trace length if the layout is
available, and connector/cable estimates from the project.

**Fix.** A buffer that splits the bus into two capacitive halves (SCPA054), fewer devices per segment, a
shorter route, or a lower mode with its longer tr allowance.

**References.** UM10204 Table 11 (Cb 400 pF, 550 pF Fm+); SLVA689 §2.1; SCPA054 §3 (capacitance buffering);
Wikipedia, I²C "Physical layer".

## `I2C-22` Several masters on one bus <a id="i2c-22"></a>

💡 suggestion · needs: device roles (library) · [catalogue entry](../checks.md#i2c-22)

**What it catches.** Two or more devices that can act as master (MCUs, SoCs, PMICs with host mode, bus
analysers) on one bus.

**Why it matters.** I2C supports multi-master through arbitration on SDA and clock synchronisation, but only
if *every* master's controller implements it and the firmware handles arbitration loss and bus-busy. Many
MCU peripherals and nearly all bit-banged masters do not; a lost arbitration then corrupts a transfer in
progress. Two MCUs each "owning" the bus at different times (a supervisor and an application processor) is
the usual shape.

**Example.** An application MCU and a small supervisor both polling a PMIC: works until both start a transfer
within the same microsecond, then one of them sees garbage once a day.

**How to check.** Count master-capable devices per bus from the library role.

**Fix.** One master per bus, with a bus switch or a mailbox between controllers; or verify multi-master
support on every controller and plan the firmware for it.

**References.** UM10204 §3.1.7 (clock synchronisation), §3.1.8 (arbitration); Wikipedia, I²C "Arbitration
using SDA"; AoE3 chapter 14.
