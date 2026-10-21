# BAT checks, explained

Detail page for the `BAT` category of the [check catalogue](../checks.md#-bat---batteries-and-chargers). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc
needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of
truth for ID, severity and wording; this page is the commentary. The `Impl.` line says how hard the check is to
implement in better_erc and which [FEATURES.md](../../FEATURES.md) items it waits for.

References used throughout:

- **BQ2407x** · TI, *BQ2407x Standalone 1-Cell 1.5-A Linear Battery Chargers with Power Path* datasheet, SLUS810N
  (2021): power path, TS input. [ti.com/lit/ds/symlink/bq24075](https://www.ti.com/lit/ds/symlink/bq24075.pdf)
- **BQ297xx** · TI, *BQ297xx Cost-Effective Voltage and Current Protection Integrated Circuit for Single-Cell
  Li-Ion and Li-Polymer Batteries* datasheet, SLUSBU9I (2024).
  [ti.com/lit/ds/symlink/bq2970](https://www.ti.com/lit/ds/symlink/bq2970.pdf)
- **MCP73831** · Microchip, *MCP73831/2 Miniature Single-Cell, Fully Integrated Li-Ion, Li-Polymer Charge
  Management Controllers* datasheet, DS20001984H.
  [microchip.com](https://ww1.microchip.com/downloads/en/DeviceDoc/MCP73831-Family-Data-Sheet-DS20001984H.pdf)
- **AN1149** · Microchip, *Designing A Li-Ion Battery Charger and Load Sharing System With Microchip's Stand-Alone
  Li-Ion Battery Charge Management Controller* (2008).
  [microchip.com](https://ww1.microchip.com/downloads/en/AppNotes/01149c.pdf)
- **SLUA376** · TI, *Implementations of Battery Charger and Power-Path Management System Using bq2410x/11x/12x*
  (2006). [ti.com/lit/an/slua376](https://www.ti.com/lit/an/slua376/slua376.pdf)
- **bq27441-G1** · TI, *bq27441-G1 System-Side Impedance Track Fuel Gauge* datasheet, SLUSBH1C (2014).
  [ti.com/lit/ds/symlink/bq27441-g1](https://www.ti.com/lit/ds/symlink/bq27441-g1.pdf)
- **CR2032** · Energizer, *CR2032 Lithium Coin* product datasheet ("Max Rev Charge: 1 microampere").
  [data.energizer.com](https://data.energizer.com/pdfs/cr2032.pdf)
- **Maxell CR warnings** · Maxell, *Warnings: CR Coin Type Lithium Manganese Dioxide Battery*: diodes and a
  protective resistor against charging in backup circuits.
  [maxell.com.tw](https://maxell.com.tw/assets/download/Coin-Type%20Lithium%20Manganese%20Dioxide%20Battery/CR_Warnings_12e.pdf)
- **SLVA769A** · TI, *Understanding Undervoltage Lockout in Power Devices* (2016, rev. 2018).
  [ti.com/lit/an/slva769](https://www.ti.com/lit/an/slva769/slva769.pdf)
- **Adafruit** · Adafruit Learning System, *Li-Ion & LiPoly Batteries*, pages "Protection Circuitry" and
  "Proper Charging". [learn.adafruit.com](https://learn.adafruit.com/li-ion-and-lipoly-batteries)
- **Wikipedia** · readable overviews: [Lithium-ion battery](https://en.wikipedia.org/wiki/Lithium-ion_battery),
  [Battery management system](https://en.wikipedia.org/wiki/Battery_management_system),
  [Button cell](https://en.wikipedia.org/wiki/Button_cell).

## `BAT-1` Rechargeable lithium cell without protection <a id="bat-1"></a>

🛑 error · needs: battery parts and their chemistry, protection ICs and FETs on the cell, the pack's own protection (project) · [catalogue entry](../checks.md#bat-1)

**Impl.** mid · depends on NET-7, CFG-2

**What it catches.** A rechargeable lithium cell (Li-ion, LiPo, LiFePO4) on the board or on a battery connector with
no protection against overcharge, over-discharge and overcurrent: no protector IC with its FETs on the board, and
no statement that the pack carries one.

**Why it matters.** A charger limits the charge voltage only while it works and only while it is the source. A
shorted load, a charger fault or a load that drains the cell to zero is not stopped by anything: a shorted lithium
cell delivers tens of amperes and can go into thermal runaway, and a cell discharged far below its cut-off is
damaged and unsafe to charge again.

**Example.** A raw 18650 cell (bare, no protection board) on a board with an MCP73831 charger: the charger stops at
4.2 V, but nothing opens the circuit when a downstream regulator fails short, and nothing stops the system from
draining the cell to 0 V in storage. A BQ2970 with a dual NMOS in the cell's negative lead covers all three.

**How to check.** Find battery parts (`BT` designator, battery connector class) and their chemistry from the MPN
or the project; look for a part the library classes as a cell protector (`NET-7`) between the cell and the system.
A pack with its own protection is a project fact (`CFG-2`): without it, report.

**Fix.** Use protected packs and say so in the project, or put a protector (BQ297xx class: overcharge,
over-discharge, charge and discharge overcurrent, short circuit) with its two FETs on the board.

**References.** BQ297xx §8.4 (normal, overcharge, over-discharge and overcurrent states), §9.2 (typical
application); Adafruit, "Protection Circuitry" (raw cells are not protected); Wikipedia, Battery management system.

## `BAT-2` Charger thermistor pin floating or tied so it disables charging <a id="bat-2"></a>

⚠️ warning · needs: charger pin roles (TS, NTC, THERM), the documented way to disable the function · [catalogue entry](../checks.md#bat-2)

**Impl.** mid · depends on NET-7

**What it catches.** A charger's temperature-sense input (TS, NTC, THERM) that is left unconnected, tied to
ground or to a rail, instead of going to a thermistor in the pack or to the fixed resistor the datasheet gives for
"no thermistor".

**Why it matters.** The charger reads the TS voltage against a window: too high reads as cold, too low as hot, and
either suspends charging. A floating pin is pulled out of the window by the charger's own bias current, so the
board never charges; tied to ground it reads as a permanently hot pack. Each family documents one specific way to
disable the check, and it differs between parts.

**Example.** A BQ24075 with TS tied to ground "because there is no thermistor": TS reads below the hot threshold
and charging stays suspended, while CHG keeps showing "charging". The datasheet asks for a 10 kΩ resistor from TS
to VSS, or a 103AT-2 type 10 kΩ NTC in the pack.

**How to check.** The pin with role TS of a part classed as charger (`NET-7`): flag it unconnected, on a rail or
on ground with no resistor, unless the part's data says that is its documented disable option; compare a fixed
resistor with the value the part data gives.

**Fix.** Wire the pack's thermistor, or the fixed resistor (or divider) the datasheet documents for "TS not used".

**References.** BQ2407x §9.3.6 (battery pack temperature monitoring, 10 kΩ from TS to VSS when not used),
§10.2.2.3 (TS function), §7 (pin functions, TS).

## `BAT-3` Charger without power path while the system load sits on the battery <a id="bat-3"></a>

⚠️ warning · needs: charger class (power path or not), what is on the battery net · [catalogue entry](../checks.md#bat-3)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A linear or switching charger without power path whose battery pin also feeds the system
load directly, so charge current and load current share the same pin.

**Why it matters.** The charger ends the charge when its current falls below the termination threshold. With the
load on the battery it measures charge plus load current, which never falls that low: the cell is held at 4.2 V
indefinitely (shorter life), or the charger terminates, the load pulls the cell down, and it starts again in a
loop. The safety timer may also expire on a cell that was never full.

**Example.** An MCP73831 with the MCU's LDO on its VBAT pin, the MCU drawing 30 mA while the charger's
termination is set at a few tens of milliamperes: the charge never terminates. Microchip's AN1149 recommends a
PMOS and diode that move the load to the input while it is present; a BQ24075 with power path does it internally.

**How to check.** A part classed as charger (`NET-7`) without the power-path attribute; the net on its battery pin
(`NET-5`) has loads other than the cell, its divider and its gauge.

**Fix.** A charger with power path (BQ2407x class), a load-sharing PMOS and Schottky (AN1149), or a charger
without automatic termination for designs that accept the trade-off.

**References.** AN1149 "Connecting the system load to the battery" (the charge may never end); SLUA376 §3
(design considerations of a load on the battery); BQ2407x §9.3.4 (dynamic power path management).

## `BAT-4` Primary cell with a path that can charge it <a id="bat-4"></a>

🛑 error · needs: battery parts and their chemistry, the paths from rails or chargers to the cell · [catalogue entry](../checks.md#bat-4)

**Impl.** mid · depends on NET-5, NET-10, CFG-2

**What it catches.** A primary (non-rechargeable) cell, a lithium coin cell or alkaline cells, connected so that
current can flow into it: wired straight to a rail it shares with another supply, diode-ORed with only the rail's
diode, or on a charger.

**Why it matters.** Primary cells are not built to take charge current. A lithium coin cell may take on the order
of a microampere of reverse current; more generates gas inside the cell, which can then leak, swell or burst. The
fault only shows after weeks of operation, in the field.

**Example.** An RTC backup with a CR2032 whose positive terminal joins `+3V3` through one diode on the 3.3 V side
only: whenever the board is on, 3.3 V drives current into the 3.0 V cell. Energizer's CR2032 sheet allows 1 µA of
reverse charge; Maxell's warnings ask for a diode against charging plus a protective resistor that limits the
current if the diode fails. An MCU backup pin with its own internal power switch (STM32 VBAT) is a different case:
there the part does the switching.

**How to check.** Find battery parts and their chemistry (MPN or project, `CFG-2`); follow every path from the
cell's positive terminal to a rail or a charger (`NET-5`, through resistors and diodes: `NET-10`); each path must
have a diode blocking current into the cell, or end at a pin the library marks as a backup-supply input.

**Fix.** A low-leakage diode (silicon or low-reverse-current Schottky) in series with the cell plus a protective
resistor, or a part with a dedicated backup input; never a charger on a primary cell.

**References.** CR2032 datasheet (Max Rev Charge 1 µA); Maxell CR warnings ("Never charge", diodes and protective
resistor in memory and RTC backup); Wikipedia, Button cell.

## `BAT-5` Battery connector polarity vs the common convention <a id="bat-5"></a>

💡 suggestion · needs: battery connector family, which pin carries the positive terminal · [catalogue entry](../checks.md#bat-5)

**Impl.** judgement · depends on netlist

**What it catches.** A battery connector whose pin assignment differs from the convention most packs for that
connector use: the positive pin of a JST PH battery socket, the centre of a barrel jack.

**Why it matters.** There is no standard for which pin of a JST PH battery plug is positive, and both versions are
sold. A pack plugged into a board with the other convention reverses the battery on the board: without reverse
protection (`PIN-1`) the charger and the regulator die, and a lithium cell is shorted through their diodes.

**Example.** A board whose 2-pin JST PH socket has pin 1 as positive, to match a hobby pack bought online; the
LiPo packs from Adafruit, SparkFun and Particle use the other assignment (looking at the pack's plug with the
key up and the holes towards you, positive is on the right). The first customer with an Adafruit pack sees smoke.

**How to check.** A connector with a battery role and a known family (library class); compare its positive pin
with the project's chosen convention. better_erc can list battery connectors and their polarity for review; there
is no rule to decide it.

**Fix.** Choose the convention of the packs you will ship with and write it on the silkscreen next to the
connector, and add reverse protection in any case (`PIN-1`).

**References.** Particle community, *Why isn't battery polarity always the same in JST pins?* (2018: no standard;
Particle, SparkFun and Adafruit use the same polarity)
[community.particle.io](https://community.particle.io/t/why-isnt-battery-polarity-always-the-same-in-jst-pins/44474);
DigiKey TechForum, *Polarity standard for JST PH connectors on LiPo batteries* (2026)
[forum.digikey.com](https://forum.digikey.com/t/polarity-standard-for-jst-ph-connectors-on-lipo-batteries/64548);
Adafruit, Li-Ion & LiPoly Batteries ("Do NOT install the battery in reverse polarity").

## `BAT-6` Regulator or boost running from a battery without undervoltage lockout <a id="bat-6"></a>

⚠️ warning · needs: regulators on the battery net, their UVLO thresholds, the cell's cut-off voltage · [catalogue entry](../checks.md#bat-6)

**Impl.** mid · depends on NET-5, NET-7, CFG-2

**What it catches.** A regulator or boost converter fed from a battery whose undervoltage lockout (internal, by an
EN divider, or by a protector) is below the voltage at which the cell must stop discharging, or absent.

**Why it matters.** A boost converter keeps running as the cell falls, drawing more and more current to hold its
output, until the cell is at its own internal limit. A lithium cell taken below its cut-off (about 2.5-3.0 V per
cell) loses capacity and can become unsafe to recharge; an unprotected cell stored with such a load reaches 0 V.
Even with a pack protector, the protector's cut-off is the last line, not a run limit.

**Example.** A boost from one Li-ion cell to 5 V whose own UVLO is around 1.8 V (fine for two alkaline cells, the
part's other use): on a raw cell it runs the cell down to 1.8 V. A divider on EN setting a 3.2 V start and a 3.0 V
stop keeps the cell inside its range.

**How to check.** Regulators on a battery net (`NET-5`); their UVLO from part data or an EN divider (`NET-7`); the
cell's cut-off from its chemistry or the project (`CFG-2`). Flag a threshold below the cut-off.

**Fix.** An EN-pin divider with hysteresis, a supervisor or a load switch with UVLO, or a protector whose
over-discharge threshold is set at the run limit.

**References.** SLVA769A §1 (UVLO in battery-powered applications), §3 (hysteresis); BQ297xx §8.4.3
(over-discharge status); `PIN-16` for sagging inputs in general.

## `BAT-7` Fuel gauge sense resistor not in the leg the gauge expects <a id="bat-7"></a>

💡 suggestion · needs: gauge pin roles (SRP, SRN), which leg each gauge senses, the resistor's position · [catalogue entry](../checks.md#bat-7)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A coulomb-counting fuel gauge whose sense resistor sits in the battery's negative return when
the gauge measures in the positive lead, or the other way round, or whose sense inputs are swapped.

**Why it matters.** The gauge's sense inputs have a common-mode range tied to one leg: a high-side gauge expects
SRP and SRN near the cell voltage, a low-side gauge near its own ground. On the wrong leg the inputs sit outside
the conditions the gauge is specified and calibrated for, the reported current cannot be trusted, and every
state-of-charge figure derived from it drifts. Swapped inputs invert charge and discharge.

**Example.** A bq27441-G1 (high side: a 10 mΩ, 1 % resistor between the pack positive and VSYS, SRP on the pack
side) laid out from a low-side gauge's reference with the resistor in the ground return: SRP and SRN sit near
0 V instead of at the pack voltage, and the gauge's current and state of charge are not to be trusted.

**How to check.** The part classed as fuel gauge with roles SRP/SRN (`NET-7`) and its sensing side from part
data; the resistor between those pins must be in series with the battery's positive or negative lead (`NET-5`),
matching it, with SRP on the side the datasheet names.

**Fix.** Move the sense resistor to the leg the gauge expects and Kelvin-connect SRP/SRN as the datasheet shows.

**References.** bq27441-G1 §7 (pin functions: SRN, SRP, high-side current path), §10.2.2.3 (sense resistor
selection).
