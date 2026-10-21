# Better ERC - check catalogue

What better_erc should catch on a schematic or netlist: the checks only, no implementation status. Status of the
code (which checks run, parsers, reports, UI) lives in [FEATURES.md](../FEATURES.md) and refers to these IDs. The
condensed [schematic review checklist](review-checklist.md) links back here by ID.

- **IDs** (`PIN-1`, `I2C-3`) are stable: never renumber or reuse. A new check takes the next free number of its
  category. A hidden comment (`<!-- CHK-59 -->`) on some entries keeps the number from an earlier internal tracker.
- **Severity** of the finding: 🛑 error (board won't work or gets damaged) · ⚠️ warning (risk or margin, depends
  on the design) · 💡 suggestion (debug aids, tidiness, conventions).
- `≈` marks overlaps: merge when implementing.
- A miss found at assembly or bring-up becomes a new check here, so the list grows from real failures.

## Power

### 🛡️ PIN - Power input and output protection
- <a id="pin-1"></a>⚠️ `PIN-1` No reverse polarity, overcurrent or overvoltage protection on power input. No input filter. <!-- CHK-59 -->
- <a id="pin-2"></a>⚠️ `PIN-2` Fast blow fuses used; fuse rating vs expected current <!-- CHK-136 -->
- <a id="pin-3"></a>⚠️ `PIN-3` Power outputs on connectors from nets used on board: suggest an eFuse or switch with short and overload protection <!-- CHK-111 -->

### 🔋 RAIL - Rails and regulators
- <a id="rail-1"></a>🛑 `RAIL-1` Power input pins of parts not powered <!-- CHK-69 -->
- <a id="rail-2"></a>🛑 `RAIL-2` DC-DC or LDO ref voltage known: find the divider, calculate output, compare with net name or specified voltage ≈ RAIL-3 <!-- CHK-87 -->
- <a id="rail-3"></a>⚠️ `RAIL-3` Net name carries a voltage (e.g. `_3V3`) that is wrong <!-- CHK-142 -->

### 🧱 DEC - Decoupling
- <a id="dec-1"></a>⚠️ `DEC-1` No decoupling capacitors on power nets <!-- CHK-26 -->
- <a id="dec-2"></a>💡 `DEC-2` Recommended decoupling values <!-- CHK-184 -->
- <a id="dec-3"></a>🛑 `DEC-3` STM32 VCAP pins need 2.2 µF ceramic caps <!-- CHK-196 -->
- <a id="dec-4"></a>💡 `DEC-4` Caution against ferrite beads without measurements <!-- CHK-82 -->

### ⏯️ SEQ - Sequencing, enables, inrush, discharge
- <a id="seq-1"></a>⚠️ `SEQ-1` Power sequencing: intermittent problems on power on or off <!-- CHK-135 -->
- <a id="seq-2"></a>🛑 `SEQ-2` Feedback or bootstrap loops, e.g. EN set by a divider powered by the device's own output, never starts <!-- CHK-199 -->
- <a id="seq-3"></a>⚠️ `SEQ-3` SD card insertion inrush: no power switch or resistor; decoupling after that switch or resistor <!-- CHK-76 -->
- <a id="seq-4"></a>⚠️ `SEQ-4` Soft start circuits with a MOSFET or resistor as load; if load is specified, report performance <!-- CHK-131 -->
- <a id="seq-5"></a>⚠️ `SEQ-5` High voltage net without self-discharge resistors <!-- CHK-137 -->

### 🌐 GND - Grounds and isolation
- <a id="gnd-1"></a>⚠️ `GND-1` Split grounds without an isolated circuit: warn, point to books and pages <!-- CHK-60 -->
- <a id="gnd-2"></a>💡 `GND-2` Suggest a Y cap between isolated grounds <!-- CHK-92 -->

### 🎚️ DRV - MOSFETs, switches and drivers
- <a id="drv-1"></a>🛑 `DRV-1` MOSFET gate threshold vs driving level <!-- CHK-30 -->
- <a id="drv-2"></a>🛑 `DRV-2` High side MOSFETs driven incorrectly <!-- CHK-126 -->
- <a id="drv-3"></a>⚠️ `DRV-3` PWM from MCU or SoC pins driving transistors directly: suggest a driver to protect the pin <!-- CHK-61 -->
- <a id="drv-4"></a>⚠️ `DRV-4` Several gates on one driver: check charge pump capacity <!-- CHK-132 -->
- <a id="drv-5"></a>⚠️ `DRV-5` MOSFET as inverter: gate resistor to stop Cgd pulling the output below ground <!-- CHK-215 -->

    Glitches possible on turn off, Miller capacitance, rM

