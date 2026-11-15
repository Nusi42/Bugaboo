//! The rules of ten people planning at once (architecture §11, "Ten people plan at once",
//! TEST-CONCURRENT, PLAN-01, PLAN-11, PLAN-13, ADR-0043):
//!
//! - No date is lost: the dates of an issue are the dates of the last move the server kept.
//! - A plan warning that was accepted holds only the dates it was accepted on, never dates of a
//!   move that came meanwhile: a bar that moved brings the warning back, because the plan counts
//!   an acceptance only while the dates of its issue are the ones it holds.
//! - An away time that was entered is stored once, and the first answer of the plan after it showed
//!   the days.
//! - A *Move it after* is kept whole or refused whole: when it was refused, no issue it names was
//!   changed by it, and when it was kept, every issue it names was.
//!
//! The versions of the moves are judged with the others (every kept change has its event and the
//! versions run up one by one); these rules look only at what is special about dates.

use crate::book::Book;
use crate::facts::{Facts, IssueFacts};

fn issue_of<'a>(facts: &'a Facts, key: &str) -> Option<&'a IssueFacts> {
    facts.issues.iter().find(|issue| issue.key == key)
}

/// The dates of an issue are the dates of the last move that was kept.
fn no_date_is_lost(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found = Vec::new();
    for issue in &facts.issues {
        let Some(last) = book
            .moved
            .iter()
            .filter(|each| each.key == issue.key)
            .max_by_key(|each| each.version)
        else {
            continue;
        };
        if last.start != issue.start_date || last.due != issue.due_date {
            found.push(format!(
                "{}: the last move that was kept (version {}) left the dates {:?} to {:?}, the issue has {:?} to {:?}",
                issue.key, last.version, last.start, last.due, issue.start_date, issue.due_date
            ));
        }
    }
    found
}

/// A warning is accepted on the dates the person saw, never on the dates of a move that came
/// meanwhile.
fn accepted_warnings_come_back(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found = Vec::new();
    for accepted in &book.accepted {
        let Some(issue) = issue_of(facts, &accepted.key) else {
            continue;
        };
        let wanted = (accepted.start.clone(), accepted.due.clone());
        if issue.accepted_on.iter().any(|held| *held != wanted) {
            found.push(format!(
                "{}: a warning was accepted on {:?}, the dates the person saw were {:?}",
                accepted.key, issue.accepted_on, wanted
            ));
        }
    }
    found
}

/// Every away time that was entered is stored once, and the next answer of the plan showed it.
fn away_times_show_at_once(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found = Vec::new();
    for entered in &book.away {
        let name = format!(
            "the away time of {} from {} to {}",
            entered.person, entered.first, entered.last
        );
        let stored = facts
            .away_times
            .iter()
            .filter(|each| {
                each.person == entered.person
                    && each.first == entered.first
                    && each.last == entered.last
            })
            .count();
        if stored != 1 {
            found.push(format!("{name} is stored {stored} times, not once"));
        }
        if !entered.shown_on_next_read {
            found.push(format!(
                "{name}: the first answer of the plan after it did not show the days"
            ));
        }
    }
    found
}

/// A *Move it after* is kept whole or refused whole.
fn a_move_after_is_whole(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found = Vec::new();
    for moved in &book.moved_after {
        for named in &moved.named {
            let Some(issue) = issue_of(facts, &named.key) else {
                continue;
            };
            let changed_meanwhile = named.key == moved.changed;
            // The issue somebody changed meanwhile is at least one version on; the others are on
            // the version of the preview when the move was refused, and one version on when it was
            // kept.
            let expected = named.version + i64::from(moved.kept && !changed_meanwhile);
            let at_least = changed_meanwhile && !moved.kept;
            let wrong = if at_least {
                issue.version <= named.version
            } else if changed_meanwhile {
                false
            } else {
                issue.version != expected
            };
            if wrong {
                found.push(format!(
                    "{}: a move after that was {} left it at version {}, the preview read {}",
                    named.key,
                    if moved.kept { "kept" } else { "refused whole" },
                    issue.version,
                    named.version
                ));
            }
        }
    }
    found
}

