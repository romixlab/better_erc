# MUX checks, explained

Detail page for the `MUX` category of the [check catalogue](../checks.md#-mux---pin-functions). Each section explains
one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the
usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID, severity and
wording; this page is the commentary.

The **Impl.** line says how hard the check is to build in better_erc without an LLM: `easy` (the netlist alone), `mid`
(part data or a project setting first), `hard` (analysis the code does not have yet) or `judgement` (a human or an LLM
decides, better_erc lists candidates), followed by the [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **stm32-data** · embassy-rs, machine-readable STM32 pin and alternate-function data generated from ST's sources; the
  catalogue links its AF parser (`stm32-data-gen/src/gpio_af.rs`).
  [github.com/embassy-rs/stm32-data](https://github.com/embassy-rs/stm32-data)
- **CMSIS-SVD** · Arm, *Open-CMSIS-SVD: System View Description* (register descriptions, no pin mux).
  [open-cmsis-pack.github.io/svd-spec](https://open-cmsis-pack.github.io/svd-spec/main/index.html)
- **STM32G0B1** · ST datasheet DS13560 Rev 6 (2026): Table 11 (pin structure legend: FT, FT_a, FT_c ...), Table 12 (pin
  assignment, `PF2-NRST`), Table 21 (input voltage per pin structure).
  [st.com/stm32g0b1re.pdf](https://www.st.com/resource/en/datasheet/stm32g0b1re.pdf)
- **RM0444** · ST, *STM32G0x1 reference manual*, Rev 6 (2024): `NRST_MODE` and `nBOOT_SEL` in FLASH_OPTR (§3.7.8),
  PF2-NRST in GPIO mode (§7.3.17).
  [st.com/rm0444](https://www.st.com/resource/en/reference_manual/rm0444-stm32g0x1-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
- **AN5096** · ST, *Getting started with STM32G0 MCUs hardware development*, Rev 4 (2025): NRST modes (§1.3.2), SWD pins
  and their pulls (§4.3.2).
  [st.com/an5096](https://www.st.com/resource/en/application_note/an5096-getting-started-with-stm32g0-series-hardware-development-stmicroelectronics.pdf)
- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, Rev 4 (2026):
  five-volt tolerant GPIO (§5.2.2), no tolerance without VDD (§6.3), debug pins (§6.6).
  [st.com/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **RM0008** · ST, *STM32F101xx ... STM32F107xx reference manual*, Rev 21 (2021): USB and CAN share a 512-byte SRAM
  (§23.2).
  [st.com/rm0008](https://www.st.com/resource/en/reference_manual/rm0008-stm32f101xx-stm32f102xx-stm32f103xx-stm32f105xx-and-stm32f107xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
- **RP2350** · Raspberry Pi, *RP2350 Datasheet*: ADC inputs only on GPIO26-29 (QFN-60) or GPIO40-47 (QFN-80) (§12.4).
  [datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- **GreenPAK** · Renesas, *GreenPAK: Programmable Mixed-Signal Products*, and the SLG46826 product page. [renesas.com
  GreenPAK](https://www.renesas.com/en/products/programmable-mixed-signal-asic-ip-products/greenpak-programmable-mixed-signal-products),
  [renesas.com/en/products/slg46826](https://www.renesas.com/en/products/slg46826)

## `MUX-1` SCL goes to a pin named SCL, MISO to MISO etc <a id="mux-1"></a>

🛑 error · needs: net names, pin names · [catalogue entry](../checks.md#mux-1)

**Impl.** easy · depends on IMP-1

**What it catches.** A net whose name carries an interface signal (`SCL`, `SDA`, `MISO`, `MOSI`, `TX`, `RX`, `D+`)
landing on a pin whose name carries a different one: SDA on an SCL pin, MISO on MOSI, TX on TX.

**Why it matters.** Swapped I2C lines, SPI data lines or UART TX/RX are the most common board bring-up bug and the
cheapest to catch: the schematic tells you twice what each wire is. The catch is UART and SPI naming: `TX → RX` across
parts is correct, and SPI pin names on a peripheral may be from its own point of view (`SDI`, `SDO`, `DIN`).

**Example.** A sensor's `SDA` pin on the net `I2C1_SCL` because the symbol pins were drawn in the other order than the
MCU's: the bus never acknowledges, and the scope shows clock on the data line.

**How to check.** Normalise signal names on nets and pins (`IMP-1`); I2C and clock signals must match, UART TX must meet
RX, SPI MOSI/SDO of the controller must meet MOSI/SDI of the peripheral, and a pin with an alternate-function list
(`NET-12`) is matched against all of its names. better_erc (`erc_checks/src/mux/mux_1.rs`, the signal grammar in
`erc_analysis::names`) reads the I2C, SPI, USB and CAN signal a named net carries (`I2C1_SCL`, `SPI_MOSI`, `USB_D+`,
`CAN_H`) and every pin on it by all its names: the pin name split at `/` (`PB6/I2C1_SCL`), the alternate chosen and the
symbol's alternates. A pin whose names carry signals of the net's interface, none of them the net's, is a finding;
`SDI`, `SDO`, `DIN` and `DOUT` fit either SPI data net, since they are named from the part's side. UART is judged pin
against pin, because a net named `TX` may be named from either end: TX pins of two parts on one net, or RX pins of two
parts with no TX and no connector to bring one, are findings; connector and test point pins are left out. Alternate
functions from part data (`NET-12`) will add the MCU pins whose symbols list none.

**Fix.** Swap the connection, or rename the net if the name is the error.

**References.** The interface specs per bus; [I2C](i2c.md) for bus checks that build on this one.

## `MUX-2` MCUs with SVD/pin data: check the alternate function actually supports the mode used <a id="mux-2"></a>

🛑 error · needs: alternate-function tables of the part · [catalogue entry](../checks.md#mux-2)

**Impl.** mid · depends on NET-12, IMP-9

**What it catches.** A signal routed to a pin that cannot carry it: SPI SCK on a pin with no SCK function, a UART RX on
a TX-only pin, a timer channel on a pin without that timer, two signals of one peripheral instance on pins of different
instances.

**Why it matters.** The schematic symbol of an MCU shows a few names per pin; the full alternate-function table is in
the datasheet. A wrong pin is found when the firmware is written, and the fix is a board revision or bit-banging. SVD
files describe registers, not pins, so the pin data has to come from the datasheet or a generated database (stm32-data
for STM32, the function tables for RP2350).

**Example.** B200A rev A routes the SD card clock to RP2350 GPIO4, which is SPI0 RX, not SCK, and its D0-D3 to GPIO2, 3,
7, 6, which are not consecutive: SPI mode needs PIO or bit-banging, and PIO 4-bit SDIO needs consecutive data pins.
Rework 5 and rev B move CK to GPIO2, CMD to GPIO3 and D0-D3 to GPIO4-7 (B200 rev B change list).

**How to check.** Load the part's alternate-function table (`NET-12` from stm32-data, `IMP-9` from a pinmux sheet),
infer each pin's intended function from the net name or the project's pin map, and check that the pin offers it and that
the signals of one interface share one instance.

**Fix.** Move the signal to a pin with the function; for PIO-style controllers, check the pin-order constraints too.

**References.** stm32-data; CMSIS-SVD; RP2350 datasheet GPIO function table; B200A hardware reference §11.

## `MUX-3` SLG* (GreenPAK): suggest using all pins <a id="mux-3"></a>

💡 suggestion · needs: the part's free pins, nearby signals · [catalogue entry](../checks.md#mux-3)

**Impl.** judgement · depends on netlist

**What it catches.** A GreenPAK (or similar configurable mixed-signal part) with unused pins on a board where nearby
signals could use them: monitoring a power-good, driving an enable, adding an inversion or delay.

**Why it matters.** A GreenPAK is reprogrammable glue: once it is on the board, extra functions cost only a trace. Free
pins routed to enables and status lines make later fixes a configuration change instead of a rework.

**Example.** An SLG46826 that sequences two rails has six free GPIOs; routing the third regulator's `PG` and `EN`
through it would allow fixing the sequence later without a board spin.

**How to check.** List unconnected pins of SLG* parts and the control nets (enables, power-goods, resets) in the same
area; what to route is a design decision.

**Fix.** Route candidates to free pins (with a 0 Ω option where the direct path should stay default).

**References.** Renesas GreenPAK overview; SLG46826 product page.

## `MUX-4` GPIO on a pin that is NRST, BOOT or another reset-time function by default <a id="mux-4"></a>

🛑 error · needs: per-pin reset-time functions, option-byte defaults · [catalogue entry](../checks.md#mux-4)

**Impl.** mid · depends on NET-12, IMP-9

**What it catches.** A pin used as a plain GPIO although at reset (and with factory option bytes) it is the reset input,
a boot pin or another function that the firmware cannot take over before it matters.

**Why it matters.** On the STM32G0 the pin `PF2-NRST` is the reset input/output as delivered (AN5096 §1.3.2); it becomes
PF2 only after `NRST_MODE` in the option bytes is changed (RM0444 §3.7.8), and even then the pin must rise above
VIH(NRST) at power-on or the chip never leaves power-on reset (RM0444 §7.3.17). A pull-down on it, or a load that pulls
it low, holds the MCU in reset.

**Example.** B153A used PF2 as the hub reset output, with R304 10 kΩ to GND. The library symbol said plain `PF2`. In
factory state PF2 is the MCU's NRST: the pull-down fought the internal pull-up of about 40 kΩ (AN4899 §6.7), and
flashing and booting failed until the trace was cut and the hub reset moved to PC13. Rev B names the symbol pin
`PF2-NRST`.

**How to check.** Mark pins whose reset-time function is not GPIO (part data, `NET-12`; pin names like `PF2-NRST`,
`PA14-BOOT0` help but are not reliable); report any use as a GPIO, and any pull or load that would block the reset-time
function.

**Fix.** Use another pin; if the pin must be a GPIO, program the option bytes in production and keep its load compatible
with the reset function at power-on.

**References.** RM0444 §3.7.8, §7.3.17; AN5096 §1.3.2; STM32G0B1 Table 12; B153 rev B change list.

## `MUX-5` Debug pins reused as GPIO <a id="mux-5"></a>

🛑 error · needs: which pins are SWD/JTAG on the part · [catalogue entry](../checks.md#mux-5)

**Impl.** mid · depends on NET-12

**What it catches.** SWDIO, SWCLK, TCK, TMS, TDI, TDO (or a SoC's JTAG) used for an application signal, so the firmware
must reconfigure them as GPIO.

**Why it matters.** Once the firmware takes the pins, the debugger cannot attach; flashing then needs connect-under-
reset (if `NRST` is wired to the probe) or the ROM bootloader. A load on the pin also fights the debug pulls: SWDIO
comes up with a pull-up, SWCLK with a pull-down (AN5096 §4.3.2), and driving them the other way wastes current (AN4899
§6.6).

**Example.** An STM32 board that uses PA13/PA14 for two LEDs because "all other pins were taken": the second firmware
image locks the debugger out, and production programming has to change to connect-under-reset.

**How to check.** Debug functions per pin from part data (`NET-12`); report nets on those pins that also reach other
parts than a debug connector, unless the project marks it as intended (with NRST on the debug connector).

**Fix.** Keep debug pins for debug; if one must be shared, make it an input with a weak load and wire NRST to the debug
connector.

**References.** AN5096 §4.3.2; AN4899 §6.6.

## `MUX-6` Analog signal on a pin without that analog function <a id="mux-6"></a>

🛑 error · needs: analog functions per pin, which nets are analog · [catalogue entry](../checks.md#mux-6)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** A net that carries an analog signal (a divider output, a sensor output, a current-sense amplifier
output, a DAC target) landing on a pin with no ADC, DAC or comparator input.

**Why it matters.** Analog functions exist on a subset of pins only; on the RP2350 the ADC is on GPIO26-29 (QFN-60) or
GPIO40-47 (QFN-80) and nowhere else (RP2350 §12.4). A divider on a digital-only pin reads 0 or 1.

**Example.** A battery divider routed to RP2350A GPIO22 because it was the nearest free pin: the firmware can only
compare it against VIH.

**How to check.** Classify analog nets (connected to an amplifier output, a divider without a digital load, a net named
`*_ADC`, `*_SENSE`, `VBAT_MEAS`; `NET-7`) and require an analog function on the controller pin (`NET-12`).

**Fix.** Move to an ADC-capable pin; check the channel is free and in the right ADC instance for simultaneous sampling.

**References.** RP2350 §12.4; STM32G0B1 Table 12 (ADC_INx per pin); AN4899 §5.2.2 (analog function limits the input
voltage).

## `MUX-7` One peripheral signal on two pins, or two uses of a shared peripheral <a id="mux-7"></a>

🛑 error · needs: alternate functions per pin, peripherals in use · [catalogue entry](../checks.md#mux-7)

**Impl.** mid · depends on NET-12, CFG-2

**What it catches.** The same peripheral signal routed to two pins (USART1_TX on PA9 and on PB6), or two peripherals
used together that share a resource and cannot run at the same time.

**Why it matters.** A peripheral signal can be mapped to one pin at a time; the second net is dead. Shared resources are
worse because they are documented in the reference manual only: on STM32F103 (low- to XL-density) USB and CAN share a
dedicated 512-byte SRAM and cannot be used at the same time (RM0008 §23.2). A board with both a USB port and a CAN bus
"works" with each alone.

**Example.** A USB-to-CAN adapter designed around an STM32F103C8: USB enumerates, CAN works in a test without USB, and
the product cannot do both at once. The fix is an MCU change (F105/F107, G4, or one with FDCAN).

**How to check.** Map each pin to the peripheral signal it carries (`NET-12`); duplicate signals are findings. For
shared resources, a per-family rule list (USB/CAN on F1, shared DMA requests, timers whose channels double as another
function) against the peripherals the project uses (`CFG-2`).

**Fix.** One pin per signal; for shared resources, another instance, another family, or a documented time-sharing in
firmware.

**References.** RM0008 §23.2; stm32-data.

## `MUX-8` 5 V net on a pin that is not 5 V tolerant where the part has tolerant (FT) pins <a id="mux-8"></a>

🛑 error · needs: pin structure per pin, real net voltages · [catalogue entry](../checks.md#mux-8)

**Impl.** mid · depends on NET-5, NET-12

**What it catches.** A net at 5 V (a 5 V UART, a 5 V-pulled open-drain line, a VBUS sense) on a pin that is not 5 V
tolerant, while the same part has tolerant pins with the needed function.

**Why it matters.** STM32 pin tolerance varies per pin: the datasheet's pin table marks each with its structure (FT,
FT_a, FT_f, FT_c, TT ...), and the absolute maximum follows from it (STM32G0B1 Tables 11, 12, 21). Tolerance holds only
in input mode, only with VDD present, and only with no analog function active (AN4899 §5.2.2, §6.3). Moving the signal
to a tolerant pin is often free; adding a translator is not.

**Example.** A 5 V-pulled open-drain fault line from a motor driver on PA4 of an STM32G0B1: PA4 is the one TT_a pin of
the package, 3.6 V tolerant, while every neighbouring pin is FT (STM32G0B1 Tables 11, 12). Moving the line one pin over
removes the problem.

**How to check.** For nets at 5 V (`NET-5`) on controller pins, look up the pin structure (`NET-12`); if it is not
tolerant and an FT pin with the same function exists, suggest the move (otherwise `LVL-10` reports it as an error).

**Fix.** Move to an FT pin, keep it in input mode, and never let the 5 V appear while the MCU is unpowered; otherwise a
translator or divider.

**References.** STM32G0B1 Tables 11, 12, 21; AN4899 §5.2.2, §6.3.
