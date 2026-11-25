//! The port of e-mailed searches: what the use cases and the jobs need from storage.
//!
//! Like the other ports, reads take `&self` and **writes** are associated functions that run inside
//! `record_change`, so a change of the person and its domain event are committed together
//! (ARCH-EVENTS). What the jobs write is bookkeeping of a choice that is stored already (when the
//! next e-mail is due, when the last one went, why it waits): reads that write, like the queue's.

use std::future::Future;

use bb_core::id::{SavedSearchId, SearchMailId, UserId};
use bb_core::search_mail::{MailPause, SearchMail};
use jiff::Timestamp;

use crate::jobs::QueuedJob;
use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the use cases and the jobs of e-mailed searches need from storage.
pub trait SearchMailStore: ChangeStore + 'static {
    queries! {
        /// The e-mailed search with this ID.
        fn search_mail(id: SearchMailId) -> Option<SearchMail> = search_mail_in;
        /// The e-mailed search a person has of a saved search, if they have one.
        fn search_mail_of(holder: UserId, search: SavedSearchId) -> Option<SearchMail>
            = search_mail_of_in;
        /// How many e-mailed searches a person has.
        fn count_search_mails_of(holder: UserId) -> u64 = count_search_mails_of_in;
    }

    reads! {
        /// The e-mailed searches of a person, oldest first: at most `SEARCH_MAILS_MAX`.
        fn search_mails_of(holder: UserId) -> Vec<SearchMail>;
        /// The e-mailed searches that send and whose next e-mail is due at `until` or before, the
        /// earliest first, at most `limit`.
        fn search_mails_due(until: Timestamp, limit: u64) -> Vec<SearchMail>;
        /// Queues the job of the e-mail that is due at `due` and moves the next e-mail on to
        /// `next`, both or neither; `false` (and nothing written) when the e-mailed search is gone,
        /// waits for its person or was given another time meanwhile.
        fn queue_search_mail(
            id: SearchMailId,
            due: Timestamp,
            next: Timestamp,
            job: &QueuedJob,
            now: Timestamp
        ) -> bool;
        /// Claims the e-mail of the due time `due` for the job that asks: `true` for the one job that
        /// gets it, `false` when that due time (or a later one) was claimed already. The database
        /// decides, so each due time goes at most once.
        fn claim_search_mail(id: SearchMailId, due: Timestamp) -> bool;
        /// Gives a claim back after its e-mail could not go, so another try of the job may send
        /// it: the claim of `due` becomes `before` again, unless a later one was made meanwhile.
        fn give_back_search_mail(id: SearchMailId, due: Timestamp, before: Option<Timestamp>) -> ();
        /// Writes down when the last e-mail went.
        fn search_mail_sent(id: SearchMailId, at: Timestamp) -> ();
        /// Makes the e-mailed search wait for its person, for `pause`.
        fn pause_search_mail(id: SearchMailId, pause: MailPause) -> ();
    }

    writes! {
        /// Adds an e-mailed search, or writes the choice of the one with its ID.
        fn put_search_mail(mail: &SearchMail) -> ();
        /// Deletes an e-mailed search; `false` when it was gone.
        fn delete_search_mail(id: SearchMailId) -> bool;
        /// Makes every e-mailed search of a person that sends wait for them, for `pause` (the person
        /// is switched off); how many.
        fn pause_search_mails_of(holder: UserId, pause: MailPause) -> u64;
    }
}
