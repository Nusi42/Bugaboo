//! The port of the removal of a person (PRIV-04, ADR-0044, decision 6): what *Remove a person*
//! needs from storage beyond each module's own deletion function. Both writes find their rows by
//! the walk's conditions (`PersonData`), so the removal never searches rows by person a second way.
//!
//! Like the other ports, **writes** are associated functions that run inside `record_change`, so
//! the removal and its domain event are committed together (ARCH-EVENTS). The scrub of the changes
//! is no such write: it goes through whole tables, so the job of the removal runs it in steps, each
//! its own short change (`scrub_step`).

use std::future::Future;

use bb_core::event::{EntityRef, RecordKind};
use bb_core::id::{CommentId, IssueId, JobId, UserId};
use bb_core::names::name_key;
use bb_core::personal::{EntityMarks, Link};
use bb_core::removal::PendingRemoval;
use bb_core::secret::StoredSecret;
use bb_markdown::{MENTION_MARK, MENTION_TRAILING, is_in_mention};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::ports::ChangeStore;
use crate::store_macros::{queries, reads, writes};

/// What the changes of events, the history and notifications are scrubbed of (M2-F75, M5-F63):
/// the values of a change about one of these records lose their before and after, and so does
/// every value that holds one of the person's words ([`Scrub::holds`]). The names of the members
/// stay, so the history still says what changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scrub {
    /// The kinds whose records belong to the person, as `bb_core::whose` says: every change
    /// about such a record of the person, and every change the person made to a record of such
    /// a kind that only its owner makes (a key that is gone by now), is scrubbed.
    pub kinds: Vec<RecordKind>,
    /// Records named one by one: the person's own settings, their private searches.
    pub records: Vec<EntityRef>,
    /// The person's full names, each found as one phrase of whole words when it has two words or
    /// more. A name of one word ("Mark") is a common word too often; its person is found by the
    /// handle and the address.
    pub names: Vec<String>,
    /// The person's handles (without `@`), each found only as a mention: `@` and the whole handle
    /// (`@lea.lind`, not `@lea.lindner`). A handle stands in a text only as a mention.
    pub handles: Vec<String>,
    /// The person's e-mail addresses, found anywhere in a text.
    pub addresses: Vec<String>,
}

/// The fewest characters a name or a handle has to be looked for: a shorter one ("Al") is part of
/// too many words and names nobody.
pub const NAME_LETTERS_MIN: usize = 3;

/// The fewest words a full name has to be looked for as a phrase.
const NAME_WORDS_MIN: usize = 2;

impl Scrub {
    /// The texts a value is looked for, each folded the way names are compared
    /// (`bb_core::names::name_key`): the full names, the handles and the addresses. Only these
    /// pick the rows the store reads; [`Self::holds`] decides.
    #[must_use]
    pub fn looked_for(&self) -> Vec<String> {
        self.folded_names()
            .chain(self.folded_handles())
            .chain(self.folded_addresses())
            .collect()
    }

    fn folded_names(&self) -> impl Iterator<Item = String> + '_ {
        folded_at_least(&self.names, NAME_LETTERS_MIN)
            .filter(|name| name.split_whitespace().count() >= NAME_WORDS_MIN)
    }

    /// The handles as their mentions: `@` and the folded handle.
    fn folded_handles(&self) -> impl Iterator<Item = String> + '_ {
        folded_at_least(&self.handles, NAME_LETTERS_MIN)
            .map(|handle| format!("{MENTION_MARK}{handle}"))
    }

    fn folded_addresses(&self) -> impl Iterator<Item = String> + '_ {
        folded_at_least(&self.addresses, 1)
    }

    /// Whether `text` holds one of the person's words, capitals not counting and spaces squeezed:
    /// a full name of two words or more as one phrase of whole words (no letter or digit, in any
    /// script, right before or after it), a handle as a whole mention, an address anywhere.
    #[must_use]
    pub fn holds(&self, text: &str) -> bool {
        let folded = name_key(text);
        self.folded_addresses()
            .any(|address| folded.contains(&address))
            || self
                .folded_names()
                .any(|name| holds_whole(&folded, &name, char::is_alphanumeric, &[]))
            || self
                .folded_handles()
                .any(|handle| holds_whole(&folded, &handle, is_in_mention, &MENTION_TRAILING))
    }

    /// Whether a JSON value holds one of the person's words in one of its texts.
    #[must_use]
    pub fn holds_value(&self, value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::String(text) => self.holds(text),
            serde_json::Value::Array(items) => items.iter().any(|item| self.holds_value(item)),
            serde_json::Value::Object(members) => members
                .iter()
                .any(|(key, item)| self.holds(key) || self.holds_value(item)),
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
                false
            }
        }
    }
}

