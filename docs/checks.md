# Better ERC - check catalogue

What better_erc should catch on a schematic or netlist: the checks only, no implementation status. Status of the
code (which checks run, parsers, reports, UI) lives in [FEATURES.md](../FEATURES.md) and refers to these IDs. The
condensed [schematic review checklist](review-checklist.md) links back here by ID.

- **IDs** (`PIN-1`, `I2C-3`) are stable: never renumber or reuse. A new check takes the next free number of its
  category. A hidden comment (`<!-- CHK-59 -->`) on some entries keeps the number from an earlier internal tracker.
- **One check, one concern.** A check names one thing that is wrong, so a finding, a waiver, a test schematic and a
  line of code each map to exactly one ID. An entry that grew several concerns is split: the old ID keeps its first
  concern (text narrowed, never changed in meaning), the others get new numbers, and a `<!-- split ... -->` comment
  on the old entry names them.
- **Severity** of the finding: 🛑 error (board won't work or gets damaged) · ⚠️ warning (risk or margin, depends
  on the design) · 💡 suggestion (debug aids, tidiness, conventions).
- `≈` marks overlaps: merge when implementing.
- A miss found at assembly or bring-up becomes a new check here, so the list grows from real failures. Entries
  born that way carry the board in a comment (`<!-- B153A -->`); the full story is in that board's change list.
- **Detail pages** under [docs/checks/](checks/), one per category, explain each check: what it catches, why the board
  fails, an example, how to detect it and what data that needs, the fix, verified references. The 📖 at the end of an
  index line links to its section.
- **Implementation tier** (`impl:` on each line, from the detail page): how hard the check is to implement in pure
  Rust in better_erc, with no LLM. `easy`: the netlist alone (connectivity, pin types, part classes and values), a
  graph query · `mid`: part data must be built or read first (pin roles, MPN decoding, thresholds, addresses,
  packages) or a project setting is needed · `hard`: analysis the code lacks (power states, signal following through
  FETs and translators, per-family datasheet rules, layout geometry) · `judgement`: not a rule, a human or an LLM
  reads intent or a datasheet's prose, better_erc can only list candidates. In parentheses the FEATURES.md ids it
  depends on (`NET-7` pin model, `NET-8` power states, `CFG-2` project data, ...), or `netlist` when none.
- A check that needs data the netlist does not carry (part parameters, pin functions, power states) still belongs
  here: the code reports it as not run for lack of data (`DIAG-2`), and the review checklist covers it by hand.

## Power

