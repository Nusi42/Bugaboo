//! *E-mail me this search* (QRY-07, ADR-0033, amendment 2026-11-24): a person gets a saved search
//! they may open by e-mail, on the weekdays and at the hour they chose, in their own time zone.
//!
//! Three rules run through all of it:
//! - **To the person only.** An e-mailed search is the person's own: only a session reaches its
//!   routes, never a key or an agent, and its e-mail goes to the person's own address on record.
//! - **Run as the person when it is sent.** The search runs with the person's rights at that
//!   moment, through the one search (`job.rs`); what they may not see is not in the e-mail.
//! - **It pauses, in one place.** `job.rs` decides when an e-mail goes and when nothing goes: while
//!   e-mail or `search-mail.allowed` is off nothing goes, and it goes on by itself when they are on
//!   again; a person who is switched off, or who can no longer open the search, makes it wait until
//!   they start it again (`write.rs`).
//!
//! - `write.rs`: set (make, change, start again) and stop.
//! - `read.rs`: the person's list.
//! - `job.rs`: the hourly round that queues the e-mails that are due, and the job of one e-mail.

mod job;
mod read;
mod store;
mod types;
mod write;

pub use read::search_mails;
pub use store::SearchMailStore;
pub use types::*;
pub use write::{set_search_mail, stop_search_mail};

use bb_core::settings::{Layers, MAIL_ENABLED, SEARCH_MAIL_ALLOWED};

/// Whether the e-mails of e-mailed searches may go now, by the organization's settings: e-mail is
/// switched on, and so is *E-mail me this search*. The two pauses that end by themselves.
fn sending(layers: &Layers) -> bool {
    layers.yes(&MAIL_ENABLED) && layers.yes(&SEARCH_MAIL_ALLOWED)
}
