# erc_parts

Part data for better_erc: the parts snapshot (`parts.lock.toml`) a board commits beside its project, and the part sources `berc parts refresh` asks.

- `lock`: the snapshot's format. Per MPN of the board the part facts the checks use (key, value, source, rank, time), per (board footprint hash, MPN) pair the footprint reviews covering it. Written in one fixed order, so a refresh is a clean diff.
- `keys`: the fact keys a refresh asks for, with the checks that read each.
- `board`: the MPNs of a design's fitted parts, in the base design and every variant, and the footprint hash each is placed with.
- `source`: the `Source` trait and `FileSource`, which answers from a file in the lock's shape (hand-filled data).
- `refresh`: a new snapshot of a design from a source.

The checks read only the lock: no network, the same result on every PC.
