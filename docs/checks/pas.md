# PAS checks, explained

Detail page for the `PAS` category of the [check catalogue](../checks.md#pas-1): passive selection, a capacitor,
resistor, inductor, bead or choke whose technology is wrong for where it sits. Each section explains one check: what it
catches, why the board fails without it, a concrete example, what better_erc needs to find it, the usual fix, and where
to read more. The one-line entries in the catalogue stay the source of truth for ID, severity and wording; this page is
the commentary.

The **Impl.** line rates how hard the check is to build in better_erc, in pure Rust and without an LLM: `easy` (the
netlist alone), `mid` (part data or a project setting must be built or read first), `hard` (analysis the code does not
have yet: power states, following signals, layout geometry), `judgement` (a human or an LLM reads intent, better_erc
lists candidates). The IDs after "depends on" are [FEATURES.md](../../FEATURES.md) items.

How the PAS checks work, in short. Every passive gets a **technology** with its parameters (MLCC class 1 or 2 and the
dielectric, wet aluminium, polymer, tantalum, film, thick or thin film, foil, wirewound, ferrite or powder core,
shielding) as facts with provenance: from the part data by MPN (`CFG-5`, mx3), from the MPN's series through the series
table (`CFG-7`), from the symbol's `Dielectric` and value fields, from the description, and last from the footprint
(`Assumed`). Every passive also gets a **role** (`NET-23`): what the pin it hangs on is for (a current-set pin,
soft-start, timing, crystal, noise reduction), what the topology says (an integrator, a coupling capacitor, a snubber, a
regulator's output capacitor, a part straight across a battery), and what net classes and the project's intent say
(audio, vibration, automotive, sulfur). A check pairs a role with a phenomenon of the technology. A part's datasheet can
state what its pins need in the part database's `required` section (`CFG-6`): "low-leakage ceramic", "X5R or better",
"ESR 0.1 to 1 Ω"; a constraint reports under the check of its parameter, and `PAS-1` reports the ones no specific check
covers. Where a datasheet says nothing, the pin's electrical facts (its source current and accuracy) let the check
derive a limit, and generic profiles by pin name (`SET`, `SS`, `NR`, `RT`, `XIN`) fill in for parts without an entry,
reported as estimates. A role with a constraint and a passive whose technology is unknown is reported as not run with
the data that would decide it, never as a pass. Findings computed from datasheet maxima say so: typical parts are
better, and a measurement is a good reason to waive. Stress-versus-rating checks of the same parts (ripple current,
electrolytic life, pulse energy, inductor RMS current) are `RATE` checks.

**Intent is per board.** Some checks depend on what the board is for: `vibration`, `audio`, `automotive`, `precision`,
`industrial`, `environment` (`sulfur`, `condensing`), `ambient` (the board's operating range) and `life_years`, set in
the board's own project file. There are no lab-wide or built-in defaults: on a board that does not state the intent a
check needs, that check (or the part of it that needs the intent) is reported as not run with the intent it would need,
never guessed. Where an intent only refines a check (the hottest ambient for leakage, the temperature range for drift),
the check runs with the stated fallback and says so.

**Accuracy budgets come from the parts.** How precise the passives around a part must be follows from the part they
serve, not from a default. A part with an accuracy spec (a voltage reference, a current-reference LDO, a regulator's
feedback reference, a precision op-amp, an ADC, DAC or instrumentation amplifier, a current-sense amplifier) states its
own error in its part data: initial accuracy, drift over temperature, offset, gain error. Each passive whose value or
leakage enters that error (the SET capacitor and RSET, a feedback or gain network, a reference divider, a sense
resistor) may add at most a share of it: **`passive_share` = 25 %** of the part's own error at the same condition (25 °C
against the initial spec, the board's range against the full-range spec), configurable per project, so the passives
never spend the accuracy the designer paid for in the part. The LT3045's SET current is 99 to 101 µA at 25 °C and 98 to
102 µA over its range (LT3042 and LT3094 the same, LT3080 9.90 to 10.10 and 9.80 to 10.20 µA): a 0.25 % budget at 25 °C,
250 nA of SET capacitor leakage on an LT3045 and 25 nA on an LT3080. With the leakage figures of `PAS-2`, a polymer or
wet aluminium SET capacitor at its datasheet maximum (about 3 %) and an MnO2 tantalum (0.5 %) are findings, an X7R or
film part is not. A non-precision part gives a looser budget the same way: an LDO with a 2 % reference leaves 0.5 % to
its feedback divider. The order: the project's explicit figure, a datasheet constraint on the pin, then the budget
derived from the part's spec; a part without that data falls back to its generic profile's assumed accuracy (stated in
the profile, as data) and the finding is an estimate. Where one circuit mixes precision and general-grade passives, the
weakest part sets the accuracy: `PAS-40`; precision parts where the circuit does not need them are lost cost: `PAS-41`.

References used throughout:

- **LT3045** · Analog Devices, *LT3045 20V, 500mA, Ultralow Noise, Ultrahigh PSRR Linear Regulator*, datasheet Rev. D.
  [analog.com/.../lt3045.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/lt3045.pdf)
- **LT3042** · Analog Devices, *LT3042 20V, 200mA, Ultralow Noise, Ultrahigh PSRR RF Linear Regulator*, datasheet Rev.
  C. [analog.com/.../lt3042.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/lt3042.pdf)
- **LT3094** · Analog Devices, *LT3094 −20V, 500mA, Ultralow Noise, Ultrahigh PSRR Negative Linear Regulator*, datasheet
  Rev. B. [analog.com/.../lt3094.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/lt3094.pdf)
- **LT3080** · Analog Devices, *LT3080 Adjustable 1.1A Single Resistor Low Dropout Regulator*, datasheet (3080fc).
  [analog.com/.../3080fc.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/3080fc.pdf)
- **ADP7118** · Analog Devices, *ADP7118 20 V, 200 mA, Low Noise, CMOS LDO Linear Regulator*, datasheet Rev. H.
  [analog.com/.../adp7118.pdf](https://www.analog.com/media/en/technical-documentation/data-sheets/adp7118.pdf)
- **ADI-CER** · Analog Devices (ex-Maxim), *Temperature and Voltage Variation of Ceramic Capacitors, or Why Your 4.7µF
  Capacitor Becomes a 0.33µF Capacitor*, technical article, 4 Dec 2012.
  [analog.com](https://www.analog.com/en/resources/technical-articles/temperature-and-voltage-variation-ceramic-capacitor.html)
- **ADI-BLD** · Analog Devices, *Basic Linear Design*, chapter 10 "Passive Components" (dielectric absorption table,
  high-K ceramics in signal paths).
  [analog.com/.../Chapter10.pdf](https://www.analog.com/media/en/training-seminars/design-handbooks/Basic-Linear-Design/Chapter10.pdf)
- **ADI-AN1368** · J. Eco, A. Limjoco (Analog Devices), *Ferrite Bead Demystified*, AN-1368 Rev. 0 (2015): impedance
  collapse with DC bias, bead-capacitor resonance and its damping.
  [analog.com/.../AN-1368.pdf](https://www.analog.com/media/en/technical-documentation/application-notes/AN-1368.pdf)
- **ADI-SHUNT** · M. O'Sullivan (Analog Devices), *Optimize High-Current Sensing Accuracy by Improving Pad Layout of
  Low-Value Shunt Resistors*, Analog Dialogue 46-06 (2012).
  [analog.com](https://www.analog.com/media/en/analog-dialogue/volume-46/number-2/articles/optimize-high-current-sensing-accuracy.pdf)
- **LT-AN88** · G. Perica (Linear Technology), *Ceramic Input Capacitors Can Cause Overvoltage Transients*, Application
  Note 88 (2001): "These voltage spikes can easily be twice the amplitude of the input voltage step."
  [analog.com/.../an88f.pdf](https://www.analog.com/media/en/technical-documentation/application-notes/an88f.pdf)
- **TI-SLVA115** · TI, *ESR, Stability, and the LDO Regulator*, SLVA115A (2002, rev. 2020).
  [ti.com/lit/an/slva115](https://www.ti.com/lit/an/slva115/slva115.pdf)
- **TI-SSZTB09** · M. Zhao (TI), *How to Reduce Acoustic Noise of MLCCs in Power Applications*, SSZTB09 (2016).
  [ti.com/lit/pdf/ssztb09](https://www.ti.com/lit/pdf/ssztb09)
- **TI946** · TI, *Understanding OOA Operation*, SLUA946 (2019): light-load ripple frequencies falling into the audio
  band. [ti.com/lit/pdf/slua946](https://www.ti.com/lit/pdf/slua946)
- **TIAAI2** · D. Norwood (TI), *The Use and Benefits of Ferrite Beads in Gate Drive Circuits*, SLUAAI2 (2021).
  [ti.com/lit/pdf/sluaai2](https://www.ti.com/lit/pdf/sluaai2)
- **ST-AN2867** · STMicroelectronics, *Guidelines for oscillator design on STM8AF/AL/S and STM32 MCUs/MPUs*, AN2867 Rev.
  24: load capacitance must be constant (§3.3); its worked example uses C0G (§3.7).
  [st.com](https://www.st.com/resource/en/application_note/an2867-guidelines-for-oscillator-design-on-stm8afals-and-stm32-mcusmpus-stmicroelectronics.pdf)
- **MUR-VOLT** · Murata, *The voltage characteristics of electrostatic capacitance*, Capacitor Guide (2012): DC and AC
  level dependence of high-K MLCCs.
  [article.murata.com](https://article.murata.com/en-global/article/voltage-characteristics-of-electrostatic-capacitance)
- **MUR-AGE** · Murata FAQ, capacitance change with time (ageing, recovery above the Curie point at about 125 °C).
  [murata.com](https://www.murata.com/en-global/support/faqs/capacitor/ceramiccapacitor/char/0006)
- **MUR-NOISE** · Murata FAQ, why acoustic noise occurs in ceramic capacitors (low-noise series ZRA, ZRB, KRM, KR3).
  [murata.com](https://www.murata.com/en-global/support/faqs/capacitor/ceramiccapacitor/char/0020)
- **MUR-GRM** · Murata, *High Capacitance MLCC GRM Series – Product Tutorial* (2020): EIA class 2 codes, DC bias by case
  size. [mouser.com mirror](https://www.mouser.com/pdfDocs/MurataHi-CapMLCCProductTutorial.pdf)
- **KEM-X7R**, **KEM-C0G** · KEMET, *X7R Dielectric* (C1002) and *C0G Dielectric* (C1003) surface-mount MLCC datasheets:
  insulation resistance, ageing (X7R 3 % per decade-hour, C0G 0 %), ±30 ppm/K, "No piezoelectric noise". [X7R
  mirror](https://mfgdatasheets.nlight.net/Kemet%20X7R.pdf), [C0G
  mirror](https://mfgdatasheets.nlight.net/Kemet%20C0G.pdf)
- **KEM-POLY** · KEMET, *T543 polymer tantalum* datasheet (T2061) and *T520 KO-CAP component performance
  characteristics*: leakage ≤ 0.1·C·V, derating to 90 % (≤ 10 V) and 80 % (> 10 V), no ignition failure mode. [T543
  mirror](https://www.mouser.lt/pdfdocs/KEM_T2061_T543UPDATED-2.pdf)
- **KEM-FLEX** · KEMET, *Technical Overview of Flex Mitigation Solutions* (FT-CAP up to 5 mm bend, placement away from
  edges and screws). [mouser.com mirror](https://www.mouser.com/pdfdocs/TechnicalOverviewofFlexMitigationSolutions.pdf)
- **TDK-FLEX** · TDK, *Flex Crack Countermeasures in MLCCs* (standard 3216 cracks at about 4 mm, soft termination twice
  that). [mouser.com mirror](https://www.mouser.com/pdfdocs/TDK-FlexCrack-Countermeasures.pdf)
- **AVX-TECH** · KYOCERA AVX, *Technical Summary and Application Guidelines – polymer, tantalum and niobium oxide
  capacitors*: DCL 0.01·C·V, reverse voltage limits, 1 Ω/V series resistance.
  [kyocera-avx.com](https://www.kyocera-avx.com/docs/techinfo/Tantalum-NiobiumCapacitors/TechSumAppGuide.pdf)
- **AVX-DER** · T. Zedníček, J. Gill (KYOCERA AVX), *Voltage Derating Rules for Solid Tantalum and Niobium Capacitors*.
  [kyocera-avx.com](https://www.kyocera-avx.com/docs/techinfo/Tantalum-NiobiumCapacitors/voltaged.pdf)
- **PAN-FR** · Panasonic, *Aluminum Electrolytic Capacitors FR-A series* datasheet (leakage 0.01·C·V or 3 µA).
  [industrial.panasonic.com](https://industrial.panasonic.com/cdbs/www-data/pdf/RDF0000/ABA0000C1259.pdf)
- **NIC-POLY** · Nichicon, *Conductive Polymer Aluminum Solid Electrolytic Capacitors* catalog (RPS series leakage about
  0.2 to 0.25·C·V). [nichicon.com](https://www.nichicon.com/getmedia/e78764ed-7ae5-466d-b2f8-4da63c196134/e-fpcap.pdf)
- **NIC-AL** · Nichicon, *Application Guidelines for Aluminum Electrolytic Capacitors*, CAT.8101C: life halves per 10 K,
  ripple heating, dry-out, voltage treatment after storage.
  [mirror](https://www.nmr.mgh.harvard.edu/~reese/public_html/electrolytics/tec2.pdf)
- **CDE-AL** · Cornell Dubilier, *Aluminum Electrolytic Capacitor Application Guide*: ESR 10 to 100 times at the lowest
  rated temperature, ESR doubling at end of life.
  [web.archive.org](https://web.archive.org/web/2023/https://www.cde.com/resources/catalogs/AEappGUIDE.pdf)
- **VIS-OSCON** · Vishay, *Reliability – Vishay OS-CON*, doc. 90017: polymer life ×10 per 20 K.
  [vishay.com](https://www.vishay.com/docs/90017/reliabilityoscon.pdf)
- **VIS-FILM** · Vishay Roederstein, *General Technical Information – Film Capacitors*, doc. 26033: self-healing, du/dt,
  dielectric absorption. [vishay.com](https://www.vishay.com/docs/26033/gentechinfofilm.pdf)
- **VIS-SAFETY** · Vishay, *Vishay Safety Capacitors: Film, Ceramic Single Disc, and MLCC Technologies*, doc. 45023, and
  *EMI Safety Capacitor Solutions* MS7286: IEC 60384-14 classes and humidity grades.
  [45023](https://www.vishay.com/docs/45023/vishaysafetycapacitors.pdf),
  [MS7286](https://www.vishay.com/docs/48153/ms7286_emi_safety_capacitor_solutions_v03.pdf)
- **EAT-SC** · Eaton, *Supercapacitor application guidelines*, technical note PS-5006.
  [eaton.com](https://www.eaton.com/content/dam/eaton/products/electronic-components/resources/technical/eaton-supercapacitor-application-guidelines.pdf)
- **V-BASICS** · Vishay Beyschlag, *Basics of Linear Fixed Resistors*, doc. 28771: TCR, noise, stability and voltage by
  technology. [vishay.com](https://www.vishay.com/docs/28771/basics.pdf)
- **V-CRCW**, **V-CRCWHP**, **V-CRCWSR** · Vishay, *D/CRCW e3* (doc. 20035), *CRCW-HP e3 pulse proof* (20043), *CRCW-SR
  e3 sulfur resistant* (20116) thick-film chip datasheets. [20035](https://www.vishay.com/docs/20035/dcrcwe3.pdf),
  [20043](https://www.vishay.com/docs/20043/crcwhpe3.pdf), [20116](https://www.vishay.com/docs/20116/crcw-sre3.pdf)
- **V-TNPW**, **V-TNPV**, **V-CRHV** · Vishay thin-film (28758), high-voltage thin-film (28881, under 1 ppm/V) and
  high-voltage thick-film (68002, 10 to 25 ppm/V) chip datasheets.
  [28758](https://www.vishay.com/docs/28758/tnpw_e3.pdf), [28881](https://www.vishay.com/docs/28881/tnpve3.pdf),
  [68002](https://www.vishay.com/docs/68002/crhv.pdf)
- **V-MELF**, **V-PULSE12** · Vishay, *MMU 0102, MMA 0204, MMB 0207 thin film MELF* (28963) and *12 Things to Know About
  Resistors in Pulse Load Applications* (48516). [28963](https://www.vishay.com/docs/28963/mmu0102_mma0204_mmb0207.pdf),
  [48516](https://www.vishay.com/docs/48516/_ms9702509-2401-vishaychecklistpulseload.pdf)
- **V-ACAS**, **V-ARRAYAN** · Vishay, *ACAS AT precision thin-film arrays* (28770) and *Increasing Accuracy in Feedback
  Circuits and Voltage Dividers with Thin Film Chip Resistor Arrays* (28194).
  [28770](https://www.vishay.com/docs/28770/acasat.pdf), [28194](https://www.vishay.com/docs/28194/incresarr.pdf)
- **V-WSL** · Vishay Dale, *WSL Power Metal Strip Resistors*, doc. 30100 (whole-part vs element TCR).
  [vishay.com](https://www.vishay.com/docs/30100/wsl.pdf)
- **V-RSNS**, **V-GGN** · Vishay Dale wirewound RS/NS (30204) and G/GN (30205): Ayrton-Perry non-inductive styles.
  [30204](https://www.vishay.com/docs/30204/rsns.pdf), [30205](https://www.vishay.com/docs/30205/ggn.pdf)
- **V-NTC**, **V-ICL** · Vishay NTCLE100E3 thermistors (29049, dissipation factor 7 mW/K) and AS352R035 inrush current
  limiter (24178). [29049](https://www.vishay.com/docs/29049/ntcle100.pdf),
  [24178](https://www.vishay.com/docs/24178/as352r035.pdf)
- **VPG-INTRO**, **VPG-ESD**, **VPG-F106**, **VPG-TN110** · Vishay Foil Resistors, *Introduction to High Precision
  Resistor Industry* (49787), *Resistor Sensitivity to ESD* (63129), *Facts at a Glance #106* (noise, 49997), *Technical
  Note 110* (moisture, 63516); a foil vendor's comparisons. Mirrors at [xdevs.com/doc/VPG](https://xdevs.com/doc/VPG/)
- **P-ERJS**, **P-ERJP** · Panasonic, *Anti-Sulfurated Thick Film Chip Resistors ERJ S / ERJ U* and *Anti-Surge Thick
  Film Chip Resistors ERJ P, PA, PM*.
  [ERJ-S](https://industrial.panasonic.com/cdbs/www-data/pdf/RDP0000/AOA0000C334.pdf),
  [ERJ-P](https://industrial.panasonic.com/cdbs/www-data/pdf/RDP0000/AOA0000C331.pdf)
- **K-RK73B** · KOA Speer, *RK73B thick-film chip resistor* datasheet (±400 ppm/K above about 1 MΩ, terminal-temperature
  rating). [koaspeer.com](https://www.koaspeer.com/pdfs/RK73B.pdf)
- **Y-AF**, **Y-RVP** · Yageo, *AF anti-sulfurated automotive-grade chip resistors* and *RV_P automotive high-voltage
  chip resistors*. [AF](https://www.yageogroup.com/content/datasheet/asset/file/PYU-AF_51_ROHS_L),
  [RV_P](https://www.yageogroup.com/content/datasheet/asset/file/PYU-RV_51_ROHS_P)
- **R-ESR**, **R-PMR**, **R-LTR** · Rohm, anti-surge ESR, metal-plate shunt PMR and wide-terminal LTR chip resistor
  datasheets. [ESR](https://fscdn.rohm.com/en/products/databook/datasheet/passive/resistor/chip_resistor/esr-e.pdf),
  [PMR](https://fscdn.rohm.com/en/products/databook/datasheet/passive/resistor/chip_resistor/pmr-e.pdf),
  [LTR](https://fscdn.rohm.com/en/products/databook/datasheet/passive/resistor/chip_resistor/ltr-e.pdf)
- **TT-VCR** · TT electronics Welwyn, *VCR Characteristics of High Voltage Thick Film Resistors*. [mouser.com
  mirror](https://www.mouser.com/pdfdocs/Welwyn-VCR-Characteristics.pdf)
- **CC469**, **CC1140**, **CC363**, **CC639**, **CC1009**, **CC1192** · Coilcraft documents 469 *Selecting the Best
  Inductor for Your DC-DC Converter*, 1140 *Looking Beyond the Static Data Sheet*, 363 *Measuring Self Resonant
  Frequency*, 639 *Selecting Coupled Inductors for SEPIC Applications*, 1009 *Common Mode Filter Chokes for High Speed
  Data Interfaces*, 1192 *Notes on Thermal Aging in Inductor Cores*.
  [coilcraft.com](https://www.coilcraft.com/en-us/resources/application-notes/)
- **CCeBook**, **CCcpl**, **CCcmf** · Coilcraft, *The Fundamentals of Power Inductors* (eBook, 2021), *A Guide to
  Coupled Inductors*, *Common Mode Filter Inductor Analysis*.
  [eBook](https://www.coilcraft.com/getmedia/bec1987d-3725-4d3a-a655-d3a9227abcad/Coilcraft-eBook_Power_Inductors.pdf),
  [coupled](https://www.coilcraft.com/en-us/edu/series/a-guide-to-coupled-inductors/),
  [CM](https://www.coilcraft.com/en-us/resources/application-notes/common-mode-filter-inductor-analysis/)
- **WE047**, **WE118**, **WE128** · Würth Elektronik ANP047c *The Behavior of Electro-Magnetic Radiation of Power
  Inductors*, ANP118a *Acoustic noise & Coil whine effect*, ANP128b *Introduction to Thermal Aging in Molded Power
  Inductors*. [we-online.com](https://www.we-online.com/en/support/knowledge/application-notes)
- **WE-MAPI** · Würth Elektronik, WE-MAPI 4020 74438356010 datasheet (Isat at 10 % and at 30 %, rated current at 40 K
  per IEC 62024-2). [we-online.com](https://www.we-online.com/components/products/datasheet/74438356010.pdf)
- **VI34252**, **VI34244** · Vishay Dale, *IHLP Selection Example* (34252: loss formulas, core loss under a third of the
  budget) and IHLP-2525EZ-A1 datasheet (34244). [34252](https://www.vishay.com/docs/34252/ihlpse.pdf),
  [34244](https://wwwlegacy.vishay.com/docs/34244/ihlp-2525ez-a1.pdf)
- **MMcat** · Micrometals, *Power Conversion & Line Filter Applications* catalog, Issue L, "Thermal Aging
  Considerations". [copy hosted by MIT](https://ee6131b.mit.edu/document/micrometals.pdf)
- **FX3C90** · Ferroxcube, *3C90 material specification* (saturation flux 470 mT at 25 °C, 380 mT at 100 °C).
  [ferroxcube.com](https://www.ferroxcube.com/upload/media/product/file/MDS/3c90.pdf)
- **MUbead** · Murata, *Monolithic Type Ferrite Bead*, metamorphosis no. 18.
  [murata.com](https://corporate.murata.com/technology/techmag/metamorphosis18/productsmarket/ferritebead)
- **TDK45B** · TDK, *Common mode filters ACT45B* catalog (CAN: 11 to 100 µH common mode, 0.05 to 0.2 µH stray).
  [tdk.com](https://product.tdk.com/system/files/dam/doc/product/emc/emc/cmf_cmc/catalog/cmf_automotive_signal_act45b_en.pdf)

## `PAS-1` Passive on a pin does not meet the selection its datasheet states <a id="pas-1"></a>

⚠️ warning · needs: the part's `required` selection constraints, the technology and parameters of the passive on the pin
· [catalogue entry](../checks.md#pas-1)

**Impl.** mid · depends on CFG-5, CFG-6, NET-23

**What it catches.** A capacitor, resistor or inductor on a pin whose datasheet asks for a kind of part (a technology, a
dielectric, an ESR window, a tolerance, a TCR, a termination), when the part fitted does not meet it and no more
specific PAS, RATE, RAIL or DEC check covers that parameter.

**Why it matters.** Datasheets say these things in prose ("use X5R or X7R", "a low-ESR ceramic", "ESR between 0.1 and 1
Ω", "a 1 % resistor") and a schematic review reads them once, if at all. A part that meets the value but not the kind
passes every value check and fails on the bench or in the field.

**Example.** An STM32's VCAP pin with a 2.2 µF Y5V capacitor: the value is right, the datasheet asks for a low-ESR
ceramic whose capacitance holds; or a regulator's COMP pin with an electrolytic where the datasheet asks for a ceramic.

**How to check.** For each `required` entry of each part, find the passives that match its topology (`between`,
`series`, `on`), resolve their parameters, and evaluate every selection constraint; constraints whose parameter has its
own check report there (leakage under `PAS-2`, dielectric under vibration under `PAS-8`, TCR under `PAS-22`, an LDO's
ESR window under `RAIL-8`), the rest here. The finding quotes the datasheet sentence and page from the constraint's
`source`.

**Fix.** Fit a part of the kind the datasheet asks for, or waive with the reason the datasheet's condition does not
apply.

**References.** The constraint format: better_erc design note *passive-selection* §5.1.

## `PAS-2` Capacitor leakage on a current-set or reference pin <a id="pas-2"></a>

⚠️ warning · needs: the pin's source current and accuracy, its voltage, the project's maximum ambient, the capacitor's
technology and leakage specification · [catalogue entry](../checks.md#pas-2)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-5, CFG-7, RPT-26

**What it catches.** A capacitor on a pin that sets a voltage through a precision current (the SET pin of the LT3045,
LT3042, LT3094 and LT3080, a reference's filter pin, a current-programmed reference node) whose leakage, at the pin's
voltage and the hottest ambient, is a noticeable fraction of that current. Electrolytic, polymer and tantalum parts
specify leakage in microamps; ceramics and film in nanoamps or less.

**Why it matters.** On these regulators the output is `Vout = ISET × RSET` with ISET 100 µA (LT3045, LT3042, LT3094) or
10 µA (LT3080). Any current that leaks out of the SET node is subtracted: the LT3045 datasheet says "Capacitor leakage
of even 100nA is a 0.1% DC error. Therefore, LTC recommends the use of a good quality low leakage ceramic capacitor";
the LT3080's 10 µA makes 10 nA a 0.1 % error. At datasheet maxima a 4.7 µF 6.3 V polymer tantalum may leak 0.1·C·V ≈ 3
µA (3 % low), a wet aluminium part 3 µA or more, an MnO2 tantalum 0.5 µA (0.5 %), while an X7R MLCC's insulation
resistance (≥ 500 MΩ·µF) keeps it near 30 nA at 3.3 V. Leakage of electrolytics and tantalums rises several times at 85
°C and above and is highest after storage. The datasheets do not attribute oscillation to this leakage: the loop
tolerates the SET capacitor's high ESR and ESL, and the erratic behaviour they describe comes from an unbypassed SET
node.

**Example.** An LT3045 at 3.3 V (RSET 33.2 kΩ) with a 4.7 µF polymer tantalum on SET for lower noise in a vibrating
enclosure: correct about piezo noise (`PAS-8`), but at the polymer's specified leakage the output can sit tens of
millivolts low, more when hot.

**How to check.** From the pin's electrical facts (`function = current_set` or `reference`, `source_current` with its
spec at 25 °C and over the range, pin voltage; a datasheet's own `leakage` constraint wins) and the capacitor's leakage
model (the series' DCL formula or insulation resistance, scaled to the pin voltage and the board's hottest ambient from
its intent, or 25 °C with a note when the board states none: `RPT-26`), report when `Ileak / ISET` exceeds the budget:
`passive_share` (25 %) of the part's own SET-current error at the same condition, so 0.25 % (250 nA) at 25 °C and 0.5 %
over the range on an LT3045 (the page's introduction says how the budget is derived). A generic profile (`SET`, `ISET`
on a regulator) supplies an assumed current and accuracy and makes the finding an estimate.

**Fix.** A low-leakage ceramic (class 1 where the value allows, else X7R) or a film capacitor; where vibration rules out
ceramics, film, or a tantalum whose specified leakage fits the budget. Guard ring and clean board as the datasheet
describes.

**References.** LT3045 p. 15 to 16 (leakage paths, guard ring, 100 nA = 0.1 %), p. 18 (piezo, tantalum/electrolytic/film
allowed), p. 19 (bypass capacitor leakage, low-leakage ceramic); LT3042 and LT3094, the same sections; LT3080, "Ten
nanoamperes of leakage into or out of the SET pin ... creates a 0.1% error"; KEM-POLY (DCL ≤ 0.1·C·V); KEM-X7R
(insulation resistance).

## `PAS-3` Capacitor leakage comparable to the charging current of a soft-start, tracking or timing pin <a id="pas-3"></a>

🛑 error · needs: the pin's charge current, the threshold the ramp must reach, the capacitor's leakage · [catalogue
entry](../checks.md#pas-3)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-5, RPT-26

**What it catches.** A soft-start (SS, TR/SS), tracking or timing capacitor charged by a small internal current source
(1 to 10 µA is common) whose leakage is a sizeable fraction of that current: the ramp is slow and unpredictable, and
when leakage reaches the current the pin never gets to its threshold.

**Why it matters.** A capacitor charged by a current source settles where the source current equals the leakage. The
ADP7118's SS pin is charged by 1.15 µA and its start-up time is `tSTART-UP + 0.6 V × CSS / ISS`; a wet aluminium or
polymer capacitor with a microamp of leakage at that voltage stretches the start-up by orders of magnitude or stops it.
The regulator then never reaches regulation or never asserts power-good, and only on some boards or only when hot.

**Example.** A 10 µF electrolytic on a converter's SS pin (2 µA source) to get a slow start for a large load: its
specified leakage of 3 µA means the pin stops below the soft-start end voltage, and the output stays low.

**How to check.** From the pin's facts (`function = soft_start | tracking | timing`, `source_current`, the end voltage)
and the capacitor's leakage at that voltage and the board's hottest ambient (25 °C with a note when the board states
none): error when `Ileak ≥ 0.5 × Isource`, warning when the timing moves by more than 10 %.

**Fix.** A ceramic (X7R or C0G) soft-start capacitor; for very long ramps a different soft-start method (a slower
reference, an RC on the enable, or a sequencer).

**References.** ADP7118 Rev. H, specification table (soft-start source current 1.15 µA) and "Soft Start" (`SSTIME`
equation); KEM-POLY; PAN-FR; NIC-POLY (leakage of electrolytics and polymers).

## `PAS-4` Hold-up or backup capacitor whose own leakage drains what it holds <a id="pas-4"></a>

⚠️ warning · needs: the capacitor's leakage or self-discharge, the load it holds and the hold time wanted · [catalogue
entry](../checks.md#pas-4)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-2, RPT-26

**What it catches.** A capacitor that must hold a voltage for a long time (an RTC's VBAT supercapacitor, a memory
hold-up, a sample-and-hold or peak detector capacitor, a timer's storage capacitor) whose own leakage or self-discharge
is comparable to the current it is meant to supply or the droop allowed.

**Why it matters.** The hold time is computed as `C × ΔV / Iload`; the capacitor's leakage adds to `Iload`. A
supercapacitor's leakage current after charging is microamps, the same order as an RTC's backup current; an
electrolytic's is similar. The board then keeps time for hours instead of days.

**Example.** A 0.1 F supercapacitor on a 1 µA RTC backup pin for a computed day of hold-up: with a few µA of leakage it
lasts a few hours.

**How to check.** For capacitors in the role `Hold` (VBAT of an RTC with no battery, a hold-up node, a sample-hold),
compare the leakage at the held voltage with the load current from the pin's facts (or the project's hold-up time), and
report when leakage is above a fraction (default 20 %) of the load.

**Fix.** A lower-leakage part (a lithium cell, a low-leakage supercapacitor series, a film or C0G capacitor for
sample-and-hold), or a hold time computed with the leakage included.

**References.** EAT-SC (supercapacitor leakage and self-discharge); ADI-BLD (sample-and-hold capacitor choice).

## `PAS-5` Resistors above about 1 MΩ in a precision divider or bias network <a id="pas-5"></a>

⚠️ warning · needs: the network's resistor values and role, the pin's input current, the project's environment ·
[catalogue entry](../checks.md#pas-5)

**Impl.** mid · depends on NET-7, NET-23, CFG-2, RPT-6

**What it catches.** A divider or bias network made of resistors in the megohm range on a node whose accuracy matters (a
voltage measurement, a feedback divider of a low-power regulator, an amplifier bias), where board surface leakage, the
resistors' own voltage coefficient and stability, and the input current of the pin it feeds are no longer small against
the current in the network.

**Why it matters.** At 10 MΩ a microamp is 10 V; flux residues and humidity on the board give paths of hundreds of
megohms to gigaohms, and thick-film chip resistors of high value have larger voltage coefficients and drift. The divider
works on the bench, drifts in a humid room, and fails after a cleaning-free rework.

**Example.** A 10 MΩ / 1 MΩ divider measuring a 48 V bus into an ADC on an uncoated board: 100 MΩ of surface leakage in
parallel with the 10 MΩ part is a 10 % error.

**How to check.** For dividers and bias networks in a precision role (`RPT-6` finds them), report when a resistor is
above a threshold (default 1 MΩ, configurable) or the network current falls below 100 times the pin's input current plus
an assumed board leakage at the project's environment (`condensing` lowers it).

**Fix.** Lower the impedance (switch the divider if quiescent current was the reason: `LEAK-7`), buffer the node, use
thin-film high-value parts, guard and coat the node.

**References.** VPG-TN110 (moisture and shunting); K-RK73B and P-ERJS (TCR of high values); ANA-5 for the pin's input
current; LT3045 p. 15 to 16 for guard rings and humidity on high-impedance nodes.

## `PAS-6` Class 2 or 3 ceramic where the capacitance sets a frequency, time or corner that must hold <a id="pas-6"></a>

⚠️ warning · needs: the capacitor's role (timing, oscillator, filter, integrator) and its dielectric · [catalogue
entry](../checks.md#pas-6)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-7

**What it catches.** X5R, X7R, X6S, X7S, Y5V or Z5U capacitors that set a time or a frequency: RC timers and one-shots,
RT/CT pins of oscillators and converters, VCO tank and PLL loop filter capacitors, active filter capacitors,
integrators, LC oscillators. Crystal load capacitors are `CLK-2`'s.

**Why it matters.** Class 2 capacitance moves by percent where class 1 moves by ppm: ±15 % over temperature for X7R, a
few percent per decade of hours of ageing (reset by every reflow), tens of percent lower at a DC bias, and lower again
at small AC levels than at the 1 Vrms it is specified at. A timer computed for 1 s runs 0.7 s hot, a filter corner
moves, a PLL loop loses phase margin.

**Example.** A converter's CT pin with a 1 nF X7R: the switching frequency drifts with temperature and differs board to
board after reflow.

**How to check.** For capacitors in the roles `Timing`, `LoopFilter`, `FilterCorner`, `Integrator` and `Oscillator`
(from pin functions and topology), report a class 2 or 3 dielectric. Values above what class 1 offers in that case size
get a note that film or a different time constant (larger R, smaller C) is the way out.

**Fix.** C0G/NP0 (available up to about 0.1 µF in small cases), PPS or PP film above that; or redesign for a larger
resistor and a smaller capacitor.

**References.** ADI-CER (temperature and bias); MUR-VOLT (DC and AC level); MUR-AGE and KEM-X7R (ageing, 3 % per
decade-hour, and de-ageing above the Curie point); KEM-C0G (±30 ppm/K, no ageing); MUR-GRM (EIA codes); ST-AN2867 for
the crystal case.

## `PAS-7` Class 2 ceramic in an audio or precision analog signal path <a id="pas-7"></a>

⚠️ warning · needs: the signal path (net class, role), the capacitor's dielectric · [catalogue
entry](../checks.md#pas-7)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-2

**What it catches.** X5R, X7R or higher-K capacitors where a signal voltage swings across them: audio coupling
capacitors, gain-setting and feedback capacitors, active filter capacitors in audio or precision measurement paths.

**Why it matters.** The capacitance of a class 2 dielectric depends on the voltage across it, so a signal modulates its
own filter or coupling: harmonic distortion that C0G and film capacitors do not add. In a coupling capacitor with a
large DC bias and a small signal the effect is smaller; in a feedback or filter capacitor carrying the signal it can
dominate the distortion budget.

**Example.** A Sallen-Key filter in an audio ADC front end with X7R capacitors: THD rises by an order of magnitude
against the same filter with C0G.

**How to check.** For capacitors in `Coupling`, `FeedbackCap` and `FilterCorner` roles on nets of an `Audio` or
`Precision` class, or anywhere on a board whose intent is `audio` or `precision`, report a class 2 or 3 dielectric where
a signal voltage is across it.

**Fix.** C0G/NP0 for small values, PPS, PP or PET film for larger ones; for coupling capacitors, enough capacitance that
the signal across it is small.

**References.** ADI-BLD (high-K ceramics as distortion producers, "unsuitable for signal path filtering"); ADI-CER;
MUR-VOLT.

## `PAS-8` Piezoelectric ceramic on a low-noise, high-impedance node in a product that vibrates <a id="pas-8"></a>

⚠️ warning · needs: the node's role (noise-critical, high impedance), the capacitor's dielectric, the board's intent
`vibration` (not run without it) · [catalogue entry](../checks.md#pas-8)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-2

**What it catches.** Class 2 or 3 ceramic capacitors on nodes where a few microvolts matter and the impedance is high:
the SET pin of the LT3045 family, the NR, BYP and REF pins of low-noise regulators and references, PLL loop filters, the
inputs of high-gain sensor front ends, in a product the project marks as subject to vibration or mechanical shock.

**Why it matters.** Ferroelectric ceramics are piezoelectric: vibration and thermal transients make them generate a
voltage. The LT3045 datasheet: "given the high SET pin impedance, any piezoelectric response from a ceramic SET pin
capacitor generates significant output noise – peak-to-peak excursions of hundreds of µVs", in a regulator specified for
0.8 µVrms. It allows "any non-piezoelectrically responsive (tantalum, electrolytic, or film) capacitor" there, noting
electrolytics' higher 1/f noise and that leakage still counts (`PAS-2`).

**Example.** An LT3045 powering a clock synthesiser on a drone, with a 4.7 µF X7R on SET: the phase noise shows the
motor vibration.

**How to check.** On a board whose intent includes `vibration`, report class 2 and 3 capacitors in the roles
`SetBypass`, `NoiseFilter`, `RefDecoupling` and `LoopFilter`; a datasheet constraint (`required.when.intent =
vibration`) states it for a specific part and cites the page.

**Fix.** Film (PPS, PEN), C0G where the value allows, or a tantalum whose leakage fits `PAS-2`; mechanically, smaller
cases and soft mounting help.

**References.** LT3045 p. 17 to 18 ("Stability and Output Capacitance", piezoelectric noise generators); LT3042 and
LT3094, the same section; KEM-C0G ("No piezoelectric noise").

## `PAS-9` Passive that sings: audio-band excitation of a class 2 ceramic or an open-core inductor <a id="pas-9"></a>

💡 suggestion (⚠️ warning on a board whose intent is `audio`) · needs: the converter's mode (PFM, burst, PWM in the audio
band) or the load's pulse rate, the parts' dielectric and construction, the board's intent · [catalogue
entry](../checks.md#pas-9)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-2, CFG-5

**What it catches.** Class 2 MLCCs and unshielded or drum-core inductors that carry ripple in the audio band: the
outputs of converters that enter PFM or burst mode at light load, PWM dimming and motor drive at audio frequencies,
class-D amplifier outputs, rails feeding pulsed loads (a GSM modem's 217 Hz bursts).

**Why it matters.** Class 2 ceramics change their dimensions with voltage (electrostriction) and bend the board;
inductors' windings and cores move with the current. At audio frequencies this is heard as a whine or buzz: a product
defect in consumer devices, and a sign of stress.

**Example.** A buck converter in pulse-skipping mode at 3 kHz at standby with 22 µF 1206 X5R output capacitors: the
board hums in a quiet room.

**How to check.** For capacitors and inductors in the roles `OutputCap`, `InputCap`, `PowerInductor` of regulators whose
part data names a light-load mode in the audio band (or a PWM frequency under 20 kHz from the project), report class 2
ceramics (not of a low-acoustic-noise series) and unshielded or drum-core inductors: as a suggestion, raised to a
warning on a board whose intent is `audio`, where a whine is a product defect.

**Fix.** Forced PWM above 20 kHz, low-acoustic-noise MLCC series (Murata ZRA, ZRB, or KRM and KR3 with metal terminals),
polymer or tantalum capacitors in the worst spot, moulded composite inductors.

**References.** TI-SSZTB09; MUR-NOISE; WE118 and TI946 (inductors, light-load ripple frequencies).

## `PAS-10` Dielectric absorption where a capacitor must forget its past voltage <a id="pas-10"></a>

⚠️ warning · needs: the capacitor's role (hold, integrator reset, peak detector, charge measurement) and technology ·
[catalogue entry](../checks.md#pas-10)

**Impl.** mid · depends on NET-7, NET-23, NET-24

**What it catches.** Class 2 ceramics and electrolytics as the hold capacitor of a sample-and-hold, the capacitor of an
integrator that is reset by a switch, a peak detector's storage capacitor, or a capacitor whose charge is measured.

**Why it matters.** A dielectric that has been charged keeps part of that charge in its slow polarisation: after a short
discharge the voltage creeps back by a fraction of the old value. ADI's table: polypropylene and polystyrene 0.001 to
0.02 %, NP0 ceramic under 0.1 %, high-K ceramic 0.2 % and more, tantalum 1 to 5 %, some electrolytics 10 % or more. A
sample-and-hold then remembers the previous sample, an integrator restarts from an offset.

**Example.** A 100 nF X7R as the integrating capacitor of a charge amplifier reset every millisecond: each reading
carries a few tenths of a percent of the last one, half an LSB at 8 bits.

**How to check.** For capacitors in the roles `Hold`, `Integrator` (with a reset switch across it) and `PeakDetector`,
report technologies with dielectric absorption above a threshold (default 0.1 %: anything but class 1 ceramic and PP, PS
or PTFE film).

**Fix.** Polypropylene, polystyrene or PTFE film; C0G for small values.

**References.** ADI-BLD; VIS-FILM (PET 0.2 %, PP 0.05 %, PP for sample and hold).

## `PAS-11` Class 3 dielectric (Y5V, Z5U) where capacitance matters <a id="pas-11"></a>

💡 suggestion · needs: the capacitor's dielectric · [catalogue entry](../checks.md#pas-11)

**Impl.** mid · depends on NET-24, CFG-7

**What it catches.** Y5V, Z5U and similar class 3 capacitors anywhere their capacitance is relied on: decoupling a part
with a minimum, a regulator's input or output, a filter, a timer.

**Why it matters.** Y5V is specified +22/−82 % over −30 to 85 °C, loses most of its capacitance at a DC bias, and ages
several percent per decade-hour: a 10 µF part can be 2 µF at temperature and bias. It is cheap and almost never the
right choice on a modern board.

**Example.** A 10 µF Y5V on an LDO's output that needs at least 1 µF: fine on the bench at room temperature, unstable in
the cold.

**How to check.** Report every class 3 dielectric whose role is not `Bulk` with an explicit waiver; with the role known,
raise to a warning where a minimum capacitance applies (`RAIL-21`, `DEC-6`).

**Fix.** X5R or X7R of a higher rated voltage or larger case.

**References.** ADI-CER (a 4.7 µF Y5V 0603 at 5 V is 0.33 µF); MUR-VOLT; MUR-GRM.

## `PAS-12` Class 2 ceramic or polyester film in a high-dV/dt or high-AC-current position <a id="pas-12"></a>

⚠️ warning · needs: the capacitor's role (snubber, resonant, class-D filter, capacitive dropper) and technology ·
[catalogue entry](../checks.md#pas-12)

**Impl.** mid · depends on NET-6, NET-23, NET-24

**What it catches.** X7R and other class 2 ceramics or PET (polyester) film capacitors in RC snubbers across switches
and diodes, resonant capacitors of LLC and other resonant converters, class-D output filters, and capacitive droppers on
mains.

**Why it matters.** These positions carry large AC currents at high frequency. Class 2 ceramics have a dissipation
factor of a few percent and lose capacitance with AC voltage; PET film's loss rises with frequency and temperature. The
part heats, its value moves (detuning a resonant tank), and it fails early. C0G and polypropylene have losses a hundred
times lower.

**Example.** A 1 nF X7R snubber across a flyback's primary switch: it runs hot and its capacitance falls with the 400 V
ringing, so the snubber damps less than designed.

**How to check.** For capacitors in the roles `Snubber`, `Resonant`, `ClassDFilter` and `Dropper`, report class 2
dielectrics and PET film; the dV/dt and ripple ratings themselves are `RATE-30` and `RATE-25`.

**Fix.** C0G/NP0 ceramics (high-voltage C0G series exist) or polypropylene film.

**References.** VIS-FILM; KEM-X7R and KEM-C0G (dissipation factor).

## `PAS-13` Switching regulator output capacitor technology not what its compensation assumes <a id="pas-13"></a>

⚠️ warning · needs: the regulator's compensation type from its part data or `required`, the technology and ESR of the
output capacitors · [catalogue entry](../checks.md#pas-13)

**Impl.** mid · depends on NET-6, NET-7, NET-23, NET-24, CFG-5, CFG-6

**What it catches.** A switching regulator whose loop relies on the ESR zero of an electrolytic or tantalum output
capacitor (many voltage-mode and some constant-on-time designs, type II compensation) fitted with ceramics only, or a
part made for ceramic outputs (internally compensated current mode with a fixed zero) fitted with a high-ESR bulk
capacitor alone.

**Why it matters.** The output capacitor's ESR puts a zero in the loop. Remove it by swapping to MLCCs and the loop
loses phase margin and rings or oscillates; constant-on-time parts without ripple injection need some ESR ripple to
switch cleanly. The other way round, a high-ESR capacitor on a part designed for ceramics adds ripple and a zero the
compensation did not plan for. LDOs are `RAIL-8`'s.

**Example.** A voltage-mode buck controller reference design with a 220 µF electrolytic, rebuilt with 2 × 47 µF MLCCs to
save height: it oscillates at light load.

**How to check.** From the regulator's part data (`regulator.compensation`, a `required` entry on its output with an
`esr` window or a `technology` set) and the output capacitors found by role `OutputCap`, compare the parallel ESR with
the window, or the technologies with the set.

**Fix.** The capacitor technology the design was compensated for, a different compensation network, or ripple injection
for COT parts.

**References.** TI-SLVA115 (ESR and stability); the regulator's datasheet, output capacitor section.

## `PAS-14` Wet aluminium electrolytic on a product rated for the cold <a id="pas-14"></a>

⚠️ warning · needs: the project's minimum ambient, the capacitor's series and its low-temperature impedance ratio ·
[catalogue entry](../checks.md#pas-14)

**Impl.** mid · depends on NET-23, NET-24, CFG-2, CFG-7

**What it catches.** Wet aluminium electrolytic capacitors relied on for filtering, bulk or hold-up on a product whose
ambient goes below about −20 °C, when the series' impedance ratio Z(−25 or −40 °C)/Z(20 °C) leaves less than the role
needs, or when the project's minimum is below the series' category temperature.

**Why it matters.** The electrolyte's conductivity falls in the cold: at the series' lowest rated temperature the ESR is
10 to 100 times its room-temperature value (CDE), and below it nothing is specified. A converter's input filter stops
filtering, a hold-up capacitor cannot deliver its current, an LDO that needs output ESR in a window leaves it. Polymer
and hybrid capacitors change far less.

**Example.** An outdoor sensor rated −40 °C with a general-purpose 85 °C electrolytic (Z ratio 4 at −25 °C, not
specified at −40 °C) as the input bulk of a buck: brown-outs at cold start.

**How to check.** For wet aluminium capacitors in roles `InputCap`, `OutputCap`, `InputBulk`, `Hold`, compare the
minimum of the board's `ambient` intent (not run without it) with the series' category minimum and report a ratio above
a threshold (default 4) at the lowest specified temperature.

**Fix.** A low-temperature series, a polymer or hybrid capacitor, or ceramics where the value allows.

**References.** NIC-AL (impedance ratio, low-temperature behaviour); CDE-AL.

## `PAS-15` Polarized capacitor where the voltage across it can reverse or is AC <a id="pas-15"></a>

⚠️ warning · needs: the voltage ranges of both nets, the capacitor's polarity · [catalogue entry](../checks.md#pas-15)

**Impl.** mid · depends on NET-5, NET-10, NET-24, CFG-2

**What it catches.** An electrolytic, tantalum or polymer capacitor whose two nets can swap order: a coupling capacitor
between two signals biased at similar or changing levels, a capacitor between two rails that sequence in either order, a
capacitor on an AC or bipolar signal. A fixed DC reversal is `DEC-12`'s.

**Why it matters.** Polarized capacitors tolerate at most a volt or so (tantalum: a small percentage of rating) in
reverse; beyond it they leak, heat and, for MnO2 tantalum, can fail short. An audio coupling capacitor between a 0
V-biased input and a source that is sometimes 0 V and sometimes biased sees both directions.

**Example.** A 10 µF tantalum coupling an op-amp output biased at 2.5 V to a connector that another device biases at 5
V: reversed whenever the cable is plugged in.

**How to check.** For polarized capacitors, take both nets' voltage ranges (rails, project data, bias found by `NET-10`)
and report when the anode can be below the cathode by more than the series' reverse limit.

**Fix.** A non-polarized capacitor (film, ceramic, a bipolar electrolytic), or a bias that keeps the polarity.

**References.** AVX-TECH (tantalum reverse limits: 10 % of rating up to 1 V at 25 °C, 1 % at 125 °C); NIC-AL.

## `PAS-16` Cable-fed or hot-plugged power input with only low-ESR ceramics <a id="pas-16"></a>

⚠️ warning · needs: which inputs are cable-fed or hot-plugged (connector, project), the technology of every capacitor on
the input net · [catalogue entry](../checks.md#pas-16)

**Impl.** mid · depends on NET-5, NET-23, NET-24, CFG-2

**What it catches.** A power input that arrives through a cable or is plugged live, whose input net has only MLCCs (and
the regulators behind them), with no damping element: no electrolytic or polymer bulk capacitor with ESR, no series
resistance, no clamp.

**Why it matters.** The cable's inductance and the ceramic capacitance form an LC tank with almost no damping. At
plug-in the voltage overshoots to nearly twice the supply and rings, beyond the regulator's absolute maximum on 12 V and
24 V inputs. A capacitor with ESR (or a series resistor) damps it; a TVS clamps it. `PIN-14` is the missing bulk
capacitor; this is bulk that cannot damp.

**Example.** A 24 V input on a terminal block with 2 × 10 µF X7R in front of a 36 V-rated buck: hot-plugged on a
two-metre cable it peaks near 45 V.

**How to check.** For inputs marked cable-fed or hot-pluggable (connector power pins, project `kind = external`), report
when every capacitor on the net is a ceramic and no damping element (a capacitor with ESR in the damping range, a series
resistor, a TVS) is present.

**Fix.** An electrolytic or polymer bulk capacitor with suitable ESR in parallel, a TVS, or an RC damper.

**References.** LT-AN88 (ceramic input capacitors and hot plugging); PIN-14.

## `PAS-17` Large MLCC across a battery or unfused high-energy input with standard termination <a id="pas-17"></a>

⚠️ warning · needs: the capacitor's case and termination, whether the net is battery-direct or unfused · [catalogue
entry](../checks.md#pas-17)

**Impl.** mid · depends on NET-5, NET-23, NET-24, CFG-7

**What it catches.** MLCCs of 1206 and larger (1210, 1812, 2220 especially) connected directly across a battery or a
high-energy input with no fuse between, built with standard terminations.

**Why it matters.** Board flexure (depaneling, screws, connectors, a dropped device) cracks large ceramics (TDK: a
standard 3216 part cracks at about 4 mm of board flex, a soft-termination part takes twice that); a crack through the
electrodes is a resistive short. On a source that can deliver tens of amps the cracked capacitor heats until it burns
the board. Flexible (soft) terminations absorb the bend, fail-open designs and series-stacked capacitors turn the crack
into an open or a half capacitor.

**Example.** A 22 µF 1210 X7R directly across a 2S lithium pack: a crack after a drop test smoulders.

**How to check.** For capacitors with role `BatteryDirect` or on an input with no fuse or current limit between it and
the source, report case 1206 and up without a soft, flex or fail-open termination (series table, `termination`).
Distance to the board edge, V-scores, screws and connectors is a layout check (`LAY`).

**Fix.** A soft or flexible termination (KEMET FT-CAP, TDK soft termination) or open-mode part, two capacitors in
series, a fuse, smaller cases, and placement away from flex points.

**References.** TDK-FLEX; KEM-FLEX.

## `PAS-18` Silver thick-film resistors in a sulfur environment <a id="pas-18"></a>

⚠️ warning · needs: the project's environment, the resistors' series (anti-sulfur or not) · [catalogue
entry](../checks.md#pas-18)

**Impl.** mid · depends on NET-24, CFG-2, CFG-7

**What it catches.** Standard thick-film chip resistors (silver inner electrodes) on a product whose environment
contains sulfur compounds: industrial plants, automotive engine bays, near rubber, tyres and gaskets, agriculture, oil
and gas, places with diesel exhaust or volcanic air.

**Why it matters.** Sulfur reaches the silver under the protective coat at its edge and turns it into silver sulfide,
which does not conduct: the resistor goes open after months or years, first the smallest cases. Anti-sulfur series use a
gold or palladium-rich electrode or a barrier and survive.

**Example.** A pump controller with ordinary 0402 thick-film resistors in a wastewater plant: field returns after two
years with open pull-ups.

**How to check.** On a board whose `environment` intent contains `sulfur` (not run on a board that states no
environment), report thick-film resistors whose series is not anti-sulfur.

**Fix.** Anti-sulfur series (Vishay CRCW-SR, Panasonic ERJ-S and ERJ-U, Yageo AF), thin-film or metal-strip parts
(sulfur resistant by construction), or conformal coating as a partial remedy.

**References.** V-CRCWSR; P-ERJS; Y-AF.

## `PAS-19` Thin-film or small thick-film resistor where pulses or ESD arrive <a id="pas-19"></a>

⚠️ warning · needs: the resistor's role (exposed series resistor, inrush limiter, discharge path, snubber), its
technology and case · [catalogue entry](../checks.md#pas-19)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-7

**What it catches.** Thin-film resistors and small (0402, 0201) standard thick-film resistors in positions that take
pulses: series resistors on connector lines in front of protection, the resistor of an RC input filter at a connector,
inrush limiters, bleeder and discharge resistors of bulk capacitors, snubber resistors.

**Why it matters.** Thin films are thin: ESD and surge energy concentrate in the trimming cut and shift the value or
open it. Small standard thick-film chips have little mass for a pulse and a lower ESD rating (Vishay CRCW: 400 V HBM in
0402, 2000 V in 1206). Anti-surge thick-film, MELF and carbon composition parts are built for it. The energy of a known
pulse against the part's pulse curve is `RATE-27`; this check is the technology on an exposed line, where the pulse is
not known.

**Example.** A 100 Ω 0402 thin-film resistor between a USB data line connector and an ESD diode: after an ESD event the
line works but the resistor has drifted to 200 Ω.

**How to check.** For resistors in roles `ConnectorSeries`, `InrushLimiter`, `Discharge` and `Snubber`, report thin film
and thick film below a case size (default 0603) not of an anti-surge series.

**Fix.** Anti-surge thick-film (Panasonic ERJ-P, Rohm ESR, Vishay CRCW-HP), thin-film MELF (Vishay MMA, MMB), or a
larger case.

**References.** V-PULSE12; V-CRCW (ESD by case); P-ERJP; R-ESR; VPG-ESD (thin film shifts up to 5 % before it opens);
V-MELF.

## `PAS-20` Consumer-grade passives on an automotive design <a id="pas-20"></a>

💡 suggestion (⚠️ warning on a board whose intent is `automotive`) · needs: the board's intent, the passives' series and
their AEC-Q200 status · [catalogue entry](../checks.md#pas-20)

**Impl.** mid · depends on NET-24, CFG-2, CFG-7

**What it catches.** Capacitors, resistors and inductors of commercial series (Murata GRM, TDK C, Yageo RC) on a project
whose intent is automotive, where AEC-Q200 series exist (Murata GCM, TDK CGA, Yageo AC).

**Why it matters.** AEC-Q200 qualification covers temperature cycling, humidity, board flex and vibration at automotive
levels and comes with change control; a commercial part may be the same die or may not. Customers and audits ask for it,
and it is cheapest to choose at design time.

**Example.** An automotive CAN node with GRM capacitors throughout: the customer's PPAP asks for AEC-Q200 evidence on
every passive.

**How to check.** Report passives whose series is not AEC-Q200 (series table, mx3 `aec_q200`): a warning on a board
whose intent is `automotive`, a suggestion on one whose intent is `industrial` or harsh (`vibration`, an `ambient`
beyond −40 to 85 °C), nothing on other boards, and not run on a board that states no intent.

**Fix.** The AEC-Q200 series of the same vendor and size.

**References.** Y-AF and Y-RVP (automotive-grade series beside the commercial ones); KEM-FLEX (flex mitigation series
AEC-Q200 qualified).

## `PAS-21` Capacitor across mains or an isolation barrier that is not safety rated <a id="pas-21"></a>

🛑 error · needs: which nets are mains or the two sides of a barrier, the capacitor's safety class · [catalogue
entry](../checks.md#pas-21)

**Impl.** mid · depends on NET-5, NET-23, NET-24, CFG-2

**What it catches.** A capacitor between mains conductors (line to line, line to neutral) that is not X-class, or
between a mains or hazardous side and protective earth or the touchable side of an isolation barrier that is not Y-class
(Y1 for reinforced isolation, Y2 for basic).

**Why it matters.** IEC 60384-14 defines X capacitors to survive mains surges and fail safely (open, self-healing film),
and Y capacitors to never fail short, because a shorted Y capacitor puts mains on the user. An ordinary MLCC or film
capacitor of a high enough voltage rating is neither; a crack or breakdown is a short. Isolation ratings of the barrier
as a whole are `GND-6`'s; whether a Y capacitor should be there is `GND-2`'s.

**Example.** A 4.7 nF 3 kV general-purpose MLCC across an isolated converter's barrier as the EMI capacitor: it passes
the voltage rating and is not a Y capacitor.

**How to check.** For capacitors with role `MainsX` (both nets mains, by project or net class) or `BarrierY` (nets on
two isolation sides, `GND`), report a missing or wrong safety class.

**Fix.** An X2 film capacitor across the line, a Y1 or Y2 ceramic or film capacitor (film, single-disc ceramic or
safety-certified MLCC families) across the barrier.

**References.** VIS-SAFETY (IEC 60384-14 classes: X1 4 kV, X2 2.5 kV, Y1 8 kV, Y2 5 kV impulse); VIS-FILM.

## `PAS-22` Precision resistor whose drift over temperature and life exceeds its role's budget <a id="pas-22"></a>

⚠️ warning · needs: the resistor's role and budget, its TCR and stability, the project's temperature range · [catalogue
entry](../checks.md#pas-22)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-2, RPT-26

**What it catches.** Resistors in roles where their value sets an accuracy (the RSET of a current-set regulator, a
reference or feedback divider, gain resistors, bridge completion, ADC scaling) whose drift over the temperature range
and over life is larger than the role allows; two resistors of one ratio with different technologies or TCRs. Tolerance
alone is `BOM-16`'s.

**Why it matters.** Thick-film chips are ±100 to ±200 ppm/K and drift up to about 1 % in load life; thin film is ±5 to
±50 ppm/K and a tenth of the drift; foil better again. Over 100 K a 100 ppm/K part moves 1 %: more than a 0.1 %
tolerance buys. The LT3045 datasheet: "the SET pin resistor can become the limiting factor in achieving high accuracy.
Hence, it should be a precision resistor." In a ratio, matched TCRs cancel; mismatched ones add.

**Example.** An LT3045 with a 0.1 % thick-film RSET: precise at 25 °C, 1 % off at 85 °C.

**How to check.** For roles `SetResistor`, `FeedbackDivider`, `Gain`, `Bridge`, `AdcDivider`, compute the drift (`|TCR|
× ΔT + stability`, and for a ratio the TCR difference) with `RPT-26` and compare with the budget (the project's figure,
a datasheet constraint, else `passive_share` of the error of the part the network serves, from its spec: for an LT3045's
RSET a quarter of the 2 % full-range SET current spread, 0.5 %; the introduction says how), over the board's `ambient`
range (25 ± 30 °C with a note when it states none).

**Fix.** Thin-film or foil parts, matched networks for ratios (ratio TCR of a few ppm/K), or the same technology and
series for both resistors of a ratio.

**References.** LT3045 p. 15; V-BASICS (TCR and stability by technology); V-CRCW and V-TNPW (load-life drift); V-ACAS
(tracking).

## `PAS-23` Thick-film resistors in a high-voltage measuring divider <a id="pas-23"></a>

⚠️ warning · needs: the divider's voltage, its resistors' technology, value and case · [catalogue
entry](../checks.md#pas-23)

**Impl.** mid · depends on NET-5, NET-23, NET-24, RPT-6

**What it catches.** A divider that measures a high voltage (a DC bus, a mains rectifier, an insulation monitor) built
from standard thick-film chip resistors of high value in small cases, where the error from their voltage coefficient is
larger than the measurement allows. The voltage rating of each part is `RATE-9`'s.

**Why it matters.** Thick film conducts through a matrix of particles in glass; the more voltage across it, the more
paths open, so its resistance falls with voltage (the voltage coefficient is always negative). High resistivity inks,
small parts and more than a few volts make it significant: high-voltage thick-film series still specify 10 to 25 ppm/V,
and general-purpose parts are not specified at all. At several hundred volts across the top resistor that is a
percent-level error that depends on the voltage, so calibration at one point does not remove it. Thin film specifies
under 1 ppm/V.

**Example.** A 400 V bus measured through 4 × 1 MΩ 0805 thick film into a 10 kΩ bottom resistor: the reading is low by
an amount that grows with the bus voltage.

**How to check.** For dividers found by `RPT-6` whose top end sits on a net above a threshold (default 50 V), compute
the voltage per resistor and, with the series' voltage coefficient (or a thick-film default as an estimate), the ratio
error; report above the role's budget.

**Fix.** Thin-film high-voltage chips (Vishay TNPV), dedicated high-voltage series with a specified coefficient (Vishay
CRHV, Yageo RV), more resistors in series, or a lower voltage per part.

**References.** TT-VCR; V-CRHV; V-TNPV; V-BASICS.

## `PAS-24` Thick-film resistor with DC across it in a low-noise analog path <a id="pas-24"></a>

💡 suggestion · needs: the resistor's role (low-noise analog with a DC voltage across it), its technology · [catalogue
entry](../checks.md#pas-24)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-2

**What it catches.** Thick-film or carbon resistors with a DC voltage across them in a path where low-frequency noise
matters: a reference divider, a sensor bias network, the feedback network of a low-noise preamplifier, an LDO's output
divider on a low-noise rail.

**Why it matters.** Besides thermal noise, a resistor carrying DC adds current noise (excess, 1/f noise) proportional to
the DC voltage. Vishay's overview gives thick film up to 10 µV/V against under 0.1 µV/V for metal film and less for
foil; it grows with value and with smaller cases. In a low-noise reference path it can dominate the noise budget below a
kilohertz.

**Example.** A 24-bit ADC's external reference set by a thick-film divider from a precision 5 V source: the ADC's
low-frequency noise floor rises.

**How to check.** For resistors in roles `RefDivider`, `Bias` and `FeedbackDivider` on nets of an `Analog`, `Precision`
or `Audio` class (or a regulator flagged low-noise in its part data), with a DC voltage across them, report thick-film
and carbon technologies.

**Fix.** Thin-film or metal-film resistors (or foil for the lowest noise).

**References.** V-BASICS; VPG-F106; VPG-INTRO.

## `PAS-25` Current-sense shunt whose technology limits the accuracy <a id="pas-25"></a>

⚠️ warning · needs: the shunt's value, technology, terminals, its whole-part TCR · [catalogue
entry](../checks.md#pas-25)

**Impl.** mid · depends on NET-23, NET-24, RPT-11

**What it catches.** Resistors used as current shunts (between a sense amplifier's inputs, or in a converter's
current-sense path) that are standard thick-film chips, or two-terminal parts below about 10 mΩ where the terminations
and solder are a sizeable part of the value. Kelvin routing and copper in the path are `ANA-4` and `ANA-23`.

**Why it matters.** Metal-strip and metal-plate shunts have elements under 20 ppm/K, but the whole part including its
copper terminals is specified at ±75 ppm/K at best and ±400 ppm/K below 1 mΩ (Vishay WSL). Thick-film chips are worse
and pulse-sensitive. At a few milliohms the solder joints add enough resistance that ADI measured a 22 % error on a 0.5
mΩ part without Kelvin pads, under 1 % with a four-pad layout.

**Example.** A 5 mΩ 1206 thick-film chip as a motor-current shunt: the reading drifts with the motor's heating and the
shunt shifts after a stall.

**How to check.** For resistors with role `Shunt`, report thick-film technology, and values below a threshold (default
10 mΩ) in a two-terminal part; include the whole-part TCR in the measurement error with `RPT-11`.

**Fix.** A metal-element shunt (Vishay WSL, Rohm PMR, wide-terminal types), a four-terminal part below 10 mΩ, Kelvin
pads.

**References.** V-WSL; ADI-SHUNT; R-PMR; R-LTR.

## `PAS-26` Wirewound resistor where its inductance matters <a id="pas-26"></a>

⚠️ warning · needs: the resistor's role (fast or high-frequency path), its technology and winding · [catalogue
entry](../checks.md#pas-26)

**Impl.** mid · depends on NET-7, NET-23, NET-24

**What it catches.** Wirewound resistors (not of a non-inductive, Ayrton-Perry winding) as gate resistors, in snubbers,
as fast current-sense or pulse resistors, or in RF and high-speed paths.

**Why it matters.** A wirewound resistor is a coil: its inductance rings with the circuit's capacitance, slows edges and
overshoots a snubber it was meant to damp. Non-inductive windings cancel most of it (at the cost of half the maximum
resistance value, Vishay NS and GN types); film and metal-strip parts have nanohenries.

**Example.** A 10 Ω 2 W wirewound in an RC snubber across a MOSFET: the snubber rings instead of damping.

**How to check.** For resistors in roles `GateResistor`, `Snubber`, `Shunt` with a fast signal, or on nets of a
high-speed class, report wirewound technology without the non-inductive flag.

**Fix.** A non-inductive wirewound, a metal-film or thick-film pulse-rated part, or a metal-strip shunt.

**References.** V-RSNS; V-GGN; V-PULSE12.

## `PAS-27` Resistor array whose elements sit at very different potentials or mix power and precision <a id="pas-27"></a>

⚠️ warning · needs: the nets on each element of the array, their voltages and roles · [catalogue
entry](../checks.md#pas-27)

**Impl.** mid · depends on NET-5, NET-23, NET-24

**What it catches.** A resistor array or network whose elements connect to nets at voltages that differ by more than the
array's element-to-element rating, or that mixes elements dissipating real power with elements in a precision role.

**Why it matters.** Elements of an array share one substrate with sub-millimetre spacing: the insulation between them is
rated (Vishay ACAS: 100 V), and the thermal coupling is deliberately strong. A 48 V pull-up next to a 3.3 V signal can
exceed the rating; a hot element shifts its precision neighbour. The same coupling is what makes matched networks track
well in a ratio.

**Example.** A four-element array used for two 3.3 V pull-ups and a 60 V divider's top resistor.

**How to check.** For parts classed as arrays, take the nets of each element's two pins, report a voltage difference
between adjacent elements above the array's isolation rating (default 50 V as an estimate), and elements in roles with
dissipation above a fraction of the rating next to precision roles.

**Fix.** Separate resistors for the high-voltage or power element; keep arrays for like signals and for matched ratios.

**References.** V-ACAS; V-ARRAYAN.

## `PAS-28` NTC thermistor biased so its self-heating exceeds the measurement's accuracy <a id="pas-28"></a>

⚠️ warning · needs: the NTC's bias network, supply, dissipation constant and the accuracy wanted · [catalogue
entry](../checks.md#pas-28)

**Impl.** mid · depends on NET-5, NET-23, CFG-5, RPT-6

**What it catches.** An NTC thermistor in a measuring divider whose dissipation at the operating point, divided by its
dissipation constant, warms it by more than the temperature accuracy the design needs.

**Why it matters.** Every thermistor heats itself: the error is `P / δ`, with δ from about 1 to 2 mW/K for small chips
to 7 mW/K for leaded parts in still air (Vishay NTCLE100). A 10 kΩ NTC on 3.3 V with a 10 kΩ partner dissipates 0.27 mW
at 25 °C: a few tenths of a kelvin for a chip, worse with lower values or higher voltages, and worst in still air or on
an insulated board.

**Example.** A 1 kΩ NTC chip in a divider from 5 V for a battery temperature sense: over 1 K of self-heating, so the
charger reads the battery warmer than it is.

**How to check.** For NTCs in a divider (`RPT-6`), compute the power at the expected temperature range's ends and the
error `P / δ`; report above the budget (the project's or a default of 0.5 K).

**Fix.** A higher NTC and partner resistance, a lower bias voltage, or bias only while measuring.

**References.** V-NTC.

## `PAS-29` NTC inrush limiter without a bypass or on a supply that restarts quickly <a id="pas-29"></a>

💡 suggestion · needs: the NTC's position (series in a power input), whether it is bypassed · [catalogue
entry](../checks.md#pas-29)

**Impl.** mid · depends on NET-5, NET-23

**What it catches.** A power NTC in series with a supply input to limit inrush, with no relay or FET that bypasses it
once running, or on a supply that can be switched off and on within the NTC's cooling time.

**Why it matters.** An inrush NTC limits current only while it is cold. Running, it sits hot at a low resistance and
dissipates continuously; switched off and on quickly it is still hot and does not limit the second inrush. Its energy
rating is finite (Vishay AS352R035: 700 J recommended, 1400 J to failure).

**Example.** A 2 Ω NTC in a 48 V input of a motor controller that is power-cycled by a contactor: the second inrush
after a short off-time blows the fuse.

**How to check.** For NTCs in the role `InrushLimiter`, report when no switch element (relay contact, FET) is in
parallel; the energy check against its rating is `RATE-27`.

**Fix.** A bypass relay or FET that shorts the NTC after start-up, or an active inrush limiter.

**References.** V-ICL.

## `PAS-30` Ferrite-core inductor whose saturation current is below the converter's current limit <a id="pas-30"></a>

⚠️ warning · needs: the converter's switch current limit (part data), the inductor's core material and Isat with its
definition · [catalogue entry](../checks.md#pas-30)

**Impl.** mid · depends on NET-6, NET-7, NET-24, CFG-5

**What it catches.** A converter's power inductor with a ferrite core whose saturation current is below the converter's
peak switch current limit. `RATE-15` covers the peak in normal operation; this check covers overload, short circuit and
start-up, where the current goes to the limit.

**Why it matters.** Ferrite saturates hard: past the knee the inductance collapses within a small current step and the
current ramps many times faster, overshooting the limit before the controller's blanking time ends. Ferrite's saturation
flux also falls with temperature (3C90: about 470 mT at 25 °C, 380 mT at 100 °C), so hot it saturates sooner. Powder and
metal-composite cores roll off softly and ride through. Vendors define Isat differently (an inductance drop of 10, 20 or
30 %): one Würth part lists 5.3 A at 10 % and 11.5 A at 30 %; the check normalises before comparing.

**Example.** A 3 A buck with a 4 A switch current limit and a 3.5 A Isat ferrite drum inductor: a short on the output
saturates it and the switch fails.

**How to check.** From the regulator's part data (current limit, or the controller's sense resistor and threshold) and
the inductor's Isat (with its drop definition and temperature) and core material: report ferrite cores with Isat below
the current limit; a soft core gets a note at most.

**Fix.** An inductor with Isat above the current limit at temperature, or a powder or metal-composite part.

**References.** CC469; CC1140; WE-MAPI; VI34244; WE128; FX3C90.

## `PAS-31` Inductor core loss at the converter's switching frequency and ripple <a id="pas-31"></a>

⚠️ warning · needs: the converter's switching frequency and ripple (RPT-3), the inductor's core material and loss data ·
[catalogue entry](../checks.md#pas-31)

**Impl.** hard · depends on NET-6, NET-24, CFG-5, RPT-3

**What it catches.** A power inductor chosen by inductance, Isat and DCR whose core and AC losses at the actual
switching frequency and ripple are large: powder-iron and some composite cores at MHz switching, any core with a large
ripple ratio.

**Why it matters.** Datasheet DCR gives the DC loss only; ESR curves are measured at low current and miss core loss.
Core loss grows with frequency and with the flux swing (ripple) to a power of 2 or more. Coilcraft's 4 MHz example: an
inductor with lower DCR loses over 1400 mW against under 400 mW for a part made for the frequency. Vishay's IHLP
guideline keeps core loss under a third of the loss budget for 40 K rise.

**Example.** A 2.2 MHz buck with 40 % ripple on a powder-iron inductor sized for 300 kHz parts: hot, and inefficient at
light load.

**How to check.** With `RPT-3`'s ripple and frequency and the inductor's core loss model (vendor Steinmetz coefficients,
or the series' frequency range as a coarse rule), estimate core plus AC loss and report when it exceeds the DC loss or
the part's frequency range.

**Fix.** An inductor rated for the frequency (low-loss composite or ferrite), more inductance for less ripple, or a
lower switching frequency.

**References.** CCeBook; VI34252; CC469; MMcat.

## `PAS-32` Unshielded or semi-shielded power inductor near circuits sensitive to magnetic fields <a id="pas-32"></a>

⚠️ warning · needs: the inductor's shielding, the board's sensitive circuits and their distance (layout) · [catalogue
entry](../checks.md#pas-32)

**Impl.** hard · depends on NET-23, NET-24, TOOL-3

**What it catches.** Unshielded (drum, bobbin) or semi-shielded power inductors on boards with circuits a stray magnetic
field disturbs: Hall sensors and current sensors, audio and microphone inputs, crystals, reed switches, current-sense
loops, other inductors.

**Why it matters.** An open core's field returns through the air around it, couples into nearby loops and radiates.
Würth: unshielded inductors are generally the worst offenders for EMI; semi-shielded parts suit boards where nothing
nearby is very sensitive. A switching inductor's field also induces audible noise in audio paths and offsets in Hall
sensors.

**Example.** A drum-core buck inductor 5 mm from a Hall current sensor: an offset that changes with the converter's
load.

**How to check.** On a board with the layout, report unshielded or semi-shielded power inductors within a distance
(default 10 mm) of parts in sensitive roles; without the layout, list them as candidates when the board has such parts.

**Fix.** A shielded or moulded inductor, more distance, or orientation that keeps the winding start on the switching
node.

**References.** WE047; WE118.

## `PAS-33` Inductor self-resonant frequency at or below the frequencies it has to block <a id="pas-33"></a>

⚠️ warning · needs: the inductor's role (choke, bias tee, EMI filter) and the frequencies involved, its SRF · [catalogue
entry](../checks.md#pas-33)

**Impl.** mid · depends on NET-23, NET-24, CFG-2

**What it catches.** An inductor used to block a frequency (an RF choke, a bias tee, a filter against a converter's
harmonics) whose self-resonant frequency is at or below that frequency.

**Why it matters.** Above its SRF an inductor's winding capacitance dominates: it becomes a capacitor and passes what it
should block. Large values in small cases have low SRFs; RF chip inductors specify SRF as a minimum about 15 to 20 %
under the measured average, power inductors as a typical value, and the board's capacitance lowers it further.

**Example.** A 10 µH 0603 multilayer chip as a bias-tee choke at 900 MHz: its SRF is tens of MHz, so at 900 MHz it is a
capacitor.

**How to check.** For inductors in roles `Choke`, `BiasTee` and `EmiFilter`, take the frequency to block (the net class,
the RF band in the project, the converter's switching frequency and harmonics) and report when it is above 0.7 × SRF
(configurable).

**Fix.** A smaller inductance with a higher SRF, an RF-rated part, or two inductors in series for broadband blocking.

**References.** CC363; CC469.

## `PAS-34` Ferrite bead where an inductance is needed <a id="pas-34"></a>

⚠️ warning · needs: the bead's role (converter power stage, computed LC filter, low-frequency filter) · [catalogue
entry](../checks.md#pas-34)

**Impl.** mid · depends on NET-6, NET-23, NET-24

**What it catches.** A ferrite bead in a converter's power stage in place of an inductor, in an LC filter whose design
used an inductance value, or as a filter for frequencies far below its resistive band (audio, low kHz).

**Why it matters.** A bead is built to be a resistor in its band: it dissipates noise instead of storing energy. Below
the band it is a small inductance whose value is not specified and collapses with DC bias (by up to 90 % at half its
rated current, ADI). A computed LC filter therefore has the wrong corner, and a bead in a power stage saturates.

**Example.** A "600 Ω" bead used as the inductor of a 1 MHz boost converter.

**How to check.** Report ferrite beads in roles `PowerInductor` and `LcFilter` (a filter whose design value is stated as
an inductance in the project or a `required` entry), and beads filtering a net whose noise of interest is below the
bead's specified band.

**Fix.** An inductor with a specified inductance and saturation current, or a bead chosen for the frequency band with
its DC-bias curve.

**References.** TIAAI2; MUbead; ADI-AN1368.

## `PAS-35` Ferrite bead feeding a low-ESR ceramic-only load: undamped resonance <a id="pas-35"></a>

⚠️ warning · needs: the bead's impedance curve, the capacitance and ESR on its load side · [catalogue
entry](../checks.md#pas-35)

**Impl.** mid · depends on NET-5, NET-23, NET-24, RPT-9

**What it catches.** A ferrite bead between a supply and a load whose only decoupling is low-ESR ceramics, without a
damping element (a series resistor, a lossy capacitor, a resistor across the bead).

**Why it matters.** Below its resistive band the bead is an inductance; with the MLCCs it forms an LC with almost no
damping. At the resonance the filter amplifies instead of attenuating: ADI's measurements show peaks of about 10 to 15
dB, and a peak at a nearby converter's switching frequency makes the "filtered" rail noisier than the unfiltered one. DC
bias lowers the bead's inductance and moves the peak up. `DEC-4` is the general caution, `DEC-9` the DC current.

**Example.** A 120 Ω bead feeding an ADC's AVDD with 10 µF and 100 nF MLCCs, from a 2 MHz buck: the resonance lands near
the buck's frequency.

**How to check.** For beads in role `RailFilter`, compute the resonance from the bead's low-frequency inductance (at the
rail's DC current) and the load-side capacitance, and report when no damping element exists and the resonance falls in a
range with a known noise source (a converter's frequency) or a peaking above a threshold.

**Fix.** A series resistor of about the characteristic impedance with the capacitor, a resistor across the bead, an
electrolytic or polymer capacitor with ESR, or a different bead.

**References.** ADI-AN1368; TIAAI2.

## `PAS-36` Common-mode choke wrong for the interface it sits on <a id="pas-36"></a>

⚠️ warning · needs: the interface (bus recogniser), the choke's differential and common-mode impedance, cut-off and
current rating · [catalogue entry](../checks.md#pas-36)

**Impl.** mid · depends on NET-4, NET-23, NET-24

**What it catches.** A common-mode choke on a differential interface whose differential-mode cut-off is below the
signal's bandwidth (a CAN or power-line choke on USB 2 high speed), whose leakage inductance distorts the signal, or
whose rated current is below the line's (a signal choke on a power pair). USB 3 and PCIe chokes in general are `HS-9`'s.

**Why it matters.** A good signal choke passes the differential signal untouched (flux cancels) and only impedes what is
common to both lines. Chokes made for CAN have tens of microhenries common-mode and a leakage (stray) inductance around
0.1 µH (TDK ACT45B) that is fine at 1 Mbit/s and destroys a 480 Mbit/s eye. Chokes for high-speed data list a
differential cut-off of GHz.

**Example.** A 51 µH CAN choke reused on a USB 2 port to save a part number: enumeration fails at high speed, full speed
works.

**How to check.** For chokes on a recognised bus, compare the choke's differential cut-off (or leakage inductance) with
the interface's bandwidth from a table (USB 2 HS, CAN, CAN FD, 100BASE-TX, RS-485) and its current rating with the
line's current.

**Fix.** A choke specified for the interface.

**References.** TDK45B; CC1009; CCcmf.

## `PAS-37` Coupled inductor and topology do not match <a id="pas-37"></a>

⚠️ warning · needs: the converter topology (part data), whether the magnetics are one coupled part or two inductors, its
coupling and ratio · [catalogue entry](../checks.md#pas-37)

**Impl.** mid · depends on NET-6, NET-7, NET-24, CFG-5

**What it catches.** A SEPIC, Cuk, Zeta or flyback whose datasheet design (or the inductance computed for it) assumes a
coupled inductor, fitted with two separate inductors, or the other way round; a coupled inductor whose ratio or coupling
is not what the topology needs (1:1 for SEPIC and Zeta, 1:N for a flyback, loose coupling for some SEPIC designs).

**Why it matters.** With close coupling the ripple current is shared and the required inductance per winding halves
(Coilcraft: two 22 µH singles become one 11 µH coupled part); fitted with two separate parts of the coupled value, the
ripple doubles. A coupled part's saturation rating applies to the sum of both windings' currents and its RMS rating
differs for one winding or both.

**Example.** A SEPIC designed with a 10 µH 1:1 coupled inductor rebuilt with two 10 µH single inductors: peak currents
beyond the switch limit.

**How to check.** From the regulator's topology and its `required` magnetics, compare with the inductors found on its
switching nodes (one part with four pins and a coupling, or two parts), and the coupled part's ratio and coupling.

**Fix.** The magnetics the design assumes, or the inductance recomputed for what is fitted.

**References.** CC639; CCcpl.

## `PAS-38` Inductor DCR current sensing without temperature compensation <a id="pas-38"></a>

⚠️ warning · needs: the controller's sensing method (DCR sensing from its part data and topology), whether an NTC
network is present · [catalogue entry](../checks.md#pas-38)

**Impl.** mid · depends on NET-6, NET-7, NET-23, CFG-5

**What it catches.** A converter controller that senses current across the inductor's DC resistance (an RC network
across the inductor into CS+ and CS−) without a temperature-compensating NTC network.

**Why it matters.** Copper's resistance rises about 0.4 %/K (Coilcraft: a 9 mΩ part is 11 mΩ at 85 °C, about 25 % more).
The sensed current and the current limit move by the same factor, and an inductor runs 40 to 80 K above ambient at full
load: the limit trips early hot, or the over-current protection is set loose to avoid it. DCR tolerance (often ±10 %)
adds to it.

**Example.** A multiphase buck with DCR sensing and no NTC: current balance and the over-current threshold drift with
load.

**How to check.** For regulators whose part data names DCR sensing (or an RC across the inductor into current-sense
pins), report when no NTC is in the sense network.

**Fix.** The NTC network from the controller's datasheet, or a sense resistor.

**References.** CC469; VI34252.

## `PAS-39` Iron-powder inductor run hot: thermal ageing <a id="pas-39"></a>

⚠️ warning · needs: the inductor's core material, its operating temperature (ambient plus losses) and the product life ·
[catalogue entry](../checks.md#pas-39)

**Impl.** hard · depends on NET-24, CFG-2, RPT-3

**What it catches.** Inductors with iron-powder cores bound by an organic binder (and moulded composite parts of that
construction) whose core runs hot (ambient plus core and copper losses) for long, especially with high core loss.

**Why it matters.** The organic binder ages at temperature: eddy-current loss rises, permanently, after hundreds or
thousands of hours, and the rising loss heats the core further, up to thermal runaway. Micrometals asks for it to be
considered at or above 75 °C; Coilcraft reports competitors' iron-powder parts with five times the core loss after 1500
h at 155 °C; Würth notes the effect passes AEC-Q200 tests unnoticed because inductance stays within ±20 % while Q drops.
Ferrite and some composite materials do not age this way.

**Example.** An iron-powder inductor in a 105 °C ambient converter with core loss comparable to copper loss: after two
years in the field its losses have tripled.

**How to check.** For power inductors whose series or core material is iron powder with organic binder, estimate the
core temperature (`RPT-3` losses, the board's `ambient` and `life_years` intents; not run without them) and report above
a threshold (default 100 °C) over the product's life; a series datasheet that states no thermal ageing clears it.

**Fix.** A core material without thermal ageing, a larger part with lower loss, or a lower core temperature.

**References.** MMcat; CC1192; WE128.

## `PAS-40` One precision circuit mixing precision and general-grade passives <a id="pas-40"></a>

⚠️ warning · needs: the circuit around a part with an accuracy spec, each passive's grade (tolerance, TCR, technology,
dielectric) · [catalogue entry](../checks.md#pas-40)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-5, RPT-26

**What it catches.** The passives of one circuit that serves a part needing precision (a reference divider, a
regulator's feedback divider or current-set pair, an amplifier's gain or feedback network, a matched pair, an active
filter) drawn from different grades: a 0.1 % 25 ppm/K thin-film resistor beside a 1 % 100 ppm/K thick-film one in the
same ratio, a C0G and an X7R in one filter, a low-leakage SET capacitor with a general-grade RSET.

**Why it matters.** The circuit is only as accurate as its weakest part: the precision parts were paid for and their
accuracy is lost to the one general-grade part, which usually is an oversight (a part kept from an earlier revision, a
library default, a value the precision series did not have). In a ratio, mixed technologies also bring mismatched TCRs
that no longer cancel (`PAS-22`).

**Example.** An ADC reference divider of a 0.1 % thin-film top resistor and a 1 % thick-film bottom one: the ratio is
good to about 1 %, ten times worse than the top part suggests.

**How to check.** For each circuit whose role carries an accuracy (`NET-23` roles around a part whose spec sets a
budget, as the introduction says), compare the grades of the passives that enter that accuracy; report the general-grade
ones when the circuit's budget needs the better grade (by `RPT-26`: the weak part alone exceeds `passive_share` of the
part's error), naming the precision siblings. Where no part in the circuit needs precision, the mix is `PAS-41`'s; where
the budget is unknown, the finding is an estimate. Tolerance alone on a single part stays `BOM-16`; one part's drift
against its budget `PAS-22`.

**Fix.** Bring the weak part up to the grade of its siblings (same series for both parts of a ratio), or, when the
circuit does not need the precision, take all of them down (`PAS-41`).

**References.** LT3045 p. 15 (RSET "should be a precision resistor"); V-BASICS; V-ARRAYAN (ratio accuracy of networks);
catalogue `PAS-22`, `BOM-16`, `BOM-19`.

## `PAS-41` Precision passives where the circuit does not need them <a id="pas-41"></a>

💡 suggestion · needs: the circuit's accuracy budget from its parts, each passive's grade · [catalogue
entry](../checks.md#pas-41)

**Impl.** mid · depends on NET-7, NET-23, NET-24, CFG-5, RPT-26

**What it catches.** Passives of a precision grade (0.1 % or tighter, low-TCR thin film or foil, matched networks, C0G
or film where a class 2 dielectric meets every constraint) in a circuit whose parts do not need it: a pull-up, an LED
resistor, a divider into a comparator with a 5 % threshold, a 0.1 % feedback divider on a regulator with a 3 %
reference.

**Why it matters.** Precision parts cost several times more, have fewer second sources and fewer values; where the
budget derived from the parts is far looser than the passive, the grade is lost cost. Often it is a leftover of a copied
precision circuit or a library default.

**Example.** A buck converter with a ±2 % reference and its feedback divider in 0.1 % 10 ppm/K thin film: 1 % thick film
keeps the divider's error within a quarter of the reference's.

**How to check.** For each circuit with a derived budget (the introduction: `passive_share` of the error of the parts it
serves), compute what a general-grade part would add with `RPT-26`; report a precision part when the general grade would
still fit the budget with margin, and no datasheet constraint or project figure asks for the grade. Where some passives
of the circuit are precision and others not, report under this check only when the budget is loose; otherwise it is
`PAS-40`. A circuit without part data is not reported (an estimate is not enough to call something waste).

**Fix.** Use the general grade, or write down why the precision is needed (a constraint in the part database, or a
project figure).

**References.** V-BASICS (grades and their cost); catalogue `PAS-40`, `BOM-19`.