/// `texts` folded the way names are compared, each with at least `fewest` characters.
fn folded_at_least(texts: &[String], fewest: usize) -> impl Iterator<Item = String> + '_ {
    texts
        .iter()
        .map(|text| name_key(text))
        .filter(move |folded| folded.chars().count() >= fewest)
}

/// Whether `found` stands in `text` on its own: no character of `inside` right before it, and none
/// right after it once a run of `trailing` (the dot that ends a sentence after a handle) is
/// skipped.
fn holds_whole(text: &str, found: &str, inside: fn(char) -> bool, trailing: &[char]) -> bool {
    text.match_indices(found).any(|(at, matched)| {
        let before = text[..at].chars().next_back();
        let after = text[at + matched.len()..]
            .trim_start_matches(trailing)
            .chars()
            .next();
        !before.is_some_and(inside) && !after.is_some_and(inside)
    })
}

bb_core::words! {
    /// The tables whose changes the scrub of a removal goes through, in this order.
    #[derive(Default)]
    pub enum ScrubbedTable {
        /// The events, with their reasons.
        #[default]
        Events => "events",
        /// The history of the issues.
        History => "history",
        /// The notifications.
        Notifications => "notifications",
    }
}

impl ScrubbedTable {
    /// The table after this one; none after the last.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Events => Some(Self::History),
            Self::History => Some(Self::Notifications),
            Self::Notifications => None,
        }
    }
}

/// Where the scrub of the changes of a removal goes on (ADR-0044, decision 6): the table, and the
/// key of the last row of it that a step looked at, each column as hexadecimal digits; empty at the
/// start of the table. It is the payload of the job, so a step that stopped is done again from
/// here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrubPlace {
    /// The table.
    pub table: ScrubbedTable,
    /// The key of the last row looked at.
    #[serde(default)]
    pub after: Vec<String>,
}

/// What the person's records say of them, read when the removal is prepared and again inside its
/// change: the texts no change may keep afterwards. The removal keeps them sealed as a stored
/// secret of their own until its job has scrubbed them from the changes
/// (`RemovalStore::seal_removal_words`); no payload of a job holds them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonWords {
    /// The name, which the legal-hold field keeps while work days are kept.
    pub name: String,
    /// The e-mail address.
    pub address: String,
    /// Every handle the person has or had.
    pub handles: Vec<String>,
}

