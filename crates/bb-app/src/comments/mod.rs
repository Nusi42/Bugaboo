//! Comments and mentions (ISS-05, M3-Q3, M3-Q7): writing, editing under a lock, the earlier
//! versions, the trash, `@` names, and the *Mentions* block of the start page.
//!
//! Three rules run through all of it:
//! - **A comment is seen by whoever sees its issue**, and its earlier versions with it.
//! - **A text is rendered on the server for each reader** (`resolve.rs`): an issue ID the reader may
//!   not see and a name that names nothing the reader may name stay plain text.
//! - **A mention follows *Who sees people*** (kind *Mentions*): a person who sees roles only may
//!   mention a group or a role, never a person by name.

mod read;
mod records;
mod resolve;
mod store;
mod types;
mod write;

pub use read::{comment_versions, mention_choices, mentions, read_mentions};
pub use records::CommentRecords;
pub use store::CommentStore;
pub use types::*;
pub use write::{add_comment, delete_comment, restore_comment, save_comment};
