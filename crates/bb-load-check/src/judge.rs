//! The invariants after a run (architecture §11, ADR-0019, TEST-CONCURRENT), written once. The
//! book says what the people did; the facts say what the database holds. What does not agree is
//! listed in words. Nothing else in the repository defines these rules: the test of the server and
//! `cargo xtask load` both end here.
//!
//! - Every change has its event: the versions of an issue run from 2 up to the one it has, each
//!   kept by exactly one person and counted by exactly one event. (Other events about the issue, a
//!   lock taken over or let go, count no version and are no changes.)
//! - Every comment has its event.
//! - The description is the text of the last save that went through.
//! - What a person read of an issue only goes up.
//! - No text is lost: the text of a refused save was found as the draft at once, and the heartbeat
//!   draft of an editor who vanished is still there.
//! - A lock never outlives its lease: no row ends later than the lease from now, nobody holds a
//!   lock once the leases are over, and after the purge no row is left.

use crate::book::{Book, DraftEnd, last_drafts};
use crate::facts::{Facts, IssueFacts, LockRow};

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
    if issue.event_versions != counting(2, issue.version) {
        found.push(format!(
            "{key}: the events of its changes have the versions {:?}, not 2 up to {}",
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
    }
    found.extend(no_text_lost(book, facts));
    found.extend(locks_are_gone(book, facts, afterwards));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Drafted, Kept, Seen, Work};
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

    /// A run of one issue that went well: made, a title change, a description save, a comment.
    fn good_run() -> (Book, Facts) {
        let book = Book {
            issues: vec![Work {
                id: "id-1".to_owned(),
                key: "A-1".to_owned(),
            }],
            kept: vec![kept(2, None), kept(3, Some("Last text"))],
            comments: vec!["A-1".to_owned()],
            seen: vec![seen(1, 2), seen(1, 3)],
            drafted: vec![drafted("p1", DraftEnd::Vanished, None)],
            broken: Vec::new(),
        };
        let facts = Facts {
            issues: vec![IssueFacts {
                key: "A-1".to_owned(),
                event_versions: vec![2, 3],
                comment_events: 1,
                version: 3,
                description: "Last text".to_owned(),
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
    fn test_concurrent_what_the_people_saw_break_is_reported() {
        let (mut book, facts) = good_run();
        book.broken.push("person 3 did 12 on A-1: 500".to_owned());
        assert_eq!(judge(&book, &facts, &afterwards()).len(), 1);
    }
}
