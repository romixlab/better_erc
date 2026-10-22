# NAME checks, explained

Detail page for the `NAME` category of the [check catalogue](../checks.md#name-1). Each section explains one check: what
it catches, why a name matters to people and to the other checks, a concrete example, what better_erc needs to find it,
the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID, severity
and wording; this page is the commentary.

Names are the schematic's documentation and better_erc's input at once: the power structure (`NET-5`) reads rail
voltages from net names, polarity checks (`LOGIC-8`) read active-low markers, and reviewers read both. The **Impl.**
line says how hard the check is for better_erc in pure Rust: `easy` (a query on the netlist), `mid` (part data or a
project setting first), `hard` (analysis the code does not have yet), `judgement` (intent, so better_erc can at most
list candidates).

References used throughout:

- **KiCad Eeschema** · KiCad, *Schematic Editor* 10.0 documentation: "Annotation tool" (numbering per sheet),
  "Net name assignment rules", "Text markup" (overbar `~{...}`).
  [docs.kicad.org/10.0/en/eeschema/eeschema.html](https://docs.kicad.org/10.0/en/eeschema/eeschema.html)
- **KiCad Pcbnew** · KiCad, *PCB Editor* 10.0 documentation, "Routing differential pairs" (pair suffixes).
  [docs.kicad.org/10.0/en/pcbnew/pcbnew.html](https://docs.kicad.org/10.0/en/pcbnew/pcbnew.html)
- **Wikipedia, Logic level** · active-high and active-low, and the usual notations for an active-low name.
  [en.wikipedia.org/wiki/Logic_level](https://en.wikipedia.org/wiki/Logic_level)
- **Wikipedia, Reference designator** · prefixes and their standards (IEEE 315-1975, ASME Y14.44-2008).
  [en.wikipedia.org/wiki/Reference_designator](https://en.wikipedia.org/wiki/Reference_designator)
- **SparkFun serial** · SparkFun, *Serial Communication* ("Rules and wiring": TX to RX).
  [learn.sparkfun.com/tutorials/serial-communication](https://learn.sparkfun.com/tutorials/serial-communication/all)

## `NAME-1` Inconsistent naming of inverted signals: suggest one notation <a id="name-1"></a>

💡 suggestion · needs: net names · [catalogue entry](../checks.md#name-1)

**Impl.** easy · depends on netlist

**What it catches.** A schematic that marks active-low signals in several ways at once: `RST_N`, `nIRQ`, `~{CS}`,
`OE#`, `RESETn`.

**Why it matters.** A reader has to know every convention to see the polarity, and a tool must recognise all of
them; a mix also hides the nets that carry no marker at all (`NAME-10`). Picking one is cheap and makes polarity
mistakes (`LOGIC-8`) visible at a glance.

**Example.** `~HUB_RESET`, `SPI_CS_N` and `nPWR_FAULT` on one board: three spellings, and a search for active-low
nets finds a third of them.

**How to check.** Classify net names by marker style (overbar markup, `_N`/`_n` suffix, `n` prefix, `#` suffix);
more than one style in a project is a finding, listed by style.

**Fix.** One style for the project; KiCad's overbar markup `~{...}` renders well on the sheet, `_N` survives every
export and tool.

**References.** Wikipedia, Logic level (active-low notations); KiCad Eeschema "Text markup".

## `NAME-2` Inconsistent naming of differential pairs: suggest one notation <a id="name-2"></a>

💡 suggestion · needs: net names · [catalogue entry](../checks.md#name-2)

**Impl.** easy · depends on netlist

**What it catches.** Differential pairs named in several styles (`USB_DP`/`USB_DM`, `USB_P`/`USB_N`, `USB+`/`USB-`,
`CAN_H`/`CAN_L`), or a pair whose two halves do not form a recognisable pair.

**Why it matters.** Layout tools find pairs by name: KiCad's differential pair router pairs nets with a common base
name and a `+`/`-` or `P`/`N` suffix. A pair called `D+`/`D-` on one sheet and `USB_DP`/`USB_DM` on another is routed
as two single nets unless someone notices; length matching and impedance rules miss it too.

**Example.** `USB_DP`/`USB_DM` from the MCU sheet joining `USB_D+`/`USB_D-` at the connector: the net takes one of the
names, and the pair rule attached to the other never applies.

**How to check.** Find nets that look like halves of a pair (suffix pairs, or the two pins of a differential pin pair
on a part); report mixed styles and halves without a partner.

**Fix.** One style that the layout tool recognises (`_P`/`_N` or `+`/`-` in KiCad); keep names consistent across sheets.

**References.** KiCad Pcbnew "Routing differential pairs" (`+`/`-` or `P`/`N` suffixes).

## `NAME-3` Power net names without voltage when it's clearly fixed <a id="name-3"></a>

💡 suggestion · needs: rail sources (regulator outputs, USB VBUS, Type-C with 5.1 kΩ Rd; `NET-5`) · [catalogue entry](../checks.md#name-3)

**Impl.** mid · depends on `NET-5` (sources, missing today), MPN decoding for fixed regulators

**What it catches.** A rail with a fixed voltage, known from its source, whose name says nothing about it: `VCC`,
`VDD_MCU`, `PWR`, `VBUS` on a USB 2.0 device or a Type-C sink that only presents Rd (5 V only).

**Why it matters.** better_erc's power structure reads voltages from names (`+3V3`); a rail named `VCC` gets no
voltage, so every check that compares levels skips it. Reviewers have to trace the regulator to know what a pin gets.
`RAIL-3` and `RAIL-4` then catch names that state the wrong voltage, which only works if names state one.

**Example.** An AP2112K-3.3 output named `VDD_MCU`: the MCU's VDD pins sit on a net with no known voltage, so a
1.8 V-only sensor on the same rail is not flagged.

**How to check.** For each rail with a source whose output is fixed (a fixed-output regulator by MPN, a feedback
divider with known reference, USB VBUS of a non-PD port), check whether the name carries the voltage.

**Fix.** Put the voltage in the name in the project's style (`+3V3`, `+3V3_MCU`, `VBUS_5V0`).

**References.** better_erc `erc_analysis/src/power.rs` (voltage parsed from the name); catalogue `RAIL-2`, `RAIL-3`, `RAIL-4`.

## `NAME-4` UART: suggest `X_TX_Y_RX` style names <a id="name-4"></a>

💡 suggestion · needs: UART pins (by name or `NET-12`), net names · [catalogue entry](../checks.md#name-4)

**Impl.** easy · depends on netlist (`NET-12` for pins named by port only)

**What it catches.** UART nets named `TX` and `RX`, or `UART1_TX`, without saying whose TX it is.

**Why it matters.** "TX" is ambiguous: the net from the MCU's TX to the modem's RX can be called `TX` from one side and
`RX` from the other. Half of all swapped UARTs start there (`DBG-11`). Naming the net by both ends, `MCU_TX_GPS_RX`,
makes a crossed wire visible in the name itself: `MCU_TX_GPS_TX` reads wrong.

**Example.** A sheet with `GPS_TX` and `GPS_RX` labels: on the MCU sheet `GPS_TX` lands on the MCU's TX pin (the
designer meant "transmit to the GPS"), on the GPS sheet on the GPS's TX pin (the other designer meant "the GPS's TX").
Both outputs drive one net.

**How to check.** Nets that join UART TX or RX pins and whose name contains `TX` or `RX` without naming both ends;
cross-check that a net named `..._TX_..._RX` joins exactly one TX and one RX pin.

**Fix.** Name UART nets by driver and receiver (`MCU_TX_GPS_RX`, `GPS_TX_MCU_RX`).

**References.** SparkFun serial ("Rules and wiring"); catalogue `MUX-1`, `DBG-11`.

## `NAME-5` Gaps in designator series on rev A or standalone <a id="name-5"></a>

💡 suggestion · needs: reference designators · [catalogue entry](../checks.md#name-5)

**Impl.** easy · depends on netlist

**What it catches.** Gaps in the numbering of a designator series (R1-R12, R14-R20: no R13) on a first revision or a
board that is not part of a family.

**Why it matters.** On a first revision a gap usually means a deleted part, and sometimes a part deleted by mistake;
it also makes the BOM look incomplete to the assembler, who asks. On later revisions gaps are normal and wanted:
re-annotating would renumber parts and break every document that refers to them.

**Example.** A rev A board whose resistors run R1-R40 with R17 missing: a pull-up was deleted while moving a block
and never placed again.

**How to check.** Per prefix and per sheet block (`NAME-6`), list missing numbers; only on rev A or when the project
says so.

**Fix.** Re-annotate before the first release; afterwards keep gaps and document removals in the change list.

**References.** KiCad Eeschema "Annotation tool"; Wikipedia, Reference designator.

## `NAME-6` Suggest per page designators; highlight missing ones <a id="name-6"></a>

💡 suggestion · needs: designators and the sheet each part is on · [catalogue entry](../checks.md#name-6)

**Impl.** easy · depends on netlist (sheet path per component)

**What it catches.** A multi-sheet design numbered across the whole project instead of per sheet (R1xx on sheet 1,
R2xx on sheet 2, ...), and, where per-sheet numbering is used, parts whose number does not match their sheet.

**Why it matters.** Per-sheet numbers tell anyone holding the board or the BOM where in the schematic a part lives:
R607 is on sheet 6. Revisions also stay stable: adding a part on sheet 3 does not shift the numbers on sheet 5. Our
boards number this way (B200A's MCU sheet holds U1801-U1803, R1802 and R1807).

**Example.** A part moved from sheet 2 to sheet 7 keeps R214; a reviewer looks for it on sheet 2.

**How to check.** Compare each designator's hundreds (or thousands) with its sheet number; report mismatches, and
suggest the scheme when the project uses flat numbering.

**Fix.** KiCad's annotation can start numbering at the sheet number × 100 or × 1000.

**References.** KiCad Eeschema "Annotation tool" (numbering from the sheet number × 100 or × 1000).

## `NAME-7` No nets found in strict mode: suggest `+xVy` power names and non-strict mode <a id="name-7"></a>

💡 suggestion · needs: net names, the power structure (`NET-5`) · [catalogue entry](../checks.md#name-7)

**Impl.** easy · depends on `NET-5`

**What it catches.** A project where better_erc's strict power naming finds no rail with a voltage: no net name
matches `+xVy` (`+3V3`, `+1V8`, `+12V0`).

**Why it matters.** better_erc's power structure is derived from names first. The default reads the whole power
name grammar (`+3V3`, `VDD_1V8`, `+5V`, `3.3V`, `3P3V`, `-5V`, `VCC3V3`); strict mode (`PowerOptions::strict`) counts
only names with a `+`, the integer volts, `V`, and the decimals (`+3V3`). Nets named `+V...`, `VCC`, `VDD` or `VBUS`,
or carrying power pins or power symbols, still become rails but without a voltage. A project whose rails carry no
voltage in their names gets no voltages at all, and every check that compares levels silently does nothing. The
finding says why and what to change.

**Example.** A design with `+3.3V`, `+5V` and `VDD_1V8`: strict mode reads no voltage from any of them (`+5V` lacks
the decimal digit, `+3.3V` uses a dot); the default reads all three. A design with only `VCC`, `VDD` and `VIO` gets
rails without a voltage in either mode.

**How to check.** Run the strict detection; zero rails with a voltage is a finding, with the names that look like
rails listed.

**Fix.** Rename rails to `+xVy` (`+3V3`, `+5V0`), or run in non-strict mode and check what it picked up.

**References.** better_erc `erc_analysis/src/power.rs` (`read_name`: the power name grammar; `Power::of`: names, power
symbols, power pins).

## `NAME-8` Fixed-voltage name on a net that is not fixed <a id="name-8"></a>

⚠️ warning · needs: rail sources and what can change them (PD, switches; `NET-5`, `CFG-2`) · [catalogue entry](../checks.md#name-8)

**Impl.** mid · depends on `NET-5`, `CFG-2`

**What it catches.** A net named with a fixed voltage (`+5V`) whose source is not fixed: USB VBUS behind a PD sink, a
battery, an adjustable regulator, an external supply.

**Why it matters.** Everyone downstream trusts the name: parts get rated for 5 V, a header pin is labelled 5 V, a
user plugs a 5 V-only device into it. With PD the port can negotiate up to 20 V. better_erc's checks also take the
name's voltage as fact.

**Example.** B153A's header pin 10 is named and silkscreened `+5V`, but it is the receptacle VBUS through the switch
Q102: 5 V by default, up to 20 V after PD. The rev B change list renames the net and silk to `VBUS_SW`.

**How to check.** Rails named with a voltage whose source is a USB receptacle with a PD controller, a battery, an
adjustable regulator or a connector the project marks as variable; report the name with the range.

**Fix.** Name the net by what it is (`VBUS_SW`, `VBAT`, `VIN`), and rate everything on it for the full range
(`RATE-19`).

**References.** B153 rev B change list (`+5V` to `VBUS_SW`); B153A schematic review §10 (header J101); catalogue
`RAIL-3`, `RATE-19`.

## `NAME-9` IO numbering out of order against the part's pins <a id="name-9"></a>

💡 suggestion · needs: net names with numbers, MCU pin names (GPIO numbers) · [catalogue entry](../checks.md#name-9)

**Impl.** easy · depends on netlist

**What it catches.** A numbered set of nets (IO0-IO7, LED1-LED4, CH0-CH3) whose order differs from the order of the
MCU pins they land on, where the pins are otherwise consecutive.

**Why it matters.** Firmware loves consecutive pins: a PIO program, a port write, a DMA to a GPIO register moves a
block of pins at once. One swapped pair forces a lookup table or breaks the block, and the swap is invisible on the
sheet unless the names and the pins are compared.

**Example.** On B200A IO0-IO3 sit on GPIO12-GPIO15 and IO6-IO7 on GPIO18-GPIO19, but IO4 is on GPIO17 and IO5 on
GPIO16: swapped against GPIO order, which matters for PIO programs that use consecutive pins. The rev B change list
puts them back in order.

**How to check.** For net families with a numeric index, map each to the GPIO number of the MCU pin it reaches
(from the pin name); report families whose GPIO numbers are monotonic except for a swap.

**Fix.** Swap the nets at the MCU (or rename them) so the order matches.

**References.** B200A schematic review §6 (IO4/IO5 swapped against GPIO order); B200 rev B change list.

## `NAME-10` Active-low signal without a marker in its name <a id="name-10"></a>

💡 suggestion · needs: active-low pins (overbar or `n`/`#` in the pin name, or `NET-7`), net names · [catalogue entry](../checks.md#name-10)

**Impl.** easy · depends on netlist (`NET-7` for pins whose names carry no marker)

**What it catches.** A net that connects to active-low pins (`~{RESET}`, `nCS`, `OE#`) whose own name has no
active-low marker: `HUB_RESET`, `FLASH_CS`.

**Why it matters.** The net name is what the firmware engineer reads; `HUB_RESET` invites "drive high to reset". It
is also what `LOGIC-8` checks for polarity mismatches: a net with no marker cannot be checked.

**Example.** B153A names the hub's active-low reset net `~HUB_RESET`, so the firmware drives it high to run the hub
by name alone; B200A's 595 output enable is `EXP.~OE`. A net `HUB_RESET` on the same pin would read the other way.

**How to check.** Nets on at least one pin with an active-low name, whose net name has no marker in any of the
project's styles (`NAME-1`).

**Fix.** Add the project's marker to the net name.

**References.** Wikipedia, Logic level (active-low notations); KiCad Eeschema "Text markup" (`~{...}` overbar);
catalogue `LOGIC-8`.

## `NAME-11` Negative rail named without its sign <a id="name-11"></a>

💡 suggestion · needs: net names, the power structure (`NET-5`) · [catalogue entry](../checks.md#name-11)

**Impl.** easy · depends on `NET-5`

**What it catches.** A negative rail named without a minus (`5V_NEG`, `VEE`, `V5N`), or with a sign in a form the
name parser does not read.

**Why it matters.** A name without its sign is read by a rule, not stated: better_erc reads `-5V0` as −5 V, and
`5V0_NEG` as −5 V too, but only because of the `NEG` marker (the voltage fact says so in its note). A marker it does not
know (`V5N`, `5V_M`) leaves a +5 V rail where −5 V sits, and a check that compares pin ratings sees the wrong sign.
People misread it the same way when they look for "the 5 V rail" with a meter.

**Example.** An op-amp supply `+5V0` and `5V0_NEG`: a parser that finds `5V0` in both gives the negative supply +5 V
and the op-amp's V− pin looks shorted to V+ in every voltage check.

**How to check.** Rails whose name contains `NEG`, `VEE`, `VSS_A` style markers or which a negative regulator
(by part class) feeds, and whose parsed voltage is positive or missing.

**Fix.** Name negative rails with a leading minus (`-5V0`, `-12V`) and make sure the parser reads it.

**References.** better_erc `erc_analysis/src/power.rs` (`read_name`: the sign from a `NEG`, `VEE`, `VNEG` or `V-` marker, `NetName::sign_from_marker`; `VEE` is a negative
rail, not a ground); catalogue `RAIL-3`.

## `NAME-12` Unannotated or duplicate reference designators <a id="name-12"></a>

🛑 error · needs: reference designators · [catalogue entry](../checks.md#name-12)

**Impl.** easy · depends on netlist

**What it catches.** Parts still carrying `R?`, `U?` (never annotated), or two parts with the same designator.

**Why it matters.** The designator is the key that joins schematic, layout, BOM and placement file. An unannotated
part may not reach the layout at all, and duplicates merge two parts into one in the netlist or the BOM: one of them
is never bought, or both pads get the same net.

**Example.** A block copied between two projects with its designators, pasted next to parts already called R12 and
C5: the netlist export resolves the clash by dropping one, and the layout misses a capacitor.

**How to check.** Designators ending in `?`, and designators used more than once across the hierarchy (units of one
multi-unit part excepted).

**Fix.** Annotate the whole schematic (KiCad's annotation tool), keeping existing numbers on later revisions.

**References.** KiCad Eeschema "Annotation tool"; Wikipedia, Reference designator.
