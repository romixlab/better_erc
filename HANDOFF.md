# checks-io handoff

Done: IO (3561ba7, rebased on 0.4.5) and CLK (d71b483), each check with schematic, sidecar, FEATURES line; CHANGELOG line under [Unreleased] lists them. Next: ANA-3, -4, -10, -11, -12, -16, -21, -22 on `erc_analysis::analog` (amp_units, amp_supplies, sense_pin), then engine.md tables (checks that run, analyses `pull` and `analog`), `just test-full`, `just lint`, `git rm HANDOFF.md`, `tpm work ready`.

How the schematics are made: a throwaway generator in the session's job tmp dir (`sch.py`, `specs.py`, not committed): symbols with pins only, nets by labels and power symbols at pin ends; then `just netlists`, `BLESS=1 cargo test -p erc_kicad --test differential test_schematics` (golden dumps), `BLESS=1 cargo test -p erc_checks --test schematics`, `just check-baseline`. Only commit the pictures of new schematics (`just netlists` rewrites others).

Decisions: catalogue IO-6 ≈ IO-1, IO-10 ≈ IO-6; IO-15 = flags by name; other open-drain outputs to an MCU → IO-8; pin_name::active_low wraps names::active_low (checks-wire). Notes on the tpm tasks.
