# MEM checks, explained

Detail page for the `MEM` category of the [check catalogue](../checks.md#-mem---memory-ddr-emmc-sd-flash). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc
needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of
truth for ID, severity and wording; this page is the commentary.

References used throughout:

- **SD PLS** · SD Association, *Physical Layer Simplified Specification*, Ver. 9.10 (December 2023). The SD bus
  standard (pull-ups, card detect, signal voltage switch, power-up). Listed on the simplified specifications page;
  the PDF sits behind a click-through licence (page fetched through `tpm fetch`). [sdcard.org/downloads/pls](https://www.sdcard.org/downloads/pls/)
- **JESD84-B51** · JEDEC, *Embedded MultiMediaCard (e•MMC) Electrical Standard (5.1)*. The eMMC standard (VCC,
  VCCQ, RST_n and its `RST_n_FUNCTION` register).
- **JESD79-3, JESD79-4** · JEDEC, *DDR3 SDRAM Standard* and *DDR4 SDRAM Standard*. Pin functions, ZQ, VREF,
  RESET_n and CKE at power-up.
- **SPRABI1** · TI, *DDR3 Design Requirements for KeyStone Devices*, SPRABI1D (January 2018, revised July 2022).
  ZQ, reset, VREF tolerance, VTT termination and which pins may be swapped; section and table numbers below refer
  to it. [ti.com/lit/pdf/sprabi1](https://www.ti.com/lit/pdf/sprabi1)
- **AN5692** · ST, *DDR memory routing guidelines for STM32MP13x product lines*, Rev 1 (January 2023). A compact
  schematic checklist for DDR3/3L and LPDDR2/3 (fetched 21 Oct 2026 through `tpm fetch`, st.com refuses plain HTTP
  clients). [st.com AN5692](https://www.st.com/resource/en/application_note/an5692-ddr-memory-routing-guidelines-for-stm32mp13x-product-lines--stmicroelectronics.pdf)
- **AN5200** · ST, *Getting started with the STM32H7 MCU SDMMC host controller*, Rev 3 (June 2025). VDDMMC, the
  UHS-I voltage switch transceiver (fetched through `tpm fetch`).
  [st.com AN5200](https://www.st.com/resource/en/application_note/an5200-getting-started-with-stm32h7-mcu-sdmmc-host-controller-stmicroelectronics.pdf)
- **ESP-IDF SD pull-ups** · Espressif, *SD Pull-up Requirements*, ESP-IDF Programming Guide v6.1.
  [docs.espressif.com](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/sd_pullup_requirements.html)
- **W25Q128JV** · Winbond, *W25Q128JV 3V 128M-bit serial flash memory with Dual/Quad SPI*, Rev F (March 2018).
  /WP, /HOLD, /RESET and the Quad Enable bit.
  [winbond.com](https://www.winbond.com/resource-files/w25q128jv%20revf%2003272018%20plus.pdf)
- **CAT24C256** · onsemi, *EEPROM Serial 256-Kb I2C*, Rev. 16 (February 2022). An EEPROM whose WP pin has an
  internal pull-down (fetched through `tpm fetch`). [onsemi.com](https://www.onsemi.com/pdf/datasheet/cat24c256-d.pdf)
- **MEM2061** · GCT, *MEM2061 Micro SD Memory Card Connector, 1.88mm Profile* (drawing). The micro-SD socket of
  B125A, with its card-detect switch circuit. [gct.co/files/drawings/mem2061.pdf](https://gct.co/files/drawings/mem2061.pdf)
- **RP2350 HW design** · Raspberry Pi, *Hardware design with RP2350*, release 3 (2026). Chapter 3, the QSPI flash.
  [datasheets.raspberrypi.com](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf)
- **U-Boot mmc** · *mmc command*, U-Boot documentation: `mmc rst-function` and the write-once `RST_n_FUNCTION`
  field. [docs.u-boot.org](https://docs.u-boot.org/en/latest/usage/cmd/mmc.html)
- **Wikipedia, SD card** · overview with the bus speed modes, UHS-I at 1.8 V and the pinout.
  [en.wikipedia.org/wiki/SD_card](https://en.wikipedia.org/wiki/SD_card)
- **Wikipedia, DDR3 SDRAM / DDR4 SDRAM** · overviews; the DDR4 page lists what changed against DDR3 (VrefDQ inside
  the DRAM). [DDR3](https://en.wikipedia.org/wiki/DDR3_SDRAM), [DDR4](https://en.wikipedia.org/wiki/DDR4_SDRAM)
- **Bogatin** · Eric Bogatin, *Signal and Power Integrity, Simplified*, 3rd ed. (Prentice Hall, 2018): reflections
  and termination (chapter 8, transmission lines and reflections).

## `MEM-1` SD / eMMC CMD and DAT lines without pull-ups <a id="mem-1"></a>

⚠️ warning · needs: SD socket or eMMC pins by function, resistors to a rail · [catalogue entry](../checks.md#mem-1)

**Impl.** easy · depends on netlist, NET-5

**What it catches.** An SD socket or eMMC whose CMD or DAT0-DAT3 (DAT0-DAT7 on 8-bit eMMC) lines have no pull-up
to the card's IO rail.

**Why it matters.** CMD and DAT are open-drain during card identification and tri-stated whenever no card drives
them: with no card inserted, between commands, and on the data lines the host does not use in 1-bit mode. Floating
lines pick up noise, read as random start bits, and a DAT line floating low looks like a card stuck busy. The SD
specification asks the host to pull up all of DAT0-3 even in 1-bit mode; DAT3 doubles as card detect and must not
float either. Host-internal pull-ups exist on some controllers, but they are weak, firmware-dependent, and not on
during the boot ROM.

**Example.** On B125A the micro-SD socket J1 (GCT MEM2061) connects `SDMMC_CMD` and `SDMMC_D0`-`D3` straight to
the STM32H7 with nothing else on the nets: each is a two-pin net. The board relied on the MCU's internal pulls; the
rev B change list carries "SDIO pull-ups; test with and without". B200A got it right: 10 kΩ on CMD, D0-D3 and DET
(hw_b200a.md §11).

**How to check.** Find the card side by pin function (CMD, DAT0-DAT7 on the socket or eMMC symbol), walk each line
through series resistors, and look for a resistor to a rail (`NET-5`). Accept 10-100 kΩ; a host whose internal
pull-ups the project declares as enough turns it into a note.

**Fix.** 10 kΩ (up to 47 kΩ) from CMD and every DAT line to the card's VDD (SD) or VCCQ (eMMC), placed near the
socket.

**References.** ESP-IDF SD pull-ups (10 kΩ on CMD and DAT0-DAT3, on the card side too); SD PLS (bus pull-up
resistors); Wikipedia, SD card "Transfer modes" (DAT3/CD pin).

## `MEM-2` DDR pin swap across bytes and other errors <a id="mem-2"></a>

🛑 error · needs: DDR controller and DRAM pin functions, the family's swap rules · [catalogue entry](../checks.md#mem-2)

**Impl.** hard · depends on NET-7, NET-10

**What it catches.** A DDR data bit routed into another byte lane than its strobe (DQ12 of the controller to DQ3
of the DRAM while DQS1 goes to DQS1), DM or DQS mixed between lanes, swapped address or command lines on a
memory type that forbids it, and swaps on the lanes a controller needs straight (LPDDR byte 0 on some SoCs).

**Why it matters.** Each byte lane is captured with its own strobe and trained (write levelling, read gate, DQ
deskew) as a unit: a bit wired into a foreign lane is sampled against the wrong DQS and fails at speed, if at all.
Bits *within* a byte may usually be swapped to ease routing, since data is just stored and read back in the same
order. Address and command lines may not: mode registers are written over them, so a swapped A-line writes the
wrong CAS latency or burst length into the DRAM. Some controllers read mode registers or run calibration patterns
over byte 0 and forbid any swap there. The rules differ per family and controller, so a board copied from another
SoC can be wrong while looking tidy.

**Example.** An STM32MP135 with one 16-bit LPDDR3: the layout person swaps DQ2 and DQ5 in byte 0 to save a via.
AN5692 §5.1 says byte 0 of the LPDDR2/3 must go to byte 0 of the MP13 with no bit swap inside it, so the
calibration fails at boot. The same swap on a DDR3L would have been fine (§4.1, "swappable bits in the same byte").

**How to check.** Map each controller DQ/DQS/DM pin to the DRAM pin it reaches (`NET-10` through series
resistors), group by lane (DQ0-7 with DQS0 and DM0, and so on), and flag a bit whose DRAM lane differs from its
strobe's. Then apply the controller's own swap rules, kept as per-family part data: byte swaps allowed or not, bit
swaps per lane, address mirroring.

**Fix.** Keep every DQ with its DQS and DM; swap only what the controller's hardware guide allows, and record
deliberate swaps for the firmware (some controllers need the map in a register).

**References.** SPRABI1 §6.3.1.4 ("address lines cannot be swapped"), §6.3.1.6 (data bits within a byte lane may
be swapped), Table 6-5 (byte lane grouping); AN5692 §4.1 (DDR3/3L: two swappable bytes, swappable bits),
§5.1 (LPDDR2/3: byte 0 fixed, no bit swap); JESD79-3.

## `MEM-3` SD / eMMC CLK with a pull-up, or CD / WP switch pins without one <a id="mem-3"></a>

⚠️ warning · needs: SD socket pins by function, resistors to rails · [catalogue entry](../checks.md#mem-3)

**Impl.** easy · depends on netlist, NET-5

**What it catches.** A pull-up on the SD or eMMC clock line, and the opposite mistake: card-detect (CD) or
write-protect (WP) switch contacts of a socket with no pull-up (or pull-down) at all.

**Why it matters.** CLK is always driven push-pull by the host; a pull-up does nothing useful there but adds load
and a stub on the fastest line of the bus, and it is the usual sign that a resistor array was copied over all six
lines. CD and WP are bare mechanical switches to ground or the shell: without a pull the input floats whenever the
switch is open, so the firmware sees cards appear and vanish, or reads a write-protected card as writable.

**Example.** A board uses two 4 × 10 kΩ arrays to "pull up the SD bus" and so covers CMD, CLK, DAT0-DAT3, CD and
WP, with CLK on one of them; a later revision fixes the CLK but forgets the CD pull, and a Hirose DM3AT socket's
detect switch, open with no card, leaves the MCU input floating. B200A shows the intended pattern: 33 Ω series on
CLK, no pull-up there, 10 kΩ on DET (hw_b200a.md §11).

**How to check.** A resistor from the CLK net to a rail (not counting a series resistor); a socket pin whose
function is CD, DET or WP with no resistor to a rail and no MCU pin that the project marks as pulled.

**Fix.** Remove the CLK pull-up (keep a 22-33 Ω series resistor at the host if needed); 10-100 kΩ pulls on CD and
WP to the logic rail, matching the switch's other side (to ground or to the shell, see `MEM-5`).

**References.** ESP-IDF SD pull-ups (pull-ups on CMD and DATA only); Wikipedia, SD card "Write-protect notch".

## `MEM-4` SPI / QSPI flash WP#, HOLD# or RESET# floating <a id="mem-4"></a>

🛑 error · needs: flash pin functions and internal pulls, host IO2/IO3 use · [catalogue entry](../checks.md#mem-4)

**Impl.** mid · depends on NET-7

**What it catches.** A serial NOR flash (W25Q, MX25L, IS25LP, BY25Q class) whose /WP (IO2), /HOLD (IO3) or
/RESET pin is unconnected, or goes only to a host pin that is not driven at power-up.

**Why it matters.** /HOLD low while /CS is low pauses the device: its output goes high-impedance and clock and
data are ignored, so a floating /HOLD that drifts low makes reads return garbage at random. /WP low together with
the status register's protect bits locks writes. In quad mode the two pins are data lines IO2/IO3 and the host
drives them, but only during quad transfers: in the single-bit phase of the boot ROM, or with a part ordered with
the Quad Enable bit cleared, they are /WP and /HOLD again and float. Whether a floating pin matters depends on the
exact part: on the W25Q128JV the "IQ" ordering code ships with QE = 1 (no /HOLD function), the "IM" code with
QE = 0 (/HOLD active); the dedicated /RESET pin of the SOIC-16 and TFBGA packages has an internal pull-up and may
float.

**Example.** A board reads a W25Q128JVSIQ over a plain STM32 SPI (standard SPI, IO2/IO3 not connected): it works,
since QE = 1 disables /HOLD. A second source substitutes the W25Q128JVSIM, whose QE = 0 makes /HOLD active; the
floating pin now pauses the flash whenever it drifts low, and the bug shows only on boards from that reel.

**How to check.** For each flash, the /WP, /HOLD and /RESET pins (by function or by pin name) must reach a rail
through a resistor, be tied to the rail, or reach a host pin that is a quad data line or a GPIO with a defined
level at reset. The pin model (`NET-7`) supplies the internal pulls, so a /RESET with an internal pull-up is not
flagged.

**Fix.** 10 kΩ pull-ups on /WP and /HOLD to the flash VCC even when they are also IO2/IO3 to a quad host, or tie
them to VCC in single-bit designs; /RESET to VCC or a reset GPIO. The /CS pull-up the flash wants at power-up is
`SER-19`.

**References.** W25Q128JV §4.3 (/WP), §4.4 (/HOLD), §4.6 and §6.4 (/RESET, internal pull-up), §7.1.4 (Quad
Enable, IQ vs IM ordering codes), §4.1 (/CS must track VCC at power-up); RP2350 HW design §3.1 (QSPI flash wired
directly, the QSPI_SS pull-up).

## `MEM-5` SD card detect wired to the wrong pins <a id="mem-5"></a>

⚠️ warning · needs: the socket's detect switch circuit (pin roles), resistors · [catalogue entry](../checks.md#mem-5)

**Impl.** mid · depends on NET-7

**What it catches.** A card-detect net wired against the socket's real switch: to the shell instead of the switch
contact, a detect contact tied to ground on a socket whose switch closes to the shell (or to VDD), or the
normally-open logic assumed on a normally-closed switch.

**Why it matters.** Micro-SD sockets implement detect in several ways: a switch between two dedicated pins, a
contact that closes against the grounded shell, normally open or normally closed. The generic library symbol
(two DET pins and a shield) hides which one the footprint really has. Wired against the socket, the input reads
the same with and without a card, and firmware either never mounts the card or keeps polling a card that is not
there.

**Example.** On B125A the socket J1 is a GCT MEM2061-01 ("with switch, normally closed"): its drawing shows the
card-detect contact closing to the grounded shell with no card and opening when one is inserted. The schematic
used KiCad's generic `Micro_SD_Card_Det2` symbol: DET_A (pin 10) to PG4 with a 10 kΩ pull-up, DET_B (pin 9) and
the shield to GND. The rev B change list moves it: "uSD card detect goes from both side pins to the shell, not to
GND".

**How to check.** From the socket's pin data (`NET-7`, filled from its drawing): which pins form the switch, to
what it closes, NO or NC. Then the detect net must have a pull to the logic rail, reach an MCU input, and not be
tied to the switch's own reference.

**Fix.** Wire per the socket's switch circuit, pull the detect contact up to the IO rail, and write down the
polarity (low = card present or absent) on the schematic for the firmware.

**References.** MEM2061 drawing ("Card Detect Switch Circuit", ordering code 01 = with switch, normally closed);
B125 rev B change list; Wikipedia, SD card "Transfer modes" (CD by DAT3 as the alternative).

## `MEM-6` eMMC VCCQ on another voltage than the host IO bank, or RST_n without a pull-up <a id="mem-6"></a>

🛑 error · needs: rail voltages, host SD/MMC bank supply, eMMC pins · [catalogue entry](../checks.md#mem-6)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** An eMMC whose VCCQ (IO supply) differs from the supply of the host pins it talks to (1.8 V
VCCQ against a 3.3 V host bank, or the reverse), and an eMMC RST_n pin left floating.

**Why it matters.** The eMMC's IO cells run from VCCQ. A 3.3 V host driving a 1.8 V VCCQ part pushes current
through its input clamp diodes into VCCQ and stresses the inputs; a 1.8 V host reading a 3.3 V part sees highs far
above its own rail. Neither works reliably. RST_n is ignored while the `RST_n_FUNCTION` field is 0 (the default),
but that field is write-once: once a bootloader enables it (`mmc rst-function`), a floating RST_n becomes a live
reset input forever, and a board that worked for years starts resetting its storage at random.

**Example.** An STM32H743 with an 8 GB eMMC on SDMMC1: the eMMC's VCCQ comes from the board's +1V8 rail for
HS200, while the MCU's VDDMMC pin (the separate supply of the SDMMC pads) is left tied to VDD = 3.3 V. The
host drives 3.3 V into 1.8 V IOs.

**How to check.** Find the eMMC by pin function (VCCQ, VCC, CMD, DAT, RST_n); take the VCCQ rail voltage
(`NET-5`); find the host pins on CMD/CLK/DAT and the supply of their IO bank (`NET-7`); compare. RST_n must reach
a pull-up to VCCQ or a host GPIO with a defined reset level.

**Fix.** Feed the host's SD/MMC bank from the eMMC's VCCQ rail (or VCCQ from the host bank's rail); 10 kΩ pull-up
from RST_n to VCCQ, or drive it from a host reset output.

**References.** JESD84-B51 (VCCQ, RST_n); AN5200 §1.1 (VDDMMC as independent supply of the SD I/Os); U-Boot mmc
(`RST_n_FUNCTION`, "a write-once field", default disabled).

## `MEM-7` SD host IO voltage vs card mode <a id="mem-7"></a>

⚠️ warning · needs: host bank supply and switching, wanted card mode · [catalogue entry](../checks.md#mem-7)

**Impl.** mid · depends on NET-5, NET-7, CFG-2

**What it catches.** A design that wants UHS-I speeds (SDR50, DDR50, SDR104) from an SD card while the host's SD
pins sit on a fixed 3.3 V supply with no voltage switch transceiver or switchable regulator.

**Why it matters.** Cards start at 3.3 V signalling; UHS-I modes run at 1.8 V, entered through a voltage switch
command after which the host must change its IO voltage. A host whose SD pins cannot go to 1.8 V stays at High
Speed (25 MB/s) no matter which card is inserted: not a failure, but a board that misses its throughput target,
usually found after layout.

**Example.** An STM32H7R/S design that needs 50 MB/s logging: on that line the SDMMC IOs run from VDD (no separate
VDDMMC), so AN5200 §1.2 recommends a voltage switch transceiver for UHS-I. Without one, the card stays at High
Speed.

**How to check.** The supply of the host's SD bank (`NET-7`) and whether it is a fixed rail (`NET-5`) or comes
from a regulator or transceiver with a select line; compare with the bus mode in the project file (`CFG-2`). Only
a project that asks for UHS gets the warning.

**Fix.** A voltage switch transceiver between host and card, a host bank on a regulator that the controller
switches to 1.8 V, or accept High Speed and write that down.

**References.** AN5200 §1.1 (Figure 4, voltage switch transceiver), §1.2 (H7R/S: transceiver recommended for
UHS-I); Wikipedia, SD card "UHS-I", "High Speed"; SD PLS (signal voltage switch).

## `MEM-8` DDR ZQ pin without its 240 Ω to VSS <a id="mem-8"></a>

🛑 error · needs: DRAM and controller ZQ pins, resistor value · [catalogue entry](../checks.md#mem-8)

**Impl.** easy · depends on netlist, IMP-6

**What it catches.** A ZQ pin on a DDR3/3L/4 or LPDDR DRAM (or on the controller) that has no resistor, a wrong
value, a resistor to a supply instead of VSS, or one resistor shared by several ZQ pins.

**Why it matters.** The DRAM calibrates its output drivers and on-die termination against the external resistor
at ZQ. Without it, or with the wrong one, drive strength and ODT come out at whatever the calibration converges
to: overshoot, eye closure and read errors that change with temperature. A shared resistor makes the calibration
cycles of two dies interfere.

**Example.** Two MT41K256M16 (DDR3L, 16-bit) on one controller with their ZQ pins joined to one 240 Ω: the
schematic looks complete, but each die calibrates against a resistor the other is also loading during its own
ZQCL cycle.

**How to check.** For each pin named ZQ (ZQ0, ZQ1, DDR_ZQ) on a DRAM or memory controller: exactly one resistor
to ground, value 240 Ω (`IMP-6` parses it), 1 % if tolerance is known, not shared with another ZQ pin.

**Fix.** One 240 Ω 1 % resistor from each ZQ pin to VSS, close to the pin.

**References.** SPRABI1 §3.3.4 (240 Ω ≤ 1 %, dedicated per SDRAM, "cannot share pins"); AN5692 §4.4 (240 Ω ±1 %
on each DDR and on the device), §5.2 (LPDDR2/3); JESD79-3.

## `MEM-9` DDR VREF not at VDD/2, or VTT termination missing on a fly-by topology <a id="mem-9"></a>

🛑 error · needs: VREF dividers, DRAMs per address net, terminations · [catalogue entry](../checks.md#mem-9)

**Impl.** mid · depends on NET-5, RPT-6

**What it catches.** A VREFCA or VREFDQ pin whose divider is not VDDQ/2 (unequal resistors, 5 % parts, a divider
from the wrong rail), and a fly-by address/command bus with several DRAMs but no termination resistors to VTT at
the far end, or VTT without its supply.

**Why it matters.** DDR inputs are compared against VREF: every millivolt off-centre eats into the eye on one side.
The VREF tolerance is about ±1 % of VDD/2, which a divider of 5 % resistors cannot meet, and a divider from another
rail does not track VDDQ. On a fly-by bus the address and command lines run past every DRAM to the last one; left
unterminated, the stub end reflects and corrupts commands. Point-to-point with a single DRAM can go without VTT,
but two or more chips on the same address lines cannot. DDR4 differs: VREFDQ is generated inside the DRAM and only
VREFCA is a board pin.

**Example.** Two DDR3L chips in fly-by on an STM32MP135 with the address lines going from the MP13 to the first
and second chip and stopping there: AN5692 §4.3 asks for 56 Ω to VTT (= VDD_DDR / 2) on every A/C line. Or a VREF
made of 1 kΩ / 1.2 kΩ "because that was in the BOM": 0.61 V instead of 0.675 V at VDD_DDR = 1.35 V.

**How to check.** VREF pins: find the divider (`RPT-6`), compute its ratio and source rail (`NET-5`); expect 0.5
± 1 % from VDDQ, 1 % resistors, a decoupling cap. Topology: count DRAMs sharing each address net; with more than
one, expect a resistor (39-56 Ω) from each A/C line to a VTT rail.

**Fix.** VREF from two equal 1 % resistors from VDDQ with 100 nF at the pin, or from a DDR termination regulator's
VREF output; VTT from a sink/source regulator (TPS51200 class or the PMIC), 39-56 Ω at the last DRAM.

**References.** SPRABI1 §4.7.4 (VREF ±1 %), §6.2.1 (39-42 Ω to VTT at the end of the fly-by, VTT tracks VDDQ/2),
§3.1.2 (fly-by topology); AN5692 §4.2 (single DRAM point-to-point without VTT), §4.3 (fly-by, 56 Ω to VTT), §4.5
(VREF from two 1 kΩ 1 % with 100 nF); Wikipedia, DDR4 SDRAM (VrefDQ internal, VrefCA external); Bogatin, chapter 8.

## `MEM-10` DDR RESET_n without a pull-down, CKE without a defined level at power-up <a id="mem-10"></a>

⚠️ warning · needs: DRAM pin functions, resistors to ground · [catalogue entry](../checks.md#mem-10)

**Impl.** easy · depends on netlist, NET-5

**What it catches.** DDR3/DDR4 RESET_n or CKE nets with no pull-down, so their level while the controller is
unpowered or in reset is undefined.

**Why it matters.** The DRAM power-up sequence wants RESET_n low while the supplies ramp and CKE low until the
controller starts initialisation. The controller's pins are not driven until its own IO supply and reset are up;
a floating RESET_n or CKE during that window lets the DRAM start in an undefined state, which shows as a board
that fails to train once in a few hundred cold boots. RESET_n is a CMOS input referenced to VDD (0.8 · VDD for a
high), not an SSTL line, so a plain pull-down is fine.

**Example.** An STM32MP135 with DDR3L where the designer left both pins as direct controller connections: AN5692
§4.4 lists a 10 kΩ pull-down on DDR_RESETN and on DDR_CKE as required.

**How to check.** For each DRAM: RESET_n and CKE must reach a resistor to ground (or the project marks the
controller pin as driven low from power-up).

**Fix.** 10 kΩ pull-downs on RESET_n and CKE, near the DRAM.

**References.** AN5692 §4.4 (10 kΩ pull-down on DDR_RESETN and DDR_CKE); SPRABI1 §3.3.5 (reset pin, CMOS levels);
JESD79-3 (power-up and initialisation sequence).

## `MEM-11` EEPROM or flash write-protect pin floating <a id="mem-11"></a>

⚠️ warning · needs: WP pin by function, the part's internal pull · [catalogue entry](../checks.md#mem-11)

**Impl.** mid · depends on NET-7

**What it catches.** The WP pin of an I2C/SPI EEPROM or a parallel/serial flash left unconnected on a part
without an internal pull, or a WP that is wired to a GPIO with no pull to its intended default.

**Why it matters.** A floating WP reads as whatever leakage makes it: writes work on the bench and fail on some
boards, or a "protected" calibration EEPROM gets overwritten by a firmware bug. Some parts pull WP to "write
enabled" internally and may float; others do not, and the datasheet asks for a tie. The difference is per part,
which is why the check needs part data, not just an open pin.

**Example.** A board ID EEPROM on an add-on card (24LC256 or CAT24C256 footprint, both sourced): on the
CAT24C256 WP has an on-chip pull-down and floating is harmless; a second source without that pull
makes the same layout a gamble. The intent
(read-only after production) is met only if WP is pulled high with a test-point to pull it low at programming.

**How to check.** Pins named WP, /WP, WC on memory parts: no connection, or only a GPIO, counts as floating unless
the pin model (`NET-7`) says the part has an internal pull to the side the project wants.

**Fix.** Tie WP to the intended default with a resistor (to VCC for protected, to GND for writable), add a pad or
GPIO to override it at programming.

**References.** CAT24C256 "Pin Description" (WP with on-chip pull-down) and "Hardware Write Protection" (floating
or grounded WP has no impact); W25Q128JV §4.3, §7.1.1 (/WP with SRP: hardware protection).

## `MEM-12` SD card power not switchable <a id="mem-12"></a>

💡 suggestion · needs: the card's VDD source, power switches and their enable · [catalogue entry](../checks.md#mem-12)

**Impl.** hard · depends on NET-5, NET-10

**What it catches.** An SD socket whose VDD comes straight from a rail (or through a resistor, ferrite or fuse),
with no load switch or regulator enable that firmware controls.

**Why it matters.** Some card states are left only by removing power: after the switch to 1.8 V signalling for
UHS-I the card returns to 3.3 V only through a power cycle, so a host that resets warm cannot re-initialise it;
and a card that hangs after a brown-out or an interrupted write answers no command until it loses power. Without
a switch the only cure is unplugging the card or the board, which is no cure for a logger in the field. A switch
also lets firmware start the card with a controlled ramp and cut it in sleep.

**Example.** B200A feeds `+3V3_SD` from 3.3 V through 2.2 Ω, "not switchable" (hw_b200a.md §11): a card in a bad
state needs the whole rePort unplugged from USB.

**How to check.** Follow the socket VDD pin back to its source (`NET-5`); passives and fuses do not count; a load
switch, FET or regulator does, if its enable reaches an MCU GPIO (`NET-10`).

**Fix.** A small load switch (TPS22917 class, with output discharge) on the card's VDD, enabled by a GPIO with a
pull to "off", and the SD lines driven low or tri-stated while it is off so the card is not fed through its IOs
(`BKF`).

**References.** SD PLS (power-up, signal voltage switch); Wikipedia, SD card "UHS-I"; B200 review notes
(hw_b200a.md §11).
