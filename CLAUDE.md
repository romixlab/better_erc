# CLAUDE.md

Feature tracker: [FEATURES.md](FEATURES.md). Changes: [CHANGELOG.md](CHANGELOG.md), one entry per work commit,
with a workspace version bump (docs-only commits skip the bump). The check catalogue (IDs like `I2C-3`) lives in
this repo: [docs/checks.md](docs/checks.md), with a condensed [schematic review checklist](docs/review-checklist.md)
alongside it. New checks go there first, then into FEATURES.md when code exists.

## Commands

```sh
just test              # cargo test --workspace (erc_kicad's differential test exports the test schematics with kicad-cli first; it compares a sample of the hw boards)
just test-full         # the same with every hw board in the differential (30-50 s more): CI and before landing
just lint              # clippy -D warnings + fmt --check
just fmt               # cargo fmt --all
just run               # start the better_erc app (crate better_erc_app)
just check             # berc check on the test schematics against test_schematics/berc-baseline.json
just netlists          # export test_schematics/sources to netlists and SVGs (cached by content hash)
just netlist-fixtures  # re-export the per-KiCad-version netlist fixtures with kicad-cli 8, 9, 10 (podman)
just kicad-fixtures    # re-export erc_kicad's differential-test fixtures with kicad-cli 8, 9, 10 (podman)
just install           # cargo install the better_erc app and the berc CLI
just deploy            # this PC up to main: pull --ff-only (refuses on local changes), install, check it is on PATH
```
