//! The invariants after a run (architecture §11, ADR-0019, TEST-CONCURRENT), written once. The
//! book says what the people did; the facts say what the database holds. What does not agree is
//! listed in words. Nothing else in the repository defines these rules: the test of the server and
//! `cargo xtask load` both end here.
//!
//! - Every change has its event: the versions of an issue run from 2 up to the one it has, each
//!   kept by exactly one person and counted by exactly one event. Every event that counts a version
//!   is one (`DomainEvent::version_after`): a save, a tag or a label put on or taken off, a change
//!   made by a job for many issues. (Other events about the issue, a lock taken over or let go,
//!   count no version and are no changes.)
//! - Every comment has its event.
//! - The description is the text of the last save that went through.
//! - What a person read of an issue only goes up.
//! - No text is lost: the text of a refused save was found as the draft at once, and the heartbeat
//!   draft of an editor who vanished is still there.
//! - The title is the last title that was kept, and the status the last status change that was kept
//!   (M3-F93, WF-01).
//! - The files of an issue are the ones that were added to it (ATT-01).
//! - No notification shows a Guest what a Guest may not see: only issues they reported, and none of
//!   the fields that are hidden from Guests (SEC-06).
//! - A lock never outlives its lease: no row ends later than the lease from now, nobody holds a
//!   lock once the leases are over, and after the purge no row is left.

use bb_app::access::StartingRole;

use crate::book::{Book, DraftEnd, last_drafts};
use crate::facts::{Facts, IssueFacts, LockRow, NotificationFacts};

/// When the run is looked at.
#[derive(Clone, Copy, Debug)]
pub struct Afterwards {
    /// The moment, in seconds since 1970, on the clock the server ran on.
    pub now: i64,
    /// How long a lease lasts.
    pub lease_seconds: i64,
    /// Whether the purge of the expired rows has run: then no lock row may be left.
    pub purged: bool,
}

/// The numbers from `from` up to and including `to`.
fn counting(from: i64, to: i64) -> Vec<i64> {
    (from..=to).collect()
}

/// Every change of an issue came with one event, and the versions went up one at a time.
fn events_and_versions(book: &Book, issue: &IssueFacts) -> Vec<String> {
    let key = &issue.key;
    let mut kept: Vec<i64> = book
        .kept
        .iter()
        .filter(|each| &each.key == key)
        .map(|each| each.version)
        .collect();
    kept.sort_unstable();
    let mut found = Vec::new();
    if kept != counting(2, issue.version) {
        found.push(format!(
            "{key}: the {} changes that were kept do not have the versions 2 up to {} one by one",
            kept.len(),
            issue.version
        ));
    }
    // The events come in the order they were committed (the outbox), so the versions must run up
    // one by one in that order: a change committed out of order is found too.
    if issue.event_versions != counting(2, issue.version) {
        found.push(format!(
            "{key}: the events of its changes have the versions {:?}, not 2 up to {} in the order they were committed",
            issue.event_versions, issue.version
        ));
    }
    let written = book.comments.iter().filter(|each| *each == key).count();
    if issue.comment_events != written {
        found.push(format!(
            "{key}: {} events for {written} comments written",
            issue.comment_events
        ));
    }
    found
}

/// The description is the text of the last save that was kept.
fn last_description(book: &Book, issue: &IssueFacts) -> Option<String> {
    let last = book
        .kept
        .iter()
        .filter(|each| each.key == issue.key && each.text.is_some())
        .max_by_key(|each| each.version)
        .and_then(|each| each.text.clone())
        .unwrap_or_default();
    (issue.description != last).then(|| {
        format!(
            "{}: the description is not the text that was kept last",
            issue.key
        )
    })
}

