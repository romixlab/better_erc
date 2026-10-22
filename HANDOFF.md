# Handoff: remove-old (P2503#22oct-1127b)

Done (commits on s/22oct2026-remove-old): erc_analysis::power + graph::input_pins (ported from erc_core), KiCad 10 unit names in the netlist importer, netlist fixtures moved to erc_kicad/tests/fixtures/netlist, old readers' asserts as acceptance tests (erc_kicad netlist, erc_model value), app renamed better_erc_app (bin better_erc), facade crate better_erc (re-exports + diff::pnp), berc diff --pnp (origin page|aux, y up, exclude_from_pos_files), Project::open_files, the app on the new core.

Next: delete ecad_file_format, ecad_compare, erc_core, lexpr; Cargo.lock; README, FEATURES, docs/model.md, docs/engine.md, CLAUDE.md; gate (just test, just lint, just install, berc --version, berc check, app start); tpm work ready.

Decisions: Workspace/Session not built here (Project is the entry; Session with watching comes with the MCP server / GUI tasks). Power analysis keeps the old rules (fallback layer of NET-5). PnP compare: a board vs a PnP file goes into the PnP frame; two boards or two PnP files compare as they are.

Gotchas: NO_COLOR=1 in this shell.
