//! A time entry in the history of its issue (TIME-02, PRIV-09): time was booked, an entry changed
//! or was deleted. The reader sees the length only when they may read *Time spent* on the issue,
//! and the own fields of the entry only as their levels allow; who booked is shown as the kind
//! *Time entries* says (`read.rs`).

use bb_core::event::{FieldChange, RecordKind};
use bb_core::fields::{Field, FieldKey};
use bb_core::history::{HistoryEntry, TimeEvent};
use bb_core::id::{IssueTypeId, UserId};
use bb_core::time::Minutes;
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

use super::read::{Batch, Reading};
use super::types::HistoryWhat;
use crate::auth::{AppError, AuthStore, Ctx};
use crate::member::time_entry;

/// What happened to the entry, from the change of its length: a length that came is a booking, one
/// that went a deletion, and anything else a change.
fn event_of(minutes: Option<&FieldChange>) -> TimeEvent {
    match minutes.map(|change| (change.before.is_some(), change.after.is_some())) {
        Some((false, true)) => TimeEvent::Booked,
        Some((true, false)) => TimeEvent::Deleted,
        _ => TimeEvent::Changed,
    }
}

/// Whether a reader sees the moment a row of time was written: the person who made the row and a
/// holder of *See working hours of others*. The moment an entry is saved says when somebody
/// stopped working, so nobody else is told it (PRIV-09, ADR-0035, decision 8). The one rule; the
/// permission is looked up once for a request and passed in.
#[must_use]
pub(super) fn moment_visible(made_by: Option<UserId>, reader: UserId, sees_moments: bool) -> bool {
    sees_moments || made_by == Some(reader)
}

/// The date of the entry of a row. Every event of an entry says its date (a booking and a change that
/// keeps the date set it, a change that moves it says the new one, a deletion sets the old one); an
/// older event that says none gets the date the row was written on, never its time.
#[must_use]
pub(super) fn entry_day(row: &HistoryEntry) -> Date {
    row.changes
        .iter()
        .rev()
        .find(|change| change.field == time_entry::DATE)
        .and_then(|change| change.after.as_ref().or(change.before.as_ref()))
        .and_then(|value| serde_json::from_value::<Date>(value.clone()).ok())
        .unwrap_or_else(|| row.at.to_zoned(TimeZone::UTC).date())
}

/// The date of a row of time for a reader who may not see its moment; none for a reader who may.
#[must_use]
pub(super) fn hidden_day(row: &HistoryEntry, sees_moment: bool) -> Option<Date> {
    (!sees_moment).then(|| entry_day(row))
}

