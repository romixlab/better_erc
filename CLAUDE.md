# CLAUDE.md

Feature tracker: [FEATURES.md](FEATURES.md). Changes: [CHANGELOG.md](CHANGELOG.md), one entry per work commit,
with a workspace version bump (docs-only commits skip the bump). The check catalogue (IDs like `I2C-3`) lives in
this repo: [docs/checks.md](docs/checks.md), with a condensed [schematic review checklist](docs/review-checklist.md)
alongside it. New checks go there first, then into FEATURES.md when code exists.

## Commands

```sh
just test      # cargo test --workspace
just lint      # clippy -D warnings + fmt --check
just install   # cargo install the `better_erc` GUI binary (no separate CLI exists yet)
just deploy    # this PC up to main: pull --ff-only (refuses on local changes), install, check it is on PATH
```
