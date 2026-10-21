# BOOT checks, explained

Detail page for the `BOOT` category of the [check catalogue](../checks.md#-boot---boot-modes-and-strapping-pins). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs
to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for
ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to build in better_erc without an LLM: `easy` (the netlist alone), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet) or `judgement` (a human or an LLM
decides, better_erc lists candidates), followed by the [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **AN2606** · ST, *Introduction to system memory boot mode on STM32 MCUs*, Rev 70 (2026). Activation patterns (§4.1,
  Table 2), hardware requirements and floating RX pins (§4.3), the STM32G0B1 bootloader (§50.1, Table 111).
  [st.com/an2606](https://www.st.com/resource/en/application_note/an2606-introduction-to-system-memory-boot-mode-on-stm32-mcus-stmicroelectronics.pdf)
- **AN5096** · ST, *Getting started with STM32G0 MCUs hardware development*, Rev 4 (2025). Boot configuration with
  `nBOOT_SEL` (§3, Table 3), boot pin only after reprogramming the option byte (§6.1.3).
  [st.com/an5096](https://www.st.com/resource/en/application_note/an5096-getting-started-with-stm32g0-series-hardware-development-stmicroelectronics.pdf)
- **RM0444** · ST, *STM32G0x1 reference manual*, Rev 6 (2024): `nBOOT_SEL` in FLASH_OPTR (§3.7.8).
  [st.com/rm0444](https://www.st.com/resource/en/reference_manual/rm0444-stm32g0x1-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
- **ESP32** · Espressif, *ESP32 Series Datasheet* v5.3: boot configurations (§3), strapping timing (Table 3-2), boot
  mode (§3.1, Table 3-3), VDD_SDIO voltage from MTDI/GPIO12 (§3.2).
  [espressif.com/…/esp32_datasheet_en.pdf](https://www.espressif.com/sites/default/files/documentation/esp32_datasheet_en.pdf)
- **ESP32 checklist** · Espressif, *ESP Hardware Design Guidelines*, Schematic Checklist (ESP32): strapping pins,
  CHIP_PU RC delay, GPIO0 near the EMAC clock.
  [docs.espressif.com/…/schematic-checklist.html](https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32/schematic-checklist.html)
- **esptool** · Espressif, *Boot Mode Selection* (ESP32): the DTR/RTS auto-reset circuit.
  [docs.espressif.com/…/boot-mode-selection.html](https://docs.espressif.com/projects/esptool/en/latest/esp32/advanced-topics/boot-mode-selection.html)
- **RP2350 HW** · Raspberry Pi, *Hardware design with RP2350*: QSPI_SS as the BOOTSEL strap (§3.1), supported flash
  chips and the bootrom's probe sequence (§3.3), buttons (§5.4).
  [datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf)
- **USB251xB** · Microchip, *USB251xB/xBi USB 2.0 Hi-Speed Hub Controller*, DS00001692C: configuration options (Table
  5-1), SMBus configuration (§5.3), RESET_N (§5.5.1).
  [ww1.microchip.com/…/00001692C.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf)
- **LAN8720A** · Microchip, *LAN8720A/LAN8720AI* datasheet, DS00002165B: configuration straps (§3.7), REGOFF on LED1
  (§3.7.3), nINTSEL on LED2 (§3.7.4).
  [ww1.microchip.com/…/00002165B.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00002165B.pdf)
- **FPGA-TN-02039** · Lattice, *ECP5 and ECP5-5G sysCONFIG User Guide*, v2.5 (2026): dedicated sysCONFIG pins (§4.6:
  CFGMDN §4.6.1, PROGRAMN §4.6.2, INITN §4.6.3, DONE §4.6.4) and the pull-up notes under the pin table.
  [latticesemi.com/view_document?document_id=50462](https://www.latticesemi.com/view_document?document_id=50462)
- **UG470** · AMD, *7 Series FPGAs Configuration User Guide*, v1.17 (2023): mode pins, PROGRAM_B, INIT_B, DONE, PUDC_B.
  [docs.amd.com/v/u/en-US/ug470_7Series_Config](https://docs.amd.com/v/u/en-US/ug470_7Series_Config)
- **Wikipedia, Booting** · a readable overview. [en.wikipedia.org/wiki/Booting](https://en.wikipedia.org/wiki/Booting)

## `BOOT-1` Boot or strapping pin with no defined level at reset <a id="boot-1"></a>

🛑 error · needs: strap pins of the part, what drives them at reset · [catalogue entry](../checks.md#boot-1)

**Impl.** mid · depends on NET-7, NET-9, NET-12

**What it catches.** A pin that a part samples at reset to choose its boot mode or configuration (BOOT0, ESP32
GPIO0/2/5/12/15, RP2350 QSPI_SS, i.MX BOOT_CFG, FPGA mode pins) with no pull and no driver that is already valid at that
moment.

**Why it matters.** The value is latched once, during a window of about a millisecond after reset (ESP32: hold time 1 ms
after CHIP_PU, Table 3-2); a floating pin is latched at whatever it shows then. The board boots most of the time and
occasionally lands in the ROM bootloader or the wrong flash voltage, which is hard to reproduce.

**Example.** An STM32 with BOOT0 left unconnected "because the option bytes ignore it": with factory option bytes the
pin is used (STM32F1, F4, and the G0 with `nBOOT_SEL` = 0, AN5096 Table 3), and the board starts the ROM bootloader on
every second power-up.

**How to check.** Strap roles per pin (`NET-12` and part data, `NET-7`); the net needs a resistor to a rail or a driver
whose reset state is defined at sampling time (an MCU GPIO is not, `NET-9`).

**Fix.** An explicit pull per strap, at the value the design wants (`BOOT-3`).

**References.** ESP32 §3, Table 3-2; AN5096 §3; AN2606 §4.1.

## `BOOT-2` Strapping pin shared with a signal that another device may drive at reset <a id="boot-2"></a>

🛑 error · needs: strap pins, reset-time states of the other drivers · [catalogue entry](../checks.md#boot-2)

**Impl.** hard · depends on NET-8, NET-9, NET-10

**What it catches.** A strap pin that also connects to an output of another part which may be active during the latch
window: a USB-UART bridge on ESP32 GPIO0, a card or display on an i.MX boot config pin, a clock output, a PHY LED whose
pull sets its own strap.

**Why it matters.** The strap's resistor sets the level only if nothing else drives the pin while it is sampled. Another
part that powers up first, runs its own reset sequence, or outputs a clock overrides it.

**Example.** ESP32 GPIO0 used as the RMII reference clock input from an Ethernet PHY: the clock runs during power-up and
can latch GPIO0 low, so the chip enters download mode (ESP32 checklist, Ethernet MAC). The same shape on a LAN8720A: an
LED from LED1 to VDD2A is a pull-up on the REGOFF strap, which turns the PHY's internal regulator off (LAN8720A §3.7.3).

**How to check.** For each strap pin, follow its net to every other output (`NET-10`) and evaluate whether that output
can be active during the latch window: powered (`NET-8`) and out of its own reset, with its reset state (`NET-9`).

**Fix.** Move the signal off the strap, gate it until after the latch (a buffer with OE, or the other part held in
reset), or use a strong strap with a series resistor towards the other driver (`BOOT-9`).

**References.** ESP32 checklist (Strapping Pins, Ethernet MAC); LAN8720A §3.7, §3.7.3; ESP32 Table 3-2.

## `BOOT-3` Strapping pin pulled to a mode the design does not want <a id="boot-3"></a>

🛑 error · needs: strap decode per part, the wanted mode · [catalogue entry](../checks.md#boot-3)

**Impl.** mid · depends on NET-12, CFG-2

**What it catches.** A strap with a defined level that selects the wrong mode: STM32 BOOT0 pulled high (boots the ROM),
ESP32 GPIO12 (MTDI) high (selects 1.8 V for VDD_SDIO), a PHY address strap giving two PHYs one address.

**Why it matters.** Wrong modes may not show in the lab: an ESP32 module with 3.3 V flash and GPIO12 pulled high starts
its flash at 1.8 V and fails intermittently or not at all (ESP32 §3.2). A pull-up added for another function on a strap
pin changes the boot mode silently.

**Example.** A 10 kΩ pull-up on ESP32 GPIO12, added because the pin was used as an SD card data line (SDIO wants
pull-ups): MTDI = 1 at reset selects the internal 1.8 V LDO for VDD_SDIO, and a 3.3 V flash browns out.

**How to check.** Decode each strap's latched value from its pulls (part data, `NET-12`) and compare with the mode the
project wants (`CFG-2`), defaulting to "boot from flash, normal voltages".

**Fix.** Flip the pull, move the other function off the strap, or burn the eFuse/option byte that overrides the pin
(ESP32 `EFUSE_SDIO_FORCE`, ESP32 §3.2).

**References.** ESP32 §3.1-3.2; esptool Boot Mode Selection; AN5096 Table 3.

## `BOOT-4` Boot pin wired but ignored by the factory option bytes or fuses <a id="boot-4"></a>

⚠️ warning · needs: per-family boot selection rules · [catalogue entry](../checks.md#boot-4)

**Impl.** mid · depends on NET-12, CFG-2

**What it catches.** A boot button, jumper or header pin on a part whose factory configuration ignores the pin, so it
only works after a programming step that sets an option byte or fuse.

**Why it matters.** On the STM32G0 the BOOT0 signal comes from the pin only when `nBOOT_SEL` = 0 (RM0444 §3.7.8, AN5096
Table 3); ST ships the boot option configured through the option bytes and says to use the pin only after reprogramming
that byte (AN5096 §6.1.3). If production programming does not set it, the boot button does nothing.

**Example.** B153A's boot button (Q204 → BOOT0 on PA14) did nothing on the first boards: `FLASH_OPTR` read 0xFFFFFEAA
(`nBOOT_SEL` = 1, pin ignored). Production programming now has to set it, and with the pin in use, SWCLK held high at
reset also enters the ROM (B153 rev B change list).

**How to check.** A per-family rule (`NET-12` part data): parts whose boot pin depends on an option byte, fuse or OTP
bit, with a boot control wired; report it with the setting that enables it, for the bring-up notes and the programmer
report (`CFG-2`).

**Fix.** Say so in the bring-up notes and the production programming script; or drop the pin and use a software entry
into the ROM.

**References.** AN5096 §3 Table 3, §6.1.3; RM0444 §3.7.8; AN2606 Table 2 (pattern 11).

## `BOOT-5` No way to force the ROM bootloader on a board updated in the field <a id="boot-5"></a>

💡 suggestion · needs: the part's boot entry pins, buttons and connectors · [catalogue entry](../checks.md#boot-5)

**Impl.** mid · depends on NET-12

**What it catches.** A controller with a ROM bootloader whose entry condition (a boot pin, QSPI_SS low, GPIO0 low)
cannot be reached from outside: no button, no jumper, no header pin, no test pad.

**Why it matters.** Firmware that boots and hangs before its own update code runs can only be fixed through the ROM.
Without a way to force it, every such board needs a debug probe or a return.

**Example.** B200A has no BOOT button, but `BOOT_EXT` on header J103 pin 5 goes through 255 Ω and 1 kΩ to QSPI_SS:
shorting it to the GND pin next to it while plugging in starts the RP2350's USB bootloader (B200A hardware reference
§3). The RP2350 reference design uses a button on QSPI_SS through 1 kΩ (RP2350 HW §3.1, §5.4).

**How to check.** The ROM entry pin of each programmable part (`NET-12`); require a path to a switch, jumper, connector
or test point, through at most a resistor.

**Fix.** A button or a pad pair on the boot pin, through a resistor so it can override the pin's normal use.

**References.** RP2350 HW §3.1, §5.4; ESP32 §3.1; AN2606 §4.1.

## `BOOT-6` ROM bootloader interface not reachable from outside with the firmware dead <a id="boot-6"></a>

🛑 error · needs: bootloader interfaces, what lies on the way out · [catalogue entry](../checks.md#boot-6)

**Impl.** hard · depends on NET-8, NET-10, NET-12, NET-13

**What it catches.** The ROM bootloader's interface (USB DFU, a UART, CAN) reaching the connector only through a part
that firmware must configure first: a hub, a mux, a level shifter, a PHY.

**Why it matters.** The ROM runs with the board in its reset defaults. Any part between the controller and the outside
that needs configuration, a GPIO level or a released reset stays in its default, and the ROM talks into nothing
(`LOGIC-5`).

**Example.** B153A: the STM32G0B1's USB goes through hub U301, strapped for SMBus configuration (USB251xB Table 5-1) and
held in reset by a pull-down after every MCU reset. The ROM's USB DFU runs and never reaches the host. B125B had the
same shape with a ULPI PHY: the change list for rev B says to keep it in reset so the built-in bootloader works.

**How to check.** From each bootloader interface (`NET-12`), follow the signal to the connectors (`NET-10`, across
boards with `NET-13`) and check every part on the way in its reset-default state (`NET-8`): a switch's default position,
a hub's strap mode, a translator's OE pull.

**Fix.** A path that works with the controller in reset: a default-on mux position, a strap that makes the hub work
alone, or a separate connector for the ROM's interface.

**References.** AN2606 §4.3, §50.1; USB251xB §5.3; B153 and B125 rev B change lists.

## `BOOT-7` Boot memory on pins or a bus the ROM cannot boot from <a id="boot-7"></a>

🛑 error · needs: the ROM's boot pins and supported memories · [catalogue entry](../checks.md#boot-7)

**Impl.** mid · depends on NET-12

**What it catches.** A boot flash or card on a chip select, bus instance or pins the ROM does not scan, or a flash whose
command set or mode the ROM's loader does not handle.

**Why it matters.** The ROM is fixed: it reads one bus, one chip select, a fixed command. The RP2350 bootrom probes with
an 03h serial read at 24-bit addressing and needs 02h, 05h, 06h and 20h for programming; a flash left in a
continuous-read XIP mode that does not answer the bootrom's exit sequence cannot be recovered after an internal reset
(RP2350 HW §3.3). An SoC booting from SD looks at one SDIO instance only.

**Example.** A design moves the QSPI flash of an RP2350 to the secondary chip select (`XIP_CS1n` on GPIO0) to free
QSPI_SS for a PSRAM: the RP2350 has no internal flash, the bootrom looks only behind QSPI_SS, and the board has nothing
to boot (RP2350 HW §3.1-3.2).

**How to check.** The ROM's boot sources per family (`NET-12`): pins, instance, chip select, and the memory's
command-set class from its part data; the memory found on the board must match one of them.

**Fix.** Put the boot memory where the ROM looks, or pick a memory the ROM supports.

**References.** RP2350 HW §3.1-3.3; the SoC's boot ROM chapter.

## `BOOT-8` Part strapped for a configuration that assumes a controller <a id="boot-8"></a>

⚠️ warning · needs: strap decode per part, which controller configures it · [catalogue entry](../checks.md#boot-8)

**Impl.** mid · depends on NET-7, CFG-5

**What it catches.** A part whose straps select "configured by a host" (SMBus or I2C slave configuration, register load
from a controller), so it does nothing useful until that controller runs.

**Why it matters.** In host-configured mode the defaults are often empty. The USB251xB in SMBus mode disables its
straps, waits indefinitely for the code load and attaches only after it (USB251xB Table 5-1, §5.3); its registers reset
to their defaults, in most cases 00h (§5.5.1). That is fine while the controller always starts first and always works,
and wrong for any recovery path (`BOOT-6`).

**Example.** B153A's hub is strapped CFG_SEL[1:0] = 01 (SMBus): the runtime board-variant detection could not use the
hub's PID because it powers up with every register at zero, and the ROM bootloader cannot reach the host (B153 rev B
change list).

**How to check.** Decode the strap mode of hubs, PHYs, codecs and PMICs (part data, `NET-7`, rules per part in `CFG-5`);
report host-configured modes with the controller that does it.

**Fix.** A standalone default mode with straps or an EEPROM, if the part must work without the controller; otherwise
document the dependency.

**References.** USB251xB Table 5-1, §5.3, §5.5.1.

## `BOOT-9` Strapping pin shared with a peripheral signal without a series resistor <a id="boot-9"></a>

⚠️ warning · needs: strap pins, what else is on the net · [catalogue entry](../checks.md#boot-9)

**Impl.** mid · depends on NET-12, IMP-1

**What it catches.** A strap pin that also carries a peripheral signal (an SD data line, a UART, an LED, a PHY input),
with the peripheral connected directly instead of through a series resistor.

**Why it matters.** During the latch window the peripheral's input clamp, pull or leakage loads the strap; the strap
resistor must win. A series resistor between the strap node and the peripheral keeps the peripheral's influence small
during reset and costs little at signal speed. LAN8720A asks for an external strap resistor whenever a strap pin drives
a load (LAN8720A §3.7).

**Example.** ESP32 GPIO2 (a boot-mode strap) wired straight to an SD card's D0 with its 10 kΩ pull-up: GPIO2 must be low
or floating for download mode, and the card's pull-up makes flashing over UART fail until the card is removed.

**How to check.** Strap pins (`NET-12`) whose nets reach other parts' pins with no resistor between the strap node and
those pins (`IMP-1`).

**Fix.** A series resistor (1-10 kΩ) towards the peripheral, the strap resistor on the controller side; or move one of
the two functions.

**References.** LAN8720A §3.7; ESP32 Table 3-3; ESP32 checklist (Strapping Pins, SDIO).

## `BOOT-10` FPGA configuration pins floating or without the required pulls <a id="boot-10"></a>

🛑 error · needs: configuration pin roles and pull rules per family · [catalogue entry](../checks.md#boot-10)

**Impl.** mid · depends on NET-7, IMP-10

**What it catches.** Mode select pins (MSEL, CFG, M[2:0], CFGMDN), PROGRAM_B/nCONFIG/PROGRAMN, INIT_B/nSTATUS/INITN,
DONE/CONF_DONE, PUDC_B and nCE/nCEO left floating or without the pulls the configuration guide requires.

**Why it matters.** FPGA configuration is a handshake on open-drain and weakly pulled pins. Lattice ECP5: CFGMDN has
weak internal pull-ups, a low needs an external pull-down under 500 Ω, 4.7 kΩ pull-ups are recommended; PROGRAMN, INITN
and DONE have weak pulls or are open drain; the MSPI chip select needs 4.7 kΩ and MCLK 1 kΩ (FPGA-TN-02039 §4.6). AMD
7-series have the same set under other names (UG470). Missing pulls give an FPGA that configures on the bench and not in
the cold, or a DONE that never reaches the level a supervisor waits for.

**Example.** An ECP5 board with CFGMDN[2:0] tied to GND through 10 kΩ "pull-downs": against the internal pull-ups and
leakage the pins read high on some parts, the FPGA tries slave serial instead of master SPI and never configures.

**How to check.** Configuration roles per pin from the vendor's pin file (`IMP-10`), pull rules per family as part data
(`NET-7`); each required pull present, of the right direction and within the stated value.

**Fix.** The pulls from the configuration guide's reference schematic, with values as stated there.

**References.** FPGA-TN-02039 §4.6.1-4.6.4 and the notes under its pin table; UG470.
