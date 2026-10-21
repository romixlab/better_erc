# Bare `just` only lists the recipes
default:
    @just --list

test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check

# Installs the `better_erc` GUI binary and the `berc` command line tool to ~/.cargo/bin
install:
    cargo install --locked --path better_erc
    cargo install --locked --path berc

# Bring this PC up to main: pull, install, check both binaries are on PATH. Refuses on local changes (names
# them, never discards them). No systemd units. better_erc has no --version, so that check compares crate
# versions; berc does, and is checked against it directly.
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

# Re-export the KiCad netlist fixtures (ecad_file_format/test_input/kicad_versions) with kicad-cli 8, 9 and 10
# from KiCad's own container images, so each version's netlist format stays covered by a test
netlist-fixtures:
    cd ecad_file_format/test_input/kicad_versions && podman run --rm --userns=keep-id -v "$PWD":/w:Z -w /w docker.io/kicad/kicad:8.0 kicad-cli sch export netlist -o fixture.kicad8.net fixture.kicad_sch
    cd ecad_file_format/test_input/kicad_versions && podman run --rm --userns=keep-id -v "$PWD":/w:Z -w /w docker.io/kicad/kicad:9.0 kicad-cli sch export netlist -o fixture.kicad9.net fixture.kicad_sch
    cd ecad_file_format/test_input/kicad_versions && podman run --rm --userns=keep-id -v "$PWD":/w:Z -w /w docker.io/kicad/kicad:10.0 kicad-cli sch export netlist -o fixture.kicad10.net fixture.kicad_sch
    cd ecad_file_format/test_input/kicad_versions && podman run --rm --userns=keep-id -v "$PWD":/w:Z -w /w docker.io/kicad/kicad:10.0 kicad-cli sch export netlist -o fixture_kicad10.kicad10.net fixture_kicad10.kicad_sch