/// The last value that a kept change set, by version: the title or the status.
fn last_kept<'a>(
    book: &'a Book,
    issue: &IssueFacts,
    value: impl Fn(&'a crate::book::Kept) -> Option<&'a String>,
) -> Option<&'a String> {
    book.kept
        .iter()
        .filter(|each| each.key == issue.key)
        .filter_map(|each| value(each).map(|set| (each.version, set)))
        .max_by_key(|(version, _)| *version)
        .map(|(_, set)| set)
}

/// The title and the status are what the last change that set them set (M3-F93): a title or a
/// status change that was lost or half saved is found. An issue nobody changed keeps what it had.
fn titles_and_statuses(book: &Book, issue: &IssueFacts) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(title) = last_kept(book, issue, |each| each.title.as_ref())
        && *title != issue.title
    {
        found.push(format!(
            "{}: the title is not the one that was kept last",
            issue.key
        ));
    }
    if let Some(status) = last_kept(book, issue, |each| each.status.as_ref())
        && *status != issue.status
    {
        found.push(format!(
            "{}: the status is not the one that was kept last",
            issue.key
        ));
    }
    found
}

/// The files of an issue are the ones that were added to it, none more and none fewer.
fn files_are_kept(book: &Book, issue: &IssueFacts) -> Option<String> {
    let added = book.files.iter().filter(|each| **each == issue.key).count();
    (issue.attachments != added).then(|| {
        format!(
            "{}: {} files are stored, {added} were added",
            issue.key, issue.attachments
        )
    })
}

/// The fields a Guest may not see: the built-in ones the starting role hides, and the own fields the
/// run hid from Guests.
fn hidden_from_guests(book: &Book) -> Vec<String> {
    StartingRole::Guest
        .hidden_fields()
        .map(|field| field.code().to_owned())
        .chain(book.hidden_from_guests.iter().cloned())
        .collect()
}

/// A notification of a Guest is about an issue of the book that they reported (one about any other
/// issue, also one the book does not name, is found) and keeps no field they may not see.
fn guests_hear_only_what_they_may(book: &Book, notifications: &[NotificationFacts]) -> Vec<String> {
    let hidden = hidden_from_guests(book);
    let mut found = Vec::new();
    for each in notifications {
        if let Some(issue) = &each.issue {
            let reported = book
                .issues
                .iter()
                .find(|work| &work.id == issue)
                .is_some_and(|work| work.reporter.as_ref() == Some(&each.recipient));
            if !reported {
                found.push(format!(
                    "a Guest was told of {}, which they did not report",
                    book.name_of(issue)
                ));
            }
        }
        for field in each.fields.iter().filter(|field| hidden.contains(field)) {
            found.push(format!(
                "a notification of a Guest shows {field}, which a Guest may not see"
            ));
        }
    }
    found
}

/// Every search holds the rules while others change data (SEC-06, SEC-17, QRY-05): it finds no
/// issue the person may not read (a Guest reads only what they reported), a search for everything
/// finds exactly the issues the person reads and the count line says how many, and a search that
/// reads the own field hidden from Guests is refused for a Guest, as for an unknown field, and
/// answered for everybody else.
fn searches_hold_the_rules(book: &Book) -> Vec<String> {
    let mut found = Vec::new();
    for search in &book.searched {
        let guest = book.guests.contains(&search.holder);
        let readable: Vec<&str> = book
            .issues
            .iter()
            .filter(|work| !guest || work.reporter.as_ref() == Some(&search.holder))
            .map(|work| work.key.as_str())
            .collect();
        let who = if guest { "a Guest" } else { "a person" };
        for key in &search.found {
            if !readable.contains(&key.as_str()) {
                found.push(format!(
                    "the search `{}` of {who} found {key}, which they may not read",
                    search.query
                ));
            }
        }
        if search.reads_hidden && guest != search.refused.is_some() {
            found.push(format!(
                "the search `{}` of {who} was {}, a field a Guest may not read",
                search.query,
                if guest { "answered" } else { "refused" }
            ));
        }
        if search.everything && search.refused.is_none() && !search.more {
            let mut got: Vec<&str> = search.found.iter().map(String::as_str).collect();
            got.sort_unstable();
            let mut want = readable.clone();
            want.sort_unstable();
            if got != want {
                found.push(format!(
                    "a search for everything found {got:?} for {who}, who reads {want:?}"
                ));
            }
            if let Some(count) = search.count
                && usize::try_from(count).ok() != Some(search.found.len())
            {
                found.push(format!(
                    "the count line of a search for everything said {count} for {who}, the page held {}",
                    search.found.len()
                ));
            }
        }
    }
    found
}

