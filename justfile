test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check

# Installs the `better_erc` GUI binary to ~/.cargo/bin (there is no separate CLI yet)
install:
    cargo install --locked --path better_erc

# Bring this PC up to main: pull, install, check the binary is on PATH. Refuses on local changes (names them,
# never discards them). No systemd units. The app has no --version, so the check compares crate versions.
deploy:
    #!/usr/bin/env bash
    set -eu
    dirty=$(git status --porcelain)
    if [ -n "$dirty" ]; then echo "local changes, not deploying:"; echo "$dirty"; exit 1; fi
    git pull --ff-only
    just install
    command -v better_erc
    echo "ok: better_erc installed from $(git rev-parse --short HEAD)"
