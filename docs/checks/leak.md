# LEAK checks, explained

Detail page for the `LEAK` category of the [check catalogue](../checks.md#-leak---leakage-and-low-power). Each
section explains one check: what it catches, why the board fails without it, a concrete example, what better_erc
needs to find it, the usual fix, and where to read more. The one-line entries in the catalogue stay the source of
truth for ID, severity and wording; this page is the commentary.

These checks matter most on battery and energy-harvesting boards, where a sleep budget of a few microamps is
easily eaten by one resistor. On mains or USB-powered boards most findings are reminders, not faults.

References used throughout:

- **AN4899** · ST, *Guidelines for GPIO hardware settings and low-power consumption on STM32 MCUs*, AN4899 Rev 4
  (May 2026; fetched 21 Oct 2026 through `tpm fetch`, st.com refuses plain HTTP clients).
  [st.com/resource/en/application_note/an4899](https://www.st.com/resource/en/application_note/an4899-stm32-microcontroller-gpio-configuration-for-hardware-settings-and-lowpower-consumption-stmicroelectronics.pdf)
- **Microchip analog TB** · Microchip, *Analog Applications in 8-bit PIC and AVR Microcontrollers* (technical brief,
  online docs), section 5 "Measure Power Supplies and Batteries".
  [onlinedocs.microchip.com](https://onlinedocs.microchip.com/oxy/GUID-2B17BE2C-35EB-43A9-ACC2-59322EFEA913-en-US-2/GUID-E57B23FD-FCDD-40C0-8F12-B0F2EB6DBBCA.html)
- **SLAA689** · TI, *ESD Diode Current Specification*, SLAA689B (2015, rev. 2021).
  [ti.com/lit/pdf/slaa689](https://www.ti.com/lit/pdf/slaa689)
- **RM0008** · ST, *STM32F101xx, STM32F102xx, STM32F103xx, STM32F105xx and STM32F107xx reference manual*, RM0008
  Rev 21 (2021; fetched through `tpm fetch`). [st.com/resource/en/reference_manual/rm0008](https://www.st.com/resource/en/reference_manual/rm0008-stm32f101xx-stm32f102xx-stm32f103xx-stm32f105xx-and-stm32f107xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
- **SCBA004** · TI, *Implications of Slow or Floating CMOS Inputs*, SCBA004E (1994, rev. 2021).
  [ti.com/lit/pdf/scba004](https://www.ti.com/lit/pdf/scba004)
- **Wikipedia** · *Voltage divider*, *LED circuit*: readable overviews.
  [en.wikipedia.org/wiki/Voltage_divider](https://en.wikipedia.org/wiki/Voltage_divider),
  [en.wikipedia.org/wiki/LED_circuit](https://en.wikipedia.org/wiki/LED_circuit)

## `LEAK-1` Resistor divider straight on a power rail <a id="leak-1"></a>

⚠️ warning · needs: the power structure, resistor values · [catalogue entry](../checks.md#leak-1)

**Impl.** easy · depends on NET-5, IMP-6

**What it catches.** Two or more resistors in series from a rail to ground with nothing switching them: a voltage
divider for a reference, a sense input or a feedback node that draws current in every power state.

**Why it matters.** A divider is a load. V / (R1 + R2) flows as long as the rail is up: 1.65 mA for 1 kΩ + 1 kΩ on 3.3
V, which is more than the whole sleep current of most MCUs. Low values are often chosen for a stiff reference without
asking whether the load needs it. Regulator feedback dividers are excluded (they are part of the regulator and sized by
its datasheet).

**Example.** On B153A the INA181's REF comes from R603/R604, 1 kΩ / 1 kΩ off 3.3 V: 1.65 mA forever. On a bus-powered
dongle that is harmless; on a battery board the same circuit would be the largest consumer in sleep. The REF input
current of the INA181 decides how high the divider can go.

**How to check.** Chains of resistors from a rail to ground (`NET-5`, `NET-2`) with no switch in the chain; compute the
current from the values (`IMP-6`) and report it, sorted, against a project budget.

**Fix.** Raise the values to what the load allows (with a capacitor at the tap if the load is dynamic), use a reference
or buffer, or switch the divider (`LEAK-7`).

**References.** Wikipedia, Voltage divider; Microchip analog TB §5 (Figure 5-1 "Simple Voltage Divider Circuit", Figure
5-2 "Voltage Divider with On and Off"); B153A review notes §3 (INA181 REF divider).

## `LEAK-2` Pins that cannot be high in some power states <a id="leak-2"></a>

💡 suggestion · needs: power states (`NET-8`), the domain of each pin and net · [catalogue entry](../checks.md#leak-2)

**Impl.** hard · depends on NET-8, NET-7

**What it catches.** IO pins whose net leads into another power domain that can be off while the pin's own domain is on;
the pin must be low or high-impedance whenever that other domain is off. The check lists them all, also when the
backfeed warning (`BKF-2`) for that net has been waived.

**Why it matters.** Every such pin is a rule for firmware: "drive low before turning off the sensor rail". Break it and
the pin powers the dead domain through its input clamps, which is both a leak and a backfeed. The rule lives in nobody's
head after the designer moves on, so the list is the deliverable.

**Example.** An MCU with SPI to a flash on a switched `+3V3_MEM` rail: in sleep the rail is off, so SCK, MOSI and CS
must all be low (CS is often left high "to deselect", which powers the flash through its CS pin clamp and costs hundreds
of microamps).

**How to check.** For each power state (`NET-8`), nets that join an on domain to an off one; list the on-side pins with
the level they must have.

**Fix.** A firmware rule per pin, documented in the pin map; or isolate the signals with Ioff buffers (`BKF-1`).

**References.** AN4899 §6.2 "Cross-voltage domains leakage" (Figure 15); SLAA689 §1 "Important Side Effects of the ESD
Diodes During Power Cycling"; catalogue `BKF-2`.

## `LEAK-3` NC pins: set them to analog or pull them <a id="leak-3"></a>

💡 suggestion · needs: which pins are GPIOs (pin roles), the MCU's reset state · [catalogue entry](../checks.md#leak-3)

**Impl.** mid · depends on NET-7

**What it catches.** Unconnected GPIO pins of MCUs, FPGAs and IO expanders, listed with a note on what the firmware (or
the board) must do with them.

**Why it matters.** A floating digital input sits somewhere between the thresholds, both input transistors conduct a
little, and the pin draws current and may toggle with noise. Some MCUs reset their pins to analog mode (input buffer
off, no leak), others to floating input, where every unused pin leaks until firmware changes it. Which one decides
whether the board needs firmware care or nothing.

**Example.** An STM32F103 whose pins come out of reset as floating inputs: twenty unused pins left so in a low-power
product add measurable current in Stop mode. The STM32G0 on B153A resets its pins to analog, so its spares cost nothing
until firmware touches them.

**How to check.** Unconnected pins of IO role (`NET-7`) per part, with the part's reset state from the part data.

**Fix.** Configure unused pins as analog (or output low) in firmware; for parts without an analog mode, enable an
internal pull or tie them through a resistor.

**References.** AN4899 §6.1 "Avoid floating unused pin", §7.1 "Configure unused GPIO input as analog input"; RM0008
§9.1.1 (STM32F1 pins in Input Floating mode during and after reset); SCBA004 §1.

## `LEAK-4` Divider or pull through a normally closed switch <a id="leak-4"></a>

⚠️ warning · needs: switch contact type (NC / NO), the power structure · [catalogue entry](../checks.md#leak-4)

**Impl.** mid · depends on NET-5, NET-7

**What it catches.** A pull-up or divider whose current path closes through a normally closed contact (switch, button,
reed sensor, interlock), so current flows in the rest state.

**Why it matters.** A pull-up against a normally open button costs nothing until someone presses it. Against a normally
closed one it costs V / R all the time the device sits there, which is nearly always. The mistake comes from picking an
NC contact for "fail-safe" detection without moving the sense resistor.

**Example.** A lid interlock (NC reed switch, closed while the lid is shut) to GND with a 10 kΩ pull-up to 3.3 V: 330 µA
for as long as the lid is closed, about 8 mAh a day, in a product meant to sleep for a year on a 2000 mAh cell (it lasts
eight months instead).

**How to check.** Resistors from a rail whose chain to ground passes a switch with an NC contact (part data or pin names
NC/COM).

**Fix.** Use the NO contact and invert the logic, raise the pull-up to 1 MΩ class, or sample the switch with a switched
pull-up.

**References.** AN4899 §5.1.3 "GPIO current consumption" (static current is mainly pull-ups on pins held low).

## `LEAK-5` Indicator LED permanently on a battery rail <a id="leak-5"></a>

💡 suggestion · needs: which rails are battery rails · [catalogue entry](../checks.md#leak-5)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** An LED with its resistor between a battery-powered rail and ground, with no switch or GPIO in the
path: a power-on indicator that is on whenever the battery is.

**Why it matters.** An indicator is typically 1-5 mA, three orders of magnitude above a sleep budget. Bench boards need
it; battery products lose days or weeks of standby to it. A GPIO-driven LED that blinks briefly gives the same
information for a fraction of the charge.

**Example.** A green LED with 1 kΩ on a Li-ion `VBAT` (3.7 V, Vf 2.1 V): 1.6 mA, 38 mAh a day, a 1000 mAh cell empty in
under four weeks with the device "off".

**How to check.** LEDs whose anode chain reaches a battery rail (project setting, a battery symbol or connector, or
`NET-5` naming) and whose cathode chain reaches ground without a switch or IO pin.

**Fix.** Drive it from a GPIO, put it after the power switch, or use a jumper so only bench units have it.

**References.** Wikipedia, LED circuit "Series resistor"; catalogue `DRV-17`.

## `LEAK-6` Pull-up on a line held low in sleep <a id="leak-6"></a>

💡 suggestion · needs: pin levels per power state (`NET-8`), pulls · [catalogue entry](../checks.md#leak-6)

**Impl.** hard · depends on NET-8, NET-7

**What it catches.** A pull-up on a line that some part holds low in a low-power state (or a pull-down on a line held
high): the resistor carries V / R for the whole sleep.

**Why it matters.** Pull resistors are sized for the active state and forgotten for sleep. A 10 kΩ pull-up on an enable
that the MCU holds low while asleep is 330 µA, ten times the sleep current of the MCU itself.

**Example.** A sensor's open-drain interrupt output with a 10 kΩ pull-up to 3.3 V, where the sensor asserts the
interrupt (low) until read, and the MCU sleeps before reading it: 330 µA through the night. The internal pull-up of the
MCU (~40 kΩ) or a 100 kΩ external one would do at that speed.

**How to check.** For each power state (`NET-8`), lines with a pull whose level in that state (driven by the MCU's sleep
configuration or a device's known state, `NET-7`) is the opposite of the pull.

**Fix.** Raise the value, pull the other way, or change the firmware so the line rests at the pull's level.

**References.** AN4899 §5.1.3 "GPIO current consumption", §7.4 "Configure GPIO when entering low-power modes".

## `LEAK-7` Battery measurement divider always connected <a id="leak-7"></a>

💡 suggestion · needs: which rails are battery rails, divider to an ADC · [catalogue entry](../checks.md#leak-7)

**Impl.** mid · depends on NET-5, CFG-2

**What it catches.** A divider from a battery rail to ground whose tap goes to an ADC, with no switch: the
battery-measurement form of `LEAK-1`.

**Why it matters.** The battery is measured a few times an hour but the divider drains it all the time. 100 kΩ + 100 kΩ
on a 4.2 V cell is 21 µA, often more than everything else in sleep together.

**Example.** A CR2032 (≈ 220 mAh) board measuring its own cell through 100 kΩ / 100 kΩ: 15 µA at 3 V, about 1.7 years of
the cell spent on the divider alone, before the MCU does anything.

**How to check.** Dividers from a battery rail (`NET-5`, `CFG-2`) whose tap reaches an ADC-capable pin, with no switch
in the chain.

**Fix.** Switch the top of the divider with a PMOS (driven by a small NMOS from a GPIO), or the bottom with an NMOS
where the ADC input can tolerate the full voltage when off; or go to 1-10 MΩ with a capacitor at the tap sized for the
ADC's sampling charge.

**References.** Microchip analog TB §5: Figure 5-2 (a switched divider; the low-side switch only works for supplies at
or below the MCU supply, hence a high-side switch for a battery above it), the capacitor across the lower resistor for
ADC acquisition; catalogue `LEAK-1`.