/// What each person read of an issue only went up.
fn reads_went_up(book: &Book) -> Vec<String> {
    let mut last: Vec<(u32, &str, i64)> = Vec::new();
    let mut found = Vec::new();
    for seen in &book.seen {
        match last
            .iter_mut()
            .find(|(person, key, _)| *person == seen.person && *key == seen.key)
        {
            Some((_, _, version)) => {
                if seen.version < *version {
                    found.push(format!(
                        "person {} read {} at {} after {}",
                        seen.person, seen.key, seen.version, version
                    ));
                }
                *version = seen.version;
            }
            None => last.push((seen.person, seen.key.as_str(), seen.version)),
        }
    }
    found
}

/// No text is lost: a refused text was the draft at once, and a vanished editor's draft is there.
fn no_text_lost(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found: Vec<String> = book
        .drafted
        .iter()
        .filter(|each| each.ending == DraftEnd::Refused && each.found_as_draft != Some(true))
        .map(|each| {
            format!(
                "{}: the text of a refused save is not the draft: {}",
                each.key, each.text
            )
        })
        .collect();
    for drafted in last_drafts(book) {
        if drafted.ending != DraftEnd::Vanished {
            continue;
        }
        let stored = facts
            .drafts
            .iter()
            .find(|each| each.holder == drafted.holder && each.key == drafted.key)
            .and_then(|each| each.text.as_deref());
        if stored != Some(drafted.text.as_str()) {
            found.push(format!(
                "{}: the draft of an editor who vanished is not there: {}",
                drafted.key, drafted.text
            ));
        }
    }
    found
}

/// The lock rows that end later than the lease from `now`: the one definition of *a lease
/// outlived*. The test of the server asks it while the people work; the end of a run asks it too.
#[must_use]
pub fn locks_beyond_lease(
    book: &Book,
    locks: &[LockRow],
    now: i64,
    lease_seconds: i64,
) -> Vec<String> {
    locks
        .iter()
        .filter(|lock| lock.expires_at > now + lease_seconds)
        .map(|lock| {
            format!(
                "the lock on {} outlives its lease",
                book.name_of(&lock.record)
            )
        })
        .collect()
}

/// Once the leases are over nobody holds a lock, and after the purge no row is left.
fn locks_are_gone(book: &Book, facts: &Facts, afterwards: &Afterwards) -> Vec<String> {
    let mut found =
        locks_beyond_lease(book, &facts.locks, afterwards.now, afterwards.lease_seconds);
    let furthest = afterwards.now + afterwards.lease_seconds;
    for lock in &facts.locks {
        let name = book.name_of(&lock.record);
        if lock.expires_at > afterwards.now && lock.expires_at <= furthest {
            found.push(format!("{name}: a lock is still held at the end"));
        } else if afterwards.purged && lock.expires_at <= afterwards.now {
            found.push(format!(
                "{name}: the row of the lock is not gone after the purge"
            ));
        }
    }
    found
}

