# Handoff: checks-power (s/22oct2026-checks-power)

Batch "checks-power": all 23 checks are implemented and committed; the branch is rebased onto main 0.4.6 (f31969e). What is left is the landing gate and the ready note.

## Done

- RAIL-1/5/18/19, PIN-3/4/6, DRV-5/6/7/9/12/14/17, GND-3/7, SEQ-5, LEAK-1, LVL-4/7/9, I2C-4, I2C-18: a check file each under `erc_checks/src/<cat>/`, a schematic + sidecar each (`test_schematics/sources/<id>_<slug>`), golden dumps, pictures of the new schematics, FEATURES.md lines, docs/engine.md table rows, one CHANGELOG entry under [Unreleased] (Added + Fixed).
- I2C-1/2/3/7/9/10 verified against their pages (notes on P2503#22oct-0048j).
- erc_analysis: `feed.rs` (Feeds: sources per net, power links, reach/fed_by/region; `Ctx::feeds()`); part classes `Motor`, `Solenoid`; isolators split I2C buses; power grammar: KiCad auto net names read as nothing (`is_auto_name`), shield/chassis/earth pins make no ground (`ground_pin_word`), a lone ground pin on an unconnected net is neither ground nor rail (asked by the user via checks-bus).
- erc_checks: `FindingBuilder::severity` + `Ctx::severity_or` (PIN-3 fused outputs as suggestions).
- docs/checks.md: `≈ I2C-1` on the I2C-18 line (I2C-18 folds I2C-1 in on segments behind a translator).
- tpm: all eight tasks 🚧 with dated notes and gaps.

## Next

1. `just check-baseline` (rewrites test_schematics/berc-baseline.json; never by hand), then `just check` must pass; commit the baseline.
2. `just test-full` and `just lint` green.
3. Rewrap the CHANGELOG [Unreleased] Added paragraph (it was edited piecemeal; keep the repo's ~120-column wrap).
4. `git rm HANDOFF.md`, commit, `tpm work ready --note "..."`: 23 checks (list above), schematics `<id>_<slug>` for each, FEATURES ids (the check ids, NET-4, NET-5, NET-19 lines updated), gaps (see the task notes: RAIL-1 unplaced power units, RAIL-18 paralleling part data, PIN-4 USB exemption by CFG-2, DRV-6 thresholds, DRV-9 coil found by its switch, DRV-14 loads behind connectors, DRV-17 LEDs with >2 pins, GND-3 one-plane by project, LVL-7 no bank data, I2C-18 muxes).
5. When checks-bus lands (its `erc_analysis::connector::is_shell_pin`), GND-7's private `shell_pin_name` should switch to it (checks-bus asked; not on main yet at this rebase).

## Gotchas

- Test schematics were written by a throwaway generator in the job tmp dir (not in the repo); `BLESS=1 cargo test -p erc_kicad --test differential` writes golden dumps and regenerates SVG pictures of every schematic: commit only pictures of schematics this branch added, drop the rest (commit as wip, then `git reset --hard HEAD~1` with a clean tree; no restore/stash).
- New checks fire on old schematics: `BLESS=1 cargo test -p erc_checks --test schematics`, review the diff.
- The rebase conflicts were sidecars (re-blessed after), docs/engine.md table rows and erc_checks/src/lib.rs module lists (both unioned).
