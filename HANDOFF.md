# Handoff: session core-model (P2503#22oct-0042b, #22oct-0042c)

Done:
- `erc_model`: arenas, `Fact<T>`/`Facts<T>` with provenance and rank, `Value`, import trait (`Files`, `Source`, `Importer`), canonical dump with reader (`dump::write` / `dump::read`), `netcmp` (nets as pin sets, for the differential test).
- `erc_kicad`: `sexpr` (moved from berc, berc re-exports it), `netlist` (.net importer, golden dumps for the 8/9/10 fixtures), `sch` (schematic hierarchy, KiCad connectivity; `ImportOptions::kicad_version` picks the KiCad 8/9/10 rules, default = newest file format of the project).
- Manual differential run: all 41 hw projects equal KiCad 10's netlists (names included), 18 equal KiCad 8's, 35 equal KiCad 9's; the 8 test schematics equal KiCad 10's.

- `erc_kicad/tests/fixtures/connectivity.kicad_sch` (KiCad 8 format, 17 geometry and naming cases) with its netlists from KiCad 8, 9, 10 (podman for 8 and 9): equal for all three (`cargo run -p erc_kicad --example differential -- erc_kicad/tests/fixtures/connectivity.kicad_sch erc_kicad/tests/fixtures/connectivity.kicad8.net 8`).

Next:
- Case 15 of the connectivity fixture (bus with entries, labels D0/D1) does not reach R25/R26 in KiCad's netlist either: check the generated positions (generator: `/tmp/core-model-fixture-gen.py`, a one-off Python script, not committed; edit the fixture in KiCad from now on, or fix the generator and rerun `python3 /tmp/core-model-fixture-gen.py erc_kicad/tests/fixtures`).
- A hierarchy fixture: root + a child sheet used twice (two instances, different references), hier labels and sheet pins, a vector bus through a sheet pin whose parent bus label has another name (`D[0..3]` vs `DATA[0..3]`), a group bus (`CTL{EN RST}`), global labels, power symbols. Plus a KiCad 9 file with a local power symbol, and the KiCad 10 fixture of ecad_file_format (variants, jumpers).
- A `just kicad-fixtures` recipe (podman 8/9 + kicad-cli 10) for erc_kicad's fixtures, like `netlist-fixtures`; committed netlists for the test schematics too.
- The differential test as a cargo test: test schematics + fixtures (committed netlists), hw boards when `~/git/hw` exists (kicad-cli export, cached by content hash under the target dir).
- `.kicad_pro` (net classes, text vars, variants, ERC/DRC severities and exclusions, rules) and `.kicad_pcb` (layout) into the model.
- docs/model.md, CHANGELOG, FEATURES (NET-17 model, IMP-13 KiCad import), tpm notes.

Decisions:
- Identity (designators, net names, sheet paths, pin numbers) is structure; attributes are facts. Layout geometry carries the board file as its source at the `Layout` level; footprint fields are facts.
- Power symbols are labels of kind `power`, not parts; `#` parts (PWR_FLAG) are parts, filtered as KiCad does in `netcmp::netlisted`.
- KiCad 8 does not join a junction to a wire passing through it, 9 and 10 do; KiCad 8 gives a label on a crossing of two wires to the first wire only, 9 and 10 to both: semantics follow `ImportOptions::kicad_version` (default: the newest format among the project's sheet files).
- hw boards are private: they are tested locally, never committed as fixtures or dumps.

Gotchas:
- Label text in files is escaped (`{slash}`); bus parsing runs on the unescaped text (`sch::unescape`).
- Sheet files of one project can have different format versions (KiCad saves only changed sheets).
