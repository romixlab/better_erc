# Changelog

All notable changes to better_erc are recorded here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). [FEATURES.md](FEATURES.md) holds the current
status of every feature; this file only records what changed. IDs in parentheses (`IMP-4`, `I2C-3`) refer to
FEATURES.md and the check catalogue.

All crates share one version (`[workspace.package]`). Work commits add their entries under `[Unreleased]` and
never touch the version; the landing turns `[Unreleased]` into the new version's section. History up to 0.1.0 is
grouped by month; it was backfilled on 6 Oct 2026 from the git history.

## [Unreleased]

### Added

- Footprint review: the 3D view's camera is orthographic or perspective, with the field of view as a setting (`view_3d_projection`, `fov_deg`). The 3D model section has the switch and the field of view, and the view's own right-click menu changes the same values; changing them keeps the model's size at the middle of the view. Needs vhrd_egui's `ve_view3d` with V3D-5 (`VIEW-32`).
- Footprint review: the 3D model over the page can be seen in perspective, for looking at depth (Perspective in the mode bar and in the Model views section, the field of view shared with the 3D view). The switch and the model's label say that the page's mm scale then holds at the board plane only, and Measure footprint has no model corner to snap to while it is on (`VIEW-32`).

## [0.4.56] - 2026-11-13

### Added

- Footprint review: the model onto the footprint as KiCad places it, by a key (`Home`, a setting) and a button (On footprint, in the mode bar and the Model views section). The model goes to the footprint's origin with the footprint's turn, seen from the footprint's side: from the top, or from below for a footprint in bottom view. The model entry's offset, scale and rotation are already in the model. In Silhouettes that side's view is shown and picked; in 3D the shaded model turns onto that side (`VIEW-32`).
- Footprint review: in Move a press drags what is under it: the footprint, a silhouette view or the 3D model; the empty page pans. Where two overlap, the one the moves switch (`Q`) picks wins, else the topmost. Hovering outlines what a press would pick, and the mode bar names it. The keys, turn, mirror and the SpaceMouse still move what the switch picks. Before, a press anywhere moved what the switch picked (`VIEW-32`).
- Footprint review: a hidden footprint or model is never dragged. While the one the moves switch picks is hidden, the keys, turn, mirror and the puck move the one shown, the switch shows that with the other disabled and why, and the choice comes back when it is shown again. Ctrl+drag turns only a shown 3D model (`VIEW-32`).
- Footprint review: the 3D model and its silhouettes have an opacity of their own, apart from the footprint's. The View section shows Footprint opacity and Model opacity one under the other (the Model views section has the same Model opacity), and `model_opacity` (a setting, 0.7) is where the model starts. Before, the model started at the footprint's opacity and its slider sat only in the Model views section (`VIEW-32`).

### Fixed

- Footprint review: Move no longer only turns the 3D model after the window lost its focus while the SpaceMouse's puck was turned. spacenavd sends to the focused window only, so the hand let go elsewhere and the app kept the last deflection, turning the model on and on. The puck now counts as at rest from a lost focus until it reports again. Ctrl is read once per frame, and while it makes a drag turn the model, a line on the canvas says so (`VIEW-32`).

## [0.4.55] - 2026-11-13

### Added

- Footprint review: files changed on disk are read again. Before, the tab kept what it had read: an edited footprint, symbol, board, 3D model or datasheet stayed old until the app restarted. The tab now watches every file it shows (the board and its schematics, the library tables, the symbol libraries, the footprint files, the STEP models, the datasheet). A change is read again by itself once the file stays quiet for 300 ms, and picking a symbol looks at the files first. The view stays as it was (page, scale, placement, the 3D camera, the list's filter), and the review status follows the file (a changed footprint shows its review stale). What Accept writes itself reloads nothing (`VIEW-31`).
- Footprint review: a Reload button and key (`F5`, a setting) read everything again from disk, and a line beside it says what was read again and when (`VIEW-31`).
- `berc fp todo` and `berc fp check --lib` name the files their list was made from (`files` in the JSON): the project's own files, the library tables looked at (the project's and the global one, there or not, and nested ones), the symbol libraries and the footprint files read or looked for in vain. When one of them changes the list is out of date. The footprint review watches them (`VIEW-31`).
- `berc fp todo` names the footprint a board places in place of the symbol's own on the row itself (`placed`), with its library file and review hash now (`placed_file`, `placed_hash`), or why its library has none (`placed_note`). Before, only the `overridden` list held it, under the symbol's own `footprint` (`VIEW-31`).

### Changed