### 🛡️ PIN - Power input and output protection
- <a id="pin-1"></a>⚠️ `PIN-1` No reverse polarity protection on a power input · impl: mid (NET-5, NET-7) · [📖](checks/pin.md#pin-1) <!-- CHK-59; split 21 Oct 2026: PIN-4, PIN-5, PIN-6 -->
- <a id="pin-2"></a>⚠️ `PIN-2` Fast blow fuse where inrush is expected (bulk capacitance, motors, hot-plug): nuisance trips · impl: mid (NET-5, RPT-1) · [📖](checks/pin.md#pin-2) <!-- CHK-136; split 21 Oct 2026: PIN-7 -->
- <a id="pin-3"></a>⚠️ `PIN-3` Power outputs on connectors from nets used on board: suggest an eFuse or switch with short and overload protection · impl: easy (NET-5) · [📖](checks/pin.md#pin-3) <!-- CHK-111 -->
- <a id="pin-4"></a>⚠️ `PIN-4` No overcurrent protection (fuse, polyfuse, eFuse) on a power input · impl: easy (NET-5) · [📖](checks/pin.md#pin-4) <!-- from PIN-1 -->
- <a id="pin-5"></a>⚠️ `PIN-5` No overvoltage protection (TVS, clamp, OVP switch) on a power input · impl: mid (NET-5, NET-7) · [📖](checks/pin.md#pin-5) <!-- from PIN-1 -->
- <a id="pin-6"></a>💡 `PIN-6` No input filter (bulk capacitor, ferrite or LC) on a power input · impl: easy (NET-5) · [📖](checks/pin.md#pin-6) <!-- from PIN-1 -->
- <a id="pin-7"></a>⚠️ `PIN-7` Fuse rating vs the expected current: holds the worst-case normal load, blows on the fault it is there for · impl: hard (NET-8, RPT-2, CFG-2) · [📖](checks/pin.md#pin-7) <!-- from PIN-2; B135B -->
- <a id="pin-8"></a>🛑 `PIN-8` TVS standoff voltage below the input's operating maximum: it conducts in normal operation and burns · impl: mid (NET-5, CFG-2) · [📖](checks/pin.md#pin-8) <!-- 21 Oct 2026 -->
- <a id="pin-9"></a>⚠️ `PIN-9` TVS clamping voltage above the absolute maximum of what it protects (the regulator or part behind it) · impl: mid (NET-5, NET-7) · [📖](checks/pin.md#pin-9) <!-- 21 Oct 2026 -->
- <a id="pin-10"></a>⚠️ `PIN-10` Polyfuse hold current below the load, or its resistance in series with a supply that needs to be accurate · impl: mid (CFG-2, RPT-2) · [📖](checks/pin.md#pin-10) <!-- 21 Oct 2026 -->
- <a id="pin-11"></a>⚠️ `PIN-11` Fuse or polyfuse rated above the connector, cable or trace it protects · impl: mid (CFG-2) · [📖](checks/pin.md#pin-11) <!-- 21 Oct 2026 -->
- <a id="pin-12"></a>🛑 `PIN-12` Two power sources on one net (battery and USB, two inputs, two connectors) without ORing diodes, an ideal-diode controller or a switch · impl: mid (NET-5, NET-7) · [📖](checks/pin.md#pin-12) <!-- 21 Oct 2026 -->
- <a id="pin-13"></a>💡 `PIN-13` Reverse protection by a series diode on an input over about 1 A: suggest a PMOS ideal diode (dissipation, dropout) · impl: mid (CFG-2, RPT-2) · [📖](checks/pin.md#pin-13) <!-- 21 Oct 2026 -->
- <a id="pin-14"></a>⚠️ `PIN-14` No bulk capacitance where a supply arrives through a long cable or is hot-plugged: inductive overshoot on connection can exceed ratings · impl: mid (CFG-2, RPT-1) · [📖](checks/pin.md#pin-14) <!-- 21 Oct 2026 -->
- <a id="pin-15"></a>⚠️ `PIN-15` Fuse voltage rating below the supply voltage (32 V DC fuses on a 48 V input) · impl: mid (NET-5, CFG-2) · [📖](checks/pin.md#pin-15) <!-- 21 Oct 2026 -->
- <a id="pin-16"></a>⚠️ `PIN-16` No undervoltage lockout on a power input whose source can sag (battery, long cable): regulators restart in a loop and loads see a brown-out · impl: mid (CFG-2, NET-7) · [📖](checks/pin.md#pin-16) <!-- 21 Oct 2026 -->
- <a id="pin-17"></a>🛑 `PIN-17` PMOS reverse-polarity switch with |Vin| above its Vgs(max) and no gate zener or divider, or a standard-threshold FET on an input of 5 V or less that never turns fully on ≈ DRV-10 · impl: mid (NET-5, NET-7) · [📖](checks/pin.md#pin-17) <!-- 21 Oct 2026 -->

### 🔋 RAIL - Rails and regulators
- <a id="rail-1"></a>🛑 `RAIL-1` Power input pins of parts not powered · impl: easy (NET-5) · [📖](checks/rail.md#rail-1) <!-- CHK-69 -->
- <a id="rail-2"></a>🛑 `RAIL-2` DC-DC or LDO ref voltage known: find the divider, calculate output, compare with net name or specified voltage ≈ RAIL-3 · impl: mid (NET-5, NET-7, RPT-6) · [📖](checks/rail.md#rail-2) <!-- CHK-87 -->
- <a id="rail-3"></a>⚠️ `RAIL-3` Net name carries a voltage (e.g. `_3V3`) that is wrong · impl: mid (NET-5, RPT-6) · [📖](checks/rail.md#rail-3) <!-- CHK-142 -->
- <a id="rail-4"></a>🛑 `RAIL-4` Fixed-output regulator whose MPN voltage differs from the net it feeds (a 3.3 V TLV70233 on `+1V8_LDO`) ≈ RAIL-2 · impl: mid (NET-5, NET-7) · [📖](checks/rail.md#rail-4) <!-- 21 Oct 2026; B153A, B200A -->
- <a id="rail-5"></a>🛑 `RAIL-5` Power net with loads but no source: no regulator output, connector, switch, ferrite or 0R from a sourced net (`+5V0_FLT` with nothing feeding it) ≈ RAIL-1 · impl: easy (NET-5) · [📖](checks/rail.md#rail-5) <!-- 21 Oct 2026; B200A -->
- <a id="rail-6"></a>⚠️ `RAIL-6` Current-limit resistor (ILIM, ISET, RLIM) vs the limit the design states: recompute from the datasheet formula · impl: hard (NET-7, CFG-2) · [📖](checks/rail.md#rail-6) <!-- 21 Oct 2026; B153A -->
- <a id="rail-7"></a>⚠️ `RAIL-7` Feedback divider impedance outside the datasheet range: too high for the FB bias current and noise, too low for the quiescent budget · impl: mid (NET-7, RPT-6) · [📖](checks/rail.md#rail-7) <!-- 21 Oct 2026 -->
- <a id="rail-8"></a>⚠️ `RAIL-8` LDO output capacitor not what the part needs for stability: value below the minimum, ceramic on a part that needs ESR, or the other way round · impl: mid (NET-7, RPT-1) · [📖](checks/rail.md#rail-8) <!-- 21 Oct 2026 -->
- <a id="rail-9"></a>🛑 `RAIL-9` LDO headroom below its dropout at load (3.3 V from a 3.6 V battery through a 0.5 V dropout part) · impl: mid (NET-5, NET-7, CFG-2, RPT-5) · [📖](checks/rail.md#rail-9) <!-- 21 Oct 2026 -->
- <a id="rail-10"></a>🛑 `RAIL-10` Switching regulator bootstrap (BST, BOOT) capacitor missing or far from the datasheet value · impl: mid (NET-7) · [📖](checks/rail.md#rail-10) <!-- 21 Oct 2026 -->
- <a id="rail-11"></a>⚠️ `RAIL-11` Feedback divider sensing the wrong node: before a ferrite, filter or switch, so the regulated voltage is not the one the load sees · impl: mid (NET-5, NET-7) · [📖](checks/rail.md#rail-11) <!-- 21 Oct 2026 -->
- <a id="rail-12"></a>⚠️ `RAIL-12` Conversion ratio vs switching frequency: minimum on-time or maximum duty cycle violated (24 V to 1 V at 2 MHz, 3.3 V from 3.6 V on a buck without 100 % mode) · impl: mid (NET-7, CFG-2) · [📖](checks/rail.md#rail-12) <!-- 21 Oct 2026 -->
- <a id="rail-13"></a>⚠️ `RAIL-13` Boost converter output not isolated when disabled: the input reaches the output through the inductor and diode, so "off" is Vin · impl: hard (NET-6, NET-7, NET-8) · [📖](checks/rail.md#rail-13) <!-- 21 Oct 2026 -->
- <a id="rail-14"></a>⚠️ `RAIL-14` PGOOD / PG open-drain output without a pull-up, or pulled to a rail that is not up when it is read · impl: mid (NET-7, NET-8) · [📖](checks/rail.md#rail-14) <!-- 21 Oct 2026 -->
- <a id="rail-15"></a>🛑 `RAIL-15` Enable pin tied or driven above its maximum (EN rated 6 V tied to a 12 V Vin), or driven from a rail below its high threshold · impl: mid (NET-5, NET-7) · [📖](checks/rail.md#rail-15) <!-- 21 Oct 2026 -->
- <a id="rail-16"></a>🛑 `RAIL-16` Soft-start, compensation or timing pins (SS, COMP, RT, CT) without the parts the datasheet requires · impl: mid (NET-7) · [📖](checks/rail.md#rail-16) <!-- 21 Oct 2026 -->
- <a id="rail-17"></a>🛑 `RAIL-17` Regulator configuration pins (MODE, SYNC, FREQ, FPWM, VSEL) floating where the datasheet says tie them · impl: mid (NET-7) · [📖](checks/rail.md#rail-17) <!-- 21 Oct 2026 -->
- <a id="rail-18"></a>🛑 `RAIL-18` Two regulator outputs tied together without a part that allows it (parallel LDOs fight, one sinks) · impl: easy (NET-5) · [📖](checks/rail.md#rail-18) <!-- 21 Oct 2026 -->
- <a id="rail-19"></a>💡 `RAIL-19` Analog supply (VDDA, AVDD, VREF+) fed straight from the digital rail without a filter · impl: easy (NET-5) · [📖](checks/rail.md#rail-19) <!-- 21 Oct 2026 -->
- <a id="rail-20"></a>⚠️ `RAIL-20` Reference input (VREF+, REF, VREFIN) without its own decoupling, or sourced from a noisy rail · impl: mid (NET-5, NET-7, RPT-1) · [📖](checks/rail.md#rail-20) <!-- 21 Oct 2026 -->
- <a id="rail-21"></a>⚠️ `RAIL-21` Total output capacitance of a regulator outside its stable range after ceramic DC-bias derating (a 10 µF 0603 at 5 V is about 4 µF), or more than its soft-start and current limit can charge · impl: mid (NET-7, RPT-1) · [📖](checks/rail.md#rail-21) <!-- 21 Oct 2026 -->
- <a id="rail-22"></a>⚠️ `RAIL-22` Inductor value outside the datasheet range for the switching frequency and current · impl: mid (NET-7, RPT-3) · [📖](checks/rail.md#rail-22) <!-- 21 Oct 2026 -->
- <a id="rail-23"></a>⚠️ `RAIL-23` Regulator symbol shared by orderable siblings with different VFB or Vout (0.6 / 0.8 V, 1.8 / 3.3 V): the divider or the net was computed for the other sibling, the MPN decides ≈ RAIL-2, RAIL-4 · impl: mid (NET-5, NET-7, RPT-6) · [📖](checks/rail.md#rail-23) <!-- 21 Oct 2026 -->
- <a id="rail-24"></a>🛑 `RAIL-24` Regulator EN pin floating on a part without an internal pull: never starts, or starts at random · impl: mid (NET-7, NET-9) · [📖](checks/rail.md#rail-24) <!-- 21 Oct 2026 -->
- <a id="rail-25"></a>⚠️ `RAIL-25` Input range below the regulator's minimum Vin or UVLO at the low end (a 4.5 V-minimum buck on a 3-cell battery) ≈ RATE-12 · impl: mid (NET-5, NET-7, CFG-2) · [📖](checks/rail.md#rail-25) <!-- 21 Oct 2026 -->
- <a id="rail-26"></a>⚠️ `RAIL-26` Asynchronous converter rectifying through a slow diode or a MOSFET body diode instead of a Schottky: losses and reverse recovery · impl: mid (NET-6, NET-7) · [📖](checks/rail.md#rail-26) <!-- 21 Oct 2026 -->

### 🧱 DEC - Decoupling and datasheet passives
- <a id="dec-1"></a>⚠️ `DEC-1` No decoupling capacitors on power nets · impl: easy (NET-5) · [📖](checks/dec.md#dec-1) <!-- CHK-26 -->
- <a id="dec-2"></a>💡 `DEC-2` Recommended decoupling values · impl: mid (NET-7, RPT-1) · [📖](checks/dec.md#dec-2) <!-- CHK-184 -->
- <a id="dec-3"></a>🛑 `DEC-3` STM32 VCAP pins need 2.2 µF ceramic caps · impl: mid (NET-7) · [📖](checks/dec.md#dec-3) <!-- CHK-196 -->
- <a id="dec-4"></a>💡 `DEC-4` Caution against ferrite beads without measurements · impl: easy (NET-5) · [📖](checks/dec.md#dec-4) <!-- CHK-82 -->
- <a id="dec-5"></a>🛑 `DEC-5` Datasheet-specified filter or bypass capacitors (PLL, CRFILT, VCORE, VCAP, VDDA filter) missing or at a different value than the datasheet gives · impl: mid (NET-7) · [📖](checks/dec.md#dec-5) <!-- 21 Oct 2026; B153A C306/C310 -->
- <a id="dec-6"></a>⚠️ `DEC-6` Bulk capacitance on a rail below what its regulator or load needs (sum per rail, derated) · impl: mid (RPT-1, NET-5, CFG-2) · [📖](checks/dec.md#dec-6) <!-- 21 Oct 2026 -->
- <a id="dec-7"></a>⚠️ `DEC-7` Fewer decoupling capacitors than power pins on a part that wants one per pin · impl: easy (NET-5) · [📖](checks/dec.md#dec-7) <!-- 21 Oct 2026 -->
- <a id="dec-8"></a>⚠️ `DEC-8` Decoupling only on the source side of a ferrite, filter or series resistor: nothing at the pin it serves · impl: easy (NET-5) · [📖](checks/dec.md#dec-8) <!-- 21 Oct 2026 -->
- <a id="dec-9"></a>⚠️ `DEC-9` Ferrite bead in series with a high-current rail: saturates, and its DC resistance drops the rail · impl: mid (CFG-2, RPT-2) · [📖](checks/dec.md#dec-9) <!-- 21 Oct 2026 -->
- <a id="dec-10"></a>🛑 `DEC-10` Bypass capacitor on the IC's power pin but not on the rail: a two-pin net of one passive and one power pin passes every ERC and powers nothing · impl: easy (netlist) · [📖](checks/dec.md#dec-10) <!-- 21 Oct 2026 -->
- <a id="dec-11"></a>💡 `DEC-11` Parallel ceramics a decade apart (10 µF with 10 nF): anti-resonance between them; prefer multiples of one value or a graded set · impl: easy (IMP-6, NET-5) · [📖](checks/dec.md#dec-11) <!-- 21 Oct 2026 -->
- <a id="dec-12"></a>🛑 `DEC-12` Polarized capacitor (electrolytic, tantalum) with its negative terminal on the higher-potential net · impl: easy (NET-5) · [📖](checks/dec.md#dec-12) <!-- 21 Oct 2026 -->
- <a id="dec-13"></a>⚠️ `DEC-13` Datasheet-specified bias or set resistors (RBIAS, ISET, RZQ, RREF, EXTRES) missing, off value, or looser than the tolerance asked (240 Ω 1 %, 4.87k 1 %) ≈ DEC-5 · impl: mid (NET-7) · [📖](checks/dec.md#dec-13) <!-- 21 Oct 2026 -->

### ⏯️ SEQ - Sequencing, enables, inrush, discharge
- <a id="seq-1"></a>⚠️ `SEQ-1` Power sequencing: intermittent problems on power on or off · impl: judgement (NET-8) · [📖](checks/seq.md#seq-1) <!-- CHK-135 -->
- <a id="seq-2"></a>🛑 `SEQ-2` Feedback or bootstrap loops, e.g. EN set by a divider powered by the device's own output, never starts · impl: mid (NET-7, NET-5) · [📖](checks/seq.md#seq-2) <!-- CHK-199 -->
- <a id="seq-3"></a>⚠️ `SEQ-3` SD card insertion inrush: no power switch or resistor; decoupling after that switch or resistor ≈ SEQ-6 · impl: mid (NET-7, NET-5) · [📖](checks/seq.md#seq-3) <!-- CHK-76 -->
- <a id="seq-4"></a>⚠️ `SEQ-4` Soft start circuits with a MOSFET or resistor as load; if load is specified, report performance · impl: hard (NET-10, CFG-2) · [📖](checks/seq.md#seq-4) <!-- CHK-131 -->
- <a id="seq-5"></a>⚠️ `SEQ-5` High voltage net without self-discharge resistors · impl: easy (NET-5) · [📖](checks/seq.md#seq-5) <!-- CHK-137 -->
- <a id="seq-6"></a>⚠️ `SEQ-6` Power switch turning hard on into a capacitive load (a downstream board, a hot-plug port): the inrush dips the source and resets the board's own parts. Slow turn-on, fast turn-off (gate RC with a diode, or a load switch with soft start) · impl: hard (NET-10, CFG-2) · [📖](checks/seq.md#seq-6) <!-- 21 Oct 2026; B153A VBUS_SW -->
- <a id="seq-7"></a>⚠️ `SEQ-7` Multi-rail part (SoC, FPGA, DDR) whose datasheet orders its supplies, with nothing on the board enforcing the order · impl: hard (NET-8, NET-7) · [📖](checks/seq.md#seq-7) <!-- 21 Oct 2026 -->
- <a id="seq-8"></a>⚠️ `SEQ-8` Reset of a multi-rail part released by an RC alone, not gated by its rails being valid: no supervisor or PGOOD chain · impl: mid (NET-7) · [📖](checks/seq.md#seq-8) <!-- 21 Oct 2026 -->
- <a id="seq-9"></a>💡 `SEQ-9` Output capacitors not discharged at power-off for a part that needs a full power-on reset: a quick off-on leaves it half powered · impl: judgement (NET-8) · [📖](checks/seq.md#seq-9) <!-- 21 Oct 2026 -->

### 🌐 GND - Grounds and isolation
- <a id="gnd-1"></a>⚠️ `GND-1` Split grounds without an isolated circuit: warn, point to books and pages · impl: mid (NET-5, NET-7) · [📖](checks/gnd.md#gnd-1) <!-- CHK-60 -->
- <a id="gnd-2"></a>💡 `GND-2` Suggest a Y cap between isolated grounds · impl: mid (NET-5, NET-7) · [📖](checks/gnd.md#gnd-2) <!-- CHK-92 -->
- <a id="gnd-3"></a>⚠️ `GND-3` Analog and digital grounds joined at more than one point, or not joined at all · impl: easy (NET-5) · [📖](checks/gnd.md#gnd-3) <!-- 21 Oct 2026 -->
- <a id="gnd-4"></a>🛑 `GND-4` Net crossing an isolation barrier: a label or power symbol of one side used on the other (`GND` on the isolated side) · impl: mid (NET-7, NET-5) · [📖](checks/gnd.md#gnd-4) <!-- 21 Oct 2026 -->
- <a id="gnd-5"></a>🛑 `GND-5` Non-isolated part or structure spanning the barrier: a pull-up, capacitor, resistor or protection diode between the two sides, a debug header ground, a shield or a mounting hole bonded on both · impl: mid (NET-7, NET-5) · [📖](checks/gnd.md#gnd-5) <!-- 21 Oct 2026 -->
- <a id="gnd-6"></a>💡 `GND-6` Isolation rating of the isolators, transformers and Y capacitors vs the voltage the barrier must hold · impl: mid (CFG-2, NET-7) · [📖](checks/gnd.md#gnd-6) <!-- 21 Oct 2026 -->
- <a id="gnd-7"></a>⚠️ `GND-7` Chassis or shield ground tied straight to signal ground, or left floating, without a decision (R, C, or direct) ≈ USB-4 · impl: easy (NET-5) · [📖](checks/gnd.md#gnd-7) <!-- 21 Oct 2026 -->

### 🎚️ DRV - MOSFETs, switches and drivers
- <a id="drv-1"></a>🛑 `DRV-1` MOSFET gate threshold vs driving level · impl: mid (NET-7, NET-5) · [📖](checks/drv.md#drv-1) <!-- CHK-30 -->
- <a id="drv-2"></a>🛑 `DRV-2` High side MOSFETs driven incorrectly · impl: mid (NET-5, NET-7) · [📖](checks/drv.md#drv-2) <!-- CHK-126 -->
- <a id="drv-3"></a>⚠️ `DRV-3` PWM from MCU or SoC pins driving transistors directly: suggest a driver to protect the pin · impl: mid (NET-7, NET-12) · [📖](checks/drv.md#drv-3) <!-- CHK-61 -->
- <a id="drv-4"></a>⚠️ `DRV-4` Several gates on one driver: check charge pump capacity · impl: mid (NET-7, CFG-5) · [📖](checks/drv.md#drv-4) <!-- CHK-132 -->
- <a id="drv-5"></a>⚠️ `DRV-5` MOSFET as inverter: gate resistor to stop Cgd pulling the output below ground · impl: easy (netlist) · [📖](checks/drv.md#drv-5) <!-- CHK-215 -->

    Glitches possible on turn off, Miller capacitance, rM

- <a id="drv-6"></a>🛑 `DRV-6` Q-Q chain not back to back, e.g. pogo-pin VBUS protection; double check thresholds · impl: easy (netlist (FET pin names, channel type from the symbol)) · [📖](checks/drv.md#drv-6) <!-- CHK-178 -->
- <a id="drv-7"></a>⚠️ `DRV-7` No snubbers on half bridge output · impl: easy (NET-6) · [📖](checks/drv.md#drv-7) <!-- CHK-127 -->
- <a id="drv-8"></a>🛑 `DRV-8` Flyback without feedback and no resistive or zener load on output · impl: mid (NET-5, CFG-5) · [📖](checks/drv.md#drv-8) <!-- CHK-140 -->
- <a id="drv-9"></a>🛑 `DRV-9` Relays without flyback diode · impl: easy (netlist) · [📖](checks/drv.md#drv-9) <!-- CHK-103 -->
- <a id="drv-10"></a>🛑 `DRV-10` Gate drive beyond Vgs(max): a high-side PMOS on a 24 V rail with its gate pulled to ground sees −24 V on a ±20 V gate · impl: mid (NET-5, NET-7) · [📖](checks/drv.md#drv-10) <!-- 21 Oct 2026 -->
- <a id="drv-11"></a>🛑 `DRV-11` MOSFET body diode conducting in the off state: source and drain swapped, or a load switch oriented so the load is fed through the diode · impl: mid (NET-5, NET-7) · [📖](checks/drv.md#drv-11) <!-- 21 Oct 2026 -->
- <a id="drv-12"></a>🛑 `DRV-12` BJT base driven from a logic pin without a base resistor · impl: easy (netlist) · [📖](checks/drv.md#drv-12) <!-- 21 Oct 2026 -->
- <a id="drv-13"></a>⚠️ `DRV-13` Low-side switch under a load whose signals are ground-referenced elsewhere: the load's ground floats when off · impl: mid (NET-5, NET-7) · [📖](checks/drv.md#drv-13) <!-- 21 Oct 2026 -->
- <a id="drv-14"></a>⚠️ `DRV-14` Inductive load (solenoid, motor, coil, transformer) switched without a freewheel diode, TVS or snubber ≈ DRV-9 · impl: easy (netlist (CFG-2 for loads behind connectors)) · [📖](checks/drv.md#drv-14) <!-- 21 Oct 2026 -->
- <a id="drv-15"></a>⚠️ `DRV-15` Half bridge driven by one signal and its inverter with no dead time: both transistors on during the edge · impl: hard (NET-6, NET-10, RPT-14) · [📖](checks/drv.md#drv-15) <!-- 21 Oct 2026 -->
- <a id="drv-16"></a>🛑 `DRV-16` Gate driver bootstrap diode or capacitor missing · impl: mid (NET-7, CFG-5) · [📖](checks/drv.md#drv-16) <!-- 21 Oct 2026 -->
- <a id="drv-17"></a>🛑 `DRV-17` LED driven from a GPIO or a rail without a series resistor or current limit · impl: easy (NET-5) · [📖](checks/drv.md#drv-17) <!-- 21 Oct 2026 -->
- <a id="drv-18"></a>🛑 `DRV-18` LED forward voltage above its rail (a white or blue LED on 1.8 V, two in series on 3.3 V): never lights · impl: mid (NET-5, CFG-5) · [📖](checks/drv.md#drv-18) <!-- 21 Oct 2026 -->
- <a id="drv-19"></a>💡 `DRV-19` No gate resistor between a gate driver and a MOSFET: ringing and EMI · impl: mid (NET-7) · [📖](checks/drv.md#drv-19) <!-- 21 Oct 2026 -->
- <a id="drv-20"></a>⚠️ `DRV-20` Rds(on) only specified at a gate voltage above the drive available (a 10 V-rated FET driven at 3.3 V) ≈ DRV-1 · impl: mid (NET-7, NET-5) · [📖](checks/drv.md#drv-20) <!-- 21 Oct 2026 -->

### 🔋 BAT - Batteries and chargers
- <a id="bat-1"></a>🛑 `BAT-1` Rechargeable lithium cell without protection (overvoltage, undervoltage, overcurrent), on the pack or the board · impl: mid (NET-7, CFG-2) · [📖](checks/bat.md#bat-1) <!-- 21 Oct 2026 -->
- <a id="bat-2"></a>⚠️ `BAT-2` Charger thermistor pin (TS, NTC) floating or tied in a way that disables charging, instead of a thermistor or the documented fixed divider · impl: mid (NET-7) · [📖](checks/bat.md#bat-2) <!-- 21 Oct 2026 -->
- <a id="bat-3"></a>⚠️ `BAT-3` Charger without power path while the system load sits on the battery: the load defeats termination and the cell never finishes charging · impl: mid (NET-5, NET-7) · [📖](checks/bat.md#bat-3) <!-- 21 Oct 2026 -->
- <a id="bat-4"></a>🛑 `BAT-4` Primary cell (coin cell, alkaline) with a path that can charge it (a diode-less connection to a rail, a charger) · impl: mid (NET-5, NET-10, CFG-2) · [📖](checks/bat.md#bat-4) <!-- 21 Oct 2026 -->
- <a id="bat-5"></a>💡 `BAT-5` Battery connector polarity vs the common convention for that connector (JST PH packs, barrel jacks) · impl: judgement (netlist) · [📖](checks/bat.md#bat-5) <!-- 21 Oct 2026 -->
- <a id="bat-6"></a>⚠️ `BAT-6` Regulator or boost running from a battery without undervoltage lockout: drains the cell to zero · impl: mid (NET-5, NET-7, CFG-2) · [📖](checks/bat.md#bat-6) <!-- 21 Oct 2026 -->
- <a id="bat-7"></a>💡 `BAT-7` Fuel gauge sense resistor not in the leg the gauge expects (low side vs high side) · impl: mid (NET-5, NET-7) · [📖](checks/bat.md#bat-7) <!-- 21 Oct 2026 -->

## Shorts, leakage and wiring

### 💥 SHORT - Shorts and net collisions
- <a id="short-1"></a>🛑 `SHORT-1` Net shunting a known part (R, C, L, transistor...): most likely a net label collision, or a port and net label with the same name ≈ SHORT-2 · impl: easy (netlist) · [📖](checks/short.md#short-1) <!-- CHK-1 -->
- <a id="short-2"></a>🛑 `SHORT-2` Accidental net name collision, e.g. on I2C: VCCa-R-R-VCCa/b, or two 0R leading to the same net · impl: easy (netlist (NET-2 chains)) · [📖](checks/short.md#short-2) <!-- CHK-158 -->
- <a id="short-3"></a>🛑 `SHORT-3` Non-DNM part shunting another mounted part · impl: easy (netlist (DNP flags per variant)) · [📖](checks/short.md#short-3) <!-- CHK-152 -->
- <a id="short-4"></a>🛑 `SHORT-4` Short circuit between power and ground through low value resistor(s) · impl: easy (NET-5, IMP-6) · [📖](checks/short.md#short-4) <!-- CHK-5; split 21 Oct 2026: SHORT-9, SHORT-10 -->
- <a id="short-5"></a>🛑 `SHORT-5` VDD/VCC pins shorted to GND, or GND/VEE to power nets · impl: easy (NET-5) · [📖](checks/short.md#short-5) <!-- CHK-179 -->
- <a id="short-6"></a>🛑 `SHORT-6` Inputs shorted to ground or power without a resistor · impl: easy (netlist (NET-7 to tell configurable pins)) · [📖](checks/short.md#short-6) <!-- CHK-189 -->
- <a id="short-7"></a>⚠️ `SHORT-7` Shorted IOs: warn that they must be driven together · impl: easy (netlist) · [📖](checks/short.md#short-7) <!-- CHK-160 -->
- <a id="short-8"></a>⚠️ `SHORT-8` Direct connections to battery and other high energy nets · impl: mid (NET-5, CFG-2) · [📖](checks/short.md#short-8) <!-- CHK-12 -->
- <a id="short-9"></a>🛑 `SHORT-9` Power to ground through a forward-biased diode or a MOSFET body diode · impl: mid (NET-5, NET-7) · [📖](checks/short.md#short-9) <!-- from SHORT-4 -->
- <a id="short-10"></a>🛑 `SHORT-10` Power to ground through a TVS or zener whose standoff voltage is below the net voltage · impl: mid (NET-5, CFG-2) · [📖](checks/short.md#short-10) <!-- from SHORT-4 -->
- <a id="short-11"></a>🛑 `SHORT-11` Two push-pull outputs on one net (output-output conflict) · impl: easy (netlist (NET-7 for pins whose type the symbol gets wrong)) · [📖](checks/short.md#short-11) <!-- 21 Oct 2026 -->
- <a id="short-12"></a>🛑 `SHORT-12` Differential pair with P tied to N: a label typo or a wrong bus entry · impl: easy (netlist) · [📖](checks/short.md#short-12) <!-- 21 Oct 2026 -->

### 🧵 WIRE - Connectivity hygiene
- <a id="wire-1"></a>⚠️ `WIRE-1` Net with a single pin (a label with no partner, a typo in one of two labels) · impl: easy (netlist) · [📖](checks/wire.md#wire-1) <!-- 21 Oct 2026 -->
- <a id="wire-2"></a>🛑 `WIRE-2` Net names that differ only in case, separator or suffix (`RESET_N` and `RESET_n`, `3V3` and `+3V3`): almost always one net meant · impl: easy (netlist) · [📖](checks/wire.md#wire-2) <!-- 21 Oct 2026 -->
- <a id="wire-3"></a>⚠️ `WIRE-3` Label not attached to a wire or pin (dangling, offset by a grid step) · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-3) <!-- 21 Oct 2026 -->
- <a id="wire-4"></a>🛑 `WIRE-4` Hierarchical sheet pin without a matching label inside the sheet, or unconnected on the parent · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-4) <!-- 21 Oct 2026 -->
- <a id="wire-5"></a>🛑 `WIRE-5` Power symbol and a signal label with the same name: the signal is silently on the rail · impl: mid (TOOL-3, NET-5) · [📖](checks/wire.md#wire-5) <!-- 21 Oct 2026 -->
- <a id="wire-6"></a>⚠️ `WIRE-6` Global label used on one sheet only, where a local label was meant (or the other end is misspelled) · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-6) <!-- 21 Oct 2026 -->
- <a id="wire-7"></a>🛑 `WIRE-7` No-connect flag on a pin that also has a wire · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-7) <!-- 21 Oct 2026 -->
- <a id="wire-8"></a>⚠️ `WIRE-8` Unconnected pin without a no-connect flag: not decided, or forgotten · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-8) <!-- 21 Oct 2026 -->
- <a id="wire-9"></a>🛑 `WIRE-9` No-connect flag on a pin the datasheet says must be tied (an unused input, a mode pin, an exposed pad) · impl: mid (NET-7, TOOL-3) · [📖](checks/wire.md#wire-9) <!-- 21 Oct 2026 -->
- <a id="wire-10"></a>⚠️ `WIRE-10` Hidden power pins on a symbol connected implicitly by name: the connection is not visible on the sheet · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-10) <!-- 21 Oct 2026 -->
- <a id="wire-11"></a>💡 `WIRE-11` Unnamed (auto-named) net with many pins or crossing sheets: give it a name · impl: easy (netlist) · [📖](checks/wire.md#wire-11) <!-- 21 Oct 2026 -->
- <a id="wire-12"></a>🛑 `WIRE-12` Same label text in different scopes (a local and a global label, a sheet pin and a global label): joined in one tool, separate in another, and the designer assumes one of the two · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-12) <!-- 21 Oct 2026; KiCad sameLocalGlobalLabel -->
- <a id="wire-13"></a>⚠️ `WIRE-13` Output pin whose net reaches no input (an output into nothing but passives or a test point) · impl: easy (netlist (NET-10 to look through series resistors)) · [📖](checks/wire.md#wire-13) <!-- 21 Oct 2026 -->
- <a id="wire-14"></a>🛑 `WIRE-14` Ground-named pin (GND, VSS, AGND, PGND) on a net that is not a ground · impl: easy (NET-5) · [📖](checks/wire.md#wire-14) <!-- 21 Oct 2026; KiCad groundPinNotGround -->
- <a id="wire-15"></a>💡 `WIRE-15` Four-way wire junction, or a label placed over several wires: the connection is ambiguous to a reader · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-15) <!-- 21 Oct 2026 -->
- <a id="wire-16"></a>🛑 `WIRE-16` Bus mismatch: a member net not in the bus it joins, bus widths that differ, indices out of range · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-16) <!-- 21 Oct 2026 -->
- <a id="wire-17"></a>💡 `WIRE-17` ERC or DRC rules silenced in the project file: list them in the report, they hide findings · impl: mid (TOOL-3) · [📖](checks/wire.md#wire-17) <!-- 21 Oct 2026 -->

### ↩️ BKF - Backfeed between power domains
- <a id="bkf-1"></a>🛑 `BKF-1` Power backfeed through IO, suggest Ioff capable buffers · impl: hard (NET-8, NET-7) · [📖](checks/bkf.md#bkf-1) <!-- CHK-2 -->
- <a id="bkf-2"></a>🛑 `BKF-2` General case: two power domains, IO across, one domain goes off, IO powers it through protection diodes ≈ BKF-1 · impl: hard (NET-8, NET-7) · [📖](checks/bkf.md#bkf-2) <!-- CHK-3 -->
- <a id="bkf-3"></a>🛑 `BKF-3` Same through a MOSFET body diode, e.g. in an I2C voltage translator · impl: hard (NET-8, NET-4, NET-10) · [📖](checks/bkf.md#bkf-3) <!-- CHK-4 -->
- <a id="bkf-4"></a>🛑 `BKF-4` Backfeed through MOSFETs, e.g. in DC-DC converters · impl: hard (NET-8, NET-5) · [📖](checks/bkf.md#bkf-4) <!-- CHK-123 -->
- <a id="bkf-5"></a>⚠️ `BKF-5` Backfeed through LDOs · impl: hard (NET-8, NET-5) · [📖](checks/bkf.md#bkf-5) <!-- CHK-124 -->
- <a id="bkf-6"></a>⚠️ `BKF-6` Backfeed through internal IC structure where known (beyond IO protection diodes) · impl: judgement (CFG-5) · [📖](checks/bkf.md#bkf-6) <!-- CHK-125 -->
- <a id="bkf-7"></a>⚠️ `BKF-7` Power through IO or I2C lines; also spots 1-Wire, can exclude · impl: hard (NET-8, NET-10) · [📖](checks/bkf.md#bkf-7) <!-- CHK-177 -->
- <a id="bkf-8"></a>⚠️ `BKF-8` I2C device on a rail that can be off (backfeed should catch this) · impl: hard (NET-8, NET-4) · [📖](checks/bkf.md#bkf-8) <!-- CHK-19 -->
- <a id="bkf-9"></a>🛑 `BKF-9` Pins on a connector that an external device can drive while this board is unpowered: op-amp and MCU input clamps, pull-up PMOS body diodes and ESD diodes feed the rails. Needs powered-off-safe inputs, large series resistance or back-to-back switches · impl: hard (NET-10, NET-7, CFG-2) · [📖](checks/bkf.md#bkf-9) <!-- 21 Oct 2026; B200A -->
- <a id="bkf-10"></a>⚠️ `BKF-10` Level shifter or buffer whose own pull-ups feed an unpowered target through its outputs · impl: mid (NET-4, NET-7, CFG-2) · [📖](checks/bkf.md#bkf-10) <!-- 21 Oct 2026; B153A TXS0102 -->
- <a id="bkf-11"></a>⚠️ `BKF-11` USB VBUS joined to a system rail without a diode or switch on a board that can also be externally powered or act as host · impl: mid (NET-5, CFG-2) · [📖](checks/bkf.md#bkf-11) <!-- 21 Oct 2026 -->
- <a id="bkf-12"></a>⚠️ `BKF-12` Pull-up from a rail that can be off onto a bus that stays on: the bus feeds the off rail through the resistor and drags low ≈ BKF-7 · impl: hard (NET-8) · [📖](checks/bkf.md#bkf-12) <!-- 21 Oct 2026 -->

### 🪫 LEAK - Leakage and low power
- <a id="leak-1"></a>⚠️ `LEAK-1` Resistor divider straight on a power rail, draining power all the time · impl: easy (NET-5, IMP-6) · [📖](checks/leak.md#leak-1) <!-- CHK-44; split 21 Oct 2026: LEAK-4 -->
- <a id="leak-2"></a>💡 `LEAK-2` Pins that can't be high in some power states (go to another domain): list them even if the warning is suppressed · impl: hard (NET-8, NET-7) · [📖](checks/leak.md#leak-2) <!-- CHK-147 -->
- <a id="leak-3"></a>💡 `LEAK-3` NC pins: suggest setting them to analog or pulling them to cut power draw · impl: mid (NET-7) · [📖](checks/leak.md#leak-3) <!-- CHK-143 -->
- <a id="leak-4"></a>⚠️ `LEAK-4` Resistor divider or pull through a normally closed button or switch: current flows in the rest state · impl: mid (NET-5, NET-7) · [📖](checks/leak.md#leak-4) <!-- from LEAK-1 -->
- <a id="leak-5"></a>💡 `LEAK-5` Indicator LED permanently on a battery rail · impl: mid (NET-5, CFG-2) · [📖](checks/leak.md#leak-5) <!-- 21 Oct 2026 -->
- <a id="leak-6"></a>💡 `LEAK-6` Pull-up on a line that is held low in sleep (or pull-down on one held high): steady current in the low power state · impl: hard (NET-8, NET-7) · [📖](checks/leak.md#leak-6) <!-- 21 Oct 2026 -->
- <a id="leak-7"></a>💡 `LEAK-7` Battery measurement divider always connected: suggest a switched divider or a high-side switch ≈ LEAK-1 · impl: mid (NET-5, CFG-2) · [📖](checks/leak.md#leak-7) <!-- 21 Oct 2026 -->

## Logic and IO

### 📶 LVL - Logic levels and IO banks
- <a id="lvl-1"></a>🛑 `LVL-1` Pull-up, divider or other circuitry to a different voltage level (needs IO bank voltage; easy for single supply parts) · impl: mid (NET-5, NET-7, IMP-10) · [📖](checks/lvl.md#lvl-1) <!-- CHK-9 -->
- <a id="lvl-2"></a>🛑 `LVL-2` 1V8 output to 3V3 input and similar: compare thresholds · impl: mid (NET-5, NET-7) · [📖](checks/lvl.md#lvl-2) <!-- CHK-67 -->
- <a id="lvl-3"></a>💡 `LVL-3` I2C thresholds may allow skipping a translator, e.g. 1V8 into a 3V3 device with Vth 1.4 V · impl: mid (NET-4, NET-7) · [📖](checks/lvl.md#lvl-3) <!-- CHK-24 -->
- <a id="lvl-4"></a>🛑 `LVL-4` I2C discrete translator pull-up to the wrong voltage · impl: easy (NET-4, NET-5) · [📖](checks/lvl.md#lvl-4) <!-- CHK-161; split 21 Oct 2026: LVL-9 -->
- <a id="lvl-5"></a>⚠️ `LVL-5` Bidirectional level shifter used for unidirectional signals (e.g. USART + boot mode) · impl: mid (NET-7, NET-12) · [📖](checks/lvl.md#lvl-5) <!-- CHK-206 -->
- <a id="lvl-6"></a>💡 `LVL-6` A better suited bank exists for the net voltage or target bank voltage · impl: hard (NET-7, NET-12, IMP-10) · [📖](checks/lvl.md#lvl-6) <!-- CHK-156 -->

    For example on SPLD +3V3 side used, but +1V8 is the same side were net goes

- <a id="lvl-7"></a>⚠️ `LVL-7` Several power rails on a part and no banks defined: warn · impl: easy (NET-5) · [📖](checks/lvl.md#lvl-7) <!-- CHK-186 -->
- <a id="lvl-8"></a>🛑 `LVL-8` Signal not referenced to GND, e.g. an inverting buck and its enable signal · impl: mid (NET-5, NET-7) · [📖](checks/lvl.md#lvl-8) <!-- CHK-213 -->
- <a id="lvl-9"></a>🛑 `LVL-9` Discrete translator (BSS138 pair) with a gate reference that gives the wrong threshold for one side · impl: easy (NET-4, NET-5) · [📖](checks/lvl.md#lvl-9) <!-- from LVL-4 -->
- <a id="lvl-10"></a>🛑 `LVL-10` Higher-voltage output into a lower-voltage input above its absolute maximum (5 V into a 3.3 V pin that is not 5 V tolerant) · impl: mid (NET-5, NET-7) · [📖](checks/lvl.md#lvl-10) <!-- 21 Oct 2026 -->
- <a id="lvl-11"></a>⚠️ `LVL-11` Edge-accelerated auto-direction shifter (TXS01xx class) driving push-pull signals into a pull-down, a long line or capacitance: the high is held only by its weak pull-up, so SWD, UART and SPI come out slow and low. Use a direction-controlled translator (2T45 class) · impl: mid (NET-7, NET-10) · [📖](checks/lvl.md#lvl-11) <!-- 21 Oct 2026; B153A -->
- <a id="lvl-12"></a>🛑 `LVL-12` Level shifter direction pin (DIR) floating, or fixed against the signal's direction · impl: mid (NET-7, NET-12) · [📖](checks/lvl.md#lvl-12) <!-- 21 Oct 2026 -->
- <a id="lvl-13"></a>⚠️ `LVL-13` Level shifter output enable floating, or enabled before both of its rails are up · impl: mid (NET-7, NET-8) · [📖](checks/lvl.md#lvl-13) <!-- 21 Oct 2026 -->
- <a id="lvl-14"></a>🛑 `LVL-14` Digital isolator or transceiver channel direction against the signal (TX into an input channel) · impl: mid (NET-7, NET-12) · [📖](checks/lvl.md#lvl-14) <!-- 21 Oct 2026 -->
- <a id="lvl-15"></a>⚠️ `LVL-15` Single-MOSFET open-drain shifter (BSS138 pair) on a fast push-pull line (SPI over a few MHz, WS2812 data): too slow, use a buffer or a direction-controlled translator · impl: mid (NET-10, NET-12) · [📖](checks/lvl.md#lvl-15) <!-- 21 Oct 2026 -->

### 🎛️ IO - Default states and pulls
- <a id="io-1"></a>⚠️ `IO-1` Input pin without a drive source, smart about resistors, FETs etc. ≈ IO-2 · impl: easy (IMP-1, NET-10) · [📖](checks/io.md#io-1) <!-- CHK-14; split 21 Oct 2026: IO-10 -->
- <a id="io-2"></a>⚠️ `IO-2` IO to an input without pull-up/down: High-Z gives an undefined state · impl: mid (NET-7, NET-9) · [📖](checks/io.md#io-2) <!-- CHK-201 -->
- <a id="io-3"></a>🛑 `IO-3` Enable driven by an MCU or SoC GPIO without a pull: think of uninit and reboot state; `default_gpio_state` part property overrides · impl: mid (NET-7, NET-9) · [📖](checks/io.md#io-3) <!-- CHK-62; split 21 Oct 2026: IO-11 -->
- <a id="io-4"></a>⚠️ `IO-4` Assume IO pins are high/low/Z by default and look for bugs ≈ IO-5 · impl: hard (NET-8, NET-9, NET-10) · [📖](checks/io.md#io-4) <!-- CHK-168 -->
- <a id="io-5"></a>⚠️ `IO-5` If IO pin default states are known, derive checks · impl: hard (NET-9, NET-10, CFG-2) · [📖](checks/io.md#io-5) <!-- CHK-167 -->
- <a id="io-6"></a>⚠️ `IO-6` High impedance nets acting as touch buttons · impl: easy (IMP-1) · [📖](checks/io.md#io-6) <!-- CHK-130 -->
- <a id="io-7"></a>🛑 `IO-7` Button to ground with a pull-down, or button to power with a pull-up ≈ IO-8 · impl: easy (NET-5) · [📖](checks/io.md#io-7) <!-- CHK-175 -->
- <a id="io-8"></a>💡 `IO-8` Mention a needed pull-up or down where it applies, e.g. buttons · impl: easy (NET-5, RPT-13) · [📖](checks/io.md#io-8) <!-- CHK-145 -->
- <a id="io-9"></a>🛑 `IO-9` Pins a part drives in its non-firmware modes: ROM bootloader, factory test, reset and fault states. A ROM bootloader configures its interfaces (UART, I2C, SPI, CAN, USB) and may drive or toggle those pins while it scans for a host, with no firmware to keep anything else safe; check what each such pin controls (switches, enables, buses, resets) and what that level does there. Example: an STM32 ROM bootloader drives its USART TX pin high while it scans for a host; if that pin also selects a USB data switch, a firmware update over the ROM cuts the downstream device's data ≈ IO-3, IO-4 · impl: hard (NET-10, NET-12, CFG-2) · [📖](checks/io.md#io-9) <!-- B153A -->
- <a id="io-10"></a>⚠️ `IO-10` Input driven only from a connector: floats when nothing is plugged in · impl: easy (IMP-1) · [📖](checks/io.md#io-10) <!-- from IO-1 -->
- <a id="io-11"></a>🛑 `IO-11` MOSFET gate without a pull: floats while its driver is in reset or unpowered · impl: easy (IMP-1) · [📖](checks/io.md#io-11) <!-- from IO-3 -->
- <a id="io-12"></a>🛑 `IO-12` Output enable (`~OE`, `~G`, `EN`) of a shift register, driver, mux or buffer without a pull to its disabled state: outputs do whatever until firmware loads them · impl: mid (NET-7, NET-9) · [📖](checks/io.md#io-12) <!-- 21 Oct 2026; B200A 595 -->
- <a id="io-13"></a>⚠️ `IO-13` Peripheral reset pin pulled into reset by default, driven by a controller that resets: every controller reset (also into its ROM bootloader) takes the peripheral down with it. Decide on purpose: a pull-up with the controller able to drive low keeps the peripheral configured ≈ LOGIC-5 · impl: mid (NET-7, NET-9) · [📖](checks/io.md#io-13) <!-- 21 Oct 2026; B153A hub RESET_N -->
- <a id="io-14"></a>🛑 `IO-14` Configuration, mode or address straps of a non-MCU part (PHY, hub, codec, sensor) floating where the datasheet requires a level · impl: mid (NET-7, CFG-5) · [📖](checks/io.md#io-14) <!-- 21 Oct 2026 -->
- <a id="io-15"></a>🛑 `IO-15` Open-drain interrupt or fault output without a pull-up · impl: easy (IMP-1, NET-7) · [📖](checks/io.md#io-15) <!-- 21 Oct 2026 -->
- <a id="io-16"></a>🛑 `IO-16` Pull-up and pull-down both on one net: an accidental divider, neither level clean · impl: easy (NET-5) · [📖](checks/io.md#io-16) <!-- 21 Oct 2026 -->
- <a id="io-17"></a>⚠️ `IO-17` Pull value too weak for the leakage on the net (1 MΩ against ESD diode and input leakage) or too strong for the driver (1 kΩ on an open-drain output) · impl: mid (NET-7) · [📖](checks/io.md#io-17) <!-- 21 Oct 2026 -->
- <a id="io-18"></a>💡 `IO-18` Button without a debounce capacitor where the firmware assumes an RC filter · impl: mid (CFG-2, RPT-13) · [📖](checks/io.md#io-18) <!-- 21 Oct 2026; B153A PA7 -->

### 🧩 LOGIC - Pin logic and fan-out
- <a id="logic-1"></a>🛑 `LOGIC-1` Logical contradictions in GPIO or GPIO expander connections · AI prompt: check that GPIOs that drive multiple things have no logical contradictions, table of each occurrence · impl: judgement (NET-10, RPT-14) · [📖](checks/logic.md#logic-1) <!-- CHK-222 -->
- <a id="logic-2"></a>⚠️ `LOGIC-2` IO going to two MCUs or SoCs: odd requirements likely, esp. IRQs and buttons not going to reset inputs · impl: mid (NET-7, NET-9) · [📖](checks/logic.md#logic-2) <!-- CHK-174 -->
- <a id="logic-3"></a>💡 `LOGIC-3` Suggest buffering high fan-out signals · impl: easy (IMP-1) · [📖](checks/logic.md#logic-3) <!-- CHK-83 -->
- <a id="logic-4"></a>⚠️ `LOGIC-4` Total IO current per IC or bank · impl: hard (NET-7, NET-8, IMP-10) · [📖](checks/logic.md#logic-4) <!-- CHK-112 -->
- <a id="logic-5"></a>🛑 `LOGIC-5` Dependency loops in bring-up, update or recovery: part A only works once part B has set it up, while B needs A to start, update or recover. Then the board works while the firmware runs and fails exactly when it is needed (dead or half-written firmware, reset, ROM bootloader). Walk every boot, update and recovery path with each programmable part in its reset or unconfigured state. Example: an MCU whose only update path (its USB ROM bootloader) goes through a USB hub that the MCU itself must release from reset and configure over I2C: after a reset the bootloader never reaches the host. Same shape elsewhere: a sequencer or supervisor powered or configured by what it controls, a clock or PHY configured by the chip it serves, a config memory written through the bus it configures, a mux or level shifter on the programming path driven by the target ≈ SEQ-2 · impl: judgement (NET-8, NET-10, NET-12) · [📖](checks/logic.md#logic-5) <!-- B153A -->
- <a id="logic-6"></a>🛑 `LOGIC-6` Current through one pin above its maximum (an LED without resistor, a relay coil, a long pull chain) ≈ LOGIC-4 · impl: mid (NET-7) · [📖](checks/logic.md#logic-6) <!-- 21 Oct 2026 -->
- <a id="logic-7"></a>🛑 `LOGIC-7` Open-drain or open-collector output as the only driver of a net with no pull-up: never goes high · impl: easy (IMP-1, NET-7) · [📖](checks/logic.md#logic-7) <!-- 21 Oct 2026 -->
- <a id="logic-8"></a>⚠️ `LOGIC-8` Polarity mismatch by name: an active-low output (`_N`, `#`, `~`) into an active-high input or the reverse (`PGOOD` into `~EN`, `INT_N` into a rising-edge IRQ) with no inverter · impl: easy (IMP-1) · [📖](checks/logic.md#logic-8) <!-- 21 Oct 2026 -->

### 🔀 MUX - Pin functions
- <a id="mux-1"></a>🛑 `MUX-1` SCL goes to a pin named SCL, MISO to MISO etc · impl: easy (IMP-1) · [📖](checks/mux.md#mux-1) <!-- CHK-159 -->
- <a id="mux-2"></a>🛑 `MUX-2` MCUs with SVD/pin data: check the alternate function actually supports the mode used · impl: mid (NET-12, IMP-9) · [📖](checks/mux.md#mux-2) <!-- CHK-108 -->

    - Load alternate modes CSV for SoCs

    [https://github.com/embassy-rs/stm32-data/blob/d828c292da3dfb6679e7b126db5b1c4408dc6ec6/stm32-data-gen/src/gpio_af.rs#L48](https://github.com/embassy-rs/stm32-data/blob/d828c292da3dfb6679e7b126db5b1c4408dc6ec6/stm32-data-gen/src/gpio_af.rs#L48)

- <a id="mux-3"></a>💡 `MUX-3` SLG* (GreenPAK): suggest using all pins, e.g. to monitor things, connect to ENs, route EN through if inverting or delay is needed · impl: judgement (netlist) · [📖](checks/mux.md#mux-3) <!-- CHK-198 -->
- <a id="mux-4"></a>🛑 `MUX-4` GPIO on a pin that is NRST, BOOT or another reset-time function by default (STM32G0 PF2-NRST, option-byte controlled): the firmware never gets the pin, or a low resets the chip · impl: mid (NET-12, IMP-9) · [📖](checks/mux.md#mux-4) <!-- 21 Oct 2026; B153A -->
- <a id="mux-5"></a>🛑 `MUX-5` Debug pins (SWDIO, SWCLK, JTAG) reused as GPIO: debugging and flashing lost once the firmware runs ≈ DBG-1 · impl: mid (NET-12) · [📖](checks/mux.md#mux-5) <!-- 21 Oct 2026 -->
- <a id="mux-6"></a>🛑 `MUX-6` Analog signal (ADC, DAC, comparator) on a pin without that analog function · impl: mid (NET-7, NET-12) · [📖](checks/mux.md#mux-6) <!-- 21 Oct 2026 -->
- <a id="mux-7"></a>🛑 `MUX-7` One peripheral signal on two pins, or two uses of a shared peripheral (USB and CAN on an STM32F103 share one SRAM block) · impl: mid (NET-12, CFG-2) · [📖](checks/mux.md#mux-7) <!-- 21 Oct 2026 -->
- <a id="mux-8"></a>🛑 `MUX-8` 5 V net on a pin that is not 5 V tolerant where the part has tolerant (FT) pins to choose from ≈ LVL-10 · impl: mid (NET-5, NET-12) · [📖](checks/mux.md#mux-8) <!-- 21 Oct 2026 -->

### 🥾 BOOT - Boot modes and strapping pins
- <a id="boot-1"></a>🛑 `BOOT-1` Boot or strapping pin with no defined level at reset (no pull, no driver that is up at that moment) · impl: mid (NET-7, NET-9, NET-12) · [📖](checks/boot.md#boot-1) <!-- 21 Oct 2026 -->
- <a id="boot-2"></a>🛑 `BOOT-2` Strapping pin shared with a signal that another device may drive at reset: a UART bridge on ESP32 GPIO0, a card or display on an i.MX boot config pin, a PHY LED with a pull that changes its mode strap · impl: hard (NET-8, NET-9, NET-10) · [📖](checks/boot.md#boot-2) <!-- 21 Oct 2026 -->
- <a id="boot-3"></a>🛑 `BOOT-3` Strapping pin pulled to a mode the design does not want (STM32 BOOT0 high boots the ROM, ESP32 GPIO12 high selects 1.8 V flash) · impl: mid (NET-12, CFG-2) · [📖](checks/boot.md#boot-3) <!-- 21 Oct 2026 -->
- <a id="boot-4"></a>⚠️ `BOOT-4` Boot pin wired but ignored by the factory option bytes or fuses (STM32G0 nBOOT_SEL): a programming step must enable it, say so in the bring-up notes · impl: mid (NET-12, CFG-2) · [📖](checks/boot.md#boot-4) <!-- 21 Oct 2026; B153A -->
- <a id="boot-5"></a>💡 `BOOT-5` No way to force the ROM bootloader (button, jumper, header pin) on a board updated in the field · impl: mid (NET-12) · [📖](checks/boot.md#boot-5) <!-- 21 Oct 2026 -->
- <a id="boot-6"></a>🛑 `BOOT-6` ROM bootloader interface not reachable from outside with the firmware dead: behind a hub, mux or shifter that only firmware configures ≈ LOGIC-5 · impl: hard (NET-8, NET-10, NET-12, NET-13) · [📖](checks/boot.md#boot-6) <!-- 21 Oct 2026; B153A -->
- <a id="boot-7"></a>🛑 `BOOT-7` Boot memory on pins or a bus the ROM cannot boot from (QSPI flash on a non-boot CS, SD on a non-boot SDIO instance), or a flash whose command set the ROM's loader does not support · impl: mid (NET-12) · [📖](checks/boot.md#boot-7) <!-- 21 Oct 2026 -->
- <a id="boot-8"></a>⚠️ `BOOT-8` Part strapped for a configuration that assumes a controller (a hub strapped for SMBus config powers up with every register at zero): fine only if that controller is always there first · impl: mid (NET-7, CFG-5) · [📖](checks/boot.md#boot-8) <!-- 21 Oct 2026; B153A -->
- <a id="boot-9"></a>⚠️ `BOOT-9` Strapping pin shared with a peripheral signal without a series resistor between them: the peripheral loads the strap during the latch window, the strap must win · impl: mid (NET-12, IMP-1) · [📖](checks/boot.md#boot-9) <!-- 21 Oct 2026 -->
- <a id="boot-10"></a>🛑 `BOOT-10` FPGA configuration pins (mode select MSEL/CFG, nCONFIG/PROGRAM_B, nSTATUS/INIT_B, CONF_DONE/DONE, PUDC_B, nCE/nCEO) floating or without the pull-ups and pull-downs the configuration guide requires · impl: mid (NET-7, IMP-10) · [📖](checks/boot.md#boot-10) <!-- 21 Oct 2026 -->

### ⏱️ CLK - Clocks and reset
- <a id="clk-1"></a>🛑 `CLK-1` MCU with CAN or USB and no crystal · impl: mid (NET-12) · [📖](checks/clk.md#clk-1) <!-- CHK-100 -->
- <a id="clk-2"></a>⚠️ `CLK-2` HS crystal capacitors should be high frequency rated · impl: mid (netlist (MPN field)) · [📖](checks/clk.md#clk-2) <!-- CHK-79 -->
- <a id="clk-3"></a>⚠️ `CLK-3` Crystal NC pins may need to go to ground · impl: easy (IMP-1) · [📖](checks/clk.md#clk-3) <!-- CHK-33 -->
- <a id="clk-4"></a>⚠️ `CLK-4` Crystal pins used as IO: warn about low drive strength · impl: mid (NET-12) · [📖](checks/clk.md#clk-4) <!-- CHK-173 -->
- <a id="clk-5"></a>⚠️ `CLK-5` No pull-up on a reset_n input · impl: easy (IMP-1, NET-7) · [📖](checks/clk.md#clk-5) <!-- CHK-70; split 21 Oct 2026: CLK-7 -->
- <a id="clk-6"></a>💡 `CLK-6` Suggest a cap on buttons to reset inputs; note hw debounce in the programmer report · impl: easy (IMP-1, RPT-13) · [📖](checks/clk.md#clk-6) <!-- CHK-133 -->
- <a id="clk-7"></a>⚠️ `CLK-7` No capacitor on a reset_n input: noise and brown-outs reach it ≈ CLK-6 · impl: easy (IMP-1) · [📖](checks/clk.md#clk-7) <!-- from CLK-5 -->
- <a id="clk-8"></a>⚠️ `CLK-8` Crystal load capacitors vs the crystal's CL: C1 = C2 ≈ 2·CL − Cstray; far off means the frequency is off or it does not start · impl: mid (NET-7) · [📖](checks/clk.md#clk-8) <!-- 21 Oct 2026 -->
- <a id="clk-9"></a>⚠️ `CLK-9` Crystal ESR and drive level vs the oscillator: gain margin (gm / gm_crit) under 5, or no series resistor where the drive level is exceeded (ST AN2867) · impl: mid (NET-7) · [📖](checks/clk.md#clk-9) <!-- 21 Oct 2026 -->
- <a id="clk-10"></a>🛑 `CLK-10` Crystal frequency outside the range the part's oscillator supports · impl: mid (NET-7) · [📖](checks/clk.md#clk-10) <!-- 21 Oct 2026 -->
- <a id="clk-11"></a>⚠️ `CLK-11` 32.768 kHz crystal with a CL or ESR the low-power oscillator cannot drive · impl: mid (NET-7) · [📖](checks/clk.md#clk-11) <!-- 21 Oct 2026 -->
- <a id="clk-12"></a>⚠️ `CLK-12` Active oscillator output into a crystal pin without bypass mode or the attenuation the datasheet asks for · impl: mid (NET-7, CFG-2) · [📖](checks/clk.md#clk-12) <!-- 21 Oct 2026 -->
- <a id="clk-13"></a>🛑 `CLK-13` Oscillator or clock buffer output-enable / standby pin floating · impl: mid (NET-7) · [📖](checks/clk.md#clk-13) <!-- 21 Oct 2026 -->
- <a id="clk-14"></a>⚠️ `CLK-14` Reset input driven by a push-pull output: fights the debugger, supervisor or button that also drives it; reset lines are open-drain · impl: easy (IMP-1, NET-7) · [📖](checks/clk.md#clk-14) <!-- 21 Oct 2026 -->
- <a id="clk-15"></a>⚠️ `CLK-15` One reset net shared by the MCU and its peripherals: a debugger reset (or a watchdog) resets the whole board, and peripherals lose configuration on every firmware restart ≈ IO-13 · impl: easy (IMP-1) · [📖](checks/clk.md#clk-15) <!-- 21 Oct 2026 -->
- <a id="clk-16"></a>⚠️ `CLK-16` RTC in use with VBAT floating, no backup source, or no 32 kHz crystal · impl: mid (NET-12, CFG-2) · [📖](checks/clk.md#clk-16) <!-- 21 Oct 2026 -->
- <a id="clk-17"></a>💡 `CLK-17` External watchdog with no way to disable it for debugging (jumper, DNM resistor) · impl: mid (NET-7) · [📖](checks/clk.md#clk-17) <!-- 21 Oct 2026 -->
- <a id="clk-18"></a>💡 `CLK-18` Clock output fanned out to several loads without a buffer or series termination · impl: easy (IMP-1, NET-12) · [📖](checks/clk.md#clk-18) <!-- 21 Oct 2026 -->
- <a id="clk-19"></a>⚠️ `CLK-19` Crystal tolerance vs the interface that uses it: USB HS ±500 ppm, Ethernet ±50 ppm, BLE ±50 ppm or tighter, CAN FD per bit rate · impl: mid (NET-12, CFG-2) · [📖](checks/clk.md#clk-19) <!-- 21 Oct 2026 -->

## Interfaces

### 🚌 I2C - I2C
- <a id="i2c-1"></a>🛑 `I2C-1` No pull-ups on an I2C bus · impl: easy (NET-4, NET-5) · [📖](checks/i2c.md#i2c-1) <!-- CHK-20; split 21 Oct 2026: I2C-9, I2C-10 -->
- <a id="i2c-2"></a>⚠️ `I2C-2` I2C multiple pull-ups · impl: easy (NET-4) · [📖](checks/i2c.md#i2c-2) <!-- CHK-107 -->
- <a id="i2c-3"></a>🛑 `I2C-3` Pull-up from two series resistors to nowhere (part of I2C-1) · impl: easy (NET-4, NET-10) · [📖](checks/i2c.md#i2c-3) <!-- CHK-162 -->
- <a id="i2c-4"></a>🛑 `I2C-4` Pull-up to a non-rail net (resistor with only two pins connected)? · impl: easy (NET-4, NET-5) · [📖](checks/i2c.md#i2c-4) <!-- CHK-163 -->
- <a id="i2c-5"></a>🛑 `I2C-5` I2C address collision (needs addresses in the library, maybe AI can pull them out) · impl: mid (NET-11) · [📖](checks/i2c.md#i2c-5) <!-- CHK-15 -->
- <a id="i2c-6"></a>💡 `I2C-6` I2C going to a connector that isn't FPC or fine pitch B2B: suggest another interface, keep I2C on board? · impl: mid (NET-4, LIB-2) · [📖](checks/i2c.md#i2c-6) <!-- CHK-99 -->
- <a id="i2c-7"></a>⚠️ `I2C-7` Series tie resistor on an I2C line too high (over 100 Ω) · impl: easy (NET-4, NET-10) · [📖](checks/i2c.md#i2c-7) <!-- new 5 Oct 2026, found in code -->
- <a id="i2c-8"></a>⚠️ `I2C-8` Pull-up too weak for the bus speed: Rp > tr_max / (0.847 · Cb), tr (30-70 %) max 1000 ns at 100k, 300 ns at 400k, 120 ns at 1M; Cb estimated from the pins on the net (or a project setting). Measured on a bench: ~135 pF per line, 10k gives 1170 ns (fails 100k), 2.2k 277 ns (meets 400k), 1M needs ≤ 1.05k · impl: mid (NET-4, CFG-2) · [📖](checks/i2c.md#i2c-8) <!-- new 10 Oct 2026 -->
- <a id="i2c-9"></a>⚠️ `I2C-9` Pull-up value outside the recommended range (2.2k-10k): too strong for the sink current (3 mA, 20 mA in Fm+), too weak for the edges ≈ I2C-8 · impl: easy (NET-4, IMP-6) · [📖](checks/i2c.md#i2c-9) <!-- from I2C-1 -->
- <a id="i2c-10"></a>⚠️ `I2C-10` SCL and SDA pull-ups of different value · impl: easy (NET-4) · [📖](checks/i2c.md#i2c-10) <!-- from I2C-1 -->
- <a id="i2c-11"></a>🛑 `I2C-11` I2C device whose supply can be off while the bus is on, without fail-safe (Ioff) inputs: it feeds itself through its ESD diodes and clamps both lines to ~0.6 V, the whole bus is dead ≈ BKF-8 · impl: hard (NET-7, NET-8) · [📖](checks/i2c.md#i2c-11) <!-- 21 Oct 2026; P2623 I2C in the wild -->
- <a id="i2c-12"></a>⚠️ `I2C-12` SMBus device on an I2C bus: it times out when the clock drops under 10 kHz or a transfer stalls over 25-35 ms, and its fixed thresholds (0.8 / 2.1 V in SMBus 2.0, 0.8 / 1.35 V in 3.0) do not match I2C's 0.3 / 0.7 VDD on 5 V and 1.8 V buses · impl: mid (CFG-5, CFG-2) · [📖](checks/i2c.md#i2c-12) <!-- 21 Oct 2026; P2623 -->
- <a id="i2c-13"></a>🛑 `I2C-13` Static-offset bus buffers (P82B96, PCA9600, PCA9515 class) in series, or one facing a device with a low VIL: the buffer's ~0.5 V low is never seen as low, the bus locks · impl: mid (CFG-5) · [📖](checks/i2c.md#i2c-13) <!-- 21 Oct 2026; P2623 -->
- <a id="i2c-14"></a>⚠️ `I2C-14` Push-pull auto-direction shifter (TXB010x class) on I2C: its direction sensing fails on open-drain lines and with pull-ups under ~50k. Edge-accelerated open-drain parts (TXS01xx) work but their one-shots can glitch slow edges; a plain open-drain shifter (PCA9306, BSS138 pair) is the safe choice ≈ LVL-11 · impl: mid (CFG-5) · [📖](checks/i2c.md#i2c-14) <!-- 21 Oct 2026; P2623 -->
- <a id="i2c-15"></a>⚠️ `I2C-15` I2C mux or switch whose channels, joined by firmware, carry devices with the same address, or whose summed capacitance exceeds the limit · impl: mid (NET-11, CFG-5) · [📖](checks/i2c.md#i2c-15) <!-- 21 Oct 2026; P2623 -->
- <a id="i2c-16"></a>⚠️ `I2C-16` Device at a reserved address: 0x00-0x07 (general call, CBUS, Hs master codes) or 0x78-0x7F (10-bit prefix, device ID) · impl: mid (NET-11) · [📖](checks/i2c.md#i2c-16) <!-- 21 Oct 2026 -->
- <a id="i2c-17"></a>🛑 `I2C-17` Address straps (A0-A2, ADDR, SA0) floating: the address is undefined, or read at power-up only · impl: mid (NET-7) · [📖](checks/i2c.md#i2c-17) <!-- 21 Oct 2026 -->
- <a id="i2c-18"></a>🛑 `I2C-18` Bus segment behind a translator, isolator, buffer or mux with no pull-ups of its own · impl: easy (NET-4) · [📖](checks/i2c.md#i2c-18) <!-- 21 Oct 2026 -->
- <a id="i2c-19"></a>⚠️ `I2C-19` Pull-ups of a segment to a rail that can be off while the rest of the bus is on ≈ BKF-12 · impl: hard (NET-4, NET-8) · [📖](checks/i2c.md#i2c-19) <!-- 21 Oct 2026 -->
- <a id="i2c-20"></a>💡 `I2C-20` Device that stretches the clock for its conversions on a master known not to support stretching (BCM2835 and its errata) · impl: judgement (CFG-5) · [📖](checks/i2c.md#i2c-20) <!-- 21 Oct 2026; P2623 -->
- <a id="i2c-21"></a>⚠️ `I2C-21` Estimated bus capacitance over 400 pF (long cable, many devices): needs a buffer or a slower mode ≈ I2C-8 · impl: mid (NET-4, CFG-2) · [📖](checks/i2c.md#i2c-21) <!-- 21 Oct 2026 -->
- <a id="i2c-22"></a>💡 `I2C-22` Several masters on one bus: all controllers must support multi-master arbitration, and the firmware must handle bus-busy · impl: mid (NET-7, CFG-5) · [📖](checks/i2c.md#i2c-22) <!-- 21 Oct 2026 -->

### 🔁 SER - UART, SPI, CAN, RS-485, LVDS and whole-bus checks
- <a id="ser-1"></a>🛑 `SER-1` USART TX connected to TX (not crossed) · impl: mid (NET-7, NET-12) · [📖](checks/ser.md#ser-1) <!-- CHK-115; split 21 Oct 2026: SER-7, SER-8 -->
- <a id="ser-2"></a>🛑 `SER-2` SPI CS line grounded: some ICs need it to work · impl: mid (NET-7) · [📖](checks/ser.md#ser-2) <!-- CHK-27 -->
- <a id="ser-3"></a>⚠️ `SER-3` CAN terminator based on MOSFETs or SSRs · impl: mid (NET-7, NET-10) · [📖](checks/ser.md#ser-3) <!-- CHK-122 -->
- <a id="ser-4"></a>🛑 `SER-4` LVDS without terminators · impl: mid (NET-7, IMP-10) · [📖](checks/ser.md#ser-4) <!-- CHK-138 -->
- <a id="ser-5"></a>🛑 `SER-5` Not the whole bus is present, maybe nets shorted out ≈ SER-6 · impl: mid (NET-7, NET-12) · [📖](checks/ser.md#ser-5) <!-- CHK-166 -->
- <a id="ser-6"></a>🛑 `SER-6` Not the whole bus is connected to a device · impl: mid (NET-7, NET-12) · [📖](checks/ser.md#ser-6) <!-- CHK-164 -->
- <a id="ser-7"></a>🛑 `SER-7` CAN TX and RX crossed between MCU and transceiver (they are not crossed, unlike UART) · impl: mid (NET-7, NET-12) · [📖](checks/ser.md#ser-7) <!-- from SER-1 -->
- <a id="ser-8"></a>🛑 `SER-8` UART flow control mis-paired: CTS to CTS, or RTS of one side not to CTS of the other · impl: mid (NET-7, NET-12) · [📖](checks/ser.md#ser-8) <!-- from SER-1 -->
- <a id="ser-9"></a>🛑 `SER-9` RS-485 bus without termination at both ends (120 Ω) where this board is an end node · impl: mid (NET-7, CFG-2) · [📖](checks/ser.md#ser-9) <!-- 21 Oct 2026 -->
- <a id="ser-10"></a>⚠️ `SER-10` RS-485 without fail-safe biasing (A pulled up, B pulled down) on a transceiver without built-in fail-safe: idle bus reads noise · impl: mid (NET-7) · [📖](checks/ser.md#ser-10) <!-- 21 Oct 2026 -->
- <a id="ser-11"></a>🛑 `SER-11` RS-485 DE / ~RE without a pull to the receive state: the transceiver may drive the bus while the MCU is in reset · impl: mid (NET-7, NET-9) · [📖](checks/ser.md#ser-11) <!-- 21 Oct 2026 -->
- <a id="ser-12"></a>🛑 `SER-12` RS-485 A/B or CAN H/L swapped against the connector's documented pinout · impl: mid (NET-7, CFG-2) · [📖](checks/ser.md#ser-12) <!-- 21 Oct 2026 -->
- <a id="ser-13"></a>⚠️ `SER-13` CAN termination wrong for the node's place: none at a bus end, or a 120 Ω on a node in the middle · impl: mid (NET-7, CFG-2) · [📖](checks/ser.md#ser-13) <!-- 21 Oct 2026 -->
- <a id="ser-14"></a>⚠️ `SER-14` CAN split termination (2 × 60 Ω) without the centre capacitor to ground · impl: easy (netlist) · [📖](checks/ser.md#ser-14) <!-- 21 Oct 2026 -->
- <a id="ser-15"></a>🛑 `SER-15` CAN transceiver STB, S or EN pin floating: it stays in standby or silent mode · impl: mid (NET-7, NET-9) · [📖](checks/ser.md#ser-15) <!-- 21 Oct 2026 -->
- <a id="ser-16"></a>⚠️ `SER-16` CAN FD data rate above the transceiver's class (a classic 1 Mbit/s transceiver on a 5 Mbit/s FD bus) · impl: mid (CFG-2) · [📖](checks/ser.md#ser-16) <!-- 21 Oct 2026 -->
- <a id="ser-17"></a>🛑 `SER-17` Transceiver logic side (VIO) on a different rail than the MCU pins it talks to · impl: mid (NET-5, NET-7) · [📖](checks/ser.md#ser-17) <!-- 21 Oct 2026 -->
- <a id="ser-18"></a>🛑 `SER-18` SPI chip select shared by two slaves, or a slave whose MISO does not tri-state when deselected on a shared MISO · impl: mid (NET-7) · [📖](checks/ser.md#ser-18) <!-- 21 Oct 2026 -->
- <a id="ser-19"></a>⚠️ `SER-19` SPI slave CS without a pull-up (flash, sensor, any slave): floats during the host's reset, the slave sees commands in the noise and a second slave answers on a shared MISO · impl: mid (NET-7, NET-9) · [📖](checks/ser.md#ser-19) <!-- 21 Oct 2026 -->
- <a id="ser-20"></a>🛑 `SER-20` RS-232 levels (±12 V) straight into MCU pins, no transceiver · impl: mid (NET-7, NET-10) · [📖](checks/ser.md#ser-20) <!-- 21 Oct 2026 -->
- <a id="ser-21"></a>💡 `SER-21` USB-UART bridge RTS / DTR wired to boot and reset for auto-programming, or deliberately not: say which · impl: judgement (NET-7) · [📖](checks/ser.md#ser-21) <!-- 21 Oct 2026 -->
- <a id="ser-22"></a>💡 `SER-22` 1-Wire with parasitically powered devices and only a weak pull-up: suggest a strong pull-up switch · impl: mid (NET-7, NET-10) · [📖](checks/ser.md#ser-22) <!-- 21 Oct 2026 -->
- <a id="ser-23"></a>⚠️ `SER-23` RS-485 or CAN node with no ground reference to the bus (no ground wire, no isolation), or its ground tied straight to the remote ground (loop): the standards want a resistor-separated ground or an isolated node · impl: hard (NET-5) · [📖](checks/ser.md#ser-23) <!-- 21 Oct 2026 -->
- <a id="ser-24"></a>⚠️ `SER-24` RS-485 unit-load budget exceeded: nodes plus the bias network over 32 UL (256 with 1/8-UL transceivers) · impl: mid (NET-7, CFG-2) · [📖](checks/ser.md#ser-24) <!-- 21 Oct 2026 -->

### 🌐 ETH - Ethernet
- <a id="eth-1"></a>🛑 `ETH-1` Magnetics centre taps terminated for the wrong PHY type (current-mode PHYs want the taps tied to the supply, voltage-mode PHYs want separate taps to ground through capacitors; DP83848C §7.2, KSZ9031 §12), or only two of the four taps terminated on a gigabit design copied from 10/100 · impl: mid (NET-7, NET-5) · [📖](checks/eth.md#eth-1) <!-- 21 Oct 2026 -->
- <a id="eth-2"></a>⚠️ `ETH-2` RGMII delay: neither the PHY's internal delay strap nor the MAC provides the 2 ns skew, or both do · impl: hard (NET-7, CFG-2) · [📖](checks/eth.md#eth-2) <!-- 21 Oct 2026 -->
- <a id="eth-3"></a>🛑 `ETH-3` MDIO without a pull-up (1.5k-10k) · impl: easy (NET-5) · [📖](checks/eth.md#eth-3) <!-- 21 Oct 2026 -->
- <a id="eth-4"></a>⚠️ `ETH-4` PHY address straps unset, two PHYs on one MDIO bus with the same address, or address 0 on a PHY that treats it as broadcast · impl: mid (NET-7) · [📖](checks/eth.md#eth-4) <!-- 21 Oct 2026 -->
- <a id="eth-5"></a>💡 `ETH-5` Unused pairs and centre taps without Bob Smith termination (75 Ω to a 1 nF 2 kV capacitor to chassis), or with it returned to signal ground, on the PHY side, or through a cap below 2 kV · impl: mid (NET-7, NET-5) · [📖](checks/eth.md#eth-5) <!-- 21 Oct 2026 -->
- <a id="eth-6"></a>🛑 `ETH-6` RMII REF_CLK direction wrong: both or neither side sources the 50 MHz · impl: mid (NET-7, NET-12) · [📖](checks/eth.md#eth-6) <!-- 21 Oct 2026 -->
- <a id="eth-7"></a>🛑 `ETH-7` PHY LED pins that double as configuration straps: the LED and its pull set a mode at reset · impl: mid (NET-7) · [📖](checks/eth.md#eth-7) <!-- 21 Oct 2026 -->
- <a id="eth-8"></a>⚠️ `ETH-8` MDI pair polarity or pair order against the jack's pinout (Auto-MDIX saves pair swaps, not always polarity) · impl: hard (NET-10, NET-7) · [📖](checks/eth.md#eth-8) <!-- 21 Oct 2026 -->
- <a id="eth-9"></a>⚠️ `ETH-9` PHY crystal tolerance above ±50 ppm · impl: mid (CFG-5, NET-7) · [📖](checks/eth.md#eth-9) <!-- 21 Oct 2026 -->
- <a id="eth-10"></a>🛑 `ETH-10` PoE powered device without isolation of the PD side, or without the classification resistor · impl: hard (NET-5, NET-7) · [📖](checks/eth.md#eth-10) <!-- 21 Oct 2026 -->
- <a id="eth-11"></a>💡 `ETH-11` RGMII / RMII lines without the series resistors the PHY's reference design uses · impl: mid (NET-7) · [📖](checks/eth.md#eth-11) <!-- 21 Oct 2026 -->
- <a id="eth-12"></a>⚠️ `ETH-12` PHY reset released before its clock is stable, or with no pull at all · impl: hard (NET-8, NET-10) · [📖](checks/eth.md#eth-12) <!-- 21 Oct 2026 -->
- <a id="eth-13"></a>⚠️ `ETH-13` Magnetics not rated for the design: isolation under 1500 Vac, or not PoE-rated where PoE current biases the core · impl: mid (CFG-5) · [📖](checks/eth.md#eth-13) <!-- 21 Oct 2026 -->
- <a id="eth-14"></a>⚠️ `ETH-14` PHY multi-level strap divider computed without the pin's internal pull (9k-40k, ±25 %), with 5 % resistors, or with a capacitor straight on the strap · impl: mid (NET-7, IMP-6) · [📖](checks/eth.md#eth-14) <!-- 21 Oct 2026 -->

### 💾 MEM - Memory (DDR, eMMC, SD, flash)
- <a id="mem-1"></a>⚠️ `MEM-1` SD / eMMC CMD and DAT lines without pull-ups (10k-100k) · impl: easy (netlist, NET-5) · [📖](checks/mem.md#mem-1) <!-- CHK-31; split 21 Oct 2026: MEM-3 -->
- <a id="mem-2"></a>🛑 `MEM-2` DDR pin swap across bytes and other errors · impl: hard (NET-7, NET-10) · [📖](checks/mem.md#mem-2) <!-- CHK-40 -->
- <a id="mem-3"></a>⚠️ `MEM-3` SD / eMMC CLK with a pull-up, or CD / WP switch pins without one · impl: easy (netlist, NET-5) · [📖](checks/mem.md#mem-3) <!-- from MEM-1 -->
- <a id="mem-4"></a>🛑 `MEM-4` SPI / QSPI flash WP#, HOLD# or RESET# floating: writes blocked or the part held at random · impl: mid (NET-7) · [📖](checks/mem.md#mem-4) <!-- 21 Oct 2026 -->
- <a id="mem-5"></a>⚠️ `MEM-5` SD card detect wired to the wrong pins (shell instead of the switch pins, or to ground on a socket that signals to VDD) · impl: mid (NET-7) · [📖](checks/mem.md#mem-5) <!-- 21 Oct 2026; B125A -->
- <a id="mem-6"></a>🛑 `MEM-6` eMMC VCCQ on a different voltage than the host's IO bank, or RST_n without a pull-up · impl: mid (NET-5, NET-7) · [📖](checks/mem.md#mem-6) <!-- 21 Oct 2026 -->
- <a id="mem-7"></a>⚠️ `MEM-7` SD host IO voltage vs card mode: UHS needs 1.8 V signalling, a 3.3 V-only host stays at high-speed · impl: mid (NET-5, NET-7, CFG-2) · [📖](checks/mem.md#mem-7) <!-- 21 Oct 2026 -->
- <a id="mem-8"></a>🛑 `MEM-8` DDR ZQ pin without its 240 Ω to VSS · impl: easy (netlist, IMP-6) · [📖](checks/mem.md#mem-8) <!-- 21 Oct 2026 -->
- <a id="mem-9"></a>🛑 `MEM-9` DDR VREF (CA, DQ) not at VDD/2, or VTT termination missing on a fly-by topology · impl: mid (NET-5, RPT-6) · [📖](checks/mem.md#mem-9) <!-- 21 Oct 2026 -->
- <a id="mem-10"></a>⚠️ `MEM-10` DDR RESET_n without a pull-down and CKE without a defined level at power-up · impl: easy (netlist, NET-5) · [📖](checks/mem.md#mem-10) <!-- 21 Oct 2026 -->
- <a id="mem-11"></a>⚠️ `MEM-11` EEPROM or flash write-protect pin floating · impl: mid (NET-7) · [📖](checks/mem.md#mem-11) <!-- 21 Oct 2026 -->
- <a id="mem-12"></a>💡 `MEM-12` SD card power not switchable: a card that hangs cannot be re-initialised without a board power cycle ≈ SEQ-3 · impl: hard (NET-5, NET-10) · [📖](checks/mem.md#mem-12) <!-- 21 Oct 2026 -->

### ⚡ HS - High speed
- <a id="hs-1"></a>🛑 `HS-1` PCIe, DP: correct AC caps present vs connector or onboard IC · impl: mid (NET-7, NET-10) · [📖](checks/hs.md#hs-1) <!-- CHK-104 -->
- <a id="hs-2"></a>⚠️ `HS-2` Lane swaps, bit swaps: warn to check actual support · impl: mid (NET-7, NET-10) · [📖](checks/hs.md#hs-2) <!-- CHK-105 -->
- <a id="hs-3"></a>⚠️ `HS-3` Clocked buses where some signals go through dividers or buffers and others don't: delay, setup/hold problems · impl: hard (NET-10) · [📖](checks/hs.md#hs-3) <!-- CHK-43 -->
- <a id="hs-4"></a>⚠️ `HS-4` Differential asymmetry · impl: easy (netlist) · [📖](checks/hs.md#hs-4) <!-- CHK-190 -->
- <a id="hs-5"></a>⚠️ `HS-5` Series resistors on USB D+/D−; skip only for known parts that need them, e.g. F103 · impl: mid (NET-7, NET-10) · [📖](checks/hs.md#hs-5) <!-- CHK-110; split 21 Oct 2026: HS-6 -->
- <a id="hs-6"></a>⚠️ `HS-6` External D+ pull-up on a USB device that has it built in (or none on one that has not) · impl: mid (NET-7, IMP-9) · [📖](checks/hs.md#hs-6) <!-- from HS-5 -->
- <a id="hs-7"></a>🛑 `HS-7` USB D+ and D− swapped (by pin name on the part vs the connector) · impl: easy (netlist) · [📖](checks/hs.md#hs-7) <!-- 21 Oct 2026 -->
- <a id="hs-8"></a>⚠️ `HS-8` Protection device capacitance too high for the data rate (over ~1 pF on USB 2.0 HS, over ~0.3 pF on USB 3, PCIe, HDMI) · impl: mid (NET-7, CFG-2) · [📖](checks/hs.md#hs-8) <!-- 21 Oct 2026 -->
- <a id="hs-9"></a>💡 `HS-9` Common-mode choke on USB 3 or PCIe lanes: hurts more than it helps at these rates · impl: easy (netlist) · [📖](checks/hs.md#hs-9) <!-- 21 Oct 2026 -->
- <a id="hs-10"></a>🛑 `HS-10` Differential pair P and N swapped across a connector or between two parts (where the receiver cannot invert polarity) · impl: mid (NET-7, NET-10) · [📖](checks/hs.md#hs-10) <!-- 21 Oct 2026 -->
- <a id="hs-11"></a>🛑 `HS-11` PCIe sideband missing or unpulled: PERST#, WAKE#, CLKREQ#, REFCLK; PRSNT pins on a card · impl: mid (NET-7, NET-5) · [📖](checks/hs.md#hs-11) <!-- 21 Oct 2026 -->
- <a id="hs-12"></a>🛑 `HS-12` PCIe, USB 3, SATA TX/RX not crossed between host and device (host TX goes to device RX) · impl: mid (NET-7, NET-10) · [📖](checks/hs.md#hs-12) <!-- 21 Oct 2026 -->
- <a id="hs-13"></a>🛑 `HS-13` HDMI: +5V not supplied by the source, DDC without level translation and pull-ups, HPD without its pull · impl: mid (NET-7, NET-5, NET-10) · [📖](checks/hs.md#hs-13) <!-- 21 Oct 2026 -->
- <a id="hs-14"></a>🛑 `HS-14` DisplayPort: AUX pull-up / pull-down pair missing or on the wrong side, HPD, DP_PWR; DP++ dual-mode pins vs native DP · impl: mid (NET-7, NET-10) · [📖](checks/hs.md#hs-14) <!-- 21 Oct 2026 -->
- <a id="hs-15"></a>🛑 `HS-15` MIPI DSI / CSI lanes with series parts or protection in line: the lanes go straight to the connector · impl: easy (netlist) · [📖](checks/hs.md#hs-15) <!-- 21 Oct 2026 -->
- <a id="hs-16"></a>🛑 `HS-16` Reference clock coupling vs receiver type: HCSL needs 50 Ω to ground at the source, LVDS 100 Ω across at the receiver, LVPECL its bias; AC coupling only where the receiver self-biases · impl: hard (NET-7, NET-10) · [📖](checks/hs.md#hs-16) <!-- 21 Oct 2026 -->

### 📡 RF - RF
- <a id="rf-1"></a>⚠️ `RF-1` RF port without tuning network · impl: mid (NET-7) · [📖](checks/rf.md#rf-1) <!-- CHK-72 -->
- <a id="rf-2"></a>⚠️ `RF-2` RF net to connector without PI network · impl: mid (NET-7) · [📖](checks/rf.md#rf-2) <!-- CHK-180 -->
- <a id="rf-3"></a>💡 `RF-3` Suggest DC block caps between RF parts · impl: mid (NET-7) · [📖](checks/rf.md#rf-3) <!-- CHK-80 -->
- <a id="rf-4"></a>💡 `RF-4` Suggest an inline RF test connector · impl: mid (NET-7) · [📖](checks/rf.md#rf-4) <!-- CHK-81 -->
- <a id="rf-5"></a>🛑 `RF-5` RF switch, LNA or PA control pins floating · impl: mid (NET-7, NET-9) · [📖](checks/rf.md#rf-5) <!-- 21 Oct 2026 -->
- <a id="rf-6"></a>⚠️ `RF-6` Radio crystal or TCXO tolerance looser than the radio requires (BLE, LoRa, Wi-Fi each have their own) ≈ CLK-19 · impl: mid (CFG-5) · [📖](checks/rf.md#rf-6) <!-- 21 Oct 2026 -->
- <a id="rf-7"></a>🛑 `RF-7` Module ANT pin wired to an external antenna while the module's onboard antenna is selected (or both) · impl: mid (CFG-5) · [📖](checks/rf.md#rf-7) <!-- 21 Oct 2026 -->
- <a id="rf-8"></a>🛑 `RF-8` DC present on an RF port that cannot take it (bias tee without a block, a biased antenna into a DC-coupled input) ≈ RF-3 · impl: hard (NET-7, NET-10) · [📖](checks/rf.md#rf-8) <!-- 21 Oct 2026 -->
- <a id="rf-9"></a>⚠️ `RF-9` Radio with nothing on its RF port (no antenna, connector or load): transmitting into an open can damage the PA · impl: mid (NET-7, NET-10) · [📖](checks/rf.md#rf-9) <!-- 21 Oct 2026 -->

## Connectors

### 🔌 USB - USB and Type-C
- <a id="usb-1"></a>🛑 `USB-1` Type-C without 5k1 · impl: easy (NET-5, IMP-6) · [📖](checks/usb.md#usb-1) <!-- CHK-86 -->
- <a id="usb-2"></a>⚠️ `USB-2` No overvoltage protection on Type-C CC and SBU · impl: mid (NET-7, NET-10) · [📖](checks/usb.md#usb-2) <!-- CHK-55 -->
- <a id="usb-3"></a>⚠️ `USB-3` Directly exposed Type-C SBU: resistance must be over the "safe state" zSBUTermination > 950k · impl: mid (NET-7, IMP-6) · [📖](checks/usb.md#usb-3) <!-- CHK-13 -->
- <a id="usb-4"></a>⚠️ `USB-4` USB shield connected through L/R/C, or not connected · impl: easy (netlist) · [📖](checks/usb.md#usb-4) <!-- CHK-38 -->
- <a id="usb-5"></a>⚠️ `USB-5` USB device: more than 10 µF on VBUS (the host sees a short at attach) · impl: mid (NET-5, NET-10, IMP-6) · [📖](checks/usb.md#usb-5) <!-- CHK-46; split 21 Oct 2026: USB-6 -->
- <a id="usb-6"></a>⚠️ `USB-6` USB host port with less than 120 µF bulk on VBUS · impl: mid (NET-5, CFG-2, IMP-6) · [📖](checks/usb.md#usb-6) <!-- from USB-5 -->
- <a id="usb-7"></a>🛑 `USB-7` More than one Rd or Rp source on a CC line: a fixed Rd next to a PD controller's own Rd, or two Rp; two in parallel are out of spec and break PD · impl: mid (NET-7, NET-12, IMP-6) · [📖](checks/usb.md#usb-7) <!-- 21 Oct 2026; B153A -->
- <a id="usb-8"></a>🛑 `USB-8` CC resistors against the port role: Rp on a sink, Rd on a source, Rp on the receptacle side of a pass-through where the plug side needs it · impl: mid (NET-5, CFG-2, IMP-6) · [📖](checks/usb.md#usb-8) <!-- 21 Oct 2026; B153A -->
- <a id="usb-9"></a>🛑 `USB-9` CC1 and CC2 tied together on a receptacle: orientation detection and PD are gone, and a cable's VCONN lands on the device · impl: easy (netlist) · [📖](checks/usb.md#usb-9) <!-- 21 Oct 2026 -->
- <a id="usb-10"></a>🛑 `USB-10` Plug or captive cable with Rd on both CC pins: one is CC, the other is VCONN · impl: easy (IMP-6) · [📖](checks/usb.md#usb-10) <!-- 21 Oct 2026 -->
- <a id="usb-11"></a>🛑 `USB-11` Type-C receptacle with USB 2.0 wired to one orientation only: A6/B6 and A7/B7 must be joined · impl: easy (netlist) · [📖](checks/usb.md#usb-11) <!-- 21 Oct 2026 -->
- <a id="usb-12"></a>⚠️ `USB-12` Self-powered USB device without VBUS detection: it must not pull D+ up before VBUS is there · impl: hard (NET-8, NET-12) · [📖](checks/usb.md#usb-12) <!-- 21 Oct 2026 -->
- <a id="usb-13"></a>🛑 `USB-13` VBUS straight into a sense pin above its rating: a PD-capable VBUS reaches 20 V, a divider is needed · impl: mid (NET-7, CFG-2, RPT-6) · [📖](checks/usb.md#usb-13) <!-- 21 Oct 2026 -->
- <a id="usb-14"></a>🛑 `USB-14` USB hub configuration straps and upstream VBUS detect not wired per its datasheet (non-removable ports, self/bus powered) · impl: hard (NET-7, CFG-2, CFG-5) · [📖](checks/usb.md#usb-14) <!-- 21 Oct 2026 -->
- <a id="usb-15"></a>🛑 `USB-15` USB 3 SuperSpeed pairs on a Type-C receptacle without an orientation mux (or a PHY that handles both orientations) · impl: mid (NET-7, NET-10) · [📖](checks/usb.md#usb-15) <!-- 21 Oct 2026 -->
- <a id="usb-16"></a>🛑 `USB-16` Type-C receptacle with some VBUS or GND pins left unconnected · impl: easy (netlist) · [📖](checks/usb.md#usb-16) <!-- 21 Oct 2026 -->
- <a id="usb-17"></a>💡 `USB-17` Micro-B / Mini-B ID pin left floating on a port meant to be OTG, or wired on a device-only port · impl: mid (CFG-2, NET-12) · [📖](checks/usb.md#usb-17) <!-- 21 Oct 2026 -->
- <a id="usb-18"></a>⚠️ `USB-18` CC or SBU protection that clamps inside the normal range (CC sees up to 5.5 V in operation, must clamp before a 20 V VBUS short) or adds more capacitance than cReceiver allows ≈ USB-2 · impl: mid (NET-7, IMP-6) · [📖](checks/usb.md#usb-18) <!-- 21 Oct 2026 -->
- <a id="usb-19"></a>⚠️ `USB-19` Type-C receptacle variant (6, 12, 16 or 24 pin) without the pins the design needs: SBU or TX/RX for an alternate mode, SuperSpeed pairs for USB 3 · impl: mid (CFG-2, NET-7) · [📖](checks/usb.md#usb-19) <!-- 21 Oct 2026 -->

### 🔗 CONN - Connector protection and pinout
- <a id="conn-1"></a>⚠️ `CONN-1` No protection on lines going to connectors · impl: mid (CFG-2, NET-2) · [📖](checks/conn.md#conn-1) <!-- CHK-58 -->
- <a id="conn-2"></a>💡 `CONN-2` GPIOs going straight to connectors without series protection: 1k on long lines or better · impl: easy (netlist) · [📖](checks/conn.md#conn-2) <!-- CHK-73 -->
- <a id="conn-3"></a>🛑 `CONN-3` Unipolar protector on a bipolar signal (clamps the negative half) · impl: mid (CFG-2) · [📖](checks/conn.md#conn-3) <!-- CHK-65; split 21 Oct 2026: CONN-9 -->
- <a id="conn-4"></a>⚠️ `CONN-4` Connector without a ground pin · impl: easy (NET-5) · [📖](checks/conn.md#conn-4) <!-- CHK-93; split 21 Oct 2026: CONN-10 -->
- <a id="conn-5"></a>⚠️ `CONN-5` High speed on a connector: a ground for each pair · impl: mid (CFG-2, VIEW-10) · [📖](checks/conn.md#conn-5) <!-- CHK-94 -->
- <a id="conn-6"></a>💡 `CONN-6` Connectors with spare pins: suggest spreading power over more pins; use pin rating and currents if known · impl: mid (CFG-2) · [📖](checks/conn.md#conn-6) <!-- CHK-121 -->
- <a id="conn-7"></a>💡 `CONN-7` Reverse/flip insertion: with many similar signals, suggest a pinout that avoids power shorts · impl: mid (VIEW-11, CFG-2) · [📖](checks/conn.md#conn-7) <!-- CHK-96 -->
- <a id="conn-8"></a>🛑 `CONN-8` MicroHDMI wired with the full size HDMI pinout · impl: easy (netlist) · [📖](checks/conn.md#conn-8) <!-- CHK-102 -->
- <a id="conn-9"></a>⚠️ `CONN-9` Bipolar protector on a unipolar signal: clamps a diode drop later than a unipolar one would · impl: mid (CFG-2) · [📖](checks/conn.md#conn-9) <!-- from CONN-3 -->
- <a id="conn-10"></a>⚠️ `CONN-10` Many pins and few grounds on a connector: suggest more · impl: easy (NET-5) · [📖](checks/conn.md#conn-10) <!-- from CONN-4 -->
- <a id="conn-11"></a>⚠️ `CONN-11` Connector pin current rating below the current on that pin · impl: mid (CFG-2) · [📖](checks/conn.md#conn-11) <!-- 21 Oct 2026 -->
- <a id="conn-12"></a>⚠️ `CONN-12` Connector voltage rating below the net it carries · impl: mid (NET-5, CFG-2) · [📖](checks/conn.md#conn-12) <!-- 21 Oct 2026 -->
- <a id="conn-13"></a>🛑 `CONN-13` Mating pair pinout not mirrored: board-to-board plug and receptacle, or two boards joined by an FPC, both drawn from the same side · impl: hard (NET-13, VIEW-10) · [📖](checks/conn.md#conn-13) <!-- 21 Oct 2026 -->
- <a id="conn-14"></a>🛑 `CONN-14` FPC / FFC contact side (top or bottom) and pin order vs the cable: a same-side cable reverses the pin order · impl: hard (NET-13, VIEW-10) · [📖](checks/conn.md#conn-14) <!-- 21 Oct 2026 -->
- <a id="conn-15"></a>⚠️ `CONN-15` Debug header pinout against the standard it claims (ARM 10-pin SWD, 20-pin JTAG, Tag-Connect) ≈ LIB-2 · impl: mid (CFG-2) · [📖](checks/conn.md#conn-15) <!-- 21 Oct 2026 -->
- <a id="conn-16"></a>⚠️ `CONN-16` Connector shell or mounting pins left floating without a decision (ground, chassis, or isolated) · impl: easy (netlist) · [📖](checks/conn.md#conn-16) <!-- 21 Oct 2026 -->
- <a id="conn-17"></a>⚠️ `CONN-17` Two identical connectors on one board with incompatible pinouts: a cable fits the wrong one · impl: easy (NET-5) · [📖](checks/conn.md#conn-17) <!-- 21 Oct 2026 -->
- <a id="conn-18"></a>💡 `CONN-18` Power and ground on adjacent pins where a tilted insertion shorts them; prefer a signal or a gap between · impl: mid (VIEW-11, NET-5) · [📖](checks/conn.md#conn-18) <!-- 21 Oct 2026 -->
- <a id="conn-19"></a>⚠️ `CONN-19` High voltage and logic on adjacent pins of one connector: creepage and a wrong-pin fault reach the logic · impl: mid (NET-5, VIEW-11) · [📖](checks/conn.md#conn-19) <!-- 21 Oct 2026 -->
- <a id="conn-20"></a>⚠️ `CONN-20` Exposed metal a user can touch (connector shells, button frames, card housings, pot shafts) with no ESD path to chassis or ground · impl: judgement (CFG-2) · [📖](checks/conn.md#conn-20) <!-- 21 Oct 2026 -->

## Analog, ratings and errata

### 📈 ANA - Analog and sensing
- <a id="ana-1"></a>⚠️ `ANA-1` ADC input from a high impedance source without buffer · impl: mid (NET-7, NET-12, RPT-6) · [📖](checks/ana.md#ana-1) <!-- CHK-114 -->
- <a id="ana-2"></a>⚠️ `ANA-2` Sensitive signals (ADC, high impedance) next to high speed ones on a part (by pin or ball number) · impl: mid (NET-7, NET-12) · [📖](checks/ana.md#ana-2) <!-- CHK-113 -->
- <a id="ana-3"></a>💡 `ANA-3` Shunt lines to the amplifier without a filter · impl: easy (netlist) · [📖](checks/ana.md#ana-3) <!-- CHK-98 -->
- <a id="ana-4"></a>💡 `ANA-4` Current sense resistor straight to ground or power: suggest net ties · impl: easy (NET-5) · [📖](checks/ana.md#ana-4) <!-- CHK-128 -->
- <a id="ana-5"></a>⚠️ `ANA-5` Reference or bias input (REF, VBIAS) fed from a divider: divider impedance vs the pin's input current shifts the reference · impl: mid (NET-7, RPT-6) · [📖](checks/ana.md#ana-5) <!-- 21 Oct 2026; B153A INA181 -->
- <a id="ana-6"></a>🛑 `ANA-6` Op-amp input common-mode range vs the signal: a signal at a rail on a non-rail-to-rail input · impl: hard (NET-7, NET-10, CFG-2, RPT-10) · [📖](checks/ana.md#ana-6) <!-- 21 Oct 2026 -->
- <a id="ana-7"></a>⚠️ `ANA-7` Op-amp output swing vs what the next stage needs: an output expected at 0 V on a single-supply part that stops 50 mV short · impl: hard (NET-7, NET-10, RPT-10) · [📖](checks/ana.md#ana-7) <!-- 21 Oct 2026 -->
- <a id="ana-8"></a>⚠️ `ANA-8` Op-amp input driven beyond its supply with no series resistance: phase reversal on older parts, damage on all · impl: mid (NET-5, CFG-2) · [📖](checks/ana.md#ana-8) <!-- 21 Oct 2026 -->
- <a id="ana-9"></a>⚠️ `ANA-9` Op-amp driving a capacitive load (an ADC input, a long line) without an isolation resistor · impl: mid (NET-7) · [📖](checks/ana.md#ana-9) <!-- 21 Oct 2026 -->
- <a id="ana-10"></a>🛑 `ANA-10` Op-amp with no feedback path, or feedback into the non-inverting input (a comparator by accident) · impl: easy (netlist) · [📖](checks/ana.md#ana-10) <!-- 21 Oct 2026 -->
- <a id="ana-11"></a>⚠️ `ANA-11` Unused op-amp channel left floating: wire it as a buffer to mid-rail or ground · impl: easy (netlist) · [📖](checks/ana.md#ana-11) <!-- 21 Oct 2026 -->
- <a id="ana-12"></a>💡 `ANA-12` Comparator on a slow or noisy signal without hysteresis · impl: easy (netlist) · [📖](checks/ana.md#ana-12) <!-- 21 Oct 2026 -->
- <a id="ana-13"></a>🛑 `ANA-13` ADC input range exceeded without scaling (a 5 V signal into a 3.3 V ADC) ≈ LVL-10 · impl: mid (NET-5, NET-12, RPT-6, CFG-2) · [📖](checks/ana.md#ana-13) <!-- 21 Oct 2026 -->
- <a id="ana-14"></a>🛑 `ANA-14` Current sense amplifier common-mode rating below the bus it sits on (a 26 V part on 48 V) · impl: mid (NET-5, NET-7) · [📖](checks/ana.md#ana-14) <!-- 21 Oct 2026 -->
- <a id="ana-15"></a>⚠️ `ANA-15` Sense gain times shunt drop saturates below the current that must be measured, or wastes most of the ADC range · impl: mid (RPT-11, CFG-2) · [📖](checks/ana.md#ana-15) <!-- 21 Oct 2026 -->
- <a id="ana-16"></a>⚠️ `ANA-16` Gain or filter resistors far outside the sane range for the op-amp (10 Ω loading the output, 10 MΩ against bias current) · impl: easy (netlist) · [📖](checks/ana.md#ana-16) <!-- 21 Oct 2026 -->
- <a id="ana-17"></a>💡 `ANA-17` Thermistor or sensor divider referenced to VDD while the ADC uses a separate reference (or the reverse): the ratio drifts with the rail · impl: mid (NET-5, NET-12, IMP-9) · [📖](checks/ana.md#ana-17) <!-- 21 Oct 2026 -->
- <a id="ana-18"></a>💡 `ANA-18` No anti-aliasing filter before an ADC on a noisy or fast source · impl: judgement (NET-12) · [📖](checks/ana.md#ana-18) <!-- 21 Oct 2026 -->
- <a id="ana-19"></a>⚠️ `ANA-19` DAC output into a load it cannot drive (low impedance, capacitance) without a buffer · impl: mid (NET-12, IMP-9) · [📖](checks/ana.md#ana-19) <!-- 21 Oct 2026 -->
- <a id="ana-20"></a>🛑 `ANA-20` Analog input (ADC, comparator, op-amp) above VDDA + 0.3 V, also on a 5 V-tolerant pin: tolerance is for the digital input, the analog path has its own diodes ≈ LVL-10, ANA-13 · impl: mid (NET-5, NET-7, NET-12, CFG-2) · [📖](checks/ana.md#ana-20) <!-- 21 Oct 2026 -->
- <a id="ana-21"></a>🛑 `ANA-21` AC-coupled (or transformer-coupled) amplifier input with no DC return path for the bias current: the coupling capacitor charges until the amplifier saturates, hours later · impl: easy (netlist) · [📖](checks/ana.md#ana-21) <!-- 21 Oct 2026; ADI AN-937 -->
- <a id="ana-22"></a>💡 `ANA-22` Mid-rail bias divider without a capacitor to ground: the supply noise rides on the bias · impl: easy (NET-5) · [📖](checks/ana.md#ana-22) <!-- 21 Oct 2026 -->
- <a id="ana-23"></a>⚠️ `ANA-23` Sense path from a shunt through a copper trace or a part with high TCR (copper is 3900 ppm/°C), or sense lines taken from the power pads instead of Kelvin points ≈ ANA-4 · impl: hard (NET-10, TOOL-2) · [📖](checks/ana.md#ana-23) <!-- 21 Oct 2026 -->

### 🌡️ RATE - Ratings and derating
- <a id="rate-1"></a>⚠️ `RATE-1` Capacitor voltage rating margins · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-1) <!-- CHK-29 -->
- <a id="rate-2"></a>⚠️ `RATE-2` Resistor power rating · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-2) <!-- CHK-6; split 21 Oct 2026: RATE-9 -->
- <a id="rate-3"></a>⚠️ `RATE-3` Shunt power ratings · impl: mid (CFG-2, CFG-5) · [📖](checks/rate.md#rate-3) <!-- CHK-7 -->
- <a id="rate-4"></a>⚠️ `RATE-4` MOSFET Vds rating · impl: mid (NET-5, CFG-5, LIB-3) · [📖](checks/rate.md#rate-4) <!-- CHK-217 -->
- <a id="rate-5"></a>⚠️ `RATE-5` MOSFET power dissipation estimate · impl: hard (CFG-2, CFG-5, RPT-4) · [📖](checks/rate.md#rate-5) <!-- CHK-8 -->
- <a id="rate-6"></a>⚠️ `RATE-6` LDO current rating, when power draw is known · impl: mid (RPT-2, CFG-2, CFG-5) · [📖](checks/rate.md#rate-6) <!-- CHK-16 -->
- <a id="rate-7"></a>⚠️ `RATE-7` LED brightness estimate: warn if too bright · impl: mid (NET-5, CFG-5) · [📖](checks/rate.md#rate-7) <!-- CHK-185; split 21 Oct 2026: RATE-10 -->
- <a id="rate-8"></a>⚠️ `RATE-8` Temperature ranges that make no sense, e.g. X7R caps but 85 °C ICs · impl: mid (CFG-5, CFG-2) · [📖](checks/rate.md#rate-8) <!-- CHK-47 -->
- <a id="rate-9"></a>⚠️ `RATE-9` Resistor voltage rating (high voltage dividers, a single 0402 across 100 V) · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-9) <!-- from RATE-2 -->
- <a id="rate-10"></a>⚠️ `RATE-10` LED current above its maximum forward current · impl: mid (NET-5, CFG-5) · [📖](checks/rate.md#rate-10) <!-- from RATE-7 -->
- <a id="rate-11"></a>🛑 `RATE-11` Supply pin voltage outside the part's operating range or above its absolute maximum (a 3.3 V rail into a 1.8 V-only part) · impl: mid (NET-5, NET-7, CFG-5) · [📖](checks/rate.md#rate-11) <!-- 21 Oct 2026 -->
- <a id="rate-12"></a>🛑 `RATE-12` Regulator input above its maximum Vin (24 V into a 6 V LDO) · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-12) <!-- 21 Oct 2026 -->
- <a id="rate-13"></a>⚠️ `RATE-13` Diode forward current or reverse voltage vs the circuit (a 20 V Schottky on a 24 V rail, a 200 mA diode in a 1 A path) · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-13) <!-- 21 Oct 2026 -->
- <a id="rate-14"></a>⚠️ `RATE-14` TVS clamping voltage above the absolute maximum of the pin it protects ≈ PIN-9 · impl: mid (NET-7, CFG-5) · [📖](checks/rate.md#rate-14) <!-- 21 Oct 2026 -->
- <a id="rate-15"></a>🛑 `RATE-15` Inductor saturation current below the peak current of its converter (Iout plus half the ripple) · impl: mid (CFG-2, CFG-5, RPT-3) · [📖](checks/rate.md#rate-15) <!-- 21 Oct 2026 -->
- <a id="rate-16"></a>⚠️ `RATE-16` Ceramic capacitor DC-bias derating ignored: effective capacitance at the working voltage below what the circuit needs · impl: mid (CFG-5, RPT-1) · [📖](checks/rate.md#rate-16) <!-- 21 Oct 2026 -->
- <a id="rate-17"></a>⚠️ `RATE-17` Electrolytic or tantalum without voltage derating (50 % for tantalum), or tantalum on a low-impedance rail without surge limiting · impl: mid (NET-5, CFG-5) · [📖](checks/rate.md#rate-17) <!-- 21 Oct 2026 -->
- <a id="rate-18"></a>⚠️ `RATE-18` Relay, solenoid or motor current above its driver's rating (BJT Ic, driver IC, GPIO) · impl: mid (CFG-5, CFG-2, NET-10) · [📖](checks/rate.md#rate-18) <!-- 21 Oct 2026 -->
- <a id="rate-19"></a>⚠️ `RATE-19` Parts on a PD-capable VBUS rated below 22 V: the port can negotiate 20 V · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-19) <!-- 21 Oct 2026; B200A -->
- <a id="rate-20"></a>⚠️ `RATE-20` Bus termination resistors not rated for a bus fault (RS-485 or CAN shorted to the supply) · impl: mid (CFG-2, CFG-5) · [📖](checks/rate.md#rate-20) <!-- 21 Oct 2026; B125B -->
- <a id="rate-21"></a>⚠️ `RATE-21` Linear regulator dissipation (Vin − Vout) × Iout vs its package (a SOT-23 at 200 °C/W does not do 0.3 W) ≈ RATE-6 · impl: mid (RPT-2, CFG-2, CFG-5) · [📖](checks/rate.md#rate-21) <!-- 21 Oct 2026; B200A -->
- <a id="rate-22"></a>⚠️ `RATE-22` Zener power at the maximum current its series resistor allows · impl: mid (NET-5, CFG-2, CFG-5) · [📖](checks/rate.md#rate-22) <!-- 21 Oct 2026 -->
- <a id="rate-23"></a>⚠️ `RATE-23` Optocoupler LED current vs its CTR at the load: not enough drive, or over its maximum · impl: mid (NET-5, NET-7, CFG-5) · [📖](checks/rate.md#rate-23) <!-- 21 Oct 2026 -->
- <a id="rate-24"></a>⚠️ `RATE-24` LED reverse voltage exceeded (LEDs across AC or a reversed supply) · impl: mid (NET-5, CFG-5) · [📖](checks/rate.md#rate-24) <!-- 21 Oct 2026 -->

### 📋 ERR - Known errata
- <a id="err-1"></a>⚠️ `ERR-1` Part with a known hardware erratum that touches this circuit (by MPN, from a database): RP2350-E9 input pull-down latch needs an external 8.2k, STM32 I2C v1 BUSY flag stuck after a glitch, BCM2835 clock stretching, STM32F103 USB and CAN sharing one SRAM · impl: mid (CFG-5, NET-7) · [📖](checks/err.md#err-1) <!-- 21 Oct 2026; B200A -->

## Debug and test

### 🐞 DBG - Debug access, indicators, rework options
- <a id="dbg-1"></a>🛑 `DBG-1` SWD/JTAG not connected · impl: easy (netlist (`NET-12` for symbols that name only the port pin)) · [📖](checks/dbg.md#dbg-1) <!-- CHK-48 -->
- <a id="dbg-2"></a>💡 `DBG-2` No series resistors on an obvious debug header (e.g. SWD) · impl: easy (netlist) · [📖](checks/dbg.md#dbg-2) <!-- CHK-84 -->
- <a id="dbg-3"></a>💡 `DBG-3` No LEDs on the board at all: suggest at least one for debug · impl: easy (netlist) · [📖](checks/dbg.md#dbg-3) <!-- CHK-85 -->
- <a id="dbg-4"></a>💡 `DBG-4` No power rail LEDs · impl: easy (`NET-5`) · [📖](checks/dbg.md#dbg-4) <!-- CHK-56 -->
- <a id="dbg-5"></a>💡 `DBG-5` No error flag LEDs when flags aren't routed to IOs · impl: mid (`NET-7`) · [📖](checks/dbg.md#dbg-5) <!-- CHK-134 -->
- <a id="dbg-6"></a>💡 `DBG-6` Power rails not monitored by an ADC · impl: mid (`NET-5`, `NET-10`, `NET-12`) · [📖](checks/dbg.md#dbg-6) <!-- CHK-51 -->
- <a id="dbg-7"></a>💡 `DBG-7` Suggest DNM pull-up and down on EN pins · impl: easy (netlist) · [📖](checks/dbg.md#dbg-7) <!-- CHK-17 -->
- <a id="dbg-8"></a>💡 `DBG-8` Suggest DNM pull-ups on open drain pins; test point detector should highlight those nets · impl: easy (netlist) · [📖](checks/dbg.md#dbg-8) <!-- CHK-42 -->
- <a id="dbg-9"></a>💡 `DBG-9` Suggest DNM 0R bypasses on power switches · impl: mid (`NET-5`) · [📖](checks/dbg.md#dbg-9) <!-- CHK-37 -->
- <a id="dbg-10"></a>💡 `DBG-10` Many things on one power net: suggest splitting with a 0R before each power input · impl: easy (`NET-5`) · [📖](checks/dbg.md#dbg-10) <!-- CHK-34 -->
- <a id="dbg-11"></a>💡 `DBG-11` Suggest 0R on USART TX, RX, CTS, RTS and SPI MOSI, MISO, as they're often wrong · impl: easy (netlist (`NET-12` for pins named by port only)) · [📖](checks/dbg.md#dbg-11) <!-- CHK-45 -->
- <a id="dbg-12"></a>💡 `DBG-12` Spot potentially useful 0R series resistors (AI?) · impl: judgement (netlist (candidates only)) · [📖](checks/dbg.md#dbg-12) <!-- CHK-10 -->
- <a id="dbg-13"></a>💡 `DBG-13` No early power-fail signal on a board that writes storage (SD, eMMC, flash): a comparator on the input ahead of the rail lets firmware stop writing before the rail drops · impl: mid (`NET-5`, `NET-7`) · [📖](checks/dbg.md#dbg-13) <!-- 21 Oct 2026; B125B -->
- <a id="dbg-14"></a>💡 `DBG-14` No UART console reachable on a header or test points · impl: easy (netlist (`NET-12` for pins named by port only)) · [📖](checks/dbg.md#dbg-14) <!-- 21 Oct 2026 -->
- <a id="dbg-15"></a>💡 `DBG-15` Board revision and variant not readable by firmware (ID resistor divider on an ADC pin, straps): the firmware cannot tell boards apart · impl: judgement (`NET-12` (candidates: ADC pins with a fixed divider, strap pins)) · [📖](checks/dbg.md#dbg-15) <!-- 21 Oct 2026; B153A -->
- <a id="dbg-16"></a>💡 `DBG-16` No firmware-driven LED (heartbeat), only power LEDs ≈ DBG-3 · impl: easy (netlist (`NET-10` for LEDs driven through a transistor)) · [📖](checks/dbg.md#dbg-16) <!-- 21 Oct 2026 -->
- <a id="dbg-17"></a>💡 `DBG-17` SWD header without NRST (and SWO): the probe cannot reset a stuck target or trace · impl: easy (netlist) · [📖](checks/dbg.md#dbg-17) <!-- 21 Oct 2026 -->
- <a id="dbg-18"></a>💡 `DBG-18` I2C or SPI buses not reachable at a header or test points for a bus analyser · impl: easy (`NET-4`) · [📖](checks/dbg.md#dbg-18) <!-- 21 Oct 2026 -->
- <a id="dbg-19"></a>⚠️ `DBG-19` Debug header VTref not wired straight to the target rail (a series resistor, or not connected): the probe senses level and power through it; and a header VCC pin able to source current into the rail back-powers the board to a half-alive state · impl: mid (`NET-5`, `NET-7`) · [📖](checks/dbg.md#dbg-19) <!-- 21 Oct 2026 -->
- <a id="dbg-20"></a>💡 `DBG-20` No hardware way to reset or power-cycle bus peripherals after an MCU-only reset: a stuck I2C slave stays stuck · impl: hard (`NET-8`, `NET-10`) · [📖](checks/dbg.md#dbg-20) <!-- 21 Oct 2026 -->
- <a id="dbg-21"></a>⚠️ `DBG-21` JTAG pulls: TCK without a pull-down, TMS and TDI without pull-ups, nTRST floating where the header has no nTRST pin (the debug logic resets) · impl: mid (`NET-7`) · [📖](checks/dbg.md#dbg-21) <!-- 21 Oct 2026 -->
- <a id="dbg-22"></a>💡 `DBG-22` SWDIO without a pull-up or SWCLK without a pull-down (10k-100k): noise on an unplugged header enters the debug port ≈ DBG-2 · impl: mid (`NET-7`) · [📖](checks/dbg.md#dbg-22) <!-- 21 Oct 2026; B153A SWCLK -->
- <a id="dbg-23"></a>💡 `DBG-23` Spare GPIOs not brought to pads or a header for bodges and debug · impl: mid (`NET-7`, `NET-12`) · [📖](checks/dbg.md#dbg-23) <!-- 21 Oct 2026 -->
- <a id="dbg-24"></a>💡 `DBG-24` No non-volatile or backup storage for reset cause and crash logs (backup registers, FRAM, a few flash pages reserved) · impl: judgement (netlist (candidates: no FRAM/EEPROM, no backup supply)) · [📖](checks/dbg.md#dbg-24) <!-- 21 Oct 2026 -->

### 🧪 TEST - Test points and testability
- <a id="test-1"></a>💡 `TEST-1` Unconnected pins without test points (optional report table) · impl: easy (netlist) · [📖](checks/test.md#test-1) <!-- CHK-11 -->
- <a id="test-2"></a>💡 `TEST-2` Several test points on one net (incl. through 0R), unless top and bottom · impl: easy (netlist, `IMP-5`) · [📖](checks/test.md#test-2) <!-- CHK-153 -->
- <a id="test-3"></a>💡 `TEST-3` All harness connector pins self-testable · impl: hard (`NET-10`, `NET-12`, `CFG-2`) · [📖](checks/test.md#test-3) <!-- CHK-225 -->
- <a id="test-4"></a>💡 `TEST-4` Power rail or ground without a test point · impl: easy (`NET-5`) · [📖](checks/test.md#test-4) <!-- 21 Oct 2026 -->
- <a id="test-5"></a>💡 `TEST-5` Bring-up bus (UART, I2C, SPI, SWD) without test points · impl: easy (`NET-4` (`NET-12` for pins named by port only)) · [📖](checks/test.md#test-5) <!-- 21 Oct 2026 -->
- <a id="test-6"></a>💡 `TEST-6` Clock, reset or enable net without a test point · impl: mid (`NET-7`) · [📖](checks/test.md#test-6) <!-- 21 Oct 2026 -->
- <a id="test-7"></a>💡 `TEST-7` Test points too small or too close for the probe or fixture they are meant for (layout) · impl: hard (a layout import (none yet), `CFG-2`) · [📖](checks/test.md#test-7) <!-- 21 Oct 2026 -->

## Documentation and data

### 🧾 BOM - BOM hygiene
- <a id="bom-1"></a>💡 `BOM-1` Resistors (or capacitors) with values a step apart that could be one value · impl: easy (`IMP-6`) · [📖](checks/bom.md#bom-1) <!-- CHK-23; split 21 Oct 2026: BOM-11 -->
- <a id="bom-2"></a>💡 `BOM-2` Capacitor voltage, size and dielectric report: find odd ones · impl: mid (`NET-5`, MPN decoding (no FEATURES id yet)) · [📖](checks/bom.md#bom-2) <!-- CHK-25 -->
- <a id="bom-3"></a>💡 `BOM-3` Big and small connectors, big and small parts, SMT and THT mixed and similar · impl: easy (netlist) · [📖](checks/bom.md#bom-3) <!-- CHK-141 -->
- <a id="bom-4"></a>💡 `BOM-4` Propose consolidating identical resistors into resistor networks · impl: easy (`IMP-6`) · [📖](checks/bom.md#bom-4) <!-- CHK-234 -->
- <a id="bom-5"></a>🛑 `BOM-5` No TODOs or "calculate later" values left ≈ BOM-6 · impl: easy (IMP-6) · [📖](checks/bom.md#bom-5) <!-- CHK-230 -->
- <a id="bom-6"></a>🛑 `BOM-6` "Calculate later" part mounted · impl: easy (IMP-6) · [📖](checks/bom.md#bom-6) <!-- CHK-155 -->
- <a id="bom-7"></a>💡 `BOM-7` Depopulate spare and debug parts for release · impl: judgement (netlist (candidates: debug headers, bypass 0 Ω, test LEDs)) · [📖](checks/bom.md#bom-7) <!-- CHK-224 -->
- <a id="bom-8"></a>🛑 `BOM-8` LED swapped (polarity) · impl: easy (`NET-5`) · [📖](checks/bom.md#bom-8) <!-- CHK-200 -->
- <a id="bom-9"></a>🛑 `BOM-9` Part without a value · impl: easy (netlist) · [📖](checks/bom.md#bom-9) <!-- new 5 Oct 2026, found in code -->
- <a id="bom-10"></a>⚠️ `BOM-10` Value that doesn't parse or isn't a standard series value · impl: easy (IMP-6) · [📖](checks/bom.md#bom-10) <!-- new 5 Oct 2026, found in code -->
- <a id="bom-11"></a>💡 `BOM-11` Same value in two packages · impl: easy (netlist) · [📖](checks/bom.md#bom-11) <!-- from BOM-1 -->
- <a id="bom-12"></a>🛑 `BOM-12` Value field vs the MPN's parameters: resistance, capacitance, voltage, tolerance or package decoded from the part number disagree with the value or footprint (B200A R201/R203 13k3 for 13k7, B153A R219/R222 13k3 for 3k3; B135B C405, C509 MPN errors) · impl: mid (MPN decoding (no FEATURES id yet), `IMP-6`) · [📖](checks/bom.md#bom-12) <!-- 21 Oct 2026; B153A, B200A, B135B -->
- <a id="bom-13"></a>⚠️ `BOM-13` Variant: a part's DNP flag disagrees with its per-variant fields, or a variant BOM is built from the base DNP flags · impl: mid (`IMP-1` (variant data from the schematic), `IMP-8`) · [📖](checks/bom.md#bom-13) <!-- 21 Oct 2026; B153A -->
- <a id="bom-14"></a>⚠️ `BOM-14` Mounted part without an MPN · impl: easy (netlist) · [📖](checks/bom.md#bom-14) <!-- 21 Oct 2026 -->
- <a id="bom-15"></a>💡 `BOM-15` MPN not recommended for new designs, obsolete or single-source (DPO2036) · impl: mid (a part lifecycle source (no FEATURES id yet)) · [📖](checks/bom.md#bom-15) <!-- 21 Oct 2026; B153A -->
- <a id="bom-16"></a>💡 `BOM-16` Tolerance missing or loose on a part in a precision role (regulator feedback, ADC reference divider, sense gain) · impl: mid (`NET-7`, `RPT-6`, MPN decoding) · [📖](checks/bom.md#bom-16) <!-- 21 Oct 2026 -->
- <a id="bom-17"></a>🛑 `BOM-17` One MPN with two different values (a copy-paste part) · impl: easy (netlist) · [📖](checks/bom.md#bom-17) <!-- 21 Oct 2026 -->
- <a id="bom-18"></a>💡 `BOM-18` Parts of the vendor's typical application or reference design left out without a stated reason · impl: judgement (part data (typical application circuits; no FEATURES id yet)) · [📖](checks/bom.md#bom-18) <!-- 21 Oct 2026 -->

### 🏷️ NAME - Naming and conventions
- <a id="name-1"></a>💡 `NAME-1` Inconsistent naming of inverted signals: suggest one notation · impl: easy (netlist) · [📖](checks/name.md#name-1) <!-- CHK-63 -->
- <a id="name-2"></a>💡 `NAME-2` Inconsistent naming of differential pairs: suggest one notation · impl: easy (netlist) · [📖](checks/name.md#name-2) <!-- CHK-64 -->
- <a id="name-3"></a>💡 `NAME-3` Power net names without voltage when it's clearly fixed (DC-DC or LDO output, USB 2, Type-C with 5k1) · impl: mid (`NET-5` (sources, missing today), MPN decoding for fixed regulators) · [📖](checks/name.md#name-3) <!-- CHK-77 -->
- <a id="name-4"></a>💡 `NAME-4` UART: suggest `X_TX_Y_RX` style names · impl: easy (netlist (`NET-12` for pins named by port only)) · [📖](checks/name.md#name-4) <!-- CHK-165 -->
- <a id="name-5"></a>💡 `NAME-5` Gaps in designator series on rev A or standalone ≈ NAME-6 · impl: easy (netlist) · [📖](checks/name.md#name-5) <!-- CHK-49 -->
- <a id="name-6"></a>💡 `NAME-6` Suggest per page designators; highlight missing ones · impl: easy (netlist (sheet path per component)) · [📖](checks/name.md#name-6) <!-- CHK-54 -->
- <a id="name-7"></a>💡 `NAME-7` No nets found in strict mode: suggest `+xVy` power names and non-strict mode · impl: easy (`NET-5`) · [📖](checks/name.md#name-7) <!-- CHK-204 -->
- <a id="name-8"></a>⚠️ `NAME-8` Fixed-voltage name on a net that is not fixed: a header `+5V` that is the receptacle VBUS, 5 to 20 V after PD ≈ RAIL-3 · impl: mid (`NET-5`, `CFG-2`) · [📖](checks/name.md#name-8) <!-- 21 Oct 2026; B153A -->
- <a id="name-9"></a>💡 `NAME-9` IO numbering out of order against the part's pins (IO4 on GPIO17 while IO5 is on GPIO16) · impl: easy (netlist) · [📖](checks/name.md#name-9) <!-- 21 Oct 2026; B200A -->
- <a id="name-10"></a>💡 `NAME-10` Active-low signal without a marker in its name (`_N`, `#`, `~`): reviewers and LOGIC-8 cannot see the polarity · impl: easy (netlist (`NET-7` for pins whose names carry no marker)) · [📖](checks/name.md#name-10) <!-- 21 Oct 2026 -->
- <a id="name-11"></a>💡 `NAME-11` Negative rail named without its sign (`5V_NEG` vs `-5V`), or with a sign the tool does not parse · impl: easy (`NET-5`) · [📖](checks/name.md#name-11) <!-- 21 Oct 2026 -->
- <a id="name-12"></a>🛑 `NAME-12` Unannotated or duplicate reference designators · impl: easy (netlist) · [📖](checks/name.md#name-12) <!-- 21 Oct 2026 -->

### 📚 LIB - Library, symbols and footprints
- <a id="lib-1"></a>⚠️ `LIB-1` Symbol pin names differ from the same part elsewhere in the database · impl: mid (CFG-5, NET-7) · [📖](checks/lib.md#lib-1) <!-- CHK-78; split 21 Oct 2026: LIB-4, LIB-5 -->
- <a id="lib-2"></a>⚠️ `LIB-2` Global pinout database for common connectors, check against it · impl: mid (CFG-5, CFG-2) · [📖](checks/lib.md#lib-2) <!-- CHK-116 -->
- <a id="lib-3"></a>⚠️ `LIB-3` MOSFET symbol pins not named G/D/S (other checks need them) · impl: easy (netlist) · [📖](checks/lib.md#lib-3) <!-- new 5 Oct 2026, found in code -->
- <a id="lib-4"></a>⚠️ `LIB-4` Pin electrical type wrong (a power pin as passive, an output as input): the type-based checks go blind · impl: mid (NET-7, IMP-9, IMP-10) · [📖](checks/lib.md#lib-4) <!-- from LIB-1 -->
- <a id="lib-5"></a>⚠️ `LIB-5` Pins missing or duplicated against the package's pin count · impl: mid (netlist, CFG-5) · [📖](checks/lib.md#lib-5) <!-- from LIB-1 -->
- <a id="lib-6"></a>🛑 `LIB-6` Footprint does not match the MPN's package (an inductor on the wrong footprint, a diode in another case size) · impl: mid (CFG-5) · [📖](checks/lib.md#lib-6) <!-- 21 Oct 2026; B153A -->
- <a id="lib-7"></a>⚠️ `LIB-7` Symbol pin name lacks a shared special function the pin has (`PF2` for a pin that is `PF2-NRST`): the reviewer and MUX-4 never see it · impl: mid (NET-12, IMP-9) · [📖](checks/lib.md#lib-7) <!-- 21 Oct 2026; B153A -->
- <a id="lib-8"></a>⚠️ `LIB-8` Footprint pads too small or too large for the part's terminals (poor wetting, tombstoning, bridging) · impl: hard (TOOL-3, CFG-5) · [📖](checks/lib.md#lib-8) <!-- 21 Oct 2026; B153A D801/D802 -->
- <a id="lib-9"></a>💡 `LIB-9` Footprint without a pin-1 or polarity mark on silk and fab layers · impl: hard (TOOL-3) · [📖](checks/lib.md#lib-9) <!-- 21 Oct 2026; B125A, B135B, B153A -->
- <a id="lib-10"></a>🛑 `LIB-10` Exposed or thermal pad not connected (where it is the ground or substrate), or connected when the datasheet says float · impl: mid (NET-7, CFG-5) · [📖](checks/lib.md#lib-10) <!-- 21 Oct 2026 -->
- <a id="lib-11"></a>🛑 `LIB-11` Footprint pad to symbol pin mapping wrong for a polarized part (anode and cathode, pin 1 of a diode package) ≈ BOM-8 · impl: mid (CFG-5, NET-7) · [📖](checks/lib.md#lib-11) <!-- 21 Oct 2026 -->
- <a id="lib-12"></a>💡 `LIB-12` Footprint without a courtyard ≈ LAY-3 · impl: mid (TOOL-3) · [📖](checks/lib.md#lib-12) <!-- 21 Oct 2026 -->
- <a id="lib-13"></a>⚠️ `LIB-13` Units of one multi-unit symbol with different footprints, values or power nets · impl: mid (TOOL-3, IMP-1) · [📖](checks/lib.md#lib-13) <!-- 21 Oct 2026 -->
- <a id="lib-14"></a>⚠️ `LIB-14` Unit of a multi-unit symbol not placed: an unused gate or amplifier whose inputs float ≈ ANA-11 · impl: easy (IMP-1) · [📖](checks/lib.md#lib-14) <!-- 21 Oct 2026 -->
- <a id="lib-15"></a>⚠️ `LIB-15` Symbol with no power pins at all (hidden, or forgotten): the part is never checked for power ≈ WIRE-10 · impl: easy (netlist) · [📖](checks/lib.md#lib-15) <!-- 21 Oct 2026 -->
- <a id="lib-16"></a>🛑 `LIB-16` Symbol pin without a footprint pad, pad without a pin, or a pin mapped to a pad twice · impl: mid (TOOL-3, IMP-1) · [📖](checks/lib.md#lib-16) <!-- 21 Oct 2026 -->
- <a id="lib-17"></a>⚠️ `LIB-17` Placed symbol differs from its library copy, or its footprint link is broken or fails the symbol's footprint filter · impl: mid (TOOL-3) · [📖](checks/lib.md#lib-17) <!-- 21 Oct 2026 -->
- <a id="lib-18"></a>🛑 `LIB-18` Footprint built from a bottom-view drawing: the pinout is mirrored (regulators, transistors and connectors most often) · impl: judgement (TOOL-3) · [📖](checks/lib.md#lib-18) <!-- 21 Oct 2026 -->

## Layout

### 📐 LAY - Layout, placement and fabrication
- <a id="lay-1"></a>🛑 `LAY-1` Schematic and layout match · impl: mid (TOOL-2, TOOL-3) · [📖](checks/lay.md#lay-1) <!-- CHK-229 -->
- <a id="lay-2"></a>⚠️ `LAY-2` Parts in component areas, test points in designated areas, nothing in forbidden areas · impl: hard (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-2) <!-- CHK-154 -->
- <a id="lay-3"></a>⚠️ `LAY-3` Auto courtyard check and fix · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-3) <!-- CHK-219 -->

    Check that cortyard is equidistant from 3D boundary.

    Add a proper one if requested.

- <a id="lay-4"></a>💡 `LAY-4` Check stackup · impl: mid (TOOL-3) · [📖](checks/lay.md#lay-4) <!-- CHK-232 -->

    SGGS for 4L recommend

- <a id="lay-5"></a>⚠️ `LAY-5` Stitching vias · impl: hard (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-5) <!-- CHK-233 -->

    6 vias in each wavelength of highest frequency signal to not excite the cavity

    [https://youtu.be/kdCJxdR7L_I?si=rVgj4WJCnR0p1eez](https://youtu.be/kdCJxdR7L_I?si=rVgj4WJCnR0p1eez)

- <a id="lay-6"></a>⚠️ `LAY-6` Polarized part or IC without a pin-1 / polarity mark on the silk · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-6) <!-- 21 Oct 2026; B125A, B135B, B153A -->
- <a id="lay-7"></a>💡 `LAY-7` Test points without their net name on the silk · impl: mid (TOOL-3) · [📖](checks/lay.md#lay-7) <!-- 21 Oct 2026; B153A -->
- <a id="lay-8"></a>⚠️ `LAY-8` No fiducials (three global, local ones at fine-pitch parts) on a machine-assembled board · impl: mid (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-8) <!-- 21 Oct 2026 -->
- <a id="lay-9"></a>💡 `LAY-9` No tooling holes or panel rails planned · impl: mid (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-9) <!-- 21 Oct 2026 -->
- <a id="lay-10"></a>💡 `LAY-10` Large pads (exposed pads, screw pads, big THT) with a full paste aperture: window-pane it · impl: mid (TOOL-3) · [📖](checks/lay.md#lay-10) <!-- 21 Oct 2026; B135B, B153A, B156A -->
- <a id="lay-11"></a>💡 `LAY-11` Small parts (0402 and under) packed too closely for hand assembly where hand assembly is planned · impl: hard (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-11) <!-- 21 Oct 2026; B125A -->
- <a id="lay-12"></a>⚠️ `LAY-12` Big THT pads or connector pins into a plane without thermal relief: hard to solder · impl: mid (TOOL-3) · [📖](checks/lay.md#lay-12) <!-- 21 Oct 2026; B135B -->
- <a id="lay-13"></a>🛑 `LAY-13` Track barely touching a via or pad it should connect to (DRC passes, the connection is a sliver) · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-13) <!-- 21 Oct 2026; P2503#6 -->
- <a id="lay-14"></a>🛑 `LAY-14` Creepage and clearance below the requirement across high voltage and isolation barriers · impl: hard (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-14) <!-- 21 Oct 2026 -->
- <a id="lay-15"></a>⚠️ `LAY-15` Mounting holes without a decision (grounded, chassis, or isolated with a keepout), or without clearance for the screw head and washer · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-15) <!-- 21 Oct 2026 -->
- <a id="lay-16"></a>⚠️ `LAY-16` Trace width below the current it carries (per net current, from the project data) · impl: hard (TOOL-3, CFG-2) · [📖](checks/lay.md#lay-16) <!-- 21 Oct 2026 -->
- <a id="lay-17"></a>⚠️ `LAY-17` Decoupling capacitor far from the pin it serves · impl: hard (TOOL-3, NET-7) · [📖](checks/lay.md#lay-17) <!-- 21 Oct 2026 -->
- <a id="lay-18"></a>💡 `LAY-18` No keepout under the antenna or around the crystal · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-18) <!-- 21 Oct 2026 -->
- <a id="lay-19"></a>💡 `LAY-19` Assembly drawing without the marking codes of parts that are hard to tell apart · impl: judgement (CFG-5) · [📖](checks/lay.md#lay-19) <!-- 21 Oct 2026; B135B -->
- <a id="lay-20"></a>💡 `LAY-20` Pin-1 orientation differs between connectors of one type on the board · impl: mid (TOOL-3) · [📖](checks/lay.md#lay-20) <!-- 21 Oct 2026 -->
- <a id="lay-21"></a>💡 `LAY-21` Silk without revision, date code or a blank for serial and assembly revision · impl: mid (TOOL-3) · [📖](checks/lay.md#lay-21) <!-- 21 Oct 2026 -->
- <a id="lay-22"></a>⚠️ `LAY-22` Copper or parts too close to the board edge or a V-score line (copper under 0.5 mm, parts under 3 mm) · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-22) <!-- 21 Oct 2026 -->
- <a id="lay-23"></a>💡 `LAY-23` Silk over pads or vias, text too small for the fab, bottom text not mirrored · impl: hard (TOOL-3) · [📖](checks/lay.md#lay-23) <!-- 21 Oct 2026 -->
- <a id="lay-24"></a>💡 `LAY-24` Fab notes without the stackup, controlled-impedance nets and targets, or an IPC-356 netlist (catches inverted inner layers) · impl: judgement (CFG-2) · [📖](checks/lay.md#lay-24) <!-- 21 Oct 2026 -->
- <a id="lay-25"></a>💡 `LAY-25` Hot parts next to oscillators, references, ADCs or temperature sensors · impl: hard (TOOL-3, RPT-2, NET-7) · [📖](checks/lay.md#lay-25) <!-- 21 Oct 2026 -->
