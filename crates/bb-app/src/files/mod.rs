//! Files on issues and comments (ATT-01…ATT-04, ATT-07, ADR-0032): uploads in parts, the check of
//! the real type, attachments with the trash and *Undo*, signed downloads, previews and the daily
//! clean-up.
//!
//! Four rules run through all of it:
//! - **Whoever may change the issue may add files to it** (a Guest only on the issues they
//!   reported); a file on a comment is added by the person who wrote it. Whoever added a file, or
//!   may change issues, may delete it.
//! - **Nothing is trusted from the sender:** the real type comes from the content, the size is
//!   checked before the parts are stored, and the name is cut to a plain file name.
//! - **A download is a signed link for one person and five minutes**, and every use judges access
//!   again, so a link that is passed on or kept after access ended opens nothing.
//! - **A preview is made in a process of its own** and a file that cannot be previewed just has
//!   none; the server goes on.

mod attachments;
mod download;
mod file_store;
mod jobs;
mod member;
mod rules;
mod store;
mod types;
mod upload;

pub use attachments::{delete_attachment, file_rules, issue_files, restore_attachment};
pub use download::download;
pub use file_store::{
    ChunkReader, FileBody, FileHold, FileStore, Inspected, NoFileStore, PartError, Sniffed,
};
pub use store::AttachmentStore;
pub use types::*;
pub use upload::{cancel_upload, finish_upload, send_part, start_upload, upload_state};