/// What the removal of a person needs from storage.
pub trait RemovalStore: ChangeStore + 'static {
    reads! {
        /// Makes the full-text index forget the texts a removal deleted: the rows of `comments` go,
        /// and the rows of `issues` are made again from what is stored now. The index follows the
        /// stored texts, so this is no change of its own (the job `privacy.finish-removal`).
        fn reindex_removed_texts(issues: &[IssueId], comments: &[CommentId]) -> ();
        /// Lets the database go of the old text after the last step of a removal: on SQLite the
        /// index merges its segments and the write-ahead log is folded back and emptied, so no
        /// old page keeps a deleted text; on PostgreSQL nothing.
        fn compact_removed_texts() -> ();
        /// One step of the scrub of the changes of the events, the history and the notifications
        /// of the words of `scrub` (`Scrub::looked_for`): the next rows of the table of `from`, a
        /// page of
        /// `SCRUB_STEP_ROWS` read by key outside any change, and the rows of them that change
        /// written in one short change, so a writer waits for one step at most. Where the next step
        /// goes on; none after the last row of the last table. Safe to run again: a row scrubbed
        /// once has nothing left to scrub.
        fn scrub_step(scrub: &Scrub, from: &ScrubPlace) -> Option<ScrubPlace>;
        /// The words of the removal of `person`, encrypted with a key of their own that the newest
        /// master key wraps (`bb-store`, `secrets`): what the change of the removal stores with its
        /// row (`start_removal`), so that only the key files open it and the last step, which
        /// deletes the secret with its key, leaves nothing that opens a copy.
        fn seal_removal_words(person: UserId, words: &PersonWords, at: Timestamp) -> StoredSecret;
        /// The words of the removal of `person` that is not finished, opened; none when no secret
        /// of it is stored.
        fn removal_words(person: UserId) -> Option<PersonWords>;
        /// The removals that stopped: their job gave up (it is on the dead-letter list) or is gone.
        /// `privacy.resume-removals` gives each a new job, and the status page counts them.
        fn stopped_removals() -> Vec<PendingRemoval>;
    }

    queries! {
        /// The removal of `person` that is not finished: a person is being removed exactly while
        /// it exists (*removing…*).
        fn pending_removal(person: UserId) -> Option<PendingRemoval> = pending_removal_in;
    }

    writes! {
        /// The name, the address and the handles of `person`, read under the change; none for
        /// nobody (a replay into a backup older than the account).
        fn person_words(person: UserId) -> Option<PersonWords>;
        /// Records a removal that is not finished: its row and the secret of its words, in the
        /// change of the removal.
        fn start_removal(pending: &PendingRemoval, words: &StoredSecret) -> ();
        /// Gives the removal of `person` its next step `step` and the job `to` that does it, only
        /// while `from` is the job of the removal; whether it did. A step that runs twice (its
        /// worker lost the lease after the step was done) finds another job and queues nothing.
        fn move_removal(
            person: UserId,
            from: JobId,
            to: JobId,
            step: &str,
            at: Timestamp
        ) -> bool;
        /// Ends the removal of `person`, only while `job` is its job: deletes its row and the
        /// secret of its words with its key; whether it did.
        fn end_removal(person: UserId, job: JobId) -> bool;
        /// Puts every comment of `author` that is not in the trash into it at `at`, in the change
        /// of a removal whose comments go: hidden from every read, search and export at once (the
        /// trash is), and kept from being restored while the removal goes on; the job deletes them;
        /// how many.
        fn hide_comments_of(author: UserId, at: Timestamp) -> u64;
        /// Deletes the rows of the entity of `kind` that name `person` by a link that is one of
        /// `links`, found by the walk's conditions; how many went. An entity that is not stored
        /// deletes nothing.
        fn delete_linked(person: UserId, kind: &EntityMarks, links: &[Link]) -> u64;
        /// Deletes the jobs whose payload holds `address` (in any case), waiting or dead: a payload
        /// that names the person only by the address (a queued e-mail); how many went.
        fn delete_jobs_with_address(address: &str) -> u64;
        /// Scrubs every value of the changes of the events, the history and the notifications
        /// about a record that `scrub` names (`Scrub::kinds`, `Scrub::records`), about `person`,
        /// found through the records and the index of each table; how many rows changed. The
        /// words of `scrub` are the job's (`scrub_step`).
        fn scrub_records(person: UserId, scrub: &Scrub) -> u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(name: &str) -> Scrub {
        Scrub {
            names: vec![name.to_owned()],
            handles: vec!["lea.lind".to_owned()],
            addresses: vec!["Lea@Example.test".to_owned()],
            ..Scrub::default()
        }
    }

    #[test]
    fn priv_04_a_full_name_is_found_as_one_phrase_whatever_the_capitals_and_spaces() {
        let lea = of("Lea Lind");
        assert!(lea.holds("ask LEA   lind, please"));
        assert!(lea.holds("Mail lea@example.test now"));
        assert!(!lea.holds("LEA, please"));
        assert!(!lea.holds("Lind will know"));
        assert!(!lea.holds("Leadership and Linden"));
        assert!(of("Jürgen Groß").holds("Frag JÜRGEN GROß"));
        assert!(!of("Jürgen Groß").holds("Jürgen Großmann"));
        assert!(of("Özlem Çelik").holds("ÖZLEM ÇELIK"));
    }

    #[test]
    fn priv_04_a_surname_that_is_a_common_word_leaves_the_word_alone() {
        for name in ["Anna Will", "Ben Koch"] {
            let scrub = of(name);
            assert!(!scrub.holds("Will fix later"), "{name}");
            assert!(!scrub.holds("Koch the rice"), "{name}");
        }
    }

    #[test]
    fn priv_04_a_handle_is_found_as_a_whole_handle() {
        let lea = of("Lea Lind");
        assert!(lea.holds("thanks @lea.lind."));
        assert!(lea.holds("@LEA.LIND said"));
        assert!(!lea.holds("@lea.lindner said"));
        assert!(!lea.holds("@lea.lind.2 said"));
        assert!(!lea.holds("@x.lea.lind said"));
        assert!(
            !lea.holds("lea.lind said"),
            "a handle without `@` is no mention"
        );
        assert!(!lea.holds("x@lea.lind said"), "an address is no mention");
    }

    #[test]
    fn priv_04_a_name_of_one_word_is_found_only_by_its_mention_and_address() {
        let mark = Scrub {
            names: vec!["Mark".to_owned()],
            handles: vec!["mark".to_owned()],
            addresses: vec!["mark@example.test".to_owned()],
            ..Scrub::default()
        };
        assert!(!mark.holds("Mark as done"));
        assert!(!mark.holds("mark the issue, Mark"));
        assert!(mark.holds("thanks @mark."));
        assert!(mark.holds("@MARK said"));
        assert!(mark.holds("write to mark@example.test"));
        let max = Scrub {
            names: vec!["Max".to_owned()],
            handles: vec!["max".to_owned()],
            ..Scrub::default()
        };
        assert!(!max.holds("max retries"));
        assert!(max.holds("ask @max"));
    }

    #[test]
    fn priv_04_a_short_name_is_never_looked_for() {
        let al = Scrub {
            names: vec!["Al".to_owned()],
            handles: vec!["al".to_owned()],
            ..Scrub::default()
        };
        assert!(!al.holds("Alpha"));
        assert!(!al.holds("Al said"));
        assert!(al.looked_for().is_empty());
    }
}