- <a id="drv-6"></a>🛑 `DRV-6` Q-Q chain not back to back, e.g. pogo-pin VBUS protection; double check thresholds <!-- CHK-178 -->
- <a id="drv-7"></a>⚠️ `DRV-7` No snubbers on half bridge output <!-- CHK-127 -->
- <a id="drv-8"></a>🛑 `DRV-8` Flyback without feedback and no resistive or zener load on output <!-- CHK-140 -->
- <a id="drv-9"></a>🛑 `DRV-9` Relays without flyback diode <!-- CHK-103 -->

## Shorts and leakage

### 💥 SHORT - Shorts and net collisions
- <a id="short-1"></a>🛑 `SHORT-1` Net shunting a known part (R, C, L, transistor...): most likely a net label collision, or a port and net label with the same name ≈ SHORT-2 <!-- CHK-1 -->
- <a id="short-2"></a>🛑 `SHORT-2` Accidental net name collision, e.g. on I2C: VCCa-R-R-VCCa/b, or two 0R leading to the same net <!-- CHK-158 -->
- <a id="short-3"></a>🛑 `SHORT-3` Non-DNM part shunting another mounted part <!-- CHK-152 -->
- <a id="short-4"></a>🛑 `SHORT-4` Short circuit through low value resistor(s), diode, body diode, TVS <!-- CHK-5 -->
- <a id="short-5"></a>🛑 `SHORT-5` VDD/VCC pins shorted to GND, or GND/VEE to power nets <!-- CHK-179 -->
- <a id="short-6"></a>🛑 `SHORT-6` Inputs shorted to ground or power without a resistor <!-- CHK-189 -->
- <a id="short-7"></a>⚠️ `SHORT-7` Shorted IOs: warn that they must be driven together <!-- CHK-160 -->
- <a id="short-8"></a>⚠️ `SHORT-8` Direct connections to battery and other high energy nets <!-- CHK-12 -->

### ↩️ BKF - Backfeed between power domains
- <a id="bkf-1"></a>🛑 `BKF-1` Power backfeed through IO, suggest Ioff capable buffers <!-- CHK-2 -->
- <a id="bkf-2"></a>🛑 `BKF-2` General case: two power domains, IO across, one domain goes off, IO powers it through protection diodes ≈ BKF-1 <!-- CHK-3 -->
- <a id="bkf-3"></a>🛑 `BKF-3` Same through a MOSFET body diode, e.g. in an I2C voltage translator <!-- CHK-4 -->
- <a id="bkf-4"></a>🛑 `BKF-4` Backfeed through MOSFETs, e.g. in DC-DC converters <!-- CHK-123 -->
- <a id="bkf-5"></a>⚠️ `BKF-5` Backfeed through LDOs <!-- CHK-124 -->
- <a id="bkf-6"></a>⚠️ `BKF-6` Backfeed through internal IC structure where known (beyond IO protection diodes) <!-- CHK-125 -->
- <a id="bkf-7"></a>⚠️ `BKF-7` Power through IO or I2C lines; also spots 1-Wire, can exclude <!-- CHK-177 -->
- <a id="bkf-8"></a>⚠️ `BKF-8` I2C device on a rail that can be off (backfeed should catch this) <!-- CHK-19 -->

### 🪫 LEAK - Leakage and low power
- <a id="leak-1"></a>⚠️ `LEAK-1` Resistor dividers straight on a power rail draining power, or through a normally closed button <!-- CHK-44 -->
- <a id="leak-2"></a>💡 `LEAK-2` Pins that can't be high in some power states (go to another domain): list them even if the warning is suppressed <!-- CHK-147 -->
- <a id="leak-3"></a>💡 `LEAK-3` NC pins: suggest setting them to analog or pulling them to cut power draw <!-- CHK-143 -->

## Logic and IO

