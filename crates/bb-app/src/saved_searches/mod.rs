//! Saved searches (QRY-06, CFG-02, ADR-0034): a name, the canonical search text, how the results
//! are shown, the person who made it and whom it is shared with.
//!
//! Three rules run through all of it:
//! - **Private when it is made.** Sharing needs *Share searches* at the place; only the person who
//!   made a search changes, shares or deletes it, and a person with *Change organization settings*
//!   may delete any (`share.rs`, `write.rs`).
//! - **It always runs with the rights of the person who opens it.** Being shared with never adds a
//!   right; a receiver is reached through a group of theirs or a project they see (`reach.rs`).
//! - **A search that is not for a person is not there for them.** It is not listed, and its
//!   address answers exactly like one that does not exist (`reach.rs`, `read.rs`).

mod events;
mod judge;
mod project_list;
mod reach;
mod read;
mod share;
mod store;
mod types;
mod view;
mod write;

pub use project_list::{project_list, set_project_list};
pub use read::{saved_search, saved_searches, share_choices};
pub use share::{share_search, stop_sharing_search};
pub use store::{Reach, SavedSearchStore};
pub use types::*;
pub use write::{change_search, delete_search, restore_search, save_copy, save_search};
