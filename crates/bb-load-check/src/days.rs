//! The rule of the work days of a run: each change the server kept is a version of the day with its
//! event, and its warnings are the ones the rules find.

use crate::book::Book;
use crate::facts::Facts;

/// Every change of a work day that the server kept is a version of the day with its event, and the
/// warnings the day holds are the ones the rules find in its times: each change was evaluated once.
pub(crate) fn days_are_kept_once(book: &Book, facts: &Facts) -> Vec<String> {
    let mut found = Vec::new();
    for day in &facts.days {
        let kept = book
            .days
            .iter()
            .filter(|each| each.person == day.person && each.date == day.date)
            .count();
        let name = format!("the work day {} of {}", day.date, day.person);
        if i64::try_from(kept).unwrap_or(-1) != day.version {
            found.push(format!(
                "{name}: {kept} changes were kept, the day has the version {}",
                day.version
            ));
        }
        if day.events != kept {
            found.push(format!(
                "{name}: {kept} changes were kept, {} events are about the day",
                day.events
            ));
        }
        if !day.warnings_as_the_rules_find_them {
            found.push(format!(
                "{name}: its warnings are not the ones the rules find in its times"
            ));
        }
    }
    found
}
