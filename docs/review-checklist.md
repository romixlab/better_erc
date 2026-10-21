# Schematic review checklist

Condensed from the [check catalogue](checks.md); each bracketed ID links back to its entry there. `VIEW`/`RPT`
ids are features tracked in [FEATURES.md](../FEATURES.md); until better_erc automates them, they're checked by
hand or by Claude on a netlist export.

- *Power:* decoupling on every power net, recommended values [[DEC-1]](checks.md#dec-1), [[DEC-2]](checks.md#dec-2) ·
  reverse polarity, overcurrent, overvoltage protection and filter on power input [[PIN-1]](checks.md#pin-1) ·
  LDO/DC-DC divider output matches the net name [[RAIL-2]](checks.md#rail-2), [[RAIL-3]](checks.md#rail-3) ·
  power sequencing and enable pins never bootstrapped by their own output [[SEQ-1]](checks.md#seq-1), [[SEQ-2]](checks.md#seq-2) ·
  self-discharge on high voltage nets [[SEQ-5]](checks.md#seq-5) · fuse type and rating [[PIN-2]](checks.md#pin-2)
- *Backfeed and shorts:* no IO powering an off domain through protection diodes, translators, MOSFET body
  diodes or LDOs [[BKF-1]](checks.md#bkf-1), [[BKF-2]](checks.md#bkf-2), [[BKF-3]](checks.md#bkf-3), [[BKF-4]](checks.md#bkf-4), [[BKF-5]](checks.md#bkf-5) ·
  pull-ups only to the right rail [[LVL-1]](checks.md#lvl-1) · 1V8 to 3V3 thresholds [[LVL-2]](checks.md#lvl-2) ·
  net shunted by a part (net label collision) [[SHORT-1]](checks.md#short-1), [[SHORT-2]](checks.md#short-2) ·
  inputs tied to rails without a resistor [[SHORT-6]](checks.md#short-6)
- *IO and defaults:* no floating inputs [[IO-1]](checks.md#io-1), [[IO-2]](checks.md#io-2) · enables driven by MCU
  pins have a pull for reset and unpowered state [[IO-3]](checks.md#io-3), [[IO-4]](checks.md#io-4) · reset_n with
  cap and pull-up [[CLK-5]](checks.md#clk-5) · button pulls the right way [[IO-7]](checks.md#io-7) · no MCU pin
  driving a power transistor directly [[DRV-3]](checks.md#drv-3) · total IO current per bank [[LOGIC-4]](checks.md#logic-4) ·
  pins a ROM bootloader or other non-firmware mode drives are safe where they land [[IO-9]](checks.md#io-9) · no
  dependency loop on a boot, update or recovery path (a part set up by the one that needs it to start) [[LOGIC-5]](checks.md#logic-5)
- *Interfaces:* I2C pull-ups present, once, to the right voltage; no address collisions [[I2C-1]](checks.md#i2c-1),
  [[I2C-2]](checks.md#i2c-2), [[I2C-5]](checks.md#i2c-5), [[LVL-4]](checks.md#lvl-4) · UART TX/RX crossed, CAN TX/RX
  not [[SER-1]](checks.md#ser-1) · MCU with USB or CAN has a crystal [[CLK-1]](checks.md#clk-1) · SPI CS not
  grounded [[SER-2]](checks.md#ser-2) · whole bus connected [[SER-6]](checks.md#ser-6), [[SER-5]](checks.md#ser-5) ·
  SCL goes to SCL, MISO to MISO [[MUX-1]](checks.md#mux-1)
- *Connectors:* Type-C 5k1 on CC, protection on CC/SBU [[USB-1]](checks.md#usb-1), [[USB-2]](checks.md#usb-2),
  [[USB-3]](checks.md#usb-3) · USB shield and VBUS capacitance [[USB-4]](checks.md#usb-4), [[USB-5]](checks.md#usb-5) ·
  ESD and series protection on lines to connectors [[CONN-1]](checks.md#conn-1), [[CONN-2]](checks.md#conn-2) ·
  enough ground pins, ground per high-speed pair [[CONN-4]](checks.md#conn-4), [[CONN-5]](checks.md#conn-5) · what
  lands where on a reversed or flipped plug [VIEW-11], [[CONN-7]](checks.md#conn-7) · power outputs on connectors
  have an eFuse or switch [[PIN-3]](checks.md#pin-3)
- *Ratings:* capacitor voltage margin and dielectric, resistor and shunt power, MOSFET Vds, Vgs threshold vs
  drive, dissipation [[RATE-1]](checks.md#rate-1), [[RATE-2]](checks.md#rate-2), [[RATE-3]](checks.md#rate-3),
  [[RATE-4]](checks.md#rate-4), [[DRV-1]](checks.md#drv-1), [[RATE-5]](checks.md#rate-5) · temperature ranges
  consistent [[RATE-8]](checks.md#rate-8) · relay flyback diode [[DRV-9]](checks.md#drv-9) · HS crystal caps rated
  for it [[CLK-2]](checks.md#clk-2)
- *Debug and test:* SWD/JTAG connected, with series resistors [[DBG-1]](checks.md#dbg-1), [[DBG-2]](checks.md#dbg-2) ·
  at least one LED, power rail LEDs [[DBG-3]](checks.md#dbg-3), [[DBG-4]](checks.md#dbg-4) · test points on power
  nets and buses [RPT-15] · DNM pulls on EN and open-drain pins, 0R bypasses on power switches
  [[DBG-7]](checks.md#dbg-7), [[DBG-8]](checks.md#dbg-8), [[DBG-9]](checks.md#dbg-9) · 0R on UART/SPI lines that
  often end up swapped [[DBG-11]](checks.md#dbg-11)
- *Housekeeping:* no TODO or "calculate later" values left [[BOM-5]](checks.md#bom-5) · consistent names for
  inverted signals and pairs [[NAME-1]](checks.md#name-1), [[NAME-2]](checks.md#name-2) · one-pin and unnamed nets
  reviewed [VIEW-5] · schematic and layout match [[LAY-1]](checks.md#lay-1) · odd BOM values and packages
  consolidated [[BOM-1]](checks.md#bom-1), [[BOM-2]](checks.md#bom-2)

Every bug that a review misses and assembly finds becomes a new rule in better_erc, so the list grows from real
failures.
