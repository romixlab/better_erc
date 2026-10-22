# Handoff: session core-model (P2503#22oct-0042b, #22oct-0042c)

Done:
- `erc_model`: arenas, `Fact<T>`/`Facts<T>` with provenance and rank, `Value`, import trait (`Files`, `Source`, `Importer`), canonical dump with reader (`dump::write` / `dump::read`), `netcmp` (nets as pin sets, for the differential test).
- `erc_kicad`: `sexpr` (moved from berc, berc re-exports it), `netlist` (.net importer, golden dumps for the 8/9/10 fixtures), `sch` (schematic hierarchy, KiCad connectivity; `ImportOptions::kicad_version` picks the KiCad 8/9/10 rules, default = newest file format of the project).
- Manual differential run: all 41 hw projects equal KiCad 10's netlists (names included), 18 equal KiCad 8's, 35 equal KiCad 9's; the 8 test schematics equal KiCad 10's.

Next:
- Committed fixtures under `erc_kicad/tests/fixtures` (connectivity rules, hierarchy with buses, multi-instance sheet, variants) with netlists from KiCad 8/9/10 (`just kicad-fixtures`, podman) and golden dumps.
- The differential test as a cargo test: test schematics + fixtures (committed netlists), hw boards when `~/git/hw` exists (kicad-cli export, cached by content hash under the target dir).
- `.kicad_pro` (net classes, text vars, variants, ERC/DRC severities and exclusions, rules) and `.kicad_pcb` (layout) into the model.
- docs/model.md, CHANGELOG, FEATURES (NET-17 model, IMP-13 KiCad import), tpm notes.

Decisions:
- Identity (designators, net names, sheet paths, pin numbers) is structure; attributes are facts. Layout geometry carries the board file as its source at the `Layout` level; footprint fields are facts.
- Power symbols are labels of kind `power`, not parts; `#` parts (PWR_FLAG) are parts, filtered as KiCad does in `netcmp::netlisted`.
- KiCad 8 does not join a junction to a wire passing through it, 9 and 10 do: semantics follow `kicad_version`.
- hw boards are private: they are tested locally, never committed as fixtures or dumps.

Gotchas:
- Label text in files is escaped (`{slash}`); bus parsing runs on the unescaped text (`sch::unescape`).
- Sheet files of one project can have different format versions (KiCad saves only changed sheets).