### 📶 LVL - Logic levels and IO banks
- <a id="lvl-1"></a>🛑 `LVL-1` Pull-up, divider or other circuitry to a different voltage level (needs IO bank voltage; easy for single supply parts) <!-- CHK-9 -->
- <a id="lvl-2"></a>🛑 `LVL-2` 1V8 output to 3V3 input and similar: compare thresholds <!-- CHK-67 -->
- <a id="lvl-3"></a>💡 `LVL-3` I2C thresholds may allow skipping a translator, e.g. 1V8 into a 3V3 device with Vth 1.4 V <!-- CHK-24 -->
- <a id="lvl-4"></a>🛑 `LVL-4` I2C discrete translator pull-up to wrong voltage, wrong threshold <!-- CHK-161 -->
- <a id="lvl-5"></a>⚠️ `LVL-5` Bidirectional level shifter used for unidirectional signals (e.g. USART + boot mode) <!-- CHK-206 -->
- <a id="lvl-6"></a>💡 `LVL-6` A better suited bank exists for the net voltage or target bank voltage <!-- CHK-156 -->

    For example on SPLD +3V3 side used, but +1V8 is the same side were net goes

- <a id="lvl-7"></a>⚠️ `LVL-7` Several power rails on a part and no banks defined: warn <!-- CHK-186 -->
- <a id="lvl-8"></a>🛑 `LVL-8` Signal not referenced to GND, e.g. an inverting buck and its enable signal <!-- CHK-213 -->

### 🎛️ IO - Default states and pulls
- <a id="io-1"></a>⚠️ `IO-1` Input pins without a drive source, smart about resistors, FETs etc. Also flag pins going to connectors ≈ IO-2 <!-- CHK-14 -->
- <a id="io-2"></a>⚠️ `IO-2` IO to an input without pull-up/down: High-Z gives an undefined state <!-- CHK-201 -->
- <a id="io-3"></a>🛑 `IO-3` Enables on MCU/SoC GPIOs or MOSFET gates without a pull: think of uninit and reboot state; `default_gpio_state` part property overrides <!-- CHK-62 -->
- <a id="io-4"></a>⚠️ `IO-4` Assume IO pins are high/low/Z by default and look for bugs ≈ IO-5 <!-- CHK-168 -->
- <a id="io-5"></a>⚠️ `IO-5` If IO pin default states are known, derive checks <!-- CHK-167 -->
- <a id="io-6"></a>⚠️ `IO-6` High impedance nets acting as touch buttons <!-- CHK-130 -->
- <a id="io-7"></a>🛑 `IO-7` Button to ground with a pull-down, or button to power with a pull-up ≈ IO-8 <!-- CHK-175 -->
- <a id="io-8"></a>💡 `IO-8` Mention a needed pull-up or down where it applies, e.g. buttons <!-- CHK-145 -->
- <a id="io-9"></a>🛑 `IO-9` Pins a part drives in its non-firmware modes: ROM bootloader, factory test, reset and fault states. A ROM bootloader configures its interfaces (UART, I2C, SPI, CAN, USB) and may drive or toggle those pins while it scans for a host, with no firmware to keep anything else safe; check what each such pin controls (switches, enables, buses, resets) and what that level does there. Example: an STM32 ROM bootloader drives its USART TX pin high while it scans for a host; if that pin also selects a USB data switch, a firmware update over the ROM cuts the downstream device's data ≈ IO-3, IO-4

### 🧩 LOGIC - Pin logic and fan-out
- <a id="logic-1"></a>🛑 `LOGIC-1` Logical contradictions in GPIO or GPIO expander connections · AI prompt: check that GPIOs that drive multiple things have no logical contradictions, table of each occurrence <!-- CHK-222 -->
- <a id="logic-2"></a>⚠️ `LOGIC-2` IO going to two MCUs or SoCs: odd requirements likely, esp. IRQs and buttons not going to reset inputs <!-- CHK-174 -->
- <a id="logic-3"></a>💡 `LOGIC-3` Suggest buffering high fan-out signals <!-- CHK-83 -->
- <a id="logic-4"></a>⚠️ `LOGIC-4` Total IO current per IC or bank <!-- CHK-112 -->
- <a id="logic-5"></a>🛑 `LOGIC-5` Dependency loops in bring-up, update or recovery: part A only works once part B has set it up, while B needs A to start, update or recover. Then the board works while the firmware runs and fails exactly when it is needed (dead or half-written firmware, reset, ROM bootloader). Walk every boot, update and recovery path with each programmable part in its reset or unconfigured state. Example: an MCU whose only update path (its USB ROM bootloader) goes through a USB hub that the MCU itself must release from reset and configure over I2C: after a reset the bootloader never reaches the host. Same shape elsewhere: a sequencer or supervisor powered or configured by what it controls, a clock or PHY configured by the chip it serves, a config memory written through the bus it configures, a mux or level shifter on the programming path driven by the target ≈ SEQ-2

