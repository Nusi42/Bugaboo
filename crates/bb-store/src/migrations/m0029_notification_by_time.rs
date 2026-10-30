//! Migration 29: the list of a person's notifications is read newest first by when the event
//! happened (NTF-01, UX §4.11, M5 exit), with the ID as the tie-break, so its index is the
//! person, the time of the event and the ID; the index by the person and the ID goes.

use sea_query::Index;

use super::Step;
use crate::schema::NotificationRow;

pub(super) fn steps() -> Vec<Step> {
    vec![
        Step::DropIndex(
            Index::drop()
                .name("notification_by_recipient")
                .table(NotificationRow::Table)
                .to_owned(),
        ),
        // The list of a person, newest first by the time of the event.
        Step::Index(
            Index::create()
                .name("notification_by_recipient_time")
                .table(NotificationRow::Table)
                .col(NotificationRow::Recipient)
                .col(NotificationRow::At)
                .col(NotificationRow::Id)
                .to_owned(),
        ),
    ]
}
