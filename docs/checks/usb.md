# USB checks, explained

Detail page for the `USB` category of the [check catalogue](../checks.md#-usb---usb-and-type-c). Each section
explains one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to
find it, how hard that is to implement, the usual fix, and where to read more. The one-line entries in the
catalogue stay the source of truth for ID, severity and wording; this page is the commentary.

The **Impl.** line says how hard the check is to implement in better_erc without an LLM: `easy` is a graph query on
the netlist alone, `mid` needs part data or a project setting first, `hard` needs analysis the code does not have
yet, `judgement` needs a person (better_erc can at most list candidates). The IDs after it are the
[FEATURES.md](../../FEATURES.md) items the check depends on.

References used throughout:

- **Type-C** · USB-IF, *USB Type-C® Cable and Connector Specification*, Release 2.5 (March 2026). Section and table
  numbers below refer to it (the PDF is in the zip on the document library page).
  [usb.org/document-library/usb-type-cr-cable-and-connector-specification-release-25](https://www.usb.org/document-library/usb-type-cr-cable-and-connector-specification-release-25)
- **USB 2.0** · USB-IF, *Universal Serial Bus Specification*, Revision 2.0, with its ECNs (zip of 3 Jun 2025, file
  `usb_20.pdf`). [usb.org/document-library/usb-20-specification](https://www.usb.org/document-library/usb-20-specification)
- **Micro-USB** · USB-IF, *Micro-USB Cables and Connectors Specification*, Rev. 1.01 (April 2007), in the same USB
  2.0 zip.
- **AN4879** · ST, *Introduction to USB hardware and PCB guidelines using STM32 MCUs*, Rev 12 (June 2026), fetched
  through `tpm fetch` (st.com refuses plain HTTP clients).
  [st.com/resource/en/application_note/an4879](https://www.st.com/resource/en/application_note/an4879-usb-hardware-and-pcb-guidelines-using-stm32-mcus-stmicroelectronics.pdf)
- **AN5225** · ST, *Introduction to USB Type-C® Power Delivery for STM32 MCUs and MPUs*, Rev 7 (June 2023).
  [st.com/resource/en/application_note/an5225](https://www.st.com/resource/en/application_note/an5225-usb-typec-power-delivery-using-stm32-mcus-and-mpus-stmicroelectronics.pdf)
- **TPD6S300A** · TI, *TPD6S300A USB Type-C™ Port Protector: Short-to-VBUS Overvoltage and IEC ESD Protection*,
  SLVSEL8 (June 2018). [ti.com/lit/ds/symlink/tpd6s300a.pdf](https://www.ti.com/lit/ds/symlink/tpd6s300a.pdf)
- **USB251xB** · Microchip, *USB251xB/xBi USB 2.0 Hi-Speed Hub Controller*, DS00001692C (2015).
  [ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf](https://ww1.microchip.com/downloads/en/DeviceDoc/00001692C.pdf)
- **Wikipedia, USB-C** · a readable overview of CC roles, receptacles, Debug Accessory Mode and the pin usage per
  mode. [en.wikipedia.org/wiki/USB-C](https://en.wikipedia.org/wiki/USB-C)
- **Wikipedia, USB hardware** · connectors, pin assignments and power of the legacy connectors.
  [en.wikipedia.org/wiki/USB_hardware](https://en.wikipedia.org/wiki/USB_hardware)
- **Ott** · Henry W. Ott, *Electromagnetic Compatibility Engineering* (Wiley, 2009), §2.15 (shield terminations),
  §3.3 (equipment and system grounding), §16.2 (PCB-to-chassis ground connection).

## `USB-1` Type-C without 5k1 <a id="usb-1"></a>

🛑 error · needs: Type-C connector, CC pins, resistors to ground · [catalogue entry](../checks.md#usb-1)

**Impl.** easy · depends on NET-5, IMP-6

**What it catches.** A Type-C receptacle on a sink (a device that takes power) whose CC1 and CC2 have no 5.1 kΩ to
ground and go to no CC/PD controller that could present Rd itself.

**Why it matters.** A Type-C source keeps VBUS off until it sees Rd on a CC pin: that is how it knows a sink is
attached and which way the plug is turned. Without Rd a C-to-C cable from a laptop or a Type-C charger gives no
power at all. The board still "works" on the bench with a C-to-A cable, whose A end always has VBUS and whose C
plug carries its own 56 kΩ Rp, so the mistake survives until the first customer with a modern laptop.

**Example.** B200A (rePort) does it right: CC1 and CC2 of J1601 each have their own 5K1 to GND (R1603, R1604), a
USB 2.0 sink with default power and no PD. Leave them out and the rePort is dead on every C-to-C host.

**How to check.** Find Type-C receptacles by symbol or pin names (CC1/CC2, A5/B5), follow each CC net: a resistor
of 5.1 kΩ (±20 %, or ±10 % if the sink reads the advertised current) to a ground net, or a pin of a part that
terminates CC itself, must be there.

**Fix.** One 5.1 kΩ to GND per CC pin (never one shared, see `USB-9`), or a PD controller or port protector with
dead-battery Rd.

**References.** Type-C Table 4-28 (Sink CC termination Rd), §4.5.1.2.1 (detecting a valid source-to-sink
connection), Table 4-10 (source perspective); AN5225 §11.3.1 (dead-battery Rd on STM32 UCPD); Wikipedia, USB-C
"USB 2.0/1.1".

## `USB-2` No overvoltage protection on Type-C CC and SBU <a id="usb-2"></a>

⚠️ warning · needs: CC and SBU nets, protection part type (library) · [catalogue entry](../checks.md#usb-2)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** CC1, CC2, SBU1 or SBU2 of a Type-C connector reaching an IC pin with no part rated for a
short to VBUS in between: no OVP switch (TPD6S300A, TCPP01/03, DPO2036 class) and no 24 V-capable clamp.

**Why it matters.** On a Type-C receptacle CC and SBU sit right next to VBUS (A4/A5 and B8/B9). A tilted plug, a
bent contact, debris or a non-compliant cable can short them, and with USB PD VBUS may be 20 V. Ringing on a hot
short can reach twice the settled voltage. A CC/PD controller or MCU pin rated 5.5 V dies, and with it the port.
At 5 V only, most parts survive; the risk is real once the port negotiates more.

**Example.** B153A was built with both DPO2036 (U402 receptacle, U501 plug) removed and their footprints shorted
across. A CC or SBU short to a 20 V PD VBUS then lands straight on PA8, PB15 (UCPD1), PA13, PA14 (SWD) on the
receptacle side, PD0 and PD2 on the plug side. The board reference warns the firmware and the user; rev B has to
decide the protection again.

**How to check.** Follow each CC and SBU net from the connector to the first IC pin; a part on the path must be of
the OVP-switch class, or the IC pin itself must be rated for the highest VBUS the port can see (from the project
or from a PD controller on the board).

**Fix.** A Type-C port protector on CC and SBU (TPD6S300A for both, TCPP01-M12 for sinks); for a 5 V-only port a
series resistor plus a clamp may do, but say so.

**References.** TPD6S300A §8.3.1 (short-to-VBUS protection, 24 V DC, ringing to ~2×), §9.2.2.4; AN5225 §14.1
(ESD/EOS protection: 20 V-compliant parts for CC and SBU); Type-C Table 3-4 (pin positions).

## `USB-3` Exposed SBU with a pull-down stronger than zSBUTermination <a id="usb-3"></a>

⚠️ warning · needs: SBU nets, resistance to ground, pin data of what SBU reaches · [catalogue entry](../checks.md#usb-3)

**Impl.** mid · depends on NET-7, IMP-6

**What it catches.** SBU1 or SBU2 of a port wired, outside an alternate mode, to a resistor or an input whose
impedance to ground is below zSBUTermination (950 kΩ).

**Why it matters.** The standard allows SBU on a port without USB4 or an alternate mode to be either open or weakly
pulled down, no stronger than 950 kΩ, "functionally equivalent to an open circuit". The partner may use SBU for
liquid detection or try an alternate mode, and a strong pull-down looks like a load or like liquid. A port that
uses SBU for its own purpose must also meet the USB Safe State (0-1.5 V common mode) while switching.

**Example.** On B153A the receptacle SBU1 goes (through the DPO2036 footprint) to PA14 `SWCLK_BOOT0`, which carries
a 100 kΩ pull-down R209 for BOOT0, plus the STM32's own SWCLK pull-down: well under 950 kΩ, on a host-facing port
that never enters an alternate mode. B200A leaves SBU unconnected, which is compliant.

**How to check.** For each SBU net, the resistance to ground through resistors (and the pin's pull from the part
data, where it has one); flag anything below 950 kΩ unless the project says the port enters an alternate mode.

**Fix.** Leave SBU open, or put a switch between SBU and the circuit that is off by default; use pull-downs of
1 MΩ or more where a defined level is needed.

**References.** Type-C §4.3 (SBU), Table 4-31 (zSBUTermination ≥ 950 kΩ), Appendix E Table E-1 (USB Safe State);
AN5225 §8.1 (SBU open when no alternate mode).

## `USB-4` USB shield connected through L/R/C, or not connected <a id="usb-4"></a>

⚠️ warning · needs: connector shell pins, what they connect to · [catalogue entry](../checks.md#usb-4)

**Impl.** easy · depends on netlist

**What it catches.** The shell or shield pins of a USB connector left unconnected, or tied to ground through a
ferrite, resistor or capacitor network, so the designer can confirm it is a decision and not an accident.

**Why it matters.** The cable shield only works when it is terminated with low impedance at the frequencies that
matter: a floating shell is an antenna for the cable's common-mode current and an ESD entry with no path, and a
ferrite in series is high impedance exactly where the shield should be low. On the other hand, a shell hard-tied
to signal ground joins chassis and signal ground of two products through the cable. The Type-C spec asks for the
receptacle shell and mid-plate to be soldered to the PCB ground plane with several tails and for the receptacle to
be bonded to a metal chassis where there is one. A deliberate RC (1 MΩ ∥ 4.7 nF to GND) is a valid choice for some
products, but it must be one.

**Example.** A Type-C receptacle whose four shell tabs are on a net `SHIELD` that reaches GND only through a 600 Ω
@ 100 MHz ferrite "for EMC": radiated emissions on the cable rise, and an 8 kV contact discharge on the shell has
to go through the ferrite and the signal ground of the USB PHY.

**How to check.** Pins named or typed as shield/shell (`SH`, `S1`-`S4`, `SHIELD`) on USB connectors: unconnected,
or connected to a ground only through an inductor, resistor or capacitor. Report the topology for a decision.

**Fix.** Shell straight to the ground plane with several vias, and to chassis where there is one; if a separate
chassis ground is wanted, an RC or a direct chassis bond chosen on purpose and noted on the schematic (`GND-7`).

**References.** Type-C §3.10.1 (EMC management: mid-plate and shell to the PCB ground plane, receptacle to
chassis); Ott §2.15, §3.3, §16.2; AN4879 §2.3 (ESD and EMI).

## `USB-5` USB device with more than 10 µF on VBUS <a id="usb-5"></a>

⚠️ warning · needs: port role, capacitors on VBUS up to the first current limit · [catalogue entry](../checks.md#usb-5)

**Impl.** mid · depends on NET-5, NET-10, IMP-6

**What it catches.** A device (upstream-facing) port whose VBUS carries more than 10 µF of capacitance, directly or
through a regulator, without an inrush limiter in front of it.

**Why it matters.** USB 2.0 limits what a device may show at attach to 10 µF in parallel with 44 Ω, so the host's
VBUS droops by no more than 330 mV and the other ports on the hub stay up. A larger capacitance charges from the
host's bulk at attach: the host's port droops, other devices on the same hub reset or disconnect, and an
over-current limit may trip. More bulk is allowed only behind a soft-start or a current limit.

**Example.** On the P2625 bench, B153A switched its plug VBUS hard onto a B129A with more than 110 µF on its input:
the dongle's own supply dipped and the hub reset. A 10 Ω in series in the wire cured it; the rev B change list asks
for a soft start on that switch. The same B129A plugged into a laptop port droops the laptop's port in the same
way.

**How to check.** Decide the port role (Rd on CC, a Type-B or Micro-B receptacle, a device PHY); sum the
capacitance on the VBUS net and on nets reached through ferrites, 0 Ω ties and regulators that have no soft start,
up to the first load switch or limiter; compare with 10 µF.

**Fix.** Keep ≤ 10 µF on VBUS and put the rest behind a load switch with controlled rise time (AN5225 suggests
a gate-drain capacitor of 100 pF per 10 µF of load); or a dedicated inrush limiter.

**References.** USB 2.0 §7.2.4.1 (inrush current limiting: 10 µF ∥ 44 Ω, 330 mV droop); AN5225 §14.3.1 (sink
mode: Cvbus 2.2 µF, bulk behind a switch), §14.3.4 (limit the inrush current in sink mode).

## `USB-6` USB host port with less than 120 µF bulk on VBUS <a id="usb-6"></a>

⚠️ warning · needs: port role, capacitors on VBUS · [catalogue entry](../checks.md#usb-6)

**Impl.** mid · depends on NET-5, CFG-2, IMP-6

**What it catches.** A host or hub downstream-facing port that supplies VBUS with less than 120 µF of low-ESR
capacitance behind its power switch.

**Why it matters.** The 120 µF is the other half of the inrush budget of `USB-5`: when a device with its allowed
10 µF is attached, the charge comes from the host's bulk and the droop stays under 330 mV. With only a few µF the
port voltage collapses on every attach, the devices already on that supply brown out, and the port switch may
read the attach as a short and latch off.

**Example.** A carrier board whose downstream Type-A port is fed from the 5 V rail through a TPS2051-class
switch with 10 µF after it: plugging in a USB stick with 10 µF dips VBUS by more than a volt, and the hub on the
same rail drops its other devices.

**How to check.** Ports that source VBUS (Rp on CC, a Type-A receptacle, a power switch from a rail to VBUS, or the
project says host); sum the capacitance on VBUS behind the switch; compare with 120 µF per hub (per port group
sharing one supply).

**Fix.** 120 µF of low-ESR bulk (polymer, or ceramics with derating counted, `RPT-1`) close to the connectors.
Type-C sources follow AN5225's values for the connector side, with the bulk behind the switch.

**References.** USB 2.0 §7.2.4.1 (CHPB no less than 120 µF per hub); AN5225 §14.3.2 (source mode capacitors).

## `USB-7` More than one Rd or Rp on a CC line <a id="usb-7"></a>

🛑 error · needs: CC nets, resistors, which IC pins terminate CC internally · [catalogue entry](../checks.md#usb-7)

**Impl.** mid · depends on NET-7, NET-12, IMP-6

**What it catches.** Two terminations on one CC pin: a fixed 5.1 kΩ next to a PD controller or MCU (STM32 UCPD) that
switches its own Rd on, two Rp, or an Rp and an Rd that can be on together.

**Why it matters.** The Rd and Rp values in the spec are the total resistance seen at the pin, including every
internal and parallel path. Two Rd in parallel make ~2.55 kΩ, outside the 5.1 kΩ ±20 % window: the sink reads
the advertised current one step low, and against a default-current Rp the voltage sits at the Ra threshold, so
some sources see a powered cable and no sink. BMC messaging does not care about the DC level, but the port
is out of spec and fragile.

**Example.** B153A's receptacle got a hand-added 5K1 Rd on both CC lines after the DPO2036 removal, so the board
powers up from a C-to-C host with an empty MCU. UCPD1 on PA8/PB15 switches its own Rd on together with its
receivers, so enabling PD makes 5K1 ∥ 5K1. Against a 56 kΩ default Rp that is ~0.22 V on CC, at the 0.2 V Ra
threshold. The rev B change list calls for one source of Rd.

**How to check.** On each CC net count resistors to ground and to a rail, plus IC pins whose function includes CC
termination (UCPD CCx, TCPC CC, port protectors with dead-battery Rd); more than one Rd or Rp that can be present
together is a finding.

**Fix.** One termination per pin: either the fixed resistor (and no PD) or the controller's own Rd, with the
dead-battery Rd of the controller or the port protector covering the unpowered case (STM32: DBCCx tied to CCx).

**References.** Type-C Table 4-27 note 2 and Table 4-28 note 2 (total resistance including internal and
parallel paths), Table 4-29 (Ra 800 Ω-1.2 kΩ); AN5225 §11.3.1 (DBCC1/DBCC2, dead-battery Rd on UCPD).

## `USB-8` CC resistors against the port role <a id="usb-8"></a>

🛑 error · needs: port role (sink, source, pass-through), CC terminations · [catalogue entry](../checks.md#usb-8)

**Impl.** mid · depends on NET-5, CFG-2, IMP-6

**What it catches.** Rp on a port that only sinks, Rd on a port that only sources, or, on a pass-through between a
receptacle and a plug, the termination on the wrong side.

**Why it matters.** CC termination is how the two ends decide who supplies VBUS and who is host. Rp on a sink port
makes it look like a source: facing a real source nothing attaches (two Rp), facing another device both wait for
the other to drive VBUS. Rd on a source port means it never sees an attach. On a pass-through each side faces a
different partner, so a termination meant for the far device must sit on the side that faces it.

**Example.** B153A's debug-accessory Rp (PA15 `CC_DA_EN` → Q201-Q203 → 36 kΩ from 3.3 V on both CC lines, the
default-current Rp) sits on the receptacle CC, which faces the user's computer. It was meant for the device under
test, on the plug. Enabled, it presents Rp/Rp to a host that itself presents Rp. The traces were cut on rev A;
rev B moves it to the plug.

**How to check.** Infer the role per port from the power structure (VBUS only sunk: sink; VBUS switched from a
rail: source) or take it from the project; flag Rp on sink-only ports and Rd on source-only ports. Pass-throughs:
compare the terminations on each connector with the role that connector faces.

**Fix.** Rd on sinks, Rp on sources (Table 4-27 values for the pull-up voltage used), a DRP controller where the
port must do both; on pass-throughs, terminate on the side of the partner that needs it.

**References.** Type-C §4.5.1.3 (CC functional models: source, sink, DRP), Table 4-9 (port interoperability),
Table 4-10 (source perspective, Rd/Rd = Debug Accessory), Table 4-27 (Rp values); Wikipedia, USB-C "Debug
Accessory Mode".

## `USB-9` CC1 and CC2 tied together on a receptacle <a id="usb-9"></a>

🛑 error · needs: CC pins of the receptacle · [catalogue entry](../checks.md#usb-9)

**Impl.** easy · depends on netlist

**What it catches.** A Type-C receptacle whose CC1 (A5) and CC2 (B5) are on the same net, usually to save a
resistor.

**Why it matters.** The source tells the orientation from which CC pin is terminated, and an e-marked or active
cable presents Ra on the other one (the VCONN pin of its plug). With both pins joined, the cable's Ra (800 Ω-1.2 kΩ)
lands in parallel with the sink's Rd: the source sees Ra/Ra or Ra plus too-low Rd and does not turn VBUS on. A
source that supplies VCONN puts it straight onto the sink's CC net. Orientation detection and PD are gone.

**Example.** The Raspberry Pi 4 (first revisions) shared one CC pull-down, R79, between both CC pins. With e-marked
cables (a MacBook charger's cable, for example) the charger saw Ra and never powered the Pi; with plain cables it
worked, which is why it shipped.

**How to check.** CC1 and CC2 pins of one receptacle on the same net, directly or through a 0 Ω tie.

**Fix.** A separate 5.1 kΩ to GND on each CC pin, or a controller with two CC pins.

**References.** Type-C §2.5 (VCONN on the CC pin not used by the cable), §4.5.1.1 (plug flip-ability), Table 4-10
(source perspective), Table 4-29 (Ra); Tyler Ward, "Pi4 not working with some chargers (or why you need two cc
resistors)" (2019),
[scorpia.co.uk](https://www.scorpia.co.uk/2019/06/28/pi4-not-working-with-some-chargers-or-why-you-need-two-cc-resistors/).

## `USB-10` Plug or captive cable with Rd on both CC pins <a id="usb-10"></a>

🛑 error · needs: Type-C plug (or captive cable) pins, resistors to ground · [catalogue entry](../checks.md#usb-10)

**Impl.** easy · depends on IMP-6

**What it catches.** A Type-C *plug* (a dongle, a thumb drive, a captive cable's end) with a 5.1 kΩ Rd on A5 and
another on B5.

**Why it matters.** A plug has only one CC contact, A5. B5 in a plug is VCONN, the supply for the electronics in a
cable or accessory. Rd on both pins of a plug shows the host Rd/Rd, which the spec reserves for Debug Accessory
Mode, so the host may enter debug mode instead of USB, or not attach as a sink at all.

**Example.** A USB-C thumb-drive design copied from a receptacle reference keeps "one 5K1 per CC": the plug now
shows Rd/Rd. B153A's plug J501 does it right: CC (A5) and VCONN (B5) go to UCPD2, and the firmware sets the role.

**How to check.** On parts whose symbol or footprint is a Type-C plug (or a captive cable entry in the project),
resistors to ground on both A5 and B5.

**Fix.** Rd on A5 only; B5 open or used as VCONN where the design is a VCONN-powered device.

**References.** Type-C §2.5 (VCONN), §3.4.3 (captive cable assembly), Table 4-10 (Rd/Rd = Debug Accessory Mode
attached), §3.5.3 item 1 ("Pin A5 (CC) of the USB Type-C plug shall be connected to GND through a resistor Rd")
for the legacy cable plugs.

## `USB-11` USB 2.0 wired to one orientation only <a id="usb-11"></a>

🛑 error · needs: D+/D− pins of the receptacle · [catalogue entry](../checks.md#usb-11)

**Impl.** easy · depends on netlist

**What it catches.** A Type-C receptacle where A6 (Dp1) and B6 (Dp2), or A7 (Dn1) and B7 (Dn2), are not on the same
net.

**Why it matters.** The plug carries D+/D− on one side only (B6/B7 "should not be present in the plug"), so which
receptacle row is live depends on how the plug is turned. With only one row wired, USB works in one orientation and
not after flipping the cable, the exact failure Type-C was meant to remove. The two rows are shorted on the board,
close to the receptacle, with a stub of no more than 3.5 mm.

**Example.** B153A's receptacle J401 joins both rows to the hub upstream port, as it should. Its plug J501 uses
A6/A7 only, which is correct for a plug: the check must look at receptacles, not plugs.

**How to check.** For each Type-C receptacle, A6 and B6 on one net, A7 and B7 on one net (directly, through 0 Ω, or
through a part that joins them by design, such as a mux).

**Fix.** Join the rows at the receptacle; route the stub short.

**References.** Type-C Table 3-4 note 1, Table 3-5 note 2 (USB 2.0 pairs, 3.5 mm stub), §4.5.1.1 (USB 2.0
flip-ability by shorting D+ and D− pins).

## `USB-12` Self-powered USB device without VBUS detection <a id="usb-12"></a>

⚠️ warning · needs: power states (`NET-8`), D+ pull-up source, VBUS sense pin · [catalogue entry](../checks.md#usb-12)

**Impl.** hard · depends on NET-8, NET-12

**What it catches.** A device that can be powered while VBUS is absent (battery, its own supply, another port) and
whose D+ (or D−) pull-up does not depend on VBUS: no VBUS sense input to the USB controller, or one tied high.

**Why it matters.** The spec forbids a device to power its pull-up unless VBUS is there, and requires it removed
within 10 s when VBUS goes. A self-powered device that keeps its pull-up on drives current into an unpowered host's
D+ (back-powering its PHY), confuses hubs about attach and detach, and never sees a disconnect, so it does not
re-enumerate when plugged back in.

**Example.** B153A's hub U301 has VBUS_DET tied high through R307, so it "always thinks the upstream is attached".
But the board runs from either side's VBUS through a diode-OR: a device under test that feeds its own VBUS powers
the board through D802, and once configured the hub can pull D+ up on the receptacle with no host VBUS present.

**How to check.** For each USB device port, in every power state where the device is on and the port's VBUS is
not: is the pull-up (internal to the PHY or external) still powered? The controller's VBUS-sense pin must be
connected to VBUS (through a divider, `USB-13`), not to a rail.

**Fix.** Wire VBUS through a divider to the VBUS-sense pin (STM32: PA9 or a GPIO, AN4879's 33 kΩ/82 kΩ for 5 V) and
let firmware or the PHY connect the pull-up only with VBUS present; for hubs, VBUS_DET from upstream VBUS through a
2:1 divider.

**References.** USB 2.0 §7.2.1 (no pull-up power unless VBUS is present, remove within 10 s), §7.1.5.1 (pull-up
voltage derived from or controlled by VBUS); AN4879 §2.6 (VBUS sensing mandatory for self-powered devices), §3.1.1;
USB251xB §3.3 (pin descriptions: VBUS_DET).

## `USB-13` VBUS straight into a sense pin above its rating <a id="usb-13"></a>

🛑 error · needs: VBUS range (PD or not), divider ratio, pin rating · [catalogue entry](../checks.md#usb-13)

**Impl.** mid · depends on NET-7, CFG-2, RPT-6

**What it catches.** VBUS reaching an MCU or controller pin with no divider, or with a divider sized for 5 V on a
port that can negotiate 9, 15 or 20 V.

**Why it matters.** Even a "5 V-tolerant" STM32 pin is only tolerant up to about VDD + 4 V *while powered*; with
the MCU off, 5 V on it exceeds the absolute maximum. A PD sink's VBUS goes to 20 V (21.5 V in transitions) once a
contract is made, and an overvoltage-protected switch upstream may only cut at 24 V. A divider that gives 3.6 V
at 5 V gives 14 V at 20 V.

**Example.** AN4879's recommended divider for 5 V VBUS sensing (33 kΩ to VBUS, 82 kΩ to GND, ratio 0.71) copied onto
a PD sink: 20 V gives 14.3 V on PA9. B153A's AN1 measures the receptacle VBUS through 10 kΩ / 1.62 kΩ (÷7.17):
2.8 V at 20 V, and still 3.35 V at the 24 V OVP threshold of its VBUS switch, right at the analog limit
(`ANA-20`).

**How to check.** Nets from VBUS to IC pins through resistors: compute the ratio (`RPT-6`), take the highest VBUS
from the project or from a PD controller on the port, compare with the pin's rating with the part unpowered.

**Fix.** A divider sized for the highest VBUS plus margin, with a series resistor large enough to limit the clamp
current with the MCU off; or a comparator or protector with a VBUS-rated input.

**References.** AN4879 §2.6 (PA9 at most VDD + 4 V, divider mandatory), §2.6.1 (33 kΩ/82 kΩ for VDD 3.0-3.6 V);
TPD6S300A §8.3.1 (20 V PD VBUS up to 21.5 V in transitions); AN5225 §3.1 (VBUS power options, 5-20 V).

## `USB-14` Hub straps and upstream VBUS detect not per the datasheet <a id="usb-14"></a>

🛑 error · needs: hub part type, strap pins, its datasheet rules · [catalogue entry](../checks.md#usb-14)

**Impl.** hard · depends on NET-7, CFG-2, CFG-5

**What it catches.** A USB hub whose configuration straps (CFG_SEL, NON_REM, LOCAL_PWR/SELF_PWR, port disables) or
upstream VBUS detect are wired against its datasheet or against what the board needs: ports marked removable that
are soldered down, self-powered straps on a bus-powered board, VBUS_DET tied to a rail on a detachable hub, a
configuration mode that waits for a host that is not there.

**Why it matters.** Hubs read their straps once at reset and then behave accordingly. Wrong power straps make the
hub report a power mode it does not have (ports limited to 100 mA, or over-current reports ignored); a wrong
NON_REM breaks the host's view of what is removable; a VBUS_DET tied high keeps the pull-up on without a host
(`USB-12`); an SMBus or EEPROM configuration strap with nothing to configure the hub leaves it silent forever.

**Example.** B153A's USB2512B/USB2514B is strapped for SMBus configuration (CFG_SEL0 = SCL high, CFG_SEL1 low): out
of reset it waits for the MCU to configure it over I2C. Its RESET_N has R304 10 kΩ to GND, so after any MCU reset
the hub is held off, and the STM32 ROM bootloader (USB DFU) behind it never reaches the host. VBUS_DET is tied
high by R307, although the datasheet asks for a 2:1 divider from upstream VBUS on a detachable hub.

**How to check.** Per hub family, the datasheet's strap table and pin rules (from the part library), checked
against the nets and against the project's intent (self or bus powered, which ports are on board).

**Fix.** Wire each strap per the datasheet for the intended mode; VBUS_DET from upstream VBUS through the 2:1
divider; for SMBus-configured hubs, make sure the configuring part is up whenever the hub is (`LOGIC-5`), or use
the default configuration with straps.

**References.** USB251xB §3.3 (pin descriptions; VBUS_DET: two 100 kΩ from upstream VBUS on a detachable
hub; NON_REM[1:0]), §3.3.1 (configuring the strap pins), Table 5-1 (CFG_SEL[1:0] configuration options); USB 2.0
§7.2.1.1, §7.2.1.2 (bus- and self-powered hubs); B153 rev B change list (hub RESET_N pull-up).

## `USB-15` SuperSpeed pairs without an orientation mux <a id="usb-15"></a>

🛑 error · needs: SuperSpeed pins of the receptacle, what they reach · [catalogue entry](../checks.md#usb-15)

**Impl.** mid · depends on NET-7, NET-10

**What it catches.** A USB 3 single-lane PHY wired to only one set of TX/RX pairs of a Type-C receptacle (TX1/RX1 or
TX2/RX2), with no mux and no PHY that handles both orientations.

**Why it matters.** Which TX/RX pair set is live depends on the plug orientation and the cable twist; the spec
requires "the functional equivalent of a switch" in both host and device to route the PHY to the connected pairs,
steered by which CC pin is terminated. Without one, the link is SuperSpeed in one orientation and falls back to
USB 2.0 (or fails) in the other.

**Example.** A USB 3 camera board routes its controller's single SuperSpeed lane to A2/A3 (TX1) and B11/B10 (RX1)
only. Flipped, the camera enumerates as a High-Speed device at a tenth of the bandwidth. A HD3SS3212 between the
PHY and both pair sets, its select pin driven by the CC controller's orientation output, fixes it.

**How to check.** On each Type-C receptacle, if the TX1/RX1 and TX2/RX2 pins are used, both sets must reach either
a mux (by part class) or a PHY whose part data says it handles both lanes.

**Fix.** A 2:1 SuperSpeed mux (HD3SS3212 class) steered by orientation, a controller with an integrated mux, or a
dual-lane PHY.

**References.** Type-C §4.5.1.1 (data bus interface and plug flip-ability, Figure 4-3), Table 3-4 (pin assignments);
TI, *HD3SS3212x Two-Channel Differential 2:1/1:2 USB3.1 Mux/Demux*, SLASE74F (2016), §10 (application),
[ti.com/lit/ds/symlink/hd3ss3212.pdf](https://www.ti.com/lit/ds/symlink/hd3ss3212.pdf).

## `USB-16` Type-C receptacle with VBUS or GND pins unconnected <a id="usb-16"></a>

🛑 error · needs: receptacle pins · [catalogue entry](../checks.md#usb-16)

**Impl.** easy · depends on netlist

**What it catches.** A Type-C receptacle with one of its VBUS pins (A4, A9, B4, B9) or ground pins (A1, A12, B1,
B12, and the shell where the footprint counts it) not on the VBUS or GND net.

**Why it matters.** The spec requires all VBUS pins bussed together and all ground pins bussed together at the
mounted receptacle. The cable carries the current through all of them; with half the contacts open, each one left
carries double, the 3 A or 5 A rating no longer holds, contacts heat, and the IR-drop budget is gone. A missing
ground on one row also breaks the return path next to the high-speed pairs of that row.

**Example.** A 16-pin USB 2.0 receptacle symbol with the B-row VBUS pins as separate pins that were never wired
(the KiCad ERC does not complain about unconnected passive pins): at 3 A the remaining two contacts each carry
1.5 A, and the receptacle gets hot.

**How to check.** All pins named VBUS on a Type-C connector on one net, all GND pins on one ground net.

**Fix.** Connect every VBUS and GND pin, with copper sized for the current.

**References.** Type-C Table 3-4 notes 2 and 3, Table 3-5 notes 4 and 5 (all VBUS and all ground pins bussed
together), §4.4.1 (IR drop).

## `USB-17` Micro-B or Mini-B ID pin floating on an OTG port, or wired on a device-only port <a id="usb-17"></a>

💡 suggestion · needs: ID pin, port role · [catalogue entry](../checks.md#usb-17)

**Impl.** mid · depends on CFG-2, NET-12

**What it catches.** The ID pin of a Micro-AB/Mini-AB receptacle left open on a port meant to be On-The-Go, or the ID
pin of a device-only port wired to something.

**Why it matters.** The ID pin is how an OTG controller learns which plug is in: a Micro-A plug ties ID to ground
(< 10 Ω, become host), a Micro-B plug leaves it open or above 100 kΩ (stay device). If the receptacle's ID pin is not
wired to the controller's ID input (with its pull-up), the board never becomes host with an OTG adapter. On a
device-only port the pin has no function; wiring it to ground or to a GPIO only suggests a feature that is not
there.

**Example.** An STM32F4 board with a Micro-AB receptacle and OTG_FS in the firmware plans, ID pin left
unconnected: an OTG cable to a USB stick does nothing.

**How to check.** ID pins of Micro/Mini receptacles: open while the project or the MCU pin function (OTG_FS_ID)
says OTG, or connected while the port is device-only.

**Fix.** OTG: ID to the controller's ID input with its pull-up, and a VBUS switch for host mode; device-only: leave
it open and use a Micro-B receptacle.

**References.** Micro-USB §4.2, Table 4-2 (ID: Micro-A plug to GND, Micro-B open or > 100 kΩ, Ra_PLUG_ID 10 Ω,
Rb_PLUG_ID 100 kΩ); AN4879 §3.3 (OTG applications through embedded PHY); Wikipedia, USB hardware "USB On-The-Go
connectors".

## `USB-18` CC or SBU protection that clamps in range or adds too much capacitance <a id="usb-18"></a>

⚠️ warning · needs: protection part data (clamp, standoff, capacitance), capacitors on CC · [catalogue entry](../checks.md#usb-18)

**Impl.** mid · depends on NET-7, IMP-6

**What it catches.** A TVS or clamp on CC or SBU whose standoff is below the voltages the pin legitimately sees (CC
carries up to 5.5 V as VCONN, a PD VBUS short needs ≥ 24 V standoff to survive), or a CC line whose total
capacitance (controller, protector, external capacitor, TVS) falls outside the 200-600 pF of cReceiver.

**Why it matters.** A 5 V TVS on CC conducts on a 5.5 V VCONN and in a short to a 20 V VBUS it carries the whole
supply until it fails short; a deep-snapback diode triggered by the ringing stays on. On the capacitance side, PD's
BMC signalling is specified for 200-600 pF on CC: too much rounds the edges into errors, too little lets noise in.
Both mistakes come from adding "one more ESD part" next to a port protector.

**Example.** B153A has 330 pF on each CC in front of the DPO2036 and UCPD1. Copying ST's AN5225 recommendation of
390 pF next to it, or adding a 30 pF TVS array, takes the line past 600 pF together with the protector and the MCU
pin. A USB-C board using an ESDA6V1-class 6 V TVS on CC survives on a 5 V charger and dies the first time a 20 V
VBUS shorts to CC.

**How to check.** For each CC/SBU protection part, its standoff and clamp voltage from the library against 5.5 V and
24 V; sum the capacitances on each CC net from the part data and capacitor values against 200-600 pF.

**Fix.** CC and SBU protection rated for 24 V standoff (or a port protector that does both), no deep-snapback parts;
size the CC capacitor so the total stays inside 200-600 pF.

**References.** TPD6S300A §9.2.2.3 (CC line capacitance, cReceiver 200-600 pF from USB PD §5.8.6), §9.2.2.4 (24 V
standoff, no deep-snapback diodes); AN5225 §14.1 (ESDA25L for 20 V, ESDA6V1 only for 5 V), §14.2 (capacitors on CC
lines); Type-C Table 4-5 (VCONN 3.0-5.5 V), Table 4-40 (vCC-Clamp ≥ 2.9 V).

## `USB-19` Type-C receptacle variant without the pins the design needs <a id="usb-19"></a>

⚠️ warning · needs: receptacle variant (footprint, pin count), what the design uses · [catalogue entry](../checks.md#usb-19)

**Impl.** mid · depends on CFG-2, NET-7

**What it catches.** A reduced Type-C receptacle (6-pin power-only, 12- or 16-pin USB 2.0) on a port the design
uses for more: SBU for an alternate mode or a sideband, TX/RX pairs for USB 3, D+/D− on a power-only part.

**Why it matters.** Reduced receptacles are popular because they are cheaper and easier to solder, but the
missing contacts are simply not there: a signal routed to them in the schematic ends on a pin the footprint lacks
or on a pad nothing mates with. It shows up only when the alternate mode or the SuperSpeed link never comes up.

**Example.** A power-only receptacle (GCT USB4125 class, VBUS, GND, CC1, CC2) chosen for a board that also needs
USB 2.0 for firmware updates; or a 16-pin USB 2.0 receptacle (GCT USB4105 class) on a board that routes a
DisplayPort alternate mode. B153A uses its receptacle's SBU pins for SWD, which a 16-pin USB 2.0 receptacle still
has.

**How to check.** From the footprint or the part library, which contacts exist; compare with the nets the design
puts on that connector (SBU, SuperSpeed pins, D+/D−) and with the project's list of interfaces on the port.

**Fix.** A receptacle variant with the needed contacts (24-pin full-featured for USB 3 or alternate modes).

**References.** Type-C Table 3-4 (full 24-pin assignment), Table 3-5 (USB 2.0-only support, unused contacts not
connected); GCT, *USB4105 - USB 2.0 Connector Type C Horizontal Receptacle* [gct.co/connector/usb4105](https://gct.co/connector/usb4105)
and *USB4125 - USB Charging Connector Type C Horizontal Receptacle* [gct.co/connector/usb4125](https://gct.co/connector/usb4125)
(fetched through `tpm fetch`); Wikipedia, USB-C "Receptacles".
