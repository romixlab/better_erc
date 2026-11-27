//! better_erc part data: the parts snapshot a board commits beside its project (`parts.lock.toml`, [`lock`]), the
//! part sources a refresh asks ([`source`]), and what of a design the snapshot covers ([`board`]).
//!
//! The checks read only the lock, so a run is offline and the same on every PC; `berc parts refresh`
//! ([`refresh()`]) writes it from a source, and its diff shows a changed fact or a retracted review. The lock and
//! the checks need no source: the mx3 part store is one behind the cargo feature `mx3` (`mx3::Mx3Source`),
//! [`source::FileSource`] reads hand-filled data.

pub mod board;
pub mod datasheet;
pub mod keys;
pub mod lock;
pub mod mpn;
#[cfg(feature = "mx3")]
pub mod mx3;
pub mod refresh;
pub mod source;

pub use board::BoardMpns;
pub use lock::{LockError, PartsLock};
pub use refresh::{RefreshError, refresh};
pub use source::{FileSource, NewReview, Source, SourceError};
