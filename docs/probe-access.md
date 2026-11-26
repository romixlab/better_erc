# Probe access

`berc probe-access PROJECT` lists every net of a board with where a probe reaches it, per board side: the test point and connector pads on it, and its vias. It answers "can I put a probe on this net from the top, from the bottom?" with the same notion of access the `DBG` and `TEST` checks use, so a tool that makes nets reachable (`hwb pcb tent-vias` in the hw repo opens vias for hand probing) and the checks never disagree.

## What counts as access

- **Test points and connectors**, fitted or not, by their part class (`erc_analysis::access::is_access`): a DNP header's footprint is still there to touch. The class comes from the schematic.
- **Pads KiCad marks as test points** (pad property `pad_prop_testpoint`), for a footprint placed on the board only. Without a schematic these are the only access.
- A pad is **on a side** when it has copper and a mask opening there: an SMD pad on its footprint's side, a through-hole pad (`*.Cu`, `*.Mask`) on both.
- **Behind one series part**: a test point or connector one series resistor, ferrite, net tie or jumper away (`series_neighbours`; a pull to a rail or ground does not count). The side's reach is `series` when only such access is there.

## Output

The text form is a table: net, reach on top and bottom (`yes`, `series`, `-`), the number of vias, the access pads with their sides. The last lines count the nets without access per side.

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
        { "reference": "TP3", "pad": "1", "kind": "test_point", "sides": ["top"], "series": "R12" }
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
- `top`, `bottom`: `direct`, `series` or `none`.
- `vias[].sides`: the outer copper layers the via reaches; a blind via from the top reaches only `top`.

`schema` changes when a field changes meaning or goes away; new fields may appear without it, so a reader ignores fields it does not know.

## Library

```rust
let p = better_erc::probe::probe_access(std::path::Path::new("hw/board"))?;
for n in p.nets.iter().filter(|n| n.top == better_erc::probe::Reach::None) {
    println!("{} has no access on top, {} vias", n.net, n.vias.len());
}
```

`probe::of(design, board, config)` does the same on a design already open.
