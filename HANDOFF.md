# Handoff: session checks-bom (stage 1 easy checks: BOM, LIB, MUX, LOGIC)

Tasks: P2503#22oct-0051h (BOM), #22oct-0051l (LIB), #22oct-0048d (MUX), #22oct-0048 (LOGIC).

## Done

- BOM: BOM-1, 3, 4, 8, 11, 14, 17 implemented with schematics; BOM-5 takes `calc` as a placeholder (its page says so); BOM-5, 6, 9, 10 checked against their pages.
- LIB-14 (unplaced units, from the symbol's pins per unit; schematics only, the netlist importer keeps no unit per library pin) and LIB-15 (IC symbol without power pins by type or name); LIB-3 unchanged.
- `erc_analysis::package` (footprint case size, SMD/THT, side), `ESeries::steps`.
- berc and erc_checks project tests that count findings on i2c-1 run `--only I2C` (BOM-6 for the baseline step): new checks no longer move their numbers.

## Next

- Rebase onto checks-wire's e891308 (sample.dump fix) and, once checks-dec lands, `tpm work rebase`; then drop every `[[not_run]]` block BLESS wrote into the sidecars (user's rule: not_run blocks are hand-written subset assertions; BLESS writes findings only after checks-dec's test change).
- `just test-full`, `just lint`, `git rm HANDOFF.md`, `tpm work ready`.

## Decisions

- BOM-1: "one step" is adjacency in the smallest series that has both values (33/39 E12, 4k7/5k1 E24, 1u/2u2 E3), as the page's examples imply; no role filter until NET-7.
- BOM-4: `min_group` 2 (arrays come with 2); I2C pull-up pairs show as candidates.
- BOM-14: one board finding when no part has an MPN at all.
- BOM-8: pins named A/K; walk through resistors and ferrites, `max_hops` 2.
- MUX-1, LOGIC-8 build on checks-wire's `erc_analysis::names` (branch rebased onto their 06522b7, not cherry-picked); my additions live in `names/signals.rs` (one `mod`/`pub use` line in names.rs) to keep the merge surface small.
- LOGIC-8: word senses (Go/Stop/Event): PG into ~SHDN is right, PGOOD into ~EN wrong; the logic.md example said PG into ~SHDN was a bug and was corrected.
- LOGIC-7: bidirectional pins are not drivers (needs NET-7); a pull-up may run through an LED and resistor (`max_hops` 2).
- MUX-1: UART judged pin against pin (net names may be from either side); connectors and test points left out.
- Helper ref `wire-names-06522b7` (= checks-wire's 06522b7) left in this repo; the guard keeps it until landing.

## Gotchas

- Every check runs on every test schematic: re-BLESS sidecars (`BLESS=1 cargo test -p erc_checks --test schematics`) and `just check-baseline` after adding checks or rebasing onto sibling batches; erc_kicad golden dumps for new schematics with `BLESS=1 cargo test -p erc_kicad --test differential`.
- kicad-cli re-renders the i2c SVGs with a different element order: put the committed ones back (`git show HEAD:path > path`).
