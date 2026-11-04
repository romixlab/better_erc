# Signal-level database

`erc_levels` holds what each logic family, bus and differential standard defines for its levels: the supply
range, the input thresholds, the output levels with the currents they are tested at, input currents, hysteresis,
differential swing and common mode, termination. Level checks (`LVL-2`, `LVL-3`, `LVL-10`; `DIAG-7`) and
simulations read their limits from it instead of numbers typed from memory.

The data is one TOML file, [`erc_levels/data/signal_levels.toml`](../erc_levels/data/signal_levels.toml),
embedded in the crate (`Database::builtin()`).

## Where the numbers come from

Every family names a source document, and every value says whether it was read there:

- `[docs.ID]` describes a document: title, publisher, number, revision, date, the URL it was fetched from, and
  its file in the technical library (`library`, relative to the library root). The document is the standard
  itself where it is free (NXP UM10204 for I²C, the USB specifications); where the standard is behind a paywall
  or a login it is the best free substitute, and says so: the standard's harmonised twin (JEITA for JEDEC),
  a vendor handbook or application note, or a transceiver datasheet (for ISO 11898-2, TIA-485, TIA-232).
  `kind` and `substitute_for` record which.
- `source = { doc, loc }` on a family is the default document and location (table and page) of its values; a
  value can name another document (`doc`) or location (`loc`).
- `verified = true` means the number was read from that document at that location. A number from anywhere else
  (memory, a secondary source, a reasonable assumption such as a supply range the standard leaves open) stays
  `verified = false` and its `note` says why.

A datasheet substitute gives one part's guaranteed numbers, which meet the standard but may be tighter than it;
the family's `notes` say so where it matters.

## Format

```toml
[family.i2c-fm]
name = "I²C Fast-mode (400 kbit/s)"
group = "I2C"                       # the family the variants belong to
kind = "open-drain"                 # single-ended | open-drain | differential | bus | no-levels
standard = "UM10204 Rev. 7.0, Fast-mode"
source = { doc = "UM10204", loc = "Table 10, p. 43" }
supply = [
  { name = "VDD", min = 1.65, max = 5.5, verified = false, note = "UM10204 sets no VDD range" },
]
level = [
  { param = "VIL", min = -0.5, max = "0.3*VDD", unit = "V", verified = true },
  { param = "VOL", max = 0.4, unit = "V", iout_ma = 3.0, cond = "VDD > 2 V", verified = true },
]
```

- One table per standard and supply variant: `74lvc-3v3` and `74lvc-1v8` are separate families of group `74LVC`.
- `supply` lists the supplies and references in order; the first is the main one. A later one may refer to an
  earlier one (`VREF = "0.5*VDDQ"`, a termination voltage `"VCC-2"`).
- A limit is a number in the row's `unit` (V, mV, Vppd, mVppd, A, mA, uA, nA, ohm, kohm, pF), or a linear
  expression of a supply in volts: `"0.7*VDD"`, `"VDD-0.4"`, `"VCC-1.025"`, `"VREF+0.1"`.
- `iout_ma` is the output test current with JEDEC's sign, positive into the pin: an `IOH` test is negative.
  `vsupply` is the supply a row is specified at (74HC's thresholds at 2.0, 4.5 and 6.0 V). `temp_c` narrows a
  row to a temperature range, `state` to a bus state or mode (`dominant`, `recessive`, `high-speed`).
- Parameter names: single-ended `VIH VIL VT+ VT- VHYS VOH VOL IIH IIL II IOZ IOH IOL IOS CI`, differential
  `VOD dVOD VOS dVOS VTH VTL VID VIC VCM RIN RTERM`. Any other name needs a `desc`; its first letter gives its
  dimension.
- `kind = "no-levels"` is for interfaces with no electrical standard of their own (SPI, UART at logic level,
  SWD and JTAG): the `notes` say which levels they run at in practice.

## Checks on the data

`Database::validate()` (run by the crate's tests) reports:

- documents that are unknown or unused, and verified values whose document is not in the library or that have no
  location;
- min <= typ <= max, evaluated at the low, typical and high supply corners;
- VIL below VIH and VOL below VOH for rows of the same supply, state and temperature, and VT- below VT+;
- units that don't fit the parameter, rows of one parameter in different units, expressions in anything but V,
  expressions of a supply the family doesn't have;
- families without levels (unless `no-levels`, which needs notes), unknown `see` references.

A test also checks that every document's file is in the library when the library is on the machine
(`BERC_LIBRARY_DIR`, else `~/sync`); without it the test says so and skips.

## Adding a family

1. Get the document (the standard, else the best free substitute), put it in the library, and add its
   `[docs.ID]` block with the printed number, revision and date.
2. Add the family table, reading each number from the document; give the table and page in `loc`.
3. Check the file: `cargo run -p erc_levels --example validate` (the embedded file), or with extra files to
   check a fragment before merging it: `cargo run -p erc_levels --example validate -- fragment.toml`.