/// What does not hold of the plans of a run, in words.
pub(crate) fn plans_hold_the_rules(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found = no_date_is_lost(book, facts);
    found.extend(accepted_warnings_come_back(book, facts));
    found.extend(away_times_show_at_once(book, facts));
    found.extend(a_move_after_is_whole(book, facts));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Accepted, AwayEntered, Moved, MovedAfter, NamedByMove};
    use crate::facts::AwayFacts;

    fn issue(
        key: &str,
        version: i64,
        dates: (&str, &str),
        accepted_on: &[(&str, &str)],
    ) -> IssueFacts {
        IssueFacts {
            key: key.to_owned(),
            event_versions: Vec::new(),
            comment_events: 0,
            version,
            description: String::new(),
            title: String::new(),
            status: String::new(),
            attachments: 0,
            timer_entries: Vec::new(),
            start_date: Some(dates.0.to_owned()),
            due_date: Some(dates.1.to_owned()),
            accepted_on: accepted_on
                .iter()
                .map(|(start, due)| (Some((*start).to_owned()), Some((*due).to_owned())))
                .collect(),
        }
    }

    fn moved(key: &str, version: i64, dates: (&str, &str)) -> Moved {
        Moved {
            key: key.to_owned(),
            version,
            start: Some(dates.0.to_owned()),
            due: Some(dates.1.to_owned()),
        }
    }

    fn facts_of(issues: Vec<IssueFacts>) -> Facts {
        Facts {
            issues,
            ..Facts::default()
        }
    }

    #[test]
    fn plan_03_the_dates_are_the_dates_of_the_last_move_that_was_kept() {
        let book = Book {
            moved: vec![
                moved("A-1", 2, ("2027-02-01", "2027-02-03")),
                moved("A-1", 3, ("2027-02-02", "2027-02-04")),
            ],
            ..Book::default()
        };
        let held = facts_of(vec![issue("A-1", 3, ("2027-02-02", "2027-02-04"), &[])]);
        assert!(no_date_is_lost(&book, &held).is_empty());
        let lost = facts_of(vec![issue("A-1", 3, ("2027-02-01", "2027-02-03"), &[])]);
        assert_eq!(no_date_is_lost(&book, &lost).len(), 1);
    }

    #[test]
    fn plan_13_a_warning_accepted_before_its_bar_moved_comes_back() {
        let accepted = Accepted {
            key: "A-1".to_owned(),
            start: Some("2027-02-01".to_owned()),
            due: Some("2027-02-03".to_owned()),
        };
        let book = Book {
            accepted: vec![accepted],
            ..Book::default()
        };
        // The bar moved after the acceptance: the acceptance keeps the old dates, so it no longer
        // counts and the warning is back.
        let seen = [("2027-02-01", "2027-02-03")];
        let back = facts_of(vec![issue("A-1", 3, ("2027-02-02", "2027-02-04"), &seen)]);
        assert!(accepted_warnings_come_back(&book, &back).is_empty());
        let still = facts_of(vec![issue("A-1", 2, ("2027-02-01", "2027-02-03"), &seen)]);
        assert!(accepted_warnings_come_back(&book, &still).is_empty());
        // An acceptance on dates the person never saw is found.
        let moved = [("2027-02-02", "2027-02-04")];
        let wrong = facts_of(vec![issue("A-1", 3, ("2027-02-02", "2027-02-04"), &moved)]);
        assert_eq!(accepted_warnings_come_back(&book, &wrong).len(), 1);
    }

    #[test]
    fn plan_11_an_away_time_is_stored_once_and_shows_on_the_next_answer() {
        let entered = |shown| AwayEntered {
            person: "p1".to_owned(),
            first: "2027-02-01".to_owned(),
            last: "2027-02-05".to_owned(),
            shown_on_next_read: shown,
        };
        let stored = || AwayFacts {
            person: "p1".to_owned(),
            first: "2027-02-01".to_owned(),
            last: "2027-02-05".to_owned(),
        };
        let book = Book {
            away: vec![entered(true)],
            ..Book::default()
        };
        let one = Facts {
            away_times: vec![stored()],
            ..Facts::default()
        };
        assert!(away_times_show_at_once(&book, &one).is_empty());
        assert_eq!(away_times_show_at_once(&book, &Facts::default()).len(), 1);
        let hidden = Book {
            away: vec![entered(false)],
            ..Book::default()
        };
        assert_eq!(away_times_show_at_once(&hidden, &one).len(), 1);
    }

    #[test]
    fn plan_13_a_move_after_is_kept_whole_or_refused_whole() {
        let named = |key: &str| NamedByMove {
            key: key.to_owned(),
            version: 2,
        };
        let book = |kept| Book {
            moved_after: vec![MovedAfter {
                named: vec![named("A-1"), named("A-2")],
                changed: "A-2".to_owned(),
                kept,
            }],
            ..Book::default()
        };
        // Refused: A-1 is where the preview read it, A-2 is on because somebody changed it.
        let refused = facts_of(vec![
            issue("A-1", 2, ("2027-02-01", "2027-02-03"), &[]),
            issue("A-2", 3, ("2027-02-01", "2027-02-03"), &[]),
        ]);
        assert!(a_move_after_is_whole(&book(false), &refused).is_empty());
        // Half of it was kept although it was refused.
        let half = facts_of(vec![
            issue("A-1", 3, ("2027-02-01", "2027-02-03"), &[]),
            issue("A-2", 3, ("2027-02-01", "2027-02-03"), &[]),
        ]);
        assert_eq!(a_move_after_is_whole(&book(false), &half).len(), 1);
        // Kept: the issue nobody else changed moved once.
        assert!(a_move_after_is_whole(&book(true), &half).is_empty());
        assert_eq!(a_move_after_is_whole(&book(true), &refused).len(), 1);
    }
}
