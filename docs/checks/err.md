# ERR checks, explained

Detail page for the `ERR` category of the [check catalogue](../checks.md#-err---known-errata). Each section explains
one check: what it catches, why the board fails without it, a concrete example, what better_erc needs to find it,
the usual fix, and where to read more. The one-line entries in the catalogue stay the source of truth for ID,
severity and wording; this page is the commentary.

References used throughout:

- **RP2350** · Raspberry Pi, *RP2350 Datasheet* (build 01/10/2026), Appendix D "Errata".
  [datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- **ES0340** · ST, *STM32F101xC/D/E, STM32F103xC/D/E device errata*, ES0340 Rev 17 (2022); the medium-density
  parts (STM32F103x8/B) have the same erratum in ES096 Rev 15 §2.8.7 (fetched 21 Oct 2026 through `tpm fetch`).
  [st.com/resource/en/errata_sheet/es0340](https://www.st.com/resource/en/errata_sheet/es0340-stm32f101xcde-stm32f103xcde-device-errata-stmicroelectronics.pdf),
  [es096](https://www.st.com/resource/en/errata_sheet/es096-stm32f101x8b-stm32f102x8b-and-stm32f103x8b-mediumdensity-device-limitations-stmicroelectronics.pdf)
- **RM0008** · ST, *STM32F101xx ... STM32F107xx reference manual*, RM0008 Rev 21 (2021; fetched through `tpm fetch`).
  [st.com/resource/en/reference_manual/rm0008](https://www.st.com/resource/en/reference_manual/rm0008-stm32f101xx-stm32f102xx-stm32f103xx-stm32f105xx-and-stm32f107xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf)
- **Advamation** · Advamation, *Raspberry Pi I2C clock-stretching bug* (BCM2835).
  [advamation.com/knowhow/raspberrypi/rpi-i2c-bug.html](https://www.advamation.com/knowhow/raspberrypi/rpi-i2c-bug.html)

## `ERR-1` Part with a known hardware erratum that touches this circuit <a id="err-1"></a>

⚠️ warning · needs: MPNs, an errata database with circuit conditions · [catalogue entry](../checks.md#err-1)

**Impl.** mid · depends on CFG-5, NET-7

**What it catches.** A part whose manufacturer documents a hardware erratum, used in a way the erratum affects: the
errata condition matches the circuit around it. Not every erratum of the part, only those whose trigger is visible in
the schematic.

**Why it matters.** Errata sheets are read once, when the part is chosen, if at all. The bugs they describe look like
design faults on the bench and cost days: the circuit is right by the datasheet and still fails. The fix is usually a
resistor, a pin choice or a firmware rule, cheap if known at schematic time and a rework if not.

**Example.** On B200A the user button SW101 connects GPIO22 to 3.3 V and relies on the RP2350's internal pull-down.
RP2350 erratum E9: a Bank 0 pad with its input enabled leaks up to about 120 µA once its voltage is between VIL and VIH,
and holds itself near 2.2 V; the internal pull-down is far too weak to overcome that, so after a release the button may
keep reading "pressed". Rev B adds an external pull-down. Other entries such a database should hold: the STM32F1 I2C
peripheral whose analog filter can lock the BUSY flag after a power-on reset or an ESD event (only a sequence that
toggles SCL and SDA as open-drain GPIOs clears it, so the firmware must own those pins as GPIOs at start-up), the
BCM2835 I2C master that mishandles clock stretching (`I2C-20`), and the STM32F103 whose USB and CAN share one packet
SRAM, so they cannot run at the same time.

**How to check.** Match MPNs (or part families) against an errata database; each entry carries a rule over the netlist
(here: a pin of this part on a net with a switch to a rail and no external pull-down) and only matching ones are
reported, with the erratum ID and the documented workaround.

**Fix.** The workaround the erratum gives (for E9: an external pull-down of 8.2 kΩ or less, or firmware that clears the
pad's input enable between reads, which removes the leakage; fixed in silicon A3), a different pin or part, or a
firmware rule recorded in the pin map.

**References.** RP2350 §D.5.1 "RP2350-E9" (increased leakage on Bank 0 GPIO with the input enabled; 8.2 kΩ external
pull-down; fixed in A3); ES0340 §2.9.7 "I2C analog filter may provide wrong value, locking BUSY flag and preventing
master mode entry"; RM0008 §23.2 "USB main features" (USB and CAN share a dedicated 512-byte SRAM and cannot be used
concurrently); Advamation; catalogue `I2C-20`; B200A review notes §11 and B200 rev B change list (external pull-down on
the user button).
