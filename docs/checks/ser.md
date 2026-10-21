# SER checks, explained

Detail page for the `SER` category (UART, SPI, CAN, RS-485, LVDS and whole-bus checks) of the
[check catalogue](../checks.md#-ser---uart-spi-can-rs-485-lvds-and-whole-bus-checks). Each section explains one
check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it, the
usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **SLLA272** · TI, *The RS-485 Design Guide*, rev. D (May 2021). Termination, failsafe biasing, unit loads,
  grounding. [ti.com/lit/pdf/slla272](https://www.ti.com/lit/pdf/slla272)
- **SLLA070** · TI, *RS-422 and RS-485 Standards Overview and System Configurations*, rev. D (May 2010).
  [ti.com/lit/pdf/slla070](https://www.ti.com/lit/pdf/slla070)
- **SLYT324** · TI, *RS-485: Passive failsafe for an idle bus*, Analog Applications Journal 1Q 2009.
  [ti.com/lit/pdf/slyt324](https://www.ti.com/lit/pdf/slyt324)
- **SLOA101** · TI, *Introduction to the Controller Area Network (CAN)*, rev. B (May 2016).
  [ti.com/lit/pdf/sloa101](https://www.ti.com/lit/pdf/sloa101)
- **SLLA337** · TI, *Overview of 3.3V Controller Area Network (CAN) Transceivers*, rev. A (May 2025). Split
  termination and ground shift. [ti.com/lit/pdf/slla337](https://www.ti.com/lit/pdf/slla337)
- **SLLA014** · TI, *Low-Voltage Differential Signaling (LVDS) Design Notes*, rev. A (2000).
  [ti.com/lit/pdf/slla014](https://www.ti.com/lit/pdf/slla014)
- **TCAN1042H** · TI, *TCAN1042 Fault Protected CAN Transceiver with CAN FD*, SLLSES7D (October 2021). Used as the
  typical CAN FD transceiver. [ti.com/lit/ds/symlink/tcan1042h.pdf](https://www.ti.com/lit/ds/symlink/tcan1042h.pdf)
- **SN65HVD230** · TI, *SN65HVD23x 3.3-V CAN Bus Transceivers*, SLOS346O (April 2018). A classic 1 Mbit/s
  transceiver with an RS mode pin. [ti.com/lit/ds/symlink/sn65hvd230.pdf](https://www.ti.com/lit/ds/symlink/sn65hvd230.pdf)
- **ADM3050E** · Analog Devices, *5.7kV rms, Signal Isolated, Basic CAN FD Transceiver*, data sheet rev. D (fetched
  21 Oct 2026 through `tpm fetch`, analog.com refuses plain HTTP clients). The isolated transceiver on B125 and
  B135. [analog.com/…/adm3050e.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/adm3050e.pdf)
- **THVD1450** · TI, *THVD14xx 3.3-V to 5-V RS-485 Transceivers*, SLLSEY3E (May 2019). A modern RS-485
  transceiver with built-in failsafe. [ti.com/lit/ds/symlink/thvd1450.pdf](https://www.ti.com/lit/ds/symlink/thvd1450.pdf)
- **SN75176B** · TI, *SNx5176B Differential Bus Transceivers*, SLLS101I (August 2025). The classic RS-485
  transceiver without failsafe. [ti.com/lit/ds/symlink/sn75176b.pdf](https://www.ti.com/lit/ds/symlink/sn75176b.pdf)
- **MAX3232** · TI, *MAX3232 3-V to 5.5-V Multichannel RS-232 Line Driver and Receiver*, SLLS410O (June 2021).
  [ti.com/lit/ds/symlink/max3232.pdf](https://www.ti.com/lit/ds/symlink/max3232.pdf)
- **W25Q128JV** · Winbond, *W25Q128JV 3V 128M-bit serial flash memory with dual/quad SPI*, revision M (December
  2024). [winbond.com/resource-files/W25Q128JV RevM 12242024 Plus.pdf](https://www.winbond.com/resource-files/W25Q128JV%20RevM%2012242024%20Plus.pdf)
- **DS18B20** · Maxim Integrated (Analog Devices), *DS18B20 Programmable Resolution 1-Wire Digital Thermometer*,
  19-7487 rev. 6 (July 2019; fetched through `tpm fetch`).
  [analog.com/…/DS18B20.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/DS18B20.pdf)
- **CP2102N** · Silicon Labs, *CP2102N Data Sheet*, rev. 1.5 (fetched through `tpm fetch`).
  [silabs.com/…/cp2102n-datasheet.pdf](https://www.silabs.com/documents/public/data-sheets/cp2102n-datasheet.pdf)
- **AN2606** · ST, *Introduction to system memory boot mode on STM32 MCUs*, rev. 70 (February 2026; fetched through
  `tpm fetch`). Which pins the ROM bootloader drives.
  [st.com/…/an2606….pdf](https://www.st.com/resource/en/application_note/an2606-stm32-microcontroller-system-memory-boot-mode-stmicroelectronics.pdf)
- **esptool boot mode** · Espressif, *Boot Mode Selection* (esptool docs, ESP32), "Automatic Bootloader".
  [docs.espressif.com/…/boot-mode-selection.html](https://docs.espressif.com/projects/esptool/en/latest/esp32/advanced-topics/boot-mode-selection.html)
- **ELM MMC/SDC** · ChaN, *How to Use MMC/SDC*, "Consideration on Multi-slave Configuration".
  [elm-chan.org/docs/mmc/mmc_e.html](http://elm-chan.org/docs/mmc/mmc_e.html)
- **PJRC SPI** · PJRC, *Better SPI Bus Design in 3 Steps*.
  [pjrc.com/better-spi-bus-design-in-3-steps](https://www.pjrc.com/better-spi-bus-design-in-3-steps/)
- **ISO 11898-2:2016** · *Road vehicles — Controller area network (CAN) — Part 2: High-speed medium access unit*.
  **TIA-485-A** · *Electrical Characteristics of Generators and Receivers for Use in Balanced Digital Multipoint
  Systems*. **TIA-232-F** · *Interface Between Data Terminal Equipment and Data Circuit-Terminating Equipment
  Employing Serial Binary Data Interchange*. Cited by title.
- **Wikipedia** · readable overviews: [UART](https://en.wikipedia.org/wiki/Universal_asynchronous_receiver-transmitter),
  [Serial Peripheral Interface](https://en.wikipedia.org/wiki/Serial_Peripheral_Interface),
  [CAN bus](https://en.wikipedia.org/wiki/CAN_bus), [CAN FD](https://en.wikipedia.org/wiki/CAN_FD),
  [RS-485](https://en.wikipedia.org/wiki/RS-485), [RS-232](https://en.wikipedia.org/wiki/RS-232),
  [1-Wire](https://en.wikipedia.org/wiki/1-Wire),
  [LVDS](https://en.wikipedia.org/wiki/Low-voltage_differential_signaling).
- **HSDD** · Johnson & Graham, *High-Speed Digital Design: A Handbook of Black Magic* (Prentice Hall, 1993),
  chapter 6 (terminations).
- **AoE3** · Horowitz & Hill, *The Art of Electronics*, 3rd ed. (Cambridge, 2015), chapter 14 (serial buses and
  data links).

## `SER-1` USART TX connected to TX <a id="ser-1"></a>

🛑 error · needs: UART pin roles on both parts (TX, RX) · [catalogue entry](../checks.md#ser-1)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** A UART link where the TX pin of one part is wired to the TX pin of the other (and RX to RX),
instead of TX to RX.

**Why it matters.** UART is point to point and each side names its pins from its own view: TX is an output, RX an
input. TX to TX puts two push-pull outputs against each other (both idle high, so it often survives, but every
start bit is a short) and leaves both receivers without a driver. The link is simply dead, and the only fix on a
built board is two cut traces and two wires.

**Example.** An ESP32-C3 module next to a GPS receiver (u-blox MAX-M10S): the schematic nets are named
`GPS_TX`/`GPS_RX` from the MCU's view on one sheet and from the GPS's view on the other, and the two `GPS_TX`
labels join U1 TXD0 to the GPS TXD. On B153A the SBU lines carry UART in either orientation, so the MCU sheet lists
both pairings (SBU1 = USART3_TX, or USART1_RX through a net tie): exactly the kind of table where one wrong row
crosses nothing.

**How to check.** Find UART pins by their role (pin name or alternate function, `NET-7`, `NET-12`); a net with two
UART outputs, or two UART inputs and no output, is the finding. A USB-UART bridge, a transceiver (RS-232, RS-485)
or a connector counts as the other side; a connector needs its documented direction (`CFG-2`).

**Fix.** Cross the lines (TX to RX). Name nets after the driver (`MCU_TX` → `GPS_RXD`), or by direction
(`MCU_TO_GPS`); where pins can be swapped in silicon (STM32 `USART_CR2.SWAP`) note it on the sheet.

**References.** Wikipedia, UART "Transmitting and receiving serial data"; AoE3 chapter 14.

## `SER-2` SPI CS line grounded <a id="ser-2"></a>

🛑 error · needs: SPI pin roles, ground nets · [catalogue entry](../checks.md#ser-2)

**Impl.** mid · depends on NET-7

**What it catches.** An SPI slave whose chip select is tied to ground (or to a fixed low) because it is the only
device on the bus.

**Why it matters.** Many SPI slaves use the CS edge, not just its level: the falling edge resets the command state
machine and the rising edge commits a write or starts a conversion. Serial flash ignores everything until it sees
CS go high to low after power-up; ADCs start or end conversions on CS; a shift-register-style DAC latches on the
rising edge. With CS grounded one bit slip (a glitch on SCK at power-up) misaligns every following command and
nothing resynchronises it.

**Example.** A W25Q128JV boot flash with `/CS` strapped to GND to "save a pin": the datasheet asks for a high-to-low
transition of `/CS` after power-up before any instruction is accepted, and for `/CS` to track VCC during power-up
and power-down. The part never answers the first Read JEDEC ID.

**How to check.** A pin with an SPI chip-select role (`NET-7`: names like `CS`, `~CS`, `SS`, `NSS`, `CSB`) on a net
that is a ground or has only a resistor to ground.

**Fix.** Drive CS from a GPIO (or the peripheral's NSS), with a pull-up (`SER-19`). If a slave really works with CS
tied low, its datasheet says so and the check can be waived for that part.

**References.** W25Q128JV §4.1 (Chip Select), §6.5 (Write Protection); Wikipedia, Serial Peripheral Interface
"No slave select".

## `SER-3` CAN terminator based on MOSFETs or SSRs <a id="ser-3"></a>

⚠️ warning · needs: CAN bus lines, termination path through switches · [catalogue entry](../checks.md#ser-3)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A switchable CAN termination whose switch is a MOSFET pair, an analog switch or a solid-state
relay (photo-MOS) in series with the 120 Ω.

**Why it matters.** The termination sits on the bus lines, which in a fault see the supply wires of the harness (12 V or
24 V; transceivers such as the TCAN1042 are built to survive ±58 V to ±70 V on the bus). A semiconductor switch has to
block those in both polarities, its capacitance and leakage load the bus when "off", its on-resistance adds to the 120 Ω
and varies with temperature, and its gate drive must reference a node that moves with the CAN common mode. An SSR also
has an output capacitance of tens of pF that unbalances the pair. The bus "works on the bench" and fails at the first
ground shift or miswire.

**Example.** B125 and B135 switch their split termination (2 × 60 Ω and a 4.7 nF centre capacitor) with a double-coil
latching DPDT signal relay instead (K501, Kemet EE2-5TNUH-L, set and reset coils driven by two FETs): no leakage, no
gate drive on the isolated side, and the setting survives power-off. The usual crutch is a pair of BSS138 back-to-back
driven from the MCU: the CANL side swings below the FET's source and the body diodes conduct.

**How to check.** Find a resistor of 100-130 Ω (or a 60 Ω pair) between the CAN lines (pins `CANH`/`CANL` of a
transceiver, `NET-7`) whose path passes through a MOSFET, analog switch or SSR (`NET-10`).

**Fix.** A signal relay, a solder jumper or a DIP switch; or a CAN transceiver with switchable termination built
in. If a semiconductor switch is a must, check its blocking voltage against the transceiver's bus fault rating and
its off capacitance against the bit rate.

**References.** TCAN1042H §10.2.2.1 (CAN Termination); SLOA101 §5 (the CAN bus); ADM3050E features (±40 V bus
fault protection); ISO 11898-2:2016.

## `SER-4` LVDS without terminators <a id="ser-4"></a>

🛑 error · needs: LVDS pairs, receiver pins and their on-die termination · [catalogue entry](../checks.md#ser-4)

**Impl.** mid · depends on NET-7, IMP-10

**What it catches.** An LVDS pair (or sub-LVDS, M-LVDS, LVDS clock) with no 100 Ω differential termination at the
receiver end, neither a resistor nor an enabled on-die terminator.

**Why it matters.** An LVDS driver is a current source of about 3.5 mA; the termination resistor turns it into the
±350 mV the receiver sees. Without it the current has no defined return, the line voltage floats up to the driver's
compliance limit and the far end reflects every edge. Some designs "work" at low rates from the receiver's input
capacitance, then fail with temperature or a longer cable.

**Example.** An FPGA (Artix-7, HR bank) receiving an LVDS camera link: the designer assumed `DIFF_TERM` would be
set in the constraints, the firmware team did not set it, and the board has no 100 Ω footprint to fall back on.
TI's design note states it plainly: termination at the far end of the interconnect is mandatory.

**How to check.** Find differential pairs driven by LVDS outputs or reaching LVDS inputs (pin data, `NET-7`;
FPGA bank and pin standards, `IMP-10`); look for a ~100 Ω resistor across the pair at the receiver. An FPGA input
with optional internal termination needs a project setting saying it is enabled (`CFG-2`).

**Fix.** A 100 Ω resistor across the pair, as close to the receiver pins as possible (fly-by when the receiver is
more than ~2 cm away), or the receiver's internal termination, documented on the sheet. Never at the driver.

**References.** SLLA014 "Termination" (mandatory, 100 Ω, Figure 4) and "Stubs" (fly-by, Figure 5); HSDD
chapter 6; Wikipedia, LVDS.

## `SER-5` Not the whole bus is present <a id="ser-5"></a>

🛑 error · needs: bus definitions per interface, pin roles · [catalogue entry](../checks.md#ser-5)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** An interface with one of its lines missing across the board, or two of its lines merged into
one net: an SPI bus with SCK and MOSI but no MISO path, a UART with only TX, CANH and CANL on the same net, QSPI
with IO2 and IO3 missing.

**Why it matters.** A missing line is often a deleted wire or a mismatched label (`SPI_MISO` vs `SPI1_MISO`) that
leaves both halves as one-pin nets; a merged pair is a duplicated label. Either way the interface cannot work, and
the netlist looks tidy because every pin is still connected to something.

**Example.** A flash on QSPI where the IO2/IO3 labels were renamed `WP`/`HOLD` on the memory sheet: the
controller's `QSPI_IO2`/`QSPI_IO3` end on one-pin nets and quad reads return garbage; single-SPI mode works, so
the mistake survives the first bring-up.

**How to check.** For each interface found from pin roles (`NET-7`, `NET-12`), list the lines it needs (SPI: SCK,
MOSI, MISO, CS; QSPI: plus IO2, IO3; UART: TX, RX; CAN: TX, RX or H, L) and check each exists as a separate net
reaching the other side. Two roles of one bus on a single net is a short.

**Fix.** Rename the label, add the missing wire; if a line is really unused (write-only SPI display), mark the pin
no-connect so the intent is visible.

**References.** Wikipedia, Serial Peripheral Interface "Operation".

## `SER-6` Not the whole bus is connected to a device <a id="ser-6"></a>

🛑 error · needs: pin roles per device, bus definitions · [catalogue entry](../checks.md#ser-6)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** A device on a bus with one of its bus pins unconnected or on a different net than the rest
of the bus, while the bus itself is complete elsewhere.

**Why it matters.** It is the per-device side of `SER-5`: the MCU has MISO, three slaves share it, the fourth
slave's SDO pin is left open. The bus works for three devices and the fourth reads 0xFF; nobody suspects the
schematic because the bus "works".

**Example.** An IMU (ICM-42688-P) and a barometer (BMP390) on one SPI bus: the barometer's `SDO` pin is left open
because on the breakout board it was the I2C address strap, so in SPI mode its data never reaches MISO.

**How to check.** For each device with bus pins (`NET-7`), its role pins must reach the bus nets (directly or
through series parts, `NET-10`); a role pin that is unconnected or on its own net is the finding.

**Fix.** Wire the missing pin; for dual-mode parts (I2C/SPI) check the pin names for the mode used.

**References.** Wikipedia, Serial Peripheral Interface "Multidrop configuration".

## `SER-7` CAN TX and RX crossed between MCU and transceiver <a id="ser-7"></a>

🛑 error · needs: CAN controller and transceiver pin roles · [catalogue entry](../checks.md#ser-7)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** An MCU's `CAN_TX` wired to the transceiver's `RXD` (and `CAN_RX` to `TXD`), crossed as if it
were a UART.

**Why it matters.** Unlike UART, the controller and the transceiver are not peers: the transceiver's `TXD` is an
input that takes the controller's TX, its `RXD` an output to the controller's RX. Crossed, the MCU drives the
transceiver's output and listens to nothing; with `TXD` floating the transceiver sits recessive (its internal
pull-up) and the node is silent. Designers used to UART cross them out of habit.

**Example.** An STM32G0B1 `FDCAN1_TX` (PB9) to a TCAN1042 `RXD` pin: two outputs fight on one net, the controller
never sees its own frames back and goes bus-off. B153A's SBU modes show how easy this is to get wrong on paper:
the mode table on the MCU sheet (SBU1 = FDCAN1_RX, SBU2 = FDCAN1_TX) had to be corrected at the 5 Oct 2026 review.

**How to check.** Controller pins with CAN TX/RX roles (`NET-12`) and transceiver pins `TXD`/`RXD` (`NET-7`): TX
must meet TXD, RX meet RXD.

**Fix.** Uncross them. Label the transceiver side `CAN_TXD`/`CAN_RXD` with the same names as the controller.

**References.** TCAN1042H §9.3.5 (floating terminals, TXD pull-up), §9.3.7 (digital inputs and outputs); SLOA101
§5 (Figure 6, node structure).

## `SER-8` UART flow control mis-paired <a id="ser-8"></a>

🛑 error · needs: UART RTS/CTS pin roles on both sides · [catalogue entry](../checks.md#ser-8)

**Impl.** mid · depends on NET-7, NET-12

**What it catches.** RTS of one side not going to CTS of the other: CTS to CTS, RTS to RTS, or RTS to the other
side's RTS through a level shifter.

**Why it matters.** Hardware flow control is crossed like TX/RX: RTS is an output saying "I can receive", CTS an
input that gates the transmitter. CTS-to-CTS leaves both inputs undriven; depending on the pull, a side never
transmits (CTS seen high, "not clear") or transmits into a full FIFO and drops bytes, which looks like a firmware
bug at high baud rates only.

**Example.** An nRF52840 talking HCI to a CP2102N with hardware flow control: the CP2102N's `RTS` is an active-low
output that rises when its RX FIFO reaches 448 bytes. Wired to the nRF's `RTS` instead of `CTS`, the nRF ignores
it, overruns the bridge at 1 Mbaud and loses bytes during long transfers only.

**How to check.** Same as `SER-1` for the RTS/CTS roles (`NET-7`, `NET-12`): each net must have one RTS (output)
and one CTS (input). A CTS left floating is also worth a finding: give it a pull to "clear" or to "hold", say which.

**Fix.** Cross RTS and CTS; pull unused CTS inputs to their active level if flow control is off.

**References.** CP2102N §4.3.7 (Hardware Handshaking); Wikipedia, RS-232 "RTS, CTS, and RTR"; Wikipedia, UART.

## `SER-9` RS-485 bus without termination at both ends <a id="ser-9"></a>

🛑 error · needs: RS-485 A/B nets, termination resistor, the node's place on the bus · [catalogue entry](../checks.md#ser-9)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** An RS-485 node that the project marks as a bus end (or the only node, as on a point-to-point
link) with no 120 Ω (or 100-130 Ω) resistor between A and B, fitted or selectable.

**Why it matters.** RS-485 runs on twisted pair of about 120 Ω; an unterminated end reflects each edge back. At
9600 baud over a few metres it hardly matters, but at hundreds of kbit/s or over 100 m the reflections cross the
±200 mV threshold and corrupt bits. Termination also sets the idle differential voltage together with failsafe
biasing (`SER-10`).

**Example.** A Modbus RTU sensor board with a THVD1450 at 115 200 baud on 300 m of cable, the last device on the
line, with no termination footprint: the installer adds a resistor in the terminal block, or the link runs with
errors on hot days.

**How to check.** Find RS-485 transceivers (pins `A`/`B` or `Y`/`Z`, `NET-7`); look for a 100-130 Ω resistor
across A-B (directly, through a jumper or a switch). Whether the node is a bus end is a project setting
(`CFG-2`); without it, report "no termination option" as a suggestion.

**Fix.** A 120 Ω across A-B at the node, switchable (jumper, DIP switch) if the board can sit anywhere on the bus.
Size the resistor for a driver short (RS-485 drivers deliver up to ~250 mA into a short).

**References.** SLLA272 §6 (Bus Termination and Stub Length, Figure 6-1); SLLA070 §4 (termination techniques),
§2.2.5 (250 mA short-circuit limit);
TIA-485-A; Wikipedia, RS-485.

## `SER-10` RS-485 without fail-safe biasing <a id="ser-10"></a>

⚠️ warning · needs: transceiver failsafe capability (part data), bias resistors · [catalogue entry](../checks.md#ser-10)

**Impl.** mid · depends on NET-7

**What it catches.** An RS-485 bus with a transceiver that has no built-in idle-bus failsafe and no external bias
network (A pulled up, B pulled down through resistors of a few hundred ohms).

**Why it matters.** When no driver is active, the terminated bus sits at 0 V differential: inside the ±200 mV
undefined window of a classic receiver. The receiver output then follows noise, and the UART sees start bits and
framing errors between every message. Modern transceivers shift their threshold to fail safe (output high on
an idle, open or shorted bus), but their noise margin is small, and older parts (SN75176B, MAX485 class) have
none.

**Example.** An SN75176B on a 5 V bus with 120 Ω at both ends: its datasheet's own design procedure adds external
failsafe biasing for +200 mV. SLLA272 works the numbers: with 4.75 V minimum supply, 250 mV target and 120 Ω line,
two 523 Ω bias resistors at one end.

**How to check.** For each RS-485 transceiver, read from part data whether it has idle-bus failsafe (`NET-7`); if
not, look for a resistor from A to a rail and from B to ground on the bus.

**Fix.** Use a transceiver with built-in failsafe (THVD1450, SN65HVD72 class), or add the bias pair at one node
only; its value per SLLA272 equation 2 or SLYT324.

**References.** SLLA272 §7 (Failsafe, equation 2, Figure 7-1); SLLA070 §3 (Failsafe Operation); SLYT324;
SN75176B §7.2.2; THVD1450 §10.2.1.4 (Receiver Failsafe).

## `SER-11` RS-485 DE / ~RE without a pull to the receive state <a id="ser-11"></a>

🛑 error · needs: transceiver DE/~RE pins, pulls, the MCU pin's reset state · [catalogue entry](../checks.md#ser-11)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** The driver-enable (DE) and receiver-enable (~RE) pins of an RS-485 transceiver driven by an MCU
pin with no external pull-down (DE) or pull to the receive level.

**Why it matters.** During reset, boot and firmware updates the MCU pin is an input; a floating DE can turn the
driver on and a half-duplex node then holds the whole bus, so every other node on the line stops talking while one
board reboots. Internal pulls are too weak to rely on (THVD1450: 2 MΩ). The pin may also be driven by something
other than the application: a ROM bootloader sets up its own UART pins while scanning for a host.

**Example.** On B153A the ST ROM bootloader of the STM32G0B1 drives PA9 (its USART1_TX) while it runs, which cut
the DUT's USB data through `DATA_CUT` during every ROM update. A DE on such a pin would key the RS-485 driver for
the whole update. A THVD1450 with DE on a plain GPIO and no resistor shows the simple case: its 2 MΩ internal
pull-down loses against a few pF of coupling from the adjacent TX trace.

**How to check.** DE/~RE pins (`NET-7`) whose net reaches an MCU pin with no resistor to the receive level;
whether the MCU pin floats in reset comes from `NET-9`, ROM bootloader pins from part data.

**Fix.** A 1-10 kΩ pull-down on DE (and on ~RE, which tied with DE gives receive at low); keep DE off pins the ROM
bootloader uses (AN2606 lists them per part).

**References.** THVD1450 pin functions (DE 2-MΩ internal pull-down), §12.1 item 6 (1-10 kΩ pulls on enable lines);
AN2606 §50.1 Table 111 (STM32G0B1: PA9 USART1_TX); B153 rev B change list (DATA_CUT on PA9).

## `SER-12` RS-485 A/B or CAN H/L swapped against the connector's pinout <a id="ser-12"></a>

🛑 error · needs: connector pinout (documented or standard), bus pin roles · [catalogue entry](../checks.md#ser-12)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** CANH on the connector pin documented for CANL (DE-9 pin 7 vs pin 2), or RS-485 A on the
terminal marked B.

**Why it matters.** A swapped differential pair inverts every bit. CAN with H/L swapped holds the bus dominant
from the first frame and every node goes error-passive; RS-485 swapped reads inverted data, which a UART turns
into framing errors. RS-485 adds a naming trap: the standard's A is the inverting line, most transceiver
datasheets name the non-inverting pin A, and Profibus calls the green wire A. Two products wired "A to A" can be
inverted against each other.

**Example.** B125 and B135 bring CAN out on a DE-9 plug: the de facto pinout (CiA 303-1) puts CAN_L on pin 2,
ground on pin 3, CAN_H on pin 7. A footprint whose pin numbering runs mirrored (plug vs receptacle) swaps 2 and 7
without a single wrong label on the schematic.

**How to check.** For a connector with a known pinout (DE-9 CAN, RJ45 for RS-485 on a given standard, a project
setting `CFG-2`), compare the pin each bus line reaches with the documented one. Without data: report the pairing
for review.

**Fix.** Swap in the schematic, or in the footprint if the pin numbers are mirrored; put the pinout and the A/B
convention on the silkscreen.

**References.** Wikipedia, CAN bus "Layers" (DE-9 pin-out); Wikipedia, RS-485 "Signals" (A/B naming);
SLOA101 §5.

## `SER-13` CAN termination wrong for the node's place <a id="ser-13"></a>

⚠️ warning · needs: CAN lines, termination, the node's place on the bus · [catalogue entry](../checks.md#ser-13)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** A CAN node with no termination although it sits at a bus end, or with a fixed 120 Ω although
it sits in the middle of the bus.

**Why it matters.** ISO 11898-2 wants exactly two 120 Ω terminations, one at each end of the line: about 60 Ω seen
by every driver. A missing one leaves reflections; an extra one in the middle drops the load to 40 Ω, the
differential dominant voltage drops and margins shrink, and with three or four "helpfully terminated" nodes the
bus stops working. TI's TCAN1042 note: place terminations so two always exist, even when nodes are removed.

**Example.** B125 needs a bigger termination resistor in rev B: the split pair is 2 × 60.4 Ω in 0603. A dominant
bit puts only ~1 V on each half, but a wiring fault (CAN_H on a 24 V supply wire) drives the transceiver's short
circuit current through them, and TI asks to size the termination's power rating for exactly that case.

**How to check.** Count the termination resistors on the node's CAN lines (fixed, or switchable through a relay or
jumper, `NET-10`) and compare with the project's statement of where the node sits (`CFG-2`). Without it: suggest
a switchable termination.

**Fix.** Switchable termination (relay, jumper, DIP switch) on nodes that can be anywhere; fixed termination only
on nodes that are always at an end. Rate the resistors for the bus fault current.

**References.** TCAN1042H §10.2.2.1 (CAN Termination), §9.3.6 (short-circuit current and termination power
rating); SLOA101 §5; B125 rev B change list; ISO 11898-2:2016.

## `SER-14` CAN split termination without the centre capacitor <a id="ser-14"></a>

⚠️ warning · needs: resistor pair between the CAN lines, capacitor to ground · [catalogue entry](../checks.md#ser-14)

**Impl.** easy · depends on netlist

**What it catches.** Two resistors of ~60 Ω in series between CANH and CANL whose middle node has no capacitor to
ground (or goes to a transceiver's SPLIT pin without one).

**Why it matters.** Split termination exists to give the common-mode noise a low-impedance path to ground at high
frequency through the centre capacitor (typically 4.7 nF). Without the capacitor the pair is just a 120 Ω
termination with an extra part, and the designer believes the EMC problem was handled. The two halves also need
matched tolerance (1 %) or common-mode noise is converted into differential noise.

**Example.** B125 (2 × 60.4 Ω) and B135 (R506/R507, 2 × 60 Ω) put a 4n7 (C515 on B135) from the centre to the
isolated ground, the textbook form.
The usual mistake is a DNP centre capacitor kept from an EMC experiment, or a value of 100 nF that loads the
common-mode at the bit rate.

**How to check.** A series pair of resistors, each 50-70 Ω, between two nets that reach one transceiver's `CANH`
and `CANL`; the centre net must have a capacitor (fitted in this variant) to a ground.

**Fix.** Add 4.7 nF (1-10 nF, rated for the bus fault voltage) from the centre to ground, or to the transceiver's
SPLIT pin; 1 % resistors.

**References.** SLOA101 §5 (Figure 12, CL 4.7 nF, ±1 % resistors); SLLA337 §2 (Figure 2-5, split termination
under ground shift); TCAN1042H §10.2.2.1 (Figure 10-2).

## `SER-15` CAN transceiver STB, S or EN pin floating <a id="ser-15"></a>

🛑 error · needs: transceiver mode pins, pulls, MCU pin reset state · [catalogue entry](../checks.md#ser-15)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** A CAN transceiver's mode pin (STB, S, EN, RS, SILENT) left unconnected, or driven by an MCU
pin with no pull, so its mode at power-up and during reset is set by an internal bias or by nothing.

**Why it matters.** Internal pulls choose the safe mode, which is usually not the working one: the TCAN1042 pulls
STB up into standby, where the driver is off and only the wake-up receiver runs. A floating mode pin with no
internal pull (the SN65HVD230's RS pin selects high speed, slope control or listen-only by its level) sits at
whatever leakage decides. The node receives nothing or transmits nothing, and the symptom looks like wrong bit
timing.

**Example.** A TCAN1042 with STB left open on the schematic because "the reference design has it grounded": the
transceiver stays in standby, RXD shows only the wake pattern filter, the MCU never sees a frame.

**How to check.** Mode pins of CAN transceivers (`NET-7`) that are unconnected, or on a net with only an MCU pin
that floats in reset (`NET-9`) and no resistor.

**Fix.** Tie the mode pin to the working level (STB to GND, RS to GND for high speed) or drive it from a GPIO with
a pull to the level wanted during reset.

**References.** TCAN1042H §9.3.5 (Floating Terminals), §9.4.3 (Standby Mode); SN65HVD230 §10.4 (Device
Functional Modes).

## `SER-16` CAN FD data rate above the transceiver's class <a id="ser-16"></a>

⚠️ warning · needs: transceiver MPN decoding, the bus data rate (project) · [catalogue entry](../checks.md#ser-16)

**Impl.** mid · depends on CFG-2

**What it catches.** A transceiver whose rated data rate is below the CAN FD data phase rate of the project: a
classic 1 Mbit/s part on an FD bus, or a 2 Mbit/s FD part on a 5 Mbit/s bus.

**Why it matters.** The data phase of CAN FD needs short, symmetrical loop delays and tight bit symmetry; a
transceiver specified only for 1 Mbit/s has neither. Frames with fast data phases fail CRC intermittently,
depending on cable length and temperature, and the arbitration phase (still at 500 kbit/s or 1 Mbit/s) keeps
working, so the bus looks healthy.

**Example.** The TCAN1042 family: all variants are specified for 2 Mbit/s CAN FD, only the "G" variants
(TCAN1042HG, TCAN1042HGV) for 5 Mbit/s; an SN65HVD230 is designed for up to 1 Mbit/s. The MPN suffix decides,
so a BOM substitution can silently downgrade the bus. The ADM3050E on B125/B135 is rated to 12 Mbit/s.

**How to check.** Decode the transceiver's rate class from its MPN (part data), compare with the project's CAN FD
data rate (`CFG-2`).

**Fix.** A transceiver rated for the data phase rate (the "G" variant, or a part specified at 5 or 8 Mbit/s), and
lock the suffix in the BOM.

**References.** TCAN1042H features and description (2 Mbit/s, "G" 5 Mbit/s); SN65HVD230 features (up to 1 Mbit/s);
ADM3050E features (12 Mbit/s); Wikipedia, CAN FD "CAN Transceiver".

## `SER-17` Transceiver logic side on a different rail than the MCU pins <a id="ser-17"></a>

🛑 error · needs: transceiver VIO pin, MCU IO bank supplies, power structure · [catalogue entry](../checks.md#ser-17)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A transceiver (CAN, RS-485, RS-232, Ethernet PHY strap side) whose logic supply (VIO, VL,
VCC1) is on a different rail than the MCU bank its TXD/RXD pins talk to.

**Why it matters.** The logic supply sets the RXD output high and the TXD input thresholds. VIO at 5 V drives 5 V
into a 3.3 V pin that may not be 5 V tolerant, and backfeeds the MCU rail through its clamp diodes; VIO at 1.8 V
next to a 3.3 V MCU gives RXD highs below the MCU's VIH. A 5 V-only transceiver without VIO has TTL inputs (3.3 V
in is fine) but drives RXD to 5 V.

**Example.** A TCAN1042HV with VIO tied to the 5 V CAN supply because "the V means 5 V", talking to an STM32H7
on a pin that is not 5 V tolerant: RXD drives 5 V into the MCU. TI notes that
the non-V variants drive RXD to VCC, so they need a 5 V tolerant input.

**How to check.** For each transceiver logic pin (`NET-7`), take the rail of its VIO pin (or its VCC if it has
none) and the rail of the bank of the MCU pin on the same net (`NET-5`); differing rails are the finding, unless a
translator sits between them (`NET-10`).

**Fix.** VIO from the MCU's IO rail; a transceiver with a VIO pin when the bus supply differs from the logic.

**References.** TCAN1042H §9.3.7 (Digital Inputs and Outputs, VCC-only and VIO variants); SLLA337 §1.

## `SER-18` SPI chip select shared, or a MISO that does not tri-state <a id="ser-18"></a>

🛑 error · needs: SPI pin roles, slave MISO behaviour (part data) · [catalogue entry](../checks.md#ser-18)

**Impl.** mid · depends on NET-7

**What it catches.** Two SPI slaves on the same CS net, or a slave on a shared MISO that does not release MISO
(high impedance) when deselected, or only after extra clocks.

**Why it matters.** With a shared CS both slaves answer at once: MISO is a fight between two push-pull outputs and
writes reach both. A slave that keeps driving MISO while deselected corrupts every read from the other slaves.
SD cards in SPI mode are the classic case: they release MISO only after one more byte of clocks with CS high, and
before they are put into SPI mode they may answer traffic meant for other devices.

**Example.** An SD card socket and a W25Q flash on one SPI bus of an RP2040: the flash reads fail right after an SD
transfer, until the firmware sends a dummy byte after deasserting the card's CS.

**How to check.** Count slaves per CS net (`NET-7`); for slaves on a shared MISO, look up part data for "MISO
tri-states when deselected" (SD in SPI mode: only after a clock byte; some sensors and ADCs never).

**Fix.** One CS per slave; for slaves that do not release MISO, a tri-state buffer (74LVC1G125) gated by its CS,
or a separate bus; for SD cards, initialise them first and clock a byte after CS goes high.

**References.** Wikipedia, Serial Peripheral Interface "Multidrop configuration"; ELM MMC/SDC "Consideration on
Multi-slave Configuration"; PJRC SPI (step 2, tri-state buffer).

## `SER-19` SPI slave CS without a pull-up <a id="ser-19"></a>

⚠️ warning · needs: SPI CS pins, pulls, MCU pin reset state · [catalogue entry](../checks.md#ser-19)

**Impl.** mid · depends on NET-7, NET-9

**What it catches.** An SPI slave's CS driven by an MCU pin with no external pull-up (to the slave's supply).

**Why it matters.** While the host is in reset, booting or being reprogrammed, its CS pins are inputs. A floating
CS lets the slave see SCK and MOSI noise as commands: a flash can receive a stray write-enable and erase; with a
shared MISO a second slave answers while the boot ROM reads its own flash. The flash datasheet asks for CS to
follow VCC at power-up and power-down for the same reason.

**Example.** A W25Q128JV as a data flash next to the boot flash of an ESP32-S3: during the ROM's flash read at boot
the second flash's `/CS` floats and it answers, the ROM reads garbage and the chip boot-loops depending on humidity.

**How to check.** CS pins of SPI slaves (`NET-7`) whose net has no resistor to a rail and is driven by a pin that
floats in reset (`NET-9`).

**Fix.** A 10-100 kΩ pull-up per CS to the slave's supply.

**References.** W25Q128JV §4.1, §6.5 (pull-up on /CS); Wikipedia, Serial Peripheral Interface "Multidrop
configuration" (caveat on SS at initialisation); PJRC SPI (step 1, pull-up resistors).

## `SER-20` RS-232 levels straight into MCU pins <a id="ser-20"></a>

🛑 error · needs: connector class, signal path to the MCU · [catalogue entry](../checks.md#ser-20)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** An RS-232 connector (DE-9, a terminal block marked RS-232) whose TXD/RXD pins reach MCU pins
through nothing but resistors, with no RS-232 transceiver.

**Why it matters.** RS-232 uses inverted levels of ±5 V to ±15 V (receivers must take ±25 V). Into an MCU pin the
negative half forward-biases its ESD diode to ground and the positive half to the rail, latch-up and damaged pins
follow, and the logic is inverted anyway: a TTL UART reads RS-232 idle (negative) as a continuous break.

**Example.** A "debug UART on the DE-9" of an industrial board, wired to the STM32's USART2 with a 1 kΩ in series:
it works with a 3.3 V USB-TTL cable and burns PA3 the day someone plugs in a real PC serial port. A MAX3232 (3 to
5.5 V, two drivers and receivers, receiver input resistance specified over ±3 V to ±25 V) is the usual cure.

**How to check.** A connector of DE-9 class (or one the project marks as RS-232, `CFG-2`) whose pins reach MCU IO
pins through passives only (`NET-10`).

**Fix.** An RS-232 transceiver (MAX3232 class) between connector and MCU; or rename the connector "TTL UART" and use
a connector type that a PC serial cable does not fit.

**References.** MAX3232 §6.6 (driver output), §6.7 (receiver input resistance, ±25 V); TIA-232-F; Wikipedia, RS-232
"Voltage levels".

## `SER-21` USB-UART bridge RTS/DTR wired to boot and reset, or not <a id="ser-21"></a>

💡 suggestion · needs: bridge RTS/DTR pins, MCU reset and boot pins, intent · [catalogue entry](../checks.md#ser-21)

**Impl.** judgement · depends on NET-7

**What it catches.** A USB-UART bridge (CP2102N, CH340, FT232) next to an MCU whose ROM bootloader is reached over
UART: either its RTS/DTR drive the MCU's reset and boot pins (auto-programming) or they do not, and the schematic
should say which on purpose.

**Why it matters.** With auto-programming the flashing tool resets the chip into its bootloader by itself; without
it someone presses two buttons for every flash. Wired naively it bites: some terminal programs and the Linux
serial driver assert RTS and DTR when a port opens, holding the MCU in reset, which is why Espressif's circuit uses
two transistors so that both asserted together do not reset. And the ROM bootloader drives its own UART pins,
which matters when they double as something else (B153A: PA9 USART1_TX of the STM32 ROM cut the DUT's USB data).

**Example.** An ESP32-C3 board with a CP2102N: DTR to GPIO9 (boot) and RTS to EN directly, no transistor pair;
opening the port in a terminal with hardware flow control on holds the chip in reset, and the board "does not
boot when the serial monitor is open".

**How to check.** better_erc lists bridges whose RTS/DTR go to an MCU's reset or boot pins, through what, and
bridges whose RTS/DTR are unconnected next to such an MCU (`NET-7`); a human decides which is intended.

**Fix.** Either Espressif's two-transistor circuit (or the vendor's equivalent), or a note on the sheet that RTS/DTR
are deliberately left unconnected and how to enter the bootloader by hand.

**References.** esptool boot mode, "Automatic Bootloader"; CP2102N §4.3.11 (Modem Control, DTR); AN2606 §50.1
Table 111 (STM32G0B1 ROM bootloader pins); B153 rev B change list (PA9).

## `SER-22` 1-Wire with parasitic power and only a weak pull-up <a id="ser-22"></a>

💡 suggestion · needs: 1-Wire devices and their VDD wiring, the pull-up, a strong pull-up switch · [catalogue entry](../checks.md#ser-22)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A 1-Wire bus with parasitically powered devices (VDD tied to GND, as the DS18B20 demands in
that mode) and only the usual 4.7 kΩ pull-up, without a MOSFET that can pull the line hard to the rail.

**Why it matters.** A parasitic device takes its energy from the data line while it is high. A temperature
conversion or EEPROM write draws up to 1.5 mA, which a 4.7 kΩ cannot deliver without the line sagging below
what the device needs; conversions return the power-on value 85 °C or the device resets mid-write. Several
sensors converting together make it worse, and above 100 °C leakage makes parasitic power unreliable.

**Example.** Four DS18B20 on a 10 m cable in parasite mode on an ESP32 GPIO with 4.7 kΩ to 3.3 V: 12-bit
conversions read 85 °C on the far sensors. The datasheet asks for a strong pull-up switched on within 10 µs of a
Convert T and held for the whole conversion.

**How to check.** 1-Wire devices (part data, `NET-7`) whose VDD pin is on ground; on their DQ net, look for a
MOSFET from a rail besides the resistor (`NET-10`).

**Fix.** Power the devices from VDD (a third wire), or add a P-MOSFET strong pull-up driven by a GPIO; use a 1-Wire
master (DS2482) that has one built in.

**References.** DS18B20 "Powering the DS18B20" (1.5 mA, 10 µs, Figure 6); Wikipedia, 1-Wire "Power supplies".

## `SER-23` RS-485 or CAN node without a proper ground reference <a id="ser-23"></a>

⚠️ warning · needs: ground structure, isolation barrier, connector ground pins · [catalogue entry](../checks.md#ser-23)

**Impl.** hard · depends on NET-5

**What it catches.** An RS-485 or CAN node whose bus connector carries no ground (and the node is not isolated), or
whose signal ground is tied straight to the bus ground wire so the cable closes a ground loop.

**Why it matters.** The transceivers are differential, but only within a common-mode range (RS-485 −7 to +12 V,
CAN roughly −2 to +7 V for classic parts). Remote nodes powered from different parts of an installation can sit
volts apart; without a ground wire the common mode drifts out of range, with a hard ground wire the potential
difference drives current through the cable shield and signal ground. TI's guide shows both pitfalls and
recommends a resistor-separated ground (100 Ω) or, better, an isolated node.

**Example.** B125 and B135 isolate each CAN channel: an ADM3050E (5.7 kV rms signal isolation) with an isolated B0505MT
DC-DC for the bus side. On B135 the DE-9 ground pins (3, 6) reach only the isolated ground, the shell goes to it through
1 nF or a solder jumper, and a Y-rated capacitor (C508) bridges the barrier for EMC. The failure case is a non-isolated
RS-485 node with the DE-9 pin 5 tied to the board's GND and the board's GND tied to protective earth at the other end
too.

**How to check.** For each transceiver's bus side, find its ground net (`NET-5`); recognise an isolation barrier
(isolated transceiver, isolated DC-DC) between it and the logic ground; check whether the connector carries that
ground and how (direct, through a resistor). Whether the far side is grounded is outside the netlist.

**Fix.** An isolated transceiver with an isolated supply (best); otherwise a ground wire through a ~100 Ω resistor,
and a shield to chassis at one end.

**References.** SLLA272 §11 (Grounding and Isolation, Figure 11-1); ADM3050E features and "Applications
Information"; SLLA070 §2.2.5 (−7 V to 12 V); SLLA337 §2 (ground shift); SLOA101 §5.1.3 (common-mode range).

## `SER-24` RS-485 unit-load budget exceeded <a id="ser-24"></a>

⚠️ warning · needs: transceiver unit load (part data), bias network, node count (project) · [catalogue entry](../checks.md#ser-24)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** An RS-485 bus whose nodes and bias network together load the driver beyond 32 unit loads.

**Why it matters.** TIA-485-A sizes its drivers to deliver at least 1.5 V differential into 54 Ω: two 120 Ω terminations
in parallel with 32 unit loads (one UL is about 12 kΩ). Every node adds its receiver's load (1, 1/2, 1/4 or 1/8 UL), and
failsafe bias resistors add up to 20 UL. Past the budget the differential voltage at the far end drops below the
receivers' threshold plus noise.

**Example.** 64 nodes with 1/8 UL transceivers (THVD1450) look comfortable at 8 UL, but the 523 Ω bias pair of
`SER-10` costs up to 20 UL: SLLA272 gets N = (32 − 20) / UL, so 96 devices at 1/8 UL instead of 256. Mixing in old
full-UL nodes (SN75176B) eats the rest.

**How to check.** Sum unit loads from part data per transceiver (`NET-7`) times the node count from the project
(`CFG-2`), plus the load of bias networks found on the board (`SER-10`).

**Fix.** Fewer UL per node (1/8-UL transceivers), weaker or single bias networks, or a repeater to split the bus.

**References.** SLLA272 §8 (Bus Loading, equation 3); SLLA070 §2.2.3 (1.5 V minimum VOD into 2 × 27 Ω); THVD1450
§10.2.1.3 (Bus Loading); TIA-485-A.
