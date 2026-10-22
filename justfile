# Bare `just` only lists the recipes
default:
    @just --list

# erc_kicad's differential test exports the test schematics with kicad-cli first (skipped without KiCad).

# Run every test in the workspace
test:
    cargo test --workspace

# Clippy with warnings as errors, and the format check
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check

# Format all code
fmt:
    cargo fmt --all

# Start the better_erc app from this checkout
run:
    cargo run --release -p better_erc_app

# The test schematics have findings on purpose: only a finding that is not in the committed baseline fails.

# Run berc check on the test schematics, against test_schematics/berc-baseline.json
check:
    cargo run -q -p berc -- check test_schematics/sources --baseline test_schematics/berc-baseline.json

# The leading `-`: berc exits 1 on the findings it writes into the baseline.

# Write the baseline `just check` compares with, after a change in what the checks find
check-baseline:
    -cargo run -q -p berc -- check test_schematics/sources --format json > test_schematics/berc-baseline.json

# Only the changed schematics are exported again; the netlists are cached by content hash.

# Export the test schematics to netlists and SVG pictures with kicad-cli
netlists:
    cargo run -p generate_netlists

# Install the better_erc app and the berc command line tool to ~/.cargo/bin
install:
    cargo install --locked --path better_erc_app
    cargo install --locked --path berc

# Refuses on local changes (names them, never discards them). No systemd units. better_erc has no --version,
# so only berc's version is printed.

# Bring this PC up to main: pull, install, check both binaries are on PATH
deploy:
    #!/usr/bin/env bash
    set -eu
    dirty=$(git status --porcelain)
    if [ -n "$dirty" ]; then echo "local changes, not deploying:"; echo "$dirty"; exit 1; fi
    git pull --ff-only
    just install
    command -v better_erc
    command -v berc
    berc --version
    echo "ok: better_erc and berc installed from $(git rev-parse --short HEAD)"

kicad := "podman run --rm --userns=keep-id -v $PWD:/w:Z -w /w docker.io/kicad/kicad"

# So each KiCad version's netlist format stays covered by a test (erc_kicad/tests/fixtures/netlist/kicad_versions).

# Export the netlist fixtures again with kicad-cli 8, 9 and 10 from KiCad's container images (podman)
[working-directory: 'erc_kicad/tests/fixtures/netlist/kicad_versions']
netlist-fixtures:
    {{ kicad }}:8.0 kicad-cli sch export netlist -o fixture.kicad8.net fixture.kicad_sch
    {{ kicad }}:9.0 kicad-cli sch export netlist -o fixture.kicad9.net fixture.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o fixture.kicad10.net fixture.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o fixture_kicad10.kicad10.net fixture_kicad10.kicad_sch

# The fixtures of erc_kicad's differential test (erc_kicad/tests/differential.rs); local_power uses a KiCad 10 feature.

# Export erc_kicad's fixture netlists again with kicad-cli 8, 9 and 10 from KiCad's container images (podman)
[working-directory: 'erc_kicad/tests/fixtures']
kicad-fixtures:
    {{ kicad }}:8.0 kicad-cli sch export netlist -o connectivity.kicad8.net connectivity.kicad_sch
    {{ kicad }}:9.0 kicad-cli sch export netlist -o connectivity.kicad9.net connectivity.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o connectivity.kicad10.net connectivity.kicad_sch
    {{ kicad }}:8.0 kicad-cli sch export netlist -o hierarchy.kicad8.net hierarchy.kicad_sch
    {{ kicad }}:9.0 kicad-cli sch export netlist -o hierarchy.kicad9.net hierarchy.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o hierarchy.kicad10.net hierarchy.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o local_power.kicad10.net local_power.kicad_sch
    {{ kicad }}:8.0 kicad-cli sch export netlist -o alias.kicad8.net alias.kicad_sch
    {{ kicad }}:9.0 kicad-cli sch export netlist -o alias.kicad9.net alias.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o alias.kicad10.net alias.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o alias_pro.kicad10.net alias_pro.kicad_sch