/// The moment a row is shown at to a reader who may not see the moment it was written: the start of
/// the date of a time entry, and the moment itself for every other row. It is stored with the row,
/// so the history can be put in order and cut in pages by it in the database.
#[must_use]
pub fn shown_moment(row: &HistoryEntry) -> Timestamp {
    if row.record_kind.as_str() == RecordKind::TimeEntry.code() {
        entry_day(row)
            .to_zoned(TimeZone::UTC)
            .map_or(row.at, |start| start.timestamp())
    } else {
        row.at
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// The row of a time entry as the reader sees it.
    pub(super) async fn time_change(
        &self,
        reading: &mut Reading<'_>,
        issue_type: IssueTypeId,
        batch: &Batch<'_>,
        row: &HistoryEntry,
    ) -> Result<Option<HistoryWhat>, AppError> {
        let length = row
            .changes
            .iter()
            .find(|change| change.field == time_entry::MINUTES);
        let event = event_of(length);
        let spent = reading
            .held
            .field_mask_for(Some(issue_type))
            .can_see(Field::TimeSpent);
        let minutes = length
            .filter(|_| spent)
            .and_then(|change| change.after.as_ref().or(change.before.as_ref()))
            .and_then(|value| serde_json::from_value::<Minutes>(value.clone()).ok());
        let own: Vec<FieldChange> = row
            .changes
            .iter()
            .filter(|change| {
                FieldKey::from_code(&change.field).is_some_and(|key| key.own().is_some())
            })
            .cloned()
            .collect();
        let fields = match self.field_changes(reading, issue_type, batch, &own).await? {
            Some(HistoryWhat::Changes { fields }) => fields,
            _ => Vec::new(),
        };
        let sees = moment_visible(row.by, reading.actor.person.id, reading.sees_moments);
        Ok(Some(HistoryWhat::Time {
            event,
            minutes,
            day: hidden_day(row, sees),
            fields,
        }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn minutes(before: Option<u32>, after: Option<u32>) -> FieldChange {
        FieldChange {
            field: time_entry::MINUTES.to_owned(),
            before: before.map(|value| json!(value)),
            after: after.map(|value| json!(value)),
        }
    }

    fn row(changes: Vec<FieldChange>, at: &str) -> HistoryEntry {
        let mut source = bb_core::id::SequentialIds::default();
        HistoryEntry {
            event: bb_core::id::EventId::generate(&mut source).unwrap(),
            issue: bb_core::id::IssueId::generate(&mut source).unwrap(),
            position: 1,
            at: at.parse().unwrap(),
            by: None,
            agent: None,
            kind: bb_core::history::HistoryKind::Change,
            record: bb_core::id::EntityId::from(
                bb_core::id::IssueId::generate(&mut source).unwrap(),
            ),
            record_kind: bb_core::technical::TechnicalName::new(RecordKind::TimeEntry.code())
                .unwrap(),
            run: None,
            undid_run: None,
            changes,
        }
    }

    fn date(before: Option<&str>, after: Option<&str>) -> FieldChange {
        FieldChange {
            field: time_entry::DATE.to_owned(),
            before: before.map(|day| json!(day)),
            after: after.map(|day| json!(day)),
        }
    }

    #[test]
    fn priv_09_the_moment_is_seen_by_who_made_the_row_and_who_may_see_working_hours() {
        let mut source = bb_core::id::SequentialIds::default();
        let (ana, ben) = (UserId::generate(&mut source), UserId::generate(&mut source));
        assert!(moment_visible(Some(ana), ana, false));
        assert!(moment_visible(Some(ana), ben, true));
        assert!(!moment_visible(Some(ana), ben, false));
        assert!(!moment_visible(None, ben, false));
    }

    #[test]
    fn priv_09_a_row_of_time_is_dated_by_its_event_and_never_timed() {
        let at = "2027-01-15T08:30:00Z";
        // A booking and a change that keeps the date set it; a deletion sets the old one.
        let kept = row(vec![date(None, Some("2027-01-14"))], at);
        assert_eq!(entry_day(&kept).to_string(), "2027-01-14");
        // A change that moves the entry says the new date, and the row is shown on it.
        let moved = row(vec![date(Some("2027-01-14"), Some("2027-01-13"))], at);
        assert_eq!(entry_day(&moved).to_string(), "2027-01-13");
        assert_eq!(shown_moment(&moved).to_string(), "2027-01-13T00:00:00Z");
        // An older event that says no date gets the date it was written on, not its time.
        let old = row(Vec::new(), at);
        assert_eq!(entry_day(&old).to_string(), "2027-01-15");
        assert_eq!(shown_moment(&old).to_string(), "2027-01-15T00:00:00Z");
        // A reader who may see the moment is told no date; one who may not is.
        assert_eq!(hidden_day(&kept, true), None);
        assert_eq!(hidden_day(&kept, false), Some(entry_day(&kept)));
    }

    #[test]
    fn priv_09_a_row_that_is_no_time_entry_is_shown_at_its_own_moment() {
        let mut other = row(Vec::new(), "2027-01-15T08:30:00Z");
        other.record_kind = bb_core::technical::TechnicalName::new("issue").unwrap();
        assert_eq!(shown_moment(&other), other.at);
    }

    #[test]
    fn time_02_a_row_says_whether_time_was_booked_changed_or_deleted() {
        assert_eq!(event_of(Some(&minutes(None, Some(90)))), TimeEvent::Booked);
        assert_eq!(
            event_of(Some(&minutes(Some(90), Some(60)))),
            TimeEvent::Changed
        );
        assert_eq!(event_of(Some(&minutes(Some(60), None))), TimeEvent::Deleted);
        assert_eq!(event_of(None), TimeEvent::Changed);
    }
}
