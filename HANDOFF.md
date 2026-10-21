# Handoff: erc-tidy

Done (each its own commit, CHANGELOG under [Unreleased]):
- IMP-12: KiCad 9/10 netlist elements in `kicad_netlist.rs` (taken from KiCad's `netlist_exporter_xml.cpp` 8.0.0/9.0.0/10.0.0); fixtures exported by kicad-cli 8.0.9/9.0.9/10.0.7 from KiCad's container images (`just netlist-fixtures`, podman).
- `~` as empty value (KiCad exporter writes it for an empty Value): BOM-9 now fires on KiCad designs.
- general/style tests on new test schematics `style_values`, `io_inputs` (KiCad 8 format, like the I2C ones), with assertions.
- generate_netlists drops the SVG export date; has a `main.rs` for `just netlists`.
- justfile: fmt, run, netlists, netlist-fixtures; CLAUDE.md commands list.
- berc TOOL-4: colour (anstream), completion in --help, tests/cli.rs. APP-3 filed (better_erc has no --version).

Next: README rewrite; other tidy-ups (erc_core depends on generate_netlists as a normal dependency and its build.rs runs kicad-cli on every build, also for the app; consider making it test-only); just test + just lint; git rm this file; tpm work ready.

Gotchas: this shell has NO_COLOR=1 set, so colour checks need `env -u NO_COLOR` under `script`.
