# Handoff: passive-selection (P2503), step 3

Branch s/23oct2026-passive-selection. Design note: ~/git/tpm/hobby/P2503-better-erc/design/passive-selection.md (tpm repo, commit with `tpm commit --push`). Repo pages: docs/checks.md (PAS section, RATE-25..31), docs/checks/pas.md, docs/checks/rate.md, docs/review-checklist.md, FEATURES.md, CHANGELOG.md [Unreleased]. pas.md is generated from one-line-paragraph sources in /home/roman/.claude-vhrd/jobs/7af87ec6/tmp/ (pas.md, pas-sections-1.md, pas-sections-2.md, pas-refs.md; wrap.py wraps to 120) but editing docs/checks/pas.md directly is fine too (keep the 120-column wrap; `**Impl.**` lines must match the catalogue's tier and deps).

## Done

- Steps 1-2: catalogue PAS-1..39 + RATE-25..31 with verified sources, design note, 13 tasks (P2503#23oct-1358..1400b, P2527#23oct-1359, ops#23oct-1400), ids bound to tasks.
- Roman's answers (23 Oct) folded in, commit 396d1a7 and the note: Q1 closed (LT3045 report was a YouTube video; leakage = DC error, no oscillation check); Q3 intent per board only, no lab defaults, intent-needing checks report not run; Q4 PAS-9 warning on audio boards, PAS-20 warning on automotive (suggestion on industrial/harsh); Q5 no hand-written LT entries (note §7 and §9 say 1359b dropped, generic profiles move to 1358).

## Next (Roman's refinement of Q2, user message 23 Oct; not yet done)

Replace the "0.1 % precision / 1 % otherwise" budget (written in pas.md intro paragraph "Accuracy budgets follow the circuit", PAS-2 and PAS-22 How-to-check, the PAS-2 catalogue line, CHANGELOG entry, note §5.2 `accuracy` bullet and §7 "Intent per board" fallback sentence, §10 Q2 answer) with: **precision is derived from the parts.** A precision part (precision reference, current-reference LDO like LT3045 with SET current ±1 % initial (datasheet p.1/table: 99..101 µA), LT3080 ±1 % (9.90..10.10 µA), precision op-amp/ADC/DAC/INA, current-sense amp; from part data: initial accuracy, drift, offset) sets what the passives around it need (tolerance, TCR, VCR, leakage, drift, dielectric): each passive's error contribution held well below the part's own spec (choose and document a fraction, e.g. ≤ 25 % of the part's error, configurable); a non-precision part's looser spec gives a looser budget the same way; no part data → generic profile's assumed accuracy, finding = estimate. Rule language §5.2: `accuracy` on the pin comes from the part's spec facts, not a default.

New checks, ids already claimed: **PAS-40** ⚠️ one circuit (network around a precision part, a divider, a gain/feedback network, a matched pair) mixing precision and non-precision passives where the precision part needs it: the weakest part sets the accuracy, an architectural oversight (≈ PAS-22, BOM-16). **PAS-41** 💡 precision passives (0.1 %, low-TCR thin film/foil, C0G where not needed) in a circuit that does not need precision: lost cost. Add to docs/checks.md (after PAS-39, impl mid, deps NET-7, NET-23, NET-24, CFG-5, RPT-26), pas.md sections, review checklist, FEATURES counts (PAS row 38 mid/3 hard/41; all mid 335, total 561), CHANGELOG entry, note §2/§4/§5/§10, and tasks.

Tasks to adjust (tpm-records MCP or `tpm edit/note/set`): drop P2503#23oct-1359b (status dropped, note Roman Q5); note on #23oct-1358 that generic pin profiles ship there and `accuracy` is derived from part spec facts; add PAS-40/41 to #23oct-1359c (title and text) and bind: `tpm work claim PAS-40 --task P2503#23oct-1359c`, same for PAS-41; #23oct-1359c text says test fixtures carry their own part entries (no shipped LT entries) and drop its after on 1359b; note on #22oct-0043 that intent is per board with no defaults (vibration, audio, automotive, precision, industrial, environment, ambient, life_years).

Then: `just lint`, `~/.cargo/bin/cargo test -q -p erc_checks --test catalogue` (cargo not on PATH in non-login shell), commit, `git rm HANDOFF.md`, commit, `tpm work ready --note "..."`, `tpm work say parent` a two-line summary.

## Gotchas

- `tpm work claim RATE` gives wrong ids (only knows FEATURES ids): filed ops#23oct-1400. PAS claims work.
- analog.com blocks curl; `tpm fetch URL` works for datasheets. LT datasheets text in /tmp/ps-src/lt/*.txt (RAM, may be gone).
