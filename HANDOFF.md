# Handoff: session checks-bom (stage 1 easy checks: BOM, LIB, MUX, LOGIC)

Tasks: P2503#22oct-0051h (BOM), #22oct-0051l (LIB), #22oct-0048d (MUX), #22oct-0048 (LOGIC).

## Done

- BOM: BOM-1, 3, 4, 8, 11, 14, 17 implemented with schematics; BOM-5 takes `calc` as a placeholder (its page says so); BOM-5, 6, 9, 10 checked against their pages.
- LIB-14 (unplaced units, from the symbol's pins per unit; schematics only, the netlist importer keeps no unit per library pin) and LIB-15 (IC symbol without power pins by type or name); LIB-3 unchanged.
- `erc_analysis::package` (footprint case size, SMD/THT, side), `ESeries::steps`.
- berc and erc_checks project tests that count findings on i2c-1 run `--only I2C` (BOM-6 for the baseline step): new checks no longer move their numbers.

## Next

- MUX-1 and LOGIC-8 build on checks-wire's `erc_analysis::names` (commit 06522b7 on s/22oct2026-checks-wire, gpd-omarchy): cherry-pick it, extend additively with polarity sense and interface signals. My draft of that grammar is in the job's tmp (`names_mine.rs`); merge its ideas into theirs.
- LOGIC-3, LOGIC-7 on pin types.

## Decisions

- BOM-1: "one step" is adjacency in the smallest series that has both values (33/39 E12, 4k7/5k1 E24, 1u/2u2 E3), as the page's examples imply; no role filter until NET-7.
- BOM-4: `min_group` 2 (arrays come with 2); I2C pull-up pairs show as candidates.
- BOM-14: one board finding when no part has an MPN at all.
- BOM-8: pins named A/K; walk through resistors and ferrites, `max_hops` 2.

## Gotchas

- Every check runs on every test schematic: re-BLESS sidecars (`BLESS=1 cargo test -p erc_checks --test schematics`) and `just check-baseline` after adding checks or rebasing onto sibling batches; erc_kicad golden dumps for new schematics with `BLESS=1 cargo test -p erc_kicad --test differential`.
- kicad-cli re-renders the i2c SVGs with a different element order: put the committed ones back (`git show HEAD:path > path`).