/// Everything that must hold after a run; what does not is listed in words (empty: it all does).
#[must_use]
pub fn judge(book: &Book, facts: &Facts, afterwards: &Afterwards) -> Vec<String> {
    let mut found = book.broken.clone();
    found.extend(reads_went_up(book));
    for issue in &facts.issues {
        found.extend(events_and_versions(book, issue));
        found.extend(last_description(book, issue));
        found.extend(titles_and_statuses(book, issue));
        found.extend(files_are_kept(book, issue));
    }
    found.extend(guests_hear_only_what_they_may(book, &facts.notifications));
    found.extend(searches_hold_the_rules(book));
    found.extend(no_text_lost(book, facts));
    found.extend(locks_are_gone(book, facts, afterwards));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Drafted, Kept, Searched, Seen, Work};
    use crate::facts::StoredDraft;

    const LEASE: i64 = 60;
    const NOW: i64 = 10_000;

    fn afterwards() -> Afterwards {
        Afterwards {
            now: NOW,
            lease_seconds: LEASE,
            purged: true,
        }
    }

    fn kept(version: i64, text: Option<&str>) -> Kept {
        Kept {
            key: "A-1".to_owned(),
            version,
            text: text.map(str::to_owned),
            title: None,
            status: None,
        }
    }

    fn seen(person: u32, version: i64) -> Seen {
        Seen {
            person,
            key: "A-1".to_owned(),
            version,
        }
    }

    fn drafted(holder: &str, ending: DraftEnd, found: Option<bool>) -> Drafted {
        Drafted {
            holder: holder.to_owned(),
            key: "A-1".to_owned(),
            text: format!("Text of {holder}"),
            ending,
            found_as_draft: found,
        }
    }

    /// A search for everything that was answered.
    fn search(holder: &str, everything: bool, found: &[&str], count: Option<u64>) -> Searched {
        Searched {
            holder: holder.to_owned(),
            query: String::new(),
            everything,
            found: found.iter().map(|key| (*key).to_owned()).collect(),
            count,
            ..Searched::default()
        }
    }

    /// A run of one issue that went well: made, a title change, a description save, a comment.
    fn good_run() -> (Book, Facts) {
        let book = Book {
            issues: vec![
                Work {
                    id: "id-1".to_owned(),
                    key: "A-1".to_owned(),
                    reporter: None,
                },
                Work {
                    id: "id-2".to_owned(),
                    key: "A-2".to_owned(),
                    reporter: Some("guest".to_owned()),
                },
            ],
            kept: vec![
                Kept {
                    title: Some("A title".to_owned()),
                    ..kept(2, None)
                },
                Kept {
                    status: Some("in-progress".to_owned()),
                    ..kept(3, Some("Last text"))
                },
            ],
            comments: vec!["A-1".to_owned()],
            seen: vec![seen(1, 2), seen(1, 3)],
            drafted: vec![drafted("p1", DraftEnd::Vanished, None)],
            broken: Vec::new(),
            files: vec!["A-1".to_owned(), "A-1".to_owned()],
            guests: vec!["guest".to_owned()],
            hidden_from_guests: vec!["secret-field".to_owned()],
            searched: vec![
                search("p1", true, &["A-1", "A-2"], Some(2)),
                search("guest", true, &["A-2"], Some(1)),
            ],
        };
        let facts = Facts {
            issues: vec![
                IssueFacts {
                    key: "A-1".to_owned(),
                    event_versions: vec![2, 3],
                    comment_events: 1,
                    version: 3,
                    description: "Last text".to_owned(),
                    title: "A title".to_owned(),
                    status: "in-progress".to_owned(),
                    attachments: 2,
                },
                IssueFacts {
                    key: "A-2".to_owned(),
                    event_versions: Vec::new(),
                    comment_events: 0,
                    version: 1,
                    description: String::new(),
                    title: "A bug of the Guest".to_owned(),
                    status: "open".to_owned(),
                    attachments: 0,
                },
            ],
            notifications: vec![NotificationFacts {
                recipient: "guest".to_owned(),
                issue: Some("id-2".to_owned()),
                fields: vec!["status".to_owned()],
            }],
            locks: Vec::new(),
            drafts: vec![StoredDraft {
                holder: "p1".to_owned(),
                key: "A-1".to_owned(),
                text: Some("Text of p1".to_owned()),
            }],
        };
        (book, facts)
    }

    #[test]
    fn test_concurrent_a_run_that_went_well_has_nothing_to_report() {
        let (book, facts) = good_run();
        assert_eq!(judge(&book, &facts, &afterwards()), Vec::<String>::new());
    }

    #[test]
    fn test_concurrent_a_change_without_its_event_or_a_skipped_version_is_found() {
        let (book, mut facts) = good_run();
        facts.issues[0].event_versions = vec![2];
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
        let (mut book, facts) = good_run();
        book.kept[0].version = 4;
        assert!(!judge(&book, &facts, &afterwards()).is_empty());
    }

    #[test]
    fn test_concurrent_events_must_count_the_versions_in_the_order_they_were_committed() {
        let (book, mut facts) = good_run();
        facts.issues[0].event_versions = vec![3, 2];
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
        facts.issues[0].event_versions = vec![3, 3];
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }

    #[test]
    fn test_concurrent_a_comment_without_its_event_is_found() {
        let (mut book, facts) = good_run();
        book.comments.push("A-1".to_owned());
        let found = judge(&book, &facts, &afterwards());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("comments"), "{found:?}");
    }

    #[test]
    fn test_concurrent_a_description_that_is_not_the_last_kept_text_is_found() {
        let (book, mut facts) = good_run();
        facts.issues[0].description = "Older text".to_owned();
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }

    #[test]
    fn test_concurrent_a_version_that_went_down_is_found_for_that_person_only() {
        let (mut book, facts) = good_run();
        book.seen.push(seen(2, 2));
        assert!(judge(&book, &facts, &afterwards()).is_empty());
        book.seen.push(seen(1, 2));
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }

    #[test]
    fn test_concurrent_a_vanished_editors_draft_that_is_gone_is_found() {
        let (book, mut facts) = good_run();
        facts.drafts[0].text = None;
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
        facts.drafts[0].text = Some("Another text".to_owned());
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }

    #[test]
    fn test_concurrent_a_later_save_ends_the_duty_to_keep_the_draft() {
        let (mut book, mut facts) = good_run();
        facts.drafts.clear();
        book.drafted.push(drafted("p1", DraftEnd::Saved, None));
        assert!(judge(&book, &facts, &afterwards()).is_empty());
    }

    #[test]
    fn test_concurrent_a_refused_text_that_was_not_the_draft_is_found() {
        let (mut book, facts) = good_run();
        for found in [None, Some(false)] {
            book.drafted.push(drafted("p2", DraftEnd::Refused, found));
        }
        let found = judge(&book, &facts, &afterwards());
        assert_eq!(found.len(), 2, "{found:?}");
        book.drafted.truncate(1);
        book.drafted
            .push(drafted("p2", DraftEnd::Refused, Some(true)));
        assert!(judge(&book, &facts, &afterwards()).is_empty());
    }

    #[test]
    fn test_concurrent_a_lock_is_judged_by_one_definition_of_outliving_its_lease() {
        let (book, mut facts) = good_run();
        // A lock that has run out is no lock; after the purge its row must be gone too.
        facts.locks.push(LockRow {
            record: "id-1".to_owned(),
            expires_at: NOW - 1,
        });
        let found = judge(&book, &facts, &afterwards());
        assert_eq!(found.len(), 1, "{found:?}");
        let before_purge = Afterwards {
            purged: false,
            ..afterwards()
        };
        assert!(judge(&book, &facts, &before_purge).is_empty());
        // One that is still held at the end, and one further away than the lease allows.
        facts.locks[0].expires_at = NOW + 1;
        assert_eq!(judge(&book, &facts, &before_purge).len(), 1);
        facts.locks[0].expires_at = NOW + LEASE + 1;
        assert_eq!(
            judge(&book, &facts, &before_purge),
            ["the lock on A-1 outlives its lease"]
        );
        assert_eq!(
            locks_beyond_lease(&book, &facts.locks, NOW, LEASE),
            ["the lock on A-1 outlives its lease"]
        );
        assert!(locks_beyond_lease(&book, &facts.locks, NOW + 1, LEASE).is_empty());
    }

    #[test]
    fn test_concurrent_m3_f93_a_title_that_is_not_the_last_kept_one_is_found() {
        let (book, mut facts) = good_run();
        facts.issues[0].title = "An older title".to_owned();
        let found = judge(&book, &facts, &afterwards());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("title"), "{found:?}");
    }

    #[test]
    fn test_concurrent_wf_01_a_status_that_is_not_the_last_kept_one_is_found() {
        let (book, mut facts) = good_run();
        facts.issues[0].status = "open".to_owned();
        let found = judge(&book, &facts, &afterwards());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("status"), "{found:?}");
    }

    #[test]
    fn test_concurrent_att_01_a_file_that_is_lost_or_added_twice_is_found() {
        let (book, mut facts) = good_run();
        facts.issues[0].attachments = 1;
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
        facts.issues[0].attachments = 3;
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }

    #[test]
    fn test_concurrent_sec_06_a_guest_is_told_only_of_what_they_reported_and_may_see() {
        let (book, mut facts) = good_run();
        // About an issue the team made, and about one that the book does not name at all.
        for issue in ["id-1", "id-unknown"] {
            facts.notifications[0].issue = Some(issue.to_owned());
            let found = judge(&book, &facts, &afterwards());
            assert_eq!(found.len(), 1, "{issue}: {found:?}");
            assert!(found[0].contains("did not report"), "{found:?}");
        }
        // A field the starting role hides from Guests, and one the run hid.
        facts.notifications[0].issue = Some("id-2".to_owned());
        for field in ["cost-center", "secret-field"] {
            facts.notifications[0].fields = vec!["status".to_owned(), field.to_owned()];
            let found = judge(&book, &facts, &afterwards());
            assert_eq!(found.len(), 1, "{field}: {found:?}");
            assert!(found[0].contains(field), "{found:?}");
        }
    }

    #[test]
    fn test_concurrent_sec_06_a_search_holds_the_rules_while_others_change_data() {
        let (book, facts) = good_run();
        let judged = |book: &Book| judge(book, &facts, &afterwards());
        // A Guest finds an issue they did not report.
        let mut wrong = good_run().0;
        wrong.searched[1].found = vec!["A-1".to_owned(), "A-2".to_owned()];
        assert!(
            judged(&wrong)
                .iter()
                .any(|each| each.contains("may not read"))
        );
        // A search for everything that misses an issue, and a count that differs.
        let mut missing = good_run().0;
        missing.searched[0].found = vec!["A-2".to_owned()];
        assert!(judged(&missing).iter().any(|each| each.contains("found")));
        let mut counted = good_run().0;
        counted.searched[0].count = Some(5);
        assert!(
            judged(&counted)
                .iter()
                .any(|each| each.contains("count line"))
        );
        // The hidden field: refused for a Guest, answered for a Team member.
        let mut hidden = good_run().0;
        hidden.searched.push(Searched {
            reads_hidden: true,
            query: "internalNote is x".to_owned(),
            ..search("guest", false, &[], None)
        });
        assert!(
            judged(&hidden)
                .iter()
                .any(|each| each.contains("a Guest may not read"))
        );
        hidden.searched[2].refused = Some("request.invalid".to_owned());
        assert_eq!(judged(&hidden), Vec::<String>::new());
        hidden.searched.push(Searched {
            reads_hidden: true,
            refused: Some("request.invalid".to_owned()),
            ..search("p1", false, &[], None)
        });
        assert!(judged(&hidden).iter().any(|each| each.contains("refused")));
        assert_eq!(judged(&book), Vec::<String>::new());
    }

    #[test]
    fn test_concurrent_what_the_people_saw_break_is_reported() {
        let (mut book, facts) = good_run();
        book.broken.push("person 3 did 12 on A-1: 500".to_owned());
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }
}
