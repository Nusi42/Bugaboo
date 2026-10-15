//! Editing without losing anything (ISS-17, ISS-11, ADR-0019): edit locks, drafts, the check of versions and
//! the structured conflict answer. Locks and drafts work on any record by its `EntityRef`; the kinds
//! of record say what they are through [`EditableRecords`], and the editing code names none of them.
//!
//! - `lock.rs`: the lease of 60 seconds, the heartbeat every 15 seconds, *Ask … to finish*,
//!   *Edit anyway* and the release of a stuck lock.
//! - `drafts.rs`: the private draft of a person, kept 30 days.
//! - `save.rs`: saving a long text with the lock token and the version, never dropping the text.
//! - `versions.rs`: the versions of what is on the screen.
//! - `conflict.rs`: the structured `edit.version_changed`.
//!
//! The lease, the heartbeat and the draft are the state of an open editor and write no domain
//! event; what other people can learn from is an event (see `types.rs`).

mod conflict;
mod drafts;
mod lock;
mod parts;
mod records;
mod save;
mod store;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
mod types;
mod versions;

pub use conflict::{Conflict, Stale};
pub use drafts::{purge_drafts, read_draft, save_draft, throw_away_draft};
pub use lock::{
    ask_to_finish, edit_anyway, heartbeat, read_lock, release_lock, release_stuck_lock, take_lock,
};
pub use records::{ChainedRecords, EditableRecords, NoRecords, RecordFacts, Shown};
pub use save::{TextSave, Written};
pub use store::EditStore;
pub use types::*;
pub use versions::check_versions;
