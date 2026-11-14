# Bare `just` only lists the recipes
default:
    @just --list

# erc_kicad's differential test exports the test schematics with kicad-cli first (skipped without KiCad).

# The tests. `just test`: all of them (test-all), the same run the landing gate does. `just test --changed`: only
# the workspace crates changed against main and the crates that depend on them, a quick local check before
# landing.
test *flags:
    if [ "{{flags}}" = "--changed" ]; then tpm test changed -- cargo nextest run --workspace; else {{just_executable()}} test-all {{flags}}; fi

# Run every test in the workspace; the hw differential compares a sample of the boards (BERC_HW_FULL: all)
test-all:
    cargo test --workspace

# The hw boards (~41) take 30-50 s through kicad-cli; the default `just test` compares a sample of them.

# Run every test with the full hw differential, for CI and before landing
test-full:
    BERC_HW_FULL=1 cargo test --workspace

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

# Only the changed schematics are exported again; netlists and pictures are cached by content hash. The tests export netlists only.

# Export the test schematics to netlists and SVG pictures with kicad-cli
netlists:
    cargo run -p generate_netlists

# Build the better_erc app and the berc command line tool and install them on this PC, the app in the launcher
install:
    cargo build --profile deploy --locked -p better_erc_app -p berc
    # With our release tool on PATH, it installs them (one copy on PATH, kept-releases stack); otherwise cargo install.
    if command -v tpm >/dev/null; then tpm install target/deploy/better_erc target/deploy/berc; else cargo install --locked --force --path better_erc_app && cargo install --locked --path berc; fi
    just launcher

# The app in the desktop's launcher: its icon and .desktop file (app id better_erc), then the desktop database
launcher:
    install -Dm644 better_erc_app/assets/icon-256.png ~/.local/share/icons/hicolor/256x256/apps/better_erc.png
    install -Dm644 better_erc_app/deploy/better_erc.desktop ~/.local/share/applications/better_erc.desktop
    -update-desktop-database ~/.local/share/applications
    -gtk-update-icon-cache ~/.local/share/icons/hicolor

# Refuses on local changes (names them, never discards them). No systemd units.

# Bring this PC up to main: pull --ff-only, install, check both --versions against main (works without the release tool)
deploy:
    #!/usr/bin/env bash
    set -eu
    dirty=$(git status --porcelain)
    if [ -n "$dirty" ]; then echo "local changes, not deploying:"; echo "$dirty"; exit 1; fi
    git pull --ff-only
    just install
    for bin in better_erc berc; do \
        $bin --version | grep -q "($(git rev-parse --short HEAD)" || { echo "$bin --version does not show $(git rev-parse --short HEAD):"; $bin --version; exit 1; }; \
        $bin --version; \
    done

kicad := "podman run --rm --userns=keep-id -v $PWD:/w:Z -w /w docker.io/kicad/kicad"

# So each KiCad version's netlist format stays covered by a test (erc_kicad/tests/fixtures/netlist/kicad_versions).

# Export the netlist fixtures again with kicad-cli 8, 9 and 10 from KiCad's container images (podman)
[working-directory: 'erc_kicad/tests/fixtures/netlist/kicad_versions']
netlist-fixtures:
    {{ kicad }}:8.0 kicad-cli sch export netlist -o fixture.kicad8.net fixture.kicad_sch
    {{ kicad }}:9.0 kicad-cli sch export netlist -o fixture.kicad9.net fixture.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o fixture.kicad10.net fixture.kicad_sch
    {{ kicad }}:10.0 kicad-cli sch export netlist -o fixture_kicad10.kicad10.net fixture_kicad10.kicad_sch

# The board copies of erc_kicad's footprint test (erc_kicad/tests/footprint.rs): Test.pretty placed by KiCad 10.

# Place the test footprints on a board again with KiCad 10's Python API (podman)
[working-directory: 'erc_kicad/tests/fixtures/footprints']
footprint-fixture:
    {{ kicad }}:10.0 python3 place.py > placed.kicad_pcb

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
