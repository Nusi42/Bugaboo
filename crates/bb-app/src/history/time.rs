//! A time entry in the history of its issue (TIME-02, PRIV-09): time was booked, an entry changed
//! or was deleted. The reader sees the length only when they may read *Time spent* on the issue,
//! and the own fields of the entry only as their levels allow; who booked is shown as the kind
//! *Time entries* says (`read.rs`).

use bb_core::event::FieldChange;
use bb_core::fields::{Field, FieldKey};
use bb_core::history::{HistoryEntry, TimeEvent};
use bb_core::id::IssueTypeId;
use bb_core::time::Minutes;
use bb_policy::{Permission, Place};
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

/// The date of the entry of a row, when the reader may not see the moment it was written: only the
/// person who booked it and a holder of *See working hours of others* see the moment, because the
/// moment an entry is saved says when somebody stopped working (PRIV-09, ADR-0035, decision 8).
/// Every event of an entry says
/// its date (a booking and a change that keeps the date set it, a change that moves it says the
/// new one, a deletion sets the old one); an older event that says none gets the date the row was
/// written on, never its time.
pub(super) fn hidden_day(row: &HistoryEntry, sees_moment: bool) -> Option<Date> {
    if sees_moment {
        return None;
    }
    row.changes
        .iter()
        .rev()
        .find(|change| change.field == time_entry::DATE)
        .and_then(|change| change.after.as_ref().or(change.before.as_ref()))
        .and_then(|value| serde_json::from_value::<Date>(value.clone()).ok())
        .or_else(|| Some(row.at.to_zoned(TimeZone::UTC).date()))
}

/// The moment a row is shown at: a time entry is dated, not timed, to a reader who may not see the
/// moment, so its moment is the start of its date; any other row keeps its moment.
pub(super) fn shown_at(what: &HistoryWhat, at: Timestamp) -> Timestamp {
    match what {
        HistoryWhat::Time { day: Some(day), .. } => day
            .to_zoned(TimeZone::UTC)
            .map_or(at, |start| start.timestamp()),
        _ => at,
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
        let sees_moment = row.by == Some(reading.actor.person.id)
            || self
                .evaluation_of(reading.actor, Place::Organization)
                .await?
                .allows(Permission::SeeWorkingHoursOfOthers);
        Ok(Some(HistoryWhat::Time {
            event,
            minutes,
            day: hidden_day(row, sees_moment),
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
