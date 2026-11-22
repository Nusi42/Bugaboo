//! The rules for the copy that a backup made in the middle of a run was restored into (OPS-07,
//! architecture §11, "The long run"). The backup holds one consistent state: every change kept
//! before it was asked for is in it, and what it holds of an issue is what the changes kept up to
//! that version made, each with its event, one by one. The people who left during the run are still
//! switched off or removed in the copy, whether they left before the backup or after it: the
//! restore replays the replay list (ADR-0044, decision 8).
//!
//! The rules of the whole run are judged on the book cut at the version each issue has in the copy,
//! so nothing is written twice.

use std::collections::BTreeMap;

use bb_store::Database;

use crate::book::Book;
use crate::facts::{Facts, Failure, gather};
use crate::judge::{events_and_versions, last_description, titles_and_statuses};
use crate::privacy::who_left_stays_out;

/// The book as far as the copy reaches: the changes kept up to the version each issue has in it.
fn cut_at(book: &Book, facts: &Facts) -> Book {
    let versions: BTreeMap<&str, i64> = facts
        .issues
        .iter()
        .map(|issue| (issue.key.as_str(), issue.version))
        .collect();
    Book {
        kept: book
            .kept
            .iter()
            .filter(|each| {
                versions
                    .get(each.key.as_str())
                    .is_some_and(|version| each.version <= *version)
            })
            .cloned()
            .collect(),
        ..Book::default()
    }
}

/// Judges the copy a backup of the run was restored into against the book; what does not hold is
/// listed in words.
#[must_use]
pub fn judge_restored(book: &Book, facts: &Facts) -> Vec<String> {
    let Some(backup) = &book.backup else {
        return vec!["the book names no backup made while the people worked".to_owned()];
    };
    let mut found = Vec::new();
    let before = usize::try_from(backup.kept_before).unwrap_or(usize::MAX);
    for issue in &facts.issues {
        let highest = book
            .kept
            .iter()
            .take(before)
            .filter(|each| each.key == issue.key)
            .map(|each| each.version)
            .max();
        if let Some(highest) = highest
            && issue.version < highest
        {
            found.push(format!(
                "{}: the backup holds version {}, but version {highest} was kept before it was asked for",
                issue.key, issue.version
            ));
        }
    }
    let cut = cut_at(book, facts);
    for issue in &facts.issues {
        found.extend(events_and_versions(&cut, issue));
        found.extend(last_description(&cut, issue));
        found.extend(titles_and_statuses(&cut, issue));
    }
    found.extend(who_left_stays_out(book, &facts.left));
    found
}

/// Reads the copy a backup of the run was restored into and judges it.
///
/// # Errors
/// Fails when the database cannot be read or the book names something that is not there.
pub async fn check_restored(database: &Database, book: &Book) -> Result<Vec<String>, Failure> {
    Ok(judge_restored(book, &gather(database, book).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{BackedUp, Kept};
    use crate::facts::IssueFacts;

    fn kept(version: i64, text: &str) -> Kept {
        Kept {
            key: "A-1".to_owned(),
            version,
            text: Some(text.to_owned()),
            ..Kept::default()
        }
    }

    fn book() -> Book {
        Book {
            kept: vec![kept(2, "two"), kept(3, "three"), kept(4, "four")],
            backup: Some(BackedUp {
                name: "a.backup".to_owned(),
                kept_before: 2,
            }),
            ..Book::default()
        }
    }

    fn restored(version: i64, text: &str) -> Facts {
        Facts {
            issues: vec![IssueFacts {
                key: "A-1".to_owned(),
                event_versions: (2..=version).collect(),
                comment_events: 0,
                version,
                description: text.to_owned(),
                title: String::new(),
                status: String::new(),
                attachments: 0,
                timer_entries: Vec::new(),
                start_date: None,
                due_date: None,
                accepted_on: Vec::new(),
            }],
            ..Facts::default()
        }
    }

    #[test]
    fn ops_07_ten_people_save_while_a_backup_is_made_and_every_save_up_to_its_moment_is_in_it() {
        // The backup holds what was kept before it, and maybe what was kept while it was made.
        assert!(judge_restored(&book(), &restored(3, "three")).is_empty());
        assert!(judge_restored(&book(), &restored(4, "four")).is_empty());
        // A change kept before the backup is missing from it: found.
        assert_eq!(judge_restored(&book(), &restored(2, "two")).len(), 1);
        // Half a change: the text of another version than the one it holds.
        assert_eq!(judge_restored(&book(), &restored(3, "four")).len(), 1);
        // No backup in the book.
        let none = Book {
            backup: None,
            ..book()
        };
        assert_eq!(judge_restored(&none, &restored(3, "three")).len(), 1);
    }
}
