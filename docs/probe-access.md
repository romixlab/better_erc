# Probe access

`berc probe-access PROJECT` lists every net of a board with where a probe reaches it, per board side: the pads on it a hand probe can touch, and its vias. It answers "can I put a probe on this net from the top, from the bottom?" for hand probing during development, so a tool that makes nets reachable (`hwb pcb tent-vias` in the hw repo opens vias) only has to handle the rest.

It counts more than the `DBG` and `TEST` checks do: those ask for a test point or a connector, a hand probe also touches the pads of ordinary parts.

## What counts as access

- **Test points and connectors**, fitted or not, by their part class (`erc_analysis::access::is_access`): a DNP header's footprint is still there to touch. The class comes from the schematic.
- **Not every connector.** One of a kind that is shielded or fine pitch by nature never counts: USB (A, B, micro, C), HDMI, RJ45, card sockets, FPC/FFC, coax, as the part class names the kind. Any other connector (pin headers, terminal blocks, barrel jacks, a connector of no known kind such as a board-to-board one) counts only when its footprint's pads are at least `--connector-pitch-min` apart, default 1.27 mm: the nearest two numbered pads, centre to centre. A pad that does not count is still listed, with why.
- **Pads KiCad marks as test points** (pad property `pad_prop_testpoint`), for a footprint placed on the board only. Without a schematic these are the only access.
- **Two-terminal passives** (resistor, capacitor, inductor, ferrite, diode, LED, fuse, two pad numbers) of `--passive-size-min` (`passive_size_min`, default 0402, metric 1005) and larger: 0201 and 01005 do not count. The size is the case code in the footprint name (`R_0402_1005Metric`). A footprint without one (`D_SOD-523`, a power inductor) counts when its pad centres are at least 0.8 of the minimum size's body length apart, 0.8 mm for 0402.
- **Any other part** (an IC, a transistor, a crystal, a switch, a solder jumper) by pitch: its pads count when the nearest two pads of different numbers are at least `--part-pitch-min` (`part_pitch_mm_min`, default 0.65 mm) apart: SOT, SOIC, TSSOP at 0.65 mm count, 0.5 mm does not. A footprint with one pad number has no pitch and counts.
- **Never a pad under the part's body**, whatever the pitch:
  - every pad of a footprint whose library or name has a bottom-terminated package word: `QFN` (`VQFN`, `UFQFPN`), `DFN`, `SON` (`WSON`, `X2SON`), `LGA`, `BGA`, `CSP` (`WLCSP`);
  - in any other footprint, a surface-mount pad enclosed on all four sides by pads of other numbers (an exposed thermal pad, an inner ball), and its thermal vias.
- A pad is **on a side** when it has copper and a mask opening there: an SMD pad on its footprint's side, a through-hole pad (`*.Cu`, `*.Mask`) on both.
- **Behind one series part**: a pad that counts one series resistor, ferrite, net tie or jumper away (`series_neighbours`; a pull to a rail or ground does not count). The series part's own pad on the far net is not listed. The side's reach is `series` when only such access is there. A net tie is copper of one net, so access through one is marked `tied` and makes the reach `direct`.

Parts are counted fitted or not. A footprint without a part in the schematic (a board-only mounting hole) counts only through a pad marked as a test point.

## Output

The text form is a table: net, reach on top and bottom (`yes`, `series`, `-`), the number of vias, the pads with their sides. A net lists the pads that count first, on the net itself before behind a series part, test points before connectors before other parts' pads. The last lines count the nets without access per side.

`--format json` gives the typed result of `better_erc::probe::ProbeAccess`:

```json
{
  "schema": 1,
  "board": "board.kicad_pcb",
  "schematic": true,
  "nets": [
    {
      "net": "/SDA",
      "power": null,
      "pads": 4,
      "top": "series",
      "bottom": "none",
      "access": [
        { "reference": "TP3", "pad": "1", "kind": "test_point", "sides": ["top"], "series": "R12",
          "pitch_mm": null, "size": null, "not_counted": null },
        { "reference": "J4", "pad": "A8", "kind": "connector", "sides": ["top"], "series": null,
          "pitch_mm": 0.5, "size": null, "not_counted": { "why": "kind", "kind": "UsbC" } },
        { "reference": "U1", "pad": "21", "kind": "part_pad", "sides": ["top"], "series": null,
          "pitch_mm": null, "size": null, "not_counted": { "why": "under_body" } }
      ],
      "vias": [
        { "at_mm": [131.78, 87.95], "drill_mm": 0.3, "kind": "through", "sides": ["top", "bottom"] }
      ]
    }
  ]
}
```

- `power`: `rail` or `ground` from the power analysis, else `null`.
- `pads`: the pads on the net, of any footprint; a net of one pad has nothing to probe between.
- `top`, `bottom`: `direct`, `series` or `none`, from the access that counts.
- `access[].kind`: `test_point`, `connector` or `part_pad` (a pad of any other part).
- `access[].pitch_mm`: the pitch the pad was judged by: a connector's pad pitch, a part's lead pitch, the pad distance of a passive without a case code; `null` for a test point, a one-pad footprint, a pad under the body and a passive judged by its size.
- `access[].size`: the case code a passive was judged by (`"0402"`), else `null`.
- `access[].not_counted`: `null` when the pad counts, else why:

| `why` | Other fields | Meaning |
|---|---|---|
| `kind` | `kind`: `"UsbC"` | a kind of connector that never counts |
| `pitch` | `pitch_mm`, `min_mm` | finer than the minimum pitch |
| `size` | `size`, `min` | a passive below the minimum case size |
| `under_body` | none | a pad under the part's body |

- `vias[].sides`: the outer copper layers the via reaches; a blind via from the top reaches only `top`.

`schema` changes when a field changes meaning or goes away. New fields, access kinds and reasons may appear without it, so a reader ignores fields it does not know and does not require a known `kind` or `why`.

## Library

```rust
let opts = better_erc::probe::ProbeOptions::default();
let p = better_erc::probe::probe_access(std::path::Path::new("hw/board"), &opts)?;
for n in p.nets.iter().filter(|n| n.top == better_erc::probe::Reach::None) {
    println!("{} has no access on top, {} vias", n.net, n.vias.len());
}
```

`probe::of(design, board, config, &opts)` does the same on a design already open.
