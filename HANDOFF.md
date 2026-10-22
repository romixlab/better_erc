# Handoff: checks-power (s/22oct2026-checks-power)

Batch "checks-power": RAIL-1/5/18/19, PIN-3/4/6, DRV-5/6/7/9/12/14/17, GND-3/7, SEQ-5, LEAK-1, LVL-4/7/9, I2C-4, I2C-18 (I2C-1/2/3/7/9/10 only verified).

## Done

- RAIL (f73520f), PIN (64e7714), DRV (b315610): check files, a schematic + sidecar each, FEATURES, engine.md table, CHANGELOG line under [Unreleased].
- `erc_analysis::feed` (Feeds: source pins per net, power links, reach/fed_by/region), `Ctx::feeds()`.
- `FindingBuilder::severity` + `Ctx::severity_or` (PIN-3 fused outputs one level lower).
- Part classes `Motor`, `Solenoid` (erc_model + erc_analysis class rules).

## Next

GND-3, GND-7, SEQ-5, LEAK-1; LVL-4, LVL-7, LVL-9; I2C-4 (overlaps I2C-3: add `≈ I2C-3` to its catalogue line), I2C-18 (overlaps I2C-1, `≈ I2C-1`; isolators to split I2C buses in erc_analysis::i2c). Then `just check-baseline` (berc baseline), `just test-full`, `just lint`, tpm notes, `git rm HANDOFF.md`, `tpm work ready`.

## Gotchas

- Test schematics are written by a throwaway generator (job tmp dir, not in the repo); `BLESS=1 cargo test -p erc_kicad --test differential` writes the golden dumps and the SVG pictures of new schematics; commit only the pictures of schematics this branch added.
- The differential test touches name-6 pictures: leave them unstaged.
- New checks fire on old schematics: `BLESS=1 cargo test -p erc_checks --test schematics`, review the diff.
