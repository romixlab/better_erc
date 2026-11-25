//! better_erc part data: the parts snapshot a board commits beside its project (`parts.lock.toml`, [`lock`]), the
//! part sources a refresh asks ([`source`]), and what of a design the snapshot covers ([`board`]).
//!
//! The checks read only the lock, so a run is offline and the same on every PC; `berc parts refresh`
//! ([`refresh()`]) writes it from a source, and its diff shows a changed fact or a retracted review. The lock and
//! the checks need no source: the lab's part store is one behind its own cargo feature, [`source::FileSource`]
//! reads hand-filled data.

pub mod board;
pub mod keys;
pub mod lock;
pub mod mpn;
pub mod refresh;
pub mod source;

pub use board::BoardMpns;
pub use lock::{LockError, PartsLock};
pub use refresh::{RefreshError, refresh};
pub use source::{FileSource, Source, SourceError};
