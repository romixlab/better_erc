# HS checks, explained

Detail page for the `HS` category of the [check catalogue](../checks.md#-hs---high-speed). Each section explains
one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it,
the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **SLLA414** · TI, *High-Speed Layout Guidelines for Signal Conditioners and USB Hubs*, SLLA414A (January 2026).
  §2.1-2.4 tabulate per interface whether AC caps are required and whether polarity inversion is allowed.
  [ti.com/lit/an/slla414](https://www.ti.com/lit/an/slla414/slla414.pdf)
- **SPRAAR7** · TI, *High-Speed Interface Layout Guidelines*, SPRAAR7J (February 2023).
  [ti.com/lit/an/spraar7](https://www.ti.com/lit/an/spraar7/spraar7.pdf)
- **SPRACP4** · TI, *Jacinto7 AM6x, TDA4x, and DRA8x High-Speed Interface Design Guidelines*, SPRACP4A (June 2024).
  USB, DisplayPort, PCIe and MIPI D-PHY schematics and rules in §3.1-3.4. [ti.com/lit/pdf/spracp4](https://www.ti.com/lit/pdf/spracp4)
- **AN4879** · ST, *Introduction to USB hardware and PCB guidelines using STM32 MCUs*, Rev 12 (June 2026). Fetched
  through `tpm fetch`, st.com refuses plain HTTP clients.
  [st.com AN4879](https://www.st.com/resource/en/application_note/an4879-usb-hardware-and-pcb-guidelines-using-stm32-mcus-stmicroelectronics.pdf)
- **RP2350 HW** · Raspberry Pi, *Hardware design with RP2350*, release 3 (August 2026).
  [datasheets.raspberrypi.com](https://datasheets.raspberrypi.com/rp2350/hardware-design-with-rp2350.pdf)
- **CM4** · Raspberry Pi, *Compute Module 4 Datasheet*, release 4 (June 2026); §2.3 is the PCIe interface.
  [datasheets.raspberrypi.com](https://datasheets.raspberrypi.com/cm4/cm4-datasheet.pdf)
- **USB251xB** · Microchip, *USB251xB/xBi USB 2.0 Hi-Speed Hub Controller*, DS00001692C; §5.1.29 is the Port Swap
  register. [microchip.com](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf)
- **USB 2.0** · USB-IF, *Universal Serial Bus Specification*, Revision 2.0 (2000).
- **Wikipedia** · readable overviews: [USB hardware](https://en.wikipedia.org/wiki/USB_hardware),
  [PCI Express](https://en.wikipedia.org/wiki/PCI_Express), [HDMI](https://en.wikipedia.org/wiki/HDMI),
  [DisplayPort](https://en.wikipedia.org/wiki/DisplayPort), [Camera Serial Interface](https://en.wikipedia.org/wiki/Camera_Serial_Interface).
- **Bogatin** · Eric Bogatin, *Signal and Power Integrity - Simplified*, 3rd ed. (Prentice Hall, 2018), chapter
  "Differential Pairs and Differential Impedance".
- **Johnson & Graham** · Howard Johnson and Martin Graham, *High-Speed Digital Design: A Handbook of Black Magic*
  (Prentice Hall, 1993), chapter "Clock Distribution".

## `HS-1` AC coupling caps on PCIe and DP lanes <a id="hs-1"></a>

🛑 error · needs: lane pin roles (TX/RX, which interface), series capacitors, connector vs on-board IC ·
[catalogue entry](../checks.md#hs-1)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A PCIe, USB 3 or DisplayPort main-link pair with the series AC coupling capacitors missing,
in the wrong place, or present twice: no cap on a transmitter's lane, caps added on a lane that the board
across the connector already caps, or caps on both ends of an on-board link.

**Why it matters.** These links are AC coupled by specification: transmitter and receiver each set their own
common-mode voltage, and only the series cap lets them disagree. Without it the two common modes fight through
the termination, the receiver sits outside its input range and the link does not train, or trains at a lower
rate. Two caps in series halve the capacitance and add a second discontinuity. The rule of thumb is one cap per
lane, at the transmitter side: a host board caps its own TX lanes, an add-in card caps its own TX lanes, and the
board that has both ends caps both directions. DisplayPort wants caps on all main-link lanes and on AUX; HDMI
and USB 2.0 want none (SLLA414 §2.1, §2.3).

**Example.** A Raspberry Pi CM4 carrier with an on-board VL805 USB 3 host controller instead of an M.2 socket.
The CM4 has caps on its own PCIe TX and REFCLK, but its RX pair needs caps close to the driving source, the
VL805's TX (CM4 §2.3). A carrier copied from the M.2 version, where the SSD brings its own TX caps, has none on
that pair, and the VL805 never appears on the bus.

**How to check.** Find the high-speed pairs by pin role of the IC (PCIe PETp/PERp, USB SSTX/SSRX, DP ML_Lane)
and follow each to its far end through series parts (`NET-10`). On an IC-to-IC link each TX pair needs exactly
one series cap; on an IC-to-connector link only the board's own TX pairs get one; RX pairs from a connector get
none. Part data must say when an IC has the caps inside (the CM4 module does on its TX lanes).

**Fix.** Add 0402 caps on the transmitter side of each missing lane (SPRACP4 Table 3-5 gives 175-265 nF for DP
main lanes; DP AUX takes 75-200 nF per the DP 1.2 note in AN346 §1.3); remove the duplicate where both ends cap
one lane. Same package for both caps of a pair, placed side by side (SPRAAR7 §3.10).

**References.** SLLA414 §2.1-2.4; SPRACP4 §3.1.1.1, §3.2.1.1 (Table 3-5), §3.3.1.4; CM4 §2.3; SPRAAR7 §3.10;
Pericom/Diodes *AN346 PI3EQXDP1201 DP Application Information* (2012) §1.3,
[diodes.com AN346](https://www.diodes.com/assets/App-Note-Files/AN346-P.pdf).

## `HS-2` Lane or bit swap: check that both ends support it <a id="hs-2"></a>

⚠️ warning · needs: lane and polarity pin roles on both ends, the part's swap features (library) ·
[catalogue entry](../checks.md#hs-2)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A link whose lanes arrive in a different order than they left (lane 0 on lane 3), or a pair
whose P and N, or D+ and D-, are exchanged, where the swap is allowed only if one of the parts can undo it.

**Why it matters.** Swaps are a routing convenience that some parts support and others do not, and support is often a
strap or a register the firmware must set. PCIe link training handles lane reversal and polarity inversion: TI's guide
treats lane assignment and polarity as free for its PCIe ports (SPRACP4 §3.3.1.1-3.3.1.2), but check the other end's
datasheet; USB 3 allows polarity inversion on SSTX and SSRX, USB 2.0 and HDMI allow none, DisplayPort has no built-in
support (SLLA414 §2.1-2.4). Redrivers and hubs add their own: a TMDS181 swaps its receive lanes with SWAP/POL low and
inverts their polarity with it high (retimer mode only); a TDP158 has a SWAP strap; a USB251xB hub swaps D+/D- per port.
The netlist cannot tell whether the strap or register is set, so the check warns instead of failing.

**Example.** On B153A hub ports 2 and 4 are wired with D+ and D- crossed: the plug's D+ reaches `USBDM_DN2`. The
USB2514B can undo this in its Port Swap register (FAh, bit 2 for port 2, bit 4 for port 4), but only once the MCU
has written it over SMBus. Before that a reMarkable 2 on port 2 showed up as "new low-speed USB device" with
error -32: its full-speed pull-up on D+ looked like a low-speed pull-up on D-.

**How to check.** Compare lane numbers and polarity of each pair at both ends (`NET-10` through caps, resistors
and connectors); for every difference look up whether either part can swap or invert (library data), and say
which pin or register has to do it.

**Fix.** Route straight where the parts cannot swap; where they can, write down the strap level or the register
value in the schematic and in the firmware's board support, and add a test that the link trains at full speed.

**References.** SPRACP4 §3.3.1.1-3.3.1.2; SLLA414 §2.1-2.4; USB251xB §5.1.29 (Port Swap); TI *TMDS181 datasheet*,
SLASE75D, §5 pin functions (SWAP/POL), [ti.com/lit/ds/symlink/tmds181](https://www.ti.com/lit/ds/symlink/tmds181.pdf);
TI *TDP158 Schematic Checklist*, SLLA452 (2019), [ti.com/lit/pdf/slla452](https://www.ti.com/lit/pdf/slla452).

## `HS-3` Clocked bus with only some lines through a buffer <a id="hs-3"></a>

⚠️ warning · needs: bus grouping (clock and its data lines), parts in series per line ·
[catalogue entry](../checks.md#hs-3)

**Impl.** hard · depends on NET-10

**What it catches.** A synchronous bus (SPI, QSPI, SDIO, RGMII, parallel RGB, I2S) where the clock passes through a
buffer, translator, divider, isolator or mux and some of its data lines do not, or pass through a different part.

**Why it matters.** Every part in the path adds propagation delay, a few nanoseconds for a logic translator and
tens for a digital isolator. If clock and data take different paths, the skew between them comes straight off
the setup or hold margin. The read direction is worse: the clock goes out through the buffer, and the slave's
data comes back through another one, so the master samples late by the round trip.

**Example.** An SPI flash at 50 MHz (10 ns half period) behind a level translator: SCK and MOSI go through it,
MISO comes back through a second channel. The MCU samples MISO on its own SCK edge, but the data was launched by
the delayed SCK at the flash and delayed again on the way back; at room temperature it works, hot or with a
slow batch of translators it reads garbage. The same happens when only the RGB pixel clock of an LCD gets a
buffer to "clean it up".

**How to check.** Group the lines of each bus by peripheral pins (`NET-12`), follow each line to the far device
(`NET-10`) and compare the series parts on the way. A difference in part count or type between the clock and
any data line of one bus is the finding; better_erc cannot compute the timing, only point at the asymmetry.

**Fix.** Put every line of the bus through the same kind of part (channels of one translator package match
best), lower the clock, or use the controller's sample-delay setting (QSPI and SDIO controllers often have one)
and check the timing with the datasheet numbers.

**References.** Johnson & Graham, chapter "Clock Distribution"; SPRAAR7 §2.3 (length matching within a bus).

## `HS-4` Differential pair asymmetry <a id="hs-4"></a>

⚠️ warning · needs: pair membership (net or pin names), parts on each line · [catalogue entry](../checks.md#hs-4)

**Impl.** easy · depends on netlist

**What it catches.** A differential pair whose two lines do not carry the same parts: a TVS, test point, stub,
pull resistor or filter cap on one line only, series parts of different value or package, or two different ESD
parts on P and N.

**Why it matters.** A differential receiver only rejects what both lines see alike. A part on one line converts
part of the differential signal into common mode and back: the eye closes, the pair radiates (EMI fails) and
picks up noise it would otherwise reject. Even matched parts are chosen in pairs for this reason: ESD arrays
specify the capacitance difference between channels (0.03 pF typical for the TPD2E2U06).

**Example.** A USB 2.0 pair with a single-line TPD1E10B06 (12 pF) on D+ only, because the symbol for D- was
forgotten: D+ is slowed and loaded, D- is not, and the high-speed eye fails while full speed still works.

**How to check.** Pair the nets by name (`_P`/`_N`, `+`/`-`, `DP`/`DM`) or by pin role, then compare the multiset of
parts (class, value, footprint) attached to each line. Whitelist the deliberate asymmetries: the D+ pull-up of a
full-speed USB device (`HS-6`), the DP AUX pull-up and pull-down pair (`HS-14`), RS-485 fail-safe bias.

**Fix.** Mirror the part on the other line, or use a two-channel part (array, dual resistor) that treats both
lines alike; keep test points as a pair or off the pair entirely.

**References.** SPRAAR7 §3.3 (symmetry in the differential pairs); SLLA414 §4.3; Bogatin, chapter "Differential
Pairs and Differential Impedance"; TI *TPD2E2U06 datasheet*, SLLSEG9C, electrical characteristics (ΔCIO-TO-GND),
[ti.com/lit/ds/symlink/tpd2e2u06](https://www.ti.com/lit/ds/symlink/tpd2e2u06.pdf).

## `HS-5` Series resistors on USB D+/D- <a id="hs-5"></a>

⚠️ warning · needs: USB data nets, series resistors, the PHY's need for them (library) ·
[catalogue entry](../checks.md#hs-5)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** Resistors in series with USB D+ and D- between the PHY and the connector, on a PHY that does
not ask for them.

**Why it matters.** A USB 2.0 PHY's drivers are designed to match the 90 Ω differential line by themselves
(SLLA414 §2.1). Extra series resistance adds to the driver impedance, lowers the high-speed swing and moves the
eye away from the mask; at full speed it is mostly harmless, at 480 Mbit/s it is a compliance failure. The
resistors usually come from an older reference design or from a PHY that needs them: the RP2040 and RP2350 do,
27 Ω close to the chip (RP2350 HW §5.1), so B200A rightly has R1810 and R1806 (27 Ω) between its RP2350 and the
connector. STM32 FS PHYs do not: AN4879 Figures 7-10 wire DP and DM straight to the connector. The catalogue's
F103 note belongs to this list of per-part exceptions, kept as library data.

**Example.** An STM32H7 board with an external ULPI PHY at high speed, its D+/D- carrying 22 Ω resistors copied
from an RP2040 design: enumeration works, high-speed eye tests and long cables fail.

**How to check.** Find resistors in series on USB data nets (by connector pin or PHY pin role, `NET-10`); report
them unless the PHY's library data says it wants a series resistor, and then compare the value.

**Fix.** Replace with 0 Ω or remove; keep the resistor only where the PHY's datasheet asks for it, at the value it
gives.

**References.** RP2350 HW §5.1 (USB, 27 Ω series termination); AN4879 §3.1 (Figures 7-10); SLLA414 §2.1; USB 2.0.

## `HS-6` USB D+ pull-up doubled or missing <a id="hs-6"></a>

⚠️ warning · needs: USB data nets, resistors to a rail, whether the PHY has the pull-up built in (library) ·
[catalogue entry](../checks.md#hs-6)

**Impl.** mid · depends on NET-7, IMP-9

**What it catches.** An external 1.5 kΩ pull-up on D+ (or D- for low speed) of a USB device whose PHY already
has it built in, or no pull-up on a device whose PHY lacks it.

**Why it matters.** The host detects a device and its speed by which line is pulled up. Without the pull-up the
device is never seen. With two, the line is pulled harder than the specification allows, the device cannot
disconnect in software (the external resistor stays on), and a high-speed PHY cannot complete its chirp, which
needs the full-speed pull-up switched off. A pull-up powered while VBUS is absent also back-drives the host.

**Example.** An STM32F103 needs the external 1.5 kΩ on USB_DP; most later STM32 families have it embedded
(AN4879 Table 3 lists it as embedded for the STM32F072, for example). A board moved from an F103 to an F072 keeps
the old resistor: two pull-ups in parallel, and the device can no longer disconnect in software, so a firmware
reset into a bootloader is not seen as a re-attach by the host.

**How to check.** For each USB device port find resistors from D+ and D- to a rail; compare with the PHY's
library flag "internal pull-up" (RP2350, most STM32) or "needs external 1.5 kΩ" (STM32F102/F103).

**Fix.** Remove the external pull-up on PHYs that have one; add 1.5 kΩ ± 1% to 3.0-3.6 V on those that do not,
switched by a GPIO or by VBUS detection so it is only on while VBUS is present (AN4879 §3.1).

**References.** AN4879 Table 3 ("Pull-up resistor on USB_DP line"), §3.1.1-3.1.2; RP2350 HW §5.1; USB 2.0.

## `HS-7` USB D+ and D- swapped <a id="hs-7"></a>

🛑 error · needs: USB data nets with pin names at the PHY and the connector · [catalogue entry](../checks.md#hs-7)

**Impl.** easy · depends on netlist

**What it catches.** A USB 2.0 data path where the part's D+ (DP, USBDP) pin reaches the connector's D- pin, or
the other way round, through any series resistors, ESD parts and chokes.

**Why it matters.** USB 2.0 has no polarity inversion (SLLA414 §2.1). With D+ and D- crossed, a full-speed
device's pull-up appears on D-, so the host sees a low-speed device and the first full-speed packet fails; a
high-speed device never gets past the chirp. A USB-C receptacle adds a trap: A6/A7 and B6/B7 must be joined
D+ to D+ and D- to D-.

**Example.** B153A port 2 (plug to the device under test) and port 4 (USB over SBU) are wired crossed: plug D+
reaches `USBDM_DN2`, SBU1 reaches `USBDM_DN4`. The hub can undo it in software (see `HS-2`), so on B153A this is a
warning; the same wiring on an MCU's own USB pins would be a dead port.

**How to check.** Match pin names on both ends (D+/DP/USBDP vs D-/DM/USBDM, connector pins A6/B6 = D+, A7/B7 =
D-), following series two-pin parts with `find_chains`; analog switches such as a TS3USB221A in the path need
signal following (`NET-10`). Downgrade to a warning when the part has a swap feature (library).

**Fix.** Cross the traces back (usually a symbol pin-swap on the connector or the ESD array), or set the part's
swap register and document it.

**References.** SLLA414 §2.1; USB251xB §5.1.29; Wikipedia, USB hardware "Pin assignments".

## `HS-8` Protection capacitance too high for the data rate <a id="hs-8"></a>

⚠️ warning · needs: protection part on high-speed nets, its line capacitance (library), the interface's rate ·
[catalogue entry](../checks.md#hs-8)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** An ESD or TVS part on a high-speed line whose capacitance is above what the data rate
tolerates: over about 1 pF on USB 2.0 high speed, over about 0.3-0.5 pF on USB 3, PCIe, HDMI and DP lanes.

**Why it matters.** The diode capacitance is a shunt load at the worst place, right at the connector: it slows
edges, reflects and adds insertion loss. A general-purpose TVS of 10-50 pF that is fine on a button or a UART
closes the eye at 480 Mbit/s and kills a 5 Gbit/s lane outright.

**Example.** TI's own ratings show the scale: the TPD1E10B06 (12 pF) is rated for data rates up to 400 Mbit/s,
below USB high speed; the TPD2E2U06 (1.5 pF typical) names USB 2.0 as its application; the TPD2EUSB30 (0.7 pF)
supports more than 5 Gbit/s and the TPDxE05U06 (0.42-0.5 pF) up to 6 Gbit/s. A TPD1E10B06 pair on a high-speed
USB port is the finding. B200A's RCLAMP0502BA on a full-speed RP2350 port is not: 12 Mbit/s tolerates far more.

**How to check.** For each protection part on a classified high-speed net, read its line capacitance or rated
data rate from part data and compare with the interface's rate (from the PHY's pin role, or a project setting
when the speed is not obvious, `CFG-2`).

**Fix.** Pick a part rated for the interface (the datasheet's data-rate line is the better guide than the
thresholds above), with flow-through pinout, placed right at the connector (SPRAAR7 §3.13-3.14).

**References.** TI datasheets: *TPD1E10B06*, SLLSEB1G, [ti.com](https://www.ti.com/lit/ds/symlink/tpd1e10b06.pdf);
*TPD2E2U06*, SLLSEG9C, [ti.com](https://www.ti.com/lit/ds/symlink/tpd2e2u06.pdf); *TPDxEUSB30*, SLVSAC2G,
§8.2.2.2, [ti.com](https://www.ti.com/lit/ds/symlink/tpd4eusb30.pdf); *TPDxE05U06*, SLVSBO7O,
[ti.com](https://www.ti.com/lit/ds/symlink/tpd4e05u06.pdf); SPRAAR7 §3.13-3.14.

## `HS-9` Common-mode choke on USB 3 or PCIe lanes <a id="hs-9"></a>

💡 suggestion · needs: high-speed lane nets, four-terminal chokes in series · [catalogue entry](../checks.md#hs-9)

**Impl.** easy · depends on netlist

**What it catches.** A common-mode choke fitted in series with a USB 3, PCIe or other multi-gigabit lane.

**Why it matters.** At 5 Gbit/s and above a choke's pads, windings and the via transitions to reach it add more
loss and reflection than the common-mode noise it removes; TI recommends 0402 0 Ω no-stuff options because larger
parts typically introduce more loss than the filter itself (SLLA414 §4.14). Chokes are an EMC fix to be proven on
the test bench, not a default part.

**Example.** A USB 3.0 Type-A host port with an 0805 wire-wound choke on SSTX and SSRX copied from a USB 2.0
design: SuperSpeed devices fall back to USB 2.0 on long cables.

**How to check.** A four-terminal choke (library class or reference prefix with two windings) in series with both
lines of a lane whose pins are PCIe, USB 3, SATA, DP or HDMI.

**Fix.** Default to a 0402 0 Ω footprint pair that can take a choke if EMC testing calls for it; keep AC caps on
the protected side of it, everything close to the connector (SLLA414 §4.14).

**References.** SLLA414 §4.13-4.14; SPRAAR7 §3.14.

## `HS-10` Differential pair polarity swapped <a id="hs-10"></a>

🛑 error · needs: pair membership and polarity at both ends, the receiver's polarity-inversion support (library) ·
[catalogue entry](../checks.md#hs-10)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A differential pair whose P reaches N at the far end, across a connector or between two
parts on the board, where the receiver cannot invert polarity.

**Why it matters.** An inverted pair delivers inverted data. Some links fix it in training: PCIe requires
polarity inversion support (SPRACP4 §3.3.1.1) and USB 3 allows it on SSTX and SSRX; USB 2.0 and HDMI do not, and
DisplayPort has no built-in support (SLLA414 §2.1-2.4). Where it is not supported the link is simply dead, and
the swap is often invisible on the schematic because it happens in a connector symbol's pin order.

**Example.** An HDMI output through a board-to-board connector whose symbol numbers CLK+ and CLK- the other way
round than the mating board: no picture on any monitor. A TMDS181 retimer in the path could invert its receive
lanes with SWAP/POL high, but only in retimer mode.

**How to check.** Pair nets by name or pin role, follow each to the far end through caps and connectors
(`NET-10`, and `NET-13` across boards), compare polarity; error when neither end can invert, `HS-2` warning when
one can.

**Fix.** Swap the traces back (often a pin-swap in the connector symbol), or use the inversion feature of a part
that has it and document the setting.

**References.** SPRACP4 §3.3.1.1; SLLA414 §2.1-2.4; TI *TMDS181 datasheet*, SLASE75D, §5 (SWAP/POL).

## `HS-11` PCIe sideband signals missing or unpulled <a id="hs-11"></a>

🛑 error · needs: PCIe pin roles on host, device and connector, pulls to rails · [catalogue entry](../checks.md#hs-11)

**Impl.** mid · depends on NET-7, NET-5

**What it catches.** A PCIe link where PERST#, CLKREQ#, WAKE# or REFCLK are not connected between host and
device, or open-drain sideband lines have no pull-up; on an add-in card, PRSNT1# not joined to the farthest
PRSNT2#.

**Why it matters.** The data lanes alone do not make a link. The device stays in reset without PERST# from the
host; a host that gates its reference clock on CLKREQ# never starts it if the line is not wired; WAKE# and
CLKREQ# are open-drain and float without a pull-up (WAKE# from standby power, so a card can signal it is wake
capable); a slot that sees no PRSNT loop believes it is empty.

**Example.** A CM4 carrier with an M.2 key M socket where CLKREQ# is left unconnected because "the SSD does not
need it": the CM4 datasheet says PCIe_CLK_nREQ must be connected to ensure the CM4 produces a clock, and
PCIe_nRST should be connected so the device is reset properly. Without it no clock, no link, no SSD.

**How to check.** For each PCIe controller, find its sideband pins by role and check that each reaches the
connector or device; check pulls on open-drain lines to the right rail (`NET-5`) and that an internal pull on
the host (CM4 has one on CLK_nREQ) is counted.

**Fix.** Wire PERST#, CLKREQ#, WAKE# and REFCLK through, with pull-ups on the open-drain lines; on a card, join
PRSNT1# to the PRSNT2# of the widest lane count.

**References.** CM4 §2.3 and pinout (PCIe_CLK_nREQ, PCIe_nRST); Wikipedia, PCI Express "Pinout" (PRSNT, WAKE#
pull-up); Renesas/IDT *89HPES16T4G2 & 89HPES12T3G2 Hardware Design Guide* (PERST# timing),
[renesas.com](https://www.renesas.com/document/gde/gen2-pcie-hardware-design-guide).

## `HS-12` TX and RX not crossed between host and device <a id="hs-12"></a>

🛑 error · needs: lane direction pin roles on both ends, connector pin naming convention ·
[catalogue entry](../checks.md#hs-12)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A PCIe, USB 3 or SATA link where a transmitter's pair reaches the other side's transmitter
instead of its receiver.

**Why it matters.** Each direction is a separate pair, and TX must meet RX. The confusion comes from connectors:
their pin names are written from one side's point of view, so wiring to a connector is usually straight, and
wiring IC to IC needs the cross. The CM4 datasheet says exactly this: TX and RX swap when wiring directly to an
IC, and no swap to a connector labelled from the host's point of view. Two transmitters face to face never
train, and nothing gets damaged, so the mistake survives until bring-up.

**Example.** A carrier that replaces an M.2 SSD socket with an on-board NVMe controller and keeps the net names:
`PCIE_TX_P` from the CM4 now goes to the controller's TX pins. Note the deliberate exception on B153A: the plug's
SuperSpeed TX1+ and TX2+ pins are used as plain GPIOs (IO5, IO6), which the check must accept as a waiver.

**How to check.** Follow each lane pair (`NET-10`) and compare the pin roles at both ends: IC TX to IC RX is
right, IC TX to IC TX is the finding; for connectors use the convention of the connector family (library data:
named from the host or from the card).

**Fix.** Cross the pairs on the schematic (TX to RX on both directions) and keep the AC caps on the TX side
(`HS-1`).

**References.** CM4 §2.3; Wikipedia, PCI Express "Pinout"; SLLA414 §2.2.

## `HS-13` HDMI +5V, DDC and HPD <a id="hs-13"></a>

🛑 error · needs: HDMI connector pins, rails, pulls, level translators · [catalogue entry](../checks.md#hs-13)

**Impl.** mid · depends on NET-7, NET-5, NET-10

**What it catches.** An HDMI source without its own +5 V on pin 18; DDC (SCL, SDA) wired from a 1.8 or 3.3 V SoC
straight to the connector without a translator, or without the 5 V pull-ups; HPD without its pull-down on the
source side.

**Why it matters.** The source powers +5 V for the sink: the sink's EDID memory and its HPD answer are expected to
work from it, so a source without +5 V often gets no EDID and no hot-plug. DDC is I2C at 5 V on the cable side:
an SoC pin pulled to 5 V through the monitor's pull-ups is overstressed, and one with only 3.3 V pull-ups gives
highs that a 5 V sink reads marginally. HPD floats when no monitor is attached unless the source pulls it down.
+5 V also needs current limiting and reverse blocking: two sources joined by one cable must not power each other.

**Example.** An SoC board with HDMI DDC on 3.3 V I2C pins, 2.2 kΩ to 3.3 V, and pin 18 fed from a 5 V rail
without a limit: EDID reads fail on some monitors, the SoC pins see 5 V through monitors with their own pull-ups,
and a short in a cable trips the board's 5 V. The TPD12S016 class of companion chip solves all three: 55 mA
limited 5V_OUT with reverse blocking, an auto-direction DDC translator, and the HDMI-specified pulls on DDC, CEC
and HPD built in.

**How to check.** On an HDMI connector used as a source (TMDS pins driven by a TX), check pin 18 reaches a rail
through a current-limiting part, DDC pins reach the SoC through a translator with pull-ups to 5 V on the cable
side (TI's TDP158 checklist: 2 kΩ to 5 V for a source, 47 kΩ for a sink), and HPD has a pull-down.

**Fix.** Use an HDMI companion chip (TPD12S016 class) or the discrete equivalent: limited and reverse-blocked
+5 V, an I2C translator for DDC with 2 kΩ to 5 V on the connector side, HPD divider or buffer to the SoC's level
with a pull-down.

**References.** TI *TPD12S016 datasheet*, SLLSE96F, §7.3.6 (55 mA limit), §7.3.8 (back-drive), §7.3.9 (pulls),
§7.3.11 (DDC level shift), [ti.com](https://www.ti.com/lit/ds/symlink/tpd12s016.pdf); TI SLLA452 (TDP158 DDC
pull-ups); Wikipedia, HDMI (pinout, "Display Data Channel").

## `HS-14` DisplayPort AUX pulls, HPD, DP_PWR, DP++ <a id="hs-14"></a>

🛑 error · needs: DP connector pins, AUX caps and pulls, rails · [catalogue entry](../checks.md#hs-14)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A DisplayPort source whose AUX pull-down (AUX+) and pull-up (AUX-) are missing, swapped, or on
the wrong side of the AUX caps; HPD not wired; DP_PWR (pin 20) missing or tied to another device's DP_PWR; a
dual-mode (DP++) connector without the cable adaptor detect (pin 13) handling, or with DP++ promised on a
native-only source.

**Why it matters.** The sink detects a source by AUX+ pulled low and AUX- pulled high; the pulls must sit between the
AUX caps and the connector (10-105 kΩ per DP 1.2, 100 kΩ typical), because on the SoC side the caps block their DC level
and the sink never sees them. DP_PWR is 3.3 V for adaptors and must not meet another source's DP_PWR through a cable.
For DP++ the source must pull the cable adaptor detect pin down (about 1 MΩ) so a DP-to-HDMI adaptor can pull it up;
then the source switches to TMDS on the main lanes and DDC on the AUX pins, with 2 kΩ DDC pull-ups enabled by that
signal.

**Example.** An SoC carrier with the 100 kΩ AUX pulls placed next to the SoC, before the 100 nF AUX caps: the
monitor sees no source and stays in standby. The same board with a DP++ logo but no adaptor-detect pull-down and no DDC
pull-ups: passive HDMI adaptors show nothing.

**How to check.** On a DP connector used as a source: AUX+ has a resistor to ground and AUX- one to 3.3 V on the
connector side of the series caps (`NET-10`), HPD reaches the SoC, DP_PWR comes from a rail through a limit; for
DP++ also the adaptor-detect pull-down and the switched DDC pull-ups.

**Fix.** Move the pulls to the connector side of the caps, wire HPD and DP_PWR, and either implement the dual-mode
detect fully or drop the DP++ claim.

**References.** Pericom/Diodes AN346 §1.3 (AUX caps and pulls), §1.4 (cable adaptor detect), §1.5 (DDC);
SPRACP4 §3.2.1; Wikipedia, DisplayPort "DP_PWR (pin 20)", "DisplayPort Dual-Mode (DP++)".

## `HS-15` Series parts on MIPI DSI or CSI lanes <a id="hs-15"></a>

🛑 error · needs: MIPI lane nets (pin roles), parts on them · [catalogue entry](../checks.md#hs-15)

**Impl.** easy · depends on netlist

**What it catches.** A resistor, capacitor, filter, choke or protection part in series with or shunting a MIPI
D-PHY lane (DSI to a display, CSI-2 from a camera) between the SoC and the connector.

**Why it matters.** D-PHY lanes switch between two modes on the same wires: high speed, a small differential
swing into the receiver's on-die 100 Ω termination, and low power, single-ended 1.2 V signalling with the
termination off. A series resistor shifts the high-speed levels, a shunt capacitance slows the low-power edges,
and an EMI filter breaks both. Vendor reference designs route the lanes straight from the SoC to the connector,
vias only (SPRACP4 Figures 3-8, 3-9).

**Example.** A camera FFC connector with a 22 Ω resistor array and an RC EMI filter on the CSI-2 clock and data
lanes, copied from a parallel DVP camera design: the sensor's low-power commands get through, the high-speed
frames do not.

**How to check.** Find lane pins by role (DSI/CSI CLK and D0-D3 P/N) and list every part other than the SoC and
the connector on those nets. Accept parts whose library data says they are D-PHY rated (MIPI switches, filters
designed for it) as a waiver.

**Fix.** Route the lanes straight; if ESD is needed, use a part specified for MIPI D-PHY rates and say so.

**References.** SPRACP4 §3.4.1 (Figures 3-8, 3-9), §3.4.2; Wikipedia, Camera Serial Interface.

## `HS-16` Reference clock termination vs receiver type <a id="hs-16"></a>

🛑 error · needs: clock source output type, receiver input type and internal termination (library), the passives
between · [catalogue entry](../checks.md#hs-16)

**Impl.** hard · depends on NET-7, NET-10

**What it catches.** A differential reference clock whose termination does not fit its output and input
standards: classic HCSL without its 50 Ω to ground, LVDS without its 100 Ω across the receiver, LVPECL without its
bias to VCC − 2 V, or AC coupling into a receiver that has no bias of its own.

**Why it matters.** Each standard has its own driver: classic HCSL is a 15 mA current source that only develops a
swing across an external 50 Ω (49.9 Ω) to ground; LVDS drives about 3.5 mA into a 100 Ω termination at the
receiver; LVPECL emitter followers need 50 Ω to VCC − 2 V. Leave out the matching resistor and the clock does not
switch, or switches with the wrong swing and common mode. An AC coupling cap removes the DC level, so the
receiver side then needs a bias network unless the input biases itself. Low-power HCSL outputs differ again:
they need only series resistors, and an added 49.9 Ω to ground just loads them.

**Example.** A 100 MHz PCIe reference from a classic HCSL buffer to an on-board endpoint without the two 49.9 Ω
to ground: no swing, no link. The reverse: a low-power HCSL generator (9FGL class) with the 49.9 Ω kept from an
older reference design. And an LP-HCSL output feeding an FPGA's LVDS refclk input directly, which needs the
alternative termination network of the clock vendor (Skyworks AN781 §2).

**How to check.** Needs the output type of the source pin and the input type and internal termination of the
receiver pin (part data), then the passives between them (`NET-10`): 50 Ω to ground per line for classic HCSL,
100 Ω across for LVDS (unless internal), the LVPECL bias network, a bias network after any series cap.

**Fix.** Use the termination from the clock part's datasheet for the receiver at hand: for classic HCSL 22-33 Ω
series plus 49.9 Ω to ground, at the source when the receiver is on another board and near the receiver on the
same board (AN-808 Figures 1, 2); for LVDS 100 Ω at the receiver; for mixed standards AC coupling with a bias
network or the vendor's conversion network.

**References.** Renesas/IDT *AN-808 PCI Express/HCSL Termination* (2013), Figures 1-3,
[renesas.com](https://www.renesas.com/en/document/apn/808-pci-expresshcsl-termination); TI *Interfacing Between
LVPECL, VML, CML, and LVDS Levels*, SLLA120 (2002), §3.1.1, §3.4.1-3.4.2, §5,
[ti.com/lit/an/slla120](https://www.ti.com/lit/an/slla120/slla120.pdf); TI *AC-Coupling Between Differential
LVPECL, LVDS, HSTL, and CML*, SCAA059C (2007), §1, [ti.com/lit/an/scaa059](https://www.ti.com/lit/an/scaa059/scaa059.pdf);
Skyworks *AN781 Alternative Output Termination for ... PCIe Clock Generator and Buffer Families*, Rev. 0.3 (2021),
[skyworksinc.com](https://www.skyworksinc.com/-/media/Skyworks/SL/documents/public/application-notes/an781-alternative-output-termination.pdf).