### 🔀 MUX - Pin functions
- <a id="mux-1"></a>🛑 `MUX-1` SCL goes to a pin named SCL, MISO to MISO etc <!-- CHK-159 -->
- <a id="mux-2"></a>🛑 `MUX-2` MCUs with SVD/pin data: check the alternate function actually supports the mode used <!-- CHK-108 -->

    - Load alternate modes CSV for SoCs

    [https://github.com/embassy-rs/stm32-data/blob/d828c292da3dfb6679e7b126db5b1c4408dc6ec6/stm32-data-gen/src/gpio_af.rs#L48](https://github.com/embassy-rs/stm32-data/blob/d828c292da3dfb6679e7b126db5b1c4408dc6ec6/stm32-data-gen/src/gpio_af.rs#L48)

- <a id="mux-3"></a>💡 `MUX-3` SLG* (GreenPAK): suggest using all pins, e.g. to monitor things, connect to ENs, route EN through if inverting or delay is needed <!-- CHK-198 -->

### ⏱️ CLK - Clocks and reset
- <a id="clk-1"></a>🛑 `CLK-1` MCU with CAN or USB and no crystal <!-- CHK-100 -->
- <a id="clk-2"></a>⚠️ `CLK-2` HS crystal capacitors should be high frequency rated <!-- CHK-79 -->
- <a id="clk-3"></a>⚠️ `CLK-3` Crystal NC pins may need to go to ground <!-- CHK-33 -->
- <a id="clk-4"></a>⚠️ `CLK-4` Crystal pins used as IO: warn about low drive strength <!-- CHK-173 -->
- <a id="clk-5"></a>⚠️ `CLK-5` No cap and pull-up on reset_n inputs <!-- CHK-70 -->
- <a id="clk-6"></a>💡 `CLK-6` Suggest a cap on buttons to reset inputs; note hw debounce in the programmer report <!-- CHK-133 -->

## Interfaces

### 🚌 I2C - I2C
- <a id="i2c-1"></a>🛑 `I2C-1` I2C pull-ups present; suggest standard values if far off the recommended (3k3?) <!-- CHK-20 -->
- <a id="i2c-2"></a>⚠️ `I2C-2` I2C multiple pull-ups <!-- CHK-107 -->
- <a id="i2c-3"></a>🛑 `I2C-3` Pull-up from two series resistors to nowhere (part of I2C-1) <!-- CHK-162 -->
- <a id="i2c-4"></a>🛑 `I2C-4` Pull-up to a non-rail net (resistor with only two pins connected)? <!-- CHK-163 -->
- <a id="i2c-5"></a>🛑 `I2C-5` I2C address collision (needs addresses in the library, maybe AI can pull them out) <!-- CHK-15 -->
- <a id="i2c-6"></a>💡 `I2C-6` I2C going to a connector that isn't FPC or fine pitch B2B: suggest another interface, keep I2C on board? <!-- CHK-99 -->
- <a id="i2c-7"></a>⚠️ `I2C-7` Series tie resistor on an I2C line too high (over 100 Ω) <!-- new 5 Oct 2026, found in code -->
- <a id="i2c-8"></a>⚠️ `I2C-8` Pull-up too weak for the bus speed: Rp > tr_max / (0.847 · Cb), tr (30-70 %) max 1000 ns at 100k, 300 ns at 400k, 120 ns at 1M; Cb estimated from the pins on the net (or a project setting). Measured on a bench: ~135 pF per line, 10k gives 1170 ns (fails 100k), 2.2k 277 ns (meets 400k), 1M needs ≤ 1.05k <!-- new 10 Oct 2026 -->

### 🔁 SER - UART, SPI, CAN, LVDS and whole-bus checks
- <a id="ser-1"></a>🛑 `SER-1` CAN TX and RX not swapped, USART TX and RX swapped <!-- CHK-115 -->
- <a id="ser-2"></a>🛑 `SER-2` SPI CS line grounded: some ICs need it to work <!-- CHK-27 -->
- <a id="ser-3"></a>⚠️ `SER-3` CAN terminator based on MOSFETs or SSRs <!-- CHK-122 -->
- <a id="ser-4"></a>🛑 `SER-4` LVDS without terminators <!-- CHK-138 -->
- <a id="ser-5"></a>🛑 `SER-5` Not the whole bus is present, maybe nets shorted out ≈ SER-6 <!-- CHK-166 -->
- <a id="ser-6"></a>🛑 `SER-6` Not the whole bus is connected to a device <!-- CHK-164 -->

### 💾 MEM - Memory (DDR, eMMC, SD)
- <a id="mem-1"></a>⚠️ `MEM-1` eMMC and SD pull-up checker (needs a full list compiled) <!-- CHK-31 -->
- <a id="mem-2"></a>🛑 `MEM-2` DDR pin swap across bytes and other errors <!-- CHK-40 -->

### ⚡ HS - High speed
- <a id="hs-1"></a>🛑 `HS-1` PCIe, DP: correct AC caps present vs connector or onboard IC <!-- CHK-104 -->
- <a id="hs-2"></a>⚠️ `HS-2` Lane swaps, bit swaps: warn to check actual support <!-- CHK-105 -->
- <a id="hs-3"></a>⚠️ `HS-3` Clocked buses where some signals go through dividers or buffers and others don't: delay, setup/hold problems <!-- CHK-43 -->
- <a id="hs-4"></a>⚠️ `HS-4` Differential asymmetry <!-- CHK-190 -->
- <a id="hs-5"></a>⚠️ `HS-5` Series resistors and pull-ups on USB; skip only for known parts that need them, e.g. F103 <!-- CHK-110 -->

### 📡 RF - RF
- <a id="rf-1"></a>⚠️ `RF-1` RF port without tuning network <!-- CHK-72 -->
- <a id="rf-2"></a>⚠️ `RF-2` RF net to connector without PI network <!-- CHK-180 -->
- <a id="rf-3"></a>💡 `RF-3` Suggest DC block caps between RF parts <!-- CHK-80 -->
- <a id="rf-4"></a>💡 `RF-4` Suggest an inline RF test connector <!-- CHK-81 -->

## Connectors

### 🔌 USB - USB and Type-C
- <a id="usb-1"></a>🛑 `USB-1` Type-C without 5k1 <!-- CHK-86 -->
- <a id="usb-2"></a>⚠️ `USB-2` No overvoltage protection on Type-C CC and SBU <!-- CHK-55 -->
- <a id="usb-3"></a>⚠️ `USB-3` Directly exposed Type-C SBU: resistance must be over the "safe state" zSBUTermination > 950k <!-- CHK-13 -->
- <a id="usb-4"></a>⚠️ `USB-4` USB shield connected through L/R/C, or not connected <!-- CHK-38 -->
- <a id="usb-5"></a>⚠️ `USB-5` USB VBUS capacitance as device and host <!-- CHK-46 -->

### 🔗 CONN - Connector protection and pinout
- <a id="conn-1"></a>⚠️ `CONN-1` No protection on lines going to connectors <!-- CHK-58 -->
- <a id="conn-2"></a>💡 `CONN-2` GPIOs going straight to connectors without series protection: 1k on long lines or better <!-- CHK-73 -->
- <a id="conn-3"></a>🛑 `CONN-3` Unipolar protectors on bipolar signals, and bipolar on unipolar ones <!-- CHK-65 -->
- <a id="conn-4"></a>⚠️ `CONN-4` Connectors without ground; with many pins and few grounds, suggest more <!-- CHK-93 -->
- <a id="conn-5"></a>⚠️ `CONN-5` High speed on a connector: a ground for each pair <!-- CHK-94 -->
- <a id="conn-6"></a>💡 `CONN-6` Connectors with spare pins: suggest spreading power over more pins; use pin rating and currents if known <!-- CHK-121 -->
- <a id="conn-7"></a>💡 `CONN-7` Reverse/flip insertion: with many similar signals, suggest a pinout that avoids power shorts <!-- CHK-96 -->
- <a id="conn-8"></a>🛑 `CONN-8` MicroHDMI wired with the full size HDMI pinout <!-- CHK-102 -->

## Analog and ratings

### 📈 ANA - Analog and sensing
- <a id="ana-1"></a>⚠️ `ANA-1` ADC input from a high impedance source without buffer <!-- CHK-114 -->
- <a id="ana-2"></a>⚠️ `ANA-2` Sensitive signals (ADC, high impedance) next to high speed ones on a part (by pin or ball number) <!-- CHK-113 -->
- <a id="ana-3"></a>💡 `ANA-3` Shunt lines to the amplifier without a filter <!-- CHK-98 -->
- <a id="ana-4"></a>💡 `ANA-4` Current sense resistor straight to ground or power: suggest net ties <!-- CHK-128 -->

### 🌡️ RATE - Ratings and derating
- <a id="rate-1"></a>⚠️ `RATE-1` Capacitor voltage rating margins <!-- CHK-29 -->
- <a id="rate-2"></a>⚠️ `RATE-2` Resistor power and voltage ratings <!-- CHK-6 -->
- <a id="rate-3"></a>⚠️ `RATE-3` Shunt power ratings <!-- CHK-7 -->
- <a id="rate-4"></a>⚠️ `RATE-4` MOSFET Vds rating <!-- CHK-217 -->
- <a id="rate-5"></a>⚠️ `RATE-5` MOSFET power dissipation estimate <!-- CHK-8 -->
- <a id="rate-6"></a>⚠️ `RATE-6` LDO current rating, when power draw is known <!-- CHK-16 -->
- <a id="rate-7"></a>⚠️ `RATE-7` LED brightness estimate: warn if too bright or too much current <!-- CHK-185 -->
- <a id="rate-8"></a>⚠️ `RATE-8` Temperature ranges that make no sense, e.g. X7R caps but 85 °C ICs <!-- CHK-47 -->

## Debug and test

### 🐞 DBG - Debug access, indicators, rework options
- <a id="dbg-1"></a>🛑 `DBG-1` SWD/JTAG not connected <!-- CHK-48 -->
- <a id="dbg-2"></a>💡 `DBG-2` No series resistors on an obvious debug header (e.g. SWD) <!-- CHK-84 -->
- <a id="dbg-3"></a>💡 `DBG-3` No LEDs on the board at all: suggest at least one for debug <!-- CHK-85 -->
- <a id="dbg-4"></a>💡 `DBG-4` No power rail LEDs <!-- CHK-56 -->
- <a id="dbg-5"></a>💡 `DBG-5` No error flag LEDs when flags aren't routed to IOs <!-- CHK-134 -->
- <a id="dbg-6"></a>💡 `DBG-6` Power rails not monitored by an ADC <!-- CHK-51 -->
- <a id="dbg-7"></a>💡 `DBG-7` Suggest DNM pull-up and down on EN pins <!-- CHK-17 -->
- <a id="dbg-8"></a>💡 `DBG-8` Suggest DNM pull-ups on open drain pins; test point detector should highlight those nets <!-- CHK-42 -->
- <a id="dbg-9"></a>💡 `DBG-9` Suggest DNM 0R bypasses on power switches <!-- CHK-37 -->
- <a id="dbg-10"></a>💡 `DBG-10` Many things on one power net: suggest splitting with a 0R before each power input <!-- CHK-34 -->
- <a id="dbg-11"></a>💡 `DBG-11` Suggest 0R on USART TX, RX, CTS, RTS and SPI MOSI, MISO, as they're often wrong <!-- CHK-45 -->
- <a id="dbg-12"></a>💡 `DBG-12` Spot potentially useful 0R series resistors (AI?) <!-- CHK-10 -->

### 🧪 TEST - Test points and testability
- <a id="test-1"></a>💡 `TEST-1` Unconnected pins without test points (optional report table) <!-- CHK-11 -->
- <a id="test-2"></a>💡 `TEST-2` Several test points on one net (incl. through 0R), unless top and bottom <!-- CHK-153 -->
- <a id="test-3"></a>💡 `TEST-3` All harness connector pins self-testable <!-- CHK-225 -->

## Documentation and data

### 🧾 BOM - BOM hygiene
- <a id="bom-1"></a>💡 `BOM-1` Resistors with close values, two packages of the same value and other oddities <!-- CHK-23 -->
- <a id="bom-2"></a>💡 `BOM-2` Capacitor voltage, size and dielectric report: find odd ones <!-- CHK-25 -->
- <a id="bom-3"></a>💡 `BOM-3` Big and small connectors, big and small parts, SMT and THT mixed and similar <!-- CHK-141 -->
- <a id="bom-4"></a>💡 `BOM-4` Propose consolidating identical resistors into resistor networks <!-- CHK-234 -->
- <a id="bom-5"></a>🛑 `BOM-5` No TODOs or "calculate later" values left ≈ BOM-6 <!-- CHK-230 -->
- <a id="bom-6"></a>🛑 `BOM-6` "Calculate later" part mounted <!-- CHK-155 -->
- <a id="bom-7"></a>💡 `BOM-7` Depopulate spare and debug parts for release <!-- CHK-224 -->
- <a id="bom-8"></a>🛑 `BOM-8` LED swapped (polarity) <!-- CHK-200 -->
- <a id="bom-9"></a>🛑 `BOM-9` Part without a value <!-- new 5 Oct 2026, found in code -->
- <a id="bom-10"></a>⚠️ `BOM-10` Value that doesn't parse or isn't a standard series value <!-- new 5 Oct 2026, found in code -->

### 🏷️ NAME - Naming and conventions
- <a id="name-1"></a>💡 `NAME-1` Inconsistent naming of inverted signals: suggest one notation <!-- CHK-63 -->
- <a id="name-2"></a>💡 `NAME-2` Inconsistent naming of differential pairs: suggest one notation <!-- CHK-64 -->
- <a id="name-3"></a>💡 `NAME-3` Power net names without voltage when it's clearly fixed (DC-DC or LDO output, USB 2, Type-C with 5k1) <!-- CHK-77 -->
- <a id="name-4"></a>💡 `NAME-4` UART: suggest `X_TX_Y_RX` style names <!-- CHK-165 -->
- <a id="name-5"></a>💡 `NAME-5` Gaps in designator series on rev A or standalone ≈ NAME-6 <!-- CHK-49 -->
- <a id="name-6"></a>💡 `NAME-6` Suggest per page designators; highlight missing ones <!-- CHK-54 -->
- <a id="name-7"></a>💡 `NAME-7` No nets found in strict mode: suggest `+xVy` power names and non-strict mode <!-- CHK-204 -->

### 📚 LIB - Library and symbols
- <a id="lib-1"></a>⚠️ `LIB-1` Check a symbol against others in the database: misnamed pins, wrong types, missing pins <!-- CHK-78 -->
- <a id="lib-2"></a>⚠️ `LIB-2` Global pinout database for common connectors, check against it <!-- CHK-116 -->
- <a id="lib-3"></a>⚠️ `LIB-3` MOSFET symbol pins not named G/D/S (other checks need them) <!-- new 5 Oct 2026, found in code -->

## Layout

### 📐 LAY - Layout and placement
- <a id="lay-1"></a>🛑 `LAY-1` Schematic and layout match <!-- CHK-229 -->
- <a id="lay-2"></a>⚠️ `LAY-2` Parts in component areas, test points in designated areas, nothing in forbidden areas <!-- CHK-154 -->
- <a id="lay-3"></a>⚠️ `LAY-3` Auto courtyard check and fix <!-- CHK-219 -->

    Check that cortyard is equidistant from 3D boundary.

    Add a proper one if requested.

- <a id="lay-4"></a>💡 `LAY-4` Check stackup <!-- CHK-232 -->

    SGGS for 4L recommend

- <a id="lay-5"></a>⚠️ `LAY-5` Stitching vias <!-- CHK-233 -->

    6 vias in each wavelength of highest frequency signal to not excite the cavity

    [https://youtu.be/kdCJxdR7L_I?si=rVgj4WJCnR0p1eez](https://youtu.be/kdCJxdR7L_I?si=rVgj4WJCnR0p1eez)

