# Handoff: checks-wire (SHORT, WIRE, NAME easy checks)

Done (branch s/22oct2026-checks-wire):

- `erc_analysis::names` (name grammar: active-low markers and styles, differential halves and pairs, numbered families, GPIO numbers of pin names, UART pins and net words, normalised names); `graph::series_paths`; `erc_model::Sheet::rev` from the title block (both KiCad importers, the dump).
- 24 checks: SHORT-1..7, 11, 12; WIRE-1, 2, 11, 13, 14; NAME-1, 2, 4, 5, 6, 7, 9, 10, 11, 12. One test schematic each under `test_schematics/sources` (NAME-6 has two child sheets, `name-6_child_mcu`, `name-6_child_power`), sidecars, golden dumps, SVGs. All schematics match kicad-cli 10 in the differential test.
- The schematics were written by a throwaway generator (`$CLAUDE_JOB_DIR/tmp/schgen`, not in the repo); edit them in KiCad or by hand from now on.
- berc/tests/check.rs, berc/tests/fixtures/check_i2c-1.txt, erc_checks/tests/project.rs are byte-identical copies of checks-bom's (`--only I2C`, planned count as N), so landing merges them cleanly.

Next:

- When checks-dec has landed: `tpm work rebase`, drop the `[[not_run]]` blocks BLESS wrote into the sidecars (user's rule: sidecars hold findings; not_run only where a catalogue page asks), re-run `just test`, `just lint`.
- Then `git rm HANDOFF.md`, `tpm work ready`.

Gaps (noted on the tpm tasks): SHORT-4 package power rating (CFG-5); SHORT-6 input pins of configurable parts (NET-7); NAME-4 pins named by port only (NET-12); NAME-5 project switch for later revisions (CFG-2); NAME-11 rails fed by a negative regulator (NET-5 sources).