- better_erc_app depends on `notify` 8 (default features off: inotify on Linux, no C library; `inotify-sys` only declares the kernel's calls) to watch the review's files, and on `jiff` (already in the tree through env_logger) with `tz-system` for the local time of the reload line (`VIEW-31`).

### Fixed

- Footprint review: a footprint the board places in place of the symbol's own (an override) is reviewed from its library file, the review hash being the library footprint's. Before, the tab read the board's copy, so a footprint edited in the footprint editor stayed old until KiCad's Update PCB from Library, even after a restart. A board copy that differs from the library is named in the note ("update the board from the library"); the board's copy is reviewed only where no library table has the footprint (`VIEW-31`).

## [0.4.54] - 2026-11-13

### Added

- Footprint review: the overlays hidden for a moment to see the drawing. `H` hides or shows the footprint, `J` the model over the page, and holding `Space` shows the page alone until it is let go (all three keys are settings). Their places are kept; the mode bar shows each switch with its key and the help line names them (`VIEW-28`).
- Footprint review: Measure across (`Y`), a ruler from the footprint to the drawing. Each end snaps to the nearer of the footprint's features (pad centres and side middles, model corners) and the drawing's lines; Ctrl takes the other when both are near. Each end shows what it snapped to, and the note names both (`M1 = 1.118 mm (pad centre to line end)`) (`VIEW-29`).
- Footprint review: X-ray (`U`) shows the 3D model see-through, in the 3D view and over the page: the faces fade and the model's own edges behind them are drawn too, thin or dashed (a setting). It sits in the mode bar, the Model views section and the 3D view's right-click menu (`VIEW-30`).

### Changed

- better_erc_app builds on vhrd_egui 0.33.1 (ve_view3d's X-ray, V3D-4) (`VIEW-30`).
- Footprint review: the mode bar keeps the room of its "back to Move" hint in every mode, so the canvas no longer jumps when switching modes makes the bar wrap (`VIEW-29`).
- Footprint review: a measurement goes into the review note only on request. A `+ note` button beside the measuring ruler's label on the canvas, beside the 3D view's measurement, and in the Measure section adds it as `M1 = … mm`; nothing is added by itself any more, and the numbers follow what was added. Moving a ruler's end after adding it makes a new measurement instead of changing the old one in the note (`VIEW-21`).

## [0.4.53] - 2026-11-13

### Added

- Footprint review: the shaded 3D model over the datasheet page at the drawing's scale (`VIEW-27`). A switch picks Silhouettes (the six views) or 3D (`L`), another what moves, the footprint or the model (`Q`); both sit in the mode bar and the Model views section, with pie slots. In 3D the SpaceMouse's tilt, roll and twist or Ctrl+drag turn the model, and letting go lands it on the nearest of the 24 sides, eased (the `model_snap` setting; Alt keeps the free angle); `K` turns it to the next side. It mixes into the page in every blend mode, and Measure footprint snaps to its outline's corners while it sits on a side.

### Changed

- Footprint review: `K` no longer cycles the footprint and then each view; it picks the next view (Silhouettes) or turns the 3D model to the next side, and `Q` switches between moving the footprint and the model (`VIEW-27`). The mode bar wraps when it is wider than the window.
- better_erc_app builds on vhrd_egui 0.33.0 (ve_view3d's snap to a side and offscreen render, ve_spacemouse's save buttons) (`VIEW-27`).
- Footprint review: the SpaceMouse panel saves with *Save to ~/.config* or *Save locally*, shows what is not saved and asks before the window closes with a changed mapping; the calibrate step asks for all six motions (ve_spacemouse SPM-6, `VIEW-27`).

## [0.4.52] - 2026-11-13

### Added

- `VIEW-26`: the footprint review's Datasheet section has a table, Datasheets by MPN: each `Mpn` field of the symbol, the library document that covers it, and its review state from the review lines (reviewed, reviewed elsewhere, stale, not yet). A click opens the document with its review line's view, else the page search runs. An MPN without a document says why, with a Fetch button. New settings: `library_mpn` (default `tpm lib mpn --json`, the MPNs appended; it prints `{"mpn", "path", "hash", "doc", "rev", "why"}` per document) and `library_fetch` (default `tpm lib fetch --mpn`). tpm has no `lib mpn` yet (ops#13nov-1420): until it does, the table says the command failed. Usage signals `app fp-review mpn datasheet` and `app fp-review mpn fetch`.

## [0.4.51] - 2026-11-13

### Added

- `VIEW-24`: the footprint review draws the 3D model's top, bottom, front, back, left and right views over the
  datasheet page at the drawing's scale: each the model's silhouette filled, outlined and with the edges seen from
  that side, to hold against the package drawing. The model view key (`K`) cycles what moves, the footprint or one
  of the views; the mouse in Move, the arrows, turn, mirror, blink, the puck and the Placement section act on it, and
  each view keeps its own place. A Model views section in the side panel shows and hides them and sets their blend
  mode, opacity and blink; Measure footprint (`G`) snaps to their corners. New settings: `keys.model_view`,
  `colors.model`, `colors.model_edges`. New dependency `i_overlay` (pure Rust, 4 small crates) for the silhouettes.

## [0.4.50] - 2026-11-13

### Added

- `VIEW-23`: the footprint review shows the footprint's 3D model. The 3D key (`3`) swaps the canvas between the page
  and the model, placed as KiCad places it on a board with the pads' copper; it is read on a worker thread, and the
  view says in words what is missing or failed. A new mode, Measure model (`O`), measures between two picked faces
  or edges on the STEP file's exact surfaces and curves: plane to plane, angle, pin pitch (axis to axis), height of
  an axis or a point over a plane, diameter, edge length. Each measurement goes into the review note as `M3 = 1.270
  mm (pitch, axis to axis)`. The side panel has a 3D model section; the pie has slots for both keys.
  `better_erc::fp::model` gains `project_dir`, `front_pad_copper` and `for_view`.

## [0.4.49] - 2026-11-13

### Changed

- `VIEW-25`: the footprint review's side panel is rebuilt on vhrd_egui's property-panel widgets. Accept and Next to
  review sit in a bar that stays in sight; the controls are in sections that fold and say on their header what they
  hold: Symbol and footprint, Datasheet and page, View, Ruler and scale, Measure, Placement, Review, Notes,
  SpaceMouse, Settings. New on the way: the footprint's position in mm as X and Y fields, a Measure section that
  starts a measuring ruler and shows its length and dX, dY, a Notes section with the notes file and the last note.
  Accept's button now reads just "Accept" (its tooltip says the rest), Next's "Next to review".
- `APP-4`: the whole app takes the vhrd look (IBM Plex fonts, the brand's dark and light styles); the tab list keeps
  each name on one line, and a loaded design's Close button comes before its path, so a long path no longer hides it.

## [0.4.48] - 2026-11-13

### Fixed

- erc_kicad: a footprint stating the same property twice (b153a_typec_serial U701 has `ki_fp_filters` twice) is one field with both facts, so the dump reads back equal to the model.

## [0.4.47] - 2026-11-13

### Fixed

- The writer's extended-set tests pass on kicad-library again. The round trip no longer demands every `ki_` property after the own ones in a hand-edited symbol (`ki_keywords` before `Mpn1`): KiCad 10 itself writes them last, the writer inserts after the last own property, and an existing earlier `ki_` stays. The KiCad oracle skips symdir files with a derived (`extends`) symbol, which KiCad cannot load without its parent.

## [0.4.46] - 2026-11-13

### Added

- `TOOL-12`: `berc fp model PROJECT|LIBRARY|FILE [NAME...]` reads each footprint's 3D models and measures them
  against the footprint: the file found as KiCad finds it, its unit, parts and faces, the extents and the height
  above the board in KiCad's 3D frame of the footprint, the model's middle against the pads', how far it reaches out
  of the courtyard, and whether its leads land on the pads. Findings are the new catalogue checks `LIB-22` to `LIB-28`;
  `--details`, `--format json`, `--project DIR`. The library side is `better_erc::fp::model` (`placement` and
  `board_placement` are KiCad's model transform, tested against `kicad-cli pcb export step`), and `better_erc::step`
  re-exports `erc_step`, so berc now builds the truck tree too.
- `erc_step::write::cuboids`: a STEP file of boxes, for tests and fixtures that may not hold a vendor model.

### Changed

- The footprint review tab's outline module (pad copper, mask and paste, graphics) moved into the library as
  `better_erc::fp::outline`, with `pad_copper` for one pad's copper polygons; the app uses it from there.

### Fixed

- `IMP-20`: an assembly written with `MAPPED_ITEM`s and no `NEXT_ASSEMBLY_USAGE_OCCURRENCE` (Würth's inductor
  models) had every part twice, once where the assembly maps it and once as a product of its own; a product whose
  shape is only mapped into another is no root now. A freeform face meshed more than ten times its edges' box
  outside them (a vertex 1000 mm off on a spline extrusion) is dropped with a warning, as a plane, cylinder or cone
  face already was past its edges; the warning names the distance allowed.

## [0.4.45] - 2026-11-13

### Added

- `IMP-20`: new crate `erc_step`, a STEP reader. `read_file`, `read_bytes` (on a worker thread with a time limit)
  and `read_str` give a `Model`: the length unit, the parts (a solid placed by the assembly tree, once per use, with
  its name, transform and colour), their faces (triangles in mm, the exact `SurfaceKind`, the colour) and edges
  (polyline, `CurveKind`, the faces they join), and `Warning`s with the STEP entity of every face, edge or entity
  not read as it should be. `measure` has the measurements on planes, axes, circles and edges. Built on truck from
  the vhrdtech fork, whose truck-geotrait caps the parameter division that never ends on some real files; pure
  Rust, no C build step. `cargo run -p erc_step --example step_info FILE...` prints what a file reads as.

## [0.4.44] - 2026-11-13

### Changed

- Footprint review: the SpaceMouse's mapping is the one every app on ve_spacemouse shares, `spacemouse.toml`
  (`.config/spacemouse.toml` in the folder the app started in and its parents, then `~/.config/spacemouse.toml`,
  the nearest file winning per key), shown and saved by ve_spacemouse's settings panel: Calibrate…, the file each
  value comes from, Save to every folder or to this project. `[spacemouse]` in `config.toml` moves into
  `~/.config/spacemouse.toml` at the tab's first start, once, keeping the calibration; keys that file already has
  keep their values (`VIEW-20`). Needs vhrd_egui with SPM-5 (`cargo update -p ve_spacemouse`).

## [0.4.43] - 2026-11-13

### Added

- `IMP-21`: a footprint's `(model ...)` entries read in full (path, `offset`, `scale`, `rotate`, `hide`,
  `opacity`), in `.kicad_mod` and `.kicad_pcb` · `erc_model::layout::ModelEntry`. The path resolved against the
  same `${VAR}` chain as a library table (`IMP-18`'s `KicadVars`), a missing `.wrl` trying `.step` and `.stp`
  beside it, each result saying which variables resolved it or why it did not · `erc_kicad::model3d::resolve`.

## [0.4.42] - 2026-11-13

### Changed

- `ve_spacemouse` (better_erc_app) and `exact_units` (erc_levels) no longer pin a tag or a rev: both are plain
  `git = "..."` dependencies, so `cargo update` picks up the default branch's latest commit. `exact_units` moved
  forward to a commit where `Fixed::new` and `Point::from_raw` no longer accept a float (they now only compile for
  an integer storage, to keep a NaN out of a checked value); `erc_levels` now builds `Volts`/`Amperes`/`Farads`/
  `Ohms`/`DegC` with `try_new`/`try_from_raw` and propagates a NaN as an error, as it already did for other bad
  input.

## [0.4.41] - 2026-11-13

### Added

- `TOOL-11`: a footprint review records where it was made, in the symbol: one review line per datasheet, in
  `Footprint_Review`, `Footprint_Review2`, ..., the hash first, then the MPNs it covers, the document number and
  revision, the PDF by the document library's blake3, the page, scale and placement, who and when. `berc fp check`
  (board and `--lib`) and `berc fp todo` check every line's hash and that every `Mpn`/`MpnN` of the symbol is on a
  line: a new status `partial` names the MPNs not reviewed yet, `changed` names the stale lines, `--details` prints
  every line, the JSON has `reviews`. A value with the hash alone stays valid. `berc fp accept --mpn --doc --pdf
  --page --by` writes a datasheet's line among the others; the app's Accept asks which MPNs the datasheet covers
  (those in its text ticked), records the view, and opening a reviewed symbol opens the recorded PDF from the
  document library with that page, scale and placement, saying when a line is stale or its PDF is not found. New
  app settings `library_docs` (`tpm lib docs --json --all`) and `reviewer`.
- `IMP-19`: `erc_kicad::symlib::remove_property` and `property_names`.

### Changed

- `berc fp accept` and the app's Accept write the review line merged with the symbol's other lines rather than
  overwriting `Footprint_Review` with the hash: `AcceptRun` lists each property written with its new value
  (`Written::new`, `None` for one removed) and the symbol's lines (`Accepted::lines`). `fp::accept` and
  `fp::accept_library` take the review line's source (`fp::record::ReviewLine`).

## [0.4.40] - 2026-11-13

### Added

- `TOOL-10`: placed footprint overrides, shown rather than resolved silently. `berc fp check` and `berc fp todo`
  name a board footprint placed under a different `Lib:Name` than its symbol's own `Footprint` field (a library
  migration half done, or a deliberate per-instance override); a footprint placed with no library nickname at all
  is not an override. The app's Footprint review tab follows the board: it reviews the footprint actually placed
  when it differs, noted above the canvas. `berc fp accept --footprint LIB:NAME`, and the app's Accept
  confirmation (a checkbox offering the placed one), also set the library symbol's own `Footprint` to it, through
  `IMP-19`'s writer; refused for references of more than one symbol at once. A new catalogue check, `LIB-20`.
- `TOOL-10`: a setting lists the library nicknames a symbol and its footprint may be in, an exact name or a `*`
  prefix (`berc fp check --lib`/`berc fp todo --allow NICKNAME`, repeatable; the app's `[footprint_review]
  allowed_libraries`), default none (no rule); any other library named is reported. A new catalogue check,
  `LIB-21`.

## [0.4.39] - 2026-11-13

### Fixed

- Footprint review: the once-reviewed line (the `Footprint_Review` field, old value and new) was a row of the
  footprint's own grid, so its first appearance (and the full hash it carries) changed that grid's layout, taking
  the UI a few frames to settle instead of one; moved it below the grid, as a line of its own, so the grid's rows
  never change. Fixes a deterministic GUI test failure (`review_a_symbol_from_its_library`'s `run()` after Accept's
  Write exceeded `max_steps`).

## [0.4.38] - 2026-11-13

### Added

- `VIEW-22`: Footprint review, select the drawing and save it as DXF. Select (V) is the fifth mode, on the mode bar,
  in the help line and in the left pie ring: a click picks a piece the PDF draws (a line, polyline, arc or curve, or
  closed shape, kept with its curves; inside a filled shape counts) and again drops it, a drag selects what is wholly
  inside its box, Shift adds; the selection is drawn in cyan. Save DXF (W, the button under the mode bar, the
  pie's both-buttons ring) writes it beside the notes file as `<symbol>-p<page>-<n>.dxf`, in the footprint's mm
  through the scale and the footprint's placement, so KiCad's footprint editor imports it at 0, 0 over the pads, and
  appends ``DXF `name` `` to the note. The DXF is R12 ASCII written by the app (LINE, ARC, CIRCLE, POLYLINE, comment
  lines on where it came from); Béziers that follow a circle become arcs and circles, others polylines within
  `dxf_tolerance_mm` (0.005 by default). Save is disabled with the reason until something is selected, the scale is
  set on the page and a footprint is shown. New keys `select` and `save_dxf` in `[footprint_review.keys]`.

## [0.4.37] - 2026-11-12

### Added

- `VIEW-21`: Footprint review notes without leaving the tab. A note field sits above the canvas; Enter over the canvas
  (`[footprint_review.keys] note`) or the pie's Write a note gives it the keyboard. Enter in it, or Add, appends the
  note to a Markdown file, by default `footprint-review-notes.md` beside the project or library (`notes_file`). Each
  note has the symbol, the footprint, the datasheet and its page, the time and the text. The datasheet is named as the
  document library names it, by its document number and revision where known (`tpm d DS13560 rev6`, which survives a
  rename or a move), through a command (`library_name`, `tpm lib open --json` by default); without it, the file.
  Measurements from the measuring rulers go into the note as `M1 = 1.234 mm`.
- `VIEW-19`: Footprint review, four modes on a bar above the canvas: Move (`M`), Scale (`R`, the ruler that sets the
  scale, as before), Measure PDF (`E`) and Measure footprint (`G`). The mode on is highlighted on the bar and named
  first in the help line, and each mode is in the pie menu. Measure PDF lays the ruler on the drawing's lines. Measure
  footprint snaps its ends to the pads' centres and to the middles of the sides of rect and roundrect pads. A
  measurement goes into the review note as `M1 = 1.234 mm`; moving an end changes that value, a new ruler is `M2`. The
  measuring ruler shows dx and dy; the scale's ruler waits while measuring and is back in Move.

### Changed

- `VIEW-19`: Footprint review, the bottom view's key is `X`; `M` is Move now. A settings file saved with `mirror =
  "M"` takes `X` for it when read.

### Fixed

- better_erc_app tests: the footprint review's kittests no longer fail now and then under load with `Harness::run
  exceeded max_steps`. The PDF reader and the library search sent their reply, then woke the UI from their own thread;
  the frame could take the reply in before that wake landed, so the wake fell into a later frame and how many frames a
  step took hung on the thread's timing (2 of 10 suite runs on 0.4.36 under load,
  `library_search_opens_the_first_hit`). In the tests the threads now wake nothing and the UI asks for the next frame
  itself when it takes a reply in; the app keeps the threads' wakes.
- `VIEW-19`: Footprint review, the ruler could not be left: Esc ended the ruler step but kept the ruler tool, so the
  mouse no longer moved the footprint. Esc, `M` and a mode's own key again now go back to Move.
- `VIEW-19`: Footprint review, the S length box took no typing and kept the app busy while it was open
  (0.4.33-0.4.36). It asked for the keyboard on every frame, and in egui each such request interrupts the input
  method: on Wayland egui-winit switched the input method off and on every frame, which lost what was typed and woke
  the next frame. The box now takes the keyboard once. The kittest of 0.4.33 typed through plain text events, which no
  input method touches; the new test types through input method events and fails when a frame interrupts them.

## [0.4.36] - 2026-11-12

### Changed

- `berc textconv` (`TOOL-3`): a `.kicad_sch`'s `lib_symbols` now print each library symbol's fields (`Datasheet`,
  `Footprint`, `Footprint_Review`, ...) as readable `Name=value` lines instead of folding them into the symbol's
  opaque hash, which now covers only its pins, graphics and sub-symbols. Accepting a footprint review (`berc fp
  accept`, `TOOL-9`) writes `Footprint_Review` into the schematic's copy of the library symbol, so this is the
  change that turns that accept into a one-line, readable diff instead of a changed hash taken on faith.

## [0.4.35] - 2026-11-12

### Added

- `VIEW-20`: Footprint review on a SpaceMouse, through spacenavd (vhrd_egui's `ve_spacemouse`, pure Rust): the puck
  pans the page in screen points (the same speed at every zoom) and its push and pull zoom the view about the
  pointer; P (`[footprint_review.keys] puck`) switches it to move the footprint instead. A tap of the left button
  lays the ruler at the pointer, of the right one switches what the puck moves. Held, each button opens a pie menu
  of the canvas' actions (left: page and ruler, right: footprint, both: Accept and Next to review); every slot
  names its action and key, a disabled one says why; while Accept's question is open the taps write or cancel.
  The canvas' keys and the slots are one set of actions. New keys A (Accept) and N (Next to review). Calibrate…
  in the tab's SpaceMouse section measures the mapping; it is the app's, `[spacemouse]` in the settings file.

### Fixed

- Footprint review tests: the test tab ends each frame with the PDF reader done with what the frame asked. Its
  replies came in on the reader's thread, each a repaint, so under load a `Harness::run` after a zoom or a page
  change saw them arrive over more than its 4 steps (`ruler_snaps_to_the_drawing`, `library_search_opens_the_first_hit`,
  `review_a_symbol_from_its_library` failed about one run in four at 32 test threads).

## [0.4.34] - 2026-11-12

### Added

- `VIEW-19`: Footprint review tab, the ruler's ends snap to the drawing's own lines, read from the PDF's vector
  paths (stroked lines' centre lines and widths, thin filled rectangles as lines, other filled shapes' edges): a
  line's end or two lines crossing within `[footprint_review] line_snap_px` (10 screen pixels; 0: never), else onto
  the nearest line. On a level ruler each coordinate snaps on its own, so on a horizontal ruler an end takes the X
  of the extension line beside it. The target shows at the pointer before the end is placed; Alt held leaves the
  end free. The number the dimension prints between the ruler's ends, read from the page's glyphs (a number shown
  in pieces read as one word), is outlined and offered as a chip beside the ruler's length; D
  (`[footprint_review.keys] dimension`) or a click on it sets the scale, and S still asks for another number. Both
  named in the help line.

## [0.4.33] - 2026-11-12

### Added

- `VIEW-19`: Footprint review tab, the ruler snaps level: horizontal or vertical by the larger of its dx and dy (a
  new ruler from its first end), an end dragged goes where it is dragged and the other end follows on that axis.
  Shift held, or `[footprint_review] ruler_snap = false`, lets it go at any angle. Named in the help line.

### Changed

- `VIEW-19`: Footprint review tab, S (and Z a third time) asks the ruler's length in a box in the middle of the
  canvas, the number typed shown large in mm; Enter sets the scale, Esc leaves it. Before, the keyboard went to the
  side panel's length field.

### Removed

- `VIEW-19`: Footprint review tab, the colour presets and pickers; every colour is a setting
  (`[footprint_review.colors]`), whose `pads`, `back_pads` and `vias` are gone (copper, mask and paste colour
  both sides).

### Fixed

- `VIEW-19`: Footprint review tab, Accept's warning that KiCad may overwrite the change is per file, naming the
  lock file and its owner: only a held KiCad lock beside that file (`~NAME.EXT.lck`). A lock from this computer
  counts while a KiCad runs here (or while its `pid` lives, should a lock carry one); an empty lock is stale. A
  KiCad running with the file closed no longer warns.
- `VIEW-19`: Footprint review tab, the copper, mask and paste views show only what the board makes on that layer,
  each filled in its one colour: Copper the pads' copper, copper graphics and vias (red), Mask the mask openings
  only (pink), Paste the paste apertures only (gray). Before, the pads were drawn in every view and the mask and
  paste shapes were outlines over them. Holes and pad numbers stay over every view. Back pads follow the same
  views behind their checkbox; the Pads and Vias checkboxes are gone.

## [0.4.32] - 2026-11-12

### Added

- `VIEW-19`: Footprint review tab, the page can be shown inverted (white-ish on black), `I` to toggle (setting
  `keys.invert`); the default is a setting too (`[footprint_review] invert`).

### Changed

- `VIEW-19`: Footprint review tab, copper graphics, mask and paste are shown one at a time (a radio choice, `C`
  cycles through them), not independent toggles that stack; each keeps a fixed colour (copper red, mask pink,
  paste gray) in `[footprint_review.colors]`, with no picker in the tab (a datasheet page is white, so the
  colour says what is shown).

## [0.4.31] - 2026-11-12

### Added

- `VIEW-19`: `S` on the Footprint review canvas (setting `keys.scale`) goes back from a ruler end to the view, empties
  the ruler's length field, scrolls it into sight and gives it the keyboard; Enter sets the scale and the footprint
  is the one to move again. The ruler step by keys is F, R, R, Z, Z, S, the length, Enter.

### Changed

- `VIEW-19`: Accept in the Footprint review tab asks before it writes into the library symbol and the schematic's
  copies: a dialog shows its dry run, each file with the field and its old and new value; Enter writes, Esc writes
  nothing. It warns when KiCad may overwrite the change on its next save: a file with KiCad's lock file beside it
  (`~rev.kicad_sch.lck`), or a KiCad process running.

### Fixed

- `VIEW-19`: the Footprint review tab shows the mask openings and paste apertures it was asked to: they were drawn
  before the pads, and an opening the size of its pad (no margin) or inside it (an exposed pad's mask and paste
  windows) vanished under the pad's fill, as on U401 of b153a (TI RUK0020B). They are drawn over the pads now, at
  least 2 px wide, and take the open board's `pad_to_mask_clearance`, `pad_to_paste_clearance` and its ratio
  where neither the pad nor its footprint sets a margin. The test checks the rendered pixels on a land pattern of that shape, written by the test,
  opaque, where a line under its pad does not show; the old one only checked the list of shapes. A long footprint
  name no longer widens the side panel past its edge, which hid the Mask checkbox.

## [0.4.30] - 2026-11-12

### Added

- `VIEW-19`: keys for each step of the Footprint review canvas, each a setting (`[footprint_review.keys]`, egui
  key names) and named in a help line under the canvas and in the tooltips: `F` shows the page at the view's full
  width; `R` puts one ruler end at the pointer and `R` again the other (the coarse ruler); `Z` zooms in on the
  first end, where a click or a drag places it exactly, `Z` again on the other end, `Z` again back to the view
  with the length field taking the typed length; `Esc` ends the ruler step. Buttons for the same: Full width,
  Whole page, Zoom on an end. `ruler_zoom` sets how far `Z` zooms in (8× the view).
- `VIEW-19`: the Footprint review tab draws a package's vias as vias: a ring and its hole, of their own layer group
  (Vias, its own colour `colors.vias`), without the number they share with the exposed pad. KiCad has no vias in
  footprints; a package's vias are round plated pads with no number, a heatsink mark or the number of a surface
  pad, and are in the review hash as pads, as they always were: hashes already written stay valid, pinned by tests
  (`fp1` of the test footprints, and of a footprint without vias, byte for byte).

### Changed

- `VIEW-19`: the datasheets the library search finds sit in a panel of their own, each a coloured button (the open
  one marked), and for a part just picked the first one opens by itself; the page search then finds its land
  pattern page.
- `VIEW-19`: once the scale is set (Set scale, Enter in the length field, or the page's scale note), the tool is
  Move footprint again by itself. Turning the footprint by 90° is `T` now (it was `R`, which is the ruler's).

### Fixed

- `VIEW-19`: the Footprint review tab's Mask and Paste layers show each pad's mask opening and paste aperture (by
  the pad's margins, else the footprint's); before, they drew only mask and paste graphics, which most footprints
  have none of, so ticking them changed nothing.
- `VIEW-19`: picking another part while a datasheet is open opens the new part's datasheet, or closes the old one
  when the part has none (the library search may still find it); before, the last part's PDF stayed open.

## [0.4.29] - 2026-11-12

### Added

- `TOOL-9`: `better_erc::fp::accept_library` writes a review hash into a library symbol without a project, for a
  symbol picked from a library; `fp::SymbolEntry` (`berc fp check --lib`'s JSON) carries the symbol's `datasheet`,
  and `SymbolStatus::needs_review` is shared with `fp::todo`.

### Changed

- `VIEW-19`: the Footprint review tab starts from a library symbol, the thing a schematic places and that holds
  the hash. A board or project lists the library symbols its board uses (`fp::todo`, those to review first); a
  symbol library (`.kicad_sym`, `.kicad_symdir`, or a nickname from the `sym-lib-table`s) lists its symbols; each
  row with its status. A picked symbol brings its footprint (the library footprint its `Footprint` field names)
  and its datasheet (its `Datasheet` field); one without a footprint, or not found, is refused with the reason.
  Accept writes the hash into that symbol: the library symbol, and for a project's symbol the schematic's copies.
  Next to review steps through the symbols that need one. A `.kicad_mod` opened alone or a footprint from the
  board's footprint list is a preview with Accept disabled and the reason (no symbol to put the hash on). Every
  control stays: colours, blend modes, ruler, page search, layers. Kittest tests drive a symbol from a library the
  test writes to Accept writing the hash, a project's symbol to the library and schematic copies, and the preview.

## [0.4.28] - 2026-11-12

### Added

- `VIEW-19`: the Footprint review tab's Accept writes the review hash into the symbols when the footprint was picked
  from a board: the `Footprint_Review` field of the library symbol of every footprint placed with it, and of the
  schematic's copies (`better_erc::fp::accept`, `AcceptWhat::Reviewed`: the reviewed library footprint's hash, the
  board copies may still differ). The tab lists the files written, a library that was not found, or why nothing
  was written (a symbol placed with other footprints too); the hash is still copied. A kittest test picks a board's
  footprint, accepts it and runs `fp::check` on the result.
- `TOOL-9`: `berc fp accept PROJECT REF...` writes each board footprint's review hash into the `Footprint_Review`
  field of its schematic symbol's library symbol: the `.kicad_sym` (or the symbol's file in a `.kicad_symdir`)
  found through the project's or the global `sym-lib-table`, and every schematic sheet's `lib_symbols` copy, so
  `berc fp check` finds them ok. `--symbol LIB:NAME --hash HASH` writes a hash given by hand; `--dry-run` writes
  nothing; `--format json`. A symbol placed with more than one footprint (one hash can't hold for them all),
  footprints of one symbol that differ, a symbol that sets no footprint, a footprint without a symbol or a
  malformed hash are refused, and nothing is written when anything fails; each file is replaced through a
  temporary file beside it. Library side: `better_erc::fp::accept`,
  `erc_kicad::libtable::SymbolLibraries::symbol_file`, `erc_model::fphash::is_hash`.
- `IMP-19`: the first slice of the KiCad writer, formatting-preserving: `erc_kicad::edit` parses a file into a tree
  that knows where each expression is and writes only the byte ranges that change, so a file read and written
  unchanged is the same bytes and a change is one diff hunk. `erc_kicad::symlib::set_property` sets a symbol's
  property in a `.kicad_sym` (a whole library or one file of a `.kicad_symdir`) or in a schematic's `lib_symbols`
  copy: only the value token when the property is there, else a new hidden property after the symbol's own ones
  (before `ki_keywords` and `ki_fp_filters`) in the layout KiCad gives the file's version (KiCad 10's `show_name`,
  `do_not_autoplace` and `hide` on the property; KiCad 8 and 9's `hide` in the effects), indent and line breaks as
  the file has them. Files older than KiCad 8 are refused. Tested on synthetic files of each layout, with KiCad as
  the oracle where kicad-cli is installed (KiCad saving then our edit is our edit then KiCad saving;
  `BERC_WRITE_KICAD=8,9,10` adds versions through podman), and on every file of `~/git/kicad-library` and `~/git/hw`
  where they are there (`BERC_KICAD_LIBRARY_DIR`, `BERC_HW_DIR`; every symbol with `BERC_HW_FULL`).

## [0.4.27] - 2026-11-12

### Added

- `TOOL-8`: `berc fp todo [PROJECT]` lists the library symbols a board's footprints come from, once each with the
  references that use them, checked against their library footprint as `fp check --lib` checks a library: those to
  review first in review order (changed, other version, malformed, unreviewed; by symbol), then those that can't be
  reviewed until fixed (missing, refused), then ok and no footprint (counted, `--all` lists them). Text and JSON (every
  symbol with its footprint file, the hash to write and the `Datasheet` field), exit 1 when anything is left. In the
  library: `better_erc::fp::todo` returning `FpTodo`, whose `to_review()` is the review tab's steps.
- `TOOL-8`: `berc fp check` compares each board copy with its library footprint (the board's `Lib:Name`, else its
  symbol's `Footprint` field, through `--footprints` and the `fp-lib-table`) and reports a copy that differs, both
  hashes shown: the board was not updated from the library. It fails the run like a stale schematic copy; footprints
  that could not be compared are summed up once per reason. New JSON fields `library_footprint`,
  `library_footprint_hash`, `board_differs`, `footprint_note`.
- `berc fp check` and `fp todo` take `--symbols DIR` (symbol libraries searched before the `sym-lib-table`) and
  `--footprints DIR`, which no longer needs `--lib`; `fp todo` and `--symbols` are usage signals.

### Changed

- `better_erc::fp::check` takes a `LibrarySearch` (KiCad's configuration directory and libraries by path) instead of a
  closure making `SymbolLibraries`; `LibCheckOptions` holds one in place of `config` and `footprints`. New
  `erc_kicad::libtable::SymbolLibraries::add_first`.

## [0.4.26] - 2026-11-12

### Added

- `TOOL-8`: `berc fp check --lib LIB` checks every symbol of a symbol library against the review hash of the library
  footprint its `Footprint` field names, before any board uses it: ok, changed, unreviewed, missing (the footprint is
  not found, with the reason), refused (a hash on a symbol without a footprint), other version, malformed, and no
  footprint (listed last, not a failure). `LIB` is a `.kicad_sym`, a `.kicad_symdir`, a folder of them or a
  `sym-lib-table` nickname, repeatable; `--footprints DIR` adds `.pretty` libraries searched before the
  `fp-lib-table`. Text and JSON, exit 1 on anything but ok and no footprint; `--lib` completes the table's nicknames
  and paths, and `fp check --lib` and `--footprints` are usage signals. In the library:
  `better_erc::fp::check_libraries`, and `erc_kicad::libtable::SymbolLibrary` (one `.kicad_sym` or `.kicad_symdir`:
  its symbol names, a symbol's fields, also where a directory library's file is not named after its symbol),
  `SymbolLibraries::library`, `FootprintLibraries::add_first` and `erc_kicad::symlib::symbol_names`.

## [0.4.25] - 2026-11-12

### Added

- `TOOL-8`: `better_erc::fp::FootprintSource`, one way to get a footprint to review: a `.kicad_mod` file, a board
  copy (`BOARD.kicad_pcb` and a reference), or a library id (`Lib:Name`) through the project's and the global
  `fp-lib-table` (`erc_kicad::libtable`). `read` gives the footprint in its own frame, its review hash and its fields;
  `hash_file` (`berc fp hash`) goes through it. The usage record moved into `better_erc::usage`, so the app writes
  its steps into the same file as berc; `berc usage` counts them in an `app` column.
- `VIEW-19`: the app's Footprint review tab, no longer a placeholder: a library footprint drawn over its datasheet
  drawing. Open a `.kicad_mod`, or a board to pick a footprint from (reviewed from its library file through
  `fp-lib-table`, the board copy compared; else the board copy). The datasheet from the `Datasheet` field's file, a
  local library search command, or Open PDF; PDFs are read and drawn by hayro (pure Rust, no default features) on a
  thread of their own, and a page search goes to the land pattern page. The drawing's scale is set with a ruler
  along a dimension (or the page's `SCALE: 20X`); the footprint is moved, turned, mirrored, half see-through or
  blinking, layers shown or hidden. Accept copies the footprint's review hash for the symbol's `Footprint_Review`
  field and keeps the review; it says why while it can't be used. The app's settings file
  (`~/.config/better_erc/config.toml`, `[footprint_review]`): blink rate, opacity at first, page search words,
  finest drawing scale, font folders, library search command. A kittest test drives the review on a PDF it writes.
- `VIEW-19`: the footprint's colour (presets for the pads, a picker per layer group) and blend mode: normal,
  multiply, screen and difference, where white pads over white paper go black so an exact match goes dark. The
  modes are a wgpu pass (an egui paint callback) whose shader reads the page's pixel under the footprint; without
  the wgpu renderer they are disabled and say why. Defaults in `[footprint_review]`: `blend` and `colors`
  (`#rrggbb` per layer group). A kittest test reads the rendered pixels for each mode.

## [0.4.24] - 2026-11-12

### Added

- `IMP-18`: `erc_kicad::libtable`, the one reader of KiCad's library tables. Footprint libraries come in beside symbol libraries: `FootprintLibraries::footprint("Lib:Name")` reads `Name.kicad_mod` from the library's `.pretty` directory into a `LocalFootprint` (`IMP-17`), `names(nick)` lists a library, `tables().libraries()` lists the libraries KiCad would load. Both kinds read the project's table, then the global one, follow nested tables, skip disabled libraries, and fill `${KICAD9_FOOTPRINT_DIR}` and the like from an installed KiCad when nothing else sets them.

### Changed

- `TOOL-8`: `SymbolLibraries` moved from `better_erc::fp` to `erc_kicad::libtable` (still re-exported from `better_erc::fp`), and `SymbolLibraries::kicad_config_dir()` is now `libtable::kicad_config_dir()`. `fields` returns `Result<_, LibError>` instead of `LibraryLookup`; `berc fp check` reports the same notes. `erc_kicad::symlib` keeps only the reading of symbol fields; `read_lib_table` and `expand` moved to `libtable`.

## [0.4.23] - 2026-11-12

### Fixed

- `TOOL-8`: `berc fp check` reads KiCad 9+ directory symbol libraries (`sym-lib-table` entries whose `uri` is a
  `.kicad_symdir`, one `.kicad_sym` per symbol, named after it): a derived symbol's `extends` chain now follows
  into the directory's other files for its parent's fields. Before, every such library read as "can't read" and
  the check fell back to the schematic's copy.

## [0.4.22] - 2026-11-12

### Added

- `TOOL-8`: `berc fp check [PROJECT]` checks every footprint of a KiCad board against the review hash in the
  `Footprint_Review` field of its library symbol, read from the `.kicad_sym` through the project's or the global
  `sym-lib-table` (`erc_kicad::symlib`), else from the schematic's copy. Each footprint is ok, changed, unreviewed
  (with the hash to put there), refused (the symbol sets no footprint), no symbol, other version, malformed or
  board-only (no symbol and marked board-only in KiCad: listed last under its own heading, not a failure); a
  schematic copy whose hash differs from the library's is reported too. Text and JSON, exit 1 on anything but ok and
  board-only.
  README section "Footprint review".
- `berc usage`: every `berc` run appends its command and its feature flags to a local usage file
  (`$BERC_USAGE_FILE`, else `$XDG_STATE_HOME/berc/usage.tsv`); `berc usage` sums it per command, unused ones at 0.
  `BERC_USAGE=off` records nothing. Nothing is sent anywhere.
- `TOOL-8`: the footprint review hash (`erc_model::fphash`): `fp1:` and 16 hex digits over a canonical text of what a
  footprint review checks (pads, copper, mask, paste, courtyard and board-edge graphics, zones and keepouts,
  clearance and mask settings), the same for a library file and every board copy of it, unchanged by silk, texts,
  re-saves, reordering and placement. `berc fp hash FILE.kicad_mod` or `berc fp hash BOARD.kicad_pcb REF` prints it;
  `--canonical` adds the text, `--format json`; board references complete in the shell. `fphash::compare` tells a
  stored hash the same, changed, made by another version or malformed.
- `IMP-17`: `erc_kicad::footprint::read` reads a library footprint (`.kicad_mod`) into the footprint and pad types
  the board reader fills, in the footprint's own frame (`erc_model::LocalFootprint`). `Layout::local_footprint`
  brings a board footprint back to that frame: position and rotation undone, a footprint on the back flipped to the
  front with its layers, pad angles, chamfered corners, trapezoid deltas, drill offsets and custom pad primitives.
  Test: a board KiCad 10 saved with two test footprints placed twelve ways (`just footprint-fixture`); every copy
  equals its library file.
- The board reader keeps more of each footprint: its zones and keepouts (`Footprint::zones`), its clearance, mask,
  paste and thermal settings (`Footprint::settings`), and per pad the roundrect and chamfer ratios, chamfered corners,
  trapezoid delta, drill offset, custom pad anchor and clearance shape, clearance and thermal settings and a KiCad 9
  padstack (`Pad::details`). Before, footprint zones and these settings were skipped. The canonical dump carries them
  (`fp ... zone` lines, settings on the `fp` line, details on the `pad` line); the board golden dumps gain them.

## [0.4.21] - 2026-11-12

### Added

- `VIEW-19` Footprint review tab, empty for now: a place for the schematic-vs-layout footprint review this and
  the `fp-check` session's companion work in `erc_kicad`/`erc_model`/`berc` will fill in.

### Changed

- `better_erc_app` onto the vhrd_egui stack: `egui`/`eframe` 0.31 → 0.36 (wgpu renderer instead of glow),
  `egui_tiles` 0.12 → 0.17, `egui-phosphor` 0.9 → 0.14 (`APP-1`). `eframe::App::update` became `App::ui`
  (the given `Ui` has no panel/background, so the body wraps its panels in it instead of a `Context`);
  `egui::SidePanel`/`TopBottomPanel` merged into `egui::Panel::{left,top,...}`; `egui::menu::bar` became
  `egui::MenuBar::new().ui(...)`; `ui.close_menu()` became `ui.close()`;
  `egui_tiles::SimplificationOptions` gained `flatten_tabs_in_tabs`. The existing tabs (`VIEW-1..4`,
  `PcbDataImport`) needed no changes beyond that; none had to be dropped.

## [0.4.20] - 2026-11-07

### Added

- `LIB-19` (check): a hidden symbol pin that is not grouped with a visible pin of the same meaning. A pin hidden on the
  net of visible pins with other names (an N.C. pin under GND) or with no visible pin of its part on its net (a supply
  pin joined by name) is a warning; grounds and the pad grouped under one GND pin pass. The `strict` setting
  (`checks.LIB-19.strict = 1`) also reports grouped and hidden no-connect pins, as suggestions. Test schematic
  `lib-19_hidden_pins`.

### Changed

- `just netlists` keeps its picture markers (`test_schematics/generated_pictures_cache`) in git: a test picture is
  drawn again only when its schematic changes, not on a fresh clone or another PC, so a newer kicad-cli no longer
  rewrites every SVG.
- docs: internal references removed

### Fixed

- `erc_kicad::pcb::read` fills each footprint's `pads` with the ids of its own pads, in file order. It left the
  list empty, so a layout read on its own (without a schematic to link it to) had pads that named their footprint
  but footprints that listed no pads; the linked board was not affected (`IMP-14`).
- The differential test's golden dump of the `lib-19_hidden_pins` test schematic is committed: the `LIB-19` work
  added the schematic without it, so `just test` failed on a clean checkout.

## [0.4.19] - 2026-11-04

### Added

- `erc_levels`, the signal-level database (`NET-27`): `data/signal_levels.toml` with one table per logic family,
  bus or differential standard and supply variant, 134 of them from 74 documents (supply range, input and output levels with their test
  currents, input currents, hysteresis, differential levels, termination), each value with the document it was
  read from (in the technical library, by section, table and page) and `verified = true` only when read there;
  a typed loader (`Database::builtin`, limits as numbers or expressions of a supply like `0.7*VDD`), a validator
  run by the tests (known sources, min <= typ <= max at the supply corners, VIL < VIH, VOL < VOH, units per
  parameter), and `cargo run -p erc_levels --example validate -- FILE` to check a file before merging it.
  Values come out as `exact_units` types (`Volts`, `Amperes`, `MilliAmperes`, `Ohms`, `Farads`, `DegC`; `Dim`
  for a unit's dimension); `exact_units` is a git dependency (electrical and thermal units only, no further
  dependencies).
  Using it in the level checks is `DIAG-7`.

## [0.4.18] - 2026-11-04

### Changed

- README "Readable git diffs": no longer recommends `diff.kicad.cachetextconv`. `berc textconv` is fast enough
  (0.12 s on a 17 MB `.kicad_pcb`, 8 ms on the largest `.kicad_sch` of a real hardware repository) that caching it
  isn't worth the staleness: git's cache is per blob and per textconv command, and a `berc` upgrade keeps the same
  command, so old blobs keep showing their old cached text until the command changes or the notes ref is cleared.
  Kept a short note for anyone who turns `cachetextconv` on anyway: clear `refs/notes/textconv/kicad` after every
  upgrade that changes the output (`TOOL-3`).

## [0.4.17] - 2026-11-04

### Fixed

- `berc textconv` on a `.kicad_pro`: the fold-count comment (`# N settings at KiCad's default or empty, folded:
  ...`) no longer carries a number, so a newer KiCad writing a few more settings at their default doesn't show as
  a change by itself; `schematic.used_designators` (and other comma-joined settings KiCad rebuilds from a set,
  not a sequence) is sorted naturally before printing, so a plain re-save that only reordered it no longer shows
  as a change either (`TOOL-3`).

## [0.4.16] - 2026-11-03

### Fixed

- `erc_kicad`'s differential test (`test_schematics_equal_kicad`) golden dumps for the `DRAW-40`
  `power_text_clash` and `wire_through_label` test schematics, missing since they were added in d7033e3.

## [0.4.15] - 2026-10-31

### Changed

- `DRAW-12` (library orientation) softened to a hint: mirroring, or an upside-down IC, is fine when it reads
  better, such as a level shifter facing its line side at the connector; never a warning.

### Fixed

- `DRAW-40` candidates (a straight drop's new symbol, a direct path's wire) that would run through, or fall short
  of their margin to, anything else already drawn (`DRAW-7`, `DRAW-9`, `DRAW-34`), or place a new symbol as a near
  miss of another (`DRAW-36`), are dropped; a tie with no cheaper, clean candidate left reports nothing. Two test
  schematics (`draw-40_power_text_clash`, `draw-40_wire_through_label`) cover the real-board cases that found
  it.
- `NAME-13` accepts an adjustable rail named `+<name>_ADJ` (`+VIO_ADJ`) in place of a voltage it doesn't have.

## [0.4.14] - 2026-10-31

### Added

- Drawing geometry (`NET-26`): `Board::geometry` holds per sheet file the paper, title block, placed symbols with body box, pins (direction, length, name and number boxes) and fields, labels, wires, buses, junctions, free text, text boxes, tables, groups and graphics. Text boxes use KiCad's stroke font metrics and are compared with `kicad-cli sch export svg`. The dump does not carry the geometry (`Design::without_geometry`).
- Drawing style checks on the sheet geometry (`NET-26`): a per-sheet scene of boxes and wire ties (`erc_analysis::scene`), and the checks `DRAW-7`, `DRAW-9` (bare overlaps), `DRAW-34` (clearance margins), `DRAW-36` (near-miss alignment), `DRAW-28` (label on its stub), `DRAW-29` (pin on pin), `DRAW-30` (symbols per pin group), `DRAW-32` (free text and note kinds), `DRAW-35` (small parts vertical), `DRAW-40` (routing cost), `DRAW-41` (pin lead), `DRAW-43` (sentence capitals) and `NAME-13` (power net form). Findings name the pair of objects with their sheet positions. Thresholds and weights are settings; those shared by several checks are named `drawing.*`.
- `berc check --set NAME=VALUE` sets a threshold for a run (`drawing.clearance.part=2`, `DRAW-30.max_symbols_per_rail=2`); an unknown name is refused.
- Sections on a sheet, from their headings: `DRAW-44` (two sections closer than `drawing.section_gap`), `DRAW-45` (section headings just off a shared row or column), and `DRAW-36` now also names a section's ground symbols, supplies, input and output ports that form a ladder off their row or column. `DRAW-40` weighs bends and detours by net class (`drawing.route.k_power`, `.k_ground`, `.k_signal`) and looks at any bent power or ground tie, its candidates keep the pin lead and the symbol's row, stay off other nets' wires, and leave `PWR_FLAG` stubs out of the tie; `DRAW-41` also counts a tee right at the pin tip.
- `DRAW-46` (a leader into a small part: 45 degree last segment, aimed at the body centre, tip clear of the outline along its line) and `DRAW-47` (run-on sentences in note boxes), with test schematics.
- `DRAW-42` (touching items too close in colour, CIEDE2000 on each theme) with `berc check --theme THEME.json`, repeatable: KiCad colour themes come in by path, a new need `colour-theme` reports the check as not run without one. A label in its wire's net class colour is left out, since KiCad draws it that way. `DrawnSymbol::colour` holds the library stroke colour.

### Changed

- The sidecars of the test schematics compare `DRAW` findings on the `draw-*` schematics only: the others were written for their circuit, not drawn for style.

## [0.4.13] - 2026-10-31

### Changed

- Drawing guide round 2 (Roman, 31 Oct 2026): the obvious rules shortened to the rule, a one-line reason and their check, in `docs/drawing-guide.md` and `docs/checks/draw.md`; the longer treatment kept for the rules that are not obvious. New rules: power net names `+<voltage>_<source or group>` (`NAME-13`); labels lying on their wire, left side left-justified and right side right-justified (KiCad 10 takes the side from justify) (`DRAW-28`); no pin on pin (`DRAW-29`); one power symbol per rail per part (`DRAW-30`); sections with a bold heading and a parameter box (`DRAW-31`); notes only in boxes of six kinds (TODO, Information, Caution, Debug, Critical, Layout) with exact colours on the VHRD theme, fonts and sizes (`DRAW-32`); a leader from a note to its part (`DRAW-37`, `DRAW-38`); KiCad tables instead of text lumps (`DRAW-33`); clearance margins as named settings `drawing.clearance.part`, `.power`, `.text` (`DRAW-34`); small parts to a rail or ground vertical (`DRAW-35`); power symbols and parts aligned (`DRAW-36`); simulation-only parts drawn apart. From the 30 Oct tidy feedback: label columns clustered by x (`DRAW-10`), fill and centring measured without the note boxes (`DRAW-16`, `DRAW-17`), the box-based overlap measure (`DRAW-9`), field angles relative to the symbol (`DRAW-8`). `DRAW-27` (hand-set colour) is off by default. Then: the `vhrd_power` symbols (`GND`, the KiCad 10 local `PWR_LOCAL`) and the rail colours by net class (Ground green, Power orange for `+*`); simulation-only parts as `vhrd_sim:AMMETER`, out of the BOM but kept on the board for the XML netlist; SPICE directives as text boxes grouped by meaning with their explanation beside them; group headings at 1.905 mm; a bus for a group of related nets going to three or more places, with the hint `DRAW-39` and its settings `drawing.bus_hint.places` and `.members`; routing as a weighed compromise, a cost per grid step, bend, crossing (more near a pin or junction), extra symbol and detour with named weights `drawing.route.*`, scored by `DRAW-40`, and one power symbol per pin group rather than per part (`DRAW-30`); a straight lead of one major grid step out of a pin and into a power symbol (`DRAW-41`, `drawing.pin_lead`); alignment as a near-miss rule, exact within a neighbourhood or clearly apart (`DRAW-36`, now a warning, `drawing.align.window` and `.snap`); touching items told apart by colour, a CIEDE2000 minimum on both themes, with our pin, ground and power colours (`DRAW-42`, `drawing.colour.pair_min`); sentences start with a capital letter (`DRAW-43`). Docs only.
- Drawing guide round 3 (Roman's review, 31 Oct 2026): ladders per section (one ground row, one supply row, inputs left, outputs right; `DRAW-36` compares them per section); sections apart with their headings in one row (`DRAW-44`, `DRAW-45`); power and ground ties weigh more than signals, so supplies go straight up and grounds straight down (`drawing.route.k_power`, `.k_ground`, `.k_signal`, `.min_saving`, `.step`, `DRAW-40`); the pin end decides `DRAW-41`, a tee is no bend; the ammeter symbol shows an arrow from + to -; the simulated fault cases in one generated table on the first page. The scope of `DRAW-7`, `-28`, `-29`, `-30`, `-32`, `-35`, `-43` as implemented, and their settings (`drawing.label.stub_extra`, `drawing.heading_size`, `drawing.group_heading_size`, `drawing.note.colour_slack`). Docs only.
- `DRAW-40` text: candidates keep the pin lead, end on the tie's own row, stay off other nets' wires, and `PWR_FLAG` stubs are not part of the tie; a rail wire with one symbol and a symbol per part are both fine, the routing cost picks (Roman's call).
- Drawing guide: the VHRD drawing sheet and its title block fields (comment 1 Drawn, 2 Checked, 4 Status, `VHRD_STAMP` passed by the render recipe) (Roman, 31 Oct 2026).
- Drawing guide round 4 (Roman's review, 31 Oct 2026): note boxes are lists and tables (`DRAW-47`), leaders end on a 45° diagonal aimed at a small part's centre and stop short of its outline (`DRAW-46`), IBM Plex Sans as the default font (a user setting in KiCad 10), mixed paper sizes, gold pins.
- Drawing guide: wires `rgb(90, 90, 90)` and `rgb(154, 154, 154)` apart from label text, `PWR_FLAG` from `vhrd_power` in the supply orange (Roman, 31 Oct 2026, `DRAW-42`).

## [0.4.12] - 2026-10-30

### Added

- `docs/drawing-guide.md`: a schematic drawing guide for sheets that read at a glance (sheet size and splitting, fill and centring, title block, sheet names, design-intent notes, the 1.27 mm grid and KiCad 10's Align commands, signal flow, connectors at the edge with their protection, symbol orientation, ground down and supplies up, decoupling at its pin, wires and junctions, hop-overs, labels and their scopes, sheet pins, buses, field placement, text size, overlaps, no-connect flags, DNP, colour, print and PDF), each rule with its reason, a bad/good sketch and its source (standard, house style or new), a KiCad 10 settings and shortcut table from its source, what other tools automate, and our house style measured over a private set of 157 production sheets; linked from the README and the review checklist. Check catalogue: a new category `DRAW` (drawing style, `DRAW-1..27`) with the detail page `docs/checks/draw.md`, listed in FEATURES.md as planned with `NET-26` (drawing geometry) and `TOOL-7` (the house style measurement as a `berc` command). Docs only.

## [0.4.11] - 2026-10-23

### Added

- Check catalogue: a new category `PAS` (passive selection, `PAS-1..41`) for a capacitor, resistor, inductor, bead or choke whose technology is wrong for where it sits (leakage on SET, REF and soft-start pins, class 2 ceramics in timing, signal and low-noise roles, piezo and singing parts, output capacitor technology vs compensation, flex cracking, sulfur, pulse and safety class, precision resistor drift and voltage coefficient, shunt technology, saturation type, core loss, shielding, SRF, beads and chokes), and `RATE-25..31` (ripple current, electrolytic life, resistor pulse energy, inductor RMS current, jumper current, film dV/dt, series balancing), with the detail page `docs/checks/pas.md`, the RATE sections and the review checklist. The data they need is planned as `NET-23` (passive roles), `NET-24` (passive technology), `CFG-6` (selection constraints in the part database), `CFG-7` (series table) and `RPT-26` (passive calculators). Intent (vibration, audio, automotive, precision, industrial, environment, ambient, life) is per board with no defaults: checks that need it report not run without it; accuracy budgets come from the parts: each passive may add a quarter (`passive_share`) of the error of the part it serves (250 nA of SET leakage on an LT3045), `PAS-40` flags a precision circuit mixing grades and `PAS-41` precision parts where none is needed; PAS-9 and PAS-20 rise to warnings on audio and automotive boards. `BOM-19` reports the odd one out within a circuit block (case size, mount type, series, dielectric, tolerance among siblings of one role), building on `BOM-3`'s board-wide mixes; `ANA-24` (weakest link along a signal, reference, power or clock chain wastes a precise part) and `ANA-25` (over-specified part the chain cannot use) judge whole chains with figures from part data combined by a class figure table, planned as `NET-25` (chains) and `CFG-8` (class figures); the block checks stay separate (≈). Docs only, from the passive-selection design note.

## [0.4.10] - 2026-10-23

### Added

- Stage 1 easy checks, batch checks-bus: `USB-1`, `USB-4`, `USB-9`, `USB-10`, `USB-11`, `USB-16`, `CONN-2`, `CONN-4`, `CONN-8`, `CONN-10`, `CONN-16`, `CONN-17`, `HS-4`, `HS-7`, `HS-9`, `HS-15`, `MEM-1`, `MEM-3`, `MEM-8`, `MEM-10`, `ETH-3`, `SER-14`, each with its test schematic and sidecar; `erc_analysis::connector` (shell and mounting pins, Type-C contacts by number or name, plugs) and `erc_analysis::interface` (USB data, MDIO, CAN, SD/eMMC, DDR, MIPI and multi-gigabit lane names; `usb_data` and `can_line` read through `names::interface_signals`, which now takes `OTG` as a USB word and `UD+`/`UD-`).
- Stage 1 easy checks, batch checks-bom: BOM-1, BOM-3, BOM-4, BOM-8, BOM-11, BOM-14, BOM-17, LIB-14, LIB-15, MUX-1, LOGIC-3, LOGIC-7, LOGIC-8 with a test schematic each; erc_analysis::package and names::signals (polarity by word sense, interface signals); LOGIC-8 page example corrected.

## [0.4.9] - 2026-10-22

### Added

- `just test --changed` runs only the workspace crates changed against main and their dependents (nextest's `rdeps()`); plain `just test` runs everything, as `just test-all` does.

## [0.4.8] - 2026-10-22

### Changed

- `just install` builds with `--profile deploy` and installs `better_erc` and `berc` through our release tool when it is there (each once on PATH, kept releases locally), else through `cargo install`; `better_erc --version` shows version, git sha and commit time (as berc does); `just deploy` checks both `--version`s against main's short sha. The README names `cargo install` for users without that tool.

### Fixed

- `just test` no longer rewrites the committed pictures in `test_schematics/generated_pictures`: the tests export only the netlists, and `just netlists` is the one way the pictures change (which are current is kept in the gitignored `generated_pictures_cache`). It had left about 30 modified tracked SVGs in the tree after every test run.

## [0.4.7] - 2026-10-22

### Added

- Power checks with test schematics (`<id>_<slug>`): RAIL-1, RAIL-5, RAIL-18, RAIL-19; PIN-3, PIN-4, PIN-6; DRV-5,
  DRV-6, DRV-7, DRV-9, DRV-12, DRV-14, DRV-17; GND-3, GND-7; SEQ-5; LEAK-1; LVL-4, LVL-7, LVL-9; I2C-4, I2C-18 (folds
  I2C-1 in on segments behind a translator; isolators now split a bus in `erc_analysis::i2c`). `erc_analysis::feed`:
  the source pins of each net and the two-terminal links that carry power (`Ctx::feeds`). A finding can carry its own
  severity (`FindingBuilder::severity`, PIN-3's fused outputs one level lower); the project's override still wins.
  Part classes `Motor` and `Solenoid` (inductive loads, DRV-14).

### Fixed

- The power name grammar reads KiCad's auto net names (`Net-(U1-GND)`, `unconnected-(J3-Shield-Pad6)`) as nothing,
  a ground pin alone on an unconnected net is neither a ground nor a rail, and shield, chassis or earth pins no
  longer make their net a ground (`erc_analysis::power::is_auto_name`, `ground_pin_word`).

## [0.4.6] - 2026-10-22

### Added

- Stage 1 easy checks, batch checks-io: `IO-1`, `IO-6`, `IO-7`, `IO-8`, `IO-10`, `IO-11`, `IO-15`, `IO-16`, `CLK-3`, `CLK-5`, `CLK-6`, `CLK-7`, `CLK-14`, `CLK-15`, `CLK-18`, `ANA-3`, `ANA-4`, `ANA-10`, `ANA-11`, `ANA-12`, `ANA-16`, `ANA-21`, `ANA-22`, each with its test schematic and sidecar; `erc_analysis::{pull, analog}`, `FindingBuilder::pins`, `pin_name` reads `RUN`, `CHIP_PU`, `MCO`, `CLKOUT` and says which names are active low; the catalogue marks `IO-6` ≈ `IO-1` and `IO-10` ≈ `IO-6`.

## [0.4.5] - 2026-10-22

### Added

- Stage 1 easy checks, batch checks-bom: `BOM-1` (values a step apart), `BOM-3` (assembly mixes), `BOM-4` (resistor
  array candidates), `BOM-8` (LED turned round), `BOM-11` (one value in two packages), `BOM-14` (fitted part without an
  MPN), `BOM-17` (one MPN, different values), `LIB-14` (unit not placed), `LIB-15` (symbol without power pins), `MUX-1`
  (interface signal on the wrong pin), `LOGIC-3` (high fan-out), `LOGIC-7` (open drain without pull-up), `LOGIC-8`
  (polarity mismatch by name), each with its test schematic. `erc_analysis::names` grows `polarity` and `mismatch`
  (the marker with the sense of the function word) and the interface signals of pin and net names
  (`names/signals.rs`); `erc_analysis::package` (case size, SMD or THT, board side from the footprint);
  `ESeries::steps`. `BOM-5` takes `calc` as a placeholder word, as its page says. The berc and project tests that count
  findings on `i2c-1` run the I2C checks only, and the berc snapshot writes the planned count as `N`, so new checks do
  not move their numbers.
- `erc_analysis::names`: the name grammar beyond power, for the NAME, WIRE and SHORT checks: active-low markers with
  their style (`~{X}`, `~X`, `X_N`, `X_n`, `nX`/`NRST`, `X#`, `Xn`), differential halves (`_P`/`_N`, `+`/`-`,
  `DP`/`DM`, `_H`/`_L`, glued `XP`/`XN` where both exist) and `pair_halves`, numbered families (`IO4`), the GPIO a pin
  name gives (`PA5`, `GPIO17`, `P0.12`), UART TX/RX pins and net-name words, and `normalised` names (WIRE-2).
- 24 checks on the analyses as they are, each with a test schematic of failing and correct cases (`<id>_<slug>`):
  SHORT-1, SHORT-2, SHORT-3, SHORT-4, SHORT-5, SHORT-6, SHORT-7, SHORT-11, SHORT-12; WIRE-1, WIRE-2, WIRE-11, WIRE-13,
  WIRE-14; NAME-1, NAME-2, NAME-4, NAME-5, NAME-6, NAME-7, NAME-9, NAME-10, NAME-11, NAME-12.
  `erc_analysis::graph::series_paths` (two-terminal parts in series from a net) for the SHORT chains.
- `erc_model::Sheet::rev`: the title block revision, read by the schematic and netlist importers and kept in the
  canonical dump (`rev A`).

## [0.4.4] - 2026-10-22

### Added

- Stage 1 easy checks, batch checks-dec: `DEC-1`, `DEC-4`, `DEC-7`, `DEC-8`, `DEC-10`, `DEC-11`, `DEC-12`, `DBG-1`, `DBG-2`, `DBG-3`, `DBG-4`, `DBG-7`, `DBG-8`, `DBG-10`, `DBG-11`, `DBG-14`, `DBG-16`, `DBG-17`, `DBG-18`, `TEST-1`, `TEST-2`, `TEST-4`, `TEST-5`, each with its test schematic and sidecar; `erc_analysis::{pin_name, access, supply}` (`NET-22`), `PartClass::is_programmable`.

### Changed

- Test schematic sidecars: findings compared exactly, `[[not_run]]` entries are hand-written assertions compared as a subset; `BLESS=1` writes findings only and keeps the not-run blocks; the generated not-run blocks are gone from all sidecars (`DIAG-6`).

## [0.4.3] - 2026-10-22

### Added

- The schematic PDF importer (`IMP-16`, stage 1), cargo feature `pdf` of `erc_import`: an Altium Smart PDF read into
  a board (tool `altium-pdf`) from its bookmarks: sheets, parts with their units, pins, nets, net labels and ports,
  nets joined across sheets by power names and same-named ports (Altium names nets per sheet), part properties
  from the popup scripts bound to their part by the hidden search words inside the link. Parsed with `hayro-syntax`
  (pure Rust, no unsafe of its own); every walk bounded (file size, bookmarks, nesting, links, action chains,
  script length, content operations, form drawings, words), scripts read as text, never run; owner-password files
  read, a user password is an error. `berc check board.pdf` works; tests on a synthetic Smart PDF from our own
  generator, its encrypted copies, hostile files, and the internal set of Altium PDFs when present (the Xyloni
  against its hand-made netlist).
- `erc_model`: `Provenance::Pdf { file, confidence }` with `PdfConfidence::{Bookmarks, Position, Geometry}`,
  `Rank::RECONSTRUCTED` (750), `Provenance::is_estimate`, `SourceInfo::connectivity`.
- `berc diff --nets A B` (`RPT-16`, first piece): the nets of two designs as pin sets, renamed, removed and added
  nets and net class changes, text or JSON, exit 1 on a difference; `better_erc::diff::nets`. The differential
  between a Smart PDF and an Altium export of the same board.

### Changed

- Checks read the part's value (`Part::value`), not the `Value` field by name: Altium's `Comment` counts where there is
  no `Value` (BOM-5, BOM-6, BOM-9, BOM-10, the I2C pull-up checks, the `Values` need), and findings name the field
  (`Value (from Comment)`); `erc_analysis::value::value_text` replaces `value_field`, `PartValue::field`.
- A finding resting on a fact reconstructed from a PDF by position or geometry, or on a board whose nets were traced
  from a drawing, is an estimate. Not-run hints for a PDF board say what a PDF cannot give (layout, wires,
  no-connect flags), and its findings point at the PDF and the page (`symbol U1 on page 3 (MCU.SchDoc)`).
- `Label::at` is optional (a PDF does not always say where a label is); the dump leaves `at` out then.
- The workspace needs Rust 1.92 (`rust-version`), for `hayro-syntax`.

## [0.4.2] - 2026-10-22

### Added

- `erc_model::class::PartClass`: the part class enum of architecture §1.3 with kinds (diode kind, BJT polarity, MOSFET
  channel, connector, switch, regulator, translator, memory, transceiver, logic family, fuse), `value_unit` for the
  unit a Value field is read in. `erc_analysis::class` resolves it as a fact: a `ClassSource` hook (the part database
  by MPN, later), the library symbol, description and keywords, net-tie pad groups, the designator prefix as Assumed;
  `PartClasses::coverage` for `Need::PartClasses` (`NET-19`).
- `erc_model::value`: `Value::read` reads the whole Value field (`Reading`: tolerance, ratings like `50V` and `1/4W`,
  dielectric, a ferrite's `@100MHz`, signs, RKM with a unit), `ValueWarning::SeparateUnit` (`4k7 Ohm`); `ESeries` E3 to
  E192 with membership and the nearest value. `erc_analysis::value::Values`: each fitted part's value in the unit of
  its class, per variant (`IMP-6`).
- `erc_model::View` (`Board::view(variant)`): a board as a variant mounts it, DNP and field overrides applied;
  `Need::Variant` (`NET-20`).
- `erc_analysis::power` rewritten on the power name grammar (§1.4): voltage tokens (`+3V3`, `VDD_1V8`, `+5V`, `3.3V`,
  `3P3V`, `-5V`, `VCC3V3`), power words with rail kinds (USB, battery, analog, reference), ground words with ground
  kinds, signal words that are no rail (`SCL_3V3`, `PWR_EN`); power symbols and power pins as signals, a `NetSource`
  hook for the project file; the voltage a `Facts<f64>` derived from the name; `GroundGroup`s joined at DC through net
  ties, jumpers, 0 Ω, ferrites and inductors, and `GroundLink`s between them (`NET-5` first layer, `NET-6`).
- `erc_analysis::drawing::Drawing`: labels with their scope, sheet pins matched to hierarchical labels, no-connect
  flags, dangling labels, flags on connected pins; `Need::Drawing` (`NET-21`). `LabelKind::LocalPower` for KiCad 10
  local power symbols (`erc_kicad` reads them as such).
- `erc_analysis::i2c`: `I2cBuses::on(view, classes)` on the fitted parts; ferrite, net-tie and jumper ties; discrete
  translators (a BSS138 pair) with their gate nets; `pull_ups(segment)`, `direct_segment`, `segments_of_logical_bus`
  (`NET-4`). Test schematic `i2c-18_discrete_translator`.
- `erc_checks::Ctx` builds the analyses lazily (`view`, `classes`, `values`, `power`, `i2c`, `drawing`), and
  availability comes from them (`Availability::of`, `Ctx::availability`): part class coverage, rails, rail voltages
  with their coverage, variant, drawing (`DIAG-2`).
- `test_schematics/analyses`: power, parts (with a KiCad 10 variant) and drawing schematics with `.expect.toml`
  sidecars, checked by `erc_analysis/tests/analyses.rs` (`BLESS=1` rewrites them).

### Changed

- `BOM-10` checks capacitors and inductors too and reports a value in no E series with the nearest E24 and E96
  values; the unit comes from the part class, not the letter.
- The power name parser reads the whole grammar by default: `+5V` and `+3.3V` now have a voltage; the old `+xVy`-only
  reading is `PowerOptions::strict`. `5V0_NEG` reads −5 V (the sign from its marker, said in the fact's note), `VEE`
  is a negative rail and no longer a ground, `ISO_5V` is no ground.
- The I2C recogniser and the checks on it see only the parts the variant fits: a DNP pull-up no longer counts.

## [0.4.1] - 2026-10-22

### Added

- `erc_kicad`: footprints link to schematic parts by the footprint's schematic UUID path, KiCad's own link
  (`Footprint::schematic_path`, read from `(path ...)`, in the canonical dump as `path`), so a part re-annotated in
  the schematic but not yet on the board still finds its footprint; the designator is the fallback when there is no
  path or it names no symbol. Where they disagree the import gives a new `NoteKind::Conflict` note (`IMP-14`).
- `just test-full`: `just test` with every hw board in the differential (`BERC_HW_FULL=1`).

### Changed

- `just test` compares a sample of six hw boards in the differential test instead of all ~41 (30-50 s); the full set
  runs with `just test-full`, for CI and before landing.

## [0.4.0] - 2026-10-22

### Added

- `erc_analysis::power` (`NET-5`, first layer): rails (by a voltage in the name, `+V`/`VDD`/`VCC`, or a power pin on
  the net, each a fact with its rule and the voltage from the name), grounds and switching nodes, ported from
  `erc_core`'s `power.rs`; the I2C lines and switching nodes are not rails. `name_voltage` reads `+1V25` as 1.25 V
  (the old parser read 2.25 V). `erc_analysis::graph::input_pins`: the input pins on a net (the IO-1 stub's list).
- `berc diff --pnp A B` (`TOOL-1`): the PnP compare of the old `ecad_compare` tool on `erc_model::placecmp`, each side
  any design `berc check` opens (PnP files, a `.kicad_pcb`, a KiCad project with its board); `--origin page|aux` for a
  board compared with a PnP file (brought into the PnP frame: from the origin, y up, without footprints excluded
  from position files, as kicad-cli's `pcb export pos` writes it), `--tolerance MM`, `--rotation-tolerance DEG`,
  `--format text|json`; exit 0 same, 1 different, 2 error. The text names each side's footprint and test point
  count, as the old example did. `placecmp::{Origin, to_placement_frame, aux_origin, is_board_file}`; the diff types
  serialise.
- The library facade (`TOOL-6`), crate `better_erc` (lib): `model`, `kicad`, `import`, `analysis`, `checks`
  re-exported, the run entry (`Project`, `check_path`, `CheckOptions`, `CheckRun`, `Verdict`) and the report types at
  the top, and `better_erc::diff::pnp`. berc and the app depend on it alone.
- `Project::open_files(main, others)`: a design whose files were picked one by one from anywhere (an Altium EDIF and
  its WireList, the OrCAD trio), read as if beside each other; the app's import tab uses it.
- `erc_analysis::power::PowerOptions { strict }`: the old non-strict mode (`VDD_1V8` without the `+`) kept;
  `erc_analysis::graph::part_chains`, `PcbAssembly::find_part_chains` on the board model.
- The old readers' tests as acceptance tests on the new core: `ecad_file_format::kicad_netlist`'s asserts in
  `erc_kicad/tests/netlist.rs`, the passive value table in `erc_model::value`.

### Changed

- **Breaking:** the app crate is `better_erc_app` (directory `better_erc_app/`, binary still `better_erc`, so
  `just install`, `just run` and `just deploy` work as before); the crate name `better_erc` is the library facade.
- The app runs on the new core (`IMP-7`): the import tab opens any design through the importer registry (a file or
  a directory), keeps the KiCad schematic, Altium (EDIF + WireList) and OrCAD (three files) pickers, which now all
  load, and shows each design's report: findings with their check id, severity, subjects, facts and the catalogue's
  explanation, the checks that did not run and why, the import notes; a design loaded again replaces itself, Close
  removes it. The context holds the designs with their check run and per board the part classes, I2C buses and power
  structure. Nets (`VIEW-1`: rails with their voltage, grounds, switching nodes, nets with node count), I2C
  (`VIEW-2`: the I2C findings and the buses with designators and net names), Style (`VIEW-3`: BOM and LIB findings)
  and Inputs (`VIEW-4`) read them.
- The KiCad netlist fixtures move from `ecad_file_format/test_input` to `erc_kicad/tests/fixtures/netlist`;
  `just netlist-fixtures` exports them there.

### Changed

- `VIEW-19`: Footprint review tab, S (and Z a third time) asks the ruler's length in a box in the middle of the
  canvas, the number typed shown large in mm; Enter sets the scale, Esc leaves it. Before, the keyboard went to the
  side panel's length field.

### Removed

- **Breaking:** the old crates `ecad_file_format` (netlist model and readers), `erc_core` (`PcbAssembly`, its checks
  and analyses) and `ecad_compare` (PnP compare), and with them `lexpr` and `serde-lexpr` (`NET-16`). Everything they
  did runs on the new core: the readers in `erc_kicad` and `erc_import`, values in `erc_model::value`, the
  analyses in `erc_analysis` (classes, graph, I2C, power), the eleven checks in `erc_checks`, the PnP compare as
  `berc diff --pnp`; their tests are acceptance tests there.

### Fixed

- The KiCad netlist importer keeps KiCad 10's unit names (`A`, `B`) and unit count on the symbol, as the old reader
  did (`IMP-12`).

## [0.3.9] - 2026-10-22

### Added

- `berc check PROJECT` for CI (`RPT-20`): a directory with one design (its `.kicad_pro`) or any design file the
  importer registry reads, `--variant`, `--only CAT,ID`, `--format text|json|sarif|md`, `--baseline report.json`,
  `--fail-on error|warning|suggestion`, `--require NEED`; exit codes 0 clean, 1 findings at or above `--fail-on`,
  2 usage or import error, 3 a required need unavailable. `--state` is reserved for power states and refused.
- `erc_checks::project`, the library entry under it, for the MCP server and the GUI too: `Project::open` through the
  importer registry, `Project::check` / `check_path` with `CheckOptions { run, require, fail_on, baseline }`,
  `CheckRun` with the report and the `Verdict` (exit code as data), typed `CheckError`s, `read_baseline`, progress
  stages. Cargo feature `import`, on by default.
- Baselines (`RPT-23`): `Report::compare` marks each finding new or unchanged by board and fingerprint and keeps the
  fixed ones; with a baseline only new findings fail.
- SARIF 2.1.0 (`RPT-24`) for GitHub code scanning, with rule ids from the catalogue and findings at the schematic
  file and the line of their symbol; Markdown (`RPT-25`) for a PR comment; coloured text on a terminal.
- `berc explain ID` and `berc checks list [--tier] [--status] [--project]` (`TOOL-5`), `erc_checks::list_for`,
  `Explanation::to_text` / `to_markdown`; completion of check ids, categories, needs and variants of the design in
  the current directory; a spinner for runs over two seconds.
- `erc_kicad::sexpr::Lines`: the line of any parsed expression; `UnitPlacement::line` (in the canonical dump as
  `line N`) and `Location::line` carry a symbol's line into findings. `Need::slug` / `from_slug`.
- README "Checks in CI" with a GitHub Actions example uploading SARIF to code scanning; `just check` runs berc check on
  the test schematics against `test_schematics/berc-baseline.json`, `just check-baseline` rewrites it;
  docs/engine.md describes the library entry, the formats and the command line.

## [0.3.8] - 2026-10-22

### Added

- `erc_import`, every importer behind one registry (`IMP-15`): the importer for a file is picked by its name and
  contents, KiCad's through `erc_kicad::importers()`; one cargo feature per importer so a KiCad-only or wasm build
  leaves the rest out.
- Altium (EDIF plus WireList), OrCAD (pstxnet, pstxprt, pstchip) and pick-and-place CSV files read into the board
  model (`IMP-3`, `IMP-4`, `IMP-5`), their pest grammars moved along and taught to read past unknown elements: every
  attribute a fact with the file it came from, notes for everything not kept, no panic on broken input (a test feeds
  truncated and garbled fixtures to every importer), a golden dump per fixture, and the old readers' test assertions
  as acceptance tests.
- `erc_model::placecmp`, the PnP compare on two layouts (two PnP files, two boards, or a board and its PnP file),
  rotations modulo 360 and tolerances, for `berc diff --pnp` (`TOOL-1`).
- `erc_model`: `Provenance::Placement` for facts from a PnP file, `SymbolDef::unit_names` (OrCAD section letters).

### Changed

- The Altium, OrCAD and PnP fixtures moved from `ecad_file_format/test_input` to `erc_import/tests/fixtures`; the old
  readers' tests read them there.

## [0.3.7] - 2026-10-22

### Added

- `erc_checks`, the check engine (`DIAG-1` to `DIAG-6`, `RPT-22`): one file per catalogue check with its id,
  severity, tier, scope, declared data needs, overlaps and thresholds, registered statically; per board the engine
  works out which needs are available, runs the checks that can run in parallel over one context whose analyses are
  built on first use, and reports the others as not run with why and what would enable them. One `Finding` for every
  check (severity with a project override, confidence, subjects with names, variant, facts with their sources,
  fingerprint, detail page link, fix, locations); a finding of an overlapping check folds the other in (`also
  BOM-5`). The `Report` (summary, every catalogue check ran / not run / planned, findings worst first by category,
  candidates, notes, timing) as JSON with a versioned schema and as text. The catalogue and its detail pages are
  embedded: `list()`, `explain(id)` and the not-run hints work offline. Described in [docs/engine.md](docs/engine.md).
- The eleven shipped checks run on the new core: `I2C-1`, `I2C-2`, `I2C-3`, `I2C-7`, `I2C-9`, `I2C-10`, `BOM-5`,
  `BOM-6`, `BOM-9`, `BOM-10`, `LIB-3`, each with its unit test. `I2C-9` and `I2C-10` are findings of their own now,
  `BOM-5` is an error as the catalogue says, `I2C-9`'s window is inclusive as the catalogue says (2.2 kΩ and 10 kΩ
  no longer fire; `erc_core` reported the ends with the lumped `I2C-1`), `BOM-6` reports a placeholder on a fitted part (in the variant checked)
  apart from `BOM-5`, and `BOM-5` also catches `TBD` and `TODO`.
- `erc_analysis` (`NET-4`, `NET-19`): part classes from the library symbol, the description, or the designator prefix
  as an assumption (a finding resting on it is an estimate); values read in their class's unit per variant; graph
  queries; the I2C bus recogniser ported from `erc_core` (buses, pull-up pairs and redundant pull-ups, ties, discrete
  and IC translators, segments; parts in no known role become notes).
- Test schematics as the spec: every `test_schematics/sources/<id>_<slug>.kicad_sch` has an `.expect.toml` of the
  findings and not-run checks the engine must report on it, and one test compares them (`BLESS=1` rewrites them); a
  test holds every registered check to its catalogue line.
- `erc_model`: `Board::fitted` and `Board::field` per variant, `Board::is_component`; the value parser warns about more
  than one space before the multiplier, a small `r` and an `R` after a multiplier.

### Changed

- The test schematics are named after their check (`i2c_segments` is `i2c-7_segments`, `style_values` is
  `bom-10_values`); the MOSFETs moved to `lib-3_mosfet_pin_names`, and `bom-10_values` gained a placeholder on a DNP
  part and a `TBD`.
- The catalogue lines and detail pages of the ported checks say what the code checks now, without "runs today:
  `style.rs`".

## [0.3.6] - 2026-10-22

### Added

- `erc_model`, the board model of the rewrite (`NET-17`): an arena-indexed design of boards, sheets, symbols, parts,
  pin instances, nets, labels, variants, net classes, project settings and the board layout; every attribute a fact
  with its source, rank, time and note, resolved to the highest rank (newest on a tie) with disagreements listed; typed
  values; one `Importer` trait for every tool; a canonical text dump, one sorted fact per line, that reads back to the
  same design and is the golden format of the importer tests. Described in [docs/model.md](docs/model.md).
- `erc_kicad`, KiCad 8, 9 and 10 read by better_erc itself (`IMP-13`, `IMP-14`, `NET-18`): the schematic hierarchy
  with KiCad's connectivity rules (label scopes, power symbols, sheet pins, vector and group buses, bus entries and
  aliases, hidden power pins, junctions, the version differences between KiCad 8 and 9/10), the project file (net
  classes, text variables, variants, ERC and DRC settings, design rules), each net's effective net class, and the board
  (footprints, pads, tracks, vias, zones, graphics, stackup). The KiCad netlist importer moved onto the same trait.
- The differential test: our nets against kicad-cli's netlist as pin sets, names and net classes, on fixtures with
  KiCad 8, 9 and 10 netlists, the test schematics, and every board under `~/git/hw` when it is there (41 boards equal
  KiCad 10's netlists, 94 comparisons over KiCad 8, 9 and 10). `just kicad-fixtures` exports the fixture netlists.

### Changed

- The s-expression reader moved from `berc` to `erc_kicad::sexpr`; `berc` re-exports it and `berc textconv` is
  unchanged (`NET-16`).

## [0.3.5] - 2026-10-22

### Added

- Detail pages for every check, one file per category under `docs/checks/` (34 pages, 513 sections): what the check
  catches, why the board fails without it, a concrete example (our own boards' misses where there is one), how
  better_erc can detect it and what data that needs, the fix, and references that were fetched and checked before
  being cited (vendor app notes and datasheets, specs, Wikipedia, books by edition and chapter). Each section has an
  `Impl.` line: how hard the check is to implement in pure Rust with no LLM (`easy` 139 · `mid` 291 · `hard` 64 ·
  `judgement` 19) and the FEATURES.md ids it depends on. The index `docs/checks.md` carries the tier and a 📖 link on
  every line, and explains the tiers in its header; FEATURES.md has the per-category tier table.

### Changed

- Catalogue fixes found while writing the pages: `ETH-1` had current- and voltage-mode PHY centre-tap termination the
  wrong way round; `BOM-12`'s example attributed B200A's 13k3-for-13k7 to B153A (whose R219/R222 were 13k3 for 3k3);
  `NAME-9`'s example pins (IO4 = GPIO17, IO5 = GPIO16); `I2C-12` SMBus 3.0 thresholds are 0.8 / 1.35 V; `I2C-14`
  names the push-pull TXB class as the problem on I2C, not the open-drain TXS class.

## [0.3.4] - 2026-10-21

### Changed

- Check catalogue (`docs/checks.md`) grown from 166 to 513 checks. New categories: `BAT` (batteries and chargers),
  `WIRE` (connectivity hygiene: one-pin nets, near-duplicate net names, no-connect misuse), `BOOT` (boot modes and
  strapping pins), `ETH` (Ethernet PHY, magnetics, RGMII/RMII), `ERR` (known errata by MPN). Every category got
  new entries; the real misses of the B125, B135, B153, B156 and B200 assemblies and bring-ups became checks
  (`RAIL-4` fixed regulator voltage vs net, `RAIL-5` rail without a source, `MUX-4` GPIO on an NRST pin, `IO-12`
  output enables without a pull, `IO-13` peripheral reset held by a resetting controller, `BOM-12` value vs MPN,
  `LIB-6` footprint vs MPN package, `LVL-11` TXS-class shifters on push-pull lines, `SEQ-6` hard turn-on into a
  capacitive load, `USB-7`/`USB-8` CC resistor sources and roles, among others), and the I2C-in-the-wild
  notes became `I2C-11`..`I2C-22`. New rule "one check, one concern": 22 lumped entries were split, the old ID
  keeping its first concern and the rest taking new numbers (`PIN-1` → `PIN-4`..`PIN-6`, `I2C-1` → `I2C-9`,
  `I2C-10`, `SER-1` → `SER-7`, `SER-8`, ...), each marked with a `split` comment. The review checklist follows
  the catalogue. FEATURES.md: `I2C-1`'s code also covers `I2C-9` and `I2C-10` as one finding for now.

## [0.3.3] - 2026-10-21

### Changed

- Crate docs for `ecad_file_format`, `erc_core` and `ecad_compare`; an empty test module in erc_core removed.
- `Cargo.lock` is committed (it was gitignored), so `cargo install --locked` and `just install` build every
  clone with the same dependency versions.
- Building erc_core no longer runs kicad-cli: its build script exported the test schematics on every build, also
  when building or installing the app, and wrote into the source tree. The tests now export them on their first
  `get_netlist_path` (once per test process), `generate_netlists` is a dev-dependency only, and `just netlists`
  exports on its own. A missing netlist names the likely cause (no schematic, no kicad-cli).
- README: what better_erc is and who it is for, what works today (berc textconv, the importers, the checks that
  run, the app and how far its import goes), install, a quick start for diffs and for running the checks from
  Rust, development recipes, and links to the check catalogue, the review checklist and FEATURES.md. The Rust
  example is also a doc test on `PcbAssembly`, so it keeps compiling. FEATURES.md `IMP-7` now says only Altium
  loads in the app's import tab.
- `berc` (TOOL-4): help is coloured and `berc: error: ...` has a red `error:`, both only on a terminal and never
  with `NO_COLOR` (`textconv` output stays plain); `--help` says how to turn on shell completion. Tests for the
  binary itself: the `--version` line, a plain error on a pipe, completion scripts for bash, zsh and fish.
- `justfile`: new `fmt`, `run` (the app), `netlists` (export the test schematics with kicad-cli, through a new
  `generate_netlists` binary; before, only erc_core's build script could) and `netlist-fixtures`; each recipe has
  a one-line description for `just --list`.
- The `general` and `style` tests (IMP-12) read new test schematics instead of OrCAD and Altium project files
  that only existed on one PC, and now assert what they find instead of printing it: `style_values` has one
  part per style finding (BOM-5/6, BOM-9, BOM-10 with each value warning, LIB-3, a fine value and a DNP), and
  `io_inputs` an input driven and one floating for the IO-1 stub. A second test takes the in-repo OrCAD fixture
  through `PcbAssembly`, the path the old `general` test covered. `input_without_driving_source` gets its
  input list from a new `inputs()`, which the test reads; it still prints the same list.

### Fixed

- KiCad 9 and 10 netlists load again (IMP-12): the KiCad netlist reader knows the elements those versions add,
  per KiCad's own netlist exporter source: net classes (kept as the net's `class` property), component classes,
  and in KiCad 10 the unit list (now the component's `sections`), per-component variants, jumper pins and
  groups. Fixtures exported by kicad-cli 8.0.9, 9.0.9 and 10.0.7 from one small schematic
  (`ecad_file_format/test_input/kicad_versions`, `just netlist-fixtures` regenerates them from KiCad's container
  images) must read to the same netlist; a load error now names the file. The five I2C tests are green again.
- `BOM-9` (part without a value) fires on KiCad designs: KiCad writes an empty value as `~` in the netlist, which
  the reader took for a value, so the check never saw an empty one; `~` now reads as empty. Found by the new
  `style_values` test schematic.
- The `find_net_chains` doc example compiles (a missing `;`); it was never run while the erc_core tests failed
  first.
- Regenerating the test schematic pictures (`test_schematics/generated_pictures`, on every fresh clone or
  schematic change) no longer shows all of them as changed: `generate_netlists` drops the export time KiCad
  writes into each SVG's `<title>`.

## [0.3.2] - 2026-10-21

### Added

- `docs/checks.md`: the check catalogue (IDs like `PIN-1`, `I2C-3`, severity, overlaps), moved here from our
  internal notes so it ships with the public code. `docs/review-checklist.md`: the condensed schematic review
  checklist, each ID linking back to its catalogue entry. README, FEATURES.md and CLAUDE.md now point here
  instead of our internal notes; wording scrubbed of internal project names, board names and tools (the IO-9 and
  LOGIC-5 examples, an I2C-8 bench note, and two other asides) for a public repo.

## [0.3.1] - 2026-10-17

### Added

- `berc textconv` reads `.kicad_pro` too: every project setting as a dotted path
  (`board.design_settings.defaults.zones.min_clearance = 0.3`), net classes and other named lists keyed by name,
  and settings that match KiCad 10.0.6's own default (`pro_defaults`, a table transcribed from the KiCad
  source - zones, the DRC/ERC rule severities, the ERC pin conflict matrix, `reuse_designators`) or JSON's
  "nothing here" value (`false`/`0`/`""`/`[]`/`{}`) folded into one summary line per section, so opening a
  project in a newer KiCad and saving it - which can add dozens of such settings - shows as a small diff instead
  of a wall of JSON (cannify_micro.kicad_pro, real fixture: 72 raw diff lines to 13). `--all` shows the folded
  settings too (TOOL-3).

### Changed

- `justfile install` also installs `berc` (`cargo install --locked --path berc`), and `deploy` checks
  `berc --version` instead of only `better_erc` being on PATH.
- `berc --version` labels its timestamp `commit` instead of `built`, and that timestamp is now the commit's own
  time instead of the build clock's, the way another of our CLIs already does it in its `build.rs` - so it doesn't
  change between rebuilds of the same commit, and a worktree build no longer looks stale to cargo.

### Fixed

- `just lint` is green again: mechanical clippy fixes across `ecad_file_format`, `erc_core`, `ecad_compare`,
  `better_erc` and `generate_netlists` (needless borrows, `.iter().count()` to `.len()`, `for_kv_map`,
  collapsible ifs, an unnecessary `let` binding, `len_without_is_empty`, a missing `Default` impl, a couple of
  unused imports), no behaviour change. Pre-existing drift, unrelated to the `.kicad_pro` work above.

## [0.3.0] - 2026-10-09

### Changed

- `justfile` gets a `default` recipe (`@just --list`), so a bare `just` only lists the recipes instead of
  running tests, builds or deploying.

## [0.2.1] - 2026-10-07

### Changed

- `lexpr` and `serde-lexpr` come from upstream git, pinned to `09a61d3`, instead of a `../../lexpr-rs` checkout,
  so the workspace builds on any PC. Dropping them completely is planned (NET-16).

### Fixed

- The workspace builds on Linux: `generate_netlists` uses `kicad-cli` from PATH outside macOS, and creates the
  gitignored `generated_netlists` folder on a fresh clone. Seven `erc_core` tests still fail there (IMP-12).

## [0.2.0] - 2026-10-07

### Added

- `berc` crate with `berc textconv FILE`: a `.kicad_sch` or `.kicad_pcb` as sorted, line-oriented text for git's
  textconv, so `git diff` and `git log -p` show what a person changed (symbols, fields, variant overrides, text,
  labels, footprints and their pad nets, design rules), not UUIDs, coordinates and reordering; `--with-geometry`
  adds positions; `berc --version` shows the git SHA and build time. Setup in README "Readable git diffs" (TOOL-3)
- `FEATURES.md` tracker with stable area IDs (IMP, NET, DIAG, CFG, RPT, VIEW, APP, TOOL, GEN, BIZ); `CLAUDE.md`
  points at it and at the check catalogue
- This changelog
- `justfile` with the standard recipes `test`, `lint`, `install` and `deploy` (pull --ff-only, refuses on local
  changes, install); documented in CLAUDE.md

## [0.1.0] - 2025-04-24

### April 2025

- egui app (`better_erc` crate) with tiles and tabs, shown and hidden from the side panel, new icon (APP-1)
- Netlist view and first tabs: nets, style, inputs, import (VIEW-1, VIEW-3, VIEW-4, IMP-7, in progress)
- `pnp_compare` tool: compare two PnP files (TOOL-1)

### March 2025

- Common `Netlist` model with typed `Designator`, `NetName`, `PinName`, `PinId`; nodes kept in a `HashSet`
  (NET-1)
- Graph queries: `find_chains`, `find_connected_parts`, `is_connected`, `are_parts_connected`, `connected_net`
  (NET-2)
- `PcbAssembly` collects the analysis results, with `find_part_chains` (NET-3)
- Power structure from net names (`derive_power_structure`, NET-5, in progress)
- OrCAD netlist: nets, components and library parts (IMP-3)
- Altium netlist: EDIF parser combined with the WireList for pin types (IMP-4)
- KiCad: pin types and library parts parsed (IMP-1)
- KiCad schematics turned into netlists through kicad-cli, cached per file and regenerated when they change
  (IMP-2)
- Passive value parser: Ω with milli and micro, k, M, printed with human-repr (IMP-6)
- I2C bus finder: direct and common segments, connected parts, voltage translators (NET-4)
- I2C checks: missing pull-ups, non-standard or unequal SCL/SDA pull-ups, redundant pull-ups, pull-ups through
  series resistors to nowhere, series tie resistor too high (I2C-1, I2C-2, I2C-3, I2C-7)
- Style checks: "calculate later" values, parts without a value, values that don't parse or aren't standard,
  MOSFETs without named pins (BOM-5, BOM-6, BOM-9, BOM-10, LIB-3)
- Thresholds moved from hardcoded values into `erc_core/src/config.rs` (CFG-1)
- PnP file comparator in `ecad_compare`
- Test schematics re-annotated with page X100 designators; SVG pictures generated for them

### February 2025

- pest grammars for the OrCAD netlist, part list and library parts, and for the Altium WireList
- KiCad netlist export format and reader (IMP-1)
- PnP CSV reader for KiCad, Altium and Allegro (IMP-5)
