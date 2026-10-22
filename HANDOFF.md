# Handoff: session core-model (P2503#22oct-0042b, #22oct-0042c)

Done:
- `erc_model`: arenas, `Fact<T>`/`Facts<T>` with provenance and rank, `Value`, import trait (`Files`, `Source`, `Importer`), canonical dump with reader (`dump::write` / `dump::read`), `netcmp` (nets as pin sets, plus class differences).
- `erc_kicad`: `sexpr` (moved from berc, berc re-exports it), `netlist` (.net importer, golden dumps), `sch` (hierarchy, KiCad connectivity, bus aliases from sheets and project, `~` literal since format 20250318), `pro` (.kicad_pro), `netclass` (KiCad's effective net class per net), `project::KicadImporter` (pro + sch, later pcb).
- Differential test `erc_kicad/tests/differential.rs`: fixtures (connectivity, hierarchy, local_power, alias, alias_pro, ecad_file_format's fixture and fixture_kicad10) against committed KiCad 8/9/10 netlists; test schematics through kicad-cli; hw boards (`~/git/hw`) with the installed kicad-cli, `BERC_DIFF_KICAD=8,9,10` adds podman. All green: 41 hw boards on KiCad 10, 94 comparisons over 8/9/10. Golden dumps in `erc_kicad/tests/golden/sch`, `BLESS=1` rewrites.
- `just kicad-fixtures` re-exports the fixture netlists (podman).

Next:
- `.kicad_pcb` into `Layout` (model types exist in `erc_model::layout`, dump covers them): footprints, pads, nets, courtyards, silk, test points, zones, tracks, vias, stackup, outline; `KicadImporter` reads it unless `no_layout`; golden dump of `test_schematics.kicad_pcb` and a fixture board; hw boards: import + dump stability.
- docs/model.md, CHANGELOG, FEATURES (NET-17 model, IMP-13 KiCad import), tpm notes, `just test` + `just lint`, `git rm HANDOFF.md`, `tpm work ready`.

Decisions:
- Identity (designators, net names, sheet paths, pin numbers) is structure; attributes are facts. Layout geometry carries the board file as its source at the `Layout` level; footprint fields are facts.
- Power symbols are labels of kind `power`, not parts; `#` parts (PWR_FLAG) are parts, filtered as KiCad does in `netcmp::netlisted`.
- KiCad 8 does not join a junction to a wire passing through it, 9 and 10 do; KiCad 8 gives a label on a crossing of two wires to the first wire only, 9 and 10 to both: `ImportOptions::kicad_version` (default: the newest format among the sheets).
- KiCad 9.0.9 bugs are not copied (KiCad 8 and 10 agree): buses through the second instance of a repeated sheet, and a KiCad 8 sheet's bus aliases, are dropped by 9.0.9's kicad-cli; both are `KNOWN` differences in the test.
- Local power symbols (`(power local)`) are KiCad 10 only (9.0.9 refuses the file).
- Project ERC/DRC severities are stored as the file states them (all), not only those differing from KiCad's defaults.
- The effective net class is stored as KiCad's netlist writes it (`Usb,Wide,Default`), source the project file (or the root sheet without one).
- hw boards are private: tested locally, never committed as fixtures or dumps.

Gotchas:
- Label text in files is escaped (`{slash}`); bus parsing runs on the unescaped text (`sch::unescape`).
- Sheet files of one project can have different format versions.
- The fixture generator is a one-off Python script outside the repo (`$CLAUDE_JOB_DIR/tmp/gen.py`, a copy of `/tmp/core-model-fixture-gen.py` extended); edit the fixtures in KiCad from now on, or regenerate with it while it exists.
- The test's kicad-cli cache lives under `CARGO_TARGET_TMPDIR` (with this PC's cargo build-dir under `/mnt/data/omarchy/cargo-build/.../tmp/kicad_netlists`).
