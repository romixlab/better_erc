# Schematic review checklist

Condensed from the [check catalogue](checks.md); each bracketed ID links back to its entry there. `VIEW`/`RPT`
ids are features tracked in [FEATURES.md](../FEATURES.md); until better_erc automates them, they're checked by
hand or by Claude on a netlist export.

- *Power input:* reverse polarity, overcurrent, overvoltage protection and a filter on every power input
  [[PIN-1]](checks.md#pin-1), [[PIN-4]](checks.md#pin-4), [[PIN-5]](checks.md#pin-5), [[PIN-6]](checks.md#pin-6) ·
  fuse type, rating and voltage [[PIN-2]](checks.md#pin-2), [[PIN-7]](checks.md#pin-7), [[PIN-15]](checks.md#pin-15) ·
  TVS standoff above the operating maximum, clamp below what it protects [[PIN-8]](checks.md#pin-8), [[PIN-9]](checks.md#pin-9) ·
  two sources on one net only through ORing or a switch [[PIN-12]](checks.md#pin-12) · power outputs on connectors
  have an eFuse or switch [[PIN-3]](checks.md#pin-3)
- *Rails:* every rail has a source and every power pin a rail [[RAIL-5]](checks.md#rail-5), [[RAIL-1]](checks.md#rail-1) ·
  fixed regulator MPN voltage and LDO/DC-DC divider output match the net name [[RAIL-4]](checks.md#rail-4),
  [[RAIL-2]](checks.md#rail-2), [[RAIL-3]](checks.md#rail-3), [[NAME-8]](checks.md#name-8) · LDO headroom, output cap
  type, dissipation vs package [[RAIL-9]](checks.md#rail-9), [[RAIL-8]](checks.md#rail-8), [[RATE-21]](checks.md#rate-21) ·
  switcher bootstrap cap, SS/COMP/MODE pins, FB node, inductor value and saturation [[RAIL-10]](checks.md#rail-10),
  [[RAIL-16]](checks.md#rail-16), [[RAIL-17]](checks.md#rail-17), [[RAIL-11]](checks.md#rail-11), [[RAIL-22]](checks.md#rail-22),
  [[RATE-15]](checks.md#rate-15) · boost passes Vin when off [[RAIL-13]](checks.md#rail-13) · EN and PGOOD levels and
  pulls [[RAIL-15]](checks.md#rail-15), [[RAIL-14]](checks.md#rail-14) · ILIM resistor recomputed [[RAIL-6]](checks.md#rail-6)
- *Decoupling and sequencing:* decoupling on every power net, datasheet filter caps (PLL, CRFILT, VCAP) at their
  values, one cap per power pin, at the pin side of any ferrite [[DEC-1]](checks.md#dec-1), [[DEC-5]](checks.md#dec-5),
  [[DEC-7]](checks.md#dec-7), [[DEC-8]](checks.md#dec-8), [[DEC-2]](checks.md#dec-2) · enables never bootstrapped by
  their own output, supply order of multi-rail parts, reset gated by rails [[SEQ-2]](checks.md#seq-2),
  [[SEQ-7]](checks.md#seq-7), [[SEQ-8]](checks.md#seq-8), [[SEQ-1]](checks.md#seq-1) · soft start into capacitive or
  hot-plug loads [[SEQ-6]](checks.md#seq-6), [[SEQ-3]](checks.md#seq-3) · self-discharge on high voltage nets
  [[SEQ-5]](checks.md#seq-5) · isolation barrier crossed by no net, part or label [[GND-4]](checks.md#gnd-4),
  [[GND-5]](checks.md#gnd-5), [[GND-1]](checks.md#gnd-1)
- *Switches and drivers:* Vgs threshold vs drive, Vgs(max) not exceeded (high-side PMOS gate to ground), body diode
  off when off, Rds(on) at the real gate voltage [[DRV-1]](checks.md#drv-1), [[DRV-10]](checks.md#drv-10),
  [[DRV-11]](checks.md#drv-11), [[DRV-20]](checks.md#drv-20) · no MCU pin driving a power transistor directly, BJT base
  resistor, LED resistor and forward voltage [[DRV-3]](checks.md#drv-3), [[DRV-12]](checks.md#drv-12),
  [[DRV-17]](checks.md#drv-17), [[DRV-18]](checks.md#drv-18) · freewheel path on relays and inductive loads
  [[DRV-9]](checks.md#drv-9), [[DRV-14]](checks.md#drv-14) · half bridge dead time, bootstrap, snubbers
  [[DRV-15]](checks.md#drv-15), [[DRV-16]](checks.md#drv-16), [[DRV-7]](checks.md#drv-7) · lithium cell protected, charger
  TS pin and power path, no charge path into a primary cell [[BAT-1]](checks.md#bat-1), [[BAT-2]](checks.md#bat-2),
  [[BAT-3]](checks.md#bat-3), [[BAT-4]](checks.md#bat-4)
- *Backfeed and shorts:* no IO powering an off domain through protection diodes, translators, MOSFET body diodes or
  LDOs [[BKF-1]](checks.md#bkf-1), [[BKF-2]](checks.md#bkf-2), [[BKF-3]](checks.md#bkf-3), [[BKF-4]](checks.md#bkf-4),
  [[BKF-5]](checks.md#bkf-5) · nothing an external device drives powers the unpowered board, no shifter pull-ups
  feeding an unpowered target [[BKF-9]](checks.md#bkf-9), [[BKF-10]](checks.md#bkf-10) · pull-ups only to the right
  rail, and to one that is on with the bus [[LVL-1]](checks.md#lvl-1), [[BKF-12]](checks.md#bkf-12) · 1V8 to 3V3
  thresholds, 5 V into non-tolerant pins [[LVL-2]](checks.md#lvl-2), [[LVL-10]](checks.md#lvl-10) · net shunted by a
  part (label collision), power and ground through a diode, TVS or low resistor, two outputs on one net
  [[SHORT-1]](checks.md#short-1), [[SHORT-2]](checks.md#short-2), [[SHORT-4]](checks.md#short-4), [[SHORT-9]](checks.md#short-9),
  [[SHORT-10]](checks.md#short-10), [[SHORT-11]](checks.md#short-11) · inputs tied to rails without a resistor
  [[SHORT-6]](checks.md#short-6)
- *Wiring hygiene:* one-pin nets, net names that differ only in case or separator, dangling labels, sheet pins
  without a partner, a power symbol named like a signal [[WIRE-1]](checks.md#wire-1), [[WIRE-2]](checks.md#wire-2),
  [[WIRE-3]](checks.md#wire-3), [[WIRE-4]](checks.md#wire-4), [[WIRE-5]](checks.md#wire-5) · no-connect flags only where
  the datasheet allows, none on wired pins [[WIRE-9]](checks.md#wire-9), [[WIRE-7]](checks.md#wire-7),
  [[WIRE-8]](checks.md#wire-8) · one-pin and unnamed nets reviewed [VIEW-5], [[WIRE-11]](checks.md#wire-11)
- *IO and defaults:* no floating inputs, none that float when a connector is empty [[IO-1]](checks.md#io-1),
  [[IO-2]](checks.md#io-2), [[IO-10]](checks.md#io-10) · enables and gates driven by MCU pins have a pull for the reset
  and unpowered state, output enables of registers and drivers pull to off [[IO-3]](checks.md#io-3),
  [[IO-11]](checks.md#io-11), [[IO-12]](checks.md#io-12), [[IO-4]](checks.md#io-4) · peripheral resets not held by a
  resetting controller, reset lines open-drain, reset_n with pull-up and cap [[IO-13]](checks.md#io-13),
  [[CLK-14]](checks.md#clk-14), [[CLK-5]](checks.md#clk-5), [[CLK-7]](checks.md#clk-7) · straps and mode pins of every
  non-MCU part tied, open-drain interrupts pulled up [[IO-14]](checks.md#io-14), [[IO-15]](checks.md#io-15) · button
  pulls the right way, debounce where firmware expects it [[IO-7]](checks.md#io-7), [[IO-18]](checks.md#io-18) ·
  active-low into active-high by name [[LOGIC-8]](checks.md#logic-8) · total and per-pin IO current
  [[LOGIC-4]](checks.md#logic-4), [[LOGIC-6]](checks.md#logic-6) · pins a ROM bootloader or other non-firmware mode
  drives are safe where they land [[IO-9]](checks.md#io-9) · no dependency loop on a boot, update or recovery path
  [[LOGIC-5]](checks.md#logic-5), [[BOOT-6]](checks.md#boot-6)
- *Pin functions and boot:* GPIO never on an NRST/BOOT-by-default pin, debug pins kept, analog signals on analog
  pins, one peripheral per pin set [[MUX-4]](checks.md#mux-4), [[MUX-5]](checks.md#mux-5), [[MUX-6]](checks.md#mux-6),
  [[MUX-7]](checks.md#mux-7), [[MUX-2]](checks.md#mux-2) · SCL goes to SCL, MISO to MISO [[MUX-1]](checks.md#mux-1) ·
  every strapping pin defined at reset, not shared with something another device drives, pulled to the wanted
  mode, enabled by the option bytes [[BOOT-1]](checks.md#boot-1), [[BOOT-2]](checks.md#boot-2), [[BOOT-3]](checks.md#boot-3),
  [[BOOT-4]](checks.md#boot-4) · a way to force the ROM bootloader, reachable with dead firmware
  [[BOOT-5]](checks.md#boot-5), [[BOOT-6]](checks.md#boot-6)
- *Clocks:* MCU with USB or CAN has a crystal, frequency and tolerance fit the interface [[CLK-1]](checks.md#clk-1),
  [[CLK-10]](checks.md#clk-10), [[CLK-19]](checks.md#clk-19) · load caps vs CL, drive level and ESR margin, HS caps rated
  for it [[CLK-8]](checks.md#clk-8), [[CLK-9]](checks.md#clk-9), [[CLK-2]](checks.md#clk-2) · oscillator enable pins tied
  [[CLK-13]](checks.md#clk-13) · VBAT and 32 kHz for an RTC [[CLK-16]](checks.md#clk-16)
- *Interfaces:* I2C pull-ups present, once, in range, equal, to the right voltage; no address collisions or reserved
  addresses; address straps tied [[I2C-1]](checks.md#i2c-1), [[I2C-2]](checks.md#i2c-2), [[I2C-9]](checks.md#i2c-9),
  [[I2C-10]](checks.md#i2c-10), [[LVL-4]](checks.md#lvl-4), [[I2C-5]](checks.md#i2c-5), [[I2C-16]](checks.md#i2c-16),
  [[I2C-17]](checks.md#i2c-17) · no I2C device on a rail that can be off, no SMBus part on a slow bus, no TXS-class
  shifter or stacked offset buffers, pull-ups on every segment [[I2C-11]](checks.md#i2c-11), [[I2C-12]](checks.md#i2c-12),
  [[I2C-14]](checks.md#i2c-14), [[I2C-13]](checks.md#i2c-13), [[I2C-18]](checks.md#i2c-18) · UART TX/RX crossed, CTS/RTS
  paired, CAN TX/RX not crossed [[SER-1]](checks.md#ser-1), [[SER-8]](checks.md#ser-8), [[SER-7]](checks.md#ser-7) ·
  RS-485 and CAN termination for the node's place, fail-safe bias, DE pulled to receive, STB pins tied, VIO rail
  [[SER-9]](checks.md#ser-9), [[SER-13]](checks.md#ser-13), [[SER-10]](checks.md#ser-10), [[SER-11]](checks.md#ser-11),
  [[SER-15]](checks.md#ser-15), [[SER-17]](checks.md#ser-17) · SPI CS not grounded, not shared, flash CS and WP#/HOLD#
  pulled [[SER-2]](checks.md#ser-2), [[SER-18]](checks.md#ser-18), [[SER-19]](checks.md#ser-19), [[MEM-4]](checks.md#mem-4) ·
  whole bus connected [[SER-6]](checks.md#ser-6), [[SER-5]](checks.md#ser-5) · push-pull signals not through a TXS-class
  shifter, DIR and OE pins defined [[LVL-11]](checks.md#lvl-11), [[LVL-12]](checks.md#lvl-12), [[LVL-13]](checks.md#lvl-13)
- *Ethernet and memory:* magnetics centre taps for the PHY type, RGMII delay once, MDIO pull-up, PHY address and
  LED straps, REF_CLK direction [[ETH-1]](checks.md#eth-1), [[ETH-2]](checks.md#eth-2), [[ETH-3]](checks.md#eth-3),
  [[ETH-4]](checks.md#eth-4), [[ETH-7]](checks.md#eth-7), [[ETH-6]](checks.md#eth-6) · SD/eMMC pull-ups, card detect
  wiring, VCCQ voltage [[MEM-1]](checks.md#mem-1), [[MEM-3]](checks.md#mem-3), [[MEM-5]](checks.md#mem-5),
  [[MEM-6]](checks.md#mem-6) · DDR ZQ, VREF, RESET_n [[MEM-8]](checks.md#mem-8), [[MEM-9]](checks.md#mem-9),
  [[MEM-10]](checks.md#mem-10)
- *High speed:* AC caps, TX/RX crossed between host and device, P/N not swapped, lane swaps supported
  [[HS-1]](checks.md#hs-1), [[HS-12]](checks.md#hs-12), [[HS-10]](checks.md#hs-10), [[HS-2]](checks.md#hs-2) · protection
  capacitance fits the data rate [[HS-8]](checks.md#hs-8) · PCIe sideband, HDMI and DP side signals, ref clock
  coupling for the receiver type [[HS-11]](checks.md#hs-11), [[HS-13]](checks.md#hs-13), [[HS-14]](checks.md#hs-14),
  [[HS-16]](checks.md#hs-16) · USB D+/D− not swapped, series resistors and D+ pull-up per part [[HS-7]](checks.md#hs-7),
  [[HS-5]](checks.md#hs-5), [[HS-6]](checks.md#hs-6)
- *USB and Type-C:* 5k1 on each CC, one Rd/Rp source per CC, resistors match the port role, CC1 and CC2 separate,
  both USB 2.0 orientations wired, SuperSpeed through a mux [[USB-1]](checks.md#usb-1), [[USB-7]](checks.md#usb-7),
  [[USB-8]](checks.md#usb-8), [[USB-9]](checks.md#usb-9), [[USB-11]](checks.md#usb-11), [[USB-15]](checks.md#usb-15) ·
  protection on CC/SBU, SBU safe-state resistance [[USB-2]](checks.md#usb-2), [[USB-3]](checks.md#usb-3) · shield,
  VBUS capacitance as device and as host, VBUS detection and sense pin rating, parts on a PD VBUS rated for 20 V
  [[USB-4]](checks.md#usb-4), [[USB-5]](checks.md#usb-5), [[USB-6]](checks.md#usb-6), [[USB-12]](checks.md#usb-12),
  [[USB-13]](checks.md#usb-13), [[RATE-19]](checks.md#rate-19) · hub straps and upstream VBUS detect [[USB-14]](checks.md#usb-14)
- *Connectors:* ESD and series protection on lines to connectors, unipolar vs bipolar protectors
  [[CONN-1]](checks.md#conn-1), [[CONN-2]](checks.md#conn-2), [[CONN-3]](checks.md#conn-3), [[CONN-9]](checks.md#conn-9) ·
  enough ground pins, ground per high-speed pair [[CONN-4]](checks.md#conn-4), [[CONN-10]](checks.md#conn-10),
  [[CONN-5]](checks.md#conn-5) · mating pairs mirrored, FPC contact side, debug header standard
  [[CONN-13]](checks.md#conn-13), [[CONN-14]](checks.md#conn-14), [[CONN-15]](checks.md#conn-15) · pin current and
  voltage ratings, identical connectors with different pinouts [[CONN-11]](checks.md#conn-11), [[CONN-12]](checks.md#conn-12),
  [[CONN-17]](checks.md#conn-17) · what lands where on a reversed or flipped plug [VIEW-11], [[CONN-7]](checks.md#conn-7)
- *Analog:* op-amp feedback to the right input, input common-mode and output swing in range, capacitive loads
  isolated, unused channels tied [[ANA-10]](checks.md#ana-10), [[ANA-6]](checks.md#ana-6), [[ANA-7]](checks.md#ana-7),
  [[ANA-9]](checks.md#ana-9), [[ANA-11]](checks.md#ana-11) · ADC input buffered, scaled into range, reference decoupled
  [[ANA-1]](checks.md#ana-1), [[ANA-13]](checks.md#ana-13), [[RAIL-20]](checks.md#rail-20) · REF inputs from a divider
  checked against bias current, sense amp common-mode and gain vs the bus [[ANA-5]](checks.md#ana-5),
  [[ANA-14]](checks.md#ana-14), [[ANA-15]](checks.md#ana-15)
- *Ratings:* capacitor voltage margin, DC-bias derating and dielectric, resistor power and voltage, shunt power
  [[RATE-1]](checks.md#rate-1), [[RATE-16]](checks.md#rate-16), [[RATE-2]](checks.md#rate-2), [[RATE-9]](checks.md#rate-9),
  [[RATE-3]](checks.md#rate-3) · MOSFET Vds and dissipation, diode current and reverse voltage [[RATE-4]](checks.md#rate-4),
  [[RATE-5]](checks.md#rate-5), [[RATE-13]](checks.md#rate-13) · every supply pin inside its part's range, regulator Vin
  max [[RATE-11]](checks.md#rate-11), [[RATE-12]](checks.md#rate-12) · LED current and brightness [[RATE-10]](checks.md#rate-10),
  [[RATE-7]](checks.md#rate-7) · termination resistors under a bus fault [[RATE-20]](checks.md#rate-20) · temperature
  ranges consistent [[RATE-8]](checks.md#rate-8) · known errata of every MPN read [[ERR-1]](checks.md#err-1) ·
  ripple current and electrolytic life, pulse energy, inductor RMS current, jumper current, series balancing
  [[RATE-25]](checks.md#rate-25), [[RATE-26]](checks.md#rate-26), [[RATE-27]](checks.md#rate-27),
  [[RATE-28]](checks.md#rate-28), [[RATE-29]](checks.md#rate-29), [[RATE-31]](checks.md#rate-31)
- *Passive selection:* every passive's technology fits its role: what the datasheet asks for on that pin
  [[PAS-1]](checks.md#pas-1) · no leaky (polymer, electrolytic, tantalum) capacitor on a SET, REF or soft-start pin
  [[PAS-2]](checks.md#pas-2), [[PAS-3]](checks.md#pas-3), [[PAS-4]](checks.md#pas-4) · C0G or film where a value sets
  a time, a frequency or a signal, none of class 3 [[PAS-6]](checks.md#pas-6), [[PAS-7]](checks.md#pas-7),
  [[PAS-10]](checks.md#pas-10), [[PAS-11]](checks.md#pas-11) · piezo and singing ceramics [[PAS-8]](checks.md#pas-8),
  [[PAS-9]](checks.md#pas-9) · output capacitor technology vs compensation, cold electrolytics, ceramics-only hot-plug
  inputs [[PAS-13]](checks.md#pas-13), [[PAS-14]](checks.md#pas-14), [[PAS-16]](checks.md#pas-16) · flex-crack, sulfur,
  pulse, AEC-Q200 and X/Y safety class [[PAS-17]](checks.md#pas-17), [[PAS-18]](checks.md#pas-18),
  [[PAS-19]](checks.md#pas-19), [[PAS-20]](checks.md#pas-20), [[PAS-21]](checks.md#pas-21) · precision resistor drift,
  HV divider voltage coefficient, shunt technology [[PAS-22]](checks.md#pas-22), [[PAS-23]](checks.md#pas-23),
  [[PAS-25]](checks.md#pas-25) · one grade per precision circuit, no precision where nothing needs it, no odd one out
  in a block [[PAS-40]](checks.md#pas-40), [[PAS-41]](checks.md#pas-41), [[BOM-19]](checks.md#bom-19) · inductor saturation type vs current limit, core loss, shielding, SRF; beads not as
  inductors and damped; chokes per interface [[PAS-30]](checks.md#pas-30), [[PAS-31]](checks.md#pas-31),
  [[PAS-32]](checks.md#pas-32), [[PAS-33]](checks.md#pas-33), [[PAS-34]](checks.md#pas-34), [[PAS-35]](checks.md#pas-35),
  [[PAS-36]](checks.md#pas-36)
- *Debug and test:* SWD/JTAG connected, with series resistors, NRST on the header [[DBG-1]](checks.md#dbg-1),
  [[DBG-2]](checks.md#dbg-2), [[DBG-17]](checks.md#dbg-17) · at least one LED, a firmware-driven one, power rail LEDs
  [[DBG-3]](checks.md#dbg-3), [[DBG-16]](checks.md#dbg-16), [[DBG-4]](checks.md#dbg-4) · console and buses reachable
  [[DBG-14]](checks.md#dbg-14), [[DBG-18]](checks.md#dbg-18) · board revision readable by firmware [[DBG-15]](checks.md#dbg-15) ·
  power-fail warning where storage is written [[DBG-13]](checks.md#dbg-13) · test points on power nets, buses, clocks
  and resets [RPT-15], [[TEST-4]](checks.md#test-4), [[TEST-5]](checks.md#test-5), [[TEST-6]](checks.md#test-6) · DNM
  pulls on EN and open-drain pins, 0R bypasses on power switches [[DBG-7]](checks.md#dbg-7), [[DBG-8]](checks.md#dbg-8),
  [[DBG-9]](checks.md#dbg-9) · 0R on UART/SPI lines that often end up swapped [[DBG-11]](checks.md#dbg-11)
- *BOM and library:* values match the MPN's parameters, footprints match the MPN's package, every mounted part has
  an MPN, no MPN with two values [[BOM-12]](checks.md#bom-12), [[LIB-6]](checks.md#lib-6), [[BOM-14]](checks.md#bom-14),
  [[BOM-17]](checks.md#bom-17) · no TODO or "calculate later" values left [[BOM-5]](checks.md#bom-5) · variant fields
  agree with DNP flags [[BOM-13]](checks.md#bom-13) · symbol pin names carry shared functions (`PF2-NRST`), pin types
  right, exposed pads connected [[LIB-7]](checks.md#lib-7), [[LIB-4]](checks.md#lib-4), [[LIB-10]](checks.md#lib-10) ·
  pin-1 and polarity marks on every footprint and the silk [[LIB-9]](checks.md#lib-9), [[LAY-6]](checks.md#lay-6) ·
  odd values and packages consolidated [[BOM-1]](checks.md#bom-1), [[BOM-11]](checks.md#bom-11), [[BOM-2]](checks.md#bom-2) ·
  consistent names for inverted signals, pairs and fixed voltages [[NAME-1]](checks.md#name-1), [[NAME-2]](checks.md#name-2),
  [[NAME-10]](checks.md#name-10), [[NAME-3]](checks.md#name-3) · schematic and layout match [[LAY-1]](checks.md#lay-1)

Every bug that a review misses and assembly finds becomes a new rule in better_erc, so the list grows from real
failures.
