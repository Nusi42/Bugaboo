//! The people who left while the others worked (PRIV-04, architecture §11, "The long run"): a
//! person an administrator switched off is refused at the next save of the page they had open, has
//! no session left and stays switched off; a person who was removed as well has the code of the
//! tombstone. What else the run judges holds as it does without them: nobody else's work changes.

use bb_core::person::Standing;

use crate::book::Book;
use crate::facts::LeftFacts;

/// The people the book says left hold the rules of a person who left.
pub(crate) fn who_left_stays_out(book: &Book, facts: &[LeftFacts]) -> Vec<String> {
    let mut found = Vec::new();
    for left in &book.left {
        let who = &left.person;
        if !left.refused_at_next_save {
            found.push(format!(
                "{who}: the open page of a person who was switched off saved again"
            ));
        }
        let Some(held) = facts.iter().find(|each| &each.person == who) else {
            found.push(format!(
                "{who}: the database was not read for a person who left"
            ));
            continue;
        };
        let expected = if left.removed {
            Standing::Removed
        } else {
            Standing::SwitchedOff
        };
        if held.standing != Some(expected) {
            found.push(format!(
                "{who}: the standing is {:?}, not {expected:?}",
                held.standing
            ));
        }
        if left.removed && !held.labelled {
            found.push(format!(
                "{who}: a removed person has no code of the tombstone"
            ));
        }
        if held.sessions > 0 {
            found.push(format!(
                "{who}: a person who left still has {} sessions",
                held.sessions
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::Left;

    const PERSON: &str = "00000000-0000-4000-8000-000000000009";

    fn book(removed: bool, refused: bool) -> Book {
        Book {
            left: vec![Left {
                person: PERSON.to_owned(),
                removed,
                refused_at_next_save: refused,
            }],
            ..Book::default()
        }
    }

    fn held(standing: Option<Standing>, labelled: bool, sessions: usize) -> Vec<LeftFacts> {
        vec![LeftFacts {
            person: PERSON.to_owned(),
            standing,
            labelled,
            sessions,
        }]
    }

    #[test]
    fn priv_04_a_switched_off_persons_open_page_is_refused_at_its_next_save() {
        let off = held(Some(Standing::SwitchedOff), false, 0);
        assert!(who_left_stays_out(&book(false, true), &off).is_empty());
        // A save that went through, a session that is left or a person who is active again: found.
        assert_eq!(who_left_stays_out(&book(false, false), &off).len(), 1);
        let signed_in = held(Some(Standing::SwitchedOff), false, 1);
        assert_eq!(who_left_stays_out(&book(false, true), &signed_in).len(), 1);
        assert_eq!(
            who_left_stays_out(&book(false, true), &held(None, false, 0)).len(),
            1
        );
    }

    #[test]
    fn priv_04_ten_people_work_while_a_person_is_removed_and_nothing_else_changes() {
        let removed = held(Some(Standing::Removed), true, 0);
        assert!(who_left_stays_out(&book(true, true), &removed).is_empty());
        // Removed without the code of the tombstone, or only switched off: found.
        let unlabelled = held(Some(Standing::Removed), false, 0);
        assert_eq!(who_left_stays_out(&book(true, true), &unlabelled).len(), 1);
        let only_off = held(Some(Standing::SwitchedOff), false, 0);
        assert!(!who_left_stays_out(&book(true, true), &only_off).is_empty());
        assert_eq!(who_left_stays_out(&book(true, true), &[]).len(), 1);
    }
}
